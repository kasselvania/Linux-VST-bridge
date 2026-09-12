#pragma once
#include "ap8_backend.h"
#include "ap10_results.h"
#include "if1_terminal.h"
struct ap10_context_t {
 uint32_t present,state;double rate;int64_t project,system,continuous;
 double music,bar,cycle_start,cycle_end,tempo;int32_t numerator,denominator,clock;uint32_t reserved;
};
static_assert(sizeof(ap10_context_t)==96);
extern "C" {
uint32_t ap10_setup(uint64_t,uint32_t,uint32_t,double,const uint8_t*,uint32_t,uint32_t,uint32_t*);
uint32_t ap10_notices(uint64_t,uint32_t*);
// AP10 admission failures: 0x101 extent, 0x102 event, 0x103 nonfinite
// input, 0x104 contradictory input silence, 0x105 host context. Other
// nonzero results retain the existing queue/lifetime meanings.
// AP13 extension: final argument is native callback-entry CLOCK_MONOTONIC ns.
uint32_t ap13_process(uint64_t,uint32_t,const ap8_event_t*,uint32_t,const ap10_context_t*,uint64_t,const float*,const float*,float*,float*,uint64_t*,ap7_delivery_t*,uint64_t);
uint32_t ap10_process(uint64_t,uint32_t,const ap8_event_t*,uint32_t,const ap10_context_t*,uint64_t,const float*,const float*,float*,float*,uint64_t*,ap7_delivery_t*);
}
