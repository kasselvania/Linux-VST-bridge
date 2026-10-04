// Production Windows SDK processing owner, with a scripted transport. Thread
// preparation/reuse is observed at the real lifecycle acknowledgements; this
// proves ordering and retirement, not an installed real-time timing result.
#include "offline_processing.h"
#include "component_instance_session.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include <tlhelp32.h>
#include <algorithm>
#include <array>
#include <atomic>
#include <cassert>
#include <cwchar>
#include <exception>
#include <iostream>
#include <stdexcept>
#include <string_view>
#include <thread>

using namespace Steinberg;
using namespace Steinberg::Vst;
using namespace linux_vst_bridge::wf0;

namespace {
struct PreparedThread {
    DWORD id=0;
    HANDLE handle=nullptr;
    PreparedThread()=default;
    PreparedThread(const PreparedThread&)=delete;
    ~PreparedThread(){if(handle)CloseHandle(handle);}
    void observe() {
        assert(!handle);
        HANDLE snapshot=CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD,0);
        assert(snapshot!=INVALID_HANDLE_VALUE);
        THREADENTRY32 row{};row.dwSize=sizeof(row);
        BOOL present=Thread32First(snapshot,&row);
        while(present){
            if(row.th32OwnerProcessID==GetCurrentProcessId()){
                HANDLE candidate=OpenThread(THREAD_QUERY_LIMITED_INFORMATION|SYNCHRONIZE,FALSE,row.th32ThreadID);
                if(candidate){
                    PWSTR description=nullptr;
                    const bool audio=SUCCEEDED(GetThreadDescription(candidate,&description))&&description&&wcscmp(description,L"lvb-audio")==0;
                    if(description)LocalFree(description);
                    if(audio&&WaitForSingleObject(candidate,0)==WAIT_TIMEOUT){
                        assert(!handle&&"only one prepared render thread belongs to the activation");
                        id=row.th32ThreadID;handle=candidate;
                    }else CloseHandle(candidate);
                }
            }
            row.dwSize=sizeof(row);present=Thread32Next(snapshot,&row);
        }
        CloseHandle(snapshot);
        // Behavioral red on the old loop: it creates the thread only after
        // Activated has already been acknowledged and Start has been received.
        assert(handle&&"prepared lvb-audio thread absent at Activated acknowledgement");
    }
    void live()const{assert(handle&&WaitForSingleObject(handle,0)==WAIT_TIMEOUT);}
    void retired()const{assert(handle&&WaitForSingleObject(handle,0)==WAIT_OBJECT_0);}
};
struct Setup {uint32_t maximum,mode;double rate;};
enum class Case {idle,reconfigure,legacy_finite,owner_ack,owner_before_start,owner_during_start,cancel_restart,render_before_start,refused_start,throw_start,throw_start_owner_failure,refused_stop,hung_process};

