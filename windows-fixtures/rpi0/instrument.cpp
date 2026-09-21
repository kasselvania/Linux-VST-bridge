// Original source-owned Windows VST3 instrument for the isolated RPI0 proof.
#include "synth_core.h"
#include "public.sdk/source/common/pluginview.h"
#include "public.sdk/source/main/pluginfactory.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "pluginterfaces/base/ibstream.h"
#include "pluginterfaces/vst/ivstevents.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include <algorithm>
#include <array>
#include <cstdint>
#include <cstring>
#include <windows.h>
#include <windowsx.h>
using namespace Steinberg;using namespace Steinberg::Vst;
namespace lvb::rpi0 {
static const FUID processor_id(0x52504930,0x41524D41,0x50505359,0x4E540001);
static const FUID controller_id(0x52504930,0x41524D41,0x50505359,0x4E540002);

class Processor final:public AudioEffect {
  SynthCore synth_;
 public:
  Processor(){setControllerClass(controller_id);}
  static FUnknown* create(void*){return static_cast<IAudioProcessor*>(new Processor);}
  tresult PLUGIN_API initialize(FUnknown*host)override{auto r=AudioEffect::initialize(host);if(r!=kResultOk)return r;addAudioOutput(STR16("Stereo output"),SpeakerArr::kStereo);addEventInput(STR16("MIDI input"),16);return kResultOk;}
  tresult PLUGIN_API setBusArrangements(SpeakerArrangement*in,int32 ni,SpeakerArrangement*out,int32 no)override{return ni==0&&no==1&&!in&&out&&out[0]==SpeakerArr::kStereo?AudioEffect::setBusArrangements(in,ni,out,no):kResultFalse;}
  tresult PLUGIN_API canProcessSampleSize(int32 size)override{return size==kSample32?kResultTrue:kResultFalse;}
  tresult PLUGIN_API setupProcessing(ProcessSetup&setup)override{if(setup.sampleRate<=0||setup.maxSamplesPerBlock<=0)return kInvalidArgument;synth_.reset(setup.sampleRate);return AudioEffect::setupProcessing(setup);}
  uint32 PLUGIN_API getLatencySamples()override{return 0;}uint32 PLUGIN_API getTailSamples()override{return kInfiniteTail;}
  tresult PLUGIN_API getState(IBStream*stream)override{auto state=synth_.state();int32 written=0;return stream&&stream->write(&state,sizeof(state),&written)==kResultOk&&written==sizeof(state)?kResultOk:kResultFalse;}
  tresult PLUGIN_API setState(IBStream*stream)override{State state{};int32 read=0;return stream&&stream->read(&state,sizeof(state),&read)==kResultOk&&read==sizeof(state)&&synth_.restore(state)?kResultOk:kResultFalse;}
  tresult PLUGIN_API process(ProcessData&data)override{
    if(data.symbolicSampleSize!=kSample32||data.numInputs!=0||data.numOutputs!=(data.numSamples?1:0)||data.numSamples<0)return kResultFalse;
    struct Change{int32 offset=0;ParamID id=0;ParamValue value=0;};std::array<Change,256> changes{};int32 change_count=0;
    if(data.inputParameterChanges){const auto queues=data.inputParameterChanges->getParameterCount();if(queues<0||queues>256)return kResultFalse;for(int32 q=0;q<queues;++q){auto*queue=data.inputParameterChanges->getParameterData(q);if(!queue)return kResultFalse;const auto points=queue->getPointCount();if(points<0||points>256-change_count)return kResultFalse;for(int32 p=0;p<points;++p){auto&change=changes[size_t(change_count++)];change.id=queue->getParameterId();if(queue->getPoint(p,change.offset,change.value)!=kResultOk||change.offset<0||change.offset>=data.numSamples||!std::isfinite(change.value)||change.value<0||change.value>1)return kResultFalse;}}}
    std::array<Event,256> events{};int32 event_count=0;if(data.inputEvents){event_count=data.inputEvents->getEventCount();if(event_count<0||event_count>256)return kResultFalse;for(int32 e=0;e<event_count;++e)if(data.inputEvents->getEvent(e,events[size_t(e)])!=kResultOk||events[size_t(e)].sampleOffset<0||events[size_t(e)].sampleOffset>=data.numSamples)return kResultFalse;}
    for(int32 sample=0;sample<data.numSamples;++sample){
      for(int32 p=0;p<change_count;++p){const auto&change=changes[size_t(p)];if(change.offset!=sample)continue;auto id=change.id;auto value=change.value;if(id==gain_parameter)synth_.gain(value);else if(id>=sustain_parameter_base&&id<sustain_parameter_base+16)synth_.sustain(uint8_t(id-sustain_parameter_base),value>=.5);else if(id>=all_notes_off_parameter_base&&id<all_notes_off_parameter_base+16&&value>=.5)synth_.all_notes_off(uint8_t(id-all_notes_off_parameter_base));}
      for(int32 e=0;e<event_count;++e){const auto&event=events[size_t(e)];if(event.sampleOffset!=sample)continue;if(event.type==Event::kNoteOnEvent)synth_.note_on(uint8_t(event.noteOn.channel),uint8_t(event.noteOn.pitch),std::clamp(event.noteOn.velocity,0.f,1.f),event.noteOn.noteId);else if(event.type==Event::kNoteOffEvent)synth_.note_off(uint8_t(event.noteOff.channel),uint8_t(event.noteOff.pitch),event.noteOff.noteId);}
      synth_.sample(data.outputs[0].channelBuffers32[0][sample],data.outputs[0].channelBuffers32[1][sample]);
    }
    if(data.numSamples)data.outputs[0].silenceFlags=0;return kResultOk;
  }
};

class Controller;
class Editor final:public CPluginView {
  Controller*controller_;HWND child_=nullptr;bool dragging_=false;
  static LRESULT CALLBACK window(HWND window,UINT message,WPARAM wparam,LPARAM lparam);
 public:
  explicit Editor(Controller*controller);~Editor()override;
  tresult PLUGIN_API isPlatformTypeSupported(FIDString type)override{return type&&!std::strcmp(type,kPlatformTypeHWND)?kResultOk:kResultFalse;}
  tresult PLUGIN_API attached(void*parent,FIDString type)override;
  tresult PLUGIN_API removed()override;
  tresult PLUGIN_API onSize(ViewRect*rect)override;
  tresult PLUGIN_API canResize()override{return kResultTrue;}
  tresult PLUGIN_API checkSizeConstraint(ViewRect*rect)override{if(!rect)return kInvalidArgument;rect->right=rect->left+std::clamp(rect->getWidth(),320,800);rect->bottom=rect->top+std::clamp(rect->getHeight(),160,480);return kResultOk;}
  void mouse(int x,bool begin,bool end);void paint(HWND window);
};

class Controller final:public EditControllerEx1 {
  double gain_=.5;HWND editor_window_=nullptr;std::uint64_t editor_invalidations_=0;
 public:
  static FUnknown* create(void*){return static_cast<IEditController*>(new Controller);}
  tresult PLUGIN_API initialize(FUnknown*host)override{auto r=EditControllerEx1::initialize(host);if(r!=kResultOk)return r;parameters.addParameter(STR16("Gain"),nullptr,0,.5,ParameterInfo::kCanAutomate,gain_parameter);for(uint32 i=0;i<16;++i){parameters.addParameter(STR16("Sustain"),nullptr,1,0,ParameterInfo::kIsHidden,sustain_parameter_base+i);parameters.addParameter(STR16("All notes off"),nullptr,1,0,ParameterInfo::kIsHidden,all_notes_off_parameter_base+i);}return kResultOk;}
  ParamValue PLUGIN_API getParamNormalized(ParamID id)override{return id==gain_parameter?gain_:EditControllerEx1::getParamNormalized(id);}
  tresult PLUGIN_API setParamNormalized(ParamID id,ParamValue value)override{const bool changed=id==gain_parameter&&std::isfinite(value)&&value>=0&&value<=1;if(changed)gain_=value;auto result=EditControllerEx1::setParamNormalized(id,value);if(changed&&editor_window_){++editor_invalidations_;InvalidateRect(editor_window_,nullptr,FALSE);}return result;}
  tresult PLUGIN_API setComponentState(IBStream*stream)override{return read_state(stream);}
  tresult PLUGIN_API setState(IBStream*stream)override{return read_state(stream);}
  tresult PLUGIN_API getState(IBStream*stream)override{State state{state_magic,state_version,gain_};int32 written=0;return stream&&stream->write(&state,sizeof(state),&written)==kResultOk&&written==sizeof(state)?kResultOk:kResultFalse;}
  IPlugView* PLUGIN_API createView(FIDString name)override{return name&&!std::strcmp(name,ViewType::kEditor)?new Editor(this):nullptr;}
  void edit(double value,bool begin,bool end){value=std::clamp(value,0.,1.);gain_=value;EditControllerEx1::setParamNormalized(gain_parameter,value);if(componentHandler){if(begin)componentHandler->beginEdit(gain_parameter);componentHandler->performEdit(gain_parameter,value);if(end)componentHandler->endEdit(gain_parameter);}}
  void editor_window(HWND window){editor_window_=window;}
  void editor_removed(HWND window){if(editor_window_==window)editor_window_=nullptr;}
  std::uint64_t editor_invalidation_count()const{return editor_invalidations_;}
 private:tresult read_state(IBStream*stream){State state{};int32 read=0;if(!stream||stream->read(&state,sizeof(state),&read)!=kResultOk||read!=sizeof(state)||state.magic!=state_magic||state.version!=state_version||!std::isfinite(state.gain)||state.gain<0||state.gain>1)return kResultFalse;gain_=state.gain;return setParamNormalized(gain_parameter,gain_);}
};

Editor::Editor(Controller*controller):controller_(controller){controller_->addRef();setRect({0,0,480,220});}
Editor::~Editor(){removed();controller_->release();}
LRESULT CALLBACK Editor::window(HWND window,UINT message,WPARAM wparam,LPARAM lparam){auto*self=reinterpret_cast<Editor*>(GetWindowLongPtrW(window,GWLP_USERDATA));if(message==WM_NCCREATE){self=static_cast<Editor*>(reinterpret_cast<CREATESTRUCTW*>(lparam)->lpCreateParams);SetWindowLongPtrW(window,GWLP_USERDATA,LONG_PTR(self));}if(!self)return DefWindowProcW(window,message,wparam,lparam);switch(message){case WM_PAINT:self->paint(window);return 0;case WM_LBUTTONDOWN:SetCapture(window);self->dragging_=true;self->mouse(GET_X_LPARAM(lparam),true,false);return 0;case WM_MOUSEMOVE:if(self->dragging_)self->mouse(GET_X_LPARAM(lparam),false,false);return 0;case WM_LBUTTONUP:if(self->dragging_){self->mouse(GET_X_LPARAM(lparam),false,true);self->dragging_=false;ReleaseCapture();}return 0;case WM_LBUTTONDBLCLK:if(self->plugFrame){ViewRect next{0,0,self->rect.getWidth()==480?640:480,self->rect.getHeight()==220?300:220};self->plugFrame->resizeView(self,&next);}return 0;}return DefWindowProcW(window,message,wparam,lparam);}
tresult Editor::attached(void*parent,FIDString type){if(isPlatformTypeSupported(type)!=kResultOk||!parent||child_)return kResultFalse;WNDCLASSW wc{};wc.lpfnWndProc=window;wc.hInstance=GetModuleHandleW(nullptr);wc.lpszClassName=L"LVB_RPI0_Editor";wc.hCursor=LoadCursorW(nullptr,IDC_ARROW);wc.style=CS_DBLCLKS;if(!RegisterClassW(&wc)&&GetLastError()!=ERROR_CLASS_ALREADY_EXISTS)return kResultFalse;systemWindow=parent;child_=CreateWindowExW(0,wc.lpszClassName,L"",WS_CHILD|WS_VISIBLE,0,0,rect.getWidth(),rect.getHeight(),HWND(parent),nullptr,wc.hInstance,this);if(child_)controller_->editor_window(child_);return child_?kResultOk:kResultFalse;}
tresult Editor::removed(){if(child_)controller_->editor_removed(child_);if(child_&&IsWindow(child_))DestroyWindow(child_);child_=nullptr;systemWindow=nullptr;return kResultOk;}
tresult Editor::onSize(ViewRect*next){if(!next||!child_)return kResultFalse;auto constrained=*next;checkSizeConstraint(&constrained);setRect(constrained);return MoveWindow(child_,0,0,constrained.getWidth(),constrained.getHeight(),TRUE)?kResultOk:kResultFalse;}
void Editor::mouse(int x,bool begin,bool end){controller_->edit(double(std::clamp(x,20,std::max(20,rect.getWidth()-20))-20)/std::max(1,rect.getWidth()-40),begin,end);if(child_)InvalidateRect(child_,nullptr,FALSE);}
void Editor::paint(HWND window){PAINTSTRUCT paint{};auto dc=BeginPaint(window,&paint);RECT area{};GetClientRect(window,&area);auto background=CreateSolidBrush(RGB(24,28,34));FillRect(dc,&area,background);DeleteObject(background);RECT track{20,90,area.right-20,130};auto base=CreateSolidBrush(RGB(65,74,84));FillRect(dc,&track,base);DeleteObject(base);track.right=track.left+LONG((area.right-40)*controller_->getParamNormalized(gain_parameter));auto fill=CreateSolidBrush(RGB(70,190,230));FillRect(dc,&track,fill);DeleteObject(fill);SetTextColor(dc,RGB(240,240,240));SetBkMode(dc,TRANSPARENT);DrawTextW(dc,L"LVB ARM Appliance Synth - Gain",-1,&area,DT_CENTER|DT_TOP);EndPaint(window,&paint);}
}
#ifndef RPI0_NO_FACTORY
BEGIN_FACTORY_DEF("Kasselvania Research","https://github.com/kasselvania/Linux-VST-bridge","")
DEF_CLASS2(INLINE_UID_FROM_FUID(lvb::rpi0::processor_id),PClassInfo::kManyInstances,kVstAudioEffectClass,"LVB ARM Appliance Synth",Vst::kDistributable,"Instrument|Synth","1.0.0",kVstVersionString,lvb::rpi0::Processor::create)
DEF_CLASS2(INLINE_UID_FROM_FUID(lvb::rpi0::controller_id),PClassInfo::kManyInstances,kVstComponentControllerClass,"LVB ARM Appliance Synth Controller",0,"","1.0.0",kVstVersionString,lvb::rpi0::Controller::create)
END_FACTORY
#endif
