#pragma once
#include "ap3_backend.h"
#include <cstdint>
struct ap8_event_t {uint32_t offset,kind,id;int16_t channel,pitch;double value;float tuning;uint32_t reserved;};
static_assert(sizeof(ap8_event_t)==32);
extern "C" {
uint32_t ap8_open(const uint8_t* identity, uint64_t* handle);
uint32_t ap8_validate(const uint8_t* identity,const uint8_t* blob,uint32_t length);
uint32_t ap8_process(uint64_t handle,uint32_t frames,const ap8_event_t* events,uint32_t count,const float* left,const float* right,float* out_left,float* out_right,uint64_t* flags,ap7_delivery_t* delivery);
}
