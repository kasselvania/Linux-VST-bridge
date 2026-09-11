// Parent/child test through the production SDK processing owner, Collector,
// mapped custody writer and VendorView persistent-failure containment.
#include "result_status.h"
#include "offline_processing.h"
#include "component_instance_session.h"
#include "vendor_view.h"
#include "linux_vst_bridge/wf0_probe/events.h"
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "public.sdk/source/common/pluginview.h"
#include <filesystem>
#include <cassert>
#include <atomic>
using namespace Steinberg;using namespace Steinberg::Vst;using namespace linux_vst_bridge;using namespace linux_vst_bridge::wf0;
struct Plugin:AudioEffect {
 bool reject=false,hang_stop=false;std::atomic<bool> processed{false};
 tresult PLUGIN_API initialize(FUnknown* h)override{auto r=AudioEffect::initialize(h);addAudioInput(u"In",SpeakerArr::kStereo);addAudioOutput(u"Out",SpeakerArr::kStereo);return r;}
 tresult PLUGIN_API process(ProcessData& d)override{
  if(reject){int32 q=0,p=0;auto* out=d.outputParameterChanges->addParameterData(42,q);assert(out);assert(out->addPoint(0,2.,p)==kResultFalse);assert(out->addPoint(-1,.5,p)==kResultFalse);}
  for(int ch=0;ch<2;++ch)for(int i=0;i<d.numSamples;++i)d.outputs[0].channelBuffers32[ch][i]=0.f;
  processed=true;return kResultOk;
 }
 tresult PLUGIN_API setProcessing(TBool on)override{if(!on&&hang_stop)Sleep(INFINITE);return kResultOk;}
};
struct View:CPluginView {
 bool refuse=false;
 explicit View(bool r):refuse(r){setRect({0,0,64,64});}
 tresult PLUGIN_API isPlatformTypeSupported(FIDString s)override{return s&&!strcmp(s,kPlatformTypeHWND)?kResultOk:kResultFalse;}
 tresult PLUGIN_API attached(void* p,FIDString)override{systemWindow=p;return kResultOk;}
 tresult PLUGIN_API removed()override{if(refuse)return kResultFalse;systemWindow=nullptr;return kResultOk;}
};
struct Controller:EditController {bool refuse=false;IPlugView* PLUGIN_API createView(FIDString)override{return new View(refuse);}};
BOOL WINAPI fail_destroy(HWND){SetLastError(ERROR_ACCESS_DENIED);return FALSE;}
struct External:ExternalProcessing {
 ResultStatus status;Plugin& plugin;VendorView* editor;int mode;bool supplied=false;
 External(const std::wstring& path,const std::array<uint8_t,16>& sid,Plugin& p,VendorView* v,int m):status(path,sid),plugin(p),editor(v),mode(m){}
 ResultStatus* result_status()override{return &status;}
 bool commercial()const override{return true;}bool returned_results()const override{return true;}
 bool stateful()const override{return mode>=2;}
 void service_owner()override{
  if(plugin.processed){ // worker cannot finish: persistent stop seam above
   // For rejecting cases wait for custody through the production writer, not
   // merely process entry. The first-write law makes this duplicate harmless.
   if(plugin.reject&&!status.published())return;
   assert(editor&&!editor->close());delete editor;std::abort();
  }
 }
 void ready()override{}
 bool next(ExternalBlock& b,float*,float*)override{if(supplied)return false;supplied=true;b.frames=16;b.generation=17;b.epoch=3;b.sequence=91;b.position=1440;b.silence=3;b.gain_present=false;return true;}
 void done(const float*,const float*,uint64_t,uint64_t,const ap10_results_t*)override{assert(!plugin.reject);}
};
uint64_t word(uint8_t* p,size_t at){return uint64_t(InterlockedCompareExchange64(reinterpret_cast<volatile LONG64*>(p+at),0,0));}
int wmain(int argc,wchar_t** argv){
 std::array<uint8_t,16> sid{};sid.fill(18);
 if(argc==3){
  SetErrorMode(SEM_FAILCRITICALERRORS|SEM_NOGPFAULTERRORBOX);std::set_terminate([]{ExitProcess(86);});
  int mode=_wtoi(argv[2]);Plugin plugin;HostApplication host;assert(plugin.initialize(&host)==kResultOk);plugin.reject=mode==1||mode==2||mode==4;plugin.hang_stop=mode>=2;
  Controller controller;controller.refuse=mode==4;VendorView* view=nullptr;
  if(mode>=2){view=new VendorView;assert(view->open(controller));if(mode!=4)view->destruction(fail_destroy);}
  External external(argv[1],sid,plugin,view,mode);EventWriter events(1048576);HostCallbackSink callbacks(&events,GetCurrentThreadId());
  auto result=run_offline_processing(plugin,plugin,callbacks,events,&external);assert(result.quiescent&&result.success==!plugin.reject);
  AP10Results::RejectionRecord later{};later.reason=AP10Results::Rejection::PointCapacity;
  if(plugin.reject)external.status.publish(later,99,99,99,99,99,99,99);
  return 0;
 }
 auto root=std::filesystem::temp_directory_path()/(L"ap18-results-"+std::to_wstring(GetCurrentProcessId()));std::filesystem::create_directory(root);
 wchar_t exe[32768]{};assert(GetModuleFileNameW(nullptr,exe,32768));
 for(int mode=0;mode<5;++mode){
  auto dir=root/std::to_wstring(mode);std::filesystem::create_directory(dir);
  HANDLE f=CreateFileW((dir/L"ap18.results").c_str(),GENERIC_READ|GENERIC_WRITE,FILE_SHARE_READ|FILE_SHARE_WRITE,nullptr,CREATE_NEW,0,nullptr);assert(f!=INVALID_HANDLE_VALUE);
  HANDLE map=CreateFileMappingW(f,nullptr,PAGE_READWRITE,0,1024,nullptr);assert(map);auto* p=static_cast<uint8_t*>(MapViewOfFile(map,FILE_MAP_ALL_ACCESS,0,0,1024));assert(p);memset(p,0,1024);
  memcpy(p,"LVRS",4);ap1::put(p+4,1,4);ap1::put(p+8,1024,4);ap1::put(p+12,31,4);memcpy(p+16,sid.data(),16);
  std::wstring command=L"\""+std::wstring(exe)+L"\" \""+dir.wstring()+L"\" "+std::to_wstring(mode);STARTUPINFOW si{};si.cb=sizeof(si);PROCESS_INFORMATION child{};
  assert(CreateProcessW(exe,command.data(),nullptr,nullptr,FALSE,CREATE_NO_WINDOW,nullptr,nullptr,&si,&child));
  auto wait=WaitForSingleObject(child.hProcess,10000);if(wait!=WAIT_OBJECT_0)TerminateProcess(child.hProcess,99);assert(wait==WAIT_OBJECT_0);
  DWORD code=0;assert(GetExitCodeProcess(child.hProcess,&code));assert(code==DWORD(mode>=2?86:0));
  bool rejected=mode==1||mode==2||mode==4;assert(word(p,64)==uint64_t(rejected));
  if(rejected){assert(word(p,512)==17&&word(p,520)==3&&word(p,528)==91&&word(p,536)==1440&&word(p,544)==1);assert(word(p,512+7*8)==uint32_t(AP10Results::Rejection::ValueAboveOne));assert(word(p,512+23*8)==42&&word(p,512+25*8)==std::bit_cast<uint64_t>(2.));}
  CloseHandle(child.hThread);CloseHandle(child.hProcess);UnmapViewOfFile(p);CloseHandle(map);CloseHandle(f);
 }
 std::filesystem::remove_all(root);
}
