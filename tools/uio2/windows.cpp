// Real SDK view/frame and production VendorView/pump. No vendor module.
#include "vendor_view.h"
#include "uio1/window_graph.h"
#include "public.sdk/source/common/pluginview.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include <cstdio>
#include <cstring>
#include <stdexcept>
#include <vector>
using namespace Steinberg;
using namespace Steinberg::Vst;
using linux_vst_bridge::wf0::VendorView;
namespace {
void check(bool yes,const char* why){if(!yes)throw std::runtime_error(why);}
struct alignas(8) Status {uint64_t words[512]{};};
Status* status=nullptr;
void put(size_t at,uint64_t v){if(status)std::atomic_ref<uint64_t>(status->words[at/8]).store(v,std::memory_order_release);}
uint64_t get(size_t at){return status?std::atomic_ref<uint64_t>(status->words[at/8]).load(std::memory_order_acquire):0;}
void inc(size_t at){put(at,get(at)+1);}
uint64_t born(){FILETIME c{},e{},k{},u{};check(GetProcessTimes(GetCurrentProcess(),&c,&e,&k,&u)!=0,"process identity");return(uint64_t(c.dwHighDateTime)<<32)|c.dwLowDateTime;}
struct View final:CPluginView {
  HWND child=nullptr,popup=nullptr,lookalike=nullptr,foreign=nullptr;
  std::vector<HWND> surfaces;
  unsigned requests=0,sizes=0;int result=-1;
  View(){setRect({0,0,640,400});}
  static LRESULT CALLBACK proc(HWND w,UINT m,WPARAM a,LPARAM b){
    auto* v=reinterpret_cast<View*>(GetWindowLongPtrW(w,GWLP_USERDATA));
    if(m==WM_NCCREATE){v=static_cast<View*>(reinterpret_cast<CREATESTRUCTW*>(b)->lpCreateParams);SetWindowLongPtrW(w,GWLP_USERDATA,LONG_PTR(v));}
    if(v){
      if(m==WM_NCHITTEST&&(GetWindowLongPtrW(w,GWL_EXSTYLE)&WS_EX_TRANSPARENT))return HTTRANSPARENT;
      if(m==WM_KEYDOWN&&a==VK_ESCAPE){v->dismiss();return 0;}
      if(m==WM_LBUTTONDOWN){inc(120);return 0;}
      if(m==WM_LBUTTONUP){inc(128);
        const int x=short(LOWORD(b)),y=short(HIWORD(b));
        if(w==v->child&&x>=0&&x<120&&y>=0&&y<50)v->menu();
        else if(w==v->popup&&x>=0&&x<240&&y>=0&&y<80){if(get(208)==5){v->dismiss();put(208,2);v->menu();inc(224);}
          else {v->dismiss();v->request();}}
        return 0;
      }
      if(m==WM_PAINT){PAINTSTRUCT p{};HDC d=BeginPaint(w,&p);RECT r{};GetClientRect(w,&r);
        HBRUSH brush=CreateSolidBrush(w==v->child?(v->requests?RGB(22,80,140):RGB(40,60,70)):RGB(180,180,180));
        FillRect(d,&r,brush);DeleteObject(brush);SetTextColor(d,RGB(255,255,255));SetBkMode(d,TRANSPARENT);
        DrawTextW(d,w==v->child?L"Menu":L"Resize Window",-1,&r,DT_LEFT|DT_TOP);EndPaint(w,&p);inc(112);return 0;}
    }
    return DefWindowProcW(w,m,a,b);
  }
  tresult PLUGIN_API isPlatformTypeSupported(FIDString s) override {return s&&!strcmp(s,kPlatformTypeHWND)?kResultOk:kResultFalse;}
  tresult PLUGIN_API canResize() override{return kResultTrue;}
  tresult PLUGIN_API attached(void* p,FIDString t) override {
    if(isPlatformTypeSupported(t)!=kResultOk)return kResultFalse;
    systemWindow=p;WNDCLASSW c{};c.lpfnWndProc=proc;c.hInstance=GetModuleHandleW(nullptr);c.lpszClassName=L"UIO2Fixture";c.hCursor=LoadCursorW(nullptr,IDC_ARROW);
    if(!RegisterClassW(&c)&&GetLastError()!=ERROR_CLASS_ALREADY_EXISTS)return kResultFalse;
    child=CreateWindowExW(0,c.lpszClassName,L"",WS_CHILD|WS_VISIBLE,0,0,640,400,HWND(p),nullptr,c.hInstance,this);
    foreign=CreateWindowExW(0,c.lpszClassName,L"",WS_OVERLAPPEDWINDOW,900,500,260,130,nullptr,nullptr,c.hInstance,this);
    lookalike=CreateWindowExW(WS_EX_TOOLWINDOW,c.lpszClassName,L"",WS_POPUP,920,520,240,80,foreign,nullptr,c.hInstance,this);
    ShowWindow(foreign,SW_SHOWNOACTIVATE);ShowWindow(lookalike,SW_SHOWNOACTIVATE);
    put(40,uint64_t(p));put(48,uint64_t(child));put(64,uint64_t(lookalike));return child?kResultOk:kResultFalse;
  }
  void dismiss(){
    for(auto w:surfaces)if(IsWindow(w))check(DestroyWindow(w)!=0,"surface cleanup");surfaces.clear();
    if(popup&&IsWindow(popup))check(DestroyWindow(popup)!=0,"popup cleanup");popup=nullptr;put(56,0);
  }
  HWND surface(int x,int y,int width,int height,bool transparent){
    const DWORD ex=WS_EX_TOOLWINDOW|(transparent?(WS_EX_LAYERED|WS_EX_TRANSPARENT|WS_EX_NOACTIVATE):0);
    auto w=CreateWindowExW(ex,L"UIO2Fixture",L"",WS_POPUP,x,y,width,height,nullptr,nullptr,GetModuleHandleW(nullptr),this);
    check(w!=nullptr,"ownerless surface creation");
    if(transparent)check(SetLayeredWindowAttributes(w,0,160,LWA_ALPHA)!=0,"shadow alpha");
    ShowWindow(w,SW_SHOWNOACTIVATE);surfaces.push_back(w);return w;
  }
  void menu(){check(!popup,"single generated menu");RECT r{};GetWindowRect(child,&r);
    if(get(208)>=2){
      popup=surface(r.left+10,r.top+50,240,80,false);
      if(get(208)==3||get(208)==4){
        surface(r.left+10,r.top+42,240,8,true);surface(r.left+10,r.top+130,240,8,true);
        surface(r.left+2,r.top+42,8,96,true);surface(r.left+250,r.top+42,8,96,true);
      }
      if(get(208)==4)surface(r.left+450,r.top+50,180,80,false);
      put(56,uint64_t(popup));return;
    }
    popup=CreateWindowExW(WS_EX_TOOLWINDOW,L"UIO2Fixture",L"",WS_POPUP,r.left+10,r.top+50,240,80,HWND(systemWindow),nullptr,GetModuleHandleW(nullptr),this);
    check(popup!=nullptr,"popup creation");ShowWindow(popup,SW_SHOWNOACTIVATE);put(56,uint64_t(popup));
  }
  void request(){++requests;put(88,requests);put(136,800);put(144,500);ViewRect r{0,0,800,500};
    result=plugFrame->resizeView(this,&r);put(96,uint64_t(int64_t(result)));measure();}
  tresult PLUGIN_API onSize(ViewRect* r) override {if(!r||!child)return kResultFalse;++sizes;put(104,sizes);setRect(*r);
    if(!MoveWindow(child,0,0,r->getWidth(),r->getHeight(),TRUE))return kResultFalse;InvalidateRect(child,nullptr,FALSE);return kResultOk;}
  void measure(){RECT p{},c{};GetClientRect(HWND(systemWindow),&p);GetClientRect(child,&c);put(152,p.right);put(160,p.bottom);put(168,c.right);put(176,c.bottom);}
  tresult PLUGIN_API removed() override {
    dismiss();
    for(auto w:{lookalike,foreign,child})if(w)check(DestroyWindow(w)!=0,"fixture child cleanup");
    popup=lookalike=foreign=child=nullptr;systemWindow=nullptr;return kResultOk;}
};
struct Controller final:EditController {View* view=nullptr;IPlugView* PLUGIN_API createView(FIDString t) override {if(strcmp(t,ViewType::kEditor))return nullptr;view=new View;return view;}};
}
int wmain(int argc,wchar_t** argv){try{
  check(argc==2,"status path or --self-test required");Status local{};HANDLE file=INVALID_HANDLE_VALUE,map=nullptr;
  bool self=wcscmp(argv[1],L"--self-test")==0;
  if(self)status=&local;else{
    file=CreateFileW(argv[1],GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,nullptr);check(file!=INVALID_HANDLE_VALUE,"new status");
    LARGE_INTEGER size{};size.QuadPart=sizeof(Status);check(SetFilePointerEx(file,size,nullptr,FILE_BEGIN)&&SetEndOfFile(file),"extent");
    map=CreateFileMappingW(file,nullptr,PAGE_READWRITE,0,sizeof(Status),nullptr);check(map!=nullptr,"map");status=static_cast<Status*>(MapViewOfFile(map,FILE_MAP_ALL_ACCESS,0,0,sizeof(Status)));check(status!=nullptr,"view");}
  put(0,0x324f4955);put(8,1);put(16,GetCurrentProcessId());put(24,born());put(32,GetCurrentThreadId());
  Controller controller;VendorView host;host.diagnostic([](void*,uint32_t s){put(200,s);},nullptr);
  check(host.open(controller),"production open");controller.view->measure();put(72,1);
  if(self){
    controller.view->menu();uio2::Window w{},f{};check(uio2::read_window(controller.view->popup,w),"popup census");check(uio2::read_window(controller.view->lookalike,f),"foreign census");
    check(w.owner==get(40)&&w.root_owner==get(40)&&w.tid==get(32)&&f.root_owner!=get(40),"exact owner chain");
    controller.view->request();check(get(88)==1&&get(96)==0&&get(104)==1&&get(152)==800&&get(160)==500&&get(168)==800&&get(176)==500&&get(200)==16,"production resize and child delivery");
  }else{const auto end=GetTickCount64()+120000;while(GetTickCount64()<end&&get(80)!=9){host.pump();if(get(216)){controller.view->dismiss();put(216,0);}
      controller.view->measure();Sleep(5);}check(get(80)==9,"fixture deadline");}
  check(host.close(),"production close");put(184,1);
  if(!self){UnmapViewOfFile(status);CloseHandle(map);CloseHandle(file);}status=nullptr;puts("UIO2 production resize fixture complete");return 0;
}catch(const std::exception& e){put(192,1);fprintf(stderr,"UIO2 fixture: %s\n",e.what());return 1;}}
