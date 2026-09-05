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
constexpr uint32_t guard=0x4b123456, poison=0x7fc12345;
constexpr uint64_t witness_mask=0x8d396b274e105ac3ULL;
enum Kind:uint16_t {Hello=1,Ready=2,Process=3,Done=4,Close=5,Closed=6,Error=7};
inline void require(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
inline uint64_t get(const uint8_t* p,size_t n){uint64_t v=0;for(size_t i=0;i<n;++i)v|=uint64_t(p[i])<<(8*i);return v;}
inline void put(uint8_t* p,uint64_t v,size_t n){for(size_t i=0;i<n;++i)p[i]=uint8_t(v>>(8*i));}
struct Frame {uint16_t kind;std::array<uint8_t,16> session;uint64_t sequence;std::vector<uint8_t> payload;};
inline std::vector<uint8_t> encode(const Frame& f){
 require(f.kind>=Hello&&f.kind<=Error&&f.payload.size()<=4040,"frame kind/length");
 std::vector<uint8_t>b(header_bytes+f.payload.size());put(b.data(),magic,4);put(b.data()+4,1,2);put(b.data()+6,1,2);
 put(b.data()+8,f.kind,2);put(b.data()+12,f.payload.size(),4);std::memcpy(b.data()+16,f.session.data(),16);
 put(b.data()+32,1,8);put(b.data()+40,f.sequence,8);if(!f.payload.empty())std::memcpy(b.data()+56,f.payload.data(),f.payload.size());return b;
}
inline size_t payload_length(const uint8_t* b){
 require(get(b,4)==magic&&get(b+4,2)==1&&get(b+6,2)==1&&get(b+10,2)==0,"protocol version/header");
 require(get(b+8,2)>=Hello&&get(b+8,2)<=Error&&get(b+32,8)==1&&get(b+48,8)==0,"protocol kind/instance/parent");
 auto n=get(b+12,4);require(n<=4040,"frame length");return size_t(n);
}
inline Frame decode(const std::vector<uint8_t>& b){
 require(b.size()>=header_bytes,"truncated header");auto n=payload_length(b.data());require(b.size()==header_bytes+n,"truncated/extra payload");
 Frame f{uint16_t(get(b.data()+8,2)),{},get(b.data()+40,8),{b.begin()+56,b.end()}};std::memcpy(f.session.data(),b.data()+16,16);return f;
}
struct Request {uint32_t frames;double gain;uint32_t silence;};
inline Request request(const Frame& f){
 require(f.kind==Process&&f.payload.size()==32,"process payload");auto p=f.payload.data();
 auto frames=get(p,4);auto gain_bits=get(p+16,8);double gain;std::memcpy(&gain,&gain_bits,8);
 require(frames>=1&&frames<=capacity&&get(p+4,4)==input_offset&&get(p+8,4)==output_offset&&get(p+12,4)==stride,"process extent");
 require(std::isfinite(gain)&&gain>=0&&gain<=1&&get(p+24,4)<=3&&get(p+28,4)==0,"gain/silence/reserved");
 return {uint32_t(frames),gain,uint32_t(get(p+24,4))};
}
// Actual native result handler used by MappedSession::done before publication.
// Output flags are independent claims; an unflagged channel may still be zero.
inline std::vector<uint8_t> processing_result(const Request& request,
                                             const float* left, const float* right,
                                             uint64_t silence) {
 require(request.frames>=1 && request.frames<=capacity,"result extent");
 require((silence & ~uint64_t(3))==0,"invalid output silence bits");
 for (size_t ch=0;ch<2;++ch) {
  const auto* samples=ch?right:left;
  for (size_t i=0;i<request.frames;++i) {
   require(std::isfinite(samples[i]),"nonfinite output sample");
   require(!(silence & (uint64_t(1)<<ch)) || samples[i]==0.f,
           "output silence claim has nonzero sample");
  }
 }
 std::vector<uint8_t> payload(16);
 put(payload.data(),request.frames,4);put(payload.data()+4,output_offset,4);
 put(payload.data()+8,silence,8);
 return payload;
}
struct Sequence {
 std::array<uint8_t,16> session{};uint64_t next=1;bool outstanding=false,closed=false,failed=false;
 Request begin(const Frame& f){
  try {require(!failed&&!closed&&!outstanding&&next<=64&&f.session==session&&f.sequence==next,"session/sequence/ownership");auto r=request(f);outstanding=true;return r;}
  catch(...){failed=true;throw;}
 }
 void complete(){require(outstanding&&!failed,"completion ownership");outstanding=false;++next;}
 void close(const Frame& f){try{require(!failed&&!closed&&!outstanding&&f.session==session&&f.sequence==next&&f.kind==Close&&f.payload.empty(),"close ownership");closed=true;}catch(...){failed=true;throw;}}
};
}
