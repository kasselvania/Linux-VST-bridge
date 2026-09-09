#include "module_path.h"
#include "win32_module.h"

#include <wincrypt.h>

#include <array>
#include <cctype>
#include <cstring>
#include <iomanip>
#include <sstream>
#include <stdexcept>
#include <vector>

namespace linux_vst_bridge::wf0 {
namespace {

std::string u32_hex(unsigned long value) {
    std::ostringstream out;
    out << std::hex << std::setfill('0') << std::setw(8)
        << static_cast<std::uint32_t>(value);
    return out.str();
}

template <typename Function>
Function resolve(HMODULE module, const char* name) {
    const FARPROC raw = GetProcAddress(module, name);
    static_assert(sizeof(Function) == sizeof(raw));
    Function typed = nullptr;
    std::memcpy(&typed, &raw, sizeof(typed));
    return typed;
}

} // namespace

std::wstring utf8_to_wide(const std::string& text) {
    if (text.empty()) {
        return {};
    }
    const int size = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                                         static_cast<int>(text.size()), nullptr, 0);
    if (size <= 0) {
        throw std::runtime_error("invalid UTF-8 argument");
    }
    std::wstring result(static_cast<std::size_t>(size), L'\0');
    if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                            static_cast<int>(text.size()), result.data(), size) != size) {
        throw std::runtime_error("UTF-8 conversion failed");
    }
    return result;
}

std::string sha256_file(const std::wstring& path) {
    HANDLE file = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr,
                              OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) {
        throw std::runtime_error("file hash open failed");
    }
    HCRYPTPROV provider = 0;
    HCRYPTHASH hash = 0;
    if (!CryptAcquireContextW(&provider, nullptr, nullptr, PROV_RSA_AES,
                              CRYPT_VERIFYCONTEXT) ||
        !CryptCreateHash(provider, CALG_SHA_256, 0, 0, &hash)) {
        CloseHandle(file);
        if (provider != 0) CryptReleaseContext(provider, 0);
        throw std::runtime_error("SHA-256 setup failed");
    }

    std::array<BYTE, 65536> buffer{};
    DWORD count = 0;
    bool ok = true;
    while (ReadFile(file, buffer.data(), static_cast<DWORD>(buffer.size()), &count,
                    nullptr) && count != 0) {
        if (!CryptHashData(hash, buffer.data(), count, 0)) {
            ok = false;
            break;
        }
    }
    if (GetLastError() != ERROR_SUCCESS && count != 0) ok = false;

    std::array<BYTE, 32> digest{};
    DWORD digest_size = static_cast<DWORD>(digest.size());
    if (!ok || !CryptGetHashParam(hash, HP_HASHVAL, digest.data(), &digest_size, 0) ||
        digest_size != digest.size()) {
        CryptDestroyHash(hash);
        CryptReleaseContext(provider, 0);
        CloseHandle(file);
        throw std::runtime_error("SHA-256 read failed");
    }

    CryptDestroyHash(hash);
    CryptReleaseContext(provider, 0);
    CloseHandle(file);

    static constexpr char hex[] = "0123456789abcdef";
    std::string result;
    result.reserve(64);
    for (BYTE byte : digest) {
        result.push_back(hex[byte >> 4]);
        result.push_back(hex[byte & 15]);
    }
    return result;
}

bool is_exact_absolute_module_path(const std::wstring& path) {
    return registered_module_path(path);
}

int open_module(const std::wstring& path, ModuleBinding& binding, EventWriter& events) {
    if (!is_exact_absolute_module_path(path)) return 64;
    if (!SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)) return 70;

    events.lifecycle("module_open_started");
    const auto attempt = events.call_started("load_library", nullptr);
    SetLastError(ERROR_SUCCESS);
    binding.module = LoadLibraryExW(path.c_str(), nullptr,
                                    LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR |
                                        LOAD_LIBRARY_SEARCH_SYSTEM32);
    const DWORD error = binding.module == nullptr ? GetLastError() : ERROR_SUCCESS;
    events.call_completed(attempt, "load_library", nullptr,
                          binding.module == nullptr ? "win32_error" : "handle_nonnull",
                          ",\"win32_error_u32_hex\":\"" + u32_hex(error) + "\"");
    if (binding.module == nullptr) return 70;
    events.lifecycle("module_opened");

    binding.get_factory = resolve<GetPluginFactoryFn>(binding.module, "GetPluginFactory");
    binding.required_export_found = binding.get_factory != nullptr;
    if (!binding.required_export_found) {
        binding.exit = resolve<ExitDllFn>(binding.module, "ExitDll");
        binding.exit_present = binding.exit != nullptr;
        events.lifecycle("factory_export_missing");
        return 72;
    }

    binding.init = resolve<InitDllFn>(binding.module, "InitDll");
    binding.exit = resolve<ExitDllFn>(binding.module, "ExitDll");
    binding.init_present = binding.init != nullptr;
    binding.exit_present = binding.exit != nullptr;
    return 0;
}

int enter_module(ModuleBinding& binding, EventWriter& events, int primary_exit) {
    if (!binding.required_export_found) return primary_exit;
    if (binding.init_present) {
        const auto attempt = events.call_started("init_dll", nullptr);
        binding.init_called = true;
        binding.init_result = binding.init();
        events.call_completed(attempt, "init_dll", nullptr, "bool",
                              std::string(",\"bool_result\":") +
                                  (binding.init_result ? "true" : "false"));
        events.lifecycle(binding.init_result ? "module_entry_succeeded" : "module_entry_failed");
    } else {
        events.lifecycle("module_entry_absent");
    }
    events.lifecycle("factory_export_found");
    if (binding.init_present && !binding.init_result && primary_exit == 0) return 71;
    return primary_exit;
}

int exit_and_unload_with_api(ModuleBinding& binding, EventWriter& events,
                             int primary_exit, FreeLibraryFn free_library) {
    if (binding.module == nullptr) return primary_exit;
    int result = primary_exit;
    if (binding.exit_present) {
        const auto attempt = events.call_started("exit_dll", nullptr);
        binding.exit_called = true;
        binding.exit_result = binding.exit();
        events.call_completed(attempt, "exit_dll", nullptr, "bool",
                              std::string(",\"bool_result\":") +
                                  (binding.exit_result ? "true" : "false"));
        events.lifecycle(binding.exit_result ? "module_exit_succeeded" : "module_exit_failed");
        if (!binding.exit_result && result == 0) result = 79;
    } else {
        events.lifecycle("module_exit_absent");
    }

    const auto attempt = events.call_started("free_library", nullptr);
    binding.unload_attempted = true;
    binding.unload_succeeded = free_library(binding.module) != 0;
    const DWORD error = binding.unload_succeeded ? ERROR_SUCCESS : GetLastError();
    events.call_completed(attempt, "free_library", nullptr,
                          binding.unload_succeeded ? "bool_true" : "win32_error",
                          ",\"win32_error_u32_hex\":\"" + u32_hex(error) + "\"");
    binding.module = nullptr;
    if (binding.unload_succeeded) {
        events.lifecycle("module_unloaded");
    } else if (result == 0) {
        result = 80;
    }
    return result;
}

int exit_and_unload(ModuleBinding& binding, EventWriter& events, int primary_exit) {
    return exit_and_unload_with_api(binding, events, primary_exit, &FreeLibrary);
}

} // namespace linux_vst_bridge::wf0
