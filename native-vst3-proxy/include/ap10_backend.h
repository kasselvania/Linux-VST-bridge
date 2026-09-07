#pragma once
#include "ap8_backend.h"
struct ap10_context_t {
 uint32_t present,state;double rate;int64_t project,system,continuous;
 double music,bar,cycle_start,cycle_end,tempo;int32_t numerator,denominator,clock;uint32_t reserved;
};
static_assert(sizeof(ap10_context_t)==96);
extern "C" {
uint32_t ap10_setup(uint64_t,uint32_t,uint32_t,double,const uint8_t*,uint32_t,uint32_t*);
uint32_t ap10_process(uint64_t,uint32_t,const ap8_event_t*,uint32_t,const ap10_context_t*,uint64_t,const float*,const float*,float*,float*,uint64_t*,ap7_delivery_t*);
}
