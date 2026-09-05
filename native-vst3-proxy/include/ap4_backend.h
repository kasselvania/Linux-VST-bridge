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
uint32_t ap4_activate(uint64_t handle, uint32_t maximum, uint32_t mode);
uint32_t ap4_deactivate(uint64_t handle);
uint32_t ap4_state(uint64_t handle, const uint8_t *restore, uint32_t length,
                   uint8_t *output, uint32_t capacity, uint32_t *written);
uint32_t ap4_validate(const uint8_t *blob, uint32_t length, double *gain);
#ifdef __cplusplus
}
#endif
