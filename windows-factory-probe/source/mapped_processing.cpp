#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <intrin.h>
#include "mapped_processing.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include <chrono>
#include <algorithm>
namespace linux_vst_bridge::wf0 {
using namespace ap1;
namespace {
void barrier(){_ReadWriteBarrier();MemoryBarrier();_ReadWriteBarrier();}
struct Socket {
 SOCKET value=INVALID_SOCKET; uint16_t minor=1;
 ~Socket(){if(value!=INVALID_SOCKET)closesocket(value);}
 void transfer(uint8_t* p,size_t n,bool writing,std::chrono::steady_clock::time_point end){
  while(n){auto us=std::chrono::duration_cast<std::chrono::microseconds>(end-std::chrono::steady_clock::now()).count();require(us>0,"control deadline");
   fd_set f;FD_ZERO(&f);FD_SET(value,&f);timeval t{long(us/1000000),long(us%1000000)};
   require(select(0,writing?nullptr:&f,writing?&f:nullptr,nullptr,&t)>0,"control timeout/select");
   int k=writing?send(value,reinterpret_cast<const char*>(p),int(n),0):recv(value,reinterpret_cast<char*>(p),int(n),0);
   if(k==SOCKET_ERROR&&WSAGetLastError()==WSAEWOULDBLOCK)continue;require(k>0,"control disconnected/IO");p+=k;n-=size_t(k);
  }
 }
 Frame receive(){auto end=std::chrono::steady_clock::now()+std::chrono::seconds(5);std::vector<uint8_t>b(header_bytes);transfer(b.data(),b.size(),false,end);auto n=payload_length(b.data(),minor);b.resize(header_bytes+n);if(n)transfer(b.data()+header_bytes,n,false,end);return decode(b,minor);}
 void write(const Frame& f){auto b=encode(f,minor);transfer(b.data(),b.size(),true,std::chrono::steady_clock::now()+std::chrono::seconds(5));}
};
struct Handle{HANDLE value=INVALID_HANDLE_VALUE;~Handle(){if(value&&value!=INVALID_HANDLE_VALUE)CloseHandle(value);}};
}
struct MappedSession::Impl {
 EventWriter& events;Socket socket;Handle file,mapping;uint8_t* view=nullptr;Sequence state;Request current{};bool closed=false;bool winsock=false;bool hosted=false;bool stop_requested=false;
 explicit Impl(EventWriter&e):events(e){}
 ~Impl(){if(view)UnmapViewOfFile(view);if(socket.value!=INVALID_SOCKET){closesocket(socket.value);socket.value=INVALID_SOCKET;}if(winsock)WSACleanup();}
 void error(const std::exception& e){
  state.failed=true;
  events.lifecycle("ap1_transport_error",",\"detail\":\""+std::string(e.what()).substr(0,160)+"\"");
  if(socket.value!=INVALID_SOCKET)try{socket.write(frame(Error,state.next,{1,0,0,0}));}catch(...){}
 }
 Frame frame(uint16_t kind,uint64_t sequence,std::vector<uint8_t> p={}){return {kind,state.session,sequence,std::move(p)};}
};
MappedSession::MappedSession(const std::wstring& directory,const std::string& session,EventWriter& events,bool hosted):impl_(std::make_unique<Impl>(events)){
 auto& x=*impl_;x.hosted=hosted;x.socket.minor=hosted?2:1;
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
  u_long nonblock=1;require(ioctlsocket(x.socket.value,FIONBIO,&nonblock)==0,"socket nonblocking");sockaddr_in address{};address.sin_family=AF_INET;address.sin_port=htons(u_short(port));address.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
  int connected=connect(x.socket.value,reinterpret_cast<const sockaddr*>(&address),sizeof(address));if(connected==SOCKET_ERROR){require(WSAGetLastError()==WSAEWOULDBLOCK,"loopback connect");fd_set f;FD_ZERO(&f);FD_SET(x.socket.value,&f);timeval t{5,0};require(select(0,nullptr,&f,nullptr,&t)>0,"loopback connect timeout");int e=0,n=sizeof(e);require(getsockopt(x.socket.value,SOL_SOCKET,SO_ERROR,reinterpret_cast<char*>(&e),&n)==0&&e==0,"loopback connection failed");}
  std::vector<uint8_t> hello(40);std::copy(b.begin()+20,b.end(),hello.begin());put(hello.data()+32,capacity,4);put(hello.data()+36,mapping_bytes,4);x.socket.write(x.frame(Hello,0,hello));auto reply=x.socket.receive();require(reply.kind==Hello&&reply.session==x.state.session&&reply.sequence==0&&reply.payload.empty(),"Hello acknowledgement");barrier();require(get(x.view+56,8)==(witness^1),"Linux mapping witness");
  events.lifecycle("ap1_mapping_ready",",\"mapping_count\":1,\"connection_count\":1,\"mapping_witness\":true");
 }catch(const std::exception&e){x.error(e);throw;}
}
MappedSession::~MappedSession()=default;
bool MappedSession::hosted() const{return impl_->hosted;}
uint32_t MappedSession::lifecycle_request(uint16_t kind){auto&x=*impl_;try{
 require(x.hosted&&!x.state.failed&&!x.state.outstanding,"lifecycle ownership");auto f=x.socket.receive();
 require(f.kind==kind&&f.session==x.state.session&&f.sequence==x.state.next,"lifecycle correlation");
 if(kind==Activate){require(f.payload.size()==4,"activation extent");auto n=get(f.payload.data(),4);require(n>=1&&n<=capacity,"activation maximum");return uint32_t(n);}
 require(f.payload.empty(),"lifecycle payload");if(kind==Close){x.state.close(f);x.closed=true;}return 0;
 }catch(const std::exception&e){x.error(e);throw;}}
