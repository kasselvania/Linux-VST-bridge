#include "native-vst3-proxy/source/output_results.h"
#include "windows-factory-probe/source/bus_layout.h"
#include "windows-factory-probe/source/input_observation.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include <cassert>
#include <iostream>
#include <tuple>
#include <limits>
using namespace AP10Results;
// Production BusLayout reads the SDK object's real bus declarations. No
// Pigments name dispatch: a sole auxiliary input uses the one explicitly selected stereo lane.
struct AuxiliaryInstrument final : Steinberg::Vst::AudioEffect {
 bool silent=true;
 AuxiliaryInstrument(bool main=false,bool auxiliary=true){using namespace Steinberg::Vst;if(main)addAudioInput(STR16("Track"),SpeakerArr::kStereo,kMain);if(auxiliary)addAudioInput(STR16("Sidechain"),SpeakerArr::kStereo,kAux);addAudioOutput(STR16("Stereo Out"),SpeakerArr::kStereo);addEventInput(STR16("Notes"),16);}
 Steinberg::tresult PLUGIN_API process(Steinberg::Vst::ProcessData& d) override {
  assert(d.numInputs==1&&d.numSamples==2);auto& b=d.inputs[0];assert(b.numChannels==2);
  assert(b.silenceFlags==(silent?3u:0u));
  assert(b.channelBuffers32[0][0]==(silent?0.f:.25f)&&b.channelBuffers32[0][1]==(silent?0.f:.5f));
  assert(b.channelBuffers32[1][0]==(silent?0.f:-.75f)&&b.channelBuffers32[1][1]==(silent?0.f:-.125f));
  return Steinberg::kResultOk;
 }
};

