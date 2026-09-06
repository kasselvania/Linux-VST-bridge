#ifdef WF0_LOADER_ADAPTER_TEST

#include "component_instance_session.h"
#include "win32_module.h"

#include <windows.h>

#include <cstdint>
#include <cstdio>

namespace {

BOOL WINAPI fail_free_library(HMODULE) {
    SetLastError(ERROR_INVALID_HANDLE);
    return FALSE;
}

} // namespace

int main() {
    static constexpr const char* operations[] = {
        "load_library", "init_dll", "get_plugin_factory", "get_factory_info",
        "query_factory_2", "query_factory_3", "count_classes",
        "get_class_info_unicode", "get_class_info_2", "get_class_info_1",
        "release_factory_3", "release_factory_2", "release_factory_base",
        "exit_dll", "free_library", "create_component",
        "get_controller_class_id", "initialize_component",
        "query_audio_processor", "release_audio_processor",
        "terminate_component", "release_component"};
    if (sizeof(operations) / sizeof(operations[0]) != 22 ||
        !linux_vst_bridge::wf0::run_component_owner_regressions()) return 1;
    linux_vst_bridge::wf0::EventWriter events(4096);
    linux_vst_bridge::wf0::ModuleBinding binding;
    binding.module = reinterpret_cast<HMODULE>(static_cast<std::uintptr_t>(1));
    const int result = linux_vst_bridge::wf0::exit_and_unload_with_api(
        binding, events, 0, &fail_free_library);
    if (result != 80 || !binding.unload_attempted || binding.unload_succeeded ||
        binding.module != nullptr) {
        return 2;
    }
    for (const char* operation : operations) {
        if (std::fprintf(stderr, "wf0-loader-adapter-operation=%s\n", operation) < 0) {
            return 3;
        }
    }
    if (std::fflush(stderr) != 0) return 3;
    Sleep(500);
    return 0;
}

#else

#include "component_instance_session.h"
#include "factory_census.h"
#include "mapped_processing.h"
#include "win32_module.h"

#include <windows.h>

#include <algorithm>
#include <array>
#include <cctype>
#include <cstdio>
#include <cstdlib>
#include <map>
#include <stdexcept>
#include <string>
#include <vector>

namespace wf0 = linux_vst_bridge::wf0;

