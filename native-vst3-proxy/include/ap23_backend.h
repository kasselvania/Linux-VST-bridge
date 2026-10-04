#pragma once
#include "ap10_backend.h"
// Audio completion C ABI v1. SDK objects/pointers do not cross this boundary;
// only the existing bounded borrowed planar/event spans and explicit mode do.
extern "C" {
uint32_t ap23_process_outputs(uint64_t,uint32_t,uint32_t,const ap8_event_t*,uint32_t,
 const ap10_context_t*,uint64_t,const float*,const float*,float*const*,uint32_t,
 uint64_t*,ap7_delivery_t*,uint64_t);
uint32_t ap23_cancel(uint64_t);
uint32_t ap23_deadline_failed(uint64_t);
uint32_t ap23_abi_version();
}
namespace AP23 {
inline constexpr uint32_t deadline_expired=0x108, cancelled=0x109, mode_refused=0x10A;
inline bool validMode(int configured,int actual) {
 return ((configured==0||configured==1)&&(actual==0||actual==1)) ||
        (configured==2&&actual==2);
}
}
