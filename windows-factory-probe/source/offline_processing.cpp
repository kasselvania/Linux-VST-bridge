#include <windows.h>
#include "offline_processing.h"
#include "component_instance_session.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include <array>
#include <atomic>
#include <chrono>
#include <bit>
#include <thread>
#include <string>
#include <exception>
#include <stdexcept>

namespace linux_vst_bridge::wf0 {
namespace {
using namespace Steinberg;
using namespace Steinberg::Vst;
constexpr int frames = 16;
constexpr int capacity = 256;
constexpr uint32 guard = 0x4b123456;
constexpr uint32 sentinel = 0x7fc12345;
struct Block {
    std::array<std::array<float, capacity + 2>, 2> input{}, output{}, input_before{};
    std::array<float*, 2> in{}, out{};
    AudioBusBuffers input_bus{}, output_bus{};
    ParameterChanges parameters{2};
    Changes commercial_parameters;Notes notes;ExternalBlock request{};
    ProcessData data;
    double gain{};
    tresult result{kNotInitialized};
    bool worker_thread{false};
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
    const int event_inputs=component.getBusCount(kEvent,kInput);
    if(commercial&&(inputs<0||inputs>1||component.getBusCount(kAudio,kOutput)!=1||event_inputs!=1))throw std::runtime_error("unsupported commercial bus layout");
    const bool hosted=external&&external->hosted();
    const bool sustained=external&&external->sustained();
    const bool stateful=external&&external->stateful();
    if(stateful){external->bind_component(&component);external->bind_processor(&processor);}
    for(;;) {
    if(stateful&&!external->initial_transition())return {true,true};
    uint32_t maximum=external?capacity:frames;
    const auto owner = std::this_thread::get_id();
    std::array<Block,3> blocks;
    // All buffers and SDK parameter queues are allocated/populated on the owner
    // thread before activation. Each process call receives a separate block.
    for (int b=0;b<3;++b) {
        auto& block=blocks[b]; block.gain=b==0?0.5:0.25;
        for (int ch=0;ch<2;++ch) {
            block.input[ch].front()=block.input[ch][frames+1]=std::bit_cast<float>(guard);
            block.output[ch].fill(std::bit_cast<float>(sentinel));
            block.output[ch].front()=block.output[ch][frames+1]=std::bit_cast<float>(guard);
            for(int i=0;i<frames;++i) {
                const int numerator=ch==0?((i+b*2)%9)-4:((i*3+b+2)%11)-5;
                block.input[ch][i+1]=(external||b==2)?0.f:static_cast<float>(numerator)/8.f;
            }
            block.in[ch]=block.input[ch].data()+1;block.out[ch]=block.output[ch].data()+1;
        }
        int32 parameter_index=0, point_index=0;
        auto* gain=block.parameters.addParameterData(0,parameter_index);
        auto* bypass=block.parameters.addParameterData(2,parameter_index);
        if (!gain || !bypass || gain->addPoint(0,block.gain,point_index)!=kResultOk ||
            bypass->addPoint(0,0.,point_index)!=kResultOk) return {false,true};
        block.input_bus.numChannels=block.output_bus.numChannels=2;
        block.input_bus.channelBuffers32=block.in.data();
        block.output_bus.channelBuffers32=block.out.data();
        block.input_bus.silenceFlags=b==2?3:0;
        block.output_bus.silenceFlags=0;
        block.data.processMode=sustained?kRealtime:kOffline;block.data.symbolicSampleSize=kSample32;
        block.data.numSamples=frames;block.data.numInputs=block.data.numOutputs=1;
        block.data.inputs=&block.input_bus;block.data.outputs=&block.output_bus;
        block.data.inputParameterChanges=&block.parameters;
    }
    bool ok=true, active=false, stopped=true, joined=false, worker_exception=false;
    auto call = [&](const char* operation, auto function, bool notification = false) {
        events.lifecycle("ap0_call_started",",\"operation\":\""+std::string(operation)+
            "\",\"owner_thread\":"+(std::this_thread::get_id()==owner?"true":"false"));
        callbacks.begin_plugin_call(events.sequence(),operation);
        const auto result=function();
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
    SpeakerArrangement input=SpeakerArr::kStereo, output=SpeakerArr::kStereo;
    if (!(external&&external->performance()) && !call("set_bus_arrangements",[&]{return processor.setBusArrangements(inputs?&input:nullptr,inputs,&output,1);})) return {false,true};
    ProcessSetup setup{};setup.processMode=stateful?static_cast<int32>(external->process_mode()):sustained?kRealtime:kOffline;setup.symbolicSampleSize=kSample32;
    setup.maxSamplesPerBlock=static_cast<int32>(maximum);setup.sampleRate=external?external->sample_rate():48000.;
    if (!(external&&external->performance()) && !call("setup_processing",[&]{return processor.setupProcessing(setup);})) return {false,true};
    if(hosted&&!external->performance()){
        const auto latency=processor.getLatencySamples(),tail=processor.getTailSamples();
        events.lifecycle("ap2_processor_traits",",\"latency_samples\":"+std::to_string(latency)+",\"tail_samples\":"+std::to_string(tail));
        if(latency!=0||(!commercial&&tail!=0))throw std::runtime_error("AP2 retained processor latency/tail differs");
    }
    if (inputs&&!call("activate_audio_input",[&]{return component.activateBus(kAudio,kInput,0,true);})) return {false,true};
    if (!call("activate_audio_output",[&]{return component.activateBus(kAudio,kOutput,0,true);})) return {false,true};
    if (event_inputs&&!call(commercial?"activate_event_input":"deactivate_event_input",[&]{return component.activateBus(kEvent,kInput,0,commercial);})) return {false,true};
    active=call("set_active_true",[&]{return component.setActive(true);});
    if (!active) return {false,false};
    if(hosted) external->lifecycle_ack(9);
    uint64_t processed=0,intervals=0;
    bool restart=false;
    std::exception_ptr primary_error;
    do {
    joined=false;restart=false;
    try {
        std::atomic<bool> worker_done{false};
        std::thread worker([&] {
            // No owner-thread call overlaps this thread. Logging surrounds calls;
            // the sample comparison and buffer serialization happen after join.
            SetThreadDescription(GetCurrentThread(),L"lvb-audio");
            bool started=false;
            try {
                events.lifecycle("ap0_processing_thread_started",",\"distinct_from_owner\":"+
                    std::string(std::this_thread::get_id()!=owner?"true":"false"));
                if(hosted) external->lifecycle_request(10);
                stopped=false;
                started=call("set_processing_true",[&]{return processor.setProcessing(true);},true);
                if (started && external) {if(hosted) external->lifecycle_ack(11);else external->ready();}
                if (started) for(uint64_t b=0;sustained||b<uint64_t(external?65:3);++b) {
                    auto& block=blocks[external?0:b];
                    if (external) {
                        for(int ch=0;ch<2;++ch) {
                            block.input[ch].fill(0.f);block.output[ch].fill(std::bit_cast<float>(sentinel));
                            block.input[ch].front()=block.input[ch].back()=std::bit_cast<float>(guard);
                            block.output[ch].front()=block.output[ch].back()=std::bit_cast<float>(guard);
                        }
                        auto& request=block.request;
                        if (!external->next(request,block.in[0],block.in[1])) break;
                        if(request.frames>static_cast<int>(maximum)) throw std::runtime_error("negotiated maximum exceeded");
                        block.input_before=block.input;
                        block.gain=request.gain;block.data.numSamples=request.frames;
                        block.input_bus.silenceFlags=request.silence;block.output_bus.silenceFlags=0;
                        block.parameters.clearQueue();int32 parameter=0,point=0;
                        if(request.gain_present)block.parameters.addParameterData(0,parameter)->addPoint(0,request.gain,point);
                        if(!stateful)block.parameters.addParameterData(2,parameter)->addPoint(0,0.,point);
                        block.data.numInputs=request.frames?inputs:0;block.data.numOutputs=request.frames?1:0;
                        block.data.inputs=request.frames&&inputs?&block.input_bus:nullptr;block.data.outputs=request.frames?&block.output_bus:nullptr;
                    }
                    if(commercial){
                        block.commercial_parameters.load(block.request.events.data(),block.request.event_count,block.notes);
                        block.data.inputParameterChanges=&block.commercial_parameters;block.data.inputEvents=&block.notes;
                    }
                    block.worker_thread=std::this_thread::get_id()!=owner;
                    if(!sustained)events.lifecycle("ap0_process_started",",\"block\":"+std::to_string(b));
                    if(external)external->before_process();
                    const auto process_start=std::chrono::steady_clock::now();
                    block.result=processor.process(block.data);
                    const auto process_ns=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-process_start).count();
                    if(external)external->after_process();
                    if(!sustained)events.lifecycle("ap0_process_completed",",\"block\":"+std::to_string(b)+
                        ",\"result\":"+std::to_string(block.result));
                    if(block.result!=kResultOk) {ok=false;if(sustained)throw std::runtime_error("Windows processor returned failure");break;}
                    ++processed;
                    if(external) {
                        for(int ch=0;ch<2;++ch) {
                            if(block.input[ch]!=block.input_before[ch] ||
                               std::bit_cast<uint32>(block.output[ch].front())!=guard ||
                               std::bit_cast<uint32>(block.output[ch].back())!=guard)
                                throw std::runtime_error("AP1 private buffer guard/input");
                            for(int i=block.data.numSamples+1;i<=capacity;++i)
                                if(std::bit_cast<uint32>(block.output[ch][i])!=sentinel)
                                    throw std::runtime_error("AP1 unused private output modified");
                        }
                        if(!sustained)events.lifecycle("ap1_private_buffers_valid",",\"block\":"+std::to_string(b));
                    }
                    if(external) external->done(block.out[0],block.out[1],block.output_bus.silenceFlags,uint64_t(process_ns));
                }
            } catch (...) {primary_error=std::current_exception();worker_exception=true;ok=false;}
            // Attempt bounded teardown through the same supervisor even after
            // a failed process result. A hang is owned by the outer timeout.
            try {stopped=call("set_processing_false",[&]{return processor.setProcessing(false);},true);}
            catch (...) {if(!primary_error)primary_error=std::current_exception();stopped=false;ok=false;}
            worker_done.store(true,std::memory_order_release);
        });
        if(stateful)while(!worker_done.load(std::memory_order_acquire)){
            external->service_owner();std::this_thread::sleep_for(std::chrono::microseconds(50));
        }
        worker.join();joined=true;
    } catch (...) {if(!primary_error)primary_error=std::current_exception();ok=false;}
    events.lifecycle("ap0_thread_joined",",\"joined\":"+std::string(joined?"true":"false")+
        ",\"processing_stopped\":"+(stopped?"true":"false")+
        ",\"worker_exception\":"+(worker_exception?"true":"false"));
    if (!stopped) return {false,false};
    if(hosted&&ok) {
        external->lifecycle_ack(13);
        if(sustained)restart=external->next_transition()==10;
        if(!restart)external->lifecycle_request(14);
    }
    ++intervals;
    } while(restart&&ok);
    if(sustained) {
        events.lifecycle("ap3_processing_summary",",\"processed_blocks\":"+std::to_string(processed)+
            ",\"intervals\":"+std::to_string(intervals)+",\"process_mode\":\"kRealtime\"");
        if(primary_error)try{std::rethrow_exception(primary_error);}catch(const std::exception& e){
            // Only our fixed explanatory errors are emitted, not paths or args.
            // Windows/plugin exceptions retain their stage via the outer supervisor.
            events.lifecycle("ap3_processing_error",",\"detail\":\""+std::string(e.what()).substr(0,160)+"\"");
        }catch(...){events.lifecycle("ap3_processing_error",",\"detail\":\"non-standard processing exception\"");}
    }
    active=!call("set_active_false",[&]{return component.setActive(false);});
    if (active) return {false,false};
    call("deactivate_audio_output",[&]{return component.activateBus(kAudio,kOutput,0,false);});
    if(inputs)call("deactivate_audio_input",[&]{return component.activateBus(kAudio,kInput,0,false);});
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
            ",\"output_silence_flags\":"+std::to_string(block.output_bus.silenceFlags)+
            ",\"input_bits\":["+bits(block.input[0])+","+bits(block.input[1])+"]"+
            ",\"output_bits\":["+bits(block.output[0])+","+bits(block.output[1])+"]");
    }
    return {ok && joined && !worker_exception, true};
    }
}
}
