// Source-owned UIR2 instrumentation only. No vendor, hook, subclass or input injection.
#include <windows.h>
#include <objbase.h>
#include <atomic>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <cassert>
namespace uir2 {
constexpr uint64_t capacity=262144, header_bytes=4096;
constexpr UINT pulse=WM_APP+0x731, calibration=WM_APP+0x732;
struct Header {
  uint64_t magic,version,bytes,pid,start,tid,root,child,frequency,ready,stop,armed,
      committed,dropped,finished,error,clock_request,clock_ack,clock_qpc,clock_tick,
      turns,down,up,returns,heartbeat,destroyed,calibration_ok,xid,child_xid;
};
struct Record {
  uint64_t commit,kind,qpc,end_qpc,turn,dispatch,hwnd,capture,focus,active;
  uint32_t message,message_time,tick,queue,flags,pointer_id,pointer_type,pointer_flags;
  int32_t x,y;
  uint64_t value;
};
static_assert(sizeof(Record)==128 && sizeof(Header)<header_bytes);
constexpr uint64_t bytes=header_bytes+capacity*sizeof(Record);
Header* h; Record* rows; uint64_t serial=0,current_dispatch=0,turn=0;
HWND root,child; bool self_test=false;
uint64_t qpc(){LARGE_INTEGER n{};QueryPerformanceCounter(&n);return uint64_t(n.QuadPart);}
uint64_t load(uint64_t& v){return std::atomic_ref<uint64_t>(v).load(std::memory_order_acquire);}
void put(uint64_t& v,uint64_t n){std::atomic_ref<uint64_t>(v).store(n,std::memory_order_release);}
bool selected(UINT m){return m==WM_LBUTTONDOWN||m==WM_LBUTTONUP||m==WM_MOUSEMOVE||m==WM_TOUCH||
 m==WM_POINTERDOWN||m==WM_POINTERUPDATE||m==WM_POINTERUP||m==WM_CAPTURECHANGED||
 m==WM_POINTERCAPTURECHANGED||m==WM_CANCELMODE||m==pulse||m==calibration;}
void append(Record r){
 if(!h)return;
 auto n=load(h->committed);if(n>=capacity){++h->dropped;return;}
 r.commit=n+1;rows[n]=r;put(h->committed,n+1);
}
Record base(uint64_t kind,HWND w=nullptr,UINT m=0){
 Record r{};r.kind=kind;r.qpc=qpc();r.turn=turn;r.dispatch=current_dispatch;
 r.hwnd=uint64_t(w);r.capture=uint64_t(GetCapture());r.focus=uint64_t(GetFocus());r.active=uint64_t(GetActiveWindow());r.message=m;
 r.tick=GetTickCount();return r;
}
// Compile-time import wrappers surround the *unchanged* production pump's actual
// User32 calls. Neither wrapper filters, retries, removes extra messages or alters
// arguments/results. Macro scope ends immediately after including vendor_view.h.
BOOL WINAPI observed_peek(LPMSG msg,HWND w,UINT first,UINT last,UINT flags){
 auto r=base(2);r.flags=flags;r.qpc=qpc();
 const auto ok=::PeekMessageW(msg,w,first,last,flags);r.end_qpc=qpc();r.value=ok!=FALSE;
 if(ok){r.hwnd=uint64_t(msg->hwnd);r.message=msg->message;r.message_time=msg->time;
   if(selected(msg->message)){r.x=msg->pt.x;r.y=msg->pt.y;}
   // Never retain arbitrary keyboard WPARAM/LPARAM or application payloads.
 }
 append(r);return ok;
}
LRESULT WINAPI observed_dispatch(const MSG* msg){
 const auto previous=current_dispatch;current_dispatch=++serial;
 auto r=base(3,msg->hwnd,msg->message);r.message_time=msg->time;
 const auto result=::DispatchMessageW(msg);r.end_qpc=qpc();append(r);
 current_dispatch=previous;return result;
}
}
#define PeekMessageW uir2::observed_peek
#define DispatchMessageW uir2::observed_dispatch
#include "vendor_view.h"
#undef DispatchMessageW
#undef PeekMessageW
using linux_vst_bridge::wf0::VendorView;
namespace uir2 {
LRESULT body(HWND w,UINT m,WPARAM wp,LPARAM lp){
 if(m==WM_LBUTTONDOWN){++h->down;SetCapture(w);return 0;}
 if(m==WM_LBUTTONUP){++h->up;if(GetCapture()==w)ReleaseCapture();return 0;}
 if(m==WM_MOUSEMOVE){if(wp&MK_LBUTTON)InvalidateRect(w,nullptr,FALSE);return 0;}
 if(m==WM_TIMER){if(!PostMessageW(child,pulse,0,0))h->error=1;return 0;}
 if(m==pulse){++h->heartbeat;return 0;}
 if(m==calibration){
   const auto t=DWORD(GetMessageTime());const auto before=DWORD(wp),after=DWORD(lp);
   if(DWORD(t-before)<=DWORD(after-before)+64)++h->calibration_ok;else h->error=2;
   return 0;
 }
 if(m==WM_CLOSE){put(h->stop,1);return 0;}
 if(m==WM_PAINT){PAINTSTRUCT p{};HDC dc=BeginPaint(w,&p);RECT r{};GetClientRect(w,&r);
   FillRect(dc,&r,reinterpret_cast<HBRUSH>(COLOR_WINDOW+1));SetBkMode(dc,TRANSPARENT);
   const wchar_t* text=load(h->armed)?L"UIR2 - ONE FINGER\nShort drag here, lift, then hands off":L"UIR2 touch fixture\nPlease wait for the operator instruction";
   DrawTextW(dc,text,-1,&r,DT_CENTER|DT_VCENTER|DT_WORDBREAK);EndPaint(w,&p);return 0;
 }
 return DefWindowProcW(w,m,wp,lp);
}
LRESULT CALLBACK proc(HWND w,UINT m,WPARAM wp,LPARAM lp){
 const bool keep=selected(m);
 if(m==WM_TOUCH){
   const auto count=LOWORD(wp);TOUCHINPUT contacts[16]{};
   if(count>16){h->error=6;}
   else if(GetTouchInputInfo(reinterpret_cast<HTOUCHINPUT>(lp),count,contacts,sizeof(TOUCHINPUT))){
     for(unsigned i=0;i<count;++i){auto d=base(6,w,m);d.pointer_id=contacts[i].dwID;d.pointer_flags=contacts[i].dwFlags;
       d.x=contacts[i].x;d.y=contacts[i].y;d.message_time=contacts[i].dwTime;append(d);}
   }
   // DefWindowProc retains ownership of the actual touch handle.
 }
 if(keep){auto r=base(4,w,m);r.message_time=DWORD(GetMessageTime());
   if(m==WM_POINTERDOWN||m==WM_POINTERUP||m==WM_POINTERUPDATE){
     r.pointer_id=GET_POINTERID_WPARAM(wp);POINTER_INFO info{};
     if(GetPointerInfo(r.pointer_id,&info)){r.pointer_type=info.pointerType;r.pointer_flags=info.pointerFlags;
       r.x=info.ptPixelLocation.x;r.y=info.ptPixelLocation.y;r.flags=1;}
   }else if(m==WM_MOUSEMOVE||m==WM_LBUTTONDOWN||m==WM_LBUTTONUP){r.flags=uint32_t(wp)&0xffff;
     POINT p{short(LOWORD(lp)),short(HIWORD(lp))};ClientToScreen(w,&p);r.x=p.x;r.y=p.y;}
   append(r);
 }
 const auto value=body(w,m,wp,lp);
 if(keep){append(base(5,w,m));++h->returns;}return value;
}
int run(int argc,wchar_t** argv){
 self_test=argc==2&&std::wstring(argv[1])==L"--self-test";
 if(argc!=2)return 2;
 if(FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED)))return 3;
 HANDLE file=INVALID_HANDLE_VALUE;
 if(!self_test)file=CreateFileW(argv[1],GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,nullptr);
 if(!self_test&&file==INVALID_HANDLE_VALUE)return 4;
 HANDLE map=CreateFileMappingW(file,nullptr,PAGE_READWRITE,0,DWORD(bytes),nullptr);if(!map)return 5;
 h=static_cast<Header*>(MapViewOfFile(map,FILE_MAP_ALL_ACCESS,0,0,bytes));if(!h)return 6;
 std::memset(h,0,bytes);rows=reinterpret_cast<Record*>(reinterpret_cast<uint8_t*>(h)+header_bytes);
 h->magic=0x32524955;h->version=1;h->bytes=bytes;h->pid=GetCurrentProcessId();h->tid=GetCurrentThreadId();
 FILETIME c{},e{},k{},u{};if(!GetProcessTimes(GetCurrentProcess(),&c,&e,&k,&u))return 7;
 h->start=(uint64_t(c.dwHighDateTime)<<32)|c.dwLowDateTime;LARGE_INTEGER f{};QueryPerformanceFrequency(&f);h->frequency=f.QuadPart;
 WNDCLASSW wc{};wc.lpfnWndProc=proc;wc.hInstance=GetModuleHandleW(nullptr);wc.lpszClassName=L"LVB-UIR2-source-owned";
 wc.hCursor=LoadCursorW(nullptr,IDC_ARROW);if(!RegisterClassW(&wc))return 8;
 root=CreateWindowW(wc.lpszClassName,L"UIR2 source-owned touch surface",WS_OVERLAPPEDWINDOW,120,100,900,560,nullptr,nullptr,wc.hInstance,nullptr);
 child=CreateWindowW(wc.lpszClassName,L"",WS_CHILD|WS_VISIBLE,0,0,860,500,root,nullptr,wc.hInstance,nullptr);
 if(!root||!child)return 9;h->root=uint64_t(root);h->child=uint64_t(child);
 // Default Win32 touch/pointer posture: no registration/injection forcing a route.
 ShowWindow(root,SW_SHOW);UpdateWindow(root);SetForegroundWindow(root);SetFocus(child);
 if(!SetTimer(child,1,100,nullptr))return 10;
 if(self_test){PostQuitMessage(73);if(VendorView::pump())return 11;MSG msg{};
   if(!::PeekMessageW(&msg,nullptr,WM_QUIT,WM_QUIT,PM_REMOVE)||msg.wParam!=73)return 12;}
 h->xid=uint64_t(GetPropW(root,L"__wine_x11_whole_window"));
 h->child_xid=uint64_t(GetPropW(child,L"__wine_x11_whole_window"));
 put(h->ready,1);const auto deadline=GetTickCount64()+(self_test?500:160000);uint64_t old_armed=0;
 while(!load(h->stop)&&GetTickCount64()<deadline){
   ++turn;h->turns=turn;
   auto r=base(1);r.queue=GetQueueStatus(QS_ALLINPUT);r.end_qpc=qpc();append(r);
   if(load(h->clock_request)!=load(h->clock_ack)){
     const auto request=load(h->clock_request);h->clock_qpc=qpc();h->clock_tick=GetTickCount();put(h->clock_ack,request);
   }
   if(turn%25==1){const auto before=GetTickCount();if(!PostMessageW(child,calibration,before,GetTickCount()))h->error=3;}
   if(load(h->armed)!=old_armed){old_armed=load(h->armed);InvalidateRect(child,nullptr,TRUE);}
   if(!VendorView::pump()){h->error=4;break;}
   Sleep(4); // Existing UI-owner maximum service cadence, no artificial workload.
 }
 KillTimer(child,1);if(GetCapture()==child)ReleaseCapture();
 const bool destroyed=DestroyWindow(root)&&!IsWindow(root)&&!IsWindow(child);h->destroyed=destroyed;
 if(!destroyed)h->error=5;
 if(self_test){
   assert(h->calibration_ok>0&&h->heartbeat>0&&h->error==0&&h->dropped==0);
   bool matched=false;for(uint64_t i=0;i<h->committed;++i)if(rows[i].kind==4&&rows[i].message==calibration&&rows[i].dispatch){
     for(uint64_t j=i+1;j<h->committed;++j)if(rows[j].kind==3&&rows[j].dispatch==rows[i].dispatch&&rows[j].qpc<=rows[i].qpc)matched=true;
   }assert(matched);std::printf("UIR2: actual pump removal/dispatch/procedure, clock sentinel, heartbeat, bounded clean retirement passed\n");
 }
 const auto error=h->error;put(h->finished,1);UnmapViewOfFile(h);h=nullptr;CloseHandle(map);if(file!=INVALID_HANDLE_VALUE)CloseHandle(file);
 CoUninitialize();return error?20:0;
}
}
int wmain(int argc,wchar_t** argv){return uir2::run(argc,argv);}
