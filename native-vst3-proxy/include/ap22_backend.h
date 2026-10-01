#pragma once
#include <cstdint>
// AP22 is a native-only C ABI extension. Windows protocol and its shared AP10
// header stay unchanged. An unavailable automation anchor/capacity is a refusal;
// 0x106 remains IF2's independently owned terminal-silence result.
namespace AP22 { inline constexpr uint32_t parameter_curve_unavailable = 0x107; }
extern "C" {
// Before processing starts, configure once from exact SDK parameter IDs
// (maximum 8192). IDs are copied/sorted; no values/defaults or C++ ownership
// cross the boundary, and the input pointer is not retained. Recovery keeps
// this census but invalidates the failed peer's cached numeric values.
uint32_t ap22_curve_parameters(uint64_t,const uint32_t*,uint32_t);
}
