#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <intrin.h>
#include "mapped_processing.h"
#include "ap8_state.h"
#include "delivery_trace.h"
#include "delivery_mailbox.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include <chrono>
#include <algorithm>
#include "../../vst-state/stream.h"
#include <mutex>
#include <condition_variable>
#include <thread>
#include <exception>
#include <cmath>
namespace linux_vst_bridge::wf0 {
using namespace ap1;
namespace {
void barrier(){_ReadWriteBarrier();MemoryBarrier();_ReadWriteBarrier();}
struct Socket {
 SOCKET value=INVALID_SOCKET; uint16_t minor=1; bool eager=false;
 ~Socket(){if(value!=INVALID_SOCKET)closesocket(value);}
 // Try a nonblocking operation before asking Wine to wait. In the legacy
 // path every already-ready read/write still made a select round trip.
 void transfer(uint8_t* p,size_t n,bool writing,std::chrono::steady_clock::time_point end){
  while(n){
   auto us=std::chrono::duration_cast<std::chrono::microseconds>(end-std::chrono::steady_clock::now()).count();require(us>0,"control deadline");
   if(eager){
    int k=writing?send(value,reinterpret_cast<const char*>(p),int(n),0):recv(value,reinterpret_cast<char*>(p),int(n),0);
    if(k>0){p+=k;n-=size_t(k);continue;}
    require(k==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK,"control disconnected/IO");
   }
   fd_set f;FD_ZERO(&f);FD_SET(value,&f);timeval t{};t.tv_sec=static_cast<decltype(t.tv_sec)>(us/1000000);t.tv_usec=static_cast<decltype(t.tv_usec)>(us%1000000);
   require(select(0,writing?nullptr:&f,writing?&f:nullptr,nullptr,&t)>0,"control timeout/select");
   if(!eager){
    int k=writing?send(value,reinterpret_cast<const char*>(p),int(n),0):recv(value,reinterpret_cast<char*>(p),int(n),0);
    if(k==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK)continue;require(k>0,"control disconnected/IO");p+=k;n-=size_t(k);
   }
  }
 }
 Frame receive(bool command=false){
  std::vector<uint8_t>b(header_bytes);size_t received=0;
  // Idle has no issued-request deadline. The first received byte starts one
  // five-second deadline shared by the rest of the header and payload.
  if(command)for(;;){
   if(eager){
    int n=recv(value,reinterpret_cast<char*>(b.data()),int(header_bytes),0);
    if(n>0){received=size_t(n);break;}
    require(n==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK,"control disconnected/IO");
   }
   fd_set f;FD_ZERO(&f);FD_SET(value,&f);timeval t{1,0};
   auto ready=select(0,&f,nullptr,nullptr,&t);require(ready!=SOCKET_ERROR,"command select");
   if(!ready||eager)continue;
   char first;auto n=recv(value,&first,1,MSG_PEEK);
   if(n==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK)continue;
   require(n>0,"control disconnected/IO");break;
  }
  auto end=std::chrono::steady_clock::now()+std::chrono::seconds(5);
  transfer(b.data()+received,b.size()-received,false,end);auto n=payload_length(b.data(),minor);b.resize(header_bytes+n);
  if(n)transfer(b.data()+header_bytes,n,false,end);return decode(b,minor);
 }
 void write(const Frame& f){auto b=encode(f,minor);transfer(b.data(),b.size(),true,std::chrono::steady_clock::now()+std::chrono::seconds(5));}
};
struct Handle{HANDLE value=INVALID_HANDLE_VALUE;~Handle(){if(value&&value!=INVALID_HANDLE_VALUE)CloseHandle(value);}};
}
struct MappedSession::Impl {
 EventWriter& events;DeliveryTrace diagnostic;Socket socket;std::wstring directory;std::unique_ptr<DeliveryMailbox> mailbox;bool last_fast=false;Handle file,mapping;uint8_t* view=nullptr;Sequence state;Request current{};bool closed=false;bool winsock=false;bool hosted=false;bool stop_requested=false;bool sustained=false;Timeline timeline;Frame pending{};bool has_pending=false;
 bool stateful=false,commercial=false,separate=false,performance=false,configured=false,active=false; uint32_t mode=0, maximum=256; double rate=48000.;
 Steinberg::Vst::IAudioProcessor* processor=nullptr;
 Steinberg::Vst::IEditController* controller=nullptr;
 Steinberg::Vst::IComponent* component=nullptr;std::thread::id owner;
 std::mutex mutex;std::condition_variable condition;bool waiting=false,serviced=false;
 Frame state_frame{};std::exception_ptr state_error;
 explicit Impl(EventWriter&e):events(e){}
 void state_call(Frame f){
  require(component&&std::this_thread::get_id()==owner,"state owner thread");
  require(!state.failed&&!state.outstanding&&f.session==state.session&&f.sequence==state.next,"state request correlation");
  require(f.kind==GetState||f.kind==SetState,"state request kind");
  require((f.kind==GetState&&f.payload.empty())||(f.kind==SetState&&!timeline.running),"state restore while processing refused");
  if(commercial){
   require(controller,"commercial controller absent");
   events.lifecycle("ap4_state_started",",\"operation\":\"opaque\",\"owner_thread\":true");
   auto payload=commercial_state(*component,*controller,separate,f.kind==SetState?&f.payload:nullptr);
   events.lifecycle("ap4_state_result",",\"operation\":\"opaque\",\"result\":0,\"bytes\":"+std::to_string(payload.size()));
   socket.write(frame(uint16_t(f.kind+1),state.next,std::move(payload)));require(state.next<UINT64_MAX,"state sequence overflow");++state.next;return;
  }
  auto validate=[](const std::vector<uint8_t>& p){
   require(p.size()==12,"unsupported reference state extent");float gain,reduction;
   std::memcpy(&gain,p.data(),4);std::memcpy(&reduction,p.data()+4,4);
   require(std::isfinite(gain)&&gain>=0&&gain<=1&&std::isfinite(reduction)&&reduction>=0&&reduction<=1&&get(p.data()+8,4)<=1,"unsupported reference state values");
  };
  if(f.kind==SetState){
   validate(f.payload);LVBState::Stream input(f.payload);
   events.lifecycle("ap4_state_started",",\"operation\":\"set\",\"owner_thread\":true");
   auto r=component->setState(&input);
   events.lifecycle("ap4_state_result",",\"operation\":\"set\",\"result\":"+std::to_string(r));
   require(r==Steinberg::kResultOk&&!input.failed&&input.quiescent(),"Windows component setState failed");
  }
  LVBState::Stream output;
  events.lifecycle("ap4_state_started",",\"operation\":\"get\",\"owner_thread\":true");
  auto result=component->getState(&output);
  events.lifecycle("ap4_state_result",",\"operation\":\"get\",\"result\":"+std::to_string(result));
  require(result==Steinberg::kResultOk&&!output.failed&&output.quiescent(),"Windows component getState failed");
  validate(output.bytes);
  std::string hex;const char* digits="0123456789abcdef";for(auto b:output.bytes){hex+=digits[b>>4];hex+=digits[b&15];}
  events.lifecycle("ap4_state_readback",",\"payload_bytes\":12,\"payload_hex\":\""+hex+"\",\"restored\":"+(f.kind==SetState?"true":"false")+",\"next_sequence\":"+std::to_string(state.next));
  if(f.kind==SetState)require(output.bytes==f.payload,"Windows restored state readback differs");
  socket.write(frame(uint16_t(f.kind+1),state.next,output.bytes));require(state.next<UINT64_MAX,"state sequence overflow");++state.next;
 }
 void dispatch(Frame f){
  if(std::this_thread::get_id()==owner){state_call(std::move(f));return;}
  std::unique_lock lock(mutex);require(!waiting,"state already pending");
  state_frame=std::move(f);waiting=true;serviced=false;state_error=nullptr;condition.notify_all();
  require(condition.wait_for(lock,std::chrono::seconds(10),[&]{return serviced;}),"owner state service timeout");
  waiting=false;if(state_error)std::rethrow_exception(state_error);
 }
 void configure(const Frame& f){
  using namespace Steinberg;using namespace Steinberg::Vst;
  require(performance&&processor&&std::this_thread::get_id()==owner&&!active&&!timeline.running&&!state.outstanding,"configuration requires inactive owner");
  require(f.session==state.session&&f.sequence==state.next&&f.payload.size()==24,"configuration correlation/extent");
  auto p=f.payload.data();auto m=uint32_t(get(p,4)),md=uint32_t(get(p+4,4));double hz;std::memcpy(&hz,p+8,8);
  require(m>=1&&m<=capacity&&(md==0||md==2)&&(hz==44100.||hz==48000.||hz==88200.||hz==96000.||hz==192000.)&&get(p+16,4)<=1&&get(p+20,4)==0,"unsupported processing configuration");
  const auto mailbox_version=get(p+16,4);
  if(mailbox_version&&!mailbox)mailbox=std::make_unique<DeliveryMailbox>(directory,state.session);
  require(mailbox_version==uint64_t(bool(mailbox)),"delivery configuration differs");
  const bool support32=processor->canProcessSampleSize(kSample32)==kResultTrue,support64=processor->canProcessSampleSize(kSample64)==kResultTrue;
  require(support32,"Windows plugin does not support float32");
  SpeakerArrangement input=SpeakerArr::kStereo,output=SpeakerArr::kStereo;auto inputs=component->getBusCount(kAudio,kInput);
  require(processor->setBusArrangements(inputs?&input:nullptr,inputs,&output,1)==kResultOk,"configuration bus arrangement");
  ProcessSetup setup{int32(md),kSample32,int32(m),hz};
  require(processor->setupProcessing(setup)==kResultOk,"Windows processing setup rejected");
  // SDK latency is queried on the owner only after successful setup.
  auto latency=processor->getLatencySamples(),tail=processor->getTailSamples();
  std::vector<uint8_t> reply(16);put(reply.data(),latency,4);put(reply.data()+4,tail,4);put(reply.data()+8,(support32?1:0)|(support64?2:0),4);put(reply.data()+12,mailbox_version,4);
  maximum=m;mode=md;rate=hz;configured=true;
  events.lifecycle("ap10_delivery_setup",",\"mailbox_version\":"+std::to_string(mailbox_version));
  events.lifecycle("ap9_processing_setup",",\"sample_rate\":"+std::to_string(hz)+",\"maximum\":"+std::to_string(m)+",\"precision\":\"float32\",\"vendor_float64\":"+(support64?"true":"false")+",\"latency_samples\":"+std::to_string(latency)+",\"tail_samples\":"+std::to_string(tail));
  socket.write(frame(Configured,state.next,std::move(reply)));
 }
 Frame receive(bool processing=false){for(;;){Frame f{};last_fast=false;
  if(processing&&mailbox){last_fast=mailbox->receive(f,socket.minor);if(!last_fast)f=socket.receive(true);}else f=socket.receive(true);if(f.kind==Configure){configure(f);continue;}if(stateful&&(f.kind==GetState||f.kind==SetState)){dispatch(std::move(f));continue;}return f;}}

