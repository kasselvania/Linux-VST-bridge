// Independent official-SDK consumer of the two installed first-party proxies.
// No bridge transport, state decoder, manager mock, or Windows producer is used.
#include "../../vst-state/stream.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstmessage.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/module.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include "public.sdk/source/vst/hosting/eventlist.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <cmath>
#include <dlfcn.h>
#include <fstream>
#include <functional>
#include <iostream>
#include <stdexcept>
#include <thread>
#include <vector>
using namespace Steinberg;
using namespace Steinberg::Vst;
using Clock = std::chrono::steady_clock;
namespace {
const char* stage = "load";
void need(bool value, const char* why) { if (!value) throw std::runtime_error(why); }
void ok(tresult value, const char* why) { need(value==kResultOk,why); }
using Mark = void (*)();
using Count = uint64_t (*)();
Mark auditBegin;
Count auditEnd;
template<class F> tresult callback(F function) {
    if(auditBegin) auditBegin();
    auto result=function();
    if(auditEnd) need(auditEnd()==0,"callback effect observed");
    return result;
}
std::vector<uint8_t> load(const std::string& path) {
    std::ifstream file(path,std::ios::binary|std::ios::ate);
    need(bool(file),"saved state unavailable");
    auto size=file.tellg();need(size>0&&size<=int64_t(LVBState::payloadLimit+LVBState::overhead),"saved state extent");
    std::vector<uint8_t> bytes(static_cast<size_t>(size));
    file.seekg(0);file.read(reinterpret_cast<char*>(bytes.data()),size);
    need(bool(file),"saved state read");return bytes;
}
void save(const std::string& path,const LVBState::Stream& stream) {
    need(!stream.failed&&!stream.bytes.empty()&&stream.quiescent(),"actual complete state required");
    std::ofstream file(path,std::ios::binary);
    file.write(reinterpret_cast<const char*>(stream.bytes.data()),std::streamsize(stream.bytes.size()));
    file.flush();need(bool(file),"saved state write");
}
// Own only completed SDK lifecycle steps. The audio thread is declared after
// its borrowed storage and is joined before this owner's destruction.
struct Lifecycle {
    IPtr<IComponent> component;
    IPtr<IEditController> controller;
    FUnknownPtr<IAudioProcessor> processor;
    FUnknownPtr<IConnectionPoint> cp,cc;
    bool componentInitialized=false,controllerInitialized=false;
    bool componentConnected=false,controllerConnected=false,active=false,finished=false,clean=true;
    std::function<void(bool)> unload;
    ~Lifecycle() { finish(); }
    bool finish() noexcept {
        if(finished) return clean;
        finished=true;
        auto call=[&](bool completed,auto operation) -> tresult {
            if(!completed) return kResultOk;
            try {auto result=operation();if(result!=kResultOk) clean=false;return result;}
            catch(...) {clean=false;return kInternalError;}
        };
        auto deactivate=call(active,[&]{return component->setActive(false);});
        auto controllerDisconnect=call(controllerConnected,[&]{return cc->disconnect(cp);});
        auto componentDisconnect=call(componentConnected,[&]{return cp->disconnect(cc);});
        auto terminate=call(componentInitialized,[&]{return component->terminate();});
        auto controllerTerminate=call(controllerInitialized,[&]{return controller->terminate();});
        // If an SDK owner refuses termination, retain the image until process
        // exit. A failed cleanup must not unload code still owning objects.
        if(clean) {cc=nullptr;cp=nullptr;processor=nullptr;controller=nullptr;component=nullptr;}
        if(unload) unload(clean);
        std::cout<<"{\"event\":\"host_retired\",\"deactivate_result\":"<<deactivate
                 <<",\"component_disconnect_result\":"<<componentDisconnect
                 <<",\"controller_disconnect_result\":"<<controllerDisconnect
                 <<",\"terminate_result\":"<<terminate
                 <<",\"controller_terminate_result\":"<<controllerTerminate
                 <<",\"module_unloaded\":"<<(clean?"true":"false")<<"}"<<std::endl;
        return clean;
    }
};
struct ProcessingStop {
    IAudioProcessor& processor;
    std::exception_ptr& failure;
    bool started=false;
    ~ProcessingStop() {
        if(started) try {ok(callback([&]{return processor.setProcessing(false);}),"processing stop");}
        catch(...) {if(!failure) failure=std::current_exception();}
    }
};
void synchronize(IEditController& controller,LVBState::Stream& state,double gain,double colour) {
    state.position=0;ok(controller.setComponentState(&state),"component/controller synchronization");
    need(controller.getParamNormalized(0)==gain&&controller.getParamNormalized(1)==colour,"recognizable state readback");
}
}
int main(int argc,char** argv) {
    try {
        need(argc==8||argc==9,"usage: lifecycle-host BUNDLE instrument|effect record|sibling|recall|migrate|recall-disconnected|migrate-disconnected|abrupt STATE_PREFIX FRAMES NATIVE_PROCESSOR_ID NATIVE_CONTROLLER_ID [CAPTURE_PREFIX]");
        bool instrument=std::string(argv[2])=="instrument",record=std::string(argv[3])=="record"||std::string(argv[3])=="sibling";
        need(instrument||std::string(argv[2])=="effect","fixture role");
        const std::string mode=argv[3];
        const bool migrate=mode=="migrate"||mode=="migrate-disconnected";
        const bool disconnected=mode=="recall-disconnected"||mode=="migrate-disconnected";
        const bool abrupt=mode=="abrupt";
        const bool sibling=mode=="sibling";
        need(record||mode=="recall"||mode=="recall-disconnected"||migrate||abrupt,"fixture mode");
        need(!migrate||argc==9,"migration needs a separate capture destination");
        const int frames=std::stoi(argv[5]);need(frames==1008||frames==1024,"declared callback sizes");
        const std::string prefix=argv[4];
        auditBegin=reinterpret_cast<Mark>(dlsym(RTLD_DEFAULT,"ap3_audit_begin"));
        auditEnd=reinterpret_cast<Count>(dlsym(RTLD_DEFAULT,"ap3_audit_end"));
        need(bool(auditBegin)==bool(auditEnd),"complete callback audit interface");
        if(auditBegin) {
            auto allocate=reinterpret_cast<void*(*)(size_t)>(dlsym(RTLD_DEFAULT,"malloc"));
            auto release=reinterpret_cast<void(*)(void*)>(dlsym(RTLD_DEFAULT,"free"));
            auditBegin();auto memory=allocate(32);release(memory);need(auditEnd()>=2,"audit positive control");
        }
        auto at=Clock::now();std::string error;
        auto module=VST3::Hosting::Module::create(argv[1],error);need(bool(module),"SDK module load");
        auto host=owned(new HostApplication);module->getFactory().setHostContext(host);
        auto classes=module->getFactory().classInfos();need(classes.size()==2,"exact two-class fixture factory");
        need(classes[0].ID().toString()==argv[6],"exact native processor class");
        need(classes[1].ID().toString()==argv[7],"exact native controller class");
        Lifecycle life;
        life.unload=[&](bool clean) {
            if(clean) {host=nullptr;module.reset();}
            else {[[maybe_unused]] auto* retained=new decltype(module)(std::move(module));}
        };
        auto& component=life.component;auto& controller=life.controller;
        auto& processor=life.processor;auto& cp=life.cp;auto& cc=life.cc;
        component=module->getFactory().createInstance<IComponent>(classes[0].ID());need(bool(component),"component factory");
        ok(component->initialize(host),"component initialize");life.componentInitialized=true;
        processor=FUnknownPtr<IAudioProcessor>(component);need(bool(processor),"processor interface");
        TUID controllerId{};ok(component->getControllerClassId(controllerId),"controller identity");
        char8 controllerText[33]{};FUID(controllerId).toString(controllerText);
        need(classes[1].ID().toString()==controllerText,"exact controller factory identity");
        controller=module->getFactory().createInstance<IEditController>(classes[1].ID());need(bool(controller),"controller factory");
        ok(controller->initialize(host),"controller initialize");life.controllerInitialized=true;
        cp=FUnknownPtr<IConnectionPoint>(component);cc=FUnknownPtr<IConnectionPoint>(controller);need(cp&&cc,"connection interfaces");
        auto connect=[&] {
            if(disconnected) {ok(cc->connect(cp),"controller connect");life.controllerConnected=true;}
            ok(cp->connect(cc),"component connect");life.componentConnected=true;
            if(!disconnected) {ok(cc->connect(cp),"controller connect");life.controllerConnected=true;}
        };
        if(!disconnected) connect();
        ParameterInfo first{},second{};
        const auto parameterCount=controller->getParameterCount();
        need(parameterCount==2||parameterCount==3,"declared fixture parameters");
        if(parameterCount==3) {ParameterInfo added{};ok(controller->getParameterInfo(2,added),"added parameter metadata");need(added.id==17,"stable added parameter identity");}
        ok(controller->getParameterInfo(0,first),"level metadata");ok(controller->getParameterInfo(1,second),"colour metadata");
        need(first.id==0&&second.id==1,"parameter identities");
        stage="initialized_state";LVBState::Stream initial;
        ok(component->getState(&initial),"state before processing");
        if(disconnected) {
            // Before connection the native controller has no authoritative
            // processor readback. Exercise deferred synchronization without
            // asserting that descriptor defaults are a restored state.
            initial.position=0;ok(controller->setComponentState(&initial),"deferred initial synchronization");
        } else synchronize(*controller,initial,.25,.5);
        std::cout<<"{\"event\":\"initialized_state\",\"bytes\":"<<initial.bytes.size()<<",\"elapsed_ms\":"<<std::chrono::duration_cast<std::chrono::milliseconds>(Clock::now()-at).count()<<"}"<<std::endl;
        double initialGain=.25,initialColour=.5;
        if(abrupt) {std::cout<<"{\"event\":\"abrupt_exit\",\"sdk_teardown\":false}"<<std::endl;std::_Exit(23);}
        if(!record) {
            stage="restore";LVBState::Stream saved(load(prefix+".component"));
            ok(component->setState(&saved),"component restore");
            saved.position=0;ok(controller->setComponentState(&saved),"component/controller synchronization");
            if(disconnected) connect();
            need(controller->getParamNormalized(0)==.625&&controller->getParamNormalized(1)==.125,"migrated current controller values");
            if(parameterCount==3) need(controller->getParamNormalized(17)==.75,"new parameter comes from vendor migration, not descriptor default");
            LVBState::Stream control(load(prefix+".controller"));ok(controller->setState(&control),"controller restore");
            need(controller->getParamNormalized(0)==.625&&controller->getParamNormalized(1)==.125,"restored controller values");
            initialGain=.625;initialColour=.125;
        }
        need(component->getBusCount(kAudio,kInput)==(instrument?0:1)&&component->getBusCount(kAudio,kOutput)==1,"exact audio buses");
        SpeakerArrangement stereo=SpeakerArr::kStereo;
        ok(processor->setBusArrangements(instrument?nullptr:&stereo,instrument?0:1,&stereo,1),"stereo arrangement");
        ProcessSetup setup{kRealtime,kSample32,1024,48000.};ok(processor->setupProcessing(setup),"setup processing");
        if(!instrument) ok(component->activateBus(kAudio,kInput,0,true),"audio input active");
        else ok(component->activateBus(kEvent,kInput,0,true),"notes active");
        ok(component->activateBus(kAudio,kOutput,0,true),"audio output active");
        stage="activation";ok(component->setActive(true),"Windows activation");life.active=true;
        const auto latency=processor->getLatencySamples();need(latency==1024,"declared existing buffering");
        std::cout<<"{\"event\":\"activated\",\"latency_samples\":"<<latency<<",\"elapsed_ms\":"<<std::chrono::duration_cast<std::chrono::milliseconds>(Clock::now()-at).count()<<"}"<<std::endl;
        std::atomic<int> blocks{0};std::atomic<bool> audioDone{false};std::exception_ptr audioFailure;
        uint64_t mismatches=0,nonfinite=0,nonzero=0,rejected=0,overruns=0,maxNs=0;double maxError=0.;
        const int count=sibling?2400:480;
        const int releaseBlock=sibling?count-30:350;
        // Bounded independent-host observations. No logging or allocation is
        // added inside process(); all rows are emitted after the audio join.
        struct Timing {uint64_t scheduledNs{},startedNs{},durationNs{},mismatches{};};
        std::array<Timing,2400> timing{};
        Clock::time_point audioBegin;
        uint64_t audioBeginUnixNs=0;
        std::jthread audio([&]{ProcessingStop stop{*processor,audioFailure};try {
            std::array<std::array<float,1024>,2> input{},output{};
            float* in[]{input[0].data(),input[1].data()};float* out[]{output[0].data(),output[1].data()};
            AudioBusBuffers ib{},ob{};ib.numChannels=ob.numChannels=2;ib.channelBuffers32=in;ob.channelBuffers32=out;
            ParameterChanges parameters(3);EventList notes(2);
            ProcessData data{};data.processMode=kRealtime;data.symbolicSampleSize=kSample32;data.numSamples=frames;
            data.numInputs=instrument?0:1;data.inputs=instrument?nullptr:&ib;data.numOutputs=1;data.outputs=&ob;
            data.inputParameterChanges=&parameters;data.inputEvents=instrument?&notes:nullptr;
            std::vector<std::array<float,2>> expected(size_t(count*frames+latency));
            double gain=initialGain,colour=initialColour,trim=record?.25:.75,phase=0.;bool voice=false;
            const double tau=6.2831853071795864769,increment=tau*440./48000.;
            input[0].fill(.25f);input[1].fill(-.125f);
            ok(callback([&]{return processor->setProcessing(true);}),"processing start");stop.started=true;
            auto begin=Clock::now();
            audioBegin=begin;
            audioBeginUnixNs=uint64_t(std::chrono::duration_cast<std::chrono::nanoseconds>(
                std::chrono::system_clock::now().time_since_epoch()).count());
            for(int b=0;b<count;++b) {
                parameters.clearQueue();notes.clear();
                int32 index=0,point=0;
                if(record&&b==4) {
                    auto* queue=parameters.addParameterData(0,index);
                    ok(queue->addPoint(7,.75,point),"gain automation first");
                    ok(queue->addPoint(37,.5,point),"gain automation middle");
                    ok(queue->addPoint(1007,.625,point),"gain automation whole block");
                }
                if(record&&b==5) {
                    ok(parameters.addParameterData(1,index)->addPoint(49,.125,point),"colour automation");
                    if(parameterCount==3) ok(parameters.addParameterData(17,index)->addPoint(49,.75,point),"added parameter automation");
                }
                if(instrument&&(b==3||b==releaseBlock)) {
                    Event event{};event.busIndex=0;event.sampleOffset=b==3?7:43;
                    event.type=b==3?Event::kNoteOnEvent:Event::kNoteOffEvent;
                    if(b==3) event.noteOn={2,69,0.f,.8f,0,42};else event.noteOff={2,69,0.f,42,0.f};
                    ok(notes.addEvent(event),"note event");
                }
                for(int i=0;i<frames;++i) {
                    if(record&&b==4) {if(i==7) gain=.75;if(i==37) gain=.5;if(i==1007) gain=.625;}
                    if(record&&b==5&&i==49) {colour=.125;trim=.75;}
                    if(instrument&&b==3&&i==7) {voice=true;phase=0.;}
                    if(instrument&&b==releaseBlock&&i==43) voice=false;
                    double signal=voice?double(.8f)*(std::sin(phase)+colour*.5*std::sin(phase*2.)):0.;
                    if(voice) phase=std::fmod(phase+increment,tau);
                    auto& e=expected[size_t(b*frames+i+latency)];
                    const auto factor=parameterCount==3?.75+trim:1.;
                    e=instrument?std::array<float,2>{float(factor*gain*signal/16.),float(factor*gain*signal/16.)}
                                :std::array<float,2>{float(factor*gain*.25*(.5+colour)),float(factor*gain*-.125*(.5+colour))};
                }
                const auto scheduledNs=uint64_t(b)*uint64_t(frames)*1000000000ULL/48000;
                std::this_thread::sleep_until(begin+std::chrono::nanoseconds(scheduledNs));
                auto before=Clock::now();auto result=callback([&]{return processor->process(data);});
                auto elapsed=uint64_t(std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now()-before).count());
                auto& row=timing[size_t(b)];row.scheduledNs=scheduledNs;
                row.startedNs=uint64_t(std::chrono::duration_cast<std::chrono::nanoseconds>(before-begin).count());
                row.durationNs=elapsed;
                maxNs=std::max(maxNs,elapsed);overruns+=elapsed>uint64_t(frames)*1000000000ULL/48000;
                if(result!=kResultOk) {++rejected;throw std::runtime_error("processing refused");}
                const auto priorMismatches=mismatches;
                for(int i=0;i<frames;++i) for(int ch=0;ch<2;++ch) {
                    auto actual=out[ch][i];auto difference=std::abs(double(actual)-expected[size_t(b*frames+i)][ch]);
                    nonfinite+=!std::isfinite(actual);nonzero+=actual!=0.;maxError=std::max(maxError,difference);
                    mismatches+=difference>1e-7;
                }
                row.mismatches=mismatches-priorMismatches;
                blocks.store(b+1,std::memory_order_release);
            }
        } catch(...) {audioFailure=std::current_exception();}audioDone.store(true,std::memory_order_release);});
        LVBState::Stream captured,control;std::exception_ptr captureFailure;
        while(blocks.load(std::memory_order_acquire)<96&&!audioDone.load(std::memory_order_acquire)) std::this_thread::sleep_for(std::chrono::milliseconds(10));
        try {
            stage="processing_state";need(!audioDone.load(std::memory_order_acquire),"audio remains active for capture");
            auto captureAt=Clock::now();const auto captureBeginBlock=blocks.load(std::memory_order_acquire);
            ok(component->getState(&captured),"state during processing");
            synchronize(*controller,captured,.625,.125);ok(controller->getState(&control),"controller state capture");
            if(!record&&!migrate) {
                need(captured.bytes==load(prefix+".component"),"byte-identical component state roundtrip");
                need(control.bytes==load(prefix+".controller"),"byte-identical controller state roundtrip");
            }
            const auto captureEnd=Clock::now();
            std::cout<<"{\"event\":\"processing_state\",\"component_bytes\":"<<captured.bytes.size()<<",\"controller_bytes\":"<<control.bytes.size()<<",\"gain\":0.625,\"colour\":0.125,\"duration_ms\":"<<std::chrono::duration_cast<std::chrono::milliseconds>(captureEnd-captureAt).count()
                     <<",\"begin_since_audio_start_ns\":"<<std::chrono::duration_cast<std::chrono::nanoseconds>(captureAt-audioBegin).count()
                     <<",\"end_since_audio_start_ns\":"<<std::chrono::duration_cast<std::chrono::nanoseconds>(captureEnd-audioBegin).count()
                     <<",\"begin_after_block\":"<<captureBeginBlock<<",\"end_after_block\":"<<blocks.load(std::memory_order_acquire)<<"}"<<std::endl;
        }catch(...) {captureFailure=std::current_exception();}
        audio.join();
        std::cout<<"{\"event\":\"consumer_timing\",\"schema\":1,\"audio_begin_monotonic_ns\":"<<std::chrono::duration_cast<std::chrono::nanoseconds>(audioBegin.time_since_epoch()).count()
                 <<",\"audio_begin_unix_ns_approximate\":"<<audioBeginUnixNs<<",\"blocks\":[";
        for(int b=0;b<blocks.load();++b) {
            const auto& row=timing[size_t(b)];if(b) std::cout<<',';
            std::cout<<"{\"block\":"<<b<<",\"scheduled_ns\":"<<row.scheduledNs<<",\"started_ns\":"<<row.startedNs
                     <<",\"callback_ns\":"<<row.durationNs<<",\"mismatches\":"<<row.mismatches<<'}';
        }
        std::cout<<"]}"<<std::endl;
        std::cout<<"{\"event\":\"audio\",\"role\":\""<<(instrument?"instrument":"effect")<<"\",\"frames_per_callback\":"<<frames<<",\"blocks\":"<<blocks<<",\"compared_samples\":"<<uint64_t(blocks)*uint64_t(frames)*2<<",\"mismatches\":"<<mismatches<<",\"nonfinite\":"<<nonfinite<<",\"nonzero\":"<<nonzero<<",\"max_error\":"<<maxError<<",\"rejected_callbacks\":"<<rejected<<",\"callback_max_ns\":"<<maxNs<<",\"callback_overruns\":"<<overruns<<",\"callback_audited\":"<<(auditBegin?"true":"false")<<"}"<<std::endl;
        stage="retirement";
        auto retired=life.finish();
        if(audioFailure) {stage="processing";std::rethrow_exception(audioFailure);}
        if(captureFailure) {stage="processing_state";std::rethrow_exception(captureFailure);}
        need(retired,"SDK lifecycle cleanup failed");
        need(mismatches==0&&nonfinite==0&&rejected==0&&nonzero>0,"exact installed audio comparison");
        if(record||migrate) {
            const std::string destination=migrate?argv[8]:prefix;
            need(!migrate||destination!=prefix,"original saved object must remain unchanged");
            save(destination+".component",captured);save(destination+".controller",control);
        }
        std::cout<<"{\"event\":\"passed\",\"claim\":\"installed_sdk_development_regression\",\"real_daw_project_claim\":false}"<<std::endl;
        return 0;
    }catch(const std::exception& error) {
        std::cout<<"{\"event\":\"failed\",\"stage\":\""<<stage<<"\",\"reason\":\""<<error.what()<<"\"}"<<std::endl;return 1;
    }
}
