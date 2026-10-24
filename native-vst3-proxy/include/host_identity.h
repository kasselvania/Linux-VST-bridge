#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Additive ABI v1. Exact execution admission is unchanged. Optional host_name
   is a copied VST3 self-report of 1..127 UTF-16 code units, without terminator.
   Invalid/absent metadata is unavailable, never a failed audio admission.
   Owner-thread initialization only; no callback, SDK object or pointer retained. */
uint32_t lvb_open_with_host_v1(const uint8_t *identity, const uint8_t *engine_sha256,
                             const uint8_t *descriptor_sha256,
                             const uint16_t *host_name, uint32_t length,
                             uint64_t *handle);
#ifdef __cplusplus
}
#endif
