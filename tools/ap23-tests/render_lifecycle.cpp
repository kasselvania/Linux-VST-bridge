// Actual MappedSession and production owner/render loop over Winsock, the
// paired notification transport and a version-3 mailbox, against a scripted
// native peer. Render owns Stop/Start while Activated: it acknowledges Stopped,
// takes the next Start and hands state requests and Deactivate to the owner.
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "pluginterfaces/base/ibstream.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "mapped_processing.h"
#include "component_instance_session.h"
#include "../../native-vst3-proxy/include/ap11_gui.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include <array>
#include <atomic>
#include <chrono>
#include <cstring>
#include <filesystem>
#include <iostream>
#include <string>
#include <thread>
using namespace linux_vst_bridge;
using namespace ap1;
using namespace Steinberg;
using namespace Steinberg::Vst;
using Clock=std::chrono::steady_clock;
namespace {
void check(bool ok,const char* message){if(!ok){std::cerr<<message<<'\n';std::exit(1);}}
template<class F> void until(F f,const char* message){auto end=Clock::now()+std::chrono::seconds(5);while(!f()){check(Clock::now()<end,message);Sleep(1);}}
struct View {
 HANDLE file=INVALID_HANDLE_VALUE,map=nullptr;uint8_t* p=nullptr;
 View(const std::filesystem::path& path,size_t n){
  file=CreateFileW(path.c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,0,nullptr);check(file!=INVALID_HANDLE_VALUE,"fixture file");
  map=CreateFileMappingW(file,nullptr,PAGE_READWRITE,0,DWORD(n),nullptr);check(map!=nullptr,"fixture mapping");p=static_cast<uint8_t*>(MapViewOfFile(map,FILE_MAP_ALL_ACCESS,0,0,n));check(p!=nullptr,"fixture view");std::memset(p,0,n);
 }
 ~View(){UnmapViewOfFile(p);CloseHandle(map);CloseHandle(file);}
 LONG flag(size_t at){return InterlockedCompareExchange(reinterpret_cast<volatile LONG*>(p+at),0,0);}
 void flag(size_t at,LONG v){InterlockedExchange(reinterpret_cast<volatile LONG*>(p+at),v);}
};
struct Peer {
 SOCKET fd=INVALID_SOCKET;
 void transfer(uint8_t* p,size_t n,bool out){while(n){auto k=out?send(fd,reinterpret_cast<char*>(p),int(n),0):recv(fd,reinterpret_cast<char*>(p),int(n),0);check(k>0,"test socket transfer");p+=k;n-=size_t(k);}}
 void write(Frame f){auto b=encode(f,15);transfer(b.data(),b.size(),true);}
 Frame read(){std::vector<uint8_t>b(header_bytes);transfer(b.data(),b.size(),false);auto n=payload_length(b.data(),15);b.resize(header_bytes+n);if(n)transfer(b.data()+header_bytes,n,false);return decode(b,15);}
 void hint(){uint8_t value=1;transfer(&value,1,true);}
 void close(){if(fd!=INVALID_SOCKET){closesocket(fd);fd=INVALID_SOCKET;}}
 ~Peer(){close();}
};
struct Plugin final:AudioEffect {
 std::atomic<unsigned> starts{0},stops{0},blocks{0};std::atomic<DWORD> start_thread{0};
 unsigned refuse_start=0;uint32_t restored=0;
 tresult PLUGIN_API initialize(FUnknown* h) override {auto r=AudioEffect::initialize(h);addAudioInput(u"In",SpeakerArr::kStereo);addAudioOutput(u"Out",SpeakerArr::kStereo);return r;}
 tresult PLUGIN_API getState(IBStream* stream) override {uint32_t state=0x52454e44;int32 written=0;return stream->write(&state,4,&written)==kResultOk&&written==4?kResultOk:kResultFalse;}
 tresult PLUGIN_API setState(IBStream* stream) override {int32 read=0;return stream->read(&restored,4,&read)==kResultOk&&read==4?kResultOk:kResultFalse;}
 tresult PLUGIN_API setProcessing(TBool active) override {
  if(active){start_thread.store(GetCurrentThreadId());if(++starts==refuse_start)return kResultFalse;}else ++stops;
  return kResultOk;
 }
 tresult PLUGIN_API process(ProcessData& d) override {
  for(int ch=0;ch<2&&d.numSamples;++ch)for(int i=0;i<d.numSamples;++i)d.outputs[0].channelBuffers32[ch][i]=d.inputs[0].channelBuffers32[ch][i]*.5f;
  ++blocks;return kResultOk;
 }
};
struct Controller final:EditController {
 tresult PLUGIN_API setComponentState(IBStream*) override {return kResultOk;}
 tresult PLUGIN_API setState(IBStream*) override {return kResultOk;}
 tresult PLUGIN_API getState(IBStream* stream) override {uint32_t value=0x434f4e54;int32 n=0;return stream->write(&value,4,&n);}
};
enum class Case {normal,peer_death,owner_throw,second_start_refused};
const char* name(Case c){switch(c){case Case::normal:return "normal";case Case::peer_death:return "peer death while render waits";case Case::owner_throw:return "owner throws mid-handoff";default:return "second start refused";}}
void run(Case scenario){
 const auto layout=block_layout;
 auto dir=std::filesystem::temp_directory_path()/("ap23-render-"+std::to_string(GetCurrentProcessId())+"-"+std::to_string(int(scenario)));std::filesystem::create_directory(dir);
 {
  constexpr auto gui_bytes=320+2*512*sizeof(ap11_gui_message_t);
  View audio(dir/L"ap1.audio",layout.bytes),mailbox(dir/L"ap10.delivery",33024),gui(dir/L"ap11.ui",gui_bytes);
  std::array<uint8_t,16> id{};id[0]=23;
  for(auto [at,v]:{std::pair{0,0x4d315041u},{4,layout.version},{8,layout.capacity},{12,layout.channels},{16,layout.bytes},{20,layout.input},{24,layout.output},{28,layout.stride}})put(audio.p+at,v,4);
  for(auto [base,channels]:{std::pair{layout.input,2u},{layout.output,layout.channels}})for(uint32_t ch=0;ch<channels;++ch){put(audio.p+base+ch*layout.stride,guard,4);put(audio.p+base+(ch+1)*layout.stride-4,guard,4);}
  std::memcpy(mailbox.p,"LVBM",4);put(mailbox.p+4,3,4);put(mailbox.p+8,33024,4);std::memcpy(mailbox.p+16,id.data(),16);
  std::memcpy(gui.p,"LVBU",4);put(gui.p+4,AP11::GuiProtocolVersion,4);put(gui.p+8,gui_bytes,4);put(gui.p+12,sizeof(ap11_gui_message_t),4);put(gui.p+32,512,4);put(gui.p+96,1,8);std::memcpy(gui.p+16,id.data(),16);
  SOCKET listener=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);sockaddr_in address{};address.sin_family=AF_INET;address.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
  check(bind(listener,reinterpret_cast<sockaddr*>(&address),sizeof(address))==0&&listen(listener,1)==0,"fixture listener");int len=sizeof(address);check(getsockname(listener,reinterpret_cast<sockaddr*>(&address),&len)==0,"fixture port");
  {View config(dir/L"ap1.control",60);put(config.p,ntohs(address.sin_port),2);std::memcpy(config.p+4,id.data(),16);put(config.p+2,1,2);put(config.p+52,15,2);put(config.p+56,60,4);}
  SOCKET wake_listener=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);address.sin_port=0;
  check(bind(wake_listener,reinterpret_cast<sockaddr*>(&address),sizeof(address))==0&&listen(wake_listener,1)==0,"notification listener");
  check(getsockname(wake_listener,reinterpret_cast<sockaddr*>(&address),&len)==0,"notification port");const uint16_t wake_port=ntohs(address.sin_port);
  Plugin plugin;Controller controller;HostApplication host;
  check(plugin.initialize(&host)==kResultOk&&controller.initialize(&host)==kResultOk,"SDK initialization");
  plugin.refuse_start=scenario==Case::second_start_refused?2:0;
  std::atomic<bool> peer_done{false};
  std::thread native([&]{
   Peer peer;peer.fd=accept(listener,nullptr,nullptr);closesocket(listener);DWORD timeout=6000;setsockopt(peer.fd,SOL_SOCKET,SO_RCVTIMEO,reinterpret_cast<char*>(&timeout),sizeof(timeout));
   auto hello=peer.read();check(hello.kind==Hello&&hello.session==id,"Hello");put(audio.p+56,get(audio.p+40,8)^1,8);MemoryBarrier();
   Peer notification;
   Frame offer{Hello,id,0,std::vector<uint8_t>(44)};std::memcpy(offer.payload.data(),"LVBW",4);put(offer.payload.data()+4,1,4);put(offer.payload.data()+8,wake_port,2);offer.payload[12]=71;peer.write(offer);
   notification.fd=accept(wake_listener,nullptr,nullptr);closesocket(wake_listener);setsockopt(notification.fd,SOL_SOCKET,SO_RCVTIMEO,reinterpret_cast<char*>(&timeout),sizeof(timeout));
   auto joined=notification.read();check(joined.kind==Hello&&joined.session==id,"paired notification");notification.write({Hello,id,0,{}});
   check(peer.read().kind==Ready,"Ready");
   std::vector<uint8_t> setup(28+64);put(setup.data(),256,4);double rate=48000;std::memcpy(setup.data()+8,&rate,8);put(setup.data()+16,3,4);put(setup.data()+20,1,4);put(setup.data()+24,2,4);
   for(int d=0;d<2;++d){auto*p=setup.data()+28+d*32;put(p+4,d,4);put(p+12,2,4);put(p+20,1,4);put(p+24,SpeakerArr::kStereo,8);}
   peer.write({Configure,id,1,setup});check(peer.read().kind==Configured,"configure");
   std::vector<uint8_t> activate(8);put(activate.data(),256,4);peer.write({Activate,id,1,activate});check(peer.read().kind==Activated,"activate");
   uint64_t seq=1,position=0;
   auto epoch_payload=[](uint64_t epoch){std::vector<uint8_t> e(8);put(e.data(),epoch,8);return e;};
   auto audio_blocks=[&](uint64_t epoch,int count){
    for(int b=0;b<count;++b){
     std::vector<uint8_t> p(168);put(p.data(),256,4);put(p.data()+4,layout.input,4);put(p.data()+8,layout.output,4);put(p.data()+12,layout.stride,4);put(p.data()+32,epoch,8);put(p.data()+40,position,8);
     auto bytes=encode({Process,id,seq,p},15);std::memcpy(mailbox.p+256,bytes.data(),bytes.size());put(mailbox.p+68,bytes.size(),4);mailbox.flag(64,1);notification.hint();
     until([&]{return mailbox.flag(128)==1;},"audio reply");auto n=get(mailbox.p+132,4);auto reply=decode({mailbox.p+16640,mailbox.p+16640+n},15);
     check(reply.kind==Done&&reply.sequence==seq,"ordered audio");mailbox.flag(128,0);++seq;position+=256;
    }
   };
   auto start=[&](uint64_t epoch){position=0;peer.write({Start,id,seq,epoch_payload(epoch)});};
   auto stop=[&](uint64_t epoch){
    peer.write({Stop,id,seq,epoch_payload(epoch)});mailbox.flag(64,2);notification.hint();
    auto stopped=peer.read();check(stopped.kind==Stopped&&stopped.sequence==seq&&get(stopped.payload.data(),8)==epoch,"render Stopped correlation");
   };
   start(1);auto started=peer.read();check(started.kind==Started&&started.sequence==seq,"first Started");
   audio_blocks(1,2);stop(1);
   if(scenario==Case::peer_death){peer.close();notification.close();peer_done.store(true);return;}
   if(scenario==Case::owner_throw){
    // A malformed restore makes the owner's handed-over state call throw.
    peer.write({SetState,id,seq,{1,2,3,4}});peer_done.store(true);
    Sleep(3000);return;
   }
   // Stopped: a capture and a restore are handed to the owner, then render
   // takes the next Start without the owner.
   peer.write({GetState,id,seq,{}});auto state=peer.read();check(state.kind==State&&state.sequence==seq,"state while stopped");++seq;
   peer.write({SetState,id,seq,state.payload});auto applied=peer.read();check(applied.kind==StateApplied&&applied.sequence==seq,"restore while stopped");++seq;
   start(2);
   if(scenario==Case::second_start_refused){peer_done.store(true);Sleep(3000);return;}
   started=peer.read();check(started.kind==Started&&started.sequence==seq,"second Started");
   audio_blocks(2,2);stop(2);
   // Stop then Deactivate: render hands Deactivate to the owner.
   peer.write({Deactivate,id,seq,{}});check(peer.read().kind==Deactivated,"deactivate");
   peer.write({Close,id,seq,{}});check(peer.read().kind==Closed,"close");
   peer_done.store(true);
  });
  const auto begin=Clock::now();
  wf0::OfflineResult result{};
  {
   wf0::EventWriter events(1<<20);wf0::HostCallbackSink callbacks(&events,GetCurrentThreadId());
   wf0::MappedSession session(dir.wstring(),"17000000000000000000000000000000",events,true,true,true,true,true);
   session.bind_controller(&controller,true);session.ready();
   result=wf0::run_offline_processing(plugin,plugin,callbacks,events,&session);
   if(scenario==Case::normal){check(result.success&&result.quiescent,"render-owned lifecycle succeeds");session.finish(true);}
   session.bind_controller(nullptr,true);
  }
  const auto seconds=std::chrono::duration<double>(Clock::now()-begin).count();
  native.join();
  check(peer_done.load(),"native peer script completed");
  check(plugin.start_thread.load()!=GetCurrentThreadId(),"setProcessing(true) on the render thread");
  switch(scenario){
  case Case::normal:check(plugin.starts==2&&plugin.stops==2&&plugin.blocks==4&&plugin.restored==0x52454e44,"two intervals, restore while stopped");break;
  case Case::peer_death:check(!result.success&&plugin.starts==1&&plugin.stops==1&&seconds<5,"dead peer ends render's wait promptly");break;
  case Case::owner_throw:check(!result.success&&plugin.starts==1&&plugin.stops==1&&seconds<5,"owner failure ends render's handoff wait promptly");break;
  case Case::second_start_refused:check(!result.success&&plugin.starts==2&&plugin.stops==2&&seconds<6,"refused second start is contained");break;
  }
  std::cout<<"render lifecycle "<<name(scenario)<<": "<<seconds<<" s\n";
  controller.terminate();plugin.terminate();
 }
 std::filesystem::remove_all(dir);
}
}
int main(){
 WSADATA data{};check(WSAStartup(MAKEWORD(2,2),&data)==0,"Winsock");
 for(auto c:{Case::normal,Case::peer_death,Case::owner_throw,Case::second_start_refused})run(c);
 WSACleanup();std::cout<<"render-owned Stop/Start: state handoff, deactivate, peer death, owner failure and refused restart PASS\n";
}
