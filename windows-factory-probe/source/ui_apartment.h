#pragma once
#include <windows.h>
#include <objbase.h>
#include <stdexcept>
namespace linux_vst_bridge::wf0 {
// The controller, its view and COM services share the Windows UI apartment.
// This lease precedes module loading and outlives controller/view teardown.
class UiApartment {
  DWORD owner_ = GetCurrentThreadId();
public:
  HRESULT initial, initialized;
  UiApartment() {
    APTTYPE type{};
    APTTYPEQUALIFIER qualifier{};
    initial = CoGetApartmentType(&type, &qualifier);
    initialized = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    if (FAILED(initialized))
      throw std::runtime_error("Windows UI apartment initialization failed");
  }
  ~UiApartment() {
    if (GetCurrentThreadId() != owner_)
      std::terminate();
    CoUninitialize();
  }
  UiApartment(const UiApartment &) = delete;
  UiApartment &operator=(const UiApartment &) = delete;
};
}
