#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
uint32_t ap9_open(const uint8_t* identity,uint64_t* handle);
uint32_t ap9_setup(uint64_t handle,uint32_t maximum,uint32_t mode,double sample_rate,uint32_t* latency_and_tail);
#endif
/* ABI 1: all AP4 functions are owner-thread only, never concurrent with close.
   No C++ ownership crosses this boundary. Caller owns/caps all byte buffers.
   ap4_state releases registry ownership before waiting; callbacks may continue.
   restore=null snapshots; restore!=null applies and reads back. No retry. */
uint32_t ap4_open(uint64_t *handle);
/* Copy this instance's private report path; non-RT, caller-owned buffer. */
uint32_t ap5_report_path(uint64_t handle, uint8_t *path, uint32_t capacity);
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
/* AP5 diagnostic extension; preserves the AP4 witness layout and state format. */
struct ap5_observation_t {
  struct ap4_witness_t comparison;
  uint64_t input_hash, output_hash;
};
uint32_t ap5_observation(uint64_t handle, struct ap5_observation_t *out);
/* Owner-thread failure readback; atomics and the worker detail only. This never
   reads mutable callback buffers or holds the callback registry guard. */
struct ap4_failure_t {
  uint64_t fault, first_position, processed;
  uint8_t detail[385];
};
uint32_t ap4_failure(uint64_t handle, struct ap4_failure_t *out);

/* AP6: owner-thread operations. Opaque snapshots remain per logical instance.
   recover leaves the fresh endpoint inactive until SDK/controller synchronization.
   The revision must still match the explicitly selected snapshot. */
struct ap6_snapshot_t {
  uint64_t revision, generation;
  uint32_t source, uncaptured;
  uint8_t digest[32];
};
uint32_t ap6_snapshot(uint64_t handle, struct ap6_snapshot_t *out);
uint32_t ap6_recover(uint64_t handle, uint64_t revision, uint8_t *out,
                    uint32_t capacity, uint32_t *written);

#ifdef __cplusplus
}
#endif
