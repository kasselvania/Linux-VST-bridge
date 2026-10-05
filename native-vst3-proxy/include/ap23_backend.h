#pragma once
#include "ap10_backend.h"
// Audio completion C ABI v2. SDK objects/pointers do not cross this boundary;
// only the existing bounded borrowed planar/event spans and explicit mode do.
extern "C" {
struct ap23_phase_trace_t {
 uint32_t schema;
 uint32_t size;
 uint64_t phase_reached;
 uint64_t clock_valid;
 uint64_t ordinal;
 uint64_t generation;
 uint64_t epoch;
 uint64_t host_call;
 uint64_t position;
 uint64_t frames;
 uint64_t mode;
 uint64_t delivery_mode;
 uint64_t exact;
 uint64_t allowance_ns;
 uint64_t wait_deadline_lower_ns;
 uint64_t wait_deadline_upper_ns;
 uint64_t predicate_kind;
 uint64_t predicate_required;
 uint64_t predicate_before;
 uint64_t predicate_after;
 uint64_t predicate_initial_satisfied;
 uint64_t predicate_final_satisfied;
 uint64_t cpp_entry_ns;
 uint64_t rust_entry_ns;
 uint64_t wait_begin_ns;
 uint64_t wait_end_ns;
 uint64_t wait_total_ns;
 uint64_t wait_count;
 uint64_t presentation_done_ns;
 uint64_t rust_pre_return_ns;
 uint64_t result_delivery_begin_ns;
 uint64_t result_delivery_done_ns;
 uint64_t cpp_pre_return_ns;
 uint32_t backend_result;
 int32_t sdk_result;
};
uint32_t ap23_process_outputs(uint64_t,uint32_t,uint32_t,const ap8_event_t*,uint32_t,
 const ap10_context_t*,uint64_t,const float*,const float*,float*const*,uint32_t,
 uint64_t*,ap7_delivery_t*,uint64_t);
// Diagnostic-only sibling. The ordinary processing ABI and result policy remain
// unchanged; callers prepare bounded storage before processing and never retain
// this pointer beyond the guarded call.
uint32_t ap23_phase_trace_enabled(uint64_t,uint32_t*);
uint32_t ap23_process_outputs_trace(uint64_t,uint32_t,uint32_t,const ap8_event_t*,uint32_t,
 const ap10_context_t*,uint64_t,const float*,const float*,float*const*,uint32_t,
 uint64_t*,ap7_delivery_t*,uint64_t,ap23_phase_trace_t*);
uint32_t ap23_cancel(uint64_t);
uint32_t ap23_deadline_failed(uint64_t);
// Consume the originating processing call's policy after all SDK result sinks.
// Refuses missing/reused policy, cancellation, fault or expired completion.
uint32_t ap23_finish_callback(uint64_t);
uint32_t ap23_abi_version();
}
namespace AP23 {
inline constexpr uint32_t abi_version=2;
inline constexpr bool compatibleAbi(uint32_t version) { return version==abi_version; }
inline constexpr uint32_t deadline_expired=0x108, cancelled=0x109, mode_refused=0x10A;
inline constexpr uint32_t phase_trace_schema=1, phase_trace_capacity=128;
inline constexpr uint64_t phase_cpp_entry=1ull<<0, phase_rust_entry=1ull<<1,
 phase_policy=1ull<<2, phase_identity=1ull<<3,
 phase_predicate_before=1ull<<4, phase_wait=1ull<<5,
 phase_predicate_after=1ull<<6, phase_presentation_done=1ull<<7,
 phase_rust_pre_return=1ull<<8, phase_backend_result=1ull<<9,
 phase_result_delivery_begin=1ull<<10, phase_cpp_pre_return=1ull<<11,
 phase_sdk_result=1ull<<12, phase_result_delivery_done=1ull<<13;
inline bool validMode(int configured,int actual) {
 return ((configured==0||configured==1)&&(actual==0||actual==1)) ||
        (configured==2&&actual==2);
}
}
