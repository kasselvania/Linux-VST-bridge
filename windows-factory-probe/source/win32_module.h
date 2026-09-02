#pragma once

#include "linux_vst_bridge/wf0_probe/events.h"

#include "pluginterfaces/base/ipluginbase.h"

#include <windows.h>

#include <string>

namespace linux_vst_bridge::wf0 {

using InitDllFn = bool (*)();
using ExitDllFn = bool (*)();
using GetPluginFactoryFn = Steinberg::IPluginFactory* (PLUGIN_API*)();

struct ModuleBinding {
    HMODULE module{nullptr};
    GetPluginFactoryFn get_factory{nullptr};
    InitDllFn init{nullptr};
    ExitDllFn exit{nullptr};
    bool required_export_found{false};
    bool init_present{false};
    bool exit_present{false};
    bool init_called{false};
    bool init_result{false};
    bool exit_called{false};
    bool exit_result{false};
    bool unload_attempted{false};
    bool unload_succeeded{false};
};

using FreeLibraryFn = BOOL (WINAPI*)(HMODULE);

std::wstring utf8_to_wide(const std::string& text);
std::string sha256_file(const std::wstring& path);
bool is_exact_absolute_module_path(const std::wstring& path);
int open_module(const std::wstring& path, ModuleBinding& binding, EventWriter& events);
int enter_module(ModuleBinding& binding, EventWriter& events, int primary_exit);
int exit_and_unload(ModuleBinding& binding, EventWriter& events, int primary_exit);
int exit_and_unload_with_api(ModuleBinding& binding, EventWriter& events,
                             int primary_exit, FreeLibraryFn free_library);

} // namespace linux_vst_bridge::wf0
