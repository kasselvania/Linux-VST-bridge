#include "inspect_module.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstmessage.h"
#include "../../vst-state/stream.h"
#include <windows.h>
#include <atomic>
#include <cstring>
#include <stdexcept>

namespace linux_vst_bridge::wf0 {
namespace {
using namespace Steinberg;
using namespace Steinberg::Vst;
std::string quoted(const char* value) {
    std::string out="\"";
    for (const unsigned char c:std::string(value)) {
        if (c=='"'||c=='\\') out+='\\';
        if (c>=32) out+=static_cast<char>(c);
    }
    return out+'"';
}
std::string text16(const TChar* value, size_t limit=128) {
    size_t n=0;while(n<limit&&value[n])++n;
    if(n==limit)throw std::runtime_error("unterminated SDK string");
    int size=WideCharToMultiByte(CP_UTF8,WC_ERR_INVALID_CHARS,reinterpret_cast<const wchar_t*>(value),static_cast<int>(n),nullptr,0,nullptr,nullptr);
    if(n&&size<=0)throw std::runtime_error("invalid SDK string");
    std::string out(size,'\0');
    if(size)WideCharToMultiByte(CP_UTF8,WC_ERR_INVALID_CHARS,reinterpret_cast<const wchar_t*>(value),static_cast<int>(n),out.data(),size,nullptr,nullptr);
    return quoted(out.c_str());
}
class Handler final:public IComponentHandler {
    std::atomic<uint32> refs{1};
public:
    tresult PLUGIN_API queryInterface(const TUID id,void**out) override {
        if(!out)return kInvalidArgument;*out=nullptr;
        if(FUnknownPrivate::iidEqual(id,IComponentHandler::iid)||FUnknownPrivate::iidEqual(id,FUnknown::iid)){
            *out=static_cast<IComponentHandler*>(this);addRef();return kResultOk;
        }return kNoInterface;
    }
    uint32 PLUGIN_API addRef()override{return ++refs;}
    uint32 PLUGIN_API release()override{return --refs;}
    tresult PLUGIN_API beginEdit(ParamID)override{return kResultOk;}
    tresult PLUGIN_API performEdit(ParamID,ParamValue)override{return kResultOk;}
    tresult PLUGIN_API endEdit(ParamID)override{return kResultOk;}
    tresult PLUGIN_API restartComponent(int32)override{return kResultOk;}
};
}
int inspect_module(Steinberg::IPluginFactory* factory, EventWriter& events) {
    using namespace Steinberg;using namespace Steinberg::Vst;
    HostApplication host;Handler handler;
    IComponent* component=nullptr;IAudioProcessor* audio=nullptr;IEditController* controller=nullptr;
    IConnectionPoint *cp=nullptr,*cc=nullptr;
    bool initialized=false,controller_initialized=false,connected_pc=false,connected_cp=false,handler_set=false;
    int primary=0;
    auto step=[&](const char* name){events.lifecycle("ap8_call",",\"operation\":"+quoted(name));};
    auto ok=[&](tresult result,const char* name){
        events.lifecycle("ap8_result",",\"operation\":"+quoted(name)+",\"result\":"+std::to_string(result));
        if(result!=kResultOk)throw std::runtime_error(name);
    };
    try {
        FUnknownPtr<IPluginFactory3> f3(factory);
        if(f3){
            step("setHostContext");auto result=f3->setHostContext(&host);
            events.lifecycle("ap8_result",",\"operation\":\"setHostContext\",\"result\":"+std::to_string(result));
            // Optional factory context: the official SDK hosting wrapper also
            // permits factories that do not implement this callback. Component
            // initialize still receives the actual host context below.
            if(result!=kResultOk&&result!=kNotImplemented&&result!=kResultFalse)
                throw std::runtime_error("setHostContext failed");
        }
        PClassInfo selected{};bool found=false;
        int count=factory->countClasses();
        if(count<1||count>256)throw std::runtime_error("class count bound");
        for(int i=0;i<count;++i){PClassInfo info{};ok(factory->getClassInfo(i,&info),"getClassInfo");if(std::strcmp(info.category,kVstAudioEffectClass)==0){if(found)throw std::runtime_error("multiple audio classes require explicit selection");selected=info;found=true;}}
        if(!found)throw std::runtime_error("audio class absent");
        step("createComponent");ok(factory->createInstance(selected.cid,IComponent::iid,reinterpret_cast<void**>(&component)),"createComponent");
        if(!component)throw std::runtime_error("null component");
        step("initializeComponent");ok(component->initialize(&host),"initializeComponent");initialized=true;
        step("queryAudioProcessor");ok(component->queryInterface(IAudioProcessor::iid,reinterpret_cast<void**>(&audio)),"queryAudioProcessor");
        if(!audio)throw std::runtime_error("null audio processor");
        auto query=component->queryInterface(IEditController::iid,reinterpret_cast<void**>(&controller));
        if(query==kNoInterface&&controller==nullptr){
            TUID cid{};step("getControllerClassId");ok(component->getControllerClassId(cid),"getControllerClassId");
            step("createController");ok(factory->createInstance(cid,IEditController::iid,reinterpret_cast<void**>(&controller)),"createController");
            if(!controller)throw std::runtime_error("null controller");
            step("initializeController");ok(controller->initialize(&host),"initializeController");controller_initialized=true;
        }else if(query!=kResultOk||!controller)throw std::runtime_error("controller query tuple");
        step("setComponentHandler");ok(controller->setComponentHandler(&handler),"setComponentHandler");handler_set=true;
        component->queryInterface(IConnectionPoint::iid,reinterpret_cast<void**>(&cp));
        controller->queryInterface(IConnectionPoint::iid,reinterpret_cast<void**>(&cc));
        if(cp&&cc){step("connectComponent");ok(cp->connect(cc),"connectComponent");connected_pc=true;step("connectController");ok(cc->connect(cp),"connectController");connected_cp=true;}
        for(int media=0;media<2;++media)for(int dir=0;dir<2;++dir){
            step("getBusCount");int n=component->getBusCount(media,dir);
            if(n<0||n>64)throw std::runtime_error("bus count bound");
            for(int i=0;i<n;++i){BusInfo b{};ok(component->getBusInfo(media,dir,i,b),"getBusInfo");
                events.lifecycle("ap8_bus",",\"media\":"+std::to_string(media)+",\"direction\":"+std::to_string(dir)+",\"index\":"+std::to_string(i)+",\"channels\":"+std::to_string(b.channelCount)+",\"type\":"+std::to_string(b.busType)+",\"flags\":"+std::to_string(b.flags)+",\"name\":"+text16(b.name));}
        }
        int n=controller->getParameterCount();if(n<0||n>8192)throw std::runtime_error("parameter count bound");
        for(int i=0;i<n;++i){ParameterInfo p{};ok(controller->getParameterInfo(i,p),"getParameterInfo");
            events.lifecycle("ap8_parameter",",\"id\":"+std::to_string(p.id)+",\"title\":"+text16(p.title)+",\"units\":"+text16(p.units)+",\"steps\":"+std::to_string(p.stepCount)+",\"flags\":"+std::to_string(p.flags)+",\"default\":"+std::to_string(p.defaultNormalizedValue)+",\"value\":"+std::to_string(controller->getParamNormalized(p.id)));}
        LVBState::Stream state;step("getComponentState");ok(component->getState(&state),"getComponentState");
        if(state.failed||!state.quiescent())throw std::runtime_error("state stream bounds/lifetime");
        state.position=0;step("synchronizeController");ok(controller->setComponentState(&state),"synchronizeController");
        if(state.failed||!state.quiescent())throw std::runtime_error("controller state stream lifetime");
        events.lifecycle("ap8_inspected",",\"state_bytes\":"+std::to_string(state.bytes.size())+",\"latency_samples\":"+std::to_string(audio->getLatencySamples())+",\"float32_result\":"+std::to_string(audio->canProcessSampleSize(kSample32)));
    } catch(const std::exception& e){primary=90;events.lifecycle("ap8_failure",",\"reason\":"+quoted(e.what()));}
    // A crashing/hung vendor call is contained by the existing outer process owner.
    // Ordinary failures retain the first operation and still unwind every lease.
    auto cleanup=[&](const char* name,auto call){step(name);try{ok(call(),name);}catch(...){if(!primary)primary=91;}};
    if(connected_cp)cleanup("disconnectController",[&]{return cc->disconnect(cp);});
    if(connected_pc)cleanup("disconnectComponent",[&]{return cp->disconnect(cc);});
    if(cc)cc->release();if(cp)cp->release();
    if(handler_set)cleanup("clearComponentHandler",[&]{return controller->setComponentHandler(nullptr);});
    if(controller_initialized)cleanup("terminateController",[&]{return controller->terminate();});
    if(controller)controller->release();if(audio)audio->release();
    if(initialized)cleanup("terminateComponent",[&]{return component->terminate();});
    if(component)component->release();
    events.lifecycle("ap8_inspection_closed",",\"exit_code\":"+std::to_string(primary));
    return primary;
}
}
