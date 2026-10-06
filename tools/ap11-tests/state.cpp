// Real SDK state interfaces through production commercial_state(). Reading a
// bundle must not restore it into the editor and restart the host again.
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "windows-factory-probe/source/ap8_state.h"
#include <algorithm>
#include <array>
#include <cmath>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <limits>
#include <vector>
using namespace Steinberg;
using namespace Steinberg::Vst;
void check(bool ok, const char *why) {
  if (!ok) {
    std::cerr << "FAIL: " << why << '\n';
    std::exit(1);
  }
}
struct FixtureComponent final : AudioEffect {
  double value = .375;
  unsigned restores = 0;
  tresult capture_result=kResultOk;
  bool oversized=false;
  bool whole_stream=false;
  size_t recording=0;
  tresult PLUGIN_API getState(IBStream *s) override {
    if(capture_result!=kResultOk)return capture_result;
    int32 n = 0;
    if(oversized)return s->write(&value,int32(LVBState::payloadLimit+1),&n);
    if(s->write(&value,8,&n)!=kResultOk||n!=8)return kResultFalse;
    std::array<uint8_t,4096> audio{};audio.fill(0xd3);
    for(size_t sent=0;sent<recording;){
      auto size=int32(std::min(audio.size(),recording-sent));
      if(s->write(audio.data(),size,&n)!=kResultOk||n!=size)return kResultFalse;
      sent+=size;
    }
    return kResultOk;
  }
  tresult PLUGIN_API setState(IBStream *s) override {
    int32 n = 0;
    ++restores;
    if(whole_stream){
      // The reader a widely deployed plug-in framework uses when a stream
      // cannot report its size: fixed blocks until no bytes arrive, and a
      // block is used only when its read is reported as a success.
      std::vector<uint8_t> all;std::array<uint8_t,4096> block{};
      for(;;){
        n=0;auto status=s->read(block.data(),int32(block.size()),&n);
        if(n<=0||status!=kResultTrue)break;
        all.insert(all.end(),block.begin(),block.begin()+n);
      }
      if(all.size()!=8+recording||!std::all_of(all.begin()+8,all.end(),[](auto b){return b==0xd3;}))return kResultFalse;
      std::memcpy(&value,all.data(),8);
      return kResultOk;
    }
    if(s->read(&value,8,&n)!=kResultOk||n!=8)return kResultFalse;
    std::array<uint8_t,4096> audio{};
    for(size_t read=0;read<recording;){
      auto size=int32(std::min(audio.size(),recording-read));
      if(s->read(audio.data(),size,&n)!=kResultOk||n!=size||
         !std::all_of(audio.begin(),audio.begin()+n,[](auto b){return b==0xd3;}))return kResultFalse;
      read+=size;
    }
    return kResultOk;
  }
};
struct Controller final : EditController {
  unsigned synchronizations = 0, invalidations = 0;
  bool invalid_readback = false;
  bool oversized=false;
  tresult PLUGIN_API getState(IBStream *s) override {
    if(!oversized)return EditController::getState(s);
    int32 n=0;uint8_t byte=0;
    return s->write(&byte,int32(LVBState::payloadLimit+1),&n);
  }
  double readback_value = 0.;
  ParamValue PLUGIN_API getParamNormalized(ParamID id) override {
    return invalid_readback && id == 42 ? readback_value
                                      : EditController::getParamNormalized(id);
  }
  tresult PLUGIN_API initialize(FUnknown *h) override {
    auto r = EditController::initialize(h);
    parameters.addParameter(u"Gain", u"", 0, .375, ParameterInfo::kCanAutomate,
                            42);
    return r;
  }
  tresult PLUGIN_API setComponentState(IBStream *s) override {
    ++synchronizations;
    double value = 0;
    int32 n = 0;
    if (s->read(&value, 8, &n) != kResultOk || n != 8)
      return kResultFalse;
    ++invalidations;
    return EditController::setParamNormalized(42, value);
  }
};
int main() {
  HostApplication host;
  FixtureComponent component;
  Controller controller;
  check(component.initialize(&host) == kResultOk &&
            controller.initialize(&host) == kResultOk,
        "SDK initialize");
  using linux_vst_bridge::wf0::commercial_state;
  auto original = commercial_state(component, controller, true, nullptr);
  check(controller.synchronizations == 0 && controller.invalidations == 0,
        "state capture must not reapply component state or invalidate "
        "parameters");
  for (int i = 0; i < 32; ++i)
    check(commercial_state(component, controller, true, nullptr) == original,
          "repeated capture remains pure");
  component.value = .75;
  controller.setParamNormalized(42, .75);
  auto recalled = commercial_state(component, controller, true, &original);
  check(component.restores == 1 && controller.synchronizations == 1 &&
            controller.invalidations == 1,
        "restore synchronizes controller exactly once");
  check(component.value == .375 && controller.getParamNormalized(42) == .375 &&
            recalled == original,
        "opaque restore and controller readback retained");
  check(commercial_state(component, controller, true, nullptr) == original &&
            controller.synchronizations == 1,
        "post-restore capture does not trigger another refresh");
  controller.invalid_readback = true;
  for (auto value : {-.25, 1.25, std::numeric_limits<double>::quiet_NaN()}) {
    controller.readback_value = value;
    auto captured=commercial_state(component, controller, true, nullptr);
    check(captured.size()==original.size(),"unavailable parameter retains every ID");
    check(std::equal(original.begin(),original.end()-16,captured.begin()),"opaque component/controller state preserved");
    auto* p=captured.data()+captured.size()-16;
    check(linux_vst_bridge::ap1::get(p,4)==42&&linux_vst_bridge::ap1::get(p+4,4)==0&&linux_vst_bridge::ap1::get(p+8,8)==0,"explicit unavailable tag, no invalid value or clamping");
  }
  for(auto r:{kResultFalse,kNotImplemented}){
    component.capture_result=r;
    bool refused=false;
    try{commercial_state(component,controller,true,nullptr);}
    catch(const linux_vst_bridge::wf0::SaveRefusal& e){refused=e.stage==1&&e.result==r;}
    check(refused,"completed capture refusal retains stage and SDK result");
    bool unsafe=false;
    try{commercial_state(component,controller,true,&original);}
    catch(const linux_vst_bridge::wf0::SaveRefusal&){check(false,"restore must never be recoverable save refusal");}
    catch(const std::runtime_error&){unsafe=true;}
    check(unsafe,"post-restore capture refusal remains unsafe");
  }
  component.capture_result=kResultOk;
  controller.invalid_readback=false;
  check(commercial_state(component,controller,true,nullptr)==original,"capture can succeed after refusal");
  component.recording=2*1024*1024;
  auto recorded=commercial_state(component,controller,true,nullptr);
  check(recorded.size()>2*1024*1024,"recorded audio state exceeds the old limit");
  component.value=.75;
  check(commercial_state(component,controller,true,&recorded)==recorded&&component.value==.375,
        "recorded audio and settings survive actual SDK save/restore");
  // The state's length is not a multiple of the reader's block, so its end
  // arrives as a shorter read.
  component.recording=2*1024*1024+1234;component.whole_stream=true;
  auto uneven=commercial_state(component,controller,true,nullptr);
  component.value=.75;
  check(commercial_state(component,controller,true,&uneven)==uneven&&component.value==.375,
        "a plug-in reading fixed blocks to the end restores the final partial block");
  component.whole_stream=false;
  {
    LVBState::Stream ending({1,2,3});uint8_t out[8]{};int32 got=-1;
    check(ending.read(out,8,&got)==kResultOk&&got==3&&out[2]==3&&
          ending.read(out,8,&got)==kResultOk&&got==0&&!ending.failed,
          "a short read and a read at the end succeed with exact counts");
  }
  component.recording=0;
  for(bool controllerFailure:{false,true}){
    component.oversized=!controllerFailure;controller.oversized=controllerFailure;
    bool refused=false;
    try{commercial_state(component,controller,true,nullptr);}
    catch(const linux_vst_bridge::wf0::SaveRefusal& e){
      refused=e.stage==3&&e.result==kResultFalse&&std::string(e.what()).find("256 MiB")!=std::string::npos;
    }
    check(refused,"oversized component/controller state is a specific save refusal");
    component.oversized=controller.oversized=false;
    check(commercial_state(component,controller,true,nullptr)==original,"same SDK instance saves after capacity refusal");
  }
  component.recording=LVBState::payloadLimit-8;
  bool combinedRefused=false;
  try{commercial_state(component,controller,true,nullptr);}
  catch(const linux_vst_bridge::wf0::SaveRefusal& e){combinedRefused=e.stage==3;}
  check(combinedRefused,"combined state header/parameters count toward the save limit");
  component.recording=0;
  LVBState::Stream sparse;
  check(sparse.seek(int64(LVBState::payloadLimit)+1,IBStream::kIBSeekSet,nullptr)==kResultFalse&&
        sparse.failed&&sparse.capacity_exceeded&&sparse.bytes.empty(),"oversized seek is detected before allocation");
  auto before=controller.synchronizations;
  LVBState::Stream tooLarge({},4);component.getState(&tooLarge);
  check(!linux_vst_bridge::wf0::synchronize_initial(controller,true,kResultFalse,tooLarge)&&
        controller.synchronizations==before,"capacity refusal during initialization keeps the editor available");
  for(auto r:{kResultFalse,kNotImplemented}){
    LVBState::Stream partial;partial.bytes={1,2,3};
    check(!linux_vst_bridge::wf0::synchronize_initial(controller,true,r,partial)&&controller.synchronizations==before,"fresh initialization refuses no editor and never synchronizes a declined stream");
  }
  LVBState::Stream fresh;component.getState(&fresh);
  check(linux_vst_bridge::wf0::synchronize_initial(controller,true,kResultOk,fresh)&&controller.synchronizations==before+1,"available initial state synchronizes once");
  controller.terminate();
  component.terminate();
  std::cout << "AP11 pure state capture and exactly-once restore "
               "synchronization PASS\n";
}
