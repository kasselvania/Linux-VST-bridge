#pragma once
#include <windows.h>
#include <string>
#include <vector>
namespace linux_vst_bridge::wf0 {
inline bool registered_module_path(const std::wstring& path) {
    // An exact C: path inside the manager-bound prefix, not a fixture location.
    // The caller independently verifies the registered module's SHA-256.
    if(path.size()<4 || path[0]!=L'C' || path[1]!=L':' || path[2]!=L'\\' ||
       path.find_first_of(L"/:*?\"<>|",2)!=std::wstring::npos ||
       path.find(L'\0')!=std::wstring::npos) return false;
    std::vector<wchar_t> canonical(32768);wchar_t* file_part=nullptr;
    auto n=GetFullPathNameW(path.c_str(),static_cast<DWORD>(canonical.size()),canonical.data(),&file_part);
    return n>0 && n<canonical.size() && path==canonical.data() && file_part && *file_part;
}
}
