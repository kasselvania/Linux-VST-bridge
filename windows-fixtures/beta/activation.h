// First-party stand-in for a vendor activation prompt (#414). Original code.
//
// Some protected plug-ins stop while Windows loads them and wait for the user
// to answer their own sign-in or activation window. This fixture does the same
// with an ordinary modal message box: until it is answered once, loading the
// module blocks inside LoadLibrary. Choosing OK records a fixture-private
// marker in the Windows user's roaming application data of that setup, so
// later loads (the bridge's re-check, a DAW load) do not ask again. Cancel
// leaves it unactivated and the next load asks again.
//
// No licence, account, network, service or vendor material is involved; the
// marker is an empty file owned by this fixture.
#include <windows.h>
#include <string>

namespace lvb_activation {
inline std::wstring folder() {
    wchar_t value[MAX_PATH]{};
    const DWORD size = GetEnvironmentVariableW(L"APPDATA", value, MAX_PATH);
    if (size == 0 || size >= MAX_PATH) return {};
    return std::wstring(value, size) + L"\\LVB Activation Fixture";
}
inline std::wstring marker() {
    const auto directory = folder();
    return directory.empty() ? std::wstring() : directory + L"\\activated";
}
inline bool activated() {
    const auto path = marker();
    if (path.empty()) return false;
    const DWORD attributes = GetFileAttributesW(path.c_str());
    return attributes != INVALID_FILE_ATTRIBUTES && !(attributes & FILE_ATTRIBUTE_DIRECTORY);
}
inline void record() {
    const auto directory = folder();
    if (directory.empty()) return;
    CreateDirectoryW(directory.c_str(), nullptr);
    const HANDLE file = CreateFileW(marker().c_str(), GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS,
        FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file != INVALID_HANDLE_VALUE) CloseHandle(file);
}
inline void ask(const wchar_t* name) {
    if (activated()) return;
    std::wstring text = std::wstring(name) +
        L" is not activated in this setup yet.\n\n"
        L"This first-party test plug-in stands in for a vendor's activation window. "
        L"Choose OK to activate it, or Cancel to leave it unactivated.";
    const int answer = MessageBoxW(nullptr, text.c_str(), L"LVB Activation Fixture",
        MB_OKCANCEL | MB_ICONINFORMATION | MB_SETFOREGROUND | MB_TOPMOST);
    if (answer == IDOK) record();
}
// Runs while Windows loads the module (C++ static initialisation inside the
// loader's process-attach notification), like a protection wrapper does.
struct AskWhileLoading {
    AskWhileLoading() {
        ask(LVB_BETA_INSTRUMENT ? L"LVB Activation Instrument" : L"LVB Activation Effect");
    }
};
static AskWhileLoading ask_while_loading;
} // namespace lvb_activation
