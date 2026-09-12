#include "input_observation.h"
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <intrin.h>
#include "mapped_processing.h"
#include "ap8_state.h"
#include "delivery_trace.h"
#include "fault_status.h"
#include "result_status.h"
#include "delivery_mailbox.h"
#include "process_context.h"
#include "controller_updates.h"
#include "editor_session.h"
#include "pluginterfaces/vst/vstspeaker.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include <chrono>
#include <algorithm>
#include "../../vst-state/stream.h"
#include <mutex>
#include <atomic>
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
 std::thread::id ui_owner;void(*service_ui)(void*)=nullptr;void* ui_context=nullptr;
 bool owner_wait(){return service_ui&&ui_owner==std::this_thread::get_id();}
 void pump(){if(owner_wait())service_ui(ui_context);}
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
   if(owner_wait()){pump();us=std::min<int64_t>(us,4000);}
   fd_set f;FD_ZERO(&f);FD_SET(value,&f);timeval t{};t.tv_sec=static_cast<decltype(t.tv_sec)>(us/1000000);t.tv_usec=static_cast<decltype(t.tv_usec)>(us%1000000);
   auto ready=select(0,writing?nullptr:&f,writing?&f:nullptr,nullptr,&t);require(ready!=SOCKET_ERROR,"control timeout/select");
   if(!ready){require(owner_wait(),"control timeout/select");continue;}
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
   pump();
   if(eager){
    int n=recv(value,reinterpret_cast<char*>(b.data()),int(header_bytes),0);
    if(n>0){received=size_t(n);break;}
    require(n==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK,"control disconnected/IO");
   }
   fd_set f;FD_ZERO(&f);FD_SET(value,&f);timeval t{owner_wait()?0:1,owner_wait()?4000:0};
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
 EventWriter& events;DeliveryTrace diagnostic;InputObservation input_observation;std::array<uint64_t,15> completion_trace{};std::unique_ptr<FaultStatus> fault;std::unique_ptr<ResultStatus> result_status;Socket socket;std::wstring directory;std::unique_ptr<DeliveryMailbox> mailbox;bool last_fast=false;Handle file,mapping;uint8_t* view=nullptr;Sequence state;Request current{};bool closed=false;bool winsock=false;bool hosted=false;bool stop_requested=false;bool sustained=false;Timeline timeline;Frame pending{};bool has_pending=false;
 BusLayout buses;
 std::unique_ptr<GuiChannel> gui;std::unique_ptr<EditorSession> editor;std::wstring editor_title;
 std::atomic<bool> can_notify{false},audio_active{false};
 std::atomic<uint32_t> requested_restart{0},published_restart{0};
 std::atomic<uint64_t> published_traits{0};
 bool stateful=false,commercial=false,separate=false,performance=false,configured=false,active=false; uint32_t mode=0, maximum=256; double rate=48000.;
 Steinberg::Vst::IAudioProcessor* processor=nullptr;
 Steinberg::Vst::IEditController* controller=nullptr;
 Steinberg::Vst::IComponent* component=nullptr;std::thread::id owner;
 std::mutex mutex;std::condition_variable condition;bool waiting=false,serviced=false;
 Frame state_frame{};std::exception_ptr state_error;bool concurrent_capture=false;
 std::atomic<bool> capture_active{false},capture_failed{false};
 ControllerUpdates controller_updates;
 std::atomic<bool> controller_update_failed{false};
 uint64_t controller_updates_applied=0;
 void update_controller(){
  if(!commercial||controller_update_failed.load())return;
  try {
   const bool ok=controller_updates.drain([&](uint32_t id,double value,uint64_t revision){
    FaultStatus::Scope activity(fault.get(),2,21,id);
    if(editor?!editor->host_value(id,value,revision):controller->setParamNormalized(id,value)!=Steinberg::kResultOk)return false;
    ++controller_updates_applied;return true;
   });
   if(!ok)controller_update_failed.store(true);
  }catch(...){controller_update_failed.store(true);}
 }
 explicit Impl(EventWriter&e):events(e){}
 void state_call(Frame f,bool reserved=false){
  FaultStatus::Scope activity(fault.get(),2,23,f.kind);
  require(component&&std::this_thread::get_id()==owner,"state owner thread");
  // Protocol 12 reserves an active capture's sequence on the delivery thread.
  // The owner must not inspect/mutate that thread's audio sequence or timeline.
  if(!reserved)require(!state.failed&&!state.outstanding&&f.session==state.session&&f.sequence==state.next,"state request correlation");
  auto completed=[&]{if(!reserved){require(state.next<UINT64_MAX,"state sequence overflow");++state.next;}};
  require(f.kind==GetState||f.kind==SetState,"state request kind");
  require((f.kind==GetState&&f.payload.empty())||(!reserved&&f.kind==SetState&&!timeline.running),"state restore while processing refused");
  if(commercial){
   require(controller,"commercial controller absent");
   const auto state_owner_begin=diagnostic.now();
   update_controller();require(!controller_update_failed.load(),"controller automation update failed");
   const auto state_controller_ready=diagnostic.now();
   if(editor)editor->service(true);
   const auto state_editor_ready=diagnostic.now();
   events.lifecycle("ap4_state_started",",\"operation\":\"opaque\",\"owner_thread\":true");
   ReadbackStatus readback;StateTiming state_timing;std::vector<uint8_t> payload;
   try{payload=commercial_state(*component,*controller,separate,f.kind==SetState?&f.payload:nullptr,&readback,diagnostic.enabled?&state_timing:nullptr);}
   catch(const SaveRefusal& e){
    require(socket.minor>=11&&f.kind==GetState,"save refusal requires protocol 11");
    std::vector<uint8_t> error(16);put(error.data(),1,4);put(error.data()+4,GetState,4);put(error.data()+8,e.stage,4);put(error.data()+12,uint32_t(e.result),4);
    socket.write(frame(Error,f.sequence,std::move(error)));completed();
    events.lifecycle("ap12_save_refused",",\"operation\":16,\"stage\":"+std::to_string(e.stage)+",\"sdk_result\":"+std::to_string(e.result));return;
   }
   if(diagnostic.enabled)events.lifecycle("ap13_state_timing",",\"request_sequence\":"+std::to_string(f.sequence)+",\"owner_begin_qpc\":"+std::to_string(state_owner_begin)+",\"controller_ready_qpc\":"+std::to_string(state_controller_ready)+",\"editor_ready_qpc\":"+std::to_string(state_editor_ready)+",\"frequency\":"+std::to_string(diagnostic.frequency)+",\"component_ns\":"+std::to_string(state_timing.component_ns)+",\"controller_ns\":"+std::to_string(state_timing.controller_ns)+",\"metadata_ns\":"+std::to_string(state_timing.metadata_ns)+",\"values_ns\":"+std::to_string(state_timing.values_ns)+",\"total_ns\":"+std::to_string(state_timing.total_ns));
   events.lifecycle("ap12_readback",",\"unavailable_count\":"+std::to_string(readback.unavailable)+",\"first_unavailable_id\":"+std::to_string(readback.first_id)+",\"first_unavailable_bits\":"+std::to_string(readback.first_bits));
   events.lifecycle("ap10_controller_sync",",\"applied\":"+std::to_string(controller_updates_applied)+",\"state_request_sequence\":"+std::to_string(f.sequence));
   events.lifecycle("ap4_state_result",",\"operation\":\"opaque\",\"result\":0,\"bytes\":"+std::to_string(payload.size()));
   socket.write(frame(uint16_t(f.kind+1),f.sequence,std::move(payload)));completed();return;
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
  events.lifecycle("ap4_state_readback",",\"payload_bytes\":12,\"payload_hex\":\""+hex+"\",\"restored\":"+(f.kind==SetState?"true":"false")+",\"next_sequence\":"+std::to_string(f.sequence));
  if(f.kind==SetState)require(output.bytes==f.payload,"Windows restored state readback differs");
  socket.write(frame(uint16_t(f.kind+1),f.sequence,output.bytes));completed();
 }
 void dispatch(Frame f){
  if(std::this_thread::get_id()==owner){state_call(std::move(f));return;}
  FaultStatus::Scope activity(fault.get(),1,7,f.kind);
  std::unique_lock lock(mutex);
  // A completed response may reach Linux just before the owner finishes its
  // bookkeeping. Reclaim that slot before admitting the next control request.
  if(waiting){require(condition.wait_for(lock,std::chrono::seconds(10),[&]{return serviced;}),"previous owner state service timeout");waiting=false;if(state_error)std::rethrow_exception(state_error);}
  const bool concurrent=socket.minor>=12&&mailbox&&timeline.running&&f.kind==GetState;
  if(concurrent){
   require(!state.failed&&!state.outstanding&&f.session==state.session&&f.sequence==state.next&&f.payload.empty()&&state.next<UINT64_MAX,"active capture correlation/ownership");
   ++state.next;
  }
  state_frame=std::move(f);waiting=true;serviced=false;concurrent_capture=concurrent;state_error=nullptr;capture_active.store(concurrent);condition.notify_all();
  if(concurrent)return;
  require(condition.wait_for(lock,std::chrono::seconds(10),[&]{return serviced;}),"owner state service timeout");
  waiting=false;if(state_error)std::rethrow_exception(state_error);
 }
 void configure(const Frame& f){
  using namespace Steinberg;using namespace Steinberg::Vst;
  FaultStatus::Scope activity(fault.get(),2,24,Configure);
  require(performance&&processor&&std::this_thread::get_id()==owner&&!active&&!timeline.running&&!state.outstanding,"configuration requires inactive owner");
  require(f.session==state.session&&f.sequence==state.next&&(f.payload.size()==24||(socket.minor>=8&&f.payload.size()>=28)),"configuration correlation/extent");
  auto p=f.payload.data();auto m=uint32_t(get(p,4)),md=uint32_t(get(p+4,4));double hz;std::memcpy(&hz,p+8,8);
  require(m>=1&&m<=capacity&&(md==0||md==2)&&(hz==44100.||hz==48000.||hz==88200.||hz==96000.||hz==192000.)&&(get(p+16,4)==0||get(p+16,4)==2||get(p+16,4)==3)&&(socket.minor>=8?(get(p+20,4)==1||get(p+20,4)==3):get(p+20,4)==0),"unsupported processing configuration");
  const auto mailbox_version=get(p+16,4);
  if(mailbox_version&&!mailbox){mailbox=std::make_unique<DeliveryMailbox>(directory,state.session);
   events.lifecycle("ap10_wait_resolution",",\"samples_per_method\":32,\"sleep50_mean_ns\":"+std::to_string(mailbox->sleep50_ns)+",\"ntdelay50_mean_ns\":"+std::to_string(mailbox->delay50_ns));}
  require(mailbox_version==(mailbox?mailbox->version:0),"delivery configuration differs");
  const bool support32=processor->canProcessSampleSize(kSample32)==kResultTrue,support64=processor->canProcessSampleSize(kSample64)==kResultTrue;
  require(support32,"Windows plugin does not support float32");
  can_notify.store(socket.minor>=8&&(get(p+20,4)&2));
  buses.read(*component,*processor);if(socket.minor>=8)buses.contract(f.payload);buses.negotiate(*processor);
  ProcessSetup setup{int32(md),kSample32,int32(m),hz};
  require(processor->setupProcessing(setup)==kResultOk,"Windows processing setup rejected");
  // SDK latency is queried on the owner only after successful setup.
  auto latency=processor->getLatencySamples(),tail=processor->getTailSamples();
  published_traits.store(uint64_t(latency)|(uint64_t(tail)<<32));
  std::vector<uint8_t> reply(16);put(reply.data(),latency,4);put(reply.data()+4,tail,4);put(reply.data()+8,(support32?1:0)|(support64?2:0),4);put(reply.data()+12,mailbox_version,4);
  maximum=m;mode=md;rate=hz;configured=true;
  events.lifecycle("ap10_delivery_setup",",\"mailbox_version\":"+std::to_string(mailbox_version));
  events.lifecycle("ap9_processing_setup",",\"sample_rate\":"+std::to_string(hz)+",\"maximum\":"+std::to_string(m)+",\"precision\":\"float32\",\"vendor_float64\":"+(support64?"true":"false")+",\"latency_samples\":"+std::to_string(latency)+",\"tail_samples\":"+std::to_string(tail));
  socket.write(frame(Configured,state.next,std::move(reply)));
 }
 Frame receive(bool processing=false){for(;;){Frame f{};last_fast=false;
  if(capture_failed.load(std::memory_order_acquire)){std::lock_guard lock(mutex);std::rethrow_exception(state_error);}
  if(processing&&mailbox){last_fast=mailbox->receive(f,socket.minor);if(!last_fast)f=socket.receive(true);}else f=socket.receive(true);if(f.kind==Configure){configure(f);continue;}if(stateful&&(f.kind==GetState||f.kind==SetState)){dispatch(std::move(f));continue;}return f;}}

 ~Impl(){try{if(diagnostic.enabled)input_observation.dump(events);diagnostic.dump(events);}catch(...){}if(view)UnmapViewOfFile(view);if(socket.value!=INVALID_SOCKET){closesocket(socket.value);socket.value=INVALID_SOCKET;}if(winsock)WSACleanup();}
 void error(const std::exception& e){
  state.failed=true;
  if(capture_active.load()&&socket.value!=INVALID_SOCKET){shutdown(socket.value,SD_BOTH);return;}
  events.lifecycle("ap1_transport_error",",\"detail\":\""+std::string(e.what()).substr(0,160)+"\"");
  if(socket.value!=INVALID_SOCKET)try{auto f=frame(Error,state.next,{1,0,0,0});if(last_fast&&mailbox)mailbox->send(f,socket.minor);else socket.write(f);}catch(...){}
 }
 Frame frame(uint16_t kind,uint64_t sequence,std::vector<uint8_t> p={}){return {kind,state.session,sequence,std::move(p)};}
};
MappedSession::MappedSession(const std::wstring& directory,const std::string& session,EventWriter& events,bool hosted,bool sustained,bool stateful,bool commercial,bool performance):impl_(std::make_unique<Impl>(events)){
 auto& x=*impl_;x.directory=directory;x.hosted=hosted||sustained;x.sustained=sustained;x.stateful=stateful;x.commercial=commercial;x.performance=performance;x.socket.eager=performance;x.socket.minor=performance?(commercial?12:6):commercial?5:stateful?4:sustained?3:(hosted?2:1);
 try {
  require(session.size()==32,"session syntax");for(size_t i=0;i<16;++i)x.state.session[i]=uint8_t(std::stoul(session.substr(i*2,2),nullptr,16));
  x.result_status=std::make_unique<ResultStatus>(directory,x.state.session);
  x.fault=std::make_unique<FaultStatus>(directory,x.state.session);x.fault->stage(2,20);
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
  if(x.socket.minor>=10)x.gui=std::make_unique<GuiChannel>(directory,x.state.session);
  events.lifecycle("ap1_mapping_ready",",\"mapping_count\":1,\"connection_count\":1,\"mapping_witness\":true");
 }catch(const std::exception&e){x.error(e);throw;}
}
MappedSession::~MappedSession()=default;
void MappedSession::editor_name(const char* name){
 auto&x=*impl_;int n=MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,name,-1,nullptr,0);
 require(n>1&&n<=256,"editor display name");x.editor_title.resize(size_t(n));
 require(MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,name,-1,x.editor_title.data(),n)==n,"editor display name encoding");x.editor_title.pop_back();
}
Steinberg::tresult MappedSession::editor_edit(uint32_t kind,uint32_t id,double value){
 return impl_->editor?impl_->editor->edit(kind,id,value):Steinberg::kNotImplemented;
}
const BusLayout* MappedSession::bus_layout() const{return impl_->configured?&impl_->buses:nullptr;}
bool MappedSession::returned_results() const{return impl_->socket.minor>=9;}
bool MappedSession::performance() const{return impl_->performance;}
void MappedSession::bind_processor(Steinberg::Vst::IAudioProcessor* p){impl_->processor=p;}
double MappedSession::sample_rate() const{return impl_->rate;}
bool MappedSession::commercial() const{return impl_->commercial;}
void MappedSession::bind_controller(Steinberg::Vst::IEditController* c,bool separate,Steinberg::Vst::IComponentHandler* handler){
 auto&x=*impl_;
 FaultStatus::Scope activity(x.fault.get(),2,24,c?1:2);
 if(!c&&x.editor){
  require(x.editor->retire(),"vendor view refused removal");
  const auto&v=x.editor->view();
  x.events.lifecycle("ap11_editor_summary",",\"opens\":"+std::to_string(v.opens)+",\"closes\":"+std::to_string(v.closes)+",\"focuses\":"+std::to_string(v.focuses)+",\"removal_messages\":"+std::to_string(v.removal_messages)+",\"gestures\":"+std::to_string(x.editor->gestures)+",\"values\":"+std::to_string(x.editor->values)+",\"ends\":"+std::to_string(x.editor->ends)+",\"host_updates\":"+std::to_string(x.editor->host_updates)+",\"stale_updates\":"+std::to_string(x.editor->stale_updates)+",\"suppressed_echoes\":"+std::to_string(x.editor->suppressed_echoes)+",\"scale_supported\":"+(v.scale_supported?"true":"false")+",\"scale\":"+std::to_string(v.scale)+",\"failure\":"+std::to_string(x.gui->failure()));
  x.editor.reset();
 }
 x.controller=c;x.separate=separate;
 if(c){auto n=c->getParameterCount();require(n>=0&&n<=8192,"controller update parameter bound");std::vector<uint32_t> ids;
  for(int i=0;i<n;++i){Steinberg::Vst::ParameterInfo info{};require(c->getParameterInfo(i,info)==Steinberg::kResultOk,"controller update metadata");ids.push_back(info.id);}
  require(x.controller_updates.configure(ids),"duplicate controller parameter identity");
  if(x.gui){x.editor=std::make_unique<EditorSession>(*x.gui,*c,handler);x.editor->fault_status(x.fault.get());x.editor->name(x.editor_title);}}
}
bool MappedSession::hosted() const{return impl_->hosted;}
bool MappedSession::sustained() const{return impl_->sustained;}
bool MappedSession::stateful() const{return impl_->stateful;}
void MappedSession::bind_component(Steinberg::Vst::IComponent* component){impl_->component=component;impl_->owner=std::this_thread::get_id();
 auto&x=*impl_;x.socket.ui_owner=x.owner;x.socket.ui_context=&x;
 x.socket.service_ui=[](void*p){auto&v=*static_cast<Impl*>(p);v.update_controller();if(v.editor)v.editor->service();};
}
Steinberg::tresult MappedSession::request_restart(int32_t flags){auto&x=*impl_;
 // Active topology changes are unsupported. Inactive requests are revalidated
 // against the exact descriptor when the DAW reconfigures. Latency/tail queries
 // and SDK notifications stay on the owner/UI threads.
 if(flags<=0||(flags&~30)||(flags&2&&x.audio_active.load()))return Steinberg::kNotImplemented;
 if((flags&10)&&!x.can_notify.load())return Steinberg::kNotImplemented;
 if((flags&20)&&(!x.editor||x.editor->restart(flags&20)!=Steinberg::kResultOk))return Steinberg::kNotImplemented;
 x.requested_restart.fetch_or(uint32_t(flags&10));return Steinberg::kResultOk;
}
void MappedSession::service_owner(){auto& x=*impl_;
 x.update_controller();if(x.editor)x.editor->service();
 if(auto flags=x.requested_restart.exchange(0)){FaultStatus::Scope activity(x.fault.get(),2,24,flags);auto latency=x.processor->getLatencySamples(),tail=x.processor->getTailSamples();x.published_traits.store(uint64_t(latency)|(uint64_t(tail)<<32));x.published_restart.fetch_or(flags);}
 std::unique_lock lock(x.mutex);
 if(x.waiting&&!x.serviced){
  auto frame=std::move(x.state_frame);const bool concurrent=x.concurrent_capture;
  lock.unlock();std::exception_ptr error;
  try{x.state_call(std::move(frame),concurrent);}catch(...){error=std::current_exception();}
  lock.lock();x.state_error=error;x.serviced=true;
  if(error&&concurrent){x.capture_failed.store(true,std::memory_order_release);shutdown(x.socket.value,SD_BOTH);}
  x.capture_active.store(false);x.condition.notify_all();}}

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
 if(kind==Activate){require(f.payload.size()==(x.sustained?8:4),"activation extent");if(x.sustained){auto mode=uint32_t(get(f.payload.data()+4,4));if(x.performance)require(x.configured&&mode==x.mode,"activation mode differs from setup");else {require(mode<=(x.stateful?1u:0u),"processing mode required");x.mode=mode;}}auto n=get(f.payload.data(),4);require(n>=1&&n<=capacity,"activation maximum");if(x.performance)require(x.configured&&n==x.maximum&&x.mode==get(f.payload.data()+4,4),"activation differs from setup");x.active=true;x.audio_active.store(true);return uint32_t(n);}
 if(x.sustained&&kind==Start){x.timeline.start(f);return 0;}
 require(f.payload.empty(),"lifecycle payload");if(kind==Close){x.state.close(f);x.closed=true;}return 0;
 }catch(const std::exception&e){x.error(e);throw;}}
