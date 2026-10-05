#include <cstdint>
// Original product-owned PE companion. Wine's builtin loader owns the ELF
// lifetime; no vendor/runtime DLL is modified or replaced by this unique name.
extern "C" __declspec(dllexport) std::uint32_t lvb_wait_abi() noexcept { return 1; }
