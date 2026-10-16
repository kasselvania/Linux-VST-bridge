#pragma once
// Why the bridge refused to load one plug-in instance, as one bounded code the
// processor passes to its controller (no free text crosses the boundary). The
// code comes from the backend's retained admission detail (ap2_error).
#include <cstdint>
#include <cstring>
namespace LVBAdmission {
enum Kind : int64_t {
  Unknown = 1,     // includes the stopped Windows side; the manager says which
  NotRunning = 2,  // the bridge service is not running
  Updated = 3,     // this DAW still holds an older bridge after an update
  Busy = 4,        // admission did not finish in its stated time
  Capacity = 5,    // the configured instance limit is reached
  Maintenance = 6, // the manager is changing the bridge or this plug-in
  Cleanup = 7,     // an earlier instance's cleanup is not yet confirmed
};
inline bool valid(int64_t kind) { return kind >= Unknown && kind <= Cleanup; }
inline Kind classify(const char *detail) {
  if (!detail) return Unknown;
  auto has = [detail](const char *code) { return std::strstr(detail, code) != nullptr; };
  if (has("admission_stale_native_caller")) return Updated;
  if (has("admission_service_busy")) return Busy;
  if (has("admission_global_capacity") || has("admission_class_capacity") ||
      has("admission_native_image_capacity")) return Capacity;
  if (has("admission_maintenance_active")) return Maintenance;
  if (has("admission_cleanup_unconfirmed")) return Cleanup;
  if (has("bridge_service_unreachable")) return NotRunning;
  return Unknown;
}
inline const char *text(int64_t kind) {
  switch (kind) {
  case NotRunning: return "The bridge service is not running, so this plug-in was not loaded.";
  case Updated: return "The bridge was updated. Quit and restart your DAW to load this plug-in.";
  case Busy: return "The bridge did not load this plug-in in time.";
  case Capacity: return "The bridge's plug-in limit is reached. Remove another instance first.";
  case Maintenance: return "The manager is changing the bridge. Load the plug-in when it finishes.";
  case Cleanup: return "The bridge is still confirming an earlier cleanup; the manager shows why.";
  default: return "The bridge did not load this plug-in; the manager shows why.";
  }
}
} // namespace LVBAdmission
