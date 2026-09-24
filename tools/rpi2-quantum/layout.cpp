#include "../../windows-factory-probe/source/ap1_protocol.h"
#include <cassert>
using namespace linux_vst_bridge::ap1;
int main() {
 for(uint32_t c: {256u,512u}) {
  std::array<uint8_t,64> h{};auto s=(c+2)*4, output=64+2*s, bytes=output+2*s;
  for(auto pair: {std::pair{0u,0x4d315041u},{4u,c==256?1u:2u},{8u,c},{12u,2u},{16u,bytes},{20u,64u},{24u,output},{28u,s}})
   put(h.data()+pair.first,pair.second,4);
  assert(mapping_layout(h.data(),bytes)==(c==capacity));
  if(c==capacity) {
   for(auto offset:{0u,4u,8u,12u,16u,20u,24u,28u}) {
    auto old=get(h.data()+offset,4);put(h.data()+offset,old+1,4);
    assert(!mapping_layout(h.data(),bytes));put(h.data()+offset,old,4);
   }
   assert(!mapping_layout(h.data(),bytes-1));
  }
 }
 for(uint32_t max:{256u,512u}) for(uint32_t n:{0u,1u,255u,256u,257u,511u,512u,513u}) {
  bool accepted=true;try {processing_maximum(n,max);}catch(const std::runtime_error&){accepted=false;}
  assert(accepted==(max<=capacity&&n<=max));
 }
}
