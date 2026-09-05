#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* ABI 1. Synchronous offline calls; handle is an opaque generation, never a
   pointer. All buffers are borrowed for the call. 0=success, 1=invalid/state,
   2=backend, 3=busy, 4=panic caught. No call launches a process. */
uint32_t ap2_abi_version(void);
uint32_t ap2_error(uint8_t *output, uint32_t capacity);
int32_t ap2_open(uint32_t max_frames, uint64_t *handle);
int32_t ap2_transition(uint64_t handle,
                       uint32_t operation); /* 10=start,12=stop,14=deactivate */
int32_t ap2_process(uint64_t handle, uint32_t frames, double gain,
                    uint64_t input_silence, const float *left,
                    const float *right, float *out_left, float *out_right,
                    uint64_t *output_silence);
int32_t ap2_close(uint64_t handle);
#ifdef __cplusplus
}
#endif
