#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* AP3 ABI 1. Same bounded borrowed float32 planes; opaque generation handle.
   process and start/stop are nonblocking and allocate nothing. Open,
   deactivate, stats and close are non-RT. Close joins before releasing storage. */
uint32_t ap3_abi_version(void);
uint32_t ap3_open(uint32_t maximum, uint64_t *handle);
uint32_t ap3_transition(uint64_t handle, uint32_t operation);
uint32_t ap3_process(uint64_t handle, uint32_t frames, double gain,
                     uint64_t input_silence, const float *left, const float *right,
                     float *out_left, float *out_right, uint64_t *output_silence);
/* AP7 delivery extension: zero return means a filled buffer, not necessarily
   gap-free audio. missing_frames are silenced at their original host time. */
struct ap7_delivery_t {
  uint64_t missing_frames, gaps, expired_frames, delivered_frames, priming_frames;
};
uint32_t ap7_process(uint64_t handle, uint32_t frames, double gain,
                    uint64_t input_silence, const float *left, const float *right,
                    float *out_left, float *out_right, uint64_t *output_silence,
                    struct ap7_delivery_t *delivery);
struct ap3_stats_t {
  uint64_t fault, first_position, processed, request_high, result_high, position,
      epoch;
};
uint32_t ap3_stats(uint64_t handle, struct ap3_stats_t *output);
uint32_t ap3_close(uint64_t handle);
#ifdef __cplusplus
}
#endif
