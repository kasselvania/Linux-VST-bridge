#pragma once
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <dlfcn.h>
#include <span>

// Versioned C ABI. Rust owns all pointed-to storage until module unload.
namespace AP8 {
struct Bus {
  uint32_t media, direction, index, channels, type, flags;
  uint64_t arrangement;
  char16_t name[128];
};
struct Parameter {
  uint32_t id;
  char16_t title[128], units[128];
  int32_t steps, flags;
  double initial;
  uint8_t available;
};
struct DescriptorView {
  uint32_t abi, size;
  uint8_t identity[48], engine_sha256[32], descriptor_sha256[32];
  uint8_t processor[16], controller[16];
  char class_name[64], vendor[64], version[64], subcategories[128];
  uint32_t bus_count, parameter_count;
  const Bus* buses;
  const Parameter* parameters;
};
extern "C" int32_t lvb_descriptor_open_v2(const char*, const void*, void**, DescriptorView*);
extern "C" void lvb_descriptor_close_v1(void*);

// Hidden symbols keep each loaded publication's discovery data independent.
inline std::span<const Bus> buses;
inline std::span<const Parameter> parameters;
inline uint8_t identity[48]{};
inline uint8_t engine_sha256[32]{}, descriptor_sha256[32]{};
inline char processorID[16]{}, controlID[16]{};
inline char class_name[64]{}, vendor[64]{}, version[64]{}, subcategories[128]{};

inline bool loadDescriptor() {
  struct Lifetime {
    void* owner = nullptr;
    bool valid = false;
    Lifetime() {
      Dl_info info{};
      DescriptorView view{};
      if (!dladdr(reinterpret_cast<void*>(&loadDescriptor), &info) || !info.dli_fname ||
          lvb_descriptor_open_v2(info.dli_fname,
            reinterpret_cast<const void*>(&loadDescriptor), &owner, &view) ||
          view.abi != 2 || view.size != sizeof(view)) return;
      std::memcpy(identity, view.identity, sizeof(identity));
      std::memcpy(engine_sha256, view.engine_sha256, sizeof(engine_sha256));
      std::memcpy(descriptor_sha256, view.descriptor_sha256, sizeof(descriptor_sha256));
      std::memcpy(processorID, view.processor, sizeof(processorID));
      std::memcpy(controlID, view.controller, sizeof(controlID));
      std::memcpy(class_name, view.class_name, sizeof(class_name));
      std::memcpy(vendor, view.vendor, sizeof(vendor));
      std::memcpy(version, view.version, sizeof(version));
      std::memcpy(subcategories, view.subcategories, sizeof(subcategories));
      buses = {view.buses, view.bus_count};
      parameters = {view.parameters, view.parameter_count};
      valid = true;
    }
    ~Lifetime() { lvb_descriptor_close_v1(owner); }
  };
  static Lifetime lifetime;
  return lifetime.valid;
}
} // namespace AP8
