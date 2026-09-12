#include "inspect_module.h"
#include "bus_census.h"
#include "offline_processing.h"
#include "vendor_handler.h"
#include "ap8_state.h"
#include "vendor_view.h"
#include "component_instance_session.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivsteditcontroller.h"
#include "pluginterfaces/vst/ivstmessage.h"
#include "../../vst-state/stream.h"
#include <windows.h>
#include <atomic>
#include <cstring>
#include <cmath>
#include <stdexcept>
#include <iterator>

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
std::string utf8(const TChar* value, size_t limit=128) {
    size_t n=0;while(n<limit&&value[n])++n;
    if(n==limit)throw std::runtime_error("unterminated SDK string");
    int size=WideCharToMultiByte(CP_UTF8,WC_ERR_INVALID_CHARS,reinterpret_cast<const wchar_t*>(value),static_cast<int>(n),nullptr,0,nullptr,nullptr);
    if(n&&size<=0)throw std::runtime_error("invalid SDK string");
    std::string out(size,'\0');
    if(size)WideCharToMultiByte(CP_UTF8,WC_ERR_INVALID_CHARS,reinterpret_cast<const wchar_t*>(value),static_cast<int>(n),out.data(),size,nullptr,nullptr);
    return out;
}
std::string text16(const TChar* value, size_t limit=128) { return quoted(utf8(value,limit).c_str()); }
template<size_t N> std::string bounded(const char (&value)[N]) {
    if(!std::memchr(value,0,N))throw std::runtime_error("unterminated factory metadata");
    return value;
}

}
int inspect_module(Steinberg::IPluginFactory* factory, EventWriter& events, const std::string& class_id, ExternalProcessing* external, const std::wstring& access_directory, bool bus_probe) {
    using namespace Steinberg;using namespace Steinberg::Vst;
    HostApplication host;VendorHandler handler;handler.external=external;
    IComponent* component=nullptr;IAudioProcessor* audio=nullptr;IEditController* controller=nullptr;
    IConnectionPoint *cp=nullptr,*cc=nullptr;
    bool initialized=false,controller_initialized=false,connected_pc=false,connected_cp=false,handler_set=false;
    int primary=0;bool retirement_ready=false;
    auto step=[&](const char* name){events.lifecycle("ap8_call",",\"operation\":"+quoted(name));};
    auto ok=[&](tresult result,const char* name){
        events.lifecycle("ap8_result",",\"operation\":"+quoted(name)+",\"result\":"+std::to_string(result));
        if(result!=kResultOk && !(bus_probe && std::strcmp(name,"censusSetProcessing")==0 && result==kNotImplemented))throw std::runtime_error(name);
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
        PClassInfo selected{};int selected_index=-1;bool found=false;FUID wanted;
        if(!class_id.empty()&&!wanted.fromString(class_id.c_str()))throw std::runtime_error("selected class syntax");
        int count=factory->countClasses();
        if(count<1||count>256)throw std::runtime_error("class count bound");
        for(int i=0;i<count;++i){
            PClassInfo info{};ok(factory->getClassInfo(i,&info),"getClassInfo");
            if(bounded(info.category)==kVstAudioEffectClass && (class_id.empty()||FUID::fromTUID(info.cid)==wanted)){
                if(found)throw std::runtime_error("multiple audio classes require explicit selection");
                selected=info;selected_index=i;found=true;
            }
        }
        if(!found)throw std::runtime_error("audio class absent");
        if(!std::memchr(selected.name,0,sizeof(selected.name)))throw std::runtime_error("unterminated vendor class name");
        char selected_id[33]{};FUID::fromTUID(selected.cid).toString(selected_id);
        events.lifecycle("ap11_class",",\"class_id\":"+quoted(selected_id)+",\"name\":"+quoted(selected.name));
        // Publication uses format-declared metadata, independently of bus layout.
        // Factory-1 fallback is explicit and cannot invent an instrument/FX role.
        PFactoryInfo factory_info{};ok(factory->getFactoryInfo(&factory_info),"getFactoryInfo");
        std::string name=bounded(selected.name),vendor=bounded(factory_info.vendor),version,subcategories;
        const char* tier="factory_1";
        FUnknownPtr<IPluginFactory2> f2(factory);
        if(f3){
            PClassInfoW info{};ok(f3->getClassInfoUnicode(selected_index,&info),"getClassInfoUnicode");
            if(FUID::fromTUID(info.cid)!=FUID::fromTUID(selected.cid))throw std::runtime_error("factory metadata class identity changed");
            name=utf8(info.name,std::size(info.name));auto class_vendor=utf8(info.vendor,std::size(info.vendor));
            if(!class_vendor.empty())vendor=class_vendor;
            version=utf8(info.version,std::size(info.version));subcategories=bounded(info.subCategories);tier="factory_3_unicode";
        }else if(f2){
            PClassInfo2 info{};ok(f2->getClassInfo2(selected_index,&info),"getClassInfo2");
            if(FUID::fromTUID(info.cid)!=FUID::fromTUID(selected.cid))throw std::runtime_error("factory metadata class identity changed");
            name=bounded(info.name);auto class_vendor=bounded(info.vendor);if(!class_vendor.empty())vendor=class_vendor;
            version=bounded(info.version);subcategories=bounded(info.subCategories);tier="factory_2";
        }
        events.lifecycle("ap12_class",",\"class_id\":"+quoted(selected_id)+",\"name\":"+quoted(name.c_str())+",\"vendor\":"+quoted(vendor.c_str())+",\"version\":"+quoted(version.c_str())+",\"subcategories\":"+quoted(subcategories.c_str())+",\"metadata_tier\":"+quoted(tier));
        if(external)external->editor_name(name.c_str());
        step("createComponent");ok(factory->createInstance(selected.cid,IComponent::iid,reinterpret_cast<void**>(&component)),"createComponent");
        if(!component)throw std::runtime_error("null component");
        step("initializeComponent");ok(component->initialize(&host),"initializeComponent");initialized=true;
        step("queryAudioProcessor");ok(component->queryInterface(IAudioProcessor::iid,reinterpret_cast<void**>(&audio)),"queryAudioProcessor");
        if(!audio)throw std::runtime_error("null audio processor");
        const auto initial_buses = bus_probe ? EventBusCensus::capture(*component,*audio,false) : EventBusCensus{};
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
        // A combined component/controller is already connected to itself.
        if(controller_initialized&&cp&&cc){step("connectComponent");ok(cp->connect(cc),"connectComponent");connected_pc=true;step("connectController");ok(cc->connect(cp),"connectController");connected_cp=true;}
        for(int media=0;media<2;++media)for(int dir=0;dir<2;++dir){
            step("getBusCount");int n=component->getBusCount(media,dir);
            if(n<0||n>64)throw std::runtime_error("bus count bound");
            for(int i=0;i<n;++i){BusInfo b{};ok(component->getBusInfo(media,dir,i,b),"getBusInfo");
                SpeakerArrangement arrangement=0;if(media==kAudio)ok(audio->getBusArrangement(dir,i,arrangement),"getBusArrangement");
                events.lifecycle("ap8_bus",",\"media\":"+std::to_string(media)+",\"direction\":"+std::to_string(dir)+",\"index\":"+std::to_string(i)+",\"channels\":"+std::to_string(b.channelCount)+",\"type\":"+std::to_string(b.busType)+",\"flags\":"+std::to_string(b.flags)+",\"arrangement\":"+std::to_string(arrangement)+",\"name\":"+text16(b.name));}
        }
        if(bus_probe){
            auto emit=[&](const char* stage,const EventBusCensus& row){
                std::string fields=",\"stage\":"+quoted(stage)+",\"count\":"+std::to_string(row.count)+",\"index\":0,\"result\":"+std::to_string(row.result)+",\"activated\":"+(row.activated?"true":"false");
                if(row.result==kResultOk)fields+=",\"media\":"+std::to_string(row.info.mediaType)+",\"direction\":"+std::to_string(row.info.direction)+",\"channels\":"+std::to_string(row.info.channelCount)+",\"type\":"+std::to_string(row.info.busType)+",\"flags\":"+std::to_string(row.info.flags)+",\"name\":"+text16(row.info.name);
                fields+=",\"audio\":[";
                for(size_t i=0;i<row.audio_size;++i){if(i)fields+=',';const auto& a=row.audio[i];fields+="["+std::to_string(a.direction)+","+std::to_string(a.index)+","+std::to_string(a.result)+","+std::to_string(a.arrangement)+"]";}
                events.lifecycle("ap18_event_bus_census",fields+"]");
            };
            emit("initialized",initial_buses);
            // No process, editor, parameter or state operation in this probe.
            // The outer inspection owner contains any unresponsive SDK call.
            run_event_bus_census(*component,*audio,emit,step,ok);
        }else{
        int n=controller->getParameterCount();if(n<0||n>8192)throw std::runtime_error("parameter count bound");
        events.lifecycle("ap8_parameter_count",",\"count\":"+std::to_string(n));
        step("enumerateParameters");std::string parameters;
        // Compact bounded chunks keep a large real parameter list within the
        // existing diagnostic byte/event caps. Column order is explicit.
        for(int i=0;i<n;++i){ParameterInfo p{};
            if(controller->getParameterInfo(i,p)!=kResultOk)throw std::runtime_error("getParameterInfo index "+std::to_string(i));
            auto value=controller->getParamNormalized(p.id);
            if(!parameters.empty())parameters+=',';
            parameters+='['+std::to_string(p.id)+','+text16(p.title)+','+text16(p.units)+','+std::to_string(p.stepCount)+','+std::to_string(p.flags)+','+(std::isfinite(p.defaultNormalizedValue)?std::to_string(p.defaultNormalizedValue):"null")+','+(std::isfinite(value)&&value>=0&&value<=1?std::to_string(value):"null")+']';
            if(i%32==31||i+1==n){events.lifecycle("ap8_parameters",",\"columns\":[\"id\",\"title\",\"units\",\"steps\",\"flags\",\"default\",\"value\"],\"parameters\":["+parameters+"]");parameters.clear();}
        }
        ok(kResultOk,"enumerateParameters");
        // Discovery facts remain available when lawful vendor state access is
        // unavailable. This is not a successful state or processing result.
        events.lifecycle("ap12_capabilities",",\"latency_samples\":"+std::to_string(audio->getLatencySamples())+",\"float32_result\":"+std::to_string(audio->canProcessSampleSize(kSample32))+",\"float64_result\":"+std::to_string(audio->canProcessSampleSize(kSample64))+",\"tail_samples\":"+std::to_string(audio->getTailSamples()));
        LVBState::Stream state;step("getComponentState");auto state_result=component->getState(&state);
        char unknown_iid[33]{};FUID::fromTUID(reinterpret_cast<const char*>(state.last_unknown_iid)).toString(unknown_iid);
        events.lifecycle("ap12_state_stream",",\"result\":"+std::to_string(state_result)+",\"bytes\":"+std::to_string(state.bytes.size())+",\"failed\":"+(state.failed?"true":"false")+",\"writes\":"+std::to_string(state.write_calls)+",\"largest_write\":"+std::to_string(state.largest_write)+",\"reads\":"+std::to_string(state.read_calls)+",\"seeks\":"+std::to_string(state.seek_calls)+",\"last_seek_offset\":"+std::to_string(state.last_seek_offset)+",\"last_seek_mode\":"+std::to_string(state.last_seek_mode)+",\"unknown_queries\":"+std::to_string(state.unknown_queries)+",\"last_unknown_iid\":"+quoted(unknown_iid));
        events.lifecycle("ap8_result",",\"operation\":\"getComponentState\",\"result\":"+std::to_string(state_result));
        events.lifecycle("ap12_persistence",",\"capture_available\":"+std::string(state_result==kResultOk?"true":"false")+",\"sdk_result\":"+std::to_string(state_result));
        if(state_result==kResultOk&&controller_initialized)step("synchronizeController");
        synchronize_initial(*controller,controller_initialized,state_result,state);
        if(state_result==kResultOk&&controller_initialized)ok(kResultOk,"synchronizeController");
        if(state_result==kResultOk)events.lifecycle("ap8_inspected",",\"controller_separate\":"+std::string(controller_initialized?"true":"false")+",\"state_bytes\":"+std::to_string(state.bytes.size())+",\"latency_samples\":"+std::to_string(audio->getLatencySamples())+",\"float32_result\":"+std::to_string(audio->canProcessSampleSize(kSample32))+",\"float64_result\":"+std::to_string(audio->canProcessSampleSize(kSample64))+",\"tail_samples\":"+std::to_string(audio->getTailSamples()));
        if(!access_directory.empty()){
            // Explicit unpublished access session: no DAW/DSP impersonation.
            // Reuse the same SDK view lifecycle on this initialized controller.
            VendorView view;
            if(!view.open(*controller))throw std::runtime_error("vendor access editor open failed");
            int title_size=MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,name.data(),int(name.size()),nullptr,0);
            if(title_size<=0)throw std::runtime_error("vendor access title encoding");
            std::wstring title(size_t(title_size),L'\0');
            if(MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,name.data(),int(name.size()),title.data(),title_size)!=title_size)throw std::runtime_error("vendor access title conversion");
            title+=state_result==kResultOk?L" - Vendor access (no DAW audio)":L" - Vendor access (saving unavailable; no DAW audio)";
            SetWindowTextW(view.window(),title.c_str());
            events.lifecycle("ap12_vendor_access_open",",\"save_available\":"+std::string(state_result==kResultOk?"true":"false"));
            const auto deadline=GetTickCount64()+30*60*1000;
            while(VendorView::pump()&&!view.close_requested()&&GetTickCount64()<deadline&&GetFileAttributesW((access_directory+L"\\vendor.stop").c_str())==INVALID_FILE_ATTRIBUTES)Sleep(10);
            if(!view.close())ExitProcess(92);
            events.lifecycle("ap12_vendor_access_closed");
        }
        if(external){
            external->bind_controller(controller,controller_initialized,&handler);
            HostCallbackSink calls(&events,GetCurrentThreadId());
            auto result=run_offline_processing(*component,*audio,calls,events,external);
            if(!result.quiescent)ExitProcess(92); // outer owner contains; no release of live processing objects
            // Policy-selected process retirement never unwinds vendor objects,
            // including on an incomplete processing result.
            retirement_ready=result.success && result.retirement_ready;
            if(!result.success)throw std::runtime_error("commercial processing failed");
        }
        }
    } catch(const std::exception& e){primary=90;events.lifecycle("ap8_failure",",\"reason\":"+quoted(e.what()));}
    if(external)external->retire_vendor_process(retirement_ready);
    // A crashing/hung vendor call is contained by the existing outer process owner.
    // Ordinary failures retain the first operation and still unwind every lease.
    auto cleanup=[&](const char* name,auto call){step(name);try{ok(call(),name);}catch(...){if(!primary)primary=91;}};
    // No view/frame may outlive its controller or connection points. Refusing
    // removal retains containment rather than releasing still-attached SDK objects.
    if(external)try{external->bind_controller(nullptr,false);}catch(...){ExitProcess(92);}
    if(connected_cp)cleanup("disconnectController",[&]{return cc->disconnect(cp);});
    if(connected_pc)cleanup("disconnectComponent",[&]{return cp->disconnect(cc);});
    if(cc)cc->release();if(cp)cp->release();
    if(handler_set)cleanup("clearComponentHandler",[&]{return controller->setComponentHandler(nullptr);});
    if(controller_initialized)cleanup("terminateController",[&]{return controller->terminate();});
    if(controller)controller->release();if(audio)audio->release();
    if(initialized)cleanup("terminateComponent",[&]{return component->terminate();});
    if(component)component->release();
    {FUnknownPtr<IPluginFactory3> f3(factory);if(f3)f3->setHostContext(nullptr);}
    events.lifecycle("ap8_inspection_closed",",\"exit_code\":"+std::to_string(primary));
    return primary;
}
}
