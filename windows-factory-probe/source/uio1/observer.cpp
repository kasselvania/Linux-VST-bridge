#include "record.h"
#include <windows.h>
#include <tlhelp32.h>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <stdexcept>
#include <algorithm>
#include <iterator>
namespace {
constexpr UINT pulse=WM_APP+0x541;
void check(bool ok,const char*s){if(!ok)throw std::runtime_error(s);}
uint64_t now(){LARGE_INTEGER n{};QueryPerformanceCounter(&n);return uint64_t(n.QuadPart);}
uint64_t start(HANDLE p){FILETIME c{},e{},k{},u{};check(GetProcessTimes(p,&c,&e,&k,&u)!=0,"process identity");return(uint64_t(c.dwHighDateTime)<<32)|c.dwLowDateTime;}
struct Census {DWORD pid;unsigned count=0;};
BOOL CALLBACK window(HWND w,LPARAM l){auto&c=*reinterpret_cast<Census*>(l);DWORD pid=0;const auto tid=GetWindowThreadProcessId(w,&pid);
  if(pid!=c.pid)return TRUE;if(c.count>=128)return FALSE;
  RECT r{};GetWindowRect(w,&r);wchar_t cls[128]{};GetClassNameW(w,cls,128);
  uint64_t hash=1469598103934665603ULL;for(auto ch:cls){if(!ch)break;hash=(hash^uint16_t(ch))*1099511628211ULL;}
  printf("{\"type\":\"window\",\"hwnd\":%llu,\"parent\":%llu,\"tid\":%lu,\"class_hash\":%llu,\"visible\":%u,\"enabled\":%u,\"minimized\":%u,\"dpi\":%u,\"rect\":[%ld,%ld,%ld,%ld]}\n",uint64_t(w),uint64_t(GetParent(w)),tid,hash,unsigned(IsWindowVisible(w)!=0),unsigned(IsWindowEnabled(w)!=0),unsigned(IsIconic(w)!=0),GetDpiForWindow(w),r.left,r.top,r.right,r.bottom);printf("{\"type\":\"x11_binding\",\"hwnd\":%llu,\"xid\":%llu}\n",uint64_t(w),uint64_t(GetPropW(w,L"__wine_x11_whole_window")));++c.count;return TRUE;
}
BOOL CALLBACK top(HWND w,LPARAM l){auto&c=*reinterpret_cast<Census*>(l);DWORD pid=0;GetWindowThreadProcessId(w,&pid);if(pid==c.pid){window(w,l);EnumChildWindows(w,window,l);}return c.count<128;}
void census(DWORD pid){HANDLE p=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,pid);check(p!=nullptr,"census process");auto born=start(p);CloseHandle(p);printf("{\"type\":\"process\",\"pid\":%lu,\"start\":%llu}\n",pid,born);
  Census c{pid};EnumWindows(top,reinterpret_cast<LPARAM>(&c));
  HANDLE s=CreateToolhelp32Snapshot(TH32CS_SNAPMODULE,pid);check(s!=INVALID_HANDLE_VALUE,"module census");
  MODULEENTRY32W m{};m.dwSize=sizeof(m);unsigned n=0;
  if(Module32FirstW(s,&m))do{
    // Closed renderer labels, never arbitrary module paths or vendor strings.
    const wchar_t* names[]={L"d3d9.dll",L"d3d11.dll",L"d3d12.dll",L"dxgi.dll",L"d2d1.dll",L"opengl32.dll",L"vulkan-1.dll",L"winevulkan.dll",L"wined3d.dll",L"gdi32.dll",L"uiautomationcore.dll"};
    for(unsigned i=0;i<std::size(names);++i)if(_wcsicmp(m.szModule,names[i])==0)printf("{\"type\":\"renderer_module\",\"kind\":%u}\n",i);
  }while(++n<512&&Module32NextW(s,&m));CloseHandle(s);
  printf("{\"type\":\"census_end\",\"window_capacity\":128,\"window_bound_reached\":%s,\"module_capacity\":512,\"module_bound_reached\":%s}\n",c.count==128?"true":"false",n==512?"true":"false");
}
struct Owner {
  HANDLE file=INVALID_HANDLE_VALUE,map=nullptr,process=nullptr,thread=nullptr;
  HMODULE dll=nullptr;HHOOK hooks[3]{};uio1::Header* h=nullptr;
  ~Owner(){
    if(h){
      uio1::atom(h->stop).store(1);
      PostThreadMessageW(h->tid,pulse,0,LPARAM(h->nonce));
      const auto end=GetTickCount64()+500;
      while(!uio1::atom(h->detached).load()&&GetTickCount64()<end)Sleep(5);
    }
    uint64_t errors=0;for(auto hook:hooks)if(hook&&!UnhookWindowsHookEx(hook))++errors;
    // Unhook can return with an in-flight callback. The mapped view belongs
    // separately to the injected DLL and remains valid until DLL unload; no
    // remote FreeLibrary or forced target-thread termination is attempted.
    if(h){uio1::atom(h->unhook_errors).store(errors);uio1::atom(h->closed).store(errors?2:1,std::memory_order_release);UnmapViewOfFile(h);}
    if(dll)FreeLibrary(dll);if(thread)CloseHandle(thread);if(process)CloseHandle(process);if(map)CloseHandle(map);if(file!=INVALID_HANDLE_VALUE)CloseHandle(file);
  }
};
int observe(HWND root,uint64_t expected,unsigned seconds,const wchar_t* output){
  check(seconds>=1&&seconds<=180,"duration bound");Owner o;DWORD pid=0;const auto tid=GetWindowThreadProcessId(root,&pid);check(pid&&tid&&IsWindow(root),"editor window");
  o.process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION|SYNCHRONIZE,FALSE,pid);check(o.process&&start(o.process)==expected,"process replaced");
  o.thread=OpenThread(THREAD_QUERY_LIMITED_INFORMATION|SYNCHRONIZE,FALSE,tid);check(o.thread!=nullptr,"thread identity");
  o.file=CreateFileW(output,GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,nullptr);check(o.file!=INVALID_HANDLE_VALUE,"new private mapping");
  LARGE_INTEGER length{};length.QuadPart=uio1::mapping_bytes;check(SetFilePointerEx(o.file,length,nullptr,FILE_BEGIN)&&SetEndOfFile(o.file),"mapping extent");
  wchar_t name[96]{};swprintf_s(name,L"Local\\LVBUIO1-%lu-%lu",pid,tid);
  o.map=CreateFileMappingW(o.file,nullptr,PAGE_READWRITE,0,uio1::mapping_bytes,name);check(o.map&&GetLastError()!=ERROR_ALREADY_EXISTS,"observer already exists");
  o.h=static_cast<uio1::Header*>(MapViewOfFile(o.map,FILE_MAP_READ|FILE_MAP_WRITE,0,0,uio1::mapping_bytes));check(o.h!=nullptr,"mapping view");
  auto&h=*o.h;memcpy(h.magic,"UIO1",4);h.version=1;h.bytes=uio1::mapping_bytes;h.record_size=uio1::record_bytes;h.pid=pid;h.tid=tid;h.start=expected;h.root=uint64_t(root);h.nonce=now()^uint64_t(GetCurrentProcessId());LARGE_INTEGER freq{};QueryPerformanceFrequency(&freq);h.frequency=uint64_t(freq.QuadPart);
  wchar_t path[32768]{};check(GetModuleFileNameW(nullptr,path,32768)>0,"helper path");std::wstring dllpath(path);dllpath.resize(dllpath.find_last_of(L"\\/")+1);dllpath+=L"uio1-hook.dll";
  o.dll=LoadLibraryExW(dllpath.c_str(),nullptr,LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_SYSTEM32);check(o.dll!=nullptr,"exact adjacent hook library");
  const int kinds[]={WH_GETMESSAGE,WH_CALLWNDPROC,WH_CALLWNDPROCRET};const char* symbols[]={"uio1_get","uio1_call","uio1_return"};
  for(unsigned i=0;i<3;++i){auto proc=reinterpret_cast<HOOKPROC>(GetProcAddress(o.dll,symbols[i]));check(proc!=nullptr,"hook export");o.hooks[i]=SetWindowsHookExW(kinds[i],proc,o.dll,tid);check(o.hooks[i]!=nullptr,"thread hook installation");}
  const auto end=GetTickCount64()+seconds*1000ULL;uint64_t next=0;
  while(GetTickCount64()<end&&!uio1::atom(h.stop).load()&&WaitForSingleObject(o.process,0)==WAIT_TIMEOUT&&WaitForSingleObject(o.thread,0)==WAIT_TIMEOUT){
    if(!IsWindow(root)||GetWindowThreadProcessId(root,nullptr)!=tid){uio1::atom(h.scope_errors).fetch_add(1);break;}
    const auto t=GetTickCount64();if(t>=next&&uio1::atom(h.ping).load()==uio1::atom(h.pong).load()){
      const auto id=uio1::atom(h.ping).fetch_add(1)+1;
      uio1::atom(h.ping_qpc).store(now(),std::memory_order_release);
      if(!PostThreadMessageW(tid,pulse,WPARAM(id),LPARAM(h.nonce)))uio1::atom(h.heartbeat_errors).fetch_add(1);
      next=t+200;
    }
    const auto request=uio1::atom(h.clock_request).load(std::memory_order_acquire);
    if(request!=uio1::atom(h.clock_response).load()){
      uio1::atom(h.clock_qpc).store(now(),std::memory_order_relaxed);uio1::atom(h.clock_response).store(request,std::memory_order_release);
    }
    Sleep(10); // helper only; never the editor or audio callback
  }
  return 0;
}
}
int wmain(int argc,wchar_t**argv){try{
  if(argc==3&&wcscmp(argv[1],L"census")==0){census(DWORD(wcstoul(argv[2],nullptr,0)));return 0;}
  if(argc==6&&wcscmp(argv[1],L"observe")==0)return observe(reinterpret_cast<HWND>(_wcstoui64(argv[2],nullptr,0)),_wcstoui64(argv[3],nullptr,0),unsigned(wcstoul(argv[4],nullptr,10)),argv[5]);
  return 64;
}catch(const std::exception&e){fprintf(stderr,"uio1: %s (%lu)\n",e.what(),GetLastError());return 1;}}
