#pragma once
#include <cstdint>
// Optional observation ABI1; independent of AP23 completion ABI2 and IPC15.
// Borrowed immutable POD spans only, consumed synchronously off the audio thread.
extern "C" {
struct lvb_process_call_record_t {
 uint32_t schema,size;
 uint64_t instance,sequence,entry_ns,return_ns,backend_handle,configuration;
 double sample_rate;
 int64_t project_samples;
 uint32_t namespace_pid,namespace_tid;
 int32_t frames,mode,precision,maximum,phase,sdk_result;
 uint32_t valid,outcome;
};
struct lvb_process_call_summary_t {
 uint32_t schema,size,capacity,namespace_pid,record_size,flags;
 uint64_t instance,offered,retained,capacity_dropped,contention_dropped,
  allocation_dropped,invalid_clocks,invalid_identity,after_seal,
  unfinished_writers,sequence_overflow;
};
uint32_t lvb_process_call_export(uint64_t,const lvb_process_call_summary_t*,
 const lvb_process_call_record_t*,uint32_t);
}
namespace LVBCallTiming {
inline constexpr uint32_t schema=1,capacity=262144;
inline constexpr uint32_t requested=1,allocated=2,sealed=4,complete=8;
inline constexpr uint32_t entry_valid=1,return_valid=2,duration_valid=4,
 configuration_valid=8,project_valid=16,identity_valid=32,result_valid=64,
 owner_context_valid=128;
inline constexpr uint32_t returned=1,unwound=2;
static_assert(sizeof(lvb_process_call_record_t)==112);
static_assert(sizeof(lvb_process_call_summary_t)==112);
}
