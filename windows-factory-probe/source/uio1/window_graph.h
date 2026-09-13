#pragma once
// Development observer only. No vendor objects, strings or product ABI.
#include "record.h"
#include <windows.h>
#include <array>
namespace uio2 {
constexpr uint32_t capacity = 128;
struct Window {
  uint64_t hwnd{}, parent{}, owner{}, root{}, root_owner{}, xid{};
  uint32_t pid{}, tid{}, visible{}, enabled{}, minimized{}, dpi{}, style{}, reserved{};
  int32_t rect[4]{}, client[4]{}, work[4]{};
};
static_assert(sizeof(Window) == 128);
struct alignas(8) Slot {
  uint64_t commit{}, qpc{};
  uint32_t count{}, incomplete{};
  uint64_t reserved{};
  Window windows[capacity]{};
};
struct alignas(8) Graph {
  char magic[4]{}; uint32_t version{}, bytes{}, row_bytes{};
  uint64_t pid{}, tid{}, start{}, root{}, request{}, response{}, counter{};
  uint8_t reserved[4096-72]{};
  Slot slots[2]{};
};
static_assert(offsetof(Graph, slots) == 4096);
inline bool read_window(HWND w, Window& r) {
  DWORD pid{};r.hwnd=uint64_t(w);r.tid=GetWindowThreadProcessId(w,&pid);r.pid=pid;
  r.parent=uint64_t(GetAncestor(w,GA_PARENT));r.owner=uint64_t(GetWindow(w,GW_OWNER));
  r.root=uint64_t(GetAncestor(w,GA_ROOT));r.root_owner=uint64_t(GetAncestor(w,GA_ROOTOWNER));
  r.xid=uint64_t(GetPropW(w,L"__wine_x11_whole_window"));
  r.visible=IsWindowVisible(w)!=0;r.enabled=IsWindowEnabled(w)!=0;r.minimized=IsIconic(w)!=0;
  r.dpi=GetDpiForWindow(w);r.style=uint32_t(GetWindowLongPtrW(w,GWL_STYLE));
  RECT a{},b{};POINT p{};MONITORINFO m{sizeof(m)};
  if(!r.tid||!GetWindowRect(w,&a)||!GetClientRect(w,&b)||!ClientToScreen(w,&p)||
     !GetMonitorInfoW(MonitorFromWindow(w,MONITOR_DEFAULTTONEAREST),&m))return false;
  r.rect[0]=a.left;r.rect[1]=a.top;r.rect[2]=a.right;r.rect[3]=a.bottom;
  r.client[0]=p.x;r.client[1]=p.y;r.client[2]=p.x+b.right;r.client[3]=p.y+b.bottom;
  r.work[0]=m.rcWork.left;r.work[1]=m.rcWork.top;r.work[2]=m.rcWork.right;r.work[3]=m.rcWork.bottom;
  return IsWindow(w)&&GetWindowThreadProcessId(w,nullptr)==r.tid;
}
struct Census {DWORD pid;Slot& slot;};
inline BOOL CALLBACK child(HWND w,LPARAM p) {
  auto& c=*reinterpret_cast<Census*>(p);DWORD pid{};GetWindowThreadProcessId(w,&pid);
  if(pid!=c.pid)return TRUE;
  if(c.slot.count==capacity){c.slot.incomplete=1;return FALSE;}
  if(!read_window(w,c.slot.windows[c.slot.count])){c.slot.incomplete=1;return FALSE;}
  ++c.slot.count;return TRUE;
}
inline BOOL CALLBACK top(HWND w,LPARAM p) {
  auto& c=*reinterpret_cast<Census*>(p);DWORD pid{};GetWindowThreadProcessId(w,&pid);
  if(pid==c.pid){if(!child(w,p))return FALSE;EnumChildWindows(w,child,p);}
  return !c.slot.incomplete;
}
inline void publish(Graph& g) {
  Slot next{};LARGE_INTEGER q{};QueryPerformanceCounter(&q);next.qpc=uint64_t(q.QuadPart);
  Census c{DWORD(g.pid),next};EnumWindows(top,reinterpret_cast<LPARAM>(&c));
  DWORD pid{};if(GetWindowThreadProcessId(HWND(g.root),&pid)!=g.tid||pid!=g.pid)next.incomplete=1;
  const auto n=uio1::atom(g.counter).load()+1;auto& slot=g.slots[n%2];
  uio1::atom(slot.commit).store(0,std::memory_order_release);
  // The reader validates slot + global commit before and after its copy.
  std::memcpy(reinterpret_cast<uint8_t*>(&slot)+8,reinterpret_cast<uint8_t*>(&next)+8,sizeof(Slot)-8);
  uio1::atom(slot.commit).store(n,std::memory_order_release);
  uio1::atom(g.counter).store(n,std::memory_order_release);
}
}
