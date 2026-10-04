// Run the independent consumer's host oracle against the actual original
// first-party SDK producer, and deliberately corrupt returned host observations.
// This is a functional source regression, not transport/timing qualification.
#define LVB_BETA_COMPLETION 1
#include "../../windows-fixtures/beta/stateful.cpp"
#define main completion_consumer_entry
#include "completion_host.cpp"
#undef main
#include <cstring>
#include <filesystem>
#include <limits>
#include <sys/stat.h>
#include <vector>
template<class Operation> static void refuses(Operation operation,const char* why) {
    bool failed = false;
    try { operation(); } catch (const std::exception&) { failed = true; }
    need(failed,why);
}
enum class LifecycleRefusal { None, Stop, ThrowStop, Deactivate, DeactivateOnce, ThrowDeactivateOnce, Disconnect, ComponentTerminate, ControllerTerminate };
struct LifecycleCounts {
    bool processing=false;
    int start=0;
    int stop=0,deactivate=0,componentDisconnect=0,controllerDisconnect=0,componentTerminate=0,controllerTerminate=0;
    int releases=0,componentDestruction=0,controllerDestruction=0,unloads=0;
    std::array<int,32> order{};size_t orderCount=0;
    void record(int event) { need(orderCount<order.size(),"bounded injected lifecycle order");order[orderCount++]=event; }
};
class RefusingProcessor final:public AudioEffect {
    std::shared_ptr<LifecycleCounts> counts;
    LifecycleRefusal refusal;
public:
    RefusingProcessor(std::shared_ptr<LifecycleCounts> observed,LifecycleRefusal injected):counts(std::move(observed)),refusal(injected) {}
    ~RefusingProcessor() override { ++counts->componentDestruction; }
    uint32 PLUGIN_API release() override { counts->record(3);++counts->releases; return AudioEffect::release(); }
    tresult PLUGIN_API setProcessing(TBool state) override {
        if(state){++counts->start;counts->processing=true;return kResultOk;}
        counts->record(1);++counts->stop;
        if(refusal==LifecycleRefusal::ThrowStop)throw std::runtime_error("original thrown stop failure");
        if(refusal==LifecycleRefusal::Stop)return kResultFalse;
        counts->processing=false;return kResultOk;
    }
    tresult PLUGIN_API setActive(TBool state) override {
        if(state)return kResultOk;
        counts->record(2);++counts->deactivate;need(!counts->processing,"stop must precede deactivation");
        if(refusal==LifecycleRefusal::ThrowDeactivateOnce&&counts->deactivate==1)throw std::runtime_error("original thrown inactive transition");
        return refusal==LifecycleRefusal::Deactivate||(refusal==LifecycleRefusal::DeactivateOnce&&counts->deactivate==1)?kResultFalse:kResultOk;
    }
    tresult PLUGIN_API setupProcessing(ProcessSetup&) override { return kResultOk; }
    uint32 PLUGIN_API getLatencySamples() override { return vendorLatency; }
    tresult PLUGIN_API disconnect(IConnectionPoint*) override { ++counts->componentDisconnect;return kResultOk; }
    tresult PLUGIN_API terminate() override { ++counts->componentTerminate;return refusal==LifecycleRefusal::ComponentTerminate?kResultFalse:kResultOk; }
};
class RefusingController final:public EditController {
    std::shared_ptr<LifecycleCounts> counts;
    LifecycleRefusal refusal;
public:
    RefusingController(std::shared_ptr<LifecycleCounts> observed,LifecycleRefusal injected):counts(std::move(observed)),refusal(injected) {}
    ~RefusingController() override { ++counts->controllerDestruction; }
    uint32 PLUGIN_API release() override { counts->record(3);++counts->releases; return EditController::release(); }
    tresult PLUGIN_API disconnect(IConnectionPoint*) override { ++counts->controllerDisconnect;return refusal==LifecycleRefusal::Disconnect?kResultFalse:kResultOk; }
    tresult PLUGIN_API terminate() override { ++counts->controllerTerminate;return refusal==LifecycleRefusal::ControllerTerminate?kResultFalse:kResultOk; }
};
LifetimePtr injectedLifetime(const std::shared_ptr<LifecycleCounts>& counts,LifecycleRefusal refusal) {
    auto owner=lifetime();owner->host=owned(new HostApplication);auto& life=owner->life;
    life.component=owned(new RefusingProcessor(counts,refusal));life.controller=owned(new RefusingController(counts,refusal));
    life.processor=FUnknownPtr<IAudioProcessor>(life.component);life.cp=FUnknownPtr<IConnectionPoint>(life.component);life.cc=FUnknownPtr<IConnectionPoint>(life.controller);
    need(life.processor&&life.cp&&life.cc,"injected ordinary SDK interfaces");
    life.processing=life.active=life.componentConnected=life.controllerConnected=life.componentInitialized=life.controllerInitialized=true;
    life.unload=[counts](bool clean){need(clean,"no failed lifetime unload");++counts->unloads;};
    counts->processing=true;counts->releases=0;counts->orderCount=0;return owner;
}
void lifecycleRefusals() {
    auditBegin=[]{};auditEnd=[]()->uint64_t{return 0;};
    for(auto refusal:{LifecycleRefusal::Stop,LifecycleRefusal::ThrowStop,LifecycleRefusal::Deactivate,LifecycleRefusal::Disconnect,LifecycleRefusal::ComponentTerminate,LifecycleRefusal::ControllerTerminate}) {
        auto counts=std::make_shared<LifecycleCounts>();Lifetime* retained=nullptr;std::string original;
        try {
            auto owner=injectedLifetime(counts,refusal);retained=owner.get();
            if(refusal==LifecycleRefusal::Stop||refusal==LifecycleRefusal::ThrowStop)owner->life.stop();
            throw std::runtime_error("original setup exception before processing try");
        }catch(const std::exception& error){original=error.what();}
        const auto expected=refusal==LifecycleRefusal::Stop?"processing stop":refusal==LifecycleRefusal::ThrowStop?"original thrown stop failure":"original setup exception before processing try";
        need(original==expected,"retirement must preserve original setup/stop failure");
        need(retained&&!retained->life.clean&&!retained->life.finish(),"failed retirement retained");
        need(retained->host&&retained->life.component&&retained->life.controller&&retained->life.processor&&retained->life.cp&&retained->life.cc,"entire failed lifetime interface/host ownership retained");
        need(counts->releases==0&&counts->componentDestruction==0&&counts->controllerDestruction==0&&counts->unloads==0,"failed lifetime must not release SDK objects or unload their image");
        need(counts->stop==1,"failed explicit stop must not be retried during unwind or finish");
        if(refusal==LifecycleRefusal::Stop||refusal==LifecycleRefusal::ThrowStop) {
            need(retained->life.processing&&counts->deactivate==0&&counts->controllerDisconnect==0&&counts->componentTerminate==0,"refused stop remains owned and forbids later cleanup");
            need(retained->life.stopped.result==(refusal==LifecycleRefusal::Stop?kResultFalse:kInternalError),"original stop result retained");
        }else if(refusal==LifecycleRefusal::Deactivate)need(retained->life.active&&counts->controllerDisconnect==0&&counts->componentTerminate==0,"refused deactivation remains owned");
        else if(refusal==LifecycleRefusal::Disconnect)need(retained->life.controllerConnected&&counts->componentDisconnect==0&&counts->componentTerminate==0,"refused disconnect prevents later teardown");
        else if(refusal==LifecycleRefusal::ComponentTerminate)need(retained->life.componentInitialized&&counts->controllerTerminate==0,"refused component terminate prevents later cleanup");
        else need(retained->life.controllerInitialized&&retained->life.controllerTerminated.result==kResultFalse,"refused controller terminate remains owned");
    }
    auto counts=std::make_shared<LifecycleCounts>();auto clean=injectedLifetime(counts,LifecycleRefusal::None);clean.reset();
    need(counts->componentDestruction==1&&counts->controllerDestruction==1&&counts->releases>0&&counts->unloads==1,"positive completed retirement releases exact SDK owner");
}
static uint64_t startAuditTurns=0;
uint64_t oneStartAuditFailure() { return ++startAuditTurns==1?1:0; }
void configureOwnershipRefusals() {
    Options o{};o.block=64;o.rate=48000;o.delay=0;
    auditBegin=[]{};auditEnd=[]()->uint64_t{return 0;};
    for(auto refusal:{LifecycleRefusal::DeactivateOnce,LifecycleRefusal::ThrowDeactivateOnce}) {
        auto counts=std::make_shared<LifecycleCounts>();auto owner=injectedLifetime(counts,refusal);auto* retained=owner.get();
        Exercise run(owner->life,o);std::string original;
        try{run.configure(kOffline,o.block,o.rate);}catch(const std::exception& error){original=error.what();}
        need(original==(refusal==LifecycleRefusal::DeactivateOnce?"inactive transition":"original thrown inactive transition"),"actual configure preserves its first inactive failure");
        need(owner->life.active&&!owner->life.clean&&counts->deactivate==1,"failed explicit reconfiguration remains active");
        owner.reset();
        need(!retained->life.finish()&&retained->life.active&&counts->deactivate==1,"first refused deactivation must not be retried even when second call would succeed");
        need(counts->start==0&&counts->controllerDisconnect==0&&counts->componentTerminate==0&&counts->releases==0&&counts->componentDestruction==0&&counts->controllerDestruction==0&&counts->unloads==0,"failed reconfiguration retains original SDK lifetime without later teardown");
    }
    auto counts=std::make_shared<LifecycleCounts>();auto owner=injectedLifetime(counts,LifecycleRefusal::None);
    owner->life.processing=owner->life.active=false;counts->processing=false;
    Exercise run(owner->life,o);startAuditTurns=0;auditEnd=oneStartAuditFailure;std::string original;
    try{run.configure(kOffline,o.block,o.rate);}catch(const std::exception& error){original=error.what();}
    need(original=="start callback effects"&&counts->start==1&&owner->life.processing&&counts->processing,"successful SDK start ownership must precede failed audit assertion");
    need(counts->stop==0&&counts->deactivate==0,"successful audited start has not been silently retired");
    owner.reset();auditEnd=[]()->uint64_t{return 0;};
    need(counts->stop==1&&counts->deactivate==1&&!counts->processing&&counts->orderCount>=3&&counts->order[0]==1&&counts->order[1]==2&&counts->order[2]==3,"one successful stop precedes deactivation and every SDK release after audited start failure");
    need(counts->componentDestruction==1&&counts->controllerDestruction==1&&counts->unloads==1,"audited start failure permits release only after completed lifecycle");
}
void overlapRefusals() {
    constexpr uint64_t begin=1000000000ULL,end=begin+200000000ULL;
    Row overlapping{};overlapping.begin=begin+50000000ULL;overlapping.end=begin+12000000000ULL;
    need(captureOverlaps(begin,end,overlapping),"actual prompt capture inside slow audio interval accepted");
    auto scheduledLater=overlapping;scheduledLater.begin=end+1;
    need(!captureOverlaps(begin,end,scheduledLater),"unfinished thread beginning after capture return must refuse despite twelve-second audio end");
    auto reversed=overlapping;reversed.begin=begin-1;
    need(!captureOverlaps(begin,end,reversed),"intended capture issuance ordering required");
    auto alreadyEnded=overlapping;alreadyEnded.end=end;
    need(!captureOverlaps(begin,end,alreadyEnded),"callback must still be in flight at capture return");
    need(!captureOverlaps(begin,begin+10000000000ULL,overlapping),"original capture deadline cannot be renewed");
}
std::vector<uint8_t> completionEnvelope(const Options& o,double operation) {
    constexpr size_t header=104;std::vector<uint8_t> payload;
    auto append32=[&](uint32_t value){for(size_t i=0;i<4;++i)payload.push_back(uint8_t(value>>(i*8)));};
    auto append64=[&](double value){const auto begin=payload.size();payload.resize(begin+8);std::memcpy(payload.data()+begin,&value,8);};
    std::array<uint8_t,24> vendor{'L','V','B','B',1,uint8_t(o.instrument),0,0};
    const double gain=.625,colour=.125;std::memcpy(vendor.data()+8,&gain,8);std::memcpy(vendor.data()+16,&colour,8);
    append32(24);append32(24);append32(4);append32(3);
    payload.insert(payload.end(),vendor.begin(),vendor.end());payload.insert(payload.end(),vendor.begin(),vendor.end());
    for(const auto& [id,value]:std::array<std::pair<uint32_t,double>,4>{{{0,.625},{1,.125},{31,operation},{32,0.}}}) {
        append32(id);append32(1);append64(value);
    }
    std::vector<uint8_t> envelope(header);std::copy_n(reinterpret_cast<const uint8_t*>("LVBSTATE"),8,envelope.begin());
    auto put32=[&](size_t offset,uint32_t value){for(size_t i=0;i<4;++i)envelope[offset+i]=uint8_t(value>>(i*8));};
    put32(8,3);put32(12,header);
    const auto fixtureClass=completionFixtureClass(o.instrument);
    std::copy(fixtureClass.begin(),fixtureClass.end(),envelope.begin()+16);
    std::fill(envelope.begin()+32,envelope.begin()+64,0x5a);put32(64,uint32_t(payload.size()));
    const auto digest=stateSha256(payload.data(),payload.size());std::copy(digest.begin(),digest.end(),envelope.begin()+72);
    envelope.insert(envelope.end(),payload.begin(),payload.end());return envelope;
}
void resign(std::vector<uint8_t>& envelope) {
    const auto digest=stateSha256(envelope.data()+104,envelope.size()-104);
    std::copy(digest.begin(),digest.end(),envelope.begin()+72);
}
void slowStateRefusals(const Options& o) {
    const std::array<uint8_t,3> abc{'a','b','c'};
    const std::array<uint8_t,32> abcDigest{0xba,0x78,0x16,0xbf,0x8f,0x01,0xcf,0xea,0x41,0x41,0x40,0xde,0x5d,0xae,0x22,0x23,
                                                  0xb0,0x03,0x61,0xa3,0x96,0x17,0x7a,0x9c,0xb4,0x10,0xff,0x61,0xf2,0x00,0x15,0xad};
    need(stateSha256(abc.data(),abc.size())==abcDigest,"independent SHA-256 known answer");
    const auto original=completionEnvelope(o,1.);const auto recalled=completionEnvelope(o,.25);
    const std::array<uint8_t,32> payloadDigest=o.instrument
        ?std::array<uint8_t,32>{0xbc,0x98,0xd3,0xd3,0x76,0x4c,0x39,0x02,0xed,0xbf,0x59,0x8d,0xa9,0x97,0xe4,0x57,
                                0x34,0x3f,0x8d,0xec,0xc5,0x14,0xb3,0x06,0xd8,0x82,0x00,0x47,0x70,0x77,0x33,0x6e}
        :std::array<uint8_t,32>{0x9a,0x79,0xb8,0x54,0xfb,0x31,0xbe,0x8f,0x49,0x64,0xad,0xf4,0xa8,0x4f,0x8d,0x20,
                                0x6e,0x21,0x09,0xf7,0x4c,0x7d,0x7d,0x8b,0x3f,0xb5,0x91,0x62,0x59,0xe3,0xe5,0x96};
    need(stateSha256(original.data()+104,original.size()-104)==payloadDigest,"independent 128-byte fixture SHA-256 known answer");
    slowStateRecall(o,original,recalled);
    need(original==completionEnvelope(o,1.),"slow observer leaves original saved snapshot untouched");
    auto wrongDurable=recalled;const double wrongGain=.5;std::memcpy(wrongDurable.data()+104+16+8,&wrongGain,8);resign(wrongDurable);
    refuses([&]{slowStateRecall(o,original,wrongDurable);},"wrong durable vendor value refused");
    const auto recalledView=completionState(recalled,o);
    auto wrongMirror=recalled;std::memcpy(wrongMirror.data()+recalledView.valueOffsets[0],&wrongGain,8);resign(wrongMirror);
    refuses([&]{slowStateRecall(o,original,wrongMirror);},"wrong durable mirror value refused");
    auto wrongIdentity=recalled;wrongIdentity[recalledView.valueOffsets[2]-8]=30;resign(wrongIdentity);
    refuses([&]{slowStateRecall(o,original,wrongIdentity);},"wrong mirror identity refused");
    auto duplicateIdentity=recalled;duplicateIdentity[recalledView.valueOffsets[2]-8]=1;resign(duplicateIdentity);
    refuses([&]{slowStateRecall(o,original,duplicateIdentity);},"duplicate mirror identity refused");
    auto wrongClass=recalled;wrongClass[16]^=1;
    refuses([&]{slowStateRecall(o,original,wrongClass);},"wrong class provenance refused");
    auto proxyAsVendor=recalled;auto nibble=[](char ch)->uint8_t{return uint8_t(ch<='9'?ch-'0':ch-'A'+10);};
    for(size_t i=0;i<16;++i)proxyAsVendor[16+i]=uint8_t((nibble(o.processor[i*2])<<4)|nibble(o.processor[i*2+1]));
    refuses([&]{slowStateRecall(o,original,proxyAsVendor);},"native proxy identity is not the Windows fixture class");
    auto wrongModule=recalled;wrongModule[32]^=1;
    refuses([&]{slowStateRecall(o,original,wrongModule);},"wrong module provenance refused");
    auto wrongOperation=recalled;const double stale=.5;std::memcpy(wrongOperation.data()+recalledView.valueOffsets[2],&stale,8);resign(wrongOperation);
    refuses([&]{slowStateRecall(o,original,wrongOperation);},"wrong fresh transient value refused");
    auto malformed=recalled;malformed.pop_back();
    refuses([&]{slowStateRecall(o,original,malformed);},"malformed envelope extent refused");
    auto corruptDigest=recalled;corruptDigest[72]^=1;
    refuses([&]{slowStateRecall(o,original,corruptDigest);},"malformed envelope digest refused");
    refuses([&]{need(original==recalled,"byte-exact component state recall");},"deterministic recall keeps exact full-byte assertion");
}
void originalStateRefusals() {
    const std::vector<uint8_t> saved{1,2,3};LVBState::Stream state(saved);
    originalStateRetained(state,saved);
    state.failed=true;refuses([&]{originalStateRetained(state,saved);},"failed original stream operation refused");state.failed=false;
    state.addRef();refuses([&]{originalStateRetained(state,saved);},"retained original stream reference refused");state.release();
    state.bytes[0]^=1;refuses([&]{originalStateRetained(state,saved);},"mutated original saved bytes refused");
}
struct PrivateOutputTest {
    std::string directory,prefix;
    PrivateOutputTest() {
        directory=(std::filesystem::temp_directory_path()/"lvb-completion-output-XXXXXX").string();
        need(::mkdtemp(directory.data())!=nullptr,"private owned output test directory");
        prefix=directory+"/run";
    }
    ~PrivateOutputTest() { std::error_code ignored;std::filesystem::remove_all(directory,ignored); }
};
void privateGateInputs(const Options& base) {
    need(factoryWaitBound==std::chrono::seconds(9750),"factory hold covers the declared six-class control-plane update envelope");
    PrivateOutputTest location;Options o=base;o.prefix=location.prefix.c_str();o.capturePrefix=o.prefix;
    need(!factoryReleased(o),"absent release token remains pending");
    static constexpr std::array<uint8_t,8> release{'r','e','l','e','a','s','e','\n'};
    writePrivate(location.prefix+".factory-release",release.data(),release.size(),"test release token");
    holdFactory(o);
    const auto ready=readPrivate(location.prefix+".factory-ready",512,false,"factory-ready test read");
    need(ready&&std::string_view(reinterpret_cast<const char*>(ready->data()),ready->size()).find("\"factory_cached\":true")!=std::string_view::npos
        &&std::string_view(reinterpret_cast<const char*>(ready->data()),ready->size()).find("\"wait_bound_seconds\":9750")!=std::string_view::npos
        &&factoryReleased(o),"cached factory readiness and exact release token");
    refuses([&]{holdFactory(o);},"factory-ready evidence cannot be overwritten");
    PrivateOutputTest malformed;Options bad=base;bad.prefix=malformed.prefix.c_str();bad.capturePrefix=bad.prefix;
    const std::array<uint8_t,8> wrong{'r','e','l','e','a','s','e','!'};
    writePrivate(malformed.prefix+".factory-release",wrong.data(),wrong.size(),"malformed release fixture");
    refuses([&]{factoryReleased(bad);},"malformed release token refused");
    PrivateOutputTest oversized;Options large=base;large.prefix=oversized.prefix.c_str();large.capturePrefix=large.prefix;
    const std::array<uint8_t,9> tooLarge{'r','e','l','e','a','s','e','\n','x'};
    writePrivate(oversized.prefix+".factory-release",tooLarge.data(),tooLarge.size(),"oversized release fixture");
    refuses([&]{factoryReleased(large);},"oversized release token refused before allocation");
    PrivateOutputTest linked;Options link=base;link.prefix=linked.prefix.c_str();link.capturePrefix=link.prefix;
    need(::symlink((location.prefix+".factory-release").c_str(),(linked.prefix+".factory-release").c_str())==0,
         "release symlink fixture");
    refuses([&]{factoryReleased(link);},"release symlink refused");
    PrivateOutputTest publicFile;Options publicOption=base;publicOption.prefix=publicFile.prefix.c_str();publicOption.capturePrefix=publicOption.prefix;
    writePrivate(publicFile.prefix+".factory-release",release.data(),release.size(),"public-mode fixture");
    need(::chmod((publicFile.prefix+".factory-release").c_str(),0644)==0,"public-mode fixture chmod");
    refuses([&]{factoryReleased(publicOption);},"group/world-readable release refused");
    PrivateOutputTest fifo;Options fifoOption=base;fifoOption.prefix=fifo.prefix.c_str();fifoOption.capturePrefix=fifoOption.prefix;
    need(::mkfifo((fifo.prefix+".factory-release").c_str(),0600)==0,"factory release FIFO fixture");
    refuses([&]{factoryReleased(fifoOption);},"factory release FIFO refused without blocking");
}
LifetimePtr actualRecallLifetime(const Options& o,ExternalState& saved) {
    auto owner=lifetime();auto& life=owner->life;owner->host=owned(new HostApplication);
    life.component=owned(new ReferenceProcessor);ok(life.component->initialize(owner->host),"actual recall component initialize");
    life.componentInitialized=true;life.processor=FUnknownPtr<IAudioProcessor>(life.component);need(bool(life.processor),"actual recall processor interface");
    life.controller=owned(new ReferenceController);ok(life.controller->initialize(owner->host),"actual recall controller initialize");
    life.controllerInitialized=true;life.cp=FUnknownPtr<IConnectionPoint>(life.component);life.cc=FUnknownPtr<IConnectionPoint>(life.controller);
    need(life.cp&&life.cc,"actual recall connection interfaces");
    ok(life.cp->connect(life.cc),"actual recall component connect");life.componentConnected=true;
    ok(life.cc->connect(life.cp),"actual recall controller connect");life.controllerConnected=true;
    restoreExternalState(*life.component,*life.controller,saved);
    SpeakerArrangement in=SpeakerArr::kStereo,out[2]{SpeakerArr::kStereo,SpeakerArr::kStereo};
    ok(life.processor->setBusArrangements(o.instrument?nullptr:&in,o.instrument?0:1,out,2),"actual recall bus negotiation");
    return owner;
}
void externalStateActualProducer(const Options& base) {
    PrivateOutputTest location;Options o=base;o.prefix=location.prefix.c_str();o.capturePrefix=o.prefix;
    HostApplication host;ReferenceProcessor producer;ReferenceController producerController;
    ok(producer.initialize(&host),"state source processor initialize");
    ok(producerController.initialize(&host),"state source controller initialize");
    SpeakerArrangement in=SpeakerArr::kStereo,out[2]{SpeakerArr::kStereo,SpeakerArr::kStereo};
    ok(producer.setBusArrangements(o.instrument?nullptr:&in,o.instrument?0:1,out,2),"state source bus negotiation");
    ProcessSetup setup{kOffline,kSample32,64,48000.};ok(producer.setupProcessing(setup),"state source processing setup");
    ok(producer.setActive(true),"state source activation");ok(producer.setProcessing(true),"state source start");
    Buffers initialization(o.instrument);initialization.prepare(0,kOffline,o.instrument,false);
    point(initialization.sent,0,.625);point(initialization.sent,1,.125);
    ok(producer.process(initialization.data),"state source recognizable values");
    ok(producer.setProcessing(false),"state source stop");ok(producer.setActive(false),"state source deactivate");
    LVBState::Stream component;ok(producer.getState(&component),"actual component source capture");
    component.position=0;ok(producerController.setComponentState(&component),"actual source controller synchronization");
    LVBState::Stream controller;ok(producerController.getState(&controller),"actual controller source capture");
    const auto componentBytes=component.bytes,controllerBytes=controller.bytes;
    writeState(o.prefix,".component",component);writeState(o.prefix,".controller",controller);
    struct stat componentInfo{},controllerInfo{};
    need(::stat((location.prefix+".component").c_str(),&componentInfo)==0
        &&::stat((location.prefix+".controller").c_str(),&controllerInfo)==0
        &&(componentInfo.st_mode&0777)==0600&&(controllerInfo.st_mode&0777)==0600,"actual state inputs private");
    ok(producer.terminate(),"state source processor terminate");
    ok(producerController.terminate(),"state source controller terminate");

    ExternalState saved(o);
    {
        auto lost=actualRecallLifetime(o,saved);
        auto wrong=componentBytes;const double wrongGain=.25;std::memcpy(wrong.data()+8,&wrongGain,8);
        LVBState::Stream wrongState(wrong);ok(lost->life.component->setState(&wrongState),"accepted lost-restore negative state");
        Exercise proof(lost->life,o);
        refuses([&]{proof.recalledStateAudio(64,48000);},"first recalled-state audio refuses a lost DSP gain before any parameter edit");
        LVBState::Stream stillWrong;ok(lost->life.component->getState(&stillWrong),"lost-restore negative state capture");
        need(stillWrong.bytes==wrong,"recalled-state proof does not repair the failed restore");
    }
    auto recalled=actualRecallLifetime(o,saved);Exercise proof(recalled->life,o);proof.recalledStateAudio(64,48000);
    need(proof.oracle.mismatches==0&&proof.oracle.nonfinite==0&&proof.oracle.nonzero>0
        &&proof.oracle.returnedEvents>0&&proof.oracle.returnedPoints>0,"actual output/results before any post-restore parameter edit");
    LVBState::Stream current;ok(recalled->life.component->getState(&current),"actual recalled component capture");
    LVBState::Stream currentController;ok(recalled->life.controller->getState(&currentController),"actual recalled controller capture");
    need(current.bytes==componentBytes&&currentController.bytes==controllerBytes,
         "unchanged actual fixture exact state after external recall and output");
    originalStateRetained(saved.component,componentBytes);originalStateRetained(saved.controller,controllerBytes);
    const auto componentAgain=readPrivate(saved.componentPath,LVBState::payloadLimit+LVBState::overhead,false,"source component reread");
    const auto controllerAgain=readPrivate(saved.controllerPath,LVBState::payloadLimit+LVBState::overhead,false,"source controller reread");
    need(componentAgain&&*componentAgain==componentBytes&&controllerAgain&&*controllerAgain==controllerBytes,
         "actual source state files remain byte-exact and untouched");
    refuses([&]{writeState(o.prefix,".component",current);},"external input component cannot be overwritten");
    need(recalled->life.finish(),"actual recalled-state proof retires exactly");
}
void retainedOutput(const OutputCapture& output,const PrivateOutputTest& location,
                    const Row& positive,const Row& corrupted,const Row& failedOracle) {
    need(output.written&&output.writtenBytes==output.count*sizeof(float),"actual complete output artifact written after direct processing stopped");
    std::ifstream file(location.prefix+".output.f32le",std::ios::binary|std::ios::ate);
    need(bool(file)&&file.tellg()==std::streampos(std::streamoff(output.writtenBytes)),"retained file exact whole output byte count");
    file.seekg(0);std::vector<char> bytes(output.writtenBytes);
    file.read(bytes.data(),std::streamsize(bytes.size()));need(bool(file),"retained whole output read");
    need(std::memcmp(bytes.data(),output.values.get(),bytes.size())==0,"whole retained bytes match actual copied producer output");
    need(positive.outputCount==257*4&&corrupted.outputCount==257*4&&failedOracle.outputCount==13*4,
         "actual N and four-lane output row lengths retained");
    need(positive.outputOffset+positive.outputCount<=corrupted.outputOffset
        &&corrupted.outputOffset+corrupted.outputCount==failedOracle.outputOffset,"row offsets correlate positive and failed attempts without overlap");
    float substituted=0.f,nonfinite=0.f;
    std::memcpy(&substituted,bytes.data()+corrupted.outputOffset*sizeof(float),sizeof(float));
    std::memcpy(&nonfinite,bytes.data()+(failedOracle.outputOffset+12*4+3)*sizeof(float),sizeof(float));
    need(substituted==.125f&&std::isnan(nonfinite),"raw bytes preserve corrupt main first frame and nonfinite auxiliary final frame before oracle refusal");
    struct stat info{};need(::stat((location.prefix+".output.f32le").c_str(),&info)==0&&(info.st_mode&0777)==0600,"retained output private mode");
    OutputCapture other;
    refuses([&]{other.prepare(location.prefix.c_str());},"a reused run prefix cannot overwrite retained output");
}
void sustainedMeasurement(const Options& base) {
    need(sustainedRowCapacity(256,0)==4006&&sustainedRowCapacity(128,0)==4006
        &&sustainedRowCapacity(64,0)==4006&&sustainedRowCapacity(256,256)==4007
        &&sustainedRowCapacity(128,256)==4008&&sustainedRowCapacity(64,256)==4010,
        "every declared sustained mode and actual N fits the fixed callback capacity with exact tail calls");
    std::array<Row,maximumCallbacks> rows{};
    for(size_t i=0;i<sustainedCallbacks;++i)rows[i].end=uint64_t(sustainedCallbacks-i);
    const auto statistics=wholeCallbackStatistics(rows,0,sustainedCallbacks,3500);
    need(statistics.callbacks==sustainedCallbacks&&statistics.median==2001
        &&statistics.p99==3960&&statistics.maximum==4000&&statistics.overruns==500,
        "exact bounded sustained whole-callback statistics");
    refuses([&]{wholeCallbackStatistics(rows,maximumCallbacks-1,2,1);},
        "sustained statistics cannot exceed prepared callback capacity");

    Options o=base;o.scenario="sustained";o.block=64;o.rate=48000;o.delay=0;
    auditBegin=[]{};auditEnd=[]()->uint64_t{return 0;};auditLocalWakes=[]()->uint64_t{return 0;};
    auto owner=lifetime();owner->host=owned(new HostApplication);auto& life=owner->life;
    life.component=owned(new ReferenceProcessor);life.processor=FUnknownPtr<IAudioProcessor>(life.component);
    need(bool(life.processor),"actual sustained producer processor");
    ok(life.component->initialize(owner->host),"actual sustained producer initialize");life.componentInitialized=true;
    SpeakerArrangement in=SpeakerArr::kStereo,out[2]{SpeakerArr::kStereo,SpeakerArr::kStereo};
    ok(life.processor->setBusArrangements(o.instrument?nullptr:&in,o.instrument?0:1,out,2),
       "actual sustained producer bus negotiation");
    ProcessSetup initialSetup{kOffline,kSample32,o.block,double(o.rate)};
    ok(life.processor->setupProcessing(initialSetup),"recognizable sustained-state setup");
    ok(life.component->setActive(true),"recognizable sustained-state activate");
    ok(life.processor->setProcessing(true),"recognizable sustained-state start");
    Buffers initial(o.instrument);initial.prepare(0,kOffline,o.instrument,false);
    point(initial.sent,0,.625);point(initial.sent,1,.125);
    ok(life.processor->process(initial.data),"recognizable sustained-state flush");
    initial.guards(0);
    ok(life.processor->setProcessing(false),"recognizable sustained-state stop");
    ok(life.component->setActive(false),"recognizable sustained-state deactivate");
    Exercise run(life,o);run.sustained(o.block,o.rate);
    need(run.sustainedAttempted&&run.sustainedBegin==4&&run.sustainedCount==sustainedCallbacks,
         "exact four setup flushes then 4000 fixed-size sustained callbacks");
    need(run.rowCount==sustainedCallbacks+6&&run.rowCount<=maximumCallbacks,
         "sustained callbacks plus exact D0 latency tail fit prepared storage");
    for(size_t i=0;i<sustainedCallbacks;++i) {
        const auto& row=run.rows[run.sustainedBegin+i];
        need(row.n==o.block&&row.mode==kRealtime&&row.result==kResultOk&&!row.threw,
             "every sustained callback retains exact actual N, mode and result");
    }
    need(run.oracle.mismatches==0&&run.oracle.nonfinite==0&&run.oracle.nonzero>0
        &&run.rejected==0&&run.effects==0,"actual sustained producer output and callback audit pass");
    need(life.finish(),"actual sustained producer retirement");
}
int main() {
    lifecycleRefusals();configureOwnershipRefusals();overlapRefusals();originalStateRefusals();
    const char* args[]{"completion-host","unused.vst3",LVB_BETA_INSTRUMENT?"instrument":"effect","matrix","private-state",
        "257","48000","0",LVB_BETA_INSTRUMENT?"9F2F385EF1995AAFB9B903BE757F945F":"F873988072B15B10BB2104F4CF6B5C0F",
        LVB_BETA_INSTRUMENT?"F55783DD140C5EE997498BF6549BECB6":"539E040E312B5E979DB62FBE6438D70A"};
    std::array<char*,10> mutableArgs{};
    for (size_t i = 0; i < mutableArgs.size(); ++i) mutableArgs[i] = const_cast<char*>(args[i]);
    const auto o = options(10,mutableArgs.data());
    slowStateRefusals(o);privateGateInputs(o);externalStateActualProducer(o);sustainedMeasurement(o);
    for(const auto* scenario:{"state-record","held-factory-refusal"}) {
        auto accepted=mutableArgs;accepted[3]=const_cast<char*>(scenario);(void)options(10,accepted.data());
    }
    auto sustained=mutableArgs;sustained[3]=const_cast<char*>("sustained");
    sustained[5]=const_cast<char*>("64");sustained[7]=const_cast<char*>("0");
    const auto sustainedOptions=options(10,sustained.data());
    need(sustainedOptions.block==64&&sustainedOptions.delay==0,"declared sustained measurement arguments accepted");
    for(const auto* block:{"63","257"}) {
        auto bad=mutableArgs;bad[3]=const_cast<char*>("sustained");bad[5]=const_cast<char*>(block);
        refuses([&]{options(10,bad.data());},"sustained rejects undeclared actual N");
    }
    for(const auto* delay:{"512","1024"}) {
        auto bad=mutableArgs;bad[3]=const_cast<char*>("sustained");bad[5]=const_cast<char*>("64");bad[7]=const_cast<char*>(delay);
        refuses([&]{options(10,bad.data());},"sustained rejects unsupported measurement D");
    }
    auto recallMissing=mutableArgs;recallMissing[3]=const_cast<char*>("state-recall");
    refuses([&]{options(10,recallMissing.data());},"external recall without capture destination refused");
    std::array<char*,11> recall{};std::copy(mutableArgs.begin(),mutableArgs.end(),recall.begin());
    recall[3]=const_cast<char*>("state-recall");recall[10]=const_cast<char*>("private-state-current");
    const auto recalledOptions=options(11,recall.data());
    need(std::string_view(recalledOptions.prefix)=="private-state"
        &&std::string_view(recalledOptions.capturePrefix)=="private-state-current","distinct external input and current capture prefixes");
    auto sameRecall=recall;sameRecall[10]=sameRecall[4];
    refuses([&]{options(11,sameRecall.data());},"external state input cannot be its current capture destination");
    auto unexpectedExtra=recall;unexpectedExtra[3]=const_cast<char*>("matrix");
    refuses([&]{options(11,unexpectedExtra.data());},"ordinary scenario cannot accept an unexplained argument");
    for (const auto* value : {"","0","-1","+64","64x","1025"," 64","64 ","99999999999999999999999"}) {
        auto bad = mutableArgs; bad[5] = const_cast<char*>(value);
        refuses([&]{options(10,bad.data());},"malformed/out-of-bound M refused before module load");
    }
    for (const auto* value : {"1","255","257","513","1023","1025"}) {
        auto bad = mutableArgs; bad[7] = const_cast<char*>(value);
        refuses([&]{options(10,bad.data());},"undeclared delivery selection refused");
    }
    for (const auto* value : {"44101","48000x","96001","192001"}) {
        auto bad = mutableArgs; bad[6] = const_cast<char*>(value);
        refuses([&]{options(10,bad.data());},"undeclared rate refused");
    }
    for (const auto* value : {"effectx",""}) { auto bad = mutableArgs; bad[2] = const_cast<char*>(value);
        refuses([&]{options(10,bad.data());},"unknown role refused"); }
    for (const auto* value : {"slow","offline-timeoutx",""}) { auto bad = mutableArgs; bad[3] = const_cast<char*>(value);
        refuses([&]{options(10,bad.data());},"unknown scenario refused"); }
    for (const auto* value : {"1111111111111111111111111111111","1111111111111111111111111111111g","aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}) {
        auto bad = mutableArgs; bad[8] = const_cast<char*>(value);
        refuses([&]{options(10,bad.data());},"malformed/exact-case class identity refused");
    }
    auto crossed = mutableArgs; crossed[9] = crossed[8];
    refuses([&]{options(10,crossed.data());},"crossed class pair refused");
    refuses([&]{options(9,mutableArgs.data());},"missing argument refused");
    HostApplication host; ReferenceProcessor processor;
    ok(processor.initialize(&host),"actual producer initialize");
    SpeakerArrangement in = SpeakerArr::kStereo, out[2]{SpeakerArr::kStereo,SpeakerArr::kStereo};
    ok(processor.setBusArrangements(o.instrument?nullptr:&in,o.instrument?0:1,out,2),"actual producer bus negotiation");
    Buffers b(o.instrument); Oracle oracle;
    OutputCapture output;PrivateOutputTest outputLocation;output.prepare(outputLocation.prefix.c_str());
    Row positiveOutput{},corruptedOutput{},failedOracleOutput{};
    auto configure = [&](int mode) {
        ok(processor.setProcessing(false),"direct stop"); ok(processor.setActive(false),"direct deactivate");
        ProcessSetup setup{mode,kSample32,257,48000.}; ok(processor.setupProcessing(setup),"actual producer setup");
        ok(processor.setActive(true),"direct activate"); ok(processor.setProcessing(true),"direct start");
        b.prepare(0,mode,o.instrument,false); point(b.sent,0,.625); point(b.sent,1,.125);
        ok(processor.process(b.data),"recognizable state through ordinary SDK flush");
        oracle.reset(o,48000); oracle.gain = .625; oracle.colour = .125;
    };
    auto actual = [&](int n,int mode,bool first=false,bool final=false,bool tail=false,double gain=-1.) {
        b.prepare(n,mode,o.instrument,n%2 != 0);
        if (gain >= 0.) { point(b.sent,0,gain); oracle.gain = gain; }
        oracle.predict(b,mode,first,final,tail); ok(processor.process(b.data),"actual producer process");
        Row row{};output.copy(b,n,row);
        if(n==257&&!positiveOutput.outputCount)positiveOutput=row;
        oracle.compare(b);
    };
    configure(kRealtime);
    for (int n = 0; n <= 257; ++n) actual(n,n%2?kPrefetch:kRealtime,n==1);
    actual(257,kRealtime,false,true); actual(13,kRealtime,false,false,true); actual(0,kRealtime);
    need(oracle.mismatches == 0 && oracle.nonfinite == 0 && oracle.nonzero > 0,
         "actual producer matches independent first/final stereo/multiout/in-place oracle");
    configure(kOffline);
    for (double value : {.25,.5,.75,.625}) actual(0,kOffline,false,false,false,value);
    need(oracle.eventCount == 0 && oracle.pointCount == 0,"distinct N0 results returned in each exact callback");
    // A stale setup mode would appear as the wrong returned mode, not a valid
    // SDK completion. These negatives reproduce the old mode/zero-result seam.
    b.prepare(0,kOffline,o.instrument,false); oracle.predict(b,kOffline,false,false,false);
    ok(processor.process(b.data),"actual zero-frame results for corruption test");
    auto* queue = static_cast<Queue*>(b.returned.getParameterData(0)); queue->reset(32); int32 index = 0;
    ok(queue->addPoint(0,.5,index),"substitute stale prefetch mode");
    refuses([&]{oracle.compare(b);},"stale actual mode is not accepted");
    configure(kOffline);
    b.prepare(0,kOffline,o.instrument,false); oracle.predict(b,kOffline,false,false,false);
    ok(processor.process(b.data),"actual result before omission"); b.results.clear(); b.returned.clear();
    refuses([&]{oracle.compare(b);},"missing zero-frame results are not accepted as success");
    configure(kOffline);
    b.prepare(257,kOffline,o.instrument,false); oracle.predict(b,kOffline,true,false,false);
    ok(processor.process(b.data),"actual audio before corruption"); b.out[0][0] = .125f;output.copy(b,257,corruptedOutput);
    oracle.compare(b); need(oracle.mismatches > 0,"first-frame substitution is detected");
    b.prepare(13,kOffline,o.instrument,false); oracle.predict(b,kOffline,false,true,false);
    ok(processor.process(b.data),"actual final block before nonfinite corruption");
    b.out[3][12] = std::numeric_limits<float>::quiet_NaN(); b.out[0][13] = 0.f;output.copy(b,13,failedOracleOutput);
    refuses([&]{oracle.compare(b);},"whole output survives the actual final canary oracle failure");
    need(oracle.nonfinite > 0,"nonfinite auxiliary final frame is detected");
    refuses([&]{b.guards(13);},"write beyond actualN borrowed storage is detected");
    ok(processor.setProcessing(false),"final direct stop"); ok(processor.setActive(false),"final direct deactivate");
    ok(processor.terminate(),"actual producer terminate");
    output.write();retainedOutput(output,outputLocation,positiveOutput,corruptedOutput,failedOracleOutput);
    std::cout << "COMPLETION_CONSUMER_ORACLE_V1 role=" << (o.instrument?"instrument":"effect")
              << " exact_producer=passed argument_refusals=passed stale_mode=refused missing_flush=refused first_final_corruption=refused lifecycle_refusal_retention=passed reconfiguration_refusal=retained audited_start_cleanup=passed false_capture_overlap=refused slow_state_observer=passed original_state_stream_refusals=passed external_state_actual_output=passed external_state_inputs_untouched=passed held_factory_gate=passed actual_output_retained=passed corrupt_output_retained=passed\n";
}
