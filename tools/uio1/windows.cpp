#include "uio1/record.h"
#include <windows.h>
#include <cassert>
#include <atomic>
#include <thread>
#include <vector>
#include <string>
#include <cstdio>
std::atomic<HWND> parent{},child{},foreign{};
std::atomic<unsigned> received{0},delays{0};
LRESULT CALLBACK proc(HWND w,UINT m,WPARAM a,LPARAM b){
  if(m==WM_LBUTTONDOWN){++received;SetCapture(w);return 0;}
  if(m==WM_LBUTTONUP){++received;ReleaseCapture();return 0;}
  if(m==WM_APP+7){Sleep(500);++delays;return 0;}
  return DefWindowProcW(w,m,a,b);
}
uint64_t born(){FILETIME c{},e{},k{},u{};assert(GetProcessTimes(GetCurrentProcess(),&c,&e,&k,&u));return(uint64_t(c.dwHighDateTime)<<32)|c.dwLowDateTime;}
int main(){
  auto slots=std::vector<uio1::Record>(uio1::capacity);uio1::Header h{};
  for(unsigned i=0;i<uio1::capacity;++i){uio1::Record r{};r.action=i;assert(uio1::append(h,slots.data(),r));}
  assert(!uio1::append(h,slots.data(),{}));assert(h.dropped==1&&slots[0].commit==1&&slots.back().commit==uio1::capacity);
  assert(uio1::key_class('A')==4&&uio1::key_class('Z')==4&&!uio1::selected(WM_CHAR));
  WNDCLASSW wc{};wc.hInstance=GetModuleHandleW(nullptr);wc.lpfnWndProc=proc;wc.lpszClassName=L"UIO1Fixture";assert(RegisterClassW(&wc));
  std::thread ui([]{
    auto p=CreateWindowW(L"UIO1Fixture",L"",WS_OVERLAPPEDWINDOW,0,0,320,240,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);assert(p);
    child=CreateWindowW(L"UIO1Fixture",L"",WS_CHILD|WS_VISIBLE,0,0,160,120,p,nullptr,GetModuleHandleW(nullptr),nullptr);
    foreign=CreateWindowW(L"UIO1Fixture",L"",WS_OVERLAPPEDWINDOW,0,0,40,40,nullptr,nullptr,GetModuleHandleW(nullptr),nullptr);
    parent=p;MSG m{};while(GetMessageW(&m,nullptr,0,0)>0){TranslateMessage(&m);DispatchMessageW(&m);}DestroyWindow(foreign);DestroyWindow(p);
  });
  for(unsigned n=0;n<200&&!parent;++n)Sleep(5);assert(parent&&child&&foreign);
  wchar_t own[32768]{};GetModuleFileNameW(nullptr,own,32768);std::wstring dir=own;dir.resize(dir.find_last_of(L"\\/")+1);
  wchar_t temp[MAX_PATH]{};GetTempPathW(MAX_PATH,temp);std::wstring out=std::wstring(temp)+L"uio1-fixture-"+std::to_wstring(GetCurrentProcessId())+L".bin";
  std::wstring cmd=L"\""+dir+L"uio1-observer.exe\" observe "+std::to_wstring(uint64_t(parent.load()))+L" "+std::to_wstring(born())+L" 8 \""+out+L"\"";
  STARTUPINFOW si{};si.cb=sizeof(si);PROCESS_INFORMATION pi{};assert(CreateProcessW(nullptr,cmd.data(),nullptr,nullptr,FALSE,0,nullptr,nullptr,&si,&pi));
  HANDLE f=INVALID_HANDLE_VALUE,map=nullptr;uio1::Header* shared=nullptr;
  for(unsigned n=0;n<400&&!shared;++n){f=CreateFileW(out.c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,OPEN_EXISTING,0,nullptr);if(f!=INVALID_HANDLE_VALUE){LARGE_INTEGER size{};if(GetFileSizeEx(f,&size)&&size.QuadPart==uio1::mapping_bytes){map=CreateFileMappingW(f,nullptr,PAGE_READWRITE,0,0,nullptr);if(map)shared=static_cast<uio1::Header*>(MapViewOfFile(map,FILE_MAP_ALL_ACCESS,0,0,uio1::mapping_bytes));}if(!shared){if(map)CloseHandle(map);CloseHandle(f);}}Sleep(5);}
  assert(shared);for(unsigned n=0;n<400&&!uio1::atom(shared->ready).load();++n)Sleep(5);assert(shared->ready);
  uio1::atom(shared->action).store(17);
  PostMessageW(child,WM_LBUTTONDOWN,MK_LBUTTON,MAKELPARAM(15,19));PostMessageW(child,WM_LBUTTONUP,0,MAKELPARAM(15,19));
  PostMessageW(foreign,WM_LBUTTONDOWN,MK_LBUTTON,MAKELPARAM(1,1));PostMessageW(foreign,WM_LBUTTONUP,0,MAKELPARAM(1,1));
  PostMessageW(child,WM_APP+7,0,0);for(unsigned n=0;n<300&&!delays;++n)Sleep(5);assert(delays);
  Sleep(300);uio1::atom(shared->stop).store(1);assert(WaitForSingleObject(pi.hProcess,3000)==WAIT_OBJECT_0);DWORD exit=99;assert(GetExitCodeProcess(pi.hProcess,&exit)&&exit==0&&shared->closed==1&&shared->unhook_errors==0);
  const auto count=uio1::atom(shared->committed).load();auto*r=reinterpret_cast<uio1::Record*>(reinterpret_cast<uint8_t*>(shared)+uio1::header_bytes);
  unsigned get=0,enter=0,leave=0;bool slow=false;
  for(uint64_t i=0;i<count;++i){assert(r[i].commit==i+1&&r[i].hwnd!=uint64_t(foreign.load()));if(r[i].message==WM_LBUTTONDOWN){assert(r[i].hwnd==uint64_t(child.load())&&r[i].x==15&&r[i].y==19&&r[i].action==17);get+=r[i].source==1;enter+=r[i].source==2;leave+=r[i].source==3;}if(r[i].source==4&&r[i].result>int64_t(shared->frequency/5))slow=true;}
  assert(get==1&&enter==1&&leave==1&&slow&&received==4);
  PostMessageW(child,WM_LBUTTONDOWN,0,0);Sleep(100);assert(received==5&&uio1::atom(shared->committed).load()==count);
  DWORD tid=GetWindowThreadProcessId(parent,nullptr);PostThreadMessageW(tid,WM_QUIT,0,0);ui.join();
  UnmapViewOfFile(shared);CloseHandle(map);CloseHandle(f);CloseHandle(pi.hThread);CloseHandle(pi.hProcess);assert(DeleteFileW(out.c_str()));
  puts("UIO1: exact child receipt, dispatch, delayed heartbeat, foreign refusal, bounded ring, unhook and owner survival passed");
}
