#pragma once
#include <cstdint>
// Independent IF1 query ABI v1, fixed scalar projection of LVIF v1 custody.
struct if1_terminal_t { uint64_t words[24]{}; };
static_assert(sizeof(if1_terminal_t)==192);
namespace IF1 {
inline bool valid(const if1_terminal_t& r) {
 const auto* w=r.words;
 if(w[0]!=1||(!w[1]&&!w[2])||!w[3]||w[14]<1||w[14]>3||w[18]<1||w[18]>3||w[19]<1||w[19]>5)return false;
 for(unsigned i=20;i<24;++i)if(w[i])return false;
 return true;
}
inline const char* status(const if1_terminal_t& r) {
 switch(r.words[14]){
 case 1:return "Windows host exited unexpectedly. This instance is unavailable.";
 case 2:return "Vendor editor/controller failed. This instance is unavailable.";
 default:return "Instance transport failed. This instance is unavailable.";
 }
}
}
extern "C" uint32_t if1_terminal(uint64_t,if1_terminal_t*);
