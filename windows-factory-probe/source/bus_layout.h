#pragma once
#include "ap1_protocol.h"
#include <cstdlib>
#include <string_view>
#include "../../native-vst3-proxy/include/ap18_bus_support.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/vstspeaker.h"
namespace linux_vst_bridge::wf0 {
// SDK census kept intact. The bounded implementation carries main stereo audio,
// one optional note input, and the sole stereo auxiliary input when no main
// input exists. Additional auxiliary inputs remain inactive.
inline bool event_output_policy() {
 const char* value=std::getenv("LVB_EVENT_OUTPUT_POLICY");
 if(!value)return false;
 ap1::require(std::string_view(value)=="reported_zero_event_channels_unspecified","unknown event output policy");
 return true;
}
struct BusLayout {
 struct Bus {Steinberg::Vst::BusInfo info{};Steinberg::Vst::SpeakerArrangement arrangement=0;bool supported=false,active=false;int effective_channels=0;};
 std::array<Bus,32> buses{};size_t size=0;std::array<int,4> counts{};int transported_input=-1;
 void read(Steinberg::Vst::IComponent& c,Steinberg::Vst::IAudioProcessor& p,bool zero_unspecified=event_output_policy()){
  using namespace Steinberg;using namespace Steinberg::Vst;using ap1::require;size=0;transported_input=-1;
  for(int media=0;media<2;++media)for(int dir=0;dir<2;++dir){auto n=c.getBusCount(media,dir);require(n>=0&&n<=8,"bus count bound");counts[media*2+dir]=n;
   for(int i=0;i<n;++i){auto& b=buses[size++];require(c.getBusInfo(media,dir,i,b.info)==kResultOk,"bus metadata query");
    require(b.info.mediaType==media&&b.info.direction==dir&&(b.info.busType==kMain||b.info.busType==kAux),"bus metadata tuple");
    if(media==kAudio){require(p.getBusArrangement(dir,i,b.arrangement)==kResultOk&&b.arrangement==SpeakerArr::kStereo&&b.info.channelCount==2,"only declared stereo audio supported");require(b.info.busType!=kMain||i==0,"main audio bus index");require(dir!=kOutput||b.info.busType==kMain,"main audio output required");}
    else require(b.info.channelCount>=0&&b.info.channelCount<=16,"event channel bound");
    b.supported=AP18Buses::supported(media,dir,i,b.info.busType,counts[0],b.info.channelCount,b.arrangement);b.active=b.supported&&(b.info.flags&BusInfo::kDefaultActive);
    if(media==kAudio&&dir==kInput&&b.supported)transported_input=i;
   }
  }
  for(size_t i=0;i<size;++i){auto& b=buses[i];b.effective_channels=b.info.channelCount;
   if(zero_unspecified&&b.info.mediaType==kEvent&&b.info.direction==kOutput){
    require(counts[3]==1&&b.info.channelCount==0,"event output policy metadata mismatch");b.effective_channels=16;
   }
  }
  if(zero_unspecified)require(counts[3]==1,"event output policy requires one bus");
  require(counts[1]==1&&counts[2]<=1,"one output and optional event input required");
 }
 void contract(const std::vector<uint8_t>& p){
  using namespace ap1;require(p.size()>=28&&(get(p.data()+20,4)&1)==1&&get(p.data()+24,4)==size&&p.size()==28+32*size,"bus contract extent/version");
  std::array<int,4> index{};
  for(size_t i=0;i<size;++i){const auto*r=p.data()+28+32*i;auto&b=buses[i];auto m=b.info.mediaType,d=b.info.direction;
   require(get(r,4)==uint32_t(m)&&get(r+4,4)==uint32_t(d)&&get(r+8,4)==uint32_t(index[m*2+d]++)&&get(r+12,4)==uint32_t(b.effective_channels)&&get(r+16,4)==uint32_t(b.info.busType)&&get(r+24,8)==b.arrangement,"native/Windows SDK buses differ");
   auto enabled=get(r+20,4);require(enabled<=1&&(!enabled||b.supported),"unsupported auxiliary activation");b.active=enabled!=0;
  }
 }
 // Called with fixed owner-prepared storage. The setup contract, not product
 // role, determines which SDK bus receives the single transported stereo pair.
 void map_inputs(std::array<Steinberg::Vst::AudioBusBuffers,8>& inputs,
                 float** transported,float** zeros,uint64_t silence)const {
  int index=0;
  for(size_t i=0;i<size;++i){const auto& b=buses[i];
   if(b.info.mediaType!=Steinberg::Vst::kAudio||b.info.direction!=Steinberg::Vst::kInput)continue;
   const bool selected=index==transported_input&&b.supported&&b.active;
   auto& input=inputs[size_t(index++)];input.numChannels=b.info.channelCount;
   input.channelBuffers32=selected?transported:zeros;input.silenceFlags=selected?silence:3;
  }
 }
 void negotiate(Steinberg::Vst::IAudioProcessor& p){
  using namespace Steinberg;using namespace Steinberg::Vst;std::array<SpeakerArrangement,8> in{},out{};int ni=0,no=0;
  for(size_t i=0;i<size;++i){auto&b=buses[i];if(b.info.mediaType==kAudio)(b.info.direction==kInput?in[ni++]:out[no++])=b.arrangement;}
  ap1::require(p.setBusArrangements(ni?in.data():nullptr,ni,no?out.data():nullptr,no)==kResultOk,"SDK bus negotiation rejected");
  for(int d=0;d<2;++d)for(int i=0;i<(d?no:ni);++i){SpeakerArrangement a=0;ap1::require(p.getBusArrangement(d,i,a)==kResultOk&&a==(d?out[i]:in[i]),"SDK arrangement changed");}
 }
 void activate(Steinberg::Vst::IComponent& c,bool enabled)const {
  std::array<int,4> index{};for(size_t i=0;i<size;++i){const auto& b=buses[i];auto m=b.info.mediaType,d=b.info.direction;
   ap1::require(c.activateBus(m,d,index[m*2+d]++,enabled&&b.active)==Steinberg::kResultOk,"SDK bus activation rejected");}
 }
};
}
