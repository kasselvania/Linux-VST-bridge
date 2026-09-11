#pragma once
#include <array>
#include <bit>
#include <cmath>
#include <algorithm>
#include <cstdint>
#include <string>
namespace linux_vst_bridge::wf0 {
// Trace-only fixed worker storage, drained after delivery retirement.
struct InputObservation {
 struct Row {uint64_t epoch=0,sequence=0,position=0,frames=0,silence=0;std::array<uint64_t,2> hash{};std::array<float,2> peak{};};
 std::array<Row,32> rows{};size_t count=0;uint64_t overflow=0;int last=-1;
 void observe(uint64_t epoch,uint64_t sequence,uint64_t position,uint64_t frames,uint64_t silence,const float* left,const float* right){
  if(!frames){return;}
  Row row{};row.epoch=epoch;row.sequence=sequence;row.position=position;row.frames=frames;row.silence=silence;
  const float* planes[]={left,right};for(size_t ch=0;ch<2;++ch){row.hash[ch]=14695981039346656037ull;for(size_t i=0;i<frames;++i){auto bits=std::bit_cast<uint32_t>(planes[ch][i]);for(int byte=0;byte<4;++byte){row.hash[ch]^=uint8_t(bits>>(byte*8));row.hash[ch]*=1099511628211ull;}row.peak[ch]=std::max(row.peak[ch],std::abs(planes[ch][i]));}}
  int nonzero=row.peak[0]>0||row.peak[1]>0;if(last==nonzero)return;last=nonzero;
  if(count==rows.size()){++overflow;return;}rows[count++]=row;
 }
 template<class Events> void dump(Events& events){
  events.lifecycle("ap18_input_observation",",\"retained\":"+std::to_string(count)+",\"overflow\":"+std::to_string(overflow)+",\"capacity\":32,\"collection\":\"windows_transport_worker\",\"hash\":\"fnv1a64_f32le_per_channel\"");
  for(size_t i=0;i<count;++i){const auto&r=rows[i];events.lifecycle("ap18_input",",\"epoch\":"+std::to_string(r.epoch)+",\"request_sequence\":"+std::to_string(r.sequence)+",\"position\":"+std::to_string(r.position)+",\"frames\":"+std::to_string(r.frames)+",\"silence\":"+std::to_string(r.silence)+",\"hash\":["+std::to_string(r.hash[0])+","+std::to_string(r.hash[1])+"],\"peak\":["+std::to_string(r.peak[0])+","+std::to_string(r.peak[1])+"]");}
 }
};
}