static void rejection_tests(){
 Collector c;auto reset=[&]{c.reset(16);c.buses=1;c.channels[0]=16;c.bus_active[0]=1;};
 auto event=[] {Event e{};e.type=Event::kNoteOnEvent;e.noteOn={0,60,0,.5f,0,7};return e;};
 auto check=[&](Event e,Rejection reason){assert(c.addEvent(e)==kResultFalse);assert(c.failed&&c.rejection.reason==reason);};
 reset();auto e=event();for(unsigned i=0;i<ap10_event_capacity;++i)assert(c.addEvent(e)==kResultOk);check(e,Rejection::EventCapacity);assert(c.rejection.events==64);
 auto first=c.rejection;e.type=999;check(e,Rejection::EventCapacity);int32 index=0;assert(!c.addParameterData(9,index));assert(std::memcmp(&first,&c.rejection,sizeof(first))==0);
 reset();assert(c.rejection.reason==Rejection::None&&!c.failed);
 for(auto [bus,declared,reason]:{std::tuple{-1,1,Rejection::NegativeBus},std::tuple{1,1,Rejection::UndeclaredBus},std::tuple{8,9,Rejection::BusStorage}}){reset();c.buses=declared;e=event();e.busIndex=bus;check(e,reason);assert(c.rejection.bus==bus);}
 reset();c.channels[0]=0;e=event();check(e,Rejection::EventChannel);assert(c.rejection.channel==0&&c.rejection.declared_channels==0&&c.rejection.bus==0&&c.rejection.event_type==Event::kNoteOnEvent&&c.rejection.event_bus_active==1&&c.rejection.events==0);
 for(auto [offset,reason]:{std::pair{-1,Rejection::NegativeEventOffset},std::pair{16,Rejection::EventExtent}}){reset();e=event();e.sampleOffset=offset;check(e,reason);}
 reset();e=event();e.ppqPosition=std::numeric_limits<double>::infinity();check(e,Rejection::NonFinitePPQ);
 reset();e=event();e.type=999;check(e,Rejection::UnsupportedEvent);
 reset();e=event();e.noteOn.pitch=128;check(e,Rejection::InvalidEventField);assert(c.rejection.field_a==128);
 std::array<uint8_t,513> bytes{};auto data=[&](uint32_t n){Event d{};d.type=Event::kDataEvent;d.data={n,DataEvent::kMidiSysEx,bytes.data()};return d;};
 reset();check(data(513),Rejection::EventPayload);assert(c.rejection.payload_size==513);
 reset();for(int i=0;i<8;++i){e=data(512);assert(c.addEvent(e)==kResultOk);}check(data(1),Rejection::PayloadCapacity);assert(c.rejection.bytes==4096&&c.rejection.events==8);
 reset();e=data(1);e.data.bytes=nullptr;check(e,Rejection::NullPayload);
 reset();for(int i=0;i<7;++i){e=data(512);assert(c.addEvent(e)==kResultOk);}e=data(511);assert(c.addEvent(e)==kResultOk);check(data(1),Rejection::PayloadAlignment);assert(c.rejection.bytes==4095);
 reset();for(uint32_t i=0;i<ap10_point_capacity;++i)assert(c.addParameterData(i,index));assert(!c.addParameterData(128,index));assert(c.rejection.reason==Rejection::QueueCapacity&&c.rejection.queues==128&&c.rejection.points==0&&c.rejection.parameter_id==128);
 reset();auto*q=c.addParameterData(123,index);for(uint32_t i=0;i<ap10_point_capacity;++i)assert(q->addPoint(0,.5,index)==kResultOk);assert(q->addPoint(0,.5,index)==kResultFalse);assert(c.rejection.reason==Rejection::PointCapacity&&c.rejection.points==128&&c.rejection.queues==1);
 for(auto [offset,value,reason]:{std::tuple{-1,.5,Rejection::NegativePointOffset},std::tuple{16,.5,Rejection::PointExtent},std::tuple{0,std::numeric_limits<double>::infinity(),Rejection::NonFiniteValue},std::tuple{0,std::numeric_limits<double>::quiet_NaN(),Rejection::NonFiniteValue},std::tuple{0,-.1,Rejection::ValueBelowZero},std::tuple{0,1.1,Rejection::ValueAboveOne}}){reset();q=c.addParameterData(123,index);assert(q->addPoint(offset,value,index)==kResultFalse);assert(c.rejection.reason==reason&&c.rejection.parameter_id==123&&c.rejection.offset==offset&&c.rejection.value_bits==std::bit_cast<uint64_t>(value));}
 // Existing accepted encoding is unchanged; diagnostic channel observes no payload.
 reset();static const ap10_results_t empty{};c.values=empty;ap10_results_t expected{};e=event();assert(append(expected,e,16)&&c.addEvent(e)==kResultOk);e=data(3);assert(append(expected,e,16)&&c.addEvent(e)==kResultOk);assert(std::memcmp(&expected,&c.values,sizeof(expected))==0);assert(c.rejection.reason==Rejection::None);
 c.reset(0);e=event();assert(c.addEvent(e)==kResultOk);q=c.addParameterData(1,index);assert(q->addPoint(0,.25,index)==kResultOk);e.sampleOffset=1;check(e,Rejection::EventExtent);
}

