// Optional separate process. A provider hang cannot block the hook/helper UI
// heartbeat. Caller enforces a three-second containment deadline and the exact
// profile's accessibility posture. No account text, values or UIA names read.
#include <windows.h>
#include <uiautomation.h>
#include <cstdio>
#include <array>
#include <cstdlib>
uint64_t hash(BSTR s){uint64_t h=1469598103934665603ULL;if(s)for(unsigned i=0;i<SysStringLen(s)&&i<256;++i)h=(h^uint16_t(s[i]))*1099511628211ULL;return h;}
int wmain(int argc,wchar_t**argv){
  if(argc!=2)return 64;
  auto hwnd=reinterpret_cast<HWND>(_wcstoui64(argv[1],nullptr,0));if(!IsWindow(hwnd))return 1;
  if(FAILED(CoInitializeEx(nullptr,COINIT_MULTITHREADED)))return 2;
  IUIAutomation* api=nullptr;auto result=CoCreateInstance(CLSID_CUIAutomation,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(&api));
  if(FAILED(result)){printf("{\"uia_available\":false,\"hresult\":%ld}\n",result);CoUninitialize();return 0;}
  IUIAutomationElement* root=nullptr;IUIAutomationTreeWalker* walker=nullptr;
  if(FAILED(api->ElementFromHandle(hwnd,&root))||FAILED(api->get_RawViewWalker(&walker))){if(root)root->Release();api->Release();CoUninitialize();return 3;}
  struct Node{IUIAutomationElement* element=nullptr;unsigned depth=0,parent=0;};std::array<Node,128> nodes{};nodes[0]={root,0,0};unsigned queued=1,seen=0;bool overflow=false;
  for(;seen<queued&&seen<128;++seen){auto n=nodes[seen];auto*e=n.element;CONTROLTYPEID type=0;BOOL enabled=FALSE,offscreen=FALSE,focused=FALSE;RECT r{};BSTR id=nullptr;
    e->get_CurrentControlType(&type);e->get_CurrentIsEnabled(&enabled);e->get_CurrentIsOffscreen(&offscreen);e->get_CurrentHasKeyboardFocus(&focused);e->get_CurrentBoundingRectangle(&r);e->get_CurrentAutomationId(&id);
    unsigned patterns=0;const PATTERNID kinds[]={UIA_InvokePatternId,UIA_SelectionItemPatternId,UIA_ValuePatternId,UIA_RangeValuePatternId};
    for(unsigned i=0;i<4;++i){IUnknown*p=nullptr;if(SUCCEEDED(e->GetCurrentPattern(kinds[i],&p))&&p){patterns|=1u<<i;p->Release();}}
    printf("{\"element\":%u,\"parent\":%u,\"depth\":%u,\"control_type\":%d,\"automation_id_hash\":%llu,\"enabled\":%s,\"offscreen\":%s,\"focused\":%s,\"rect\":[%ld,%ld,%ld,%ld],\"patterns\":%u}\n",seen+1,n.parent,n.depth,type,hash(id),enabled?"true":"false",offscreen?"true":"false",focused?"true":"false",r.left,r.top,r.right,r.bottom,patterns);if(id)SysFreeString(id);
    if(n.depth<6){IUIAutomationElement*c=nullptr;walker->GetFirstChildElement(e,&c);while(c){if(queued==128){overflow=true;c->Release();break;}nodes[queued++]={c,n.depth+1,seen+1};IUIAutomationElement*next=nullptr;walker->GetNextSiblingElement(c,&next);c=next;}}else overflow=true;
    e->Release();
  }
  printf("{\"uia_available\":true,\"elements\":%u,\"capacity\":128,\"depth_limit\":6,\"bound_reached\":%s}\n",seen,overflow?"true":"false");
  walker->Release();api->Release();CoUninitialize();return 0;
}
