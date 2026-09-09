#include "module_path.h"
#include <cassert>
using linux_vst_bridge::wf0::registered_module_path;
int main(){
 assert(registered_module_path(L"C:\\Program Files\\Common Files\\VST3\\A.vst3"));
 assert(registered_module_path(L"C:\\wf0\\fixture\\a.vst3"));
 for(const auto* p:{L"a.vst3",L"C:a.vst3",L"Z:\\a.vst3",L"\\\\server\\a.vst3",L"C:\\a\\..\\b.vst3",L"C:/a.vst3",L"C:\\a.vst3:stream",L"C:\\a*.vst3",L"C:\\"}) assert(!registered_module_path(p));
}
