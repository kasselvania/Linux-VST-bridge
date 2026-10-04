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
struct PrivateOutputTest {
    std::string directory,prefix;
    PrivateOutputTest() {
        directory=(std::filesystem::temp_directory_path()/"lvb-completion-output-XXXXXX").string();
        need(::mkdtemp(directory.data())!=nullptr,"private owned output test directory");
        prefix=directory+"/run";
    }
    ~PrivateOutputTest() { std::error_code ignored;std::filesystem::remove_all(directory,ignored); }
};
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
int main() {
    lifecycleRefusals();configureOwnershipRefusals();overlapRefusals();
    const char* args[]{"completion-host","unused.vst3",LVB_BETA_INSTRUMENT?"instrument":"effect","matrix","private-state",
        "257","48000","0","11111111111111111111111111111111","22222222222222222222222222222222"};
    std::array<char*,10> mutableArgs{};
    for (size_t i = 0; i < mutableArgs.size(); ++i) mutableArgs[i] = const_cast<char*>(args[i]);
    const auto o = options(10,mutableArgs.data());
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
              << " exact_producer=passed argument_refusals=passed stale_mode=refused missing_flush=refused first_final_corruption=refused lifecycle_refusal_retention=passed reconfiguration_refusal=retained audited_start_cleanup=passed false_capture_overlap=refused actual_output_retained=passed corrupt_output_retained=passed\n";
}
