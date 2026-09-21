#define RPI0_NO_FACTORY 1
#include "instrument.cpp"
#include <atomic>
#include <cstdlib>
#include <iostream>
#include <new>
using namespace lvb::rpi0;
namespace {std::atomic<uint64_t> allocations{0};thread_local bool audit=false;void check(bool value,const char*message){if(!value){std::cerr<<"FAIL: "<<message<<'\n';std::exit(1);}}
void* allocate(size_t n){if(audit)allocations.fetch_add(1);if(auto*p=std::malloc(n))return p;throw std::bad_alloc();}}
void* operator new(size_t n){return allocate(n);}void* operator new[](size_t n){return allocate(n);}void operator delete(void*p)noexcept{std::free(p);}void operator delete[](void*p)noexcept{std::free(p);}void operator delete(void*p,size_t)noexcept{std::free(p);}void operator delete[](void*p,size_t)noexcept{std::free(p);}
int main(){
 check(processor_id!=controller_id&&audio_inputs==0&&audio_outputs==1&&event_inputs==1&&event_outputs==0,"class and bus identity");SynthCore synth;synth.reset(48000);float l=0,r=0;
 for(int i=0;i<32;++i){synth.sample(l,r);check(l==0&&r==0,"note timing before offset");}
 synth.note_on(2,60,.25f,7);audit=true;allocations=0;for(int i=0;i<128;++i)synth.sample(l,r);audit=false;check(allocations==0,"no process allocation");check(l==r&&l!=0,"stereo deterministic output");float quiet=std::abs(l);
 SynthCore loud;loud.reset(48000);loud.note_on(2,60,1.f,7);for(int i=0;i<128;++i)loud.sample(l,r);check(std::abs(l)>quiet,"velocity response");
 synth.sustain(2,true);synth.note_off(2,60,7);for(int i=0;i<4096;++i)synth.sample(l,r);check(synth.active()==1,"sustain holds");synth.sustain(2,false);for(int i=0;i<4096;++i)synth.sample(l,r);check(synth.active()==0,"release retires");
 synth.note_on(1,64,1,8);synth.note_on(2,67,1,9);synth.all_notes_off(1);for(int i=0;i<4096;++i)synth.sample(l,r);check(synth.active()==1,"all notes off preserves other channel");
 synth.gain(.25);auto state=synth.state();synth.gain(.8);check(synth.restore(state)&&synth.gain()==.25,"opaque state round trip");
 Controller controller;auto*first=controller.createView(ViewType::kEditor);check(first,"editor create");WNDCLASSW wc{};wc.lpfnWndProc=DefWindowProcW;wc.hInstance=GetModuleHandleW(nullptr);wc.lpszClassName=L"RPI0_TestParent";RegisterClassW(&wc);auto parent=CreateWindowW(wc.lpszClassName,L"",WS_OVERLAPPEDWINDOW,0,0,640,360,nullptr,nullptr,wc.hInstance,nullptr);check(parent,"editor parent");check(first->attached(parent,kPlatformTypeHWND)==kResultOk,"editor attach");auto child=GetWindow(HWND(parent),GW_CHILD);UpdateWindow(child);ValidateRect(child,nullptr);check(controller.setParamNormalized(gain_parameter,.125)==kResultOk&&GetUpdateRect(child,nullptr,FALSE),"state change invalidates live editor");UpdateWindow(child);auto before=controller.getParamNormalized(gain_parameter);SendMessageW(child,WM_LBUTTONDOWN,MK_LBUTTON,MAKELPARAM(400,110));SendMessageW(child,WM_LBUTTONUP,0,MAKELPARAM(400,110));check(controller.getParamNormalized(gain_parameter)!=before,"mouse changes parameter");check(first->removed()==kResultOk,"editor remove");first->release();auto*second=controller.createView(ViewType::kEditor);check(second&&second->attached(parent,kPlatformTypeHWND)==kResultOk,"editor reopen");second->removed();second->release();DestroyWindow(parent);
 std::cout<<"RPI0 Windows fixture tests: 11 passed\n";return 0;
}
