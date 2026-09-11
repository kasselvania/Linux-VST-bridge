#include "native-vst3-proxy/source/output_results.h"
#include "windows-factory-probe/source/bus_layout.h"
#include "windows-factory-probe/source/input_observation.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include <cassert>
#include <iostream>
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

int main(){
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

 {using namespace linux_vst_bridge;wf0::BusLayout b;b.size=1;auto&out=b.buses[0];out.info.mediaType=1;out.info.direction=1;out.info.channelCount=16;out.info.busType=0;out.info.flags=0;out.supported=true;out.active=false;
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