void MappedSession::lifecycle_ack(uint16_t kind){auto&x=*impl_;require(!x.state.failed,"failed lifecycle");
 x.socket.write(x.frame(kind,x.state.next));x.events.lifecycle("ap2_lifecycle_ack",",\"kind\":"+std::to_string(kind)+",\"next_sequence\":"+std::to_string(x.state.next));}

void MappedSession::ready(){auto&x=*impl_;x.socket.write(x.frame(Ready,0));}
bool MappedSession::next(ExternalBlock& out,float* left,float* right){auto&x=*impl_;try{auto f=x.socket.receive();if(x.hosted&&f.kind==Stop){require(!x.state.failed&&!x.state.outstanding&&f.session==x.state.session&&f.sequence==x.state.next&&f.payload.empty(),"stop ownership");x.stop_requested=true;return false;}if(f.kind==Close){require(!x.hosted,"AP2 requires stop before close");x.state.close(f);x.closed=true;return false;}x.current=x.state.begin(f);barrier();
 for(size_t ch=0;ch<2;++ch){auto base=x.view+input_offset+ch*stride;require(get(base,4)==guard&&get(base+stride-4,4)==guard,"input guard");std::memcpy(ch?right:left,base+4,x.current.frames*4);}
 out={int(x.current.frames),x.current.gain,x.current.silence};return true;
 }catch(const std::exception&e){x.error(e);throw;}}
void MappedSession::done(const float* left,const float* right,uint64_t silence) {
 auto& x=*impl_;
 try {
  // Retain the actual 64-bit SDK result before validating it, including failures.
  x.events.lifecycle("ap1_output_silence",",\"block\":"+std::to_string(x.state.next-1)+
      ",\"input_silence_flags\":"+std::to_string(x.current.silence)+
      ",\"output_silence_flags\":"+std::to_string(silence));
  auto payload=processing_result(x.current,left,right,silence);
  for(size_t ch=0;ch<2;++ch)
   std::memcpy(x.view+output_offset+ch*stride+4,ch?right:left,x.current.frames*4);
  barrier();
  x.socket.write(x.frame(Done,x.state.next,payload));
  x.state.complete();
 } catch(const std::exception& error) {x.error(error);throw;}
}
void MappedSession::finish(bool success){auto&x=*impl_;require(success&&x.closed&&!x.state.outstanding&&!x.state.failed,"session did not close cleanly");require(UnmapViewOfFile(x.view)!=0,"mapping unmap");x.view=nullptr;require(CloseHandle(x.mapping.value)!=0,"mapping handle close");x.mapping.value=nullptr;require(CloseHandle(x.file.value)!=0,"file handle close");x.file.value=INVALID_HANDLE_VALUE;x.socket.write(x.frame(Closed,x.state.next));x.events.lifecycle("ap1_endpoint_closed",",\"mapping_unmapped\":true,\"instance_count\":1");}
}
