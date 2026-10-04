// Real native Processor/SDK callback; transport boundary checks exact samples.
#include "processor.h"
#include "ap4_backend.h"
#include "ap10_backend.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/hosting/eventlist.h"
#include "public.sdk/source/vst/hosting/parameterchanges.h"
#include <algorithm>
#include <array>
#include <cassert>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <dlfcn.h>
#include <fstream>
#include <iterator>
#include <string>
#include <unistd.h>
using namespace Steinberg;
using namespace Steinberg::Vst;
static unsigned setups=0,activations=0,closes=0,processes=0;
static bool input_enabled=true,last_output=false;
enum class InputCase { Original, MixedExpression, ExpressionOnly, NoteOffOnly, LateNoteOff, EndpointCurve, InvalidParameterExtent };
static InputCase input_case=InputCase::Original;
static std::array<float,32> expected_left{},expected_right{};
static uint64_t expected_silence=3;
extern "C" {
uint32_t __wrap_if2_terminal_status(uint64_t){return false ? 1 : 0;}
uint32_t __wrap_ap9_open(const uint8_t*,uint64_t*h){*h=1;return 0;}
uint32_t __wrap_ap22_curve_parameters(uint64_t,const uint32_t*,uint32_t){return 0;}
uint32_t __wrap_ap5_report_path(uint64_t,uint8_t*p,uint32_t n){if(n)*p=0;return 0;}
uint32_t __wrap_ap10_setup(uint64_t,uint32_t,uint32_t,double,const uint8_t*p,uint32_t n,uint32_t,uint32_t*t){
 assert(n==1060&&p[0]==33);assert(p[4+16]==kAux);
 assert(p[4+20]==unsigned(input_enabled)&&p[4+32+20]==1);
 for(unsigned i=2;i<33;++i)assert(p[4+32*i+20]==unsigned(i==32&&last_output));
 ++setups;t[0]=512;t[1]=t[2]=0;return 0;
}
uint32_t __wrap_ap4_activate(uint64_t,uint32_t,uint32_t){++activations;return 0;}
uint32_t __wrap_ap4_deactivate(uint64_t){return 0;}
uint32_t __wrap_ap3_transition(uint64_t,uint32_t){return 0;}
uint32_t __wrap_ap10_take_results(uint64_t,ap10_results_t*p){static const ap10_results_t empty{};*p=empty;return 0;}
uint32_t __wrap_if2_process(uint64_t,uint32_t n,const ap8_event_t*e,uint32_t count,const ap10_context_t*,uint64_t silence,const float*l,const float*r,float*ol,float*orr,uint64_t*out,ap7_delivery_t*d,uint64_t entered){
 assert(entered);
 if(input_case==InputCase::InvalidParameterExtent){
  assert(n==32&&count==1&&e[0].kind==2&&e[0].id==0&&e[0].offset==33&&e[0].value==.75);
  return 0x102;
 }
 if(input_case==InputCase::EndpointCurve){
  assert(n==32&&count==2&&e[0].kind==2&&e[1].kind==2);
  assert(e[0].id==0&&e[1].id==0&&e[0].offset==0&&e[1].offset==32);
  assert(e[0].value==.25&&e[1].value==.75);
  std::copy_n(l,n,ol);std::copy_n(r,n,orr);*out=silence;*d={};d->delivered_frames=n;++processes;return 0;
 }
 if(input_case==InputCase::Original){
  assert(count==2&&e[0].kind==0&&e[1].kind==2);
  assert(e[0].offset==(n?7u:0u)&&e[1].offset==(n?11u:0u));
 }else if(input_case==InputCase::MixedExpression){
  assert(count==2&&e[0].kind==0&&e[1].kind==1);
  assert(e[0].offset==7&&e[1].offset==19);
  assert(e[0].id==1&&e[1].id==1&&e[0].channel==9&&e[1].channel==9);
 }else if(input_case==InputCase::LateNoteOff){
  assert(n==32&&count==2&&e[0].kind==1&&e[1].kind==0);
  // Mirror the strict transport extent check, so the unmodified SDK boundary
  // reproduces the recorded permanent refusal instead of hiding it in a stub.
  if(e[0].offset>=n)return 0x102;
  assert(e[0].offset==0&&e[0].id==UINT32_MAX&&e[0].channel==9&&e[0].pitch==63);
  assert(e[0].value==.25&&e[0].tuning==0);
  assert(e[1].offset==7&&e[1].id==1&&e[1].channel==9&&e[1].pitch==60);
 }else if(input_case==InputCase::ExpressionOnly){
  assert(count==0);
 }else{
  assert(count==1&&e[0].kind==1&&e[0].offset==19&&e[0].id==1&&e[0].channel==9);
 }
 assert(n==0||n==32);
 if(n){assert(silence==expected_silence);for(unsigned i=0;i<n;++i){assert(l[i]==expected_left[i]);assert(r[i]==expected_right[i]);}}
 std::copy_n(l,n,ol);std::copy_n(r,n,orr);*out=silence;*d={};d->delivered_frames=n;++processes;return 0;
}
uint32_t __wrap_ap23_process_outputs(uint64_t id,uint32_t n,uint32_t mode,const ap8_event_t*e,uint32_t count,const ap10_context_t*c,uint64_t silence,const float*l,const float*r,float*const*outputs,uint32_t channels,uint64_t*flags,ap7_delivery_t*d,uint64_t entered){
 assert(mode==kRealtime && channels==(n?64u:2u));
 auto result=__wrap_if2_process(id,n,e,count,c,silence,l,r,outputs[0],outputs[1],flags,d,entered);
 for(unsigned ch=2;ch<channels;++ch){
  assert(bool(outputs[ch])==(last_output&&ch>=62));
  if(outputs[ch])std::fill_n(outputs[ch],n,float(ch)/64.f);
 }
 return result;
}
uint32_t __wrap_if2_close(uint64_t){++closes;return 0;}
}
int main(){
 char report_path[]="/tmp/lvb-midi0-XXXXXX";int report_fd=mkstemp(report_path);
 assert(report_fd>=0);assert(close(report_fd)==0);
 assert(setenv("LVB_AP3_REPORT",report_path,1)==0);
 HostApplication host;auto*p=new AP2::Processor;assert(p->initialize(&host)==kResultOk);
 BusInfo aux{},output{};assert(p->getBusCount(kAudio,kInput)==1);
 assert(p->getBusInfo(kAudio,kInput,0,aux)==kResultOk);
 assert(aux.mediaType==kAudio&&aux.direction==kInput&&aux.busType==kAux&&aux.channelCount==2);
 assert(std::u16string(reinterpret_cast<const char16_t*>(aux.name))==u"Aux input");
 assert(aux.flags&BusInfo::kDefaultActive);SpeakerArrangement arrangement=0;
 assert(p->getBusArrangement(kInput,0,arrangement)==kResultOk&&arrangement==SpeakerArr::kStereo);
 assert(p->getBusInfo(kAudio,kOutput,0,output)==kResultOk&&output.busType==kMain);
 assert(p->getBusCount(kAudio,kOutput)==32);
 for(int i=1;i<32;++i){
  BusInfo info{};assert(p->getBusInfo(kAudio,kOutput,i,info)==kResultOk);
  assert(info.busType==kMain&&info.channelCount==2&&(info.flags&BusInfo::kDefaultActive));
  assert(p->activateBus(kAudio,kOutput,i,true)==kResultOk);
  assert(p->activateBus(kAudio,kOutput,i,false)==kResultOk);
 }
 assert(p->activateBus(kAudio,kInput,0,true)==kResultOk);
 assert(p->activateBus(kAudio,kOutput,0,true)==kResultOk);
 ProcessSetup setup{kRealtime,kSample32,512,48000};assert(p->setupProcessing(setup)==kResultOk);
 assert(p->setActive(true)==kResultOk&&setups==2&&activations==1);assert(p->setProcessing(true)==kResultOk);
 // Guarded buffers with unused tails: no-source -> routed source -> source gone.
 std::array<float,36> left{},right{},ol{},orr{};left.fill(99);right.fill(99);ol.fill(99);orr.fill(99);
 float* in[]={left.data()+1,right.data()+1};float* out[]={ol.data()+1,orr.data()+1};
 AudioBusBuffers ib{},ob{};ib.numChannels=ob.numChannels=2;ib.channelBuffers32=in;ob.channelBuffers32=out;
 EventList notes;Event note{};note.type=Event::kNoteOnEvent;note.sampleOffset=7;note.noteOn={0,60,0,.5f,32,1};assert(notes.addEvent(note)==kResultOk);
 ParameterChanges parameters;int32 qi=0,pi=0;parameters.addParameterData(0,qi)->addPoint(11,.75,pi);
 ProcessData d{};d.processMode=kRealtime;d.symbolicSampleSize=kSample32;d.numSamples=32;d.numInputs=d.numOutputs=1;d.inputs=&ib;d.outputs=&ob;d.inputEvents=&notes;d.inputParameterChanges=&parameters;
 auto run=[&]{std::copy(expected_left.begin(),expected_left.end(),in[0]);std::copy(expected_right.begin(),expected_right.end(),in[1]);const auto l=left,r=right;assert(p->process(d)==kResultOk);assert(left==l&&right==r);for(size_t i=0;i<32;++i){assert(out[0][i]==expected_left[i]);assert(out[1][i]==expected_right[i]);}for(auto*b:{&ol,&orr}){assert((*b)[0]==99);for(size_t i=33;i<36;++i)assert((*b)[i]==99);}};
 ib.silenceFlags=3;run();
 for(unsigned i=0;i<32;++i){expected_left[i]=float(i+1)/64;expected_right[i]=-float(i+1)/128;}expected_silence=0;ib.silenceFlags=0;run();
 // Preserve the existing correction of false silence hints without dropping audio.
 ib.silenceFlags=3;run();
 expected_left.fill(0);expected_right.fill(0);expected_silence=3;run();
 // Push-style note expression can share a block with an ordinary note-off.
 // The unsupported expressive events must not discard either note event or
 // make the whole audio callback fail.
 input_case=InputCase::MixedExpression;notes.clear();parameters.clearQueue();
 note.noteOn.channel=9;notes.addEvent(note);
 Event pressure{};pressure.type=Event::kPolyPressureEvent;pressure.sampleOffset=11;
 pressure.polyPressure={9,60,.7f,1};notes.addEvent(pressure);
 Event expression{};expression.type=Event::kNoteExpressionValueEvent;expression.sampleOffset=15;
 expression.noteExpressionValue={1,1,.4};notes.addEvent(expression);
 std::array<TChar,2> expression_text{u'x',0};
 Event text_expression{};text_expression.type=Event::kNoteExpressionTextEvent;
 text_expression.sampleOffset=17;
 text_expression.noteExpressionText={1,1,1,expression_text.data()};notes.addEvent(text_expression);
 Event off{};off.type=Event::kNoteOffEvent;off.sampleOffset=19;
 off.noteOff.channel=9;off.noteOff.pitch=60;off.noteOff.velocity=.2f;
 off.noteOff.noteId=1;off.noteOff.tuning=0;notes.addEvent(off);run();
 input_case=InputCase::ExpressionOnly;notes.clear();notes.addEvent(pressure);notes.addEvent(expression);notes.addEvent(text_expression);run();
 input_case=InputCase::NoteOffOnly;notes.clear();notes.addEvent(off);run();
 // A late host note-off releases its exact voice at the first available sample.
 // Subsequent valid notes and audio must continue on the same processor. This
 // is the observed -1661 host input, not a weakened unsigned wire validator.
 auto late_audit_begin=reinterpret_cast<void(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_begin"));
 auto late_audit_end=reinterpret_cast<uint64_t(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_end"));
 assert(late_audit_begin&&late_audit_end);
 input_case=InputCase::LateNoteOff;
 Event late=off;late.noteOff.noteId=-1;late.noteOff.pitch=63;late.noteOff.velocity=.25f;
 for(int offset : {-1661,-1}){
  late.sampleOffset=offset;notes.clear();notes.addEvent(late);notes.addEvent(note);
  late_audit_begin();auto result=p->process(d);auto effects=late_audit_end();
  assert(result==kResultOk&&effects==0);
 }
 input_case=InputCase::NoteOffOnly;notes.clear();notes.addEvent(off);run();
 auto before_unknown=processes;off.type=999;notes.clear();notes.addEvent(off);
 assert(p->process(d)!=kResultOk&&processes==before_unknown);
 input_case=InputCase::Original;notes.clear();note.noteOn.channel=0;notes.addEvent(note);
 parameters.addParameterData(0,qi)->addPoint(11,.75,pi);
 // A DAW can supply all declared buses or omit trailing inactive buses. Neither
 // form may touch inactive storage or change the transported sample positions.
 std::array<AudioBusBuffers,32> outputs{};outputs[0]=ob;
 float* inactive[]={nullptr,nullptr};
 for(size_t i=1;i<outputs.size();++i){outputs[i].numChannels=2;outputs[i].channelBuffers32=inactive;}
 d.numOutputs=32;d.outputs=outputs.data();run();
 outputs[31].numChannels=1;auto count=processes;
 assert(p->process(d)!=kResultOk&&processes==count);outputs[31].numChannels=2;
 d.numOutputs=1;d.outputs=&ob;
 auto before=processes;ib.channelBuffers32=nullptr;assert(p->process(d)!=kResultOk&&processes==before);ib.channelBuffers32=in;
 in[1]=nullptr;assert(p->process(d)!=kResultOk&&processes==before);in[1]=right.data()+1;
 ib.numChannels=1;assert(p->process(d)!=kResultOk&&processes==before);ib.numChannels=2;
 ib.silenceFlags=4;assert(p->process(d)!=kResultOk&&processes==before);ib.silenceFlags=3;
 in[1]=in[0];assert(p->process(d)!=kResultOk&&processes==before);in[1]=right.data()+1;
 out[0]=in[1];assert(p->process(d)!=kResultOk&&processes==before);out[0]=ol.data()+1;
 // Inactive SDK bus may omit all channel pointers; stale nonzero input is ignored.
 assert(p->setProcessing(false)==kResultOk);assert(p->setActive(false)==kResultOk);
 input_enabled=false;assert(p->activateBus(kAudio,kInput,0,false)==kResultOk);assert(p->setActive(true)==kResultOk);assert(p->setProcessing(true)==kResultOk);
 left.fill(.8f);right.fill(-.4f);ib.channelBuffers32=nullptr;assert(p->process(d)==kResultOk);
 for(unsigned i=0;i<32;++i)assert(out[0][i]==0&&out[1][i]==0);
 // Activate output 31 and verify both independent planes reach the actual SDK host.
 assert(p->setProcessing(false)==kResultOk);assert(p->setActive(false)==kResultOk);
 last_output=true;assert(p->activateBus(kAudio,kOutput,31,true)==kResultOk);
 assert(p->setActive(true)==kResultOk);assert(p->setProcessing(true)==kResultOk);
 std::array<float,36> extra_left{},extra_right{};extra_left.fill(99);extra_right.fill(99);
 float* extra[]={extra_left.data()+1,extra_right.data()+1};
 outputs[31].channelBuffers32=extra;d.numOutputs=32;d.outputs=outputs.data();
 assert(p->process(d)==kResultOk);
 for(unsigned i=1;i<33;++i){assert(extra_left[i]==62.f/64);assert(extra_right[i]==63.f/64);}
 assert(extra_left[0]==99&&extra_left[33]==99&&extra_right[0]==99&&extra_right[33]==99);
 auto processed=processes;outputs[31].channelBuffers32=inactive;
 assert(p->process(d)!=kResultOk&&processes==processed);outputs[31].channelBuffers32=extra;
 d.numOutputs=1;d.outputs=&ob;assert(p->process(d)!=kResultOk&&processes==processed);
 // Zero-frame event/parameter flush remains independent of input storage.
 notes.clear();note.sampleOffset=0;notes.addEvent(note);parameters.clearQueue();parameters.addParameterData(0,qi)->addPoint(0,.5,pi);
 d.numSamples=0;d.numInputs=d.numOutputs=0;d.inputs=d.outputs=nullptr;assert(p->process(d)==kResultOk);
 // A shortened host block with an out-of-block automation point remains a
 // refused input. Its exact bounded witness is retained without callback I/O
 // or allocation; later rejected callbacks cannot overwrite that witness.
 auto audit_begin=reinterpret_cast<void(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_begin"));
 auto audit_end=reinterpret_cast<uint64_t(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_end"));
 assert(audit_begin&&audit_end);
 // The SDK boundary preserves a legitimate linear-curve endpoint for the
 // Rust owner to segment. It does not shift the endpoint onto sample 31.
 input_case=InputCase::EndpointCurve;notes.clear();parameters.clearQueue();
 auto* curve=parameters.addParameterData(0,qi);
 curve->addPoint(0,.25,pi);curve->addPoint(32,.75,pi);
 d.numSamples=32;d.numInputs=1;d.numOutputs=32;d.inputs=&ib;d.outputs=outputs.data();
 audit_begin();auto admitted=p->process(d);auto curve_effects=audit_end();
 assert(admitted==kResultOk&&curve_effects==0);
 input_case=InputCase::InvalidParameterExtent;notes.clear();parameters.clearQueue();
 parameters.addParameterData(0,qi)->addPoint(33,.75,pi);
 d.numSamples=32;d.numInputs=1;d.numOutputs=32;d.inputs=&ib;d.outputs=outputs.data();
 audit_begin();auto refused=p->process(d);auto effects=audit_end();
 assert(refused!=kResultOk&&effects==0);
 assert(p->process(d)!=kResultOk);
 assert(p->setProcessing(false)==kResultOk);assert(p->setActive(false)==kResultOk);
 // Processing remains refused above; this result describes successful cleanup.
 assert(p->terminate()==kResultOk);p->release();assert(closes==1);
 std::ifstream report(report_path);assert(report.good());
 std::string contents(std::istreambuf_iterator<char>{report},{});
 assert(contents.find("\"skipped_expression_callbacks\":2")!=std::string::npos);
 assert(contents.find("\"late_note_offs\":2")!=std::string::npos);
 assert(contents.find("\"event\":\"ap10_admission_failure\",\"code\":258,\"frames\":32")!=std::string::npos);
 assert(contents.find("\"event_count\":1,\"invalid_event_index\":0,\"invalid_event\":{\"kind\":2,\"id\":0,\"offset\":33")!=std::string::npos);
 report.close();assert(unlink(report_path)==0);assert(unsetenv("LVB_AP3_REPORT")==0);
 std::puts("AP18 native sole auxiliary samples/silence/guards/events PASS");
}
