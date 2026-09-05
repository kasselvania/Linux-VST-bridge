#include "offline_processing.h"
#include "component_instance_session.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include <array>
#include <bit>
#include <thread>
#include <string>

namespace linux_vst_bridge::wf0 {
namespace {
using namespace Steinberg;
using namespace Steinberg::Vst;
constexpr int frames = 16;
constexpr uint32 guard = 0x4b123456;
constexpr uint32 sentinel = 0x7fc12345;
struct Block {
    std::array<std::array<float, frames + 2>, 2> input{}, output{};
    std::array<float*, 2> in{}, out{};
    AudioBusBuffers input_bus{}, output_bus{};
    ParameterChanges parameters{2};
    ProcessData data;
    double gain{};
    tresult result{kNotInitialized};
    bool worker_thread{false};
};
std::string bits(const std::array<float, frames + 2>& values) {
    std::string text="[";
    for (int i=0;i<frames+2;++i) {
        if (i) text+=",";
        text += std::to_string(std::bit_cast<uint32>(values[i]));
    }
    return text+"]";
}
}
OfflineResult run_offline_processing(IComponent& component, IAudioProcessor& processor,
                                    HostCallbackSink& callbacks, EventWriter& events) {
    const auto owner = std::this_thread::get_id();
    std::array<Block,3> blocks;
    // All buffers and SDK parameter queues are allocated/populated on the owner
    // thread before activation. Each process call receives a separate block.
    for (int b=0;b<3;++b) {
        auto& block=blocks[b]; block.gain=b==0?0.5:0.25;
        for (int ch=0;ch<2;++ch) {
            block.input[ch].front()=block.input[ch].back()=std::bit_cast<float>(guard);
            block.output[ch].fill(std::bit_cast<float>(sentinel));
            block.output[ch].front()=block.output[ch].back()=std::bit_cast<float>(guard);
            for(int i=0;i<frames;++i) {
                const int numerator=ch==0?((i+b*2)%9)-4:((i*3+b+2)%11)-5;
                block.input[ch][i+1]=b==2?0.f:static_cast<float>(numerator)/8.f;
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
        block.data.processMode=kOffline;block.data.symbolicSampleSize=kSample32;
        block.data.numSamples=frames;block.data.numInputs=block.data.numOutputs=1;
        block.data.inputs=&block.input_bus;block.data.outputs=&block.output_bus;
        block.data.inputParameterChanges=&block.parameters;
    }
    bool ok=true, active=false, stopped=true, joined=false, worker_exception=false;
    auto call = [&](const char* operation, auto function) {
        events.lifecycle("ap0_call_started",",\"operation\":\""+std::string(operation)+
            "\",\"owner_thread\":"+(std::this_thread::get_id()==owner?"true":"false"));
        callbacks.begin_plugin_call(events.sequence(),operation);
        const auto result=function();
        callbacks.end_plugin_call();
        events.lifecycle("ap0_call_completed",",\"operation\":\""+std::string(operation)+
            "\",\"result\":"+std::to_string(result));
        ok = ok && result==kResultOk;
        return result==kResultOk;
    };
    SpeakerArrangement input=SpeakerArr::kStereo, output=SpeakerArr::kStereo;
    if (!call("set_bus_arrangements",[&]{return processor.setBusArrangements(&input,1,&output,1);})) return {false,true};
    ProcessSetup setup{};setup.processMode=kOffline;setup.symbolicSampleSize=kSample32;
    setup.maxSamplesPerBlock=frames;setup.sampleRate=48000.;
    if (!call("setup_processing",[&]{return processor.setupProcessing(setup);})) return {false,true};
    if (!call("activate_audio_input",[&]{return component.activateBus(kAudio,kInput,0,true);})) return {false,true};
    if (!call("activate_audio_output",[&]{return component.activateBus(kAudio,kOutput,0,true);})) return {false,true};
    if (!call("deactivate_event_input",[&]{return component.activateBus(kEvent,kInput,0,false);})) return {false,true};
    active=call("set_active_true",[&]{return component.setActive(true);});
    if (!active) return {false,false};
    try {
        std::thread worker([&] {
            // No owner-thread call overlaps this thread. Logging surrounds calls;
            // the sample comparison and buffer serialization happen after join.
            bool started=false;
            try {
                events.lifecycle("ap0_processing_thread_started",",\"distinct_from_owner\":"+
                    std::string(std::this_thread::get_id()!=owner?"true":"false"));
                stopped=false;
                started=call("set_processing_true",[&]{return processor.setProcessing(true);});
                if (started) for(int b=0;b<3;++b) {
                    auto& block=blocks[b];
                    block.worker_thread=std::this_thread::get_id()!=owner;
                    events.lifecycle("ap0_process_started",",\"block\":"+std::to_string(b));
                    block.result=processor.process(block.data);
                    events.lifecycle("ap0_process_completed",",\"block\":"+std::to_string(b)+
                        ",\"result\":"+std::to_string(block.result));
                    if(block.result!=kResultOk) {ok=false;break;}
                }
            } catch (...) {worker_exception=true;ok=false;}
            // Attempt bounded teardown through the same supervisor even after
            // a failed process result. A hang is owned by the outer timeout.
            try {stopped=call("set_processing_false",[&]{return processor.setProcessing(false);});}
            catch (...) {stopped=false;ok=false;}
        });
        worker.join();joined=true;
    } catch (...) {ok=false;}
    events.lifecycle("ap0_thread_joined",",\"joined\":"+std::string(joined?"true":"false")+
        ",\"processing_stopped\":"+(stopped?"true":"false")+
        ",\"worker_exception\":"+(worker_exception?"true":"false"));
    if (!stopped) return {false,false};
    active=!call("set_active_false",[&]{return component.setActive(false);});
    if (active) return {false,false};
    call("deactivate_audio_output",[&]{return component.activateBus(kAudio,kOutput,0,false);});
    call("deactivate_audio_input",[&]{return component.activateBus(kAudio,kInput,0,false);});
    for(int b=0;b<3;++b) {
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