static void event_policy_tests(){
 using namespace linux_vst_bridge;using namespace Steinberg::Vst;
 AuxiliaryInstrument instrument;instrument.addEventOutput(STR16("Output"),0);
 wf0::BusLayout raw,corrected;raw.read(instrument,instrument,false);corrected.read(instrument,instrument,true);
 auto& original=raw.buses[raw.size-1];auto& effective=corrected.buses[corrected.size-1];
 assert(original.info.channelCount==0&&original.effective_channels==0);
 assert(effective.info.channelCount==0&&effective.effective_channels==16);
 std::vector<uint8_t> setup(28+32*corrected.size);ap1::put(setup.data()+20,1,4);ap1::put(setup.data()+24,corrected.size,4);
 std::array<unsigned,4> index{};
 for(size_t i=0;i<corrected.size;++i){auto& b=corrected.buses[i];auto* p=setup.data()+28+32*i;unsigned m=b.info.mediaType,d=b.info.direction;
  ap1::put(p,m,4);ap1::put(p+4,d,4);ap1::put(p+8,index[m*2+d]++,4);ap1::put(p+12,b.effective_channels,4);ap1::put(p+16,b.info.busType,4);ap1::put(p+20,b.active,4);ap1::put(p+24,b.arrangement,8);
 }
 corrected.contract(setup);bool refused=false;try{raw.contract(setup);}catch(...){refused=true;}assert(refused);
 Collector c;c.buses=1;c.channels[0]=effective.effective_channels;c.bus_active[0]=1;
 Event e{};e.type=Event::kNoteOnEvent;e.busIndex=0;e.sampleOffset=144;e.flags=3;e.noteOn={0,60,0,.787401556968689f,0,17};
 for(int channel:{0,15,-1,16}){c.reset(256);e.noteOn.channel=channel;auto result=c.addEvent(e);
  if(channel<0||channel>=16){assert(result==kResultFalse&&c.rejection.reason==Rejection::InvalidEventField);continue;}
  assert(result==kResultOk);ap10_results_t expected{};assert(append(expected,e,256));assert(std::memcmp(&expected,&c.values,sizeof(expected))==0);
  Output output;output.packet=c.values;Collector host;host.buses=1;host.channels[0]=16;host.reset(256);ProcessData data{};data.numSamples=256;data.outputEvents=&host;
  assert(output.deliver(data,[](int b){return b==0?1:-1;},[](uint32_t){return false;}));Event delivered{};assert(host.getEvent(0,delivered)==kResultOk);
  assert(delivered.busIndex==e.busIndex&&delivered.sampleOffset==e.sampleOffset&&delivered.flags==e.flags&&delivered.noteOn.channel==channel&&delivered.noteOn.pitch==60&&delivered.noteOn.velocity==e.noteOn.velocity&&delivered.noteOn.noteId==17);
  output.release_requested=true;assert(output.release(data,[](int){return 1;}));assert(output.active_notes()==0);
 }
 c.reset(256);c.channels[0]=original.effective_channels;e.noteOn.channel=0;assert(c.addEvent(e)==kResultFalse&&c.rejection.reason==Rejection::EventChannel);
 AuxiliaryInstrument positive;positive.addEventOutput(STR16("Output"),7);raw.read(positive,positive,false);assert(raw.buses[raw.size-1].effective_channels==7);
}

