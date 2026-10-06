// Actual MappedSession, Winsock, shared mailbox and SDK component/controller.
// The fixture's getState cannot finish until later audio has been processed.
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "pluginterfaces/base/ibstream.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "mapped_processing.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include <atomic>
#include <chrono>
#include <filesystem>
#include <iostream>
#include <new>
#include <cstdlib>
#include <thread>
using namespace linux_vst_bridge;
using namespace ap1;
using namespace Steinberg;
using namespace Steinberg::Vst;
using Clock=std::chrono::steady_clock;
// IPC15 counts the actual delivery bridge lifecycle, receive (including active
// Capture handoff) and result publication. SDK DSP/state and peer instrumentation
// keep their distinct ownership. Legacy12 retains its original publication audit.
static thread_local bool count_publication=false;
static thread_local size_t publication_allocations=0,publication_deallocations=0;
void* operator new(size_t n){if(count_publication)++publication_allocations;if(auto*p=std::malloc(n))return p;throw std::bad_alloc();}
void operator delete(void*p) noexcept {if(count_publication)++publication_deallocations;std::free(p);}
void operator delete(void*p,size_t) noexcept {if(count_publication)++publication_deallocations;std::free(p);}
void* operator new[](size_t n){if(count_publication)++publication_allocations;if(auto*p=std::malloc(n))return p;throw std::bad_alloc();}
void operator delete[](void*p) noexcept {if(count_publication)++publication_deallocations;std::free(p);}
void operator delete[](void*p,size_t) noexcept {if(count_publication)++publication_deallocations;std::free(p);}
void check(bool ok,const char* message){if(!ok){std::cerr<<message<<'\n';std::exit(1);}}
template<class F> void until(F f){auto end=Clock::now()+std::chrono::seconds(3);while(!f()){check(Clock::now()<end,"test peer progress deadline");Sleep(1);}}
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
 uint16_t minor=12;
 void transfer(uint8_t* p,size_t n,bool out){while(n){auto k=out?send(fd,reinterpret_cast<char*>(p),int(n),0):recv(fd,reinterpret_cast<char*>(p),int(n),0);check(k>0,"test socket transfer");p+=k;n-=size_t(k);}}
 void write(Frame f){auto b=encode(f,minor);transfer(b.data(),b.size(),true);}
 Frame read(){std::vector<uint8_t>b(header_bytes);transfer(b.data(),b.size(),false);auto n=payload_length(b.data(),minor);b.resize(header_bytes+n);if(n)transfer(b.data()+header_bytes,n,false);return decode(b,minor);}
 void hint(){uint8_t value=1;transfer(&value,1,true);}
 ~Peer(){if(fd!=INVALID_SOCKET)closesocket(fd);}
};
struct FixtureComponent final:AudioEffect {
 std::atomic<unsigned> processed{0},entered{0};
 unsigned captures=0;std::atomic<bool> refused{false};
 tresult PLUGIN_API initialize(FUnknown* h) override {auto r=AudioEffect::initialize(h);addAudioInput(u"In",SpeakerArr::kStereo);addAudioOutput(u"Out",SpeakerArr::kStereo);return r;}
 tresult PLUGIN_API getState(IBStream* stream) override {
  auto before=processed.load();++captures;entered.store(captures);
  until([&]{return processed.load()>=before+4;});
  if(refused)return kResultFalse;
  uint32_t state=0x76543210;int32 written=0;
  return stream->write(&state,4,&written)==kResultOk&&written==4?kResultOk:kResultFalse;
 }
 tresult PLUGIN_API process(ProcessData& data) override {
  for(int ch=0;ch<2;++ch)for(int i=0;i<data.numSamples;++i)data.outputs[0].channelBuffers32[ch][i]=data.inputs[0].channelBuffers32[ch][i]*.5f;
  processed.fetch_add(1);return kResultOk;
 }
};
struct Controller final:EditController {
 unsigned captures=0;
 tresult PLUGIN_API initialize(FUnknown* h) override {auto r=EditController::initialize(h);parameters.addParameter(u"Gain",u"",0,.5,ParameterInfo::kCanAutomate,42);return r;}
 tresult PLUGIN_API getState(IBStream* stream) override {++captures;uint32_t value=0xabcdef01;int32 n=0;return stream->write(&value,4,&n);}
};
void run(uint16_t minor){
 const auto layout=minor==15?block_layout:legacy_layout;
 auto dir=std::filesystem::temp_directory_path()/("ap13-state-"+std::to_string(GetCurrentProcessId())+"-"+std::to_string(minor));std::filesystem::create_directory(dir);
 {
  View audio(dir/L"ap1.audio",layout.bytes),mailbox(dir/L"ap10.delivery",33024),gui(dir/L"ap11.ui",320+2*512*608);
  std::array<uint8_t,16> id{};id[0]=13;
  for(auto [at,v]:{std::pair{0,0x4d315041u},{4,layout.version},{8,layout.capacity},{12,layout.channels},{16,layout.bytes},{20,layout.input},{24,layout.output},{28,layout.stride}})put(audio.p+at,v,4);
  for(auto [base,channels]:{std::pair{layout.input,2u},{layout.output,layout.channels}})for(uint32_t ch=0;ch<channels;++ch){put(audio.p+base+ch*layout.stride,guard,4);put(audio.p+base+(ch+1)*layout.stride-4,guard,4);}
  std::memcpy(mailbox.p,"LVBM",4);put(mailbox.p+4,3,4);put(mailbox.p+8,33024,4);std::memcpy(mailbox.p+16,id.data(),16);
  std::memcpy(gui.p,"LVBU",4);put(gui.p+4,5,4);put(gui.p+8,320+2*512*608,4);put(gui.p+12,608,4);put(gui.p+32,512,4);put(gui.p+96,1,8);std::memcpy(gui.p+16,id.data(),16);
  SOCKET listener=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);sockaddr_in address{};address.sin_family=AF_INET;address.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
  check(bind(listener,reinterpret_cast<sockaddr*>(&address),sizeof(address))==0&&listen(listener,1)==0,"fixture listener");int len=sizeof(address);check(getsockname(listener,reinterpret_cast<sockaddr*>(&address),&len)==0,"fixture port");
  {View config(dir/L"ap1.control",minor==15?60:52);put(config.p,ntohs(address.sin_port),2);std::memcpy(config.p+4,id.data(),16);if(minor==15){put(config.p+2,1,2);put(config.p+52,15,2);put(config.p+56,60,4);}}
  SOCKET wake_listener=INVALID_SOCKET;uint16_t wake_port=0;
  if(minor==15){wake_listener=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);address.sin_port=0;check(bind(wake_listener,reinterpret_cast<sockaddr*>(&address),sizeof(address))==0&&listen(wake_listener,1)==0,"notification listener");check(getsockname(wake_listener,reinterpret_cast<sockaddr*>(&address),&len)==0,"notification port");wake_port=ntohs(address.sin_port);}
  FixtureComponent component;Controller controller;HostApplication host;
  check(component.initialize(&host)==kResultOk&&controller.initialize(&host)==kResultOk,"SDK initialization");
  std::atomic<bool> done{false};
  std::thread client([&]{
   Peer peer;peer.minor=minor;peer.fd=accept(listener,nullptr,nullptr);closesocket(listener);DWORD timeout=4000;setsockopt(peer.fd,SOL_SOCKET,SO_RCVTIMEO,reinterpret_cast<char*>(&timeout),sizeof(timeout));
   auto hello=peer.read();check(hello.kind==Hello&&hello.session==id,"Hello");put(audio.p+56,get(audio.p+40,8)^1,8);MemoryBarrier();
   Peer notification;notification.minor=minor;
   if(minor==15){Frame offer{Hello,id,0,std::vector<uint8_t>(44)};std::memcpy(offer.payload.data(),"LVBW",4);put(offer.payload.data()+4,1,4);put(offer.payload.data()+8,wake_port,2);offer.payload[12]=71;peer.write(offer);
    notification.fd=accept(wake_listener,nullptr,nullptr);closesocket(wake_listener);setsockopt(notification.fd,SOL_SOCKET,SO_RCVTIMEO,reinterpret_cast<char*>(&timeout),sizeof(timeout));auto joined=notification.read();check(joined.kind==Hello&&joined.session==id&&joined.sequence==0&&joined.payload.size()==40&&get(joined.payload.data(),4)==1&&get(joined.payload.data()+4,4)==1&&std::memcmp(joined.payload.data()+8,offer.payload.data()+12,32)==0,"paired notification authentication");notification.write({Hello,id,0,{}});
   }else peer.write({Hello,id,0,{}});
   check(peer.read().kind==Ready,"Ready");
   std::vector<uint8_t> setup(28+64);put(setup.data(),256,4);double rate=48000;std::memcpy(setup.data()+8,&rate,8);put(setup.data()+16,3,4);put(setup.data()+20,1,4);put(setup.data()+24,2,4);
   for(int d=0;d<2;++d){auto*p=setup.data()+28+d*32;put(p+4,d,4);put(p+12,2,4);put(p+20,1,4);put(p+24,SpeakerArr::kStereo,8);}
   peer.write({Configure,id,1,setup});check(peer.read().kind==Configured,"configure");
   std::vector<uint8_t> activate(8);put(activate.data(),256,4);peer.write({Activate,id,1,activate});check(peer.read().kind==Activated,"activate");
   std::vector<uint8_t> epoch(8);put(epoch.data(),1,8);peer.write({Start,id,1,epoch});check(peer.read().kind==Started,"start");
   uint64_t seq=1,position=0;
   for(unsigned capture=1;capture<=2;++capture){
    component.refused=capture==2; // Before publishing the request to its UI owner.
    const auto capture_seq=seq++;peer.write({GetState,id,capture_seq,{}});mailbox.flag(64,2);if(minor==15)notification.hint();
    until([&]{return component.entered.load()==capture&&mailbox.flag(64)==0;});
    // Four parent 512-frame callbacks, each containing adjacent 256 chunks.
    for(int parent=0;parent<4;++parent)for(int chunk=0;chunk<2;++chunk){
     std::vector<uint8_t> p(minor==15?168:160);put(p.data(),256,4);put(p.data()+4,layout.input,4);put(p.data()+8,layout.output,4);put(p.data()+12,layout.stride,4);put(p.data()+32,1,8);put(p.data()+40,position,8);
     for(int ch=0;ch<2;++ch)for(int i=0;i<256;++i){float value=float(i+position)/8192;std::memcpy(audio.p+layout.input+ch*layout.stride+4+i*4,&value,4);}
     auto b=encode({Process,id,seq,p},minor);std::memcpy(mailbox.p+256,b.data(),b.size());put(mailbox.p+68,b.size(),4);mailbox.flag(64,1);if(minor==15)notification.hint();
     until([&]{return mailbox.flag(128)==1;});auto n=get(mailbox.p+132,4);auto reply=decode({mailbox.p+16640,mailbox.p+16640+n},minor);check(reply.kind==Done&&reply.sequence==seq&&get(reply.payload.data()+24,8)==position,"ordered audio while save pending");
     for(int ch=0;ch<2;++ch)for(int i=0;i<256;++i){float value;std::memcpy(&value,audio.p+layout.output+ch*layout.stride+4+i*4,4);check(value==float(i+position)/16384,"SDK output preserved");}
     mailbox.flag(128,0);++seq;position+=256;
    }
    auto saved=peer.read();check(saved.sequence==capture_seq&&saved.session==id,"independent save correlation");
    if(capture==1)check(saved.kind==State&&get(saved.payload.data(),4)==4&&get(saved.payload.data()+4,4)==4&&get(saved.payload.data()+16,4)==0x76543210&&get(saved.payload.data()+20,4)==0xabcdef01,"both genuine opaque streams");
    else check(saved.kind==Error&&get(saved.payload.data()+4,4)==GetState&&get(saved.payload.data()+8,4)==1,"ordinary save refusal");
   }
   peer.write({Stop,id,seq,epoch});mailbox.flag(64,2);if(minor==15)notification.hint();check(peer.read().kind==Stopped,"stop after save");peer.write({Deactivate,id,seq,{}});check(peer.read().kind==Deactivated,"deactivate");peer.write({Close,id,seq,{}});check(peer.read().kind==Closed,"close");
  });
  {
   wf0::EventWriter events(1<<20);wf0::MappedSession session(dir.wstring(),"0d000000000000000000000000000000",events,true,true,true,true,true);
   session.bind_component(&component);session.bind_processor(&component);session.bind_controller(&controller,true);session.ready();check(session.initial_transition(),"initial transition");session.lifecycle_request(Activate);component.setActive(true);session.lifecycle_ack(Activated);
   if(minor==15)check(session.next_transition()==Start,"prepared render start");else{session.lifecycle_request(Start);component.setProcessing(true);session.lifecycle_ack(Started);}
   std::thread delivery([&]{wf0::ExternalBlock block{};float left[256]{},right[256]{};ap10_results_t results{};
    if(minor==15){count_publication=true;session.lifecycle_request(Start);count_publication=false;component.setProcessing(true);count_publication=true;session.lifecycle_ack(Started);count_publication=false;}
    for(;;){count_publication=minor==15;const auto has_block=session.next(block,left,right);count_publication=false;check(publication_allocations==0&&publication_deallocations==0,"MappedSession receive/capture/lifecycle allocated or freed on delivery thread");if(!has_block)break;
     float* channels[]{left,right};AudioBusBuffers bus{};bus.numChannels=2;bus.channelBuffers32=channels;ProcessData p{};p.symbolicSampleSize=kSample32;p.numSamples=block.frames;p.numInputs=1;p.numOutputs=1;p.inputs=&bus;p.outputs=&bus;
     session.before_process();check(component.process(p)==kResultOk,"SDK processing");session.after_process();
     count_publication=true;session.done(left,right,0,0,&results);count_publication=false;
     check(publication_allocations==0&&publication_deallocations==0,"MappedSession::done allocated or freed on delivery thread");}
    if(minor==15)component.setProcessing(false);
    done.store(true);
   });
   while(!done.load()){session.service_owner();Sleep(1);}delivery.join();if(minor!=15)component.setProcessing(false);session.lifecycle_ack(Stopped);check(session.next_transition()==Deactivate,"deactivate request");session.lifecycle_request(Deactivate);component.setActive(false);session.lifecycle_ack(Deactivated);check(!session.activation_again(),"close selected");session.bind_controller(nullptr,false);session.finish(true);
  }
  client.join();check(component.captures==2&&controller.captures==1&&component.processed==16,"same-instance captures and audio count");controller.terminate();component.terminate();
 }
 std::filesystem::remove_all(dir);
}
int main(){
 // Legacy AP1/AP2 requests have no actual-mode suffix. Reusing a request must
 // retain the accepted Offline setup; IPC15 requests carry their own RT/PF mode.
 wf0::ExternalBlock mode_request{};
 check(wf0::callback_process_mode(mode_request,kOffline)==kOffline,"legacy offline callback mode");
 check(wf0::callback_process_mode(mode_request,kRealtime)==kRealtime,"legacy sustained callback mode");
 mode_request.authoritative_process_mode=true;mode_request.process_mode=kPrefetch;
 check(wf0::callback_process_mode(mode_request,kRealtime)==kPrefetch,"IPC15 actual prefetch callback mode");
 mode_request.authoritative_process_mode=false;
 check(wf0::callback_process_mode(mode_request,kOffline)==kOffline,"reused legacy request preserves accepted mode");
 WSADATA data{};check(WSAStartup(MAKEWORD(2,2),&data)==0,"Winsock");run(12);run(15);WSACleanup();std::cout<<"AP13 SDK capture overlaps ordered mailbox audio in IPC12/15; IPC15 lifecycle/capture/results retain render storage; refusal and lifecycle PASS\n";
}