namespace {

constexpr const char* kHandshakeSchema = "linux-vst-bridge-wf0-handshake/v1";

bool is_lower_hex(const std::string& value, std::size_t size) {
    return value.size() == size &&
           std::all_of(value.begin(), value.end(), [](unsigned char character) {
               return std::isdigit(character) || (character >= 'a' && character <= 'f');
           });
}

std::map<std::string, std::string> parse_args(int argc, char** argv) {
    static constexpr const char* ordered[] = {
        "--session", "--scanner-sha256", "--implementation-source-manifest-sha256",
        "--module", "--module-sha256", "--bundle-manifest-sha256", "--ready",
        "--gate", "--max-classes", "--stdout-cap", "--mode", "--component-case"};
    if (argc != 25) throw std::runtime_error("wrong argument count");
    std::map<std::string, std::string> result;
    for (int index = 0; index < 12; ++index) {
        if (std::string(argv[index * 2 + 1]) != ordered[index])
            throw std::runtime_error("argument order mismatch");
        result.emplace(ordered[index], argv[index * 2 + 2]);
    }
    if (!is_lower_hex(result["--session"], 32) ||
        !is_lower_hex(result["--scanner-sha256"], 64) ||
        !is_lower_hex(result["--implementation-source-manifest-sha256"], 64) ||
        !is_lower_hex(result["--module-sha256"], 64) ||
        !is_lower_hex(result["--bundle-manifest-sha256"], 64) ||
        result["--max-classes"] != "256" || result["--stdout-cap"] != "1048576" ||
        (result["--mode"] != "wa0-audio-processor-interface-admission" &&
         result["--mode"] != "pc0-pre-setup-processing-contract" &&
         result["--mode"] != "ap0-offline-again-processing" &&
         result["--mode"] != "ap1-linux-windows-audio-roundtrip" &&
         result["--mode"] != "ap2-native-vst3-offline-bridge" &&
         result["--mode"] != "ap3-queued-audio-preview" && result["--mode"] != "ap4-plugin-state-recall") ||
        result["--component-case"] != "exact-again") {
        throw std::runtime_error("argument value mismatch");
    }
    const std::string suffix = result["--session"] + ".ready";
    const std::string gate_suffix = result["--session"] + ".gate";
    if (result["--ready"] != "C:\\wf0\\session\\" + suffix ||
        result["--gate"] != "C:\\wf0\\session\\" + gate_suffix) {
        throw std::runtime_error("handshake path mismatch");
    }
    return result;
}

std::wstring executable_path() {
    std::vector<wchar_t> path(32768);
    const DWORD size = GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
    if (size == 0 || size >= path.size()) throw std::runtime_error("executable path unavailable");
    return std::wstring(path.data(), size);
}

std::string handshake(const std::map<std::string, std::string>& args) {
    return std::string("schema=") + kHandshakeSchema + "\n" +
           "session=" + args.at("--session") + "\n" +
           "scanner_sha256=" + args.at("--scanner-sha256") + "\n" +
           "module_sha256=" + args.at("--module-sha256") + "\n" +
           "bundle_manifest_sha256=" + args.at("--bundle-manifest-sha256") + "\n" +
           "implementation_source_manifest_sha256=" +
               args.at("--implementation-source-manifest-sha256") + "\n" +
           "mode=" + args.at("--mode") + "\n" +
           "component_case=" + args.at("--component-case") + "\n" +
           "run_ordinal=1\n";
}

bool path_absent(const std::wstring& path) {
    SetLastError(ERROR_SUCCESS);
    const DWORD attributes = GetFileAttributesW(path.c_str());
    return attributes == INVALID_FILE_ATTRIBUTES &&
           (GetLastError() == ERROR_FILE_NOT_FOUND || GetLastError() == ERROR_PATH_NOT_FOUND);
}

void atomic_write(const std::wstring& path, const std::string& bytes) {
    const std::wstring temporary = path + L".tmp";
    if (!path_absent(path) || !path_absent(temporary) || bytes.size() > 1024)
        throw std::runtime_error("handshake target already exists or is oversized");
    HANDLE file = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                              FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) throw std::runtime_error("ready create failed");
    DWORD written = 0;
    const bool ok = WriteFile(file, bytes.data(), static_cast<DWORD>(bytes.size()), &written,
                              nullptr) != 0 &&
                    written == bytes.size() && FlushFileBuffers(file) != 0;
    CloseHandle(file);
    if (!ok || !MoveFileExW(temporary.c_str(), path.c_str(), MOVEFILE_WRITE_THROUGH)) {
        DeleteFileW(temporary.c_str());
        throw std::runtime_error("ready publication failed");
    }
}

std::string read_bounded(const std::wstring& path) {
    HANDLE file = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr,
                              OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) throw std::runtime_error("gate open failed");
    std::array<char, 1025> bytes{};
    DWORD read = 0;
    const bool ok = ReadFile(file, bytes.data(), static_cast<DWORD>(bytes.size()), &read, nullptr) != 0;
    CloseHandle(file);
    if (!ok || read == 0 || read > 1024) throw std::runtime_error("gate read failed");
    return std::string(bytes.data(), read);
}

void wait_for_gate(const std::wstring& gate, const std::string& expected) {
    for (int attempt = 0; attempt < 3600; ++attempt) {
        const DWORD attributes = GetFileAttributesW(gate.c_str());
        if (attributes != INVALID_FILE_ATTRIBUTES) {
            if ((attributes & FILE_ATTRIBUTE_DIRECTORY) != 0 || read_bounded(gate) != expected)
                throw std::runtime_error("gate binding mismatch");
            return;
        }
        Sleep(50);
    }
    throw std::runtime_error("gate wait expired");
}

std::string bool_json(bool value) { return value ? "true" : "false"; }

