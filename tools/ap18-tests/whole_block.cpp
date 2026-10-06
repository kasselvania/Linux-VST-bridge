// Production Windows decoder and SDK queues must deliver one whole host block.
// Only the plug-in owns its implicit preceding parameter value.
#include "ap8_events.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include <cassert>
#include <cmath>
#include <iostream>
using namespace linux_vst_bridge::ap1;
using namespace linux_vst_bridge::wf0;
using namespace Steinberg;
using namespace Steinberg::Vst;
struct CurveFixture final : AudioEffect {
 double current = .25;
 unsigned calls = 0;
 tresult PLUGIN_API process(ProcessData& data) override {
  assert(data.numSamples == 1008 && data.inputParameterChanges->getParameterCount() == 1);
  auto* queue = data.inputParameterChanges->getParameterData(0);
  assert(queue && queue->getParameterId() == 7 && queue->getPointCount() == 1);
  int32 offset = 0; double value = 0.;
  assert(queue->getPoint(0, offset, value) == kResultOk && offset == 1007 && value == .75);
  const double prior = current;
  for (int i = 0; i < data.numSamples; ++i)
   data.outputs[0].channelBuffers32[0][i] = float(prior + (value-prior)*(i+1)/(offset+1));
  current = value; ++calls; return kResultOk;
 }
};
int main() {
 Frame full{Process, {}, 1, std::vector<uint8_t>(88)};
 put(full.payload.data(), 1008, 4); put(full.payload.data()+4, block_layout.input, 4);
 put(full.payload.data()+8, block_layout.output, 4); put(full.payload.data()+12, block_layout.stride, 4);
 put(full.payload.data()+48, 1, 4); put(full.payload.data()+56, 1007, 4);
 put(full.payload.data()+60, 2, 4); put(full.payload.data()+64, 7, 4);
 const double end=.75; std::memcpy(full.payload.data()+72, &end, 8);
 assert(decode(encode(full,14),14).payload == full.payload);
 bool refused=false; try {decode(encode(full,14),13);} catch (...) {refused=true;}
 assert(refused);
 Frame base=full; base.payload.resize(32);
 assert(request(base,true,block_layout).frames == 1008);
 refused=false; try {request(base,true);} catch (...) {refused=true;} assert(refused);
 auto wrong=base; put(wrong.payload.data()+8, output_offset, 4);
 refused=false; try {request(wrong,true,block_layout);} catch (...) {refused=true;} assert(refused);
 wrong=base; put(wrong.payload.data(),1025,4);
 refused=false; try {request(wrong,true,block_layout);} catch (...) {refused=true;} assert(refused);
 std::array<InputEvent,event_capacity> events{}; Notes notes; Changes changes;
 assert(decode_events(full.payload,events,1008,0,true) == 1);
 changes.load(events.data(),1,notes);
 std::array<float,1024> samples{}; float* channels[]={samples.data()};
 AudioBusBuffers bus{}; bus.numChannels=1; bus.channelBuffers32=channels;
 ProcessData data{}; data.numSamples=1008; data.numOutputs=1; data.outputs=&bus;
 data.inputParameterChanges=&changes;
 CurveFixture plugin;
 assert(plugin.process(data)==kResultOk && plugin.calls==1);
 assert(std::abs(samples[0]-float(.25+.5/1008))<1e-7 && samples[1007]==.75f);
 // A GUI/state change belongs to the vendor. No bridge-created anchor.
 plugin.current=.5;
 assert(plugin.process(data)==kResultOk && plugin.calls==2);
 assert(std::abs(samples[0]-float(.5+.25/1008))<1e-7);
 put(full.payload.data()+56,1008,4);
 assert(decode_events(full.payload,events,1008,0,true)==1);
 changes.load(events.data(),1,notes);
 int32 offset=0; double value=0.;
 assert(changes.getParameterData(0)->getPoint(0,offset,value)==kResultOk && offset==1008 && value==.75);
 refused=false; try {decode_events(full.payload,events,1008);} catch (...) {refused=true;} assert(refused);
 put(full.payload.data()+60,0,4); // Notes cannot use a parameter's end anchor.
 refused=false; try {decode_events(full.payload,events,1008,0,true);} catch (...) {refused=true;} assert(refused);
 std::cout << "WHOLE_BLOCK_CONTRACT_V1 protocol=14 mapping=3 frames=1008 implicit=vendor_owned passed\n";
}
