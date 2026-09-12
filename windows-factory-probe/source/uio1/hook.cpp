#include "record.h"
#include <windows.h>
#include <cstdio>
namespace {
uio1::Header* header=nullptr; HANDLE mapping=nullptr;
thread_local bool inside=false;
constexpr UINT pulse=WM_APP+0x541;
uint64_t now(){LARGE_INTEGER n{};QueryPerformanceCounter(&n);return uint64_t(n.QuadPart);}
bool connect() {
  if(header)return true;
  wchar_t name[96]{};
  swprintf_s(name,L"Local\\LVBUIO1-%lu-%lu",GetCurrentProcessId(),GetCurrentThreadId());
  mapping=OpenFileMappingW(FILE_MAP_READ|FILE_MAP_WRITE,FALSE,name);
  if(!mapping)return false;
  auto p=static_cast<uio1::Header*>(MapViewOfFile(mapping,FILE_MAP_READ|FILE_MAP_WRITE,0,0,uio1::mapping_bytes));
  if(!p){CloseHandle(mapping);mapping=nullptr;return false;}
  FILETIME c{},e{},k{},u{};GetProcessTimes(GetCurrentProcess(),&c,&e,&k,&u);
  const uint64_t start=(uint64_t(c.dwHighDateTime)<<32)|c.dwLowDateTime;
  if(std::memcmp(p->magic,"UIO1",4)||p->version!=1||p->bytes!=uio1::mapping_bytes||
    p->pid!=GetCurrentProcessId()||p->tid!=GetCurrentThreadId()||p->start!=start){
    UnmapViewOfFile(p);CloseHandle(mapping);mapping=nullptr;return false;
  }
  header=p;uio1::atom(p->ready).store(1,std::memory_order_release);return true;
}
void observe(HWND hwnd,UINT msg,WPARAM wp,LPARAM lp,uint32_t source,int64_t result=0,uint32_t message_time=0) noexcept {
  if(inside)return; inside=true;
  // Initial mapping occurs only on the helper's private thread-message. No
  // vendor message can select a path or initialize an observer.
  if(!header && !(source==1&&msg==pulse&&connect())){inside=false;return;}
  auto& h=*header;
  if(uio1::atom(h.stop).load(std::memory_order_acquire)){
    uio1::atom(h.detached).store(1,std::memory_order_release);
    UnmapViewOfFile(header);header=nullptr;CloseHandle(mapping);mapping=nullptr;
    inside=false;return;
  }
  const auto began=now();
  const bool heartbeat=source==1&&msg==pulse&&uint64_t(lp)==h.nonce;
  // Retain this request's start before acknowledging it. Early acknowledgement
  // would allow the helper to replace ping_qpc while this callback is recording
  // the old pulse, incorrectly turning a long stall into near-zero latency.
  const auto sent=heartbeat?uio1::atom(h.ping_qpc).load(std::memory_order_acquire):0;
  DWORD pid=0;
  bool same=hwnd && GetWindowThreadProcessId(hwnd,&pid)==h.tid && pid==h.pid;
  auto root=reinterpret_cast<HWND>(h.root);
  bool scope=same&&(hwnd==root||IsChild(root,hwnd)||GetAncestor(hwnd,GA_ROOTOWNER)==root);
  const auto action=uio1::atom(h.action).load(std::memory_order_acquire);
  if(heartbeat || (scope&&uio1::selected(msg)&&action)){
    uio1::Record r{};r.qpc=began;r.action=action;r.hwnd=uint64_t(hwnd);r.message=msg;r.source=heartbeat?4:source;
    r.focus=uint64_t(GetFocus());r.active=uint64_t(GetActiveWindow());r.capture=uint64_t(GetCapture());
    if(msg>=WM_MOUSEMOVE&&msg<=WM_MOUSEWHEEL){
      POINT p{short(LOWORD(lp)),short(HIWORD(lp))};
      if(msg==WM_MOUSEWHEEL){r.screen_x=p.x;r.screen_y=p.y;ScreenToClient(hwnd,&p);r.x=p.x;r.y=p.y;}
      else{r.x=p.x;r.y=p.y;ClientToScreen(hwnd,&p);r.screen_x=p.x;r.screen_y=p.y;}
      r.buttons=uint32_t(wp)&0xffff;
    }
    if(msg==WM_KEYDOWN||msg==WM_KEYUP||msg==WM_SYSKEYDOWN||msg==WM_SYSKEYUP)r.key_class=uio1::key_class(wp);
    if(msg==WM_NCHITTEST&&source==3)r.result=result;
    if(heartbeat)r.result=int64_t(began-sent);
    r.message_time=message_time;r.cost_ticks=now()-began;
    uio1::append(h,reinterpret_cast<uio1::Record*>(reinterpret_cast<uint8_t*>(header)+uio1::header_bytes),r);
  }else uio1::atom(h.filtered).fetch_add(1,std::memory_order_relaxed);
  const auto cost=now()-began;
  uio1::atom(h.hook_calls).fetch_add(1,std::memory_order_relaxed);
  uio1::atom(h.hook_ticks).fetch_add(cost,std::memory_order_relaxed);
  if(cost>uio1::atom(h.max_hook_ticks).load())uio1::atom(h.max_hook_ticks).store(cost);
  if(heartbeat){
    uio1::atom(h.pong_qpc).store(began,std::memory_order_relaxed);
    uio1::atom(h.pong).store(uint64_t(wp),std::memory_order_release);
  }
  inside=false;
}
}
extern "C" __declspec(dllexport) LRESULT CALLBACK uio1_get(int c,WPARAM w,LPARAM l){
  if(c>=0&&w==PM_REMOVE){auto&m=*reinterpret_cast<MSG*>(l);observe(m.hwnd,m.message,m.wParam,m.lParam,1,0,m.time);}
  return CallNextHookEx(nullptr,c,w,l);
}
extern "C" __declspec(dllexport) LRESULT CALLBACK uio1_call(int c,WPARAM w,LPARAM l){
  if(c>=0){auto&m=*reinterpret_cast<CWPSTRUCT*>(l);observe(m.hwnd,m.message,m.wParam,m.lParam,2);}
  return CallNextHookEx(nullptr,c,w,l);
}
extern "C" __declspec(dllexport) LRESULT CALLBACK uio1_return(int c,WPARAM w,LPARAM l){
  if(c>=0){auto&m=*reinterpret_cast<CWPRETSTRUCT*>(l);observe(m.hwnd,m.message,m.wParam,m.lParam,3,m.lResult);}
  return CallNextHookEx(nullptr,c,w,l);
}
BOOL WINAPI DllMain(HINSTANCE,DWORD reason,LPVOID){
  if(reason==DLL_PROCESS_DETACH){if(header)UnmapViewOfFile(header);if(mapping)CloseHandle(mapping);}
  return TRUE;
}