void MappedSession::lifecycle_ack(uint16_t kind){auto&x=*impl_;require(!x.state.failed,"failed lifecycle");
 if(kind==Deactivated){x.active=false;x.audio_active.store(false);}
 std::vector<uint8_t> payload;if(x.sustained&&(kind==Started||kind==Stopped)){payload.resize(8);put(payload.data(),x.timeline.epoch,8);}
 x.socket.write(x.frame(kind,x.state.next,payload));x.events.lifecycle("ap2_lifecycle_ack",",\"kind\":"+std::to_string(kind)+",\"next_sequence\":"+std::to_string(x.state.next));}

void MappedSession::lifecycle_activity(bool owner,uint64_t stage){if(impl_->fault)impl_->fault->stage(owner?2:1,stage?24:0,stage);}
void MappedSession::ready(){auto&x=*impl_;x.socket.write(x.frame(Ready,0));}
bool MappedSession::next(ExternalBlock& out,float* left,float* right){auto&x=*impl_;try{require(!x.controller_update_failed.load(),"controller automation update failed");x.diagnostic.current={};x.diagnostic.stamp(0);if(x.fault)x.fault->stage(1,1);auto f=x.receive(true);x.diagnostic.stamp(1);if(x.fault)x.fault->publish(1,{0,x.timeline.epoch,f.sequence,x.timeline.position,2,f.kind});if(x.hosted&&f.kind==Stop){require(!x.state.failed&&!x.state.outstanding&&f.session==x.state.session&&f.sequence==x.state.next,"stop ownership");if(x.sustained)x.timeline.stop(f);else require(f.payload.empty(),"stop payload");x.stop_requested=true;return false;}if(f.kind==Close){require(!x.hosted,"AP2 requires stop before close");x.state.close(f);x.closed=true;return false;}x.current=x.state.begin(x.sustained?x.timeline.request_frame(f,x.commercial):f,x.sustained?UINT64_MAX-1:64,x.stateful);barrier();
 for(size_t ch=0;ch<2;++ch){auto base=x.view+input_offset+ch*stride;require(get(base,4)==guard&&get(base+stride-4,4)==guard,"input guard");std::memcpy(ch?right:left,base+4,x.current.frames*4);}
 if(x.diagnostic.enabled)x.input_observation.observe(x.timeline.epoch,x.state.next,x.timeline.position,x.current.frames,x.current.silence,left,right);
 out.generation=x.fault?x.fault->rows[1].generation:0;out.epoch=x.timeline.epoch;out.sequence=x.state.next;out.position=x.timeline.position;
 out.frames=int(x.current.frames);out.gain=x.current.gain;out.silence=x.current.silence;out.gain_present=x.current.gain_present;
 if(x.commercial){require(!x.current.gain_present,"commercial request carries legacy gain");out.event_count=decode_events(f.payload,out.events,x.current.frames,x.socket.minor>=10?104:x.socket.minor>=8?96:0);}
 out.gui_revision=x.socket.minor>=10?get(f.payload.data()+f.payload.size()-8,8):0;
 out.has_context=x.socket.minor>=8&&decode_context(f.payload.data()+f.payload.size()-(x.socket.minor>=10?104:96),out.context,x.rate);
 // Queue controller UI values separately; processor event order and offsets
 // are unchanged. The UI owner drains before each read-only state capture.
 if(x.commercial)for(size_t i=0;i<out.event_count;++i)if(out.events[i].kind==2)
  require(x.controller_updates.publish(out.events[i].id,out.events[i].value,out.gui_revision),"controller update identity/value");
 x.diagnostic.current.epoch=x.timeline.epoch;x.diagnostic.current.sequence=x.state.next;x.diagnostic.current.position=x.timeline.position;
 x.diagnostic.stamp(2);
 // The first active block establishes history, rather than triggering on startup idle.
 if(!x.diagnostic.armed){for(size_t i=0;i<out.event_count;++i)if(out.events[i].kind==0)x.diagnostic.armed=true;
  for(int i=0;i<out.frames&&!x.diagnostic.armed;++i)x.diagnostic.armed=left[i]!=0||right[i]!=0;
  if(x.diagnostic.armed)x.diagnostic.current.at[0]=x.diagnostic.current.at[1];}
 return true;
 }catch(const std::exception&e){x.error(e);throw;}}
