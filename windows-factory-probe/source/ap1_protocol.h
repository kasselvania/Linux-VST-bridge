#pragma once
// Fixed-width wire bytes, never a shared/native struct ABI.
#include <array>
#include <cstdint>
#include <cstring>
#include <stdexcept>
#include <vector>
#include <cmath>
namespace linux_vst_bridge::ap1 {
constexpr uint32_t magic=0x3141504c, capacity=256, header_bytes=56;
constexpr uint32_t stride=1032, input_offset=64, output_offset=2128, mapping_bytes=4192;
constexpr uint32_t multi_channels=64,multi_mapping_bytes=output_offset+multi_channels*stride;
struct Layout {uint32_t capacity,stride,input,output,bytes,version,channels;};
constexpr Layout legacy_layout{capacity,stride,input_offset,output_offset,mapping_bytes,1,2};
constexpr Layout multi_layout{capacity,stride,input_offset,output_offset,multi_mapping_bytes,2,multi_channels};
constexpr uint32_t block_capacity=1024,block_stride=(block_capacity+2)*4;
constexpr uint32_t block_output=input_offset+2*block_stride,block_mapping_bytes=block_output+multi_channels*block_stride;
constexpr Layout block_layout{block_capacity,block_stride,input_offset,block_output,block_mapping_bytes,3,multi_channels};
constexpr uint32_t guard=0x4b123456, poison=0x7fc12345;
constexpr uint64_t witness_mask=0x8d396b274e105ac3ULL;
enum Kind:uint16_t {Hello=1,Ready=2,Process=3,Done=4,Close=5,Closed=6,Error=7,Activate=8,Activated=9,Start=10,Started=11,Stop=12,Stopped=13,Deactivate=14,Deactivated=15,GetState=16,State=17,SetState=18,StateApplied=19,Configure=20,Configured=21};
inline void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
inline uint64_t get(const uint8_t* p,size_t n){uint64_t v=0;for(size_t i=0;i<n;++i)v|=uint64_t(p[i])<<(8*i);return v;}
inline void put(uint8_t* p,uint64_t v,size_t n){for(size_t i=0;i<n;++i)p[i]=uint8_t(v>>(8*i));}
struct Frame {uint16_t kind;std::array<uint8_t,16> session;uint64_t sequence;std::vector<uint8_t> payload;};
inline void encode_into(const Frame&,uint16_t,std::vector<uint8_t>&);
inline void decode_into(const uint8_t*,size_t,uint16_t,Frame&);
inline std::vector<uint8_t> encode(const Frame& f,uint16_t minor=1){
 std::vector<uint8_t>b;encode_into(f,minor,b);return b;
}
inline void encode_into(const Frame& f,uint16_t minor,std::vector<uint8_t>&b){
 require(minor>=1&&minor<=14&&f.kind>=Hello&&f.kind<=(minor>=6?Configured:minor>=4?StateApplied:minor>=2?Deactivated:Error)&&f.payload.size()<=(minor>=4&&f.kind>=State?(1u<<20):minor>=9&&f.kind==Done?10312:(minor==5||minor==7||minor==8||minor==9||minor==10||minor==11||minor==12||minor==13||minor==14)&&f.kind==Process?(minor>=10?8352:minor>=8?8344:8248):4040),"frame kind/length");
 b.assign(header_bytes+f.payload.size(),0);put(b.data(),magic,4);put(b.data()+4,1,2);put(b.data()+6,minor,2);
 put(b.data()+8,f.kind,2);put(b.data()+12,f.payload.size(),4);std::memcpy(b.data()+16,f.session.data(),16);
 put(b.data()+32,1,8);put(b.data()+40,f.sequence,8);if(!f.payload.empty())std::memcpy(b.data()+56,f.payload.data(),f.payload.size());
}
inline size_t payload_length(const uint8_t* b,uint16_t minor=1){
 require(get(b,4)==magic&&get(b+4,2)==1&&get(b+6,2)==minor&&(minor>=1&&minor<=14)&&get(b+10,2)==0,"protocol version/header");
 require(get(b+8,2)>=Hello&&get(b+8,2)<=uint16_t(minor>=6?Configured:minor>=4?StateApplied:minor>=2?Deactivated:Error)&&get(b+32,8)==1&&get(b+48,8)==0,"protocol kind/instance/parent");
 auto n=get(b+12,4);require(n<=(minor>=4&&get(b+8,2)>=State?(1u<<20):minor>=9&&get(b+8,2)==Done?10312:(minor==5||minor==7||minor==8||minor==9||minor==10||minor==11||minor==12||minor==13||minor==14)&&get(b+8,2)==Process?(minor>=10?8352:minor>=8?8344:8248):4040),"frame length");return size_t(n);
}
inline Frame decode(const std::vector<uint8_t>& b,uint16_t minor=1){
 Frame f{};decode_into(b.data(),b.size(),minor,f);return f;
}
inline void decode_into(const uint8_t* b,size_t size,uint16_t minor,Frame&f){
 require(size>=header_bytes,"truncated header");auto n=payload_length(b,minor);require(size==header_bytes+n,"truncated/extra payload");
 f.kind=uint16_t(get(b+8,2));std::memcpy(f.session.data(),b+16,16);f.sequence=get(b+40,8);f.payload.assign(b+56,b+size);
}
struct Request {uint32_t frames;double gain;uint32_t silence;bool gain_present=true;};
inline void processing_result_into(const Request&,const float*,const float*,uint64_t,std::vector<uint8_t>&,Layout=legacy_layout);
inline Request request(const Frame& f,bool stateful=false,Layout layout=legacy_layout){
 require(f.kind==Process&&f.payload.size()==32,"process payload");auto p=f.payload.data();
 auto frames=get(p,4);auto gain_bits=get(p+16,8);double gain;std::memcpy(&gain,&gain_bits,8);
 require((frames>=1||stateful)&&frames<=layout.capacity&&get(p+4,4)==layout.input&&get(p+8,4)==layout.output&&get(p+12,4)==layout.stride,"process extent");
 require(std::isfinite(gain)&&gain>=0&&gain<=1&&get(p+24,4)<=3&&get(p+28,4)<=(stateful?1u:0u),"gain/silence/reserved");
 return {uint32_t(frames),gain,uint32_t(get(p+24,4)),!stateful||get(p+28,4)==1};
}
// Actual native result handler used by MappedSession::done before publication.
// Output flags are independent claims; an unflagged channel may still be zero.
inline std::vector<uint8_t> processing_result(const Request& request,
                                             const float* left, const float* right,
                                             uint64_t silence) {
 std::vector<uint8_t> payload;processing_result_into(request,left,right,silence,payload);return payload;
}
inline void processing_result_into(const Request& request,const float* left,const float* right,uint64_t silence,std::vector<uint8_t>&payload,Layout layout) {
 require(request.frames<=layout.capacity,"result extent");
 require((silence & ~uint64_t(3))==0,"invalid output silence bits");
 for (size_t ch=0;ch<2;++ch) {
  const auto* samples=ch?right:left;
  for (size_t i=0;i<request.frames;++i) {
   require(std::isfinite(samples[i]),"nonfinite output sample");
   require(!(silence & (uint64_t(1)<<ch)) || samples[i]==0.f,
           "output silence claim has nonzero sample");
  }
 }
 payload.resize(16);
 put(payload.data(),request.frames,4);put(payload.data()+4,layout.output,4);
 put(payload.data()+8,silence,8);
}
struct Sequence {
 std::array<uint8_t,16> session{};uint64_t next=1;bool outstanding=false,closed=false,failed=false;
 Request begin(const Frame& f,uint64_t limit=64,bool stateful=false,Layout layout=legacy_layout){
  try {require(!failed&&!closed&&!outstanding&&next<=limit&&next<UINT64_MAX&&f.session==session&&f.sequence==next,"session/sequence/ownership");auto r=request(f,stateful,layout);outstanding=true;return r;}
  catch(...){failed=true;throw;}
 }
 void complete(){require(outstanding&&!failed,"completion ownership");outstanding=false;++next;}
 void close(const Frame& f){try{require(!failed&&!closed&&!outstanding&&f.session==session&&f.sequence==next&&f.kind==Close&&f.payload.empty(),"close ownership");closed=true;}catch(...){failed=true;throw;}}
};
// Minor 3 adds explicit activation epochs and sample positions. The existing
// sequence still owns the one shared mapping until each exact Done arrives.
struct Timeline {
 uint64_t epoch=0,position=0;bool running=false;
 void start(const Frame& f){require(!running&&f.payload.size()==8&&epoch<UINT64_MAX&&get(f.payload.data(),8)==epoch+1,"start epoch");++epoch;position=0;running=true;}
 void stop(const Frame& f){require(running&&f.payload.size()==8&&get(f.payload.data(),8)==epoch,"stop epoch");running=false;}
 Frame request_frame(const Frame& f,bool events=false) const {
  Frame base{};request_frame_into(f,base,events);return base;
 }
 void request_frame_into(const Frame& f,Frame&base,bool events=false) const {
  require(running&&(events?f.payload.size()>=56:f.payload.size()==48)&&get(f.payload.data()+32,8)==epoch&&get(f.payload.data()+40,8)==position,"process epoch/position");
  base.kind=f.kind;base.session=f.session;base.sequence=f.sequence;base.payload.assign(f.payload.begin(),f.payload.begin()+32);
 }
 void result(std::vector<uint8_t>& payload,uint32_t frames,uint32_t maximum=capacity){
  require(running&&frames<=maximum&&position<=UINT64_MAX-frames,"result timeline");payload.resize(32);put(payload.data()+16,epoch,8);put(payload.data()+24,position,8);position+=frames;
 }
};
}
