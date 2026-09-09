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
#include <thread>
using namespace linux_vst_bridge;
using namespace ap1;
using namespace Steinberg;
using namespace Steinberg::Vst;
using Clock=std::chrono::steady_clock;
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
 void transfer(uint8_t* p,size_t n,bool out){while(n){auto k=out?send(fd,reinterpret_cast<char*>(p),int(n),0):recv(fd,reinterpret_cast<char*>(p),int(n),0);check(k>0,"test socket transfer");p+=k;n-=size_t(k);}}
 void write(Frame f){auto b=encode(f,12);transfer(b.data(),b.size(),true);}
 Frame read(){std::vector<uint8_t>b(header_bytes);transfer(b.data(),b.size(),false);auto n=payload_length(b.data(),12);b.resize(header_bytes+n);if(n)transfer(b.data()+header_bytes,n,false);return decode(b,12);}
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
int main(){
 WSADATA data{};check(WSAStartup(MAKEWORD(2,2),&data)==0,"Winsock");
 auto dir=std::filesystem::temp_directory_path()/("ap13-state-"+std::to_string(GetCurrentProcessId()));std::filesystem::create_directory(dir);
 {
  View audio(dir/L"ap1.audio",mapping_bytes),mailbox(dir/L"ap10.delivery",33024),gui(dir/L"ap11.ui",256+2*512*584);
  std::array<uint8_t,16> id{};id[0]=13;
  for(auto [at,v]:{std::pair{0,0x4d315041u},{4,1u},{8,capacity},{12,2u},{16,mapping_bytes},{20,input_offset},{24,output_offset},{28,stride}})put(audio.p+at,v,4);
  for(auto base:{input_offset,output_offset})for(int ch=0;ch<2;++ch){put(audio.p+base+ch*stride,guard,4);put(audio.p+base+(ch+1)*stride-4,guard,4);}
  std::memcpy(mailbox.p,"LVBM",4);put(mailbox.p+4,3,4);put(mailbox.p+8,33024,4);std::memcpy(mailbox.p+16,id.data(),16);
  std::memcpy(gui.p,"LVBU",4);put(gui.p+4,3,4);put(gui.p+8,256+2*512*584,4);put(gui.p+12,584,4);put(gui.p+32,512,4);put(gui.p+96,1,8);std::memcpy(gui.p+16,id.data(),16);
  SOCKET listener=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);sockaddr_in address{};address.sin_family=AF_INET;address.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
  check(bind(listener,reinterpret_cast<sockaddr*>(&address),sizeof(address))==0&&listen(listener,1)==0,"fixture listener");int len=sizeof(address);check(getsockname(listener,reinterpret_cast<sockaddr*>(&address),&len)==0,"fixture port");
  {View config(dir/L"ap1.control",52);put(config.p,ntohs(address.sin_port),2);std::memcpy(config.p+4,id.data(),16);}
  FixtureComponent component;Controller controller;HostApplication host;
  check(component.initialize(&host)==kResultOk&&controller.initialize(&host)==kResultOk,"SDK initialization");
  std::atomic<bool> done{false};
  std::thread client([&]{
   Peer peer;peer.fd=accept(listener,nullptr,nullptr);closesocket(listener);DWORD timeout=4000;setsockopt(peer.fd,SOL_SOCKET,SO_RCVTIMEO,reinterpret_cast<char*>(&timeout),sizeof(timeout));
   auto hello=peer.read();check(hello.kind==Hello&&hello.session==id,"Hello");put(audio.p+56,get(audio.p+40,8)^1,8);MemoryBarrier();peer.write({Hello,id,0,{}});check(peer.read().kind==Ready,"Ready");
   std::vector<uint8_t> setup(28+64);put(setup.data(),256,4);double rate=48000;std::memcpy(setup.data()+8,&rate,8);put(setup.data()+16,3,4);put(setup.data()+20,1,4);put(setup.data()+24,2,4);
   for(int d=0;d<2;++d){auto*p=setup.data()+28+d*32;put(p+4,d,4);put(p+12,2,4);put(p+20,1,4);put(p+24,SpeakerArr::kStereo,8);}
   peer.write({Configure,id,1,setup});check(peer.read().kind==Configured,"configure");
   std::vector<uint8_t> activate(8);put(activate.data(),256,4);peer.write({Activate,id,1,activate});check(peer.read().kind==Activated,"activate");
   std::vector<uint8_t> epoch(8);put(epoch.data(),1,8);peer.write({Start,id,1,epoch});check(peer.read().kind==Started,"start");
   uint64_t seq=1,position=0;
   for(unsigned capture=1;capture<=2;++capture){
    component.refused=capture==2; // Before publishing the request to its UI owner.
    const auto capture_seq=seq++;peer.write({GetState,id,capture_seq,{}});mailbox.flag(64,2);
    until([&]{return component.entered.load()==capture&&mailbox.flag(64)==0;});
    // Four parent 512-frame callbacks, each containing adjacent 256 chunks.
    for(int parent=0;parent<4;++parent)for(int chunk=0;chunk<2;++chunk){
     std::vector<uint8_t> p(160);put(p.data(),256,4);put(p.data()+4,input_offset,4);put(p.data()+8,output_offset,4);put(p.data()+12,stride,4);put(p.data()+32,1,8);put(p.data()+40,position,8);
     for(int ch=0;ch<2;++ch)for(int i=0;i<256;++i){float value=float(i+position)/8192;std::memcpy(audio.p+input_offset+ch*stride+4+i*4,&value,4);}
     auto b=encode({Process,id,seq,p},12);std::memcpy(mailbox.p+256,b.data(),b.size());put(mailbox.p+68,b.size(),4);mailbox.flag(64,1);
     until([&]{return mailbox.flag(128)==1;});auto n=get(mailbox.p+132,4);auto reply=decode({mailbox.p+16640,mailbox.p+16640+n},12);check(reply.kind==Done&&reply.sequence==seq&&get(reply.payload.data()+24,8)==position,"ordered audio while save pending");
     for(int ch=0;ch<2;++ch)for(int i=0;i<256;++i){float value;std::memcpy(&value,audio.p+output_offset+ch*stride+4+i*4,4);check(value==float(i+position)/16384,"SDK output preserved");}
     mailbox.flag(128,0);++seq;position+=256;
    }
    auto saved=peer.read();check(saved.sequence==capture_seq&&saved.session==id,"independent save correlation");
    if(capture==1)check(saved.kind==State&&get(saved.payload.data(),4)==4&&get(saved.payload.data()+4,4)==4&&get(saved.payload.data()+16,4)==0x76543210&&get(saved.payload.data()+20,4)==0xabcdef01,"both genuine opaque streams");
    else check(saved.kind==Error&&get(saved.payload.data()+4,4)==GetState&&get(saved.payload.data()+8,4)==1,"ordinary save refusal");
   }
   peer.write({Stop,id,seq,epoch});mailbox.flag(64,2);check(peer.read().kind==Stopped,"stop after save");peer.write({Deactivate,id,seq,{}});check(peer.read().kind==Deactivated,"deactivate");peer.write({Close,id,seq,{}});check(peer.read().kind==Closed,"close");
  });
  {
   wf0::EventWriter events(1<<20);wf0::MappedSession session(dir.wstring(),"0d000000000000000000000000000000",events,true,true,true,true,true);
   session.bind_component(&component);session.bind_processor(&component);session.bind_controller(&controller,true);session.ready();check(session.initial_transition(),"initial transition");session.lifecycle_request(Activate);component.setActive(true);session.lifecycle_ack(Activated);session.lifecycle_request(Start);component.setProcessing(true);session.lifecycle_ack(Started);
   std::thread delivery([&]{wf0::ExternalBlock block{};float left[256]{},right[256]{};ap10_results_t results{};
    while(session.next(block,left,right)){float* channels[]{left,right};AudioBusBuffers bus{};bus.numChannels=2;bus.channelBuffers32=channels;ProcessData p{};p.symbolicSampleSize=kSample32;p.numSamples=block.frames;p.numInputs=1;p.numOutputs=1;p.inputs=&bus;p.outputs=&bus;
     session.before_process();check(component.process(p)==kResultOk,"SDK processing");session.after_process();session.done(left,right,0,0,&results);}
    done.store(true);
   });
   while(!done.load()){session.service_owner();Sleep(1);}delivery.join();component.setProcessing(false);session.lifecycle_ack(Stopped);check(session.next_transition()==Deactivate,"deactivate request");session.lifecycle_request(Deactivate);component.setActive(false);session.lifecycle_ack(Deactivated);check(!session.activation_again(),"close selected");session.bind_controller(nullptr,false);session.finish(true);
  }
  client.join();check(component.captures==2&&controller.captures==1&&component.processed==16,"same-instance captures and audio count");controller.terminate();component.terminate();
 }
 std::filesystem::remove_all(dir);WSACleanup();std::cout<<"AP13 SDK capture overlaps ordered mailbox audio; refusal and lifecycle PASS\n";
}