int main(){
 event_policy_tests();
 rejection_tests();
 {using namespace linux_vst_bridge::wf0;InputObservation t;float z[]={0,0},l[]={.25f,.5f},r[]={-.75f,-.125f};t.observe(1,1,0,2,3,z,z);t.observe(1,2,2,2,0,l,r);t.observe(1,3,4,2,0,l,r);t.observe(1,4,6,2,3,z,z);assert(t.count==3&&t.rows[1].sequence==2&&t.rows[0].hash==t.rows[2].hash&&t.rows[1].hash[0]!=t.rows[1].hash[1]);assert(t.rows[1].hash[0]==1596836236129080722ull&&t.rows[1].hash[1]==15537787510424757890ull);for(unsigned i=0;i<80;++i)t.observe(1,i,i,2,0,i%2?z:l,i%2?z:r);assert(t.count==32&&t.overflow>0);}

 {using namespace linux_vst_bridge;AuxiliaryInstrument instrument;wf0::BusLayout layout;layout.read(instrument,instrument);
 assert(layout.counts[0]==1&&layout.counts[1]==1);assert(layout.buses[0].supported&&layout.buses[0].active);assert(layout.buses[1].supported);
 std::vector<uint8_t> bytes(28+32*layout.size);ap1::put(bytes.data()+20,1,4);ap1::put(bytes.data()+24,layout.size,4);std::array<int,4> indices{};
 for(size_t i=0;i<layout.size;++i){auto&b=layout.buses[i];auto*r=bytes.data()+28+32*i;auto m=b.info.mediaType,d=b.info.direction;ap1::put(r,m,4);ap1::put(r+4,d,4);ap1::put(r+8,indices[m*2+d]++,4);ap1::put(r+12,b.info.channelCount,4);ap1::put(r+16,b.info.busType,4);ap1::put(r+20,b.active,4);ap1::put(r+24,b.arrangement,8);}
 layout.contract(bytes);layout.activate(instrument,true);assert(layout.buses[0].active);
 std::array<Steinberg::Vst::AudioBusBuffers,8> inputs{};float left[]={.25f,.5f},right[]={-.75f,-.125f},zero[]={0,0};float* pair[]={left,right};float* zeros[]={zero,zero};
 layout.map_inputs(inputs,pair,zeros,0);assert(inputs[0].channelBuffers32[0]==left&&inputs[0].channelBuffers32[1]==right&&inputs[0].silenceFlags==0);
 Steinberg::Vst::ProcessData data{};data.numInputs=1;data.inputs=inputs.data();data.numSamples=2;instrument.silent=false;assert(instrument.process(data)==Steinberg::kResultOk);
 layout.map_inputs(inputs,zeros,zeros,3);instrument.silent=true;assert(instrument.process(data)==Steinberg::kResultOk);
 ap1::put(bytes.data()+28+20,0,4);layout.contract(bytes);layout.map_inputs(inputs,pair,zeros,0);assert(inputs[0].channelBuffers32[0][0]==0&&inputs[0].channelBuffers32[1][1]==0&&inputs[0].silenceFlags==3);assert(instrument.process(data)==Steinberg::kResultOk);layout.activate(instrument,false);
 // Main plus auxiliary preserves main routing and refuses auxiliary activation.
 AuxiliaryInstrument effect(true);wf0::BusLayout fx;fx.read(effect,effect);
 assert(fx.transported_input==0&&fx.buses[0].supported&&!fx.buses[1].supported&&!fx.buses[1].active);
 fx.map_inputs(inputs,pair,zeros,0);assert(inputs[0].channelBuffers32[0]==left&&inputs[1].channelBuffers32[0][0]==0&&inputs[1].silenceFlags==3);
 auto contract=[](const wf0::BusLayout& l){std::vector<uint8_t> v(28+32*l.size);ap1::put(v.data()+20,1,4);ap1::put(v.data()+24,l.size,4);std::array<int,4> ix{};for(size_t i=0;i<l.size;++i){const auto& b=l.buses[i];auto*r=v.data()+28+32*i;auto m=b.info.mediaType,d=b.info.direction;ap1::put(r,m,4);ap1::put(r+4,d,4);ap1::put(r+8,ix[m*2+d]++,4);ap1::put(r+12,b.info.channelCount,4);ap1::put(r+16,b.info.busType,4);ap1::put(r+20,b.active,4);ap1::put(r+24,b.arrangement,8);}return v;};
 auto bad=contract(fx);ap1::put(bad.data()+28+32+20,1,4);bool refused=false;try{fx.contract(bad);}catch(...){refused=true;}assert(refused);
 AuxiliaryInstrument no_input(false,false);wf0::BusLayout synth;synth.read(no_input,no_input);assert(synth.transported_input==-1&&synth.counts[0]==0);
 }

 {using namespace linux_vst_bridge;wf0::BusLayout b;b.size=1;auto&out=b.buses[0];out.info.mediaType=1;out.info.direction=1;out.info.channelCount=16;out.effective_channels=16;out.info.busType=0;out.info.flags=0;out.supported=true;out.active=false;
std::vector<uint8_t> p(60);ap1::put(p.data()+20,1,4);ap1::put(p.data()+24,1,4);ap1::put(p.data()+28,1,4);ap1::put(p.data()+32,1,4);ap1::put(p.data()+40,16,4);ap1::put(p.data()+48,1,4);b.contract(p);assert(b.buses[0].active);
ap1::put(p.data()+48,0,4);b.contract(p);assert(!b.buses[0].active);out.supported=false;ap1::put(p.data()+48,1,4);bool refused=false;try{b.contract(p);}catch(...){refused=true;}assert(refused);}

 Collector c;c.buses=1;c.channels[0]=16;c.reset(128);
 std::array<Event,10> events{};uint16_t kinds[]={0,1,2,3,4,5,6,7,8,65535};
 uint8_t sysex[]={0xf0,0x01,0xf7};TChar text[]={u'A',u'B',0};
 for(int i=0;i<10;++i){auto&e=events[i];e.type=kinds[i];e.sampleOffset=i;e.flags=0xc001;e.ppqPosition=7.25;
  switch(e.type){
   case 0:e.noteOn={2,60,1.5f,0.75f,99,-1234};break;
   case 1:e.noteOff={2,60,0.25f,-1234,-2.5f};break;
   case 2:e.data={3,0,sysex};break;
   case 3:e.polyPressure={2,60,0.5f,-1234};break;
   case 4:e.noteExpressionValue={123,-1234,0.25};break;
   case 5:e.noteExpressionText={124,-1234,2,text};break;
   case 6:e.chord={60,48,-1,2,text};break;
   case 7:e.scale={60,-1,2,text};break;
   case 8:e.noteExpressionIntValue={125,-1234,0xfedcba9876543210ULL};break;
   case 65535:e.midiCCOut={129,2,32,64};break;
  }
  assert(c.addEvent(e)==kResultOk);
 }
 sysex[1]=99;text[0]=u'Z'; // producer storage may die immediately after addEvent.
 for(int i=0;i<10;++i){Event e{};assert(c.getEvent(i,e)==kResultOk);assert(e.type==kinds[i]&&e.flags==0xc001&&e.ppqPosition==7.25&&e.sampleOffset==i);if(i==2)assert(e.data.bytes[1]==1);if(i==5)assert(e.noteExpressionText.text[0]==u'A');if(i==6)assert(e.chord.mask==-1);if(i==8)assert(e.noteExpressionIntValue.value==0xfedcba9876543210ULL);}
 int32 index=0;auto*q=c.addParameterData(42,index);assert(q&&q->addPoint(13,0.75,index)==kResultOk);assert(q->getPointCount()==1);
 Output o;o.packet=c.values;Collector host;host.buses=1;host.channels[0]=16;host.reset(128);ProcessData d{};d.numSamples=128;d.outputEvents=&host;d.outputParameterChanges=&host;
 assert(o.deliver(d,[](int b){return b==0?1:-1;},[](uint32_t id){return id==42;}));assert(o.events==10&&o.points==1&&o.active_notes()==0);
 c.reset(128);auto on=events[0];assert(c.addEvent(on)==kResultOk);o.packet=c.values;assert(o.deliver(d,[](int){return 1;},[](uint32_t){return true;}));assert(o.active_notes()==1);o.release_requested=true;d.outputEvents=nullptr;assert(!o.release(d,[](int){return 1;}));assert(o.active_notes()==1);d.outputEvents=&host;assert(o.release(d,[](int){return 1;}));assert(o.active_notes()==0&&o.cleanup_sent==1);
 d.outputEvents=nullptr;d.outputParameterChanges=nullptr;o.packet=c.values;assert(o.deliver(d,[](int){return 1;},[](uint32_t){return true;}));assert(o.unrequested_events==1&&o.active_notes()==0);d.outputEvents=&host;assert(o.deliver(d,[](int){return 0;},[](uint32_t){return true;}));assert(o.unrequested_events==2);
 c.reset(0);q=c.addParameterData(42,index);assert(q->addPoint(0,0.5,index)==kResultOk);assert(q->addPoint(1,0.5,index)!=kResultOk&&c.failed);
 c.reset(128);for(int i=0;i<64;++i)assert(c.addEvent(on)==kResultOk);assert(c.addEvent(on)!=kResultOk&&c.failed);
 c.reset(128);auto bad=events[2];bad.data.size=513;assert(c.addEvent(bad)!=kResultOk&&c.failed);
 c.reset(128);std::array<uint8_t,512> bounded{};bad.data={512,0,bounded.data()};for(int i=0;i<8;++i)assert(c.addEvent(bad)==kResultOk);assert(c.values.bytes==4096);assert(c.addEvent(bad)!=kResultOk&&c.failed);
 c.reset(128);q=c.addParameterData(42,index);for(int i=0;i<128;++i)assert(q->addPoint(0,.25,index)==kResultOk);assert(q->addPoint(0,.25,index)!=kResultOk&&c.failed);
 // The native delivery helper must surface a real SDK addPoint refusal.
 host.reset(128);q=host.addParameterData(42,index);for(int i=0;i<128;++i)assert(q->addPoint(0,.5,index)==kResultOk);
 o.packet=ap10_results_t{};o.packet.points=1;o.packet.point[0]={0,42,.25};d.outputParameterChanges=&host;auto rejected=o.rejected;
 assert(!o.deliver(d,[](int){return 1;},[](uint32_t){return true;}));assert(o.rejected==rejected+1&&host.failed);
 std::cout<<"SDK process-result variants, owned payloads, output sinks, cleanup and bounds PASS\n";
}
