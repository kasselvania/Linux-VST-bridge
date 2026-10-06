#include <string_view>
#include <windows.h>
#include "offline_processing.h"
#include "result_status.h"
#include "../../native-vst3-proxy/include/ap10_sdk_results.h"
#include "component_instance_session.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include <array>
#include <algorithm>
#include <atomic>
#include <chrono>
#include <bit>
#include <thread>
#include <string>
#include <exception>
#include <memory>
#include <stdexcept>
#include <xmmintrin.h>
namespace {
// Audio hosts run vendor DSP with denormals flushed to zero, and plug-ins rely
// on it. With the default MXCSR, decaying filter, delay and reverb tails fall
// onto the CPU's denormal slow path and one block can cost many times more.
struct FlushDenormals {
    unsigned saved = _mm_getcsr();
    FlushDenormals() { _mm_setcsr(saved | 0x8040); } // FTZ | DAZ
    ~FlushDenormals() { _mm_setcsr(saved); }
};
}
#ifdef LVB_SAMPLE_PLANE_TEST
#include <cassert>
#endif

namespace linux_vst_bridge::wf0 {
namespace {
using namespace Steinberg;
using namespace Steinberg::Vst;
constexpr int frames = 16;
constexpr int capacity = ap1::block_capacity;
constexpr uint32 guard = 0x4b123456;
constexpr uint32 sentinel = 0x7fc12345;
struct Block {
    std::array<std::array<float, capacity + 2>, 2> input{}, output{}, input_before{};
    std::array<float*, 2> in{}, out{};
    std::array<std::array<float,capacity+2>,62> extra_output{};
    std::array<float*,64> output_channels{};
    AudioBusBuffers input_bus{};
    std::array<AudioBusBuffers,AP18Buses::max_audio_outputs> all_outputs{};
    std::array<float*,2> inactive_output_channels{};
    std::array<AudioBusBuffers,8> all_inputs{};
    std::array<float,capacity+2> silent_input{};
    std::array<float*,2> silent_channels{silent_input.data()+1,silent_input.data()+1};
    ParameterChanges parameters{2};
    Changes commercial_parameters;Notes notes;ExternalBlock request{};
    AP10Results::Collector returned;
    ProcessData data;
    double gain{};
    tresult result{kNotInitialized};
    bool worker_thread{false};
#ifdef LVB_SAMPLE_PLANE_TEST
    size_t sample_plane_reads=0,sample_plane_writes=0;
#endif
};
// The existing owner selects commands; the existing render thread consumes
// them. Events are hints, while the exact generation transfers the pending
// frame and interval results. No event left by preparation/Stop can satisfy a
// later Run. All objects are prepared before Activated is acknowledged.
struct ProcessingSignals {
    HANDLE command=CreateEventW(nullptr,FALSE,FALSE,nullptr);
    HANDLE quiescent=CreateEventW(nullptr,FALSE,FALSE,nullptr);
    std::atomic<uint64_t> issued{0},completed{UINT64_MAX};
    std::atomic<bool> exit{false},done{false};
    ~ProcessingSignals(){if(command)CloseHandle(command);if(quiescent)CloseHandle(quiescent);}
    void publish(uint64_t generation)noexcept{
        completed.store(generation,std::memory_order_release);
        SetEvent(quiescent);
    }
    void request_exit()noexcept{
        exit.store(true,std::memory_order_release);
        if(command)SetEvent(command);
    }
};
// Installed immediately after thread construction, including before an
// acknowledgement or owner command selection that can throw. Storage and SDK
// leases remain alive until join; a hung borrower is contained in this process.
struct ProcessingJoin {
    std::thread& worker;
    ProcessingSignals& signals;
    ExternalProcessing* external;
    bool joined=false;
    void finish(bool cancel)noexcept{
        if(!worker.joinable())return;
        if(cancel&&external)external->owner_failed();
        signals.request_exit();
        if(WaitForSingleObject(worker.native_handle(),5000)!=WAIT_OBJECT_0){
            TerminateProcess(GetCurrentProcess(),93);std::terminate();
        }
        try{worker.join();joined=true;}catch(...){TerminateProcess(GetCurrentProcess(),93);std::terminate();}
    }
    ~ProcessingJoin(){finish(true);}
};
struct TransitionRecord {
    bool attempted=false,returned=false;
    tresult result=kNotInitialized;
};
std::string bits(const std::array<float, capacity + 2>& values) {
    std::string text="[";
    for (int i=0;i<frames+2;++i) {
        if (i) text+=",";
        text += std::to_string(std::bit_cast<uint32>(values[i]));
    }
    return text+"]";
}
}
OfflineResult run_offline_processing(IComponent& component, IAudioProcessor& processor,
                                    HostCallbackSink& callbacks, EventWriter& events, ExternalProcessing* external) {
    const bool commercial=external&&external->commercial();
    const int inputs=commercial?component.getBusCount(kAudio,kInput):1;
    BusLayout layout;layout.read(component,processor);
    const bool hosted=external&&external->hosted();
    const bool sustained=external&&external->sustained();
    const bool stateful=external&&external->stateful();
    if(stateful){external->bind_component(&component);external->bind_processor(&processor);}
    for(;;) {
    if(stateful&&!external->initial_transition())return {true,true};
    if(external&&external->bus_layout())layout=*external->bus_layout();
    uint32_t maximum=external?capacity:frames;
    const auto owner = std::this_thread::get_id();
    // The full multi-output buffers are owned and allocated before activation.
    // Keeping them off the Windows owner stack also avoids its default 1 MiB
    // stack ceiling; processing only borrows this fixed storage.
    auto storage = std::make_unique<std::array<Block,3>>();
    auto& blocks = *storage;
    // All buffers and SDK parameter queues are allocated/populated on the owner
    // thread before activation. Each process call receives a separate block.
    for (int b=0;b<3;++b) {
        auto& block=blocks[b];
        block.silent_input.fill(0.f);
        block.silent_input.front()=block.silent_input.back()=std::bit_cast<float>(guard);
        for(size_t ch=0;ch<64;++ch){
            auto& plane=ch<2?block.output[ch]:block.extra_output[ch-2];
            plane.fill(std::bit_cast<float>(sentinel));
            plane.front()=plane.back()=std::bit_cast<float>(guard);
            block.output_channels[ch]=plane.data()+1;
        }
        block.gain=b==0?0.5:0.25;
        block.returned.buses=uint32_t(layout.counts[3]);
        size_t event_out=0;for(size_t i=0;i<layout.size;++i)if(layout.buses[i].info.mediaType==kEvent&&layout.buses[i].info.direction==kOutput){block.returned.channels[event_out]=layout.buses[i].effective_channels;block.returned.bus_active[event_out++]=layout.buses[i].active?1:0;}
        for (int ch=0;ch<2;++ch) {
            block.input[ch].fill(0.f);
            block.input[ch].front()=block.input[ch].back()=std::bit_cast<float>(guard);
            for(int i=0;i<frames;++i) {
                const int numerator=ch==0?((i+b*2)%9)-4:((i*3+b+2)%11)-5;
                block.input[ch][i+1]=(external||b==2)?0.f:static_cast<float>(numerator)/8.f;
            }
            if(!external){
                block.input[ch][frames+1]=std::bit_cast<float>(guard);
                block.output[ch][frames+1]=std::bit_cast<float>(guard);
            }
            block.in[ch]=block.input[ch].data()+1;block.out[ch]=block.output[ch].data()+1;
        }
        int32 parameter_index=0, point_index=0;
        auto* gain=block.parameters.addParameterData(0,parameter_index);
        auto* bypass=block.parameters.addParameterData(2,parameter_index);
        if (!gain || !bypass || gain->addPoint(0,block.gain,point_index)!=kResultOk ||
            bypass->addPoint(0,0.,point_index)!=kResultOk) return {false,true};
        block.input_bus.numChannels=block.all_outputs[0].numChannels=2;
        block.input_bus.channelBuffers32=block.in.data();
        block.all_outputs[0].channelBuffers32=block.out.data();
        block.input_bus.silenceFlags=b==2?3:0;
        layout.map_outputs(block.all_outputs,block.output_channels.data(),block.inactive_output_channels.data());
        block.data.processMode=sustained?kRealtime:kOffline;block.data.symbolicSampleSize=kSample32;
        block.data.numSamples=frames;block.data.numInputs=inputs;block.data.numOutputs=layout.counts[1];
        layout.map_inputs(block.all_inputs,block.in.data(),block.silent_channels.data(),block.input_bus.silenceFlags);
        block.data.inputs=inputs?block.all_inputs.data():nullptr;block.data.outputs=&block.all_outputs[0];
        block.data.inputParameterChanges=&block.parameters;
    }
    bool ok=true, active=false, stopped=true, joined=false, worker_exception=false;
    auto call = [&](const char* operation, auto function, bool notification = false) {
        events.lifecycle("ap0_call_started",",\"operation\":\""+std::string(operation)+
            "\",\"owner_thread\":"+(std::this_thread::get_id()==owner?"true":"false"));
        callbacks.begin_plugin_call(events.sequence(),operation);
        const uint64_t activity=std::string_view(operation)=="setup_processing"?1:std::string_view(operation)=="set_active_true"?2:std::string_view(operation)=="set_processing_true"?3:std::string_view(operation)=="set_processing_false"?4:5;
        if(external)external->lifecycle_activity(std::this_thread::get_id()==owner,activity);
        const auto result=function();
        if(external)external->lifecycle_activity(std::this_thread::get_id()==owner,0);
        callbacks.end_plugin_call();
        events.lifecycle("ap0_call_completed",",\"operation\":\""+std::string(operation)+
            "\",\"result\":"+std::to_string(result));
        // The pinned SDK AudioEffect default is a no-op notification returning
        // kNotImplemented. This exception applies only to setProcessing.
        const bool accepted = result==kResultOk || (notification && result==kNotImplemented);
        ok = ok && accepted;
        return accepted;
    };
    if(hosted) {
        maximum=external->lifecycle_request(8);

    }
    if(stateful)for(auto& block:blocks)block.data.processMode=external->process_mode();
    if (!(external&&external->performance()))layout.negotiate(processor);
    ProcessSetup setup{};setup.processMode=stateful?static_cast<int32>(external->process_mode()):sustained?kRealtime:kOffline;setup.symbolicSampleSize=kSample32;
    setup.maxSamplesPerBlock=static_cast<int32>(maximum);setup.sampleRate=external?external->sample_rate():48000.;
    if (!(external&&external->performance()) && !call("setup_processing",[&]{return processor.setupProcessing(setup);})) return {false,true};
    if(hosted&&!external->performance()){
        const auto latency=processor.getLatencySamples(),tail=processor.getTailSamples();
        events.lifecycle("ap2_processor_traits",",\"latency_samples\":"+std::to_string(latency)+",\"tail_samples\":"+std::to_string(tail));
        if(latency!=0||(!commercial&&tail!=0))throw std::runtime_error("AP2 retained processor latency/tail differs");
    }
    layout.activate(component,true);
    active=call("set_active_true",[&]{return component.setActive(true);});
    if (!active) return {false,false};
    uint64_t processed=0,intervals=0;
    std::exception_ptr primary_error;
    AP10Results::RejectionRecord first_rejection{};
    uint64_t rejected_generation=0,rejected_epoch=0,rejected_sequence=0,rejected_position=0,rejected_callback=0;
    uint32_t rejected_input_notes=0,rejected_input_parameters=0;
    bool processing_requested=false;
    bool join_recorded=false;
    bool transitions_exported=true;
    std::array<TransitionRecord,2> transitions{};
    // The installed sustained path initializes the plug-in with the SDK
    // HostApplication (inspect_module), not this wrapper's observer sink.
    // Its transition wrapper therefore touches only fixed records/status on
    // render. Noncommercial reference admission retains its legacy observer.
    const bool deferred_transitions=commercial&&sustained;
    auto render_transition=[&](bool running){
        if(!deferred_transitions)return call(running?"set_processing_true":"set_processing_false",
            [&]{return processor.setProcessing(running);},true);
        auto& record=transitions[running?0:1];record.attempted=true;
        external->lifecycle_activity(false,running?3:4);
        // A throw/refusal retains its fixed in-flight stage until the owner
        // acquires and exports the scalar failure. Cleanup may publish its own
        // stage, while this exact attempted/returned/result record survives.
        record.result=processor.setProcessing(running);record.returned=true;
        const bool accepted=record.result==kResultOk||record.result==kNotImplemented;
        if(accepted)external->lifecycle_activity(false,0);
        ok=ok&&accepted;return accepted;
    };
    auto export_transitions=[&]{
        if(transitions_exported)return;
        transitions_exported=true; // one export attempt, including output failure
        if(!deferred_transitions)return;
        constexpr std::array<const char*,2> names{"set_processing_true","set_processing_false"};
        for(size_t i=0;i<transitions.size();++i){
            const auto& record=transitions[i];if(!record.attempted)continue;
            events.lifecycle("ap0_call_started",",\"operation\":\""+std::string(names[i])+"\",\"owner_thread\":false,\"exported_after_quiescence\":true,\"returned\":"+(record.returned?"true":"false"));
            if(record.returned)events.lifecycle("ap0_call_completed",",\"operation\":\""+std::string(names[i])+"\",\"result\":"+std::to_string(record.result));
        }
    };
    try {
        ProcessingSignals signals;
        if(!signals.command||!signals.quiescent)throw std::runtime_error("processing event preparation failed");
        uint32_t preparation_error=0;
        std::thread worker([&] {
            // Processor lifecycle and DSP belong to this thread. Controller/
            // editor work remains on the owner; state requests use the existing
            // ExternalProcessing handoff. Storage stays alive through join.
            // Naming supports diagnostics; worker/event preparation supplies
            // readiness even on runners without this optional naming support.
            SetThreadDescription(GetCurrentThread(),L"lvb-audio");
            signals.publish(0); // prepared; no vendor Start or DSP has occurred
            uint64_t handled=0;
            for(;;){
            const auto wake=WaitForSingleObject(signals.command,INFINITE);
            if(signals.exit.load(std::memory_order_acquire))break;
            if(wake!=WAIT_OBJECT_0){preparation_error=2;ok=false;worker_exception=true;break;}
            const auto generation=signals.issued.load(std::memory_order_acquire);
            if(generation==handled)continue;
            if(generation!=handled+1){preparation_error=3;ok=false;worker_exception=true;break;}
            bool started=false,processing_attempted=false;
            try {
                if(hosted) external->lifecycle_request(10);
                if(signals.exit.load(std::memory_order_acquire))break;
                processing_attempted=true;stopped=false;
                started=render_transition(true);
                if (started && external) {
#ifdef LVB_LC1_TEST
                    external->lc1_hold_started();
#endif
                    if(hosted) external->lifecycle_ack(11);else external->ready();
                }
                if (started) for(uint64_t b=0;sustained||b<uint64_t(external?65:3);++b) {
                    auto& block=blocks[external?0:b];
                    if (external) {
#ifdef LVB_SAMPLE_PLANE_TEST
                        block.sample_plane_reads=block.sample_plane_writes=0;
#endif
                        auto& request=block.request;
                        if (!external->next(request,block.in[0],block.in[1])) break;
                        if(request.frames>static_cast<int>(maximum)) throw std::runtime_error("negotiated maximum exceeded");
                        if(request.frames&& !external->direct_audio()){
                            block.input_before=block.input;
#ifdef LVB_SAMPLE_PLANE_TEST
                            block.sample_plane_reads+=2;block.sample_plane_writes+=2;
#endif
                        }
                        block.gain=request.gain;block.data.numSamples=request.frames;
                        block.data.processMode=callback_process_mode(request,setup.processMode);
                        block.input_bus.silenceFlags=request.silence;
                        layout.map_inputs(block.all_inputs,block.in.data(),block.silent_channels.data(),request.silence);layout.map_outputs(block.all_outputs,block.output_channels.data(),block.inactive_output_channels.data());
                        block.parameters.clearQueue();int32 parameter=0,point=0;
                        if(request.gain_present)block.parameters.addParameterData(0,parameter)->addPoint(0,request.gain,point);
                        if(!stateful)block.parameters.addParameterData(2,parameter)->addPoint(0,0.,point);
                        block.data.numInputs=request.frames?inputs:0;block.data.numOutputs=request.frames?layout.counts[1]:0;
                        block.data.inputs=request.frames&&inputs?block.all_inputs.data():nullptr;block.data.outputs=request.frames?&block.all_outputs[0]:nullptr;
                    }
                    if(commercial){
                        block.commercial_parameters.load(block.request.events.data(),block.request.event_count,block.notes);
                        block.data.inputParameterChanges=&block.commercial_parameters;block.data.inputEvents=&block.notes;
                    }
                    block.returned.reset(block.data.numSamples);
                    block.data.outputEvents=external&&external->returned_results()?&block.returned:nullptr;
                    block.data.outputParameterChanges=external&&external->returned_results()?&block.returned:nullptr;
                    block.data.processContext=block.request.has_context?&block.request.context:nullptr;
                    block.worker_thread=std::this_thread::get_id()!=owner;
                    const auto admitted_samples=block.data.numSamples;
                    if(!sustained)events.lifecycle("ap0_process_started",",\"block\":"+std::to_string(b));
                    if(external)external->before_process();
                    const auto process_start=std::chrono::steady_clock::now();
                    {FlushDenormals flush;block.result=processor.process(block.data);}
                    const auto process_ns=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-process_start).count();
                    // Custody immediately after the vendor call, before any throw/teardown.
                    if(block.returned.failed&&first_rejection.reason==AP10Results::Rejection::None){
                        first_rejection=block.returned.rejection;rejected_callback=processed+1;
                        rejected_generation=block.request.generation;rejected_epoch=block.request.epoch;
                        rejected_sequence=block.request.sequence;rejected_position=block.request.position;
                        for(size_t i=0;i<block.request.event_count;++i){if(block.request.events[i].kind==2)++rejected_input_parameters;else ++rejected_input_notes;}
                        if(external&&external->result_status())external->result_status()->publish(first_rejection,rejected_generation,rejected_epoch,rejected_sequence,rejected_position,rejected_callback,rejected_input_notes,rejected_input_parameters);
                    }
                    if(external)external->after_process();
                    if(!sustained)events.lifecycle("ap0_process_completed",",\"block\":"+std::to_string(b)+
                        ",\"result\":"+std::to_string(block.result));
                    if(block.result!=kResultOk) {ok=false;if(sustained)throw std::runtime_error("Windows processor returned failure");break;}
                    if(block.returned.failed)throw std::runtime_error("malformed or oversized process results");
                    if(block.data.numSamples!=admitted_samples)
                        throw std::runtime_error("Windows processor changed admitted sample extent");
                    if(admitted_samples&&external&&external->direct_audio()) {
                        // Two constant canaries per exposed SDK plane. Keep
                        // overwrite containment without proof-time Nmax scans.
                        std::array<int,2> bus_index{};
                        for(size_t i=0;i<layout.size;++i) {
                            const auto& bus=layout.buses[i];
                            if(bus.info.mediaType!=kAudio)continue;
                            const auto direction=bus.info.direction;const int index=bus_index[direction]++;
                            const bool selected=bus.supported&&bus.active&&
                                (direction==kOutput||index==layout.transported_input);
                            for(int ch=0;ch<bus.info.channelCount;++ch) {
                                // Use prepared addresses/extent, never a pointer
                                // or count the vendor could replace in ProcessData.
                                const int lane=2*index+ch;
                                const auto* plane=direction==kInput?(selected?block.input[ch].data()+1:block.silent_input.data()+1):
                                    (selected?(lane<2?block.output[lane].data()+1:block.extra_output[lane-2].data()+1):nullptr);
                                if(plane&&(std::bit_cast<uint32>(plane[-1])!=guard||
                                           std::bit_cast<uint32>(plane[capacity])!=guard))
                                    throw std::runtime_error("direct private plane guard");
                            }
                        }
                    }
                    if(admitted_samples&&(!external||!external->direct_audio())&&
                       (std::bit_cast<uint32>(block.silent_input.front())!=guard||std::bit_cast<uint32>(block.silent_input.back())!=guard||
                        std::any_of(block.silent_input.begin()+1,block.silent_input.end()-1,[](float v){return v!=0.f;})))
                        throw std::runtime_error("AP18 inactive input modified");
                    ++processed;
                    if(external&&admitted_samples&&!external->direct_audio()) {
#ifdef LVB_SAMPLE_PLANE_TEST
                        block.sample_plane_reads+=65;
#endif
                        for(int ch=0;ch<2;++ch) {
                            if(block.input[ch]!=block.input_before[ch] ||
                               std::bit_cast<uint32>(block.output[ch].front())!=guard ||
                               std::bit_cast<uint32>(block.output[ch].back())!=guard)
                                throw std::runtime_error("AP1 private buffer guard/input");
                            for(int i=admitted_samples+1;i<=capacity;++i)
                                if(std::bit_cast<uint32>(block.output[ch][i])!=sentinel)
                                    throw std::runtime_error("AP1 unused private output modified");
                        }
                        for(const auto& plane:block.extra_output){
                            if(std::bit_cast<uint32>(plane.front())!=guard||std::bit_cast<uint32>(plane.back())!=guard)
                                throw std::runtime_error("extra private output guard");
                            for(int i=admitted_samples+1;i<=capacity;++i)if(std::bit_cast<uint32>(plane[i])!=sentinel)
                                throw std::runtime_error("extra private output extent");
                        }
                        if(!sustained)events.lifecycle("ap1_private_buffers_valid",",\"block\":"+std::to_string(b));
                    }
                    if(external) external->done_outputs(block.all_outputs.data(),layout.counts[1],uint64_t(process_ns),&block.returned.values);
                    if(external&&admitted_samples&&!external->direct_audio()){
                        const auto end=admitted_samples+1;
                        for(int ch=0;ch<2;++ch)
                            std::fill(block.output[ch].begin()+1,block.output[ch].begin()+end,std::bit_cast<float>(sentinel));
                        for(auto& plane:block.extra_output)
                            std::fill(plane.begin()+1,plane.begin()+end,std::bit_cast<float>(sentinel));
#ifdef LVB_SAMPLE_PLANE_TEST
                        block.sample_plane_writes+=64;
#endif
                    }
#ifdef LVB_SAMPLE_PLANE_TEST
                    assert(admitted_samples||
                           (block.sample_plane_reads==0&&block.sample_plane_writes==0));
                    assert(!admitted_samples||
                           (block.sample_plane_reads!=0&&block.sample_plane_writes!=0));
#endif
                }
            } catch (...) {primary_error=std::current_exception();worker_exception=true;ok=false;}
            // Attempt bounded teardown through the same supervisor even after
            // a failed process result. A hang is owned by the outer timeout.
            if(processing_attempted){
                try {stopped=render_transition(false);}
                catch (...) {if(!primary_error)primary_error=std::current_exception();stopped=false;ok=false;}
            }
            handled=generation;
            // This is the publication barrier previously provided by joining
            // every interval. All pending/session/frame/capture borrows and
            // vendor false are complete before the owner may inspect fields,
            // acknowledge Stopped or publish another Run.
            signals.publish(generation);
            if(!ok||!sustained)break;
            }
            signals.done.store(true,std::memory_order_release);SetEvent(signals.quiescent);
        });
        ProcessingJoin retirement{worker,signals,external};
        bool owner_error=false;
        try {
            auto await_quiescence=[&](uint64_t generation,bool preparing){
                const auto end=std::chrono::steady_clock::now()+std::chrono::seconds(5);
                while(signals.completed.load(std::memory_order_acquire)!=generation){
                    if(signals.done.load(std::memory_order_acquire)){
                        // A finite worker publishes completion immediately
                        // before done. Accept that exact completion even if
                        // the first load raced with its publication.
                        if(signals.completed.load(std::memory_order_acquire)==generation)break;
                        throw std::runtime_error("processing worker stopped before quiescence");
                    }
                    if(preparing&&std::chrono::steady_clock::now()>=end)throw std::runtime_error("processing preparation deadline");
                    if(stateful)external->service_owner();
                    const auto wake=WaitForSingleObject(signals.quiescent,stateful&&!preparing?0:4);
                    if(wake!=WAIT_OBJECT_0&&wake!=WAIT_TIMEOUT)throw std::runtime_error("processing quiescence wait failed");
                    // Preserve the existing owner state/GUI service cadence
                    // during an interval. Render/start work does not poll it.
                    if(stateful&&!preparing&&wake==WAIT_TIMEOUT)
                        std::this_thread::sleep_for(std::chrono::microseconds(50));
                }
            };
            await_quiescence(0,true);
            if(hosted&&!sustained)external->lifecycle_ack(9);
            events.lifecycle("ap0_processing_thread_started",",\"distinct_from_owner\":"+
                std::string(worker.get_id()!=owner?"true":"false"));
            if(hosted&&sustained)external->lifecycle_ack(9); // prepared, never processing_ready
            uint64_t generation=0;
            uint16_t command=sustained?external->next_transition():10;
            for(;;){
                if(command==14){external->lifecycle_request(14);break;}
                if(command!=10||generation==UINT64_MAX)throw std::runtime_error("processing command generation invalid");
                processing_requested=true;
                // Owner owns these fields while the previous generation is
                // quiescent. Clearing before Run publication also prevents an
                // Exit that wins over pending Run from exporting stale fields.
                transitions={};transitions_exported=false;
                signals.issued.store(++generation,std::memory_order_release);
                if(!SetEvent(signals.command))throw std::runtime_error("processing command wake failed");
                await_quiescence(generation,false);
                ++intervals;
                export_transitions();
                if(!stopped){
                    // A failed vendor false never grants deactivation or
                    // permission to release its processing storage/interfaces.
                    TerminateProcess(GetCurrentProcess(),93);std::terminate();
                }
                if(!ok)break;
                if(!sustained){
                    // AP0/AP1/AP2 finite admission retains its real joined
                    // worker before Stopped. Sustained intervals instead
                    // retain the prepared live thread until deactivation.
                    retirement.finish(false);joined=retirement.joined;
                    events.lifecycle("ap0_thread_joined",",\"joined\":true,\"processing_stopped\":true,\"worker_exception\":false");
                    join_recorded=true;
                }
                if(hosted)external->lifecycle_ack(13);
                if(!sustained){if(hosted)external->lifecycle_request(14);break;}
                command=external->next_transition();
            }
        } catch (...) {
            // Do not unwind a joinable std::thread, detach a borrower of our
            // buffers, or race the worker's error/result fields. Ask the owned
            // transport to stop, then join before touching shared results.
            owner_error=true;
            retirement.finish(true);
        }
        retirement.finish(false);joined=retirement.joined;
        // Owner cancellation can leave an attempted Start/Stop after the
        // ordinary quiescence wait has thrown. Join supplies its final acquire
        // barrier; export the same generation once before containment/cleanup.
        try{export_transitions();}catch(...){owner_error=true;}
        if(!stopped){TerminateProcess(GetCurrentProcess(),93);std::terminate();}
        // A preparation failure does not grant ACK9. Its fixed code is read
        // only after join, when no render thread can still write the results.
        if(preparation_error&&!primary_error)primary_error=std::make_exception_ptr(std::runtime_error("Windows processing preparation failed"));
        // Vendor exception text can contain private paths/account data. Keep
        // the fault stage in its existing status owner and emit only our code.
        if(owner_error){primary_error=std::make_exception_ptr(std::runtime_error("Windows owner service failed"));ok=false;}
    } catch (...) {if(!primary_error)primary_error=std::current_exception();ok=false;}
    if(!join_recorded)events.lifecycle("ap0_thread_joined",",\"joined\":"+std::string(joined?"true":"false")+
        ",\"processing_stopped\":"+(stopped?"true":"false")+
        ",\"worker_exception\":"+(worker_exception?"true":"false"));
    if(first_rejection.reason!=AP10Results::Rejection::None){
        // This branch is on the owner after worker.join(): no process-call I/O.
        const auto& r=first_rejection;
        static constexpr const char* names[]={"None","EventCapacity","NegativeBus","UndeclaredBus","BusStorage","NegativeEventOffset","EventExtent","NonFinitePPQ","UnsupportedEvent","InvalidEventField","EventChannel","EventPayload","PayloadCapacity","NullPayload","PayloadAlignment","QueueCapacity","PointCapacity","NegativePointOffset","PointExtent","NonFiniteValue","ValueBelowZero","ValueAboveOne"};
        std::string detail=",\"schema\":1,\"reason\":\""+std::string(names[static_cast<uint32_t>(r.reason)])+"\"";
        detail+=",\"frames\":"+std::to_string(r.frames);
        detail+=",\"events\":"+std::to_string(r.events);
        detail+=",\"points\":"+std::to_string(r.points);
        detail+=",\"queues\":"+std::to_string(r.queues);
        detail+=",\"bytes\":"+std::to_string(r.bytes);
        detail+=",\"event_type\":"+std::to_string(r.event_type);
        detail+=",\"bus\":"+std::to_string(r.bus);
        detail+=",\"offset\":"+std::to_string(r.offset);
        detail+=",\"channel\":"+std::to_string(r.channel);
        detail+=",\"declared_channels\":"+std::to_string(r.declared_channels);
        detail+=",\"event_bus_active\":"+std::to_string(r.event_bus_active);
        detail+=",\"flags\":"+std::to_string(r.flags);
        detail+=",\"declared_buses\":"+std::to_string(r.declared_buses);
        detail+=",\"payload_type\":"+std::to_string(r.payload_type);
        detail+=",\"payload_size\":"+std::to_string(r.payload_size);
        detail+=",\"parameter_id\":"+std::to_string(r.parameter_id);
        detail+=",\"ppq_bits\":"+std::to_string(r.ppq_bits);
        detail+=",\"value_bits\":"+std::to_string(r.value_bits);
        detail+=",\"field_a\":"+std::to_string(r.field_a);
        detail+=",\"field_b\":"+std::to_string(r.field_b);
        detail+=",\"extra_bits\":"+std::to_string(r.extra_bits);
        detail+=",\"generation\":"+std::to_string(rejected_generation);
        detail+=",\"epoch\":"+std::to_string(rejected_epoch);
        detail+=",\"request_sequence\":"+std::to_string(rejected_sequence);
        detail+=",\"position\":"+std::to_string(rejected_position);
        detail+=",\"callback\":"+std::to_string(rejected_callback);
        detail+=",\"input_notes\":"+std::to_string(rejected_input_notes);
        detail+=",\"input_parameters\":"+std::to_string(rejected_input_parameters);
        events.lifecycle("ap18_result_rejection",detail);
    }
    if (!stopped){TerminateProcess(GetCurrentProcess(),93);std::terminate();}
    if(sustained) {
        events.lifecycle("ap3_processing_summary",",\"processed_blocks\":"+std::to_string(processed)+
            ",\"intervals\":"+std::to_string(intervals)+",\"process_mode\":\"kRealtime\"");
        if(primary_error)try{std::rethrow_exception(primary_error);}catch(const ap1::BridgeError& e){
            events.lifecycle("ap3_processing_error",",\"detail\":\""+std::string(e.what()).substr(0,160)+"\"");
        }catch(...){events.lifecycle("ap3_processing_error",",\"detail\":\"Windows processing failed\"");}
    }
    active=!call("set_active_false",[&]{return component.setActive(false);});
    if (active) return {false,false};
    layout.activate(component,false);
    if(hosted&&ok) {
        external->lifecycle_ack(15);
        if(stateful){if(external->activation_again())continue;}
        else external->lifecycle_request(5);
    }
    if (!external) for(int b=0;b<3;++b) {
        const auto& block=blocks[b];
        events.final_lifecycle("ap0_samples",",\"block\":"+std::to_string(b)+
            ",\"gain\":"+std::to_string(block.gain)+",\"sample_rate\":48000,\"frames\":16"+
            ",\"sample_format\":\"kSample32\",\"process_mode\":\"kOffline\""+
            ",\"process_result\":"+std::to_string(block.result)+
            ",\"worker_thread\":"+(block.worker_thread?"true":"false")+
            ",\"input_silence_flags\":"+std::to_string(block.input_bus.silenceFlags)+
            ",\"output_silence_flags\":"+std::to_string(block.all_outputs[0].silenceFlags)+
            ",\"input_bits\":["+bits(block.input[0])+","+bits(block.input[1])+"]"+
            ",\"output_bits\":["+bits(block.output[0])+","+bits(block.output[1])+"]");
    }
    return {ok && (!processing_requested || joined) && !worker_exception, true, ok && stopped && joined && !worker_exception && !active};
    }
}
}
