#pragma once
#include "ap1_protocol.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/vstspeaker.h"
namespace linux_vst_bridge::wf0 {
// SDK census kept intact. The bounded implementation carries main stereo audio,
// one optional note input, and represents auxiliary buses without activating them.
struct BusLayout {
 struct Bus {Steinberg::Vst::BusInfo info{};Steinberg::Vst::SpeakerArrangement arrangement=0;bool active=false;};
 std::array<Bus,32> buses{};size_t size=0;std::array<int,4> counts{};
 void read(Steinberg::Vst::IComponent& c,Steinberg::Vst::IAudioProcessor& p){
  using namespace Steinberg;using namespace Steinberg::Vst;using ap1::require;size=0;
  for(int media=0;media<2;++media)for(int dir=0;dir<2;++dir){auto n=c.getBusCount(media,dir);require(n>=0&&n<=8,"bus count bound");counts[media*2+dir]=n;
   for(int i=0;i<n;++i){auto& b=buses[size++];require(c.getBusInfo(media,dir,i,b.info)==kResultOk,"bus metadata query");
    require(b.info.mediaType==media&&b.info.direction==dir&&(b.info.busType==kMain||b.info.busType==kAux),"bus metadata tuple");
    if(media==kAudio){require(p.getBusArrangement(dir,i,b.arrangement)==kResultOk&&b.arrangement==SpeakerArr::kStereo&&b.info.channelCount==2,"only declared stereo audio supported");require((i==0)==(b.info.busType==kMain),"main audio bus index");}
    else require(b.info.channelCount>=0&&b.info.channelCount<=16,"event channel bound");
    b.active=(media==kAudio?b.info.busType==kMain:dir==kInput);
   }
  }
  require(counts[1]==1&&counts[2]<=1,"one output and optional event input required");
 }
 void contract(const std::vector<uint8_t>& p){
  using namespace ap1;require(p.size()>=28&&(get(p.data()+20,4)&1)==1&&get(p.data()+24,4)==size&&p.size()==28+32*size,"bus contract extent/version");
  std::array<int,4> index{};
  for(size_t i=0;i<size;++i){const auto*r=p.data()+28+32*i;auto&b=buses[i];auto m=b.info.mediaType,d=b.info.direction;
   require(get(r,4)==m&&get(r+4,4)==d&&get(r+8,4)==index[m*2+d]++&&get(r+12,4)==b.info.channelCount&&get(r+16,4)==b.info.busType&&get(r+24,8)==b.arrangement,"native/Windows SDK buses differ");
   auto enabled=get(r+20,4);require(enabled<=1&&(!enabled||b.active),"unsupported auxiliary activation");b.active=enabled!=0;
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