struct Fixture final:AudioEffect {
    DWORD owner=GetCurrentThreadId();
    std::atomic<DWORD> prepared{0};
    std::atomic<unsigned> starts{0},stops{0},blocks{0};
    std::atomic<bool> process_entered{false};
    PreparedThread* thread=nullptr;
    EventWriter* events=nullptr;
    std::atomic<unsigned long long> owner_sequence{0};
    Setup accepted{};
    bool reject_start=false,reject_stop=false,hang_process=false;
    bool throw_start=false,await_cancel=false;
    std::atomic<bool>* cancelled=nullptr;
    bool deferred_logging=true;
    bool processing=false;
    unsigned active_calls=0,inactive_calls=0;
    tresult PLUGIN_API initialize(FUnknown* host)override{
        auto result=AudioEffect::initialize(host);
        addAudioInput(u"In",SpeakerArr::kStereo);addAudioOutput(u"Out",SpeakerArr::kStereo);
        return result;
    }
    tresult PLUGIN_API setupProcessing(ProcessSetup& setup)override{
        assert(GetCurrentThreadId()==owner&&!processing);
        accepted={uint32_t(setup.maxSamplesPerBlock),uint32_t(setup.processMode),setup.sampleRate};
        return kResultOk;
    }
    tresult PLUGIN_API setActive(TBool active)override{
        assert(GetCurrentThreadId()==owner&&!processing);
        if(active)++active_calls;
        else {assert(thread);thread->retired();++inactive_calls;}
        return kResultOk;
    }
    tresult PLUGIN_API setProcessing(TBool active)override{
        assert(GetCurrentThreadId()!=owner&&GetCurrentThreadId()==prepared.load(std::memory_order_acquire));
        // Only the owner exports scalar transition records. The production
        // render wrapper must not format/flush records before either call.
        if(deferred_logging)assert(events&&events->sequence()==owner_sequence.load(std::memory_order_acquire));
        if(active){
            assert(!processing);++starts;
            if(await_cancel){assert(cancelled);while(!cancelled->load(std::memory_order_acquire))Sleep(1);}
            if(throw_start)throw std::runtime_error("private-vendor-start-exception-marker");
            if(reject_start)return kResultFalse;
            processing=true;
        }
        else {++stops;if(reject_stop)return kResultFalse;processing=false;}
        return kResultOk;
    }
    tresult PLUGIN_API process(ProcessData& data)override{
        assert(processing&&GetCurrentThreadId()==prepared.load(std::memory_order_acquire));
        if(hang_process){process_entered.store(true,std::memory_order_release);Sleep(INFINITE);}
        assert(data.numSamples>=0&&uint32_t(data.numSamples)<=accepted.maximum);
        assert(uint32_t(data.processMode)==accepted.mode);
        if(data.numSamples==0)assert(!data.inputs&&!data.outputs&&data.numInputs==0&&data.numOutputs==0);
        else for(int ch=0;ch<2;++ch){
            for(int i=0;i<data.numSamples;++i)data.outputs[0].channelBuffers32[ch][i]=data.inputs[0].channelBuffers32[ch][i]*.5f;
            data.outputs[0].silenceFlags=0;
        }
        ++blocks;return kResultOk;
    }
};
struct Script final:ExternalProcessing {
    Fixture& fixture;
    Case scenario;
    std::array<PreparedThread,3> threads;
    std::array<Setup,3> setups{{{64,0,48000.},{1024,2,96000.},{128,1,48000.}}};
    size_t activation=0;
    unsigned intervals=0,block_index=0,delivered=0,started_acks=0,stopped_acks=0;
    unsigned owner_failures=0;
    std::atomic<bool> cancelled{false},start_entered{false};
    explicit Script(Fixture& p,Case c):fixture(p),scenario(c){
        fixture.reject_start=c==Case::refused_start;fixture.reject_stop=c==Case::refused_stop;fixture.hang_process=c==Case::hung_process;
        fixture.throw_start=c==Case::throw_start||c==Case::throw_start_owner_failure;
        fixture.await_cancel=c==Case::throw_start_owner_failure;fixture.cancelled=&cancelled;
        fixture.deferred_logging=c!=Case::legacy_finite;
        if(c==Case::legacy_finite)setups[0].mode=2;
    }
    bool hosted()const override{return true;}
    bool sustained()const override{return scenario!=Case::legacy_finite;}
    bool stateful()const override{return true;}
    bool commercial()const override{return scenario!=Case::legacy_finite;}
    uint32_t process_mode()const override{return setups[activation].mode;}
    double sample_rate()const override{return setups[activation].rate;}
    void ready()override{}
    uint32_t lifecycle_request(uint16_t kind)override{
        if(kind==8)return setups[activation].maximum;
        if(kind==10){
            assert(GetCurrentThreadId()==threads[activation].id);
            if(scenario==Case::render_before_start)throw std::runtime_error("scripted render lifecycle failure");
            if(scenario==Case::owner_during_start||(scenario==Case::cancel_restart&&intervals==1)){
                start_entered.store(true,std::memory_order_release);
                while(!cancelled.load(std::memory_order_acquire))Sleep(1);
                throw std::runtime_error("scripted cancelled start handoff");
            }
            block_index=0;return 0;
        }
        assert(kind==14);assert(GetCurrentThreadId()==fixture.owner);return 0;
    }
    void lifecycle_ack(uint16_t kind)override{
        if(kind==9){
            assert(GetCurrentThreadId()==fixture.owner);
            threads[activation].observe();fixture.thread=&threads[activation];
            fixture.prepared.store(threads[activation].id,std::memory_order_release);
            fixture.owner_sequence.store(fixture.events->sequence(),std::memory_order_release);
            if(scenario==Case::owner_ack)throw std::runtime_error("scripted activation acknowledgement failure");
        }else if(kind==11){
            assert(GetCurrentThreadId()==threads[activation].id);++started_acks;
        }else if(kind==13){
            assert(GetCurrentThreadId()==fixture.owner&&!fixture.processing);
            if(scenario==Case::legacy_finite)threads[activation].retired();else threads[activation].live();
            assert(delivered==4*(intervals+1));
            ++stopped_acks;++intervals;
            fixture.owner_sequence.store(fixture.events->sequence(),std::memory_order_release);
        }else{
            assert(kind==15&&GetCurrentThreadId()==fixture.owner);
            threads[activation].retired();
        }
    }
    uint16_t next_transition()override{
        assert(GetCurrentThreadId()==fixture.owner);
        threads[activation].live();
        if(scenario==Case::owner_before_start)throw std::runtime_error("scripted owner selection failure");
        return scenario==Case::idle||intervals==2?14:10;
    }
    bool activation_again()override{
        threads[activation].retired();
        if(scenario!=Case::reconfigure||++activation==setups.size())return false;
        intervals=0;delivered=0;return true;
    }
    void service_owner()override{
        if((scenario==Case::owner_during_start||scenario==Case::cancel_restart)&&start_entered.load(std::memory_order_acquire))
            throw std::runtime_error("scripted owner cancellation while Start is in flight");
        if(scenario==Case::hung_process&&fixture.process_entered.load(std::memory_order_acquire))
            throw std::runtime_error("scripted owner cancellation while vendor process is hung");
        if(scenario==Case::throw_start_owner_failure&&fixture.starts.load(std::memory_order_acquire))
            throw std::runtime_error("scripted owner cancellation before vendor start throws");
    }
    void owner_failed()noexcept override{assert(GetCurrentThreadId()==fixture.owner);++owner_failures;cancelled.store(true,std::memory_order_release);}
    bool next(ExternalBlock& block,float* left,float* right)override{
        assert(GetCurrentThreadId()==threads[activation].id);
        if(block_index==4)return false;
        const std::array<unsigned,4> lengths{0,1,setups[activation].maximum/2,setups[activation].maximum};
        block.frames=int(lengths[block_index++]);block.silence=0;block.gain_present=false;
        for(int i=0;i<block.frames;++i){left[i]=.25f;right[i]=-.5f;}
        return true;
    }
    void done(const float* left,const float* right,uint64_t,uint64_t,const ap10_results_t*)override{
        const std::array<unsigned,4> lengths{0,1,setups[activation].maximum/2,setups[activation].maximum};
        for(unsigned i=0;i<lengths[block_index-1];++i)assert(left[i]==.125f&&right[i]==-.25f);
        ++delivered;
    }
};
void exercise(Case scenario){
    HostApplication host;Fixture fixture;assert(fixture.initialize(&host)==kResultOk);
    Script script(fixture,scenario);EventWriter events(1048576);HostCallbackSink callbacks(&events,GetCurrentThreadId());
    fixture.events=&events;
    const auto begin=GetTickCount64();
    auto result=run_offline_processing(fixture,fixture,callbacks,events,&script);
    assert(result.quiescent&&fixture.active_calls==fixture.inactive_calls);
    const bool success=scenario==Case::idle||scenario==Case::reconfigure||scenario==Case::legacy_finite;
    assert(result.success==success&&result.retirement_ready==success);
    if(scenario==Case::idle){assert(fixture.starts==0&&fixture.stops==0&&fixture.blocks==0);}
    if(scenario==Case::reconfigure){
        assert(fixture.starts==6&&fixture.stops==6&&fixture.blocks==24);
        assert(script.started_acks==6&&script.stopped_acks==6&&fixture.active_calls==3);
    }
    if(scenario==Case::legacy_finite){assert(fixture.starts==1&&fixture.stops==1&&fixture.blocks==4&&script.stopped_acks==1);}
    if(scenario==Case::owner_ack||scenario==Case::owner_before_start||scenario==Case::owner_during_start){
        assert(script.cancelled.load()&&script.owner_failures==1&&fixture.starts==0&&fixture.stops==0);
    }
    if(scenario==Case::render_before_start){assert(fixture.starts==0&&fixture.stops==0&&script.started_acks==0);}
    if(scenario==Case::refused_start){assert(fixture.starts==1&&fixture.stops==1&&script.started_acks==0&&fixture.blocks==0);}
    if(scenario==Case::throw_start||scenario==Case::throw_start_owner_failure){
        assert(fixture.starts==1&&fixture.stops==1&&script.started_acks==0&&fixture.blocks==0);
        if(scenario==Case::throw_start_owner_failure)assert(script.owner_failures==1&&script.cancelled.load());
    }
    if(scenario==Case::cancel_restart){
        assert(script.owner_failures==1&&script.cancelled.load());
        assert(fixture.starts==1&&fixture.stops==1&&fixture.blocks==4&&script.stopped_acks==1);
    }
    assert(GetTickCount64()-begin<5000);
    assert(fixture.terminate()==kResultOk);
}
}
int main(int argc,char** argv){
    std::set_terminate([]{ExitProcess(96);});
    if(argc==2&&std::string_view(argv[1])=="--cancel-restart"){
        exercise(Case::cancel_restart);return 0;
    }
    if(argc==2&&(std::string_view(argv[1])=="--refuse-stop"||std::string_view(argv[1])=="--hang")){
        exercise(std::string_view(argv[1])=="--hang"?Case::hung_process:Case::refused_stop);
        // A refused vendor stop cannot authorize component deactivation or
        // releasing its borrowed storage. The production owner must contain.
        return 97;
    }
    assert(argc==1);
    exercise(Case::idle);
    exercise(Case::reconfigure);
    // Finite workers publish their exact completion immediately before exit.
    // Exercise that terminal/completion boundary and the legacy actual join
    // before ACK13, without replacing lifetime evidence with a text search.
    for(int i=0;i<16;++i)exercise(Case::legacy_finite);
    exercise(Case::owner_ack);
    exercise(Case::owner_before_start);
    exercise(Case::owner_during_start);
    exercise(Case::render_before_start);
    exercise(Case::refused_start);
    exercise(Case::throw_start);
    exercise(Case::throw_start_owner_failure);
    std::cout<<"prepared render lifecycle: activation ordering, interval reuse, zero frames, inactive reconfiguration and failure retirement passed\n";
}
