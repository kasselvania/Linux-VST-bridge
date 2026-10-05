#pragma once
// Narrow Windows transport adapter. The pump owns socket I/O after pairing;
// SDK owners/render exchange prepared frames and events, never socket calls.
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <atomic>
#include <chrono>
#include <thread>
#include <array>
#include <algorithm>
#include "ap1_protocol.h"
namespace linux_vst_bridge::wf0 {
class NotificationTransport {
 using Clock=std::chrono::steady_clock;
 struct Event {HANDLE value=nullptr;~Event(){if(value)CloseHandle(value);}};
 struct NetworkEvent {WSAEVENT value=WSA_INVALID_EVENT;~NetworkEvent(){if(value!=WSA_INVALID_EVENT)WSACloseEvent(value);}};
 struct Lane {
  std::vector<uint8_t> bytes;
  std::atomic<uint32_t> state{0}; // free, queued, sending, completed
  Event completed;
  explicit Lane(size_t capacity){bytes.reserve(capacity);completed.value=CreateEventW(nullptr,FALSE,FALSE,nullptr);ap1::require(completed.value,"transport completion event");}
 };
 SOCKET control_=INVALID_SOCKET,notification_=INVALID_SOCKET;
 uint16_t minor_=15;
 Event cancel_,activity_,request_,control_ready_;
 NetworkEvent control_network_,notification_network_;
 Lane owner_{ap1::header_bytes+(1u<<20)},render_{ap1::header_bytes+10312};
 std::vector<uint8_t> incoming_;
 size_t received_=0;
 bool input_published_=false;
 std::atomic<bool> input_ready_{false},reply_requested_{false},failed_{false},cancelled_{false};
 std::thread pump_;
 std::atomic<void*> death_context_{nullptr};
 void(*death_notify_)(void*)noexcept=nullptr;
 void notify_death()noexcept{if(auto context=death_context_.load(std::memory_order_acquire))death_notify_(context);}
 bool pending_wake_=false,closed_sent_=false;
 Lane* writing_=nullptr;size_t written_=0;
 Clock::time_point input_end_{},output_end_{},wake_end_{};
 static constexpr uint8_t wake=1;
 static HANDLE event(bool manual=false){auto e=CreateEventW(nullptr,manual?TRUE:FALSE,FALSE,nullptr);ap1::require(e,"transport event preparation");return e;}
 void check()const{ap1::require(!failed_.load(std::memory_order_acquire)&&!cancelled_.load(std::memory_order_acquire),"transport pump cancelled/failed");}
 static void startup_transfer(SOCKET socket,uint8_t* bytes,size_t count,bool writing,Clock::time_point end){
  while(count){
   auto left=std::chrono::duration_cast<std::chrono::microseconds>(end-Clock::now()).count();ap1::require(left>0,"notification pairing deadline");
   int n=writing?send(socket,reinterpret_cast<const char*>(bytes),int(count),0):recv(socket,reinterpret_cast<char*>(bytes),int(count),0);
   if(n>0){bytes+=n;count-=size_t(n);continue;}
   ap1::require(n==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK,"notification pairing disconnected/IO");
   fd_set descriptors;FD_ZERO(&descriptors);FD_SET(socket,&descriptors);timeval timeout{};timeout.tv_sec=long(left/1000000);timeout.tv_usec=long(left%1000000);
   ap1::require(select(0,writing?nullptr:&descriptors,writing?&descriptors:nullptr,nullptr,&timeout)>0,"notification pairing select/deadline");
  }
 }
 void connect(const ap1::Frame& offer,const std::array<uint8_t,16>& session){
  using namespace ap1;require(offer.kind==Hello&&offer.session==session&&offer.sequence==0&&offer.payload.size()==44,"notification offer shape");
  const auto*p=offer.payload.data();require(std::memcmp(p,"LVBW",4)==0&&get(p+4,4)==1&&get(p+8,2)>0&&get(p+10,2)==0,"notification offer schema");
  notification_=::socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);require(notification_!=INVALID_SOCKET,"notification socket create");
  u_long nonblocking=1;require(ioctlsocket(notification_,FIONBIO,&nonblocking)==0,"notification socket nonblocking");
  int enabled=1;require(setsockopt(notification_,IPPROTO_TCP,TCP_NODELAY,reinterpret_cast<const char*>(&enabled),sizeof(enabled))==0,"notification TCP_NODELAY");
  sockaddr_in address{};address.sin_family=AF_INET;address.sin_port=htons(u_short(get(p+8,2)));address.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
  auto end=Clock::now()+std::chrono::seconds(10);auto result=::connect(notification_,reinterpret_cast<const sockaddr*>(&address),sizeof(address));
  if(result==SOCKET_ERROR){require(WSAGetLastError()==WSAEWOULDBLOCK,"notification loopback connect");fd_set descriptors;FD_ZERO(&descriptors);FD_SET(notification_,&descriptors);timeval timeout{10,0};
   require(select(0,nullptr,&descriptors,nullptr,&timeout)>0,"notification connect deadline");int error=0,length=sizeof(error);require(getsockopt(notification_,SOL_SOCKET,SO_ERROR,reinterpret_cast<char*>(&error),&length)==0&&error==0,"notification connection failed");}
  Frame hello{Hello,session,0,std::vector<uint8_t>(40)};put(hello.payload.data(),1,4);put(hello.payload.data()+4,1,4);std::memcpy(hello.payload.data()+8,p+12,32);
  auto bytes=encode(hello,minor_);startup_transfer(notification_,bytes.data(),bytes.size(),true,end);
  std::array<uint8_t,header_bytes> acknowledgement{};startup_transfer(notification_,acknowledgement.data(),acknowledgement.size(),false,end);
  require(payload_length(acknowledgement.data(),minor_)==0,"notification acknowledgement extent");Frame accepted{};decode_into(acknowledgement.data(),acknowledgement.size(),minor_,accepted);
  require(accepted.kind==Hello&&accepted.session==session&&accepted.sequence==0,"notification acknowledgement correlation");
 }
 bool read_notification(){
  bool progress=false;std::array<uint8_t,64> bytes{};
  for(int turn=0;turn<4;++turn){auto n=recv(notification_,reinterpret_cast<char*>(bytes.data()),int(bytes.size()),0);
   if(n==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK)break;
   if(n==0&&closed_sent_)return false;
   ap1::require(n>0,"notification endpoint disconnected/IO");
   for(int i=0;i<n;++i)ap1::require(bytes[size_t(i)]==wake,"notification hint value");
   ap1::require(SetEvent(request_.value)!=0,"transport request wake");progress=true;
  }return progress;
 }
 bool write_notification(){
  if(reply_requested_.exchange(false,std::memory_order_acquire)&&!pending_wake_){pending_wake_=true;wake_end_=Clock::now()+std::chrono::seconds(5);}
  if(!pending_wake_)return false;
  auto n=send(notification_,reinterpret_cast<const char*>(&wake),1,0);
  if(n==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK)return false;
  ap1::require(n==1,"notification endpoint disconnected/IO");pending_wake_=false;return true;
 }
 bool read_control(){
  if(input_published_){
   if(input_ready_.load(std::memory_order_acquire))return false;
   // The consumer releases only the ready flag. All input storage and byte
   // counters stay pump-owned, including retirement before the next frame.
   incoming_.resize(ap1::header_bytes);received_=0;input_published_=false;
  }
  auto n=recv(control_,reinterpret_cast<char*>(incoming_.data()+received_),int(std::min<size_t>(incoming_.size()-received_,16384)),0);
  if(n==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK)return false;
  if(n==0&&closed_sent_)return false;
  ap1::require(n>0,"control endpoint disconnected/IO");if(!received_)input_end_=Clock::now()+std::chrono::seconds(5);received_+=size_t(n);
  if(received_==incoming_.size()){
   if(incoming_.size()==ap1::header_bytes){auto length=ap1::payload_length(incoming_.data(),minor_);ap1::require(length<=1u<<20,"control input extent");incoming_.resize(ap1::header_bytes+length);if(length)return true;}
   input_published_=true;input_ready_.store(true,std::memory_order_release);ap1::require(SetEvent(control_ready_.value)&&SetEvent(request_.value),"transport control wake");
  }return true;
 }
 bool write_control(){
  if(!writing_){
   if(render_.state.load(std::memory_order_acquire)==1)writing_=&render_;
   else if(owner_.state.load(std::memory_order_acquire)==1)writing_=&owner_;
   if(!writing_)return false;writing_->state.store(2,std::memory_order_release);written_=0;output_end_=Clock::now()+std::chrono::seconds(5);
  }
  auto& lane=*writing_;auto n=send(control_,reinterpret_cast<const char*>(lane.bytes.data()+written_),int(std::min<size_t>(lane.bytes.size()-written_,16384)),0);
  if(n==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK)return false;
  ap1::require(n>0,"control endpoint disconnected/IO");written_+=size_t(n);
  if(written_==lane.bytes.size()){
   if(ap1::get(lane.bytes.data()+8,2)==ap1::Closed)closed_sent_=true;
   lane.state.store(3,std::memory_order_release);ap1::require(SetEvent(lane.completed.value),"transport write completion");writing_=nullptr;
  }return true;
 }
 void network_events(SOCKET socket,WSAEVENT event){WSANETWORKEVENTS events{};ap1::require(WSAEnumNetworkEvents(socket,event,&events)==0,"transport network event read");
  for(int bit=0;bit<FD_MAX_EVENTS;++bit)if(events.lNetworkEvents&(1L<<bit))ap1::require(events.iErrorCode[bit]==0,"transport network event failure");
  ap1::require(!(events.lNetworkEvents&FD_CLOSE)||closed_sent_,"transport endpoint closed");
 }
 void run()noexcept{
  try{for(;;){
   if(cancelled_.load(std::memory_order_acquire))break;
   const auto now=Clock::now();ap1::require(!received_||input_ready_.load(std::memory_order_acquire)||now<input_end_,"partial control deadline");
   ap1::require(!writing_||now<output_end_,"control output deadline");ap1::require(!pending_wake_||now<wake_end_,"notification backpressure deadline");
   bool progress=read_notification();progress=write_notification()||progress;progress=read_control()||progress;progress=write_control()||progress;
   if(progress)continue;
   std::array<HANDLE,4> handles{cancel_.value,activity_.value,notification_network_.value,control_network_.value};
   const bool bounded=(received_&&!input_ready_.load(std::memory_order_acquire))||writing_||pending_wake_;
   // INFINITE is only pump idle with no issued partial input/output/wake.
   // Prepared producer, native readiness, peer-close and owned cancellation
   // events interrupt it. Admitted exchanges retain the bounds above/below.
   auto ready=WaitForMultipleObjects(DWORD(handles.size()),handles.data(),FALSE,bounded?4:INFINITE);
   if(ready==WAIT_TIMEOUT)continue;
   ap1::require(ready>=WAIT_OBJECT_0&&ready<WAIT_OBJECT_0+handles.size(),"transport pump wait");
   if(ready==WAIT_OBJECT_0)break;
   if(ready==WAIT_OBJECT_0+2)network_events(notification_,notification_network_.value);
   if(ready==WAIT_OBJECT_0+3)network_events(control_,control_network_.value);
  }}catch(...){failed_.store(true,std::memory_order_release);cancelled_.store(true,std::memory_order_release);SetEvent(cancel_.value);}
  notify_death();
  // Only the pump or non-render cancellation owner performs socket shutdown.
  shutdown(notification_,SD_BOTH);shutdown(control_,SD_BOTH);
 }
public:
 NotificationTransport(SOCKET control,const ap1::Frame& offer,const std::array<uint8_t,16>& session):control_(control){
  try{
   cancel_.value=event(true);activity_.value=event();request_.value=event();control_ready_.value=event();
   incoming_.reserve(ap1::header_bytes+(1u<<20));incoming_.resize(ap1::header_bytes);
   connect(offer,session);
   control_network_.value=WSACreateEvent();notification_network_.value=WSACreateEvent();
   ap1::require(control_network_.value!=WSA_INVALID_EVENT&&notification_network_.value!=WSA_INVALID_EVENT,"transport network event preparation");
   ap1::require(WSAEventSelect(control_,control_network_.value,FD_READ|FD_WRITE|FD_CLOSE)==0&&WSAEventSelect(notification_,notification_network_.value,FD_READ|FD_WRITE|FD_CLOSE)==0,"transport network event binding");
   pump_=std::thread([this]{run();});
  }catch(...){if(notification_!=INVALID_SOCKET){closesocket(notification_);notification_=INVALID_SOCKET;}throw;}
 }
 ~NotificationTransport(){stop();if(notification_!=INVALID_SOCKET)closesocket(notification_);}
 NotificationTransport(const NotificationTransport&)=delete;
 void bind_death(void* context,void(*notify)(void*)noexcept){
  ap1::require(context&&notify&&!death_context_.load(),"transport death observer binding");
  death_notify_=notify;death_context_.store(context,std::memory_order_release);
  if(failed_.load(std::memory_order_acquire)||cancelled_.load(std::memory_order_acquire))notify_death();
 }
 bool take_kind(ap1::Frame& frame,uint16_t kind){
  check();if(!input_ready_.load(std::memory_order_acquire)||ap1::get(incoming_.data()+8,2)!=kind)return false;
  ap1::decode_into(incoming_.data(),incoming_.size(),minor_,frame);
  input_ready_.store(false,std::memory_order_release);ap1::require(SetEvent(activity_.value),"transport input retirement wake");return true;
 }
 bool failed()const{return failed_.load(std::memory_order_acquire);}
 void cancel()noexcept{cancelled_.store(true,std::memory_order_release);notify_death();if(cancel_.value)SetEvent(cancel_.value);}
 void stop()noexcept{
  cancel();if(!pump_.joinable())return;
  // After cancellation no pump operation may borrow vendor/GUI storage or
  // block in socket I/O. Retire within the existing 5-s containment allowance.
  // A failed join cannot authorize releasing storage still used by the pump.
  if(WaitForSingleObject(pump_.native_handle(),5000)!=WAIT_OBJECT_0){TerminateProcess(GetCurrentProcess(),94);std::terminate();}
  pump_.join();
 }
 void signal_reply(){check();reply_requested_.store(true,std::memory_order_release);ap1::require(SetEvent(activity_.value),"transport reply wake");}
 void wait_request(){
  // No request has been admitted while the authoritative mailbox is empty.
  // Event publication interrupts this wait immediately; the 4-ms ceiling
  // also requests a fresh mailbox/owner inspection after a missed hint.
  check();std::array<HANDLE,2> handles{cancel_.value,request_.value};auto result=WaitForMultipleObjects(2,handles.data(),FALSE,4);check();ap1::require(result==WAIT_TIMEOUT||result==WAIT_OBJECT_0+1,"transport request event wait");
 }
 void read(ap1::Frame& frame,bool bounded,void(*service)(void*)=nullptr,void* context=nullptr){
  const auto end=Clock::now()+std::chrono::seconds(5);
  while(!input_ready_.load(std::memory_order_acquire)){check();if(service)service(context);
   ap1::require(!bounded||Clock::now()<end,"render control input deadline");
   std::array<HANDLE,2> handles{cancel_.value,control_ready_.value};auto result=WaitForMultipleObjects(2,handles.data(),FALSE,4);
   ap1::require(result==WAIT_TIMEOUT||result==WAIT_OBJECT_0+1,"transport control wait");
  }
  check();auto length=incoming_.size()-ap1::header_bytes;
  ap1::require(!bounded||length<=frame.payload.capacity(),"render control scratch extent");
  ap1::decode_into(incoming_.data(),incoming_.size(),minor_,frame);
  // Pump does not borrow the frame after publication. It resumes only after
  // this consumer has copied the exact bytes and released the prepared slot.
  input_ready_.store(false,std::memory_order_release);ap1::require(SetEvent(activity_.value),"transport input retirement wake");
 }
 void write(const ap1::Frame& frame,bool owner,void(*service)(void*)=nullptr,void* context=nullptr){
  check();auto& lane=owner?owner_:render_;ap1::require(lane.state.load(std::memory_order_acquire)==0,"transport output ownership");
  ap1::require(ap1::header_bytes+frame.payload.size()<=lane.bytes.capacity(),"transport output extent");ap1::encode_into(frame,minor_,lane.bytes);
  lane.state.store(1,std::memory_order_release);ap1::require(SetEvent(activity_.value),"transport output wake");
  const auto end=Clock::now()+std::chrono::seconds(5);
  while(lane.state.load(std::memory_order_acquire)!=3){check();if(service)service(context);
   std::array<HANDLE,2> handles{cancel_.value,lane.completed.value};auto result=WaitForMultipleObjects(2,handles.data(),FALSE,4);
   ap1::require(result==WAIT_TIMEOUT||result==WAIT_OBJECT_0+1,"transport output wait");ap1::require(Clock::now()<end,"transport output deadline");
  }lane.state.store(0,std::memory_order_release);
 }
};
}
