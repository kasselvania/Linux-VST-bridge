#pragma once
#include "ap8_backend.h"
#include "ap10_results.h"
#include "if1_terminal.h"
struct ap10_context_t {
 uint32_t present,state;double rate;int64_t project,system,continuous;
 double music,bar,cycle_start,cycle_end,tempo;int32_t numerator,denominator,clock;uint32_t reserved;
};
static_assert(sizeof(ap10_context_t)==96);
namespace AP10 { inline constexpr uint32_t parameter_curve_unavailable = 0x107; }
extern "C" {
// AP22 native-only metadata configuration, before processing starts. Exact SDK
// parameter IDs (maximum 8192), no values/defaults. Copied and sorted here;
// neither pointer nor C++ ownership crosses the boundary. Configured once.
uint32_t ap22_curve_parameters(uint64_t,const uint32_t*,uint32_t);
// Versioned planar C ABI: two pointers per SDK stereo output, null for inactive buses.
uint32_t ap19_process_outputs(uint64_t,uint32_t,const ap8_event_t*,uint32_t,const ap10_context_t*,uint64_t,const float*,const float*,float*const*,uint32_t,uint64_t*,ap7_delivery_t*,uint64_t);
uint32_t ap10_setup(uint64_t,uint32_t,uint32_t,double,const uint8_t*,uint32_t,uint32_t,uint32_t*);
uint32_t ap10_notices(uint64_t,uint32_t*);
// AP10 admission failures: 0x101 extent, 0x102 event, 0x103 nonfinite
// input, 0x104 contradictory input silence, 0x105 host context,
// 0x107 unavailable parameter curve anchor/capacity. 0x106 is reserved for
// IF2's independently owned terminal-silence result. Other
// nonzero results retain the existing queue/lifetime meanings.
// AP13 extension: final argument is native callback-entry CLOCK_MONOTONIC ns.
uint32_t ap13_process(uint64_t,uint32_t,const ap8_event_t*,uint32_t,const ap10_context_t*,uint64_t,const float*,const float*,float*,float*,uint64_t*,ap7_delivery_t*,uint64_t);
uint32_t ap10_process(uint64_t,uint32_t,const ap8_event_t*,uint32_t,const ap10_context_t*,uint64_t,const float*,const float*,float*,float*,uint64_t*,ap7_delivery_t*);
}