wf0::ComponentCase component_case(const std::string& value) {
    if (value == "exact-again") return wf0::ComponentCase::exact_again;
    if (value == "unknown-processor") return wf0::ComponentCase::unknown_processor;
    if (value == "unsupported-interface")
        return wf0::ComponentCase::unsupported_interface;
    throw std::runtime_error("component case is outside the closed contract");
}

void suppressed_shutdown(wf0::EventWriter& events,
                         const wf0::FactoryCensusResult& census,
                         bool audio_interface_quiescence_unproved) {
    const std::string disposition =
        std::string(",\"disposition\":\"") +
        (audio_interface_quiescence_unproved
             ? "not_attempted_audio_interface_quiescence_unproved"
             : "not_attempted_object_quiescence_unproved") +
        "\"";
    if (audio_interface_quiescence_unproved) {
        events.lifecycle("inherited_shutdown_suppressed",
                         ",\"operation\":\"terminate_component\"" + disposition);
        events.lifecycle("inherited_shutdown_suppressed",
                         ",\"operation\":\"release_component\"" + disposition);
    }
    if (census.factory3 != nullptr)
        events.lifecycle("inherited_shutdown_suppressed",
                         ",\"operation\":\"release_factory_3\"" + disposition);
    if (census.factory2 != nullptr)
        events.lifecycle("inherited_shutdown_suppressed",
                         ",\"operation\":\"release_factory_2\"" + disposition);
    if (census.base_acquired)
        events.lifecycle("inherited_shutdown_suppressed",
                         ",\"operation\":\"release_factory_base\"" + disposition);
    events.lifecycle("inherited_shutdown_suppressed",
                     ",\"operation\":\"exit_dll\"" + disposition);
    events.lifecycle("inherited_shutdown_suppressed",
                     ",\"operation\":\"free_library\"" + disposition);
}

} // namespace