static uint64_t thread_cpu_ticks(){FILETIME created{},exited{},kernel{},user{};
 if(!GetThreadTimes(GetCurrentThread(),&created,&exited,&kernel,&user))return UINT64_MAX;
 return (uint64_t(kernel.dwHighDateTime)<<32|kernel.dwLowDateTime)+(uint64_t(user.dwHighDateTime)<<32|user.dwLowDateTime);
}
ResultStatus* MappedSession::result_status(){return impl_->result_status.get();}
void MappedSession::before_process(){auto&x=*impl_;x.diagnostic.stamp(3);if(x.diagnostic.enabled){x.completion_trace[8]=thread_cpu_ticks();auto ui=x.fault?x.fault->owner_activity():std::array<uint64_t,2>{};x.completion_trace[10]=ui[0];x.completion_trace[11]=ui[1];}if(x.fault)x.fault->stage(1,3);}
void MappedSession::after_process(){auto&x=*impl_;x.diagnostic.stamp(4);if(x.diagnostic.enabled){x.completion_trace[9]=thread_cpu_ticks();auto ui=x.fault?x.fault->owner_activity():std::array<uint64_t,2>{};x.completion_trace[12]=ui[0];x.completion_trace[13]=ui[1];}if(x.fault)x.fault->stage(1,4);}
void MappedSession::done(const float* left,const float* right,uint64_t silence,uint64_t process_ns,const ap10_results_t* results) {
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
  if(x.socket.minor>=8){payload.resize(56);put(payload.data()+40,x.published_restart.exchange(0),4);put(payload.data()+44,x.published_traits.load(),8);}
  if(x.socket.minor>=9){
   require(results!=nullptr,"process results absent");const auto&r=*results;
   require(r.events<=ap10_event_capacity&&r.points<=ap10_point_capacity&&r.bytes<=ap10_payload_capacity,"process result capacity");
   auto base=payload.size();payload.resize(base+16+64*r.events+16*r.points+r.bytes);auto*p=payload.data()+base;
   put(p,r.events,4);put(p+4,r.points,4);put(p+8,r.bytes,4);p+=16;
   for(uint32_t i=0;i<r.events;++i,p+=64){const auto&e=r.event[i];put(p,uint32_t(e.offset),4);put(p+4,uint32_t(e.bus),4);put(p+8,std::bit_cast<uint64_t>(e.ppq),8);put(p+16,e.flags,2);put(p+18,e.kind,2);put(p+20,e.payload_offset,4);put(p+24,e.payload_size,4);put(p+28,uint32_t(e.a),4);put(p+32,uint32_t(e.b),4);put(p+36,uint32_t(e.c),4);put(p+40,e.d,4);put(p+48,e.value,8);put(p+56,e.extra,8);}
   for(uint32_t i=0;i<r.points;++i,p+=16){const auto&q=r.point[i];put(p,uint32_t(q.offset),4);put(p+4,q.id,4);put(p+8,std::bit_cast<uint64_t>(q.value),8);}
   std::memcpy(p,r.payload,r.bytes);
  }
  for(size_t ch=0;ch<2;++ch)
   std::memcpy(x.view+output_offset+ch*stride+4,ch?right:left,x.current.frames*4);
  barrier();
  x.diagnostic.stamp(6);if(x.fault)x.fault->stage(1,5);
  if(x.diagnostic.enabled){for(size_t i=0;i<8;++i)x.completion_trace[i]=x.diagnostic.current.at[i];x.completion_trace[14]=x.diagnostic.frequency;}
  if(x.last_fast&&x.mailbox)x.mailbox->send(x.frame(Done,x.state.next,payload),x.socket.minor,x.diagnostic.enabled?&x.completion_trace:nullptr);else x.socket.write(x.frame(Done,x.state.next,payload));
  if(x.fault)x.fault->stage(1,6);x.diagnostic.stamp(7);x.diagnostic.complete();
  x.state.complete();
 } catch(const std::exception& error) {x.error(error);throw;}
}
void MappedSession::finish(bool success){auto&x=*impl_;require(success&&x.closed&&!x.state.outstanding&&!x.state.failed,"session did not close cleanly");require(UnmapViewOfFile(x.view)!=0,"mapping unmap");x.view=nullptr;require(CloseHandle(x.mapping.value)!=0,"mapping handle close");x.mapping.value=nullptr;require(CloseHandle(x.file.value)!=0,"file handle close");x.file.value=INVALID_HANDLE_VALUE;x.socket.write(x.frame(Closed,x.state.next));x.diagnostic.dump(x.events);x.events.lifecycle("ap1_endpoint_closed",",\"mapping_unmapped\":true,\"instance_count\":1");}
}
