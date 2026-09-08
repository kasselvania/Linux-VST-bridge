#pragma once
#include <cstdint>
// Process-result ABI v1, protocol minor 9. No SDK pointers or objects cross it.
inline constexpr uint32_t ap10_event_capacity=64, ap10_point_capacity=128;
inline constexpr uint32_t ap10_payload_capacity=4096, ap10_event_payload_capacity=512;
struct ap10_event_result_t {
 int32_t offset=0,bus=0;double ppq=0;uint16_t flags=0,kind=0;
 uint32_t payload_offset=0,payload_size=0;
 int32_t a=0,b=0,c=0;uint32_t d=0,reserved=0;uint64_t value=0,extra=0;
};
struct ap10_point_result_t {int32_t offset=0;uint32_t id=0;double value=0;};
struct ap10_results_t {
 uint32_t events=0,points=0,bytes=0,reserved=0;
 ap10_event_result_t event[ap10_event_capacity]{};
 ap10_point_result_t point[ap10_point_capacity]{};
 alignas(8) uint8_t payload[ap10_payload_capacity]{};
};
static_assert(sizeof(ap10_event_result_t)==64 && sizeof(ap10_point_result_t)==16);
static_assert(sizeof(ap10_results_t)==10256);
struct ap10_result_stats_t {
 uint64_t late_events,late_points,discarded_on_reset,pending_events,pending_points;
};
extern "C" {
uint32_t ap10_results_abi_version();
// Drain one bounded packet eligible for the most recent process callback.
// A zero event/point count marks the end. No host pointer is retained.
uint32_t ap10_take_results(uint64_t,ap10_results_t*);
uint32_t ap10_result_stats(uint64_t,ap10_result_stats_t*);
uint32_t ap10_fail_results(uint64_t);
}