int main(int argc, char** argv) {
    std::setvbuf(stdout, nullptr, _IONBF, 0);
    bool pc0_mode = false;
    int first_primary = 0;
    try {
        const auto args = parse_args(argc, argv);
        pc0_mode = args.at("--mode") == "pc0-pre-setup-processing-contract";
        const std::wstring module_path = wf0::utf8_to_wide(args.at("--module"));
        const std::wstring ready_path = wf0::utf8_to_wide(args.at("--ready"));
        const std::wstring gate_path = wf0::utf8_to_wide(args.at("--gate"));
        if (!wf0::is_exact_absolute_module_path(module_path) || !path_absent(gate_path)) return 64;
        if (wf0::sha256_file(executable_path()) != args.at("--scanner-sha256") ||
            wf0::sha256_file(module_path) != args.at("--module-sha256")) return 64;

        wf0::EventWriter events(1048576);
        events.lifecycle("scanner_started", ",\"session\":\"" + args.at("--session") + "\"");
        const std::string binding = handshake(args);
        atomic_write(ready_path, binding);
        events.lifecycle(
            "readiness_announced",
            ",\"session\":\"" + args.at("--session") +
                "\",\"scanner_sha256\":\"" + args.at("--scanner-sha256") +
                "\",\"module_sha256\":\"" + args.at("--module-sha256") +
                "\",\"bundle_manifest_sha256\":\"" +
                args.at("--bundle-manifest-sha256") +
                "\",\"implementation_source_manifest_sha256\":\"" +
                args.at("--implementation-source-manifest-sha256") +
                "\",\"mode\":\"" + args.at("--mode") +
                "\",\"component_case\":\"" + args.at("--component-case") +
                "\",\"run_ordinal\":1");
        wait_for_gate(gate_path, binding);
        if (wf0::sha256_file(module_path) != args.at("--module-sha256")) return 65;
        events.lifecycle("supervisor_gate_accepted");

        const bool ap4_mode=args.at("--mode")=="ap4-plugin-state-recall";
        const bool ap3_mode=args.at("--mode")=="ap3-queued-audio-preview";
        const bool ap2_mode=args.at("--mode")=="ap2-native-vst3-offline-bridge";
        const bool ap1_mode=args.at("--mode")=="ap1-linux-windows-audio-roundtrip";
        std::unique_ptr<wf0::MappedSession> mapped;
        if(ap1_mode||ap2_mode||ap3_mode||ap4_mode) mapped=std::make_unique<wf0::MappedSession>(
            ready_path.substr(0,ready_path.find_last_of(L"\\/")),args.at("--session"),events,ap2_mode||ap3_mode||ap4_mode,ap3_mode||ap4_mode,ap4_mode);
        wf0::ModuleBinding module;
        int primary = wf0::open_module(module_path, module, events);
        first_primary = primary;
        if (primary == 0) primary = wf0::enter_module(module, events, primary);
        if (first_primary == 0) first_primary = primary;

        Steinberg::IPluginFactory* factory = nullptr;
        wf0::FactoryCensusResult census;
        wf0::ComponentAdmissionResult component;
        bool component_session_ran = false;
        if (primary == 0) {
            events.lifecycle("factory_get_started");
            const auto attempt = events.call_started("get_plugin_factory", nullptr);
            factory = module.get_factory();
            events.call_completed(attempt, "get_plugin_factory", nullptr,
                                  factory == nullptr ? "pointer_null" : "pointer_nonnull");
            if (factory == nullptr) {
                events.lifecycle("factory_get_failed");
                primary = 73;
                if (first_primary == 0) first_primary = primary;
            } else {
                events.lifecycle("factory_obtained");
                census = wf0::enumerate_factory(factory, events, 256);
                if (census.exit_code != 0) primary = census.exit_code;
                if (first_primary == 0) first_primary = primary;
            }
        }

        if (factory != nullptr && primary == 0) {
            component = wf0::admit_component(
                factory, events, component_case(args.at("--component-case")),
                pc0_mode || ap1_mode || ap2_mode || ap3_mode || ap4_mode || args.at("--mode") == "ap0-offline-again-processing",
                ap1_mode || ap2_mode || ap3_mode || ap4_mode || args.at("--mode") == "ap0-offline-again-processing", mapped.get());
            component_session_ran = true;
            if (component.primary_exit != 0) primary = component.primary_exit;
            if (first_primary == 0) first_primary = primary;
            events.final_lifecycle("component_session_closed", component.json_fields());
        }

        if (factory != nullptr &&
            (!component_session_ran || component.object_quiescence)) {
            primary = wf0::release_factory_interfaces(factory, census, events, primary);
            if (first_primary == 0) first_primary = primary;
        }
        if (!component_session_ran || component.object_quiescence) {
            primary = wf0::exit_and_unload(module, events, primary);
            if (first_primary == 0) first_primary = primary;
        } else {
            suppressed_shutdown(
                events, census,
                component.audio_processor_session_ran &&
                    !component.audio_processor.audio_interface_quiescence);
        }
        if(mapped) {
            if(!component.object_quiescence) mapped.release(); // OS containment; no premature unmap.
            else if(primary==0) mapped->finish(true);
        }
        if (primary != 0) return primary;

        const std::string fields =
            ",\"module_entry\":{\"present\":" + bool_json(module.init_present) +
            ",\"called\":" + bool_json(module.init_called) + ",\"result\":" +
            bool_json(module.init_result) + "},\"module_exit\":{\"present\":" +
            bool_json(module.exit_present) + ",\"called\":" + bool_json(module.exit_called) +
            ",\"result\":" + bool_json(module.exit_result) +
            "},\"module_unload\":{\"attempted\":" + bool_json(module.unload_attempted) +
            ",\"succeeded\":" + bool_json(module.unload_succeeded) + "}" +
            wf0::census_json_fields(census.census) + component.json_fields() +
            ",\"create_instance_called\":true" +
            ",\"controller_instance_created\":false" +
            ",\"audio_processor_interface_queried\":true" +
            ",\"audio_processor_method_called\":" +
                bool_json(args.at("--mode") != "wa0-audio-processor-interface-admission");
        events.final_lifecycle("scanner_completed", fields);
        return 0;
    } catch (int pc0_primary) {
        return pc0_primary;
    } catch (const std::exception&) {
        return wf0::scanner_output_failure_exit(pc0_mode, first_primary);
    }
}

#endif
