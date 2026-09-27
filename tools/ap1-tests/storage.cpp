#include "ap1_protocol.h"
#include <cstdlib>
#include <new>
#include <cstdio>
using namespace linux_vst_bridge::ap1;

static bool counting=false;
static size_t allocations=0,deallocations=0;
void* operator new(size_t n){if(counting)++allocations;if(auto*p=std::malloc(n))return p;throw std::bad_alloc();}
void operator delete(void*p) noexcept {if(counting)++deallocations;std::free(p);}
void operator delete(void*p,size_t) noexcept {if(counting)++deallocations;std::free(p);}
void* operator new[](size_t n){if(counting)++allocations;if(auto*p=std::malloc(n))return p;throw std::bad_alloc();}
void operator delete[](void*p) noexcept {if(counting)++deallocations;std::free(p);}
void operator delete[](void*p,size_t) noexcept {if(counting)++deallocations;std::free(p);}

int main(){
 Frame request_frame{Process,{7},1,std::vector<uint8_t>(8352)};
 Frame decoded{};decoded.payload.reserve(8352);
 Frame base{};base.payload.reserve(32);
 Frame reply{Done,{7},1,std::vector<uint8_t>()};reply.payload.reserve(10312);
 Frame returned{};returned.payload.reserve(10312);
 std::vector<uint8_t> request_wire,reply_wire;
 request_wire.reserve(header_bytes+8352);reply_wire.reserve(header_bytes+10312);
 const float zero[capacity]{};
 Timeline timeline;Frame start{Start,{7},0,std::vector<uint8_t>(8)};
 put(start.payload.data(),1,8);timeline.start(start);
 for(int iteration=0;iteration<5;++iteration){
  const uint32_t frames=iteration==3?0:iteration==1?1:iteration==2?17:capacity;
  request_frame.sequence=uint64_t(iteration+1);
  request_frame.payload.resize(8352);
  put(request_frame.payload.data(),frames,4);put(request_frame.payload.data()+4,input_offset,4);
  put(request_frame.payload.data()+8,output_offset,4);put(request_frame.payload.data()+12,stride,4);
  put(request_frame.payload.data()+28,0,4);put(request_frame.payload.data()+32,1,8);
  put(request_frame.payload.data()+40,timeline.position,8);
  reply.sequence=request_frame.sequence;
  counting=true;
  encode_into(request_frame,10,request_wire);
  decode_into(request_wire.data(),request_wire.size(),10,decoded);
  timeline.request_frame_into(decoded,base,true);
  auto shape=request(base,true);
  processing_result_into(shape,zero,zero,0,reply.payload);
  timeline.result(reply.payload,frames);
  reply.payload.resize(10312);
  encode_into(reply,10,reply_wire);
  decode_into(reply_wire.data(),reply_wire.size(),10,returned);
  counting=false;
  require(allocations==0&&deallocations==0,"reused processing codec allocated");
  require(returned.sequence==reply.sequence&&returned.session==reply.session&&returned.payload.size()==10312,"returned frame ownership");
 }
 bool rejected=false;auto bad=reply_wire;bad[48]=1;
 try{decode_into(bad.data(),bad.size(),10,returned);}catch(...){rejected=true;}
 require(rejected,"parent identity corruption accepted");
 std::puts("C++ production processing codec: zero allocations/deallocations across first and repeated maximum frames");
}
