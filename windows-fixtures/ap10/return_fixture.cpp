// Original bounded SDK instrumentation; no vendor DSP/state or bridge bypass.
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "public.sdk/source/main/pluginfactory.h"
#include "pluginterfaces/vst/ivstevents.h"
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include "pluginterfaces/base/ibstream.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include <array>
#include <algorithm>
#include <cmath>
#include <thread>
#include <chrono>
using namespace Steinberg;using namespace Steinberg::Vst;
static const FUID processorID(0x41503130,0x52455455,0x524E5354,0x45535401);
static const FUID controllerID(0x41503130,0x52455455,0x524E5354,0x45535402);
class ReturnProcessor final:public AudioEffect {
 double gain=.5;std::array<std::array<float,13>,2> delay{};size_t cursor=0;
public:
 ReturnProcessor(){setControllerClass(controllerID);}
 static FUnknown* create(void*){return static_cast<IAudioProcessor*>(new ReturnProcessor);}
 tresult PLUGIN_API initialize(FUnknown*host)override{auto r=AudioEffect::initialize(host);if(r!=kResultOk)return r;addAudioInput(STR16("Main input"),SpeakerArr::kStereo);addAudioOutput(STR16("Main output"),SpeakerArr::kStereo);addEventInput(STR16("Events in"),16);addEventOutput(STR16("Events out"),16,kMain,0);return kResultOk;}
 tresult PLUGIN_API setBusArrangements(SpeakerArrangement*in,int32 ni,SpeakerArrangement*out,int32 no)override{return ni==1&&no==1&&in&&out&&in[0]==SpeakerArr::kStereo&&out[0]==SpeakerArr::kStereo?AudioEffect::setBusArrangements(in,ni,out,no):kResultFalse;}
 tresult PLUGIN_API canProcessSampleSize(int32 n)override{return n==kSample32?kResultTrue:kResultFalse;}
 uint32 PLUGIN_API getLatencySamples()override{return 13;}
 tresult PLUGIN_API getState(IBStream*s)override{int32 n=0;return s&&s->write(&gain,8,&n)==kResultOk&&n==8?kResultOk:kResultFalse;}
 tresult PLUGIN_API setState(IBStream*s)override{double v=0;int32 n=0;if(!s||s->read(&v,8,&n)!=kResultOk||n!=8||!std::isfinite(v)||v<0||v>1)return kResultFalse;gain=v;return kResultOk;}
 tresult PLUGIN_API process(ProcessData&d)override{
  int command=-1;int32 trigger=0;
  if(d.inputParameterChanges)for(int32 i=0;i<d.inputParameterChanges->getParameterCount();++i){auto*q=d.inputParameterChanges->getParameterData(i);if(!q)continue;for(int32 j=0;j<q->getPointCount();++j){int32 offset=0;double value=0;if(q->getPoint(j,offset,value)!=kResultOk)return kResultFalse;if(q->getParameterId()==0)gain=value;if(q->getParameterId()==1){command=int(std::lround(value*10));trigger=offset;}}}
  if(command==2)std::this_thread::sleep_for(std::chrono::milliseconds(25));
  if(d.numSamples&&d.numInputs==1&&d.numOutputs==1){for(int32 i=0;i<d.numSamples;++i){for(int ch=0;ch<2;++ch){float in=d.inputs[0].channelBuffers32[ch][i];d.outputs[0].channelBuffers32[ch][i]=delay[ch][cursor];delay[ch][cursor]=in*float(gain);}cursor=(cursor+1)%13;}d.outputs[0].silenceFlags=0;}
  if(command<0)return kResultOk;
  if(d.outputParameterChanges){int32 index=0;auto*q=d.outputParameterChanges->addParameterData(0,index);if(q)q->addPoint(std::min(trigger,std::max(0,d.numSamples-1)),.25,index);}
  auto active=eventOutputs.at(0)&&eventOutputs.at(0)->isActive();
  if(!d.outputEvents||!active)return kResultOk;
  auto emit=[&](Event e,int offset){e.busIndex=0;e.sampleOffset=std::min(std::max(0,d.numSamples-1),trigger+offset);e.ppqPosition=7.25;e.flags=0xc001;d.outputEvents->addEvent(e);};
  auto note=[&](bool on,int id,int offset){Event e{};e.type=on?Event::kNoteOnEvent:Event::kNoteOffEvent;if(on)e.noteOn={2,60,1.5f,.75f,id==-1234?99:0,id};else e.noteOff={2,60,.25f,id,-2.5f};emit(e,offset);};
  if(command==1||command==7){note(true,-77,5);return kResultOk;}
  if(command==2){note(false,-77,7);return kResultOk;}
  if(command==4){for(int i=0;i<65;++i){Event e{};e.type=Event::kLegacyMIDICCOutEvent;e.midiCCOut={64,2,32,0};emit(e,0);}return kResultOk;}
  if(command==5){std::array<uint8,513> data{};Event e{};e.type=Event::kDataEvent;e.data={513,0,data.data()};emit(e,0);return kResultOk;}
  if(command==6)return kResultOk;
  if(command!=0)return kResultOk;
  note(true,-1234,0);note(false,-1234,1);
  uint8 bytes[]={0xf0,1,0xf7};TChar text[]={u'A',u'B',0};Event e{};
  e.type=Event::kDataEvent;e.data={3,0,bytes};emit(e,2);bytes[1]=99;
  e={};e.type=Event::kPolyPressureEvent;e.polyPressure={2,60,.5f,-1234};emit(e,3);
  e={};e.type=Event::kNoteExpressionValueEvent;e.noteExpressionValue={123,-1234,.25};emit(e,4);
  e={};e.type=Event::kNoteExpressionTextEvent;e.noteExpressionText={124,-1234,2,text};emit(e,5);
  e={};e.type=Event::kChordEvent;e.chord={60,48,-1,2,text};emit(e,6);
  e={};e.type=Event::kScaleEvent;e.scale={60,-1,2,text};emit(e,7);text[0]=u'Z';
  e={};e.type=Event::kNoteExpressionIntValueEvent;e.noteExpressionIntValue={125,-1234,0xfedcba9876543210ULL};emit(e,8);
  e={};e.type=Event::kLegacyMIDICCOutEvent;e.midiCCOut={129,2,32,64};emit(e,9);
  return kResultOk;
 }
};
class ReturnController final:public EditControllerEx1 {
public:
 static FUnknown* create(void*){return static_cast<IEditController*>(new ReturnController);}
 tresult PLUGIN_API initialize(FUnknown*h)override{auto r=EditControllerEx1::initialize(h);if(r!=kResultOk)return r;parameters.addParameter(STR16("Gain"),nullptr,0,.5,ParameterInfo::kCanAutomate,0);parameters.addParameter(STR16("Test command"),nullptr,0,0,ParameterInfo::kCanAutomate,1);return kResultOk;}
 tresult PLUGIN_API setComponentState(IBStream*s)override{double value=0;int32 n=0;if(!s||s->read(&value,8,&n)!=kResultOk||n!=8)return kResultFalse;return setParamNormalized(0,value);}
};
BEGIN_FACTORY_DEF("Linux VST Bridge", "https://github.com/kasselvania/Linux-VST-bridge", "")
DEF_CLASS2(INLINE_UID_FROM_FUID(processorID),PClassInfo::kManyInstances,kVstAudioEffectClass,"AP10 Return Fixture",Vst::kDistributable,"Fx", "1.0.0",kVstVersionString,ReturnProcessor::create)
DEF_CLASS2(INLINE_UID_FROM_FUID(controllerID),PClassInfo::kManyInstances,kVstComponentControllerClass,"AP10 Return Controller",0,"", "1.0.0",kVstVersionString,ReturnController::create)
END_FACTORY