 ~Impl(){try{diagnostic.dump(events);}catch(...){}if(view)UnmapViewOfFile(view);if(socket.value!=INVALID_SOCKET){closesocket(socket.value);socket.value=INVALID_SOCKET;}if(winsock)WSACleanup();}
 void error(const std::exception& e){
  state.failed=true;
  events.lifecycle("ap1_transport_error",",\"detail\":\""+std::string(e.what()).substr(0,160)+"\"");
  if(socket.value!=INVALID_SOCKET)try{auto f=frame(Error,state.next,{1,0,0,0});if(last_fast&&mailbox)mailbox->send(f,socket.minor);else socket.write(f);}catch(...){}
 }
 Frame frame(uint16_t kind,uint64_t sequence,std::vector<uint8_t> p={}){return {kind,state.session,sequence,std::move(p)};}
};
MappedSession::MappedSession(const std::wstring& directory,const std::string& session,EventWriter& events,bool hosted,bool sustained,bool stateful,bool commercial,bool performance):impl_(std::make_unique<Impl>(events)){
 auto& x=*impl_;x.directory=directory;x.hosted=hosted||sustained;x.sustained=sustained;x.stateful=stateful;x.commercial=commercial;x.performance=performance;x.socket.eager=performance;x.socket.minor=performance?(commercial?7:6):commercial?5:stateful?4:sustained?3:(hosted?2:1);
 try {
  require(session.size()==32,"session syntax");for(size_t i=0;i<16;++i)x.state.session[i]=uint8_t(std::stoul(session.substr(i*2,2),nullptr,16));
  Handle config;config.value=CreateFileW((directory+L"\\ap1.control").c_str(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,0,nullptr);require(config.value!=INVALID_HANDLE_VALUE,"control configuration open");
  LARGE_INTEGER size{};require(GetFileSizeEx(config.value,&size)&&size.QuadPart==52,"control configuration length");std::array<uint8_t,52>b{};DWORD read=0;require(ReadFile(config.value,b.data(),DWORD(b.size()),&read,nullptr)&&read==b.size(),"control configuration read");
  require(get(b.data()+2,2)==0&&std::equal(x.state.session.begin(),x.state.session.end(),b.begin()+4),"control session binding");auto port=get(b.data(),2);require(port>0,"control port");
  x.file.value=CreateFileW((directory+L"\\ap1.audio").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,OPEN_EXISTING,0,nullptr);require(x.file.value!=INVALID_HANDLE_VALUE,"backing file open");require(GetFileSizeEx(x.file.value,&size)&&size.QuadPart==mapping_bytes,"backing file size");
  x.mapping.value=CreateFileMappingW(x.file.value,nullptr,PAGE_READWRITE,0,0,nullptr);require(x.mapping.value!=nullptr,"CreateFileMapping");x.view=static_cast<uint8_t*>(MapViewOfFile(x.mapping.value,FILE_MAP_READ|FILE_MAP_WRITE,0,0,mapping_bytes));require(x.view!=nullptr,"MapViewOfFile");barrier();
  require(get(x.view,4)==0x4d315041&&get(x.view+4,4)==1&&get(x.view+8,4)==capacity&&get(x.view+12,4)==2&&get(x.view+16,4)==mapping_bytes&&get(x.view+20,4)==input_offset&&get(x.view+24,4)==output_offset&&get(x.view+28,4)==stride,"mapping layout");
  auto witness=get(x.view+32,8)^witness_mask;put(x.view+40,witness,8);barrier();
  WSADATA data{};require(WSAStartup(MAKEWORD(2,2),&data)==0,"WSAStartup");x.winsock=true;x.socket.value=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);require(x.socket.value!=INVALID_SOCKET,"socket create");
  if(performance){int enabled=1;require(setsockopt(x.socket.value,IPPROTO_TCP,TCP_NODELAY,reinterpret_cast<const char*>(&enabled),sizeof(enabled))==0,"TCP_NODELAY");}
  u_long nonblock=1;require(ioctlsocket(x.socket.value,FIONBIO,&nonblock)==0,"socket nonblocking");sockaddr_in address{};address.sin_family=AF_INET;address.sin_port=htons(u_short(port));address.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
  int connected=connect(x.socket.value,reinterpret_cast<const sockaddr*>(&address),sizeof(address));if(connected==SOCKET_ERROR){require(WSAGetLastError()==WSAEWOULDBLOCK,"loopback connect");fd_set f;FD_ZERO(&f);FD_SET(x.socket.value,&f);timeval t{5,0};require(select(0,nullptr,&f,nullptr,&t)>0,"loopback connect timeout");int e=0,n=sizeof(e);require(getsockopt(x.socket.value,SOL_SOCKET,SO_ERROR,reinterpret_cast<char*>(&e),&n)==0&&e==0,"loopback connection failed");}
  std::vector<uint8_t> hello(40);std::copy(b.begin()+20,b.end(),hello.begin());put(hello.data()+32,capacity,4);put(hello.data()+36,mapping_bytes,4);x.socket.write(x.frame(Hello,0,hello));auto reply=x.socket.receive();require(reply.kind==Hello&&reply.session==x.state.session&&reply.sequence==0&&reply.payload.empty(),"Hello acknowledgement");barrier();require(get(x.view+56,8)==(witness^1),"Linux mapping witness");
  events.lifecycle("ap1_mapping_ready",",\"mapping_count\":1,\"connection_count\":1,\"mapping_witness\":true");
 }catch(const std::exception&e){x.error(e);throw;}
}
MappedSession::~MappedSession()=default;
bool MappedSession::performance() const{return impl_->performance;}
void MappedSession::bind_processor(Steinberg::Vst::IAudioProcessor* p){impl_->processor=p;}
double MappedSession::sample_rate() const{return impl_->rate;}
bool MappedSession::commercial() const{return impl_->commercial;}
void MappedSession::bind_controller(Steinberg::Vst::IEditController* c,bool separate){impl_->controller=c;impl_->separate=separate;}
bool MappedSession::hosted() const{return impl_->hosted;}
bool MappedSession::sustained() const{return impl_->sustained;}
bool MappedSession::stateful() const{return impl_->stateful;}
void MappedSession::bind_component(Steinberg::Vst::IComponent* component){impl_->component=component;impl_->owner=std::this_thread::get_id();}
void MappedSession::service_owner(){auto& x=*impl_;std::unique_lock lock(x.mutex);
 if(x.waiting&&!x.serviced){try{x.state_call(std::move(x.state_frame));}catch(...){x.state_error=std::current_exception();}x.serviced=true;x.condition.notify_all();}}
bool MappedSession::initial_transition(){auto&x=*impl_;if(!x.has_pending){x.pending=x.receive();x.has_pending=true;}
 require(x.pending.kind==Activate||x.pending.kind==Close,"initial activation or close");if(x.pending.kind==Close){lifecycle_request(Close);return false;}return true;}
uint32_t MappedSession::process_mode() const{return impl_->mode;}
bool MappedSession::activation_again(){return initial_transition();}

uint16_t MappedSession::next_transition(){auto&x=*impl_;try{
 require(x.sustained&&!x.has_pending&&!x.timeline.running&&!x.state.failed&&!x.state.outstanding,"next lifecycle ownership");
 x.pending=x.receive();x.has_pending=true;
 require(x.pending.kind==Start||x.pending.kind==Deactivate,"restart or deactivate required");return x.pending.kind;
 }catch(const std::exception&e){x.error(e);throw;}}

uint32_t MappedSession::lifecycle_request(uint16_t kind){auto&x=*impl_;try{
 require(x.hosted&&!x.state.failed&&!x.state.outstanding,"lifecycle ownership");auto f=x.has_pending?std::move(x.pending):x.receive();x.has_pending=false;
 require(f.kind==kind&&f.session==x.state.session&&f.sequence==x.state.next,"lifecycle correlation");
 if(kind==Activate){require(f.payload.size()==(x.sustained?8:4),"activation extent");if(x.sustained){auto mode=uint32_t(get(f.payload.data()+4,4));if(x.performance)require(x.configured&&mode==x.mode,"activation mode differs from setup");else {require(mode<=(x.stateful?1u:0u),"processing mode required");x.mode=mode;}}auto n=get(f.payload.data(),4);require(n>=1&&n<=capacity,"activation maximum");if(x.performance)require(x.configured&&n==x.maximum&&x.mode==get(f.payload.data()+4,4),"activation differs from setup");x.active=true;return uint32_t(n);}
 if(x.sustained&&kind==Start){x.timeline.start(f);return 0;}
 require(f.payload.empty(),"lifecycle payload");if(kind==Close){x.state.close(f);x.closed=true;}return 0;
 }catch(const std::exception&e){x.error(e);throw;}}
void MappedSession::lifecycle_ack(uint16_t kind){auto&x=*impl_;require(!x.state.failed,"failed lifecycle");
 if(kind==Deactivated)x.active=false;
 std::vector<uint8_t> payload;if(x.sustained&&(kind==Started||kind==Stopped)){payload.resize(8);put(payload.data(),x.timeline.epoch,8);}
 x.socket.write(x.frame(kind,x.state.next,payload));x.events.lifecycle("ap2_lifecycle_ack",",\"kind\":"+std::to_string(kind)+",\"next_sequence\":"+std::to_string(x.state.next));}

void MappedSession::ready(){auto&x=*impl_;x.socket.write(x.frame(Ready,0));}
bool MappedSession::next(ExternalBlock& out,float* left,float* right){auto&x=*impl_;try{x.diagnostic.current={};x.diagnostic.stamp(0);auto f=x.receive(true);x.diagnostic.stamp(1);if(x.hosted&&f.kind==Stop){require(!x.state.failed&&!x.state.outstanding&&f.session==x.state.session&&f.sequence==x.state.next,"stop ownership");if(x.sustained)x.timeline.stop(f);else require(f.payload.empty(),"stop payload");x.stop_requested=true;return false;}if(f.kind==Close){require(!x.hosted,"AP2 requires stop before close");x.state.close(f);x.closed=true;return false;}x.current=x.state.begin(x.sustained?x.timeline.request_frame(f,x.commercial):f,x.sustained?UINT64_MAX-1:64,x.stateful);barrier();
 for(size_t ch=0;ch<2;++ch){auto base=x.view+input_offset+ch*stride;require(get(base,4)==guard&&get(base+stride-4,4)==guard,"input guard");std::memcpy(ch?right:left,base+4,x.current.frames*4);}
 out.frames=int(x.current.frames);out.gain=x.current.gain;out.silence=x.current.silence;out.gain_present=x.current.gain_present;
 if(x.commercial){require(!x.current.gain_present,"commercial request carries legacy gain");out.event_count=decode_events(f.payload,out.events,x.current.frames);}
 x.diagnostic.current.epoch=x.timeline.epoch;x.diagnostic.current.sequence=x.state.next;x.diagnostic.current.position=x.timeline.position;
 x.diagnostic.stamp(2);
 // The first active block establishes history, rather than triggering on startup idle.
 if(!x.diagnostic.armed){for(size_t i=0;i<out.event_count;++i)if(out.events[i].kind==0)x.diagnostic.armed=true;
  if(x.diagnostic.armed)x.diagnostic.current.at[0]=x.diagnostic.current.at[1];}
 return true;
 }catch(const std::exception&e){x.error(e);throw;}}
void MappedSession::before_process(){impl_->diagnostic.stamp(3);}
void MappedSession::after_process(){impl_->diagnostic.stamp(4);}
void MappedSession::done(const float* left,const float* right,uint64_t silence,uint64_t process_ns) {
 auto& x=*impl_;
 try {
  x.diagnostic.stamp(5);
  // Retain the actual 64-bit SDK result before validating it, including failures.
  if(!x.sustained)x.events.lifecycle("ap1_output_silence",",\"block\":"+std::to_string(x.state.next-1)+
      ",\"input_silence_flags\":"+std::to_string(x.current.silence)+
      ",\"output_silence_flags\":"+std::to_string(silence));
  auto payload=processing_result(x.current,left,right,silence);
  if(x.sustained)x.timeline.result(payload,x.current.frames);
  if(x.performance){payload.resize(40);put(payload.data()+32,process_ns,8);}
  for(size_t ch=0;ch<2;++ch)
   std::memcpy(x.view+output_offset+ch*stride+4,ch?right:left,x.current.frames*4);
  barrier();
  x.diagnostic.stamp(6);
  if(x.last_fast&&x.mailbox)x.mailbox->send(x.frame(Done,x.state.next,payload),x.socket.minor);else x.socket.write(x.frame(Done,x.state.next,payload));
  x.diagnostic.stamp(7);x.diagnostic.complete();
  x.state.complete();
 } catch(const std::exception& error) {x.error(error);throw;}
}
void MappedSession::finish(bool success){auto&x=*impl_;require(success&&x.closed&&!x.state.outstanding&&!x.state.failed,"session did not close cleanly");require(UnmapViewOfFile(x.view)!=0,"mapping unmap");x.view=nullptr;require(CloseHandle(x.mapping.value)!=0,"mapping handle close");x.mapping.value=nullptr;require(CloseHandle(x.file.value)!=0,"file handle close");x.file.value=INVALID_HANDLE_VALUE;x.socket.write(x.frame(Closed,x.state.next));x.events.lifecycle("ap1_endpoint_closed",",\"mapping_unmapped\":true,\"instance_count\":1");}
}
