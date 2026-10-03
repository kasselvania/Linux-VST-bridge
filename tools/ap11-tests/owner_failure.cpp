// Real production processing loop and pinned SDK processor. Scripted transport
// deliberately parks after captured output; no commercial or Linux claim.
#include "offline_processing.h"
#include "component_instance_session.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include <atomic>
#include <algorithm>
#include <cassert>
#include <exception>
#include <iostream>
#include <stdexcept>
#include <string_view>
#include <thread>
using namespace Steinberg;
using namespace Steinberg::Vst;
using namespace linux_vst_bridge::wf0;

struct Processor final : AudioEffect {
    unsigned starts=0, stops=0, blocks=0;
    std::thread::id owner=std::this_thread::get_id();
    tresult PLUGIN_API initialize(FUnknown* host) override {
        auto result=AudioEffect::initialize(host);
        addAudioInput(u"In", SpeakerArr::kStereo);
        addAudioOutput(u"Out", SpeakerArr::kStereo);
        return result;
    }
    tresult PLUGIN_API setProcessing(TBool active) override {
        assert(std::this_thread::get_id()!=owner);
        if(active)++starts;else ++stops;
        return kResultOk;
    }
    tresult PLUGIN_API process(ProcessData& data) override {
        assert(std::this_thread::get_id()!=owner && data.numSamples==256);
        for(int ch=0;ch<2;++ch)for(int i=0;i<data.numSamples;++i)
            data.outputs[0].channelBuffers32[ch][i]=data.inputs[0].channelBuffers32[ch][i]*.5f;
        data.outputs[0].silenceFlags=0;
        ++blocks; return kResultOk;
    }
};
struct Session final : ExternalProcessing {
    std::atomic<bool> delivered{false}, cancelled{false};
    bool hang=false;
    unsigned failures=0;
    bool stateful() const override {return true;}
    bool sustained() const override {return true;}
    uint16_t next_transition() override {return 10;}
    void ready() override {}
    void service_owner() override {
        if(delivered.load(std::memory_order_acquire)) {
            ++failures;
            throw std::runtime_error("sensitive-vendor-exception-marker");
        }
    }
    // Also compiles against the unmodified base for the red reproduction.
    void owner_failed() noexcept {cancelled.store(true,std::memory_order_release);}
    bool next(ExternalBlock& block,float* left,float* right) override {
        if(delivered.load(std::memory_order_acquire)) {
            if(hang)Sleep(INFINITE);
            while(!cancelled.load(std::memory_order_acquire))Sleep(1);
            return false;
        }
        block.frames=256;block.gain_present=false;block.silence=0;
        std::fill_n(left,256,.25f);std::fill_n(right,256,-.5f);return true;
    }
    void done(const float* left,const float* right,uint64_t,uint64_t,const ap10_results_t*) override {
        for(int i=0;i<256;++i)assert(left[i]==.125f&&right[i]==-.25f);
        delivered.store(true,std::memory_order_release);
    }
};
int main(int argc,char** argv) {
    // A joinable-thread destructor on the old path is an unambiguous failure.
    std::set_terminate([]{ExitProcess(96);});
    Session session;session.hang=argc==2&&std::string_view(argv[1])=="--hang";
    HostApplication host;Processor processor;
    assert(processor.initialize(&host)==kResultOk);
    EventWriter events(1048576);HostCallbackSink callbacks(&events,GetCurrentThreadId());
    const auto started=GetTickCount64();
    auto result=run_offline_processing(processor,processor,callbacks,events,&session);
    assert(!session.hang && !result.success && result.quiescent && !result.retirement_ready);
    assert(session.cancelled.load()&&session.failures==1&&processor.blocks==1);
    assert(processor.starts==1&&processor.stops==1&&GetTickCount64()-started<5000);
    assert(processor.terminate()==kResultOk);
    std::cout<<"owner exception: captured samples correct, worker cancelled/joined, failed result retained\n";
}
