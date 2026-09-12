#pragma once
#include "ap10_backend.h"
// Native ABI extension only. Validation is identical to AP13. This distinct
// result authorizes local silence only after complete terminal custody; no
// Windows queue or result packet is submitted/delivered in that state.
namespace IF2 { inline constexpr uint32_t contained = 0x106; }
extern "C" {
// Bounded atomic classification; zero does not assert that the peer is healthy.
uint32_t if2_terminal_status(uint64_t);
uint32_t if2_process(uint64_t,uint32_t,const ap8_event_t*,uint32_t,const ap10_context_t*,uint64_t,const float*,const float*,float*,float*,uint64_t*,ap7_delivery_t*,uint64_t);
// Non-RT: normal close OR exact terminal custody plus positive cohort/transport
// retirement. A zero result here does not claim normal vendor SDK destruction.
uint32_t if2_close(uint64_t);
}
