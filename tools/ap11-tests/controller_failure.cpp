// Production MappedSession, controller synchronization and SDK processing.
// The Python peer supplies fixed protocol-5 packets and checks actual samples.
#include "mapped_processing.h"
#include "component_instance_session.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include <atomic>
#include <cassert>
#include <iostream>
#include <stdexcept>
using namespace Steinberg;
using namespace Steinberg::Vst;
using namespace linux_vst_bridge::wf0;
struct Controller final : EditController {
    std::wstring mode;
    HANDLE entered=nullptr, release_update=nullptr;
    std::atomic<bool> busy{false};
    unsigned updates=0, saves=0;
    DWORD owner=GetCurrentThreadId();
    tresult PLUGIN_API initialize(FUnknown* host) override {
        auto result=EditController::initialize(host);
        parameters.addParameter(u"Gain",u"",0,.5,ParameterInfo::kCanAutomate,42);
        return result;
    }
    tresult PLUGIN_API setParamNormalized(ParamID id,ParamValue value) override {
        assert(GetCurrentThreadId()==owner&&id==42&&value==.25);
        ++updates;busy.store(true,std::memory_order_release);assert(SetEvent(entered));
        assert(WaitForSingleObject(release_update,5000)==WAIT_OBJECT_0);
        busy.store(false,std::memory_order_release);
        if(mode.starts_with(L"throw"))throw std::runtime_error("injected controller update exception");
        if(mode.starts_with(L"refuse"))return kResultFalse;
        return EditController::setParamNormalized(id,value);
    }
    tresult PLUGIN_API getState(IBStream*) override {++saves;return kResultFalse;}
};
struct Processor final : AudioEffect {
    Controller& controller;
    unsigned blocks=0, concurrent=0, starts=0, stops=0, saves=0;
    explicit Processor(Controller& c):controller(c){}
    tresult PLUGIN_API initialize(FUnknown* host) override {
        auto result=AudioEffect::initialize(host);
        addAudioInput(u"In",SpeakerArr::kStereo);addAudioOutput(u"Out",SpeakerArr::kStereo);return result;
    }
    tresult PLUGIN_API setProcessing(TBool active) override {
        assert(GetCurrentThreadId()!=controller.owner);
        if(active)++starts;else ++stops;return kResultOk;
    }
    tresult PLUGIN_API process(ProcessData& data) override {
        assert(data.numSamples==256&&GetCurrentThreadId()!=controller.owner);
        for(int ch=0;ch<2;++ch)for(int i=0;i<data.numSamples;++i)
            data.outputs[0].channelBuffers32[ch][i]=data.inputs[0].channelBuffers32[ch][i]*.5f;
        data.outputs[0].silenceFlags=0;
        ++blocks;if(controller.busy.load(std::memory_order_acquire))++concurrent;
        return kResultOk;
    }
    tresult PLUGIN_API getState(IBStream*) override {++saves;return kResultFalse;}
};
int wmain(int argc,wchar_t** argv) {
    assert(argc==5);
    HostApplication host;Controller controller;controller.mode=argv[1];
    std::wstring event=argv[4];
    controller.entered=OpenEventW(EVENT_MODIFY_STATE,FALSE,(event+L"-entered").c_str());
    controller.release_update=OpenEventW(SYNCHRONIZE,FALSE,(event+L"-release").c_str());
    assert(controller.entered&&controller.release_update);
    assert(controller.initialize(&host)==kResultOk);
    Processor processor(controller);assert(processor.initialize(&host)==kResultOk);
    EventWriter events(1048576);HostCallbackSink callbacks(&events,GetCurrentThreadId());
    std::wstring wide_id=argv[3];std::string id(wide_id.begin(),wide_id.end());
    {
        MappedSession session(argv[2],id,events,true,true,true,true,false);
        session.bind_controller(&controller,true);session.ready();
        auto result=run_offline_processing(processor,processor,callbacks,events,&session);
        assert(result.quiescent&&processor.starts==1&&processor.stops==1&&controller.updates==1);
        assert(processor.saves==0&&controller.saves==0);
        if(controller.mode==L"slow") {
            assert(result.success&&result.retirement_ready&&processor.concurrent>=8&&processor.blocks==10);
            session.finish(true);
        } else {
            assert(!result.success&&!result.retirement_ready&&processor.blocks>=1&&processor.blocks<=2);
        }
        session.bind_controller(nullptr,true);
        std::cout<<"controller fixture: blocks="<<processor.blocks<<", concurrent="<<processor.concurrent
            <<", state calls="<<(processor.saves+controller.saves)<<"\n";
    }
    assert(processor.terminate()==kResultOk&&controller.terminate()==kResultOk);
    CloseHandle(controller.entered);CloseHandle(controller.release_update);
}
