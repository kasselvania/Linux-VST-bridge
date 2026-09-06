#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* ABI 1: all AP4 functions are owner-thread only, never concurrent with close.
   No C++ ownership crosses this boundary. Caller owns/caps all byte buffers.
   ap4_state releases registry ownership before waiting; callbacks may continue.
   restore=null snapshots; restore!=null applies and reads back. No retry. */
uint32_t ap4_open(uint64_t *handle);
/* Optional private preview diagnostic sink, non-RT owner thread only. */
uint32_t ap4_report(const uint8_t *bytes, uint32_t length);
uint32_t ap4_activate(uint64_t handle, uint32_t maximum, uint32_t mode);
uint32_t ap4_deactivate(uint64_t handle);
uint32_t ap4_state(uint64_t handle, const uint8_t *restore, uint32_t length,
                   uint8_t *output, uint32_t capacity, uint32_t *written);
uint32_t ap4_validate(const uint8_t *blob, uint32_t length, double *gain);
struct ap4_witness_t {
  uint64_t samples, restored_samples, before_edit_samples, restores, edits,
      nonzero_samples;
  double maximum_error, restored_gain;
};
uint32_t ap4_witness(uint64_t handle, struct ap4_witness_t *out);
/* Owner-thread failure readback; atomics and the worker detail only. This never
   reads mutable callback buffers or holds the callback registry guard. */
struct ap4_failure_t {
  uint64_t fault, first_position, processed;
  uint8_t detail[385];
};
uint32_t ap4_failure(uint64_t handle, struct ap4_failure_t *out);

#ifdef __cplusplus
}
#endif
