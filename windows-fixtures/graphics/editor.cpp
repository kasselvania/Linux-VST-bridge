// Independent reference editors. Nothing is installed or licensed by this fixture.
#include "public.sdk/source/vst/vstaudioeffect.h"
#include "public.sdk/source/vst/vsteditcontroller.h"
#include "public.sdk/source/common/pluginview.h"
#include "public.sdk/source/main/pluginfactory.h"
#include "pluginterfaces/base/ibstream.h"
#include <windows.h>
#include <cstdio>
#include <cstring>
#if GRAPHICS_FIXTURE == 1
#include <d3d11.h>
#include <wrl/client.h>
#elif GRAPHICS_FIXTURE == 2
#include <GL/gl.h>
#endif
using namespace Steinberg;using namespace Steinberg::Vst;
const FUID processor_id(0x4C564247,0x52415048,0x46585452,GRAPHICS_FIXTURE*16+1);
const FUID controller_id(0x4C564247,0x52415048,0x46585452,GRAPHICS_FIXTURE*16+2);
class View final:public CPluginView {
public:
    View():CPluginView(){rect={0,0,128,128};}
    tresult PLUGIN_API isPlatformTypeSupported(FIDString type)override{return type&&std::strcmp(type,kPlatformTypeHWND)==0?kResultOk:kResultFalse;}
    tresult PLUGIN_API attached(void* parent,FIDString type)override{
        if(!parent||isPlatformTypeSupported(type)!=kResultOk)return kResultFalse;
#if GRAPHICS_FIXTURE == 3
        return kResultFalse;
#elif GRAPHICS_FIXTURE == 1
        using Microsoft::WRL::ComPtr;
        ComPtr<ID3D11Device> device;ComPtr<ID3D11DeviceContext> context;
        if(FAILED(D3D11CreateDevice(nullptr,D3D_DRIVER_TYPE_WARP,nullptr,0,nullptr,0,D3D11_SDK_VERSION,&device,nullptr,&context)))return kResultFalse;
        D3D11_TEXTURE2D_DESC desc{};desc.Width=4;desc.Height=4;desc.MipLevels=1;desc.ArraySize=1;desc.SampleDesc.Count=1;
        desc.Format=DXGI_FORMAT_R8G8B8A8_UNORM;desc.BindFlags=D3D11_BIND_RENDER_TARGET;
        ComPtr<ID3D11Texture2D> texture,capture;ComPtr<ID3D11RenderTargetView> view;
        if(FAILED(device->CreateTexture2D(&desc,nullptr,&texture))||FAILED(device->CreateRenderTargetView(texture.Get(),nullptr,&view)))return kResultFalse;
        const float red[]={1,0,0,1};context->ClearRenderTargetView(view.Get(),red);
        desc.Usage=D3D11_USAGE_STAGING;desc.BindFlags=0;desc.CPUAccessFlags=D3D11_CPU_ACCESS_READ;
        if(FAILED(device->CreateTexture2D(&desc,nullptr,&capture)))return kResultFalse;
        context->CopyResource(capture.Get(),texture.Get());D3D11_MAPPED_SUBRESOURCE mapped{};
        if(FAILED(context->Map(capture.Get(),0,D3D11_MAP_READ,0,&mapped)))return kResultFalse;
        const auto* pixels=static_cast<const unsigned char*>(mapped.pData);
        const bool matched=pixels&&pixels[0]==255&&pixels[1]==0&&pixels[2]==0&&pixels[3]==255;
        context->Unmap(capture.Get(),0);if(!matched)return kResultFalse;
        std::fputs("reference D3D11 pixels matched\n",stderr);
#elif GRAPHICS_FIXTURE == 2
        HDC dc=GetDC(HWND(parent));PIXELFORMATDESCRIPTOR p{};p.nSize=sizeof(p);p.nVersion=1;p.dwFlags=PFD_DRAW_TO_WINDOW|PFD_SUPPORT_OPENGL|PFD_DOUBLEBUFFER;p.iPixelType=PFD_TYPE_RGBA;p.cColorBits=32;
        int format=ChoosePixelFormat(dc,&p);bool matched=false;HGLRC rc=nullptr;
        if(format&&SetPixelFormat(dc,format,&p)&&(rc=wglCreateContext(dc))&&wglMakeCurrent(dc,rc)){
            glDrawBuffer(GL_BACK);glReadBuffer(GL_BACK);glClearColor(0,1,0,1);glClear(GL_COLOR_BUFFER_BIT);unsigned char pixels[4]{};glReadPixels(0,0,1,1,GL_RGB,GL_UNSIGNED_BYTE,pixels);
            matched=glGetError()==GL_NO_ERROR&&pixels[0]==0&&pixels[1]==255&&pixels[2]==0;wglMakeCurrent(nullptr,nullptr);
        }
        if(rc)wglDeleteContext(rc);ReleaseDC(HWND(parent),dc);if(!matched)return kResultFalse;
        std::fputs("reference OpenGL pixels matched\n",stderr);
#endif
        systemWindow=parent;return kResultOk;
    }
    tresult PLUGIN_API removed()override{systemWindow=nullptr;return kResultOk;}
};
class Processor final:public AudioEffect {
public:
    Processor(){setControllerClass(controller_id);}
    static FUnknown* create(void*){return static_cast<IAudioProcessor*>(new Processor);}
    tresult PLUGIN_API initialize(FUnknown* host)override{
        auto r=AudioEffect::initialize(host);if(r!=kResultOk)return r;
        addAudioInput(STR16("Input"),SpeakerArr::kStereo);addAudioOutput(STR16("Output"),SpeakerArr::kStereo);return kResultOk;
    }
    tresult PLUGIN_API canProcessSampleSize(int32 size)override{return size==kSample32?kResultOk:kResultFalse;}
    tresult PLUGIN_API getState(IBStream* stream)override{char marker='G';int32 count=0;return stream&&stream->write(&marker,1,&count)==kResultOk&&count==1?kResultOk:kResultFalse;}
};
class Controller final:public EditController {
public:
    static FUnknown* create(void*){return static_cast<IEditController*>(new Controller);}
    tresult PLUGIN_API setComponentState(IBStream*)override{return kResultOk;}
    IPlugView* PLUGIN_API createView(FIDString name)override{
        if(GRAPHICS_FIXTURE==4)return nullptr;
        return name&&std::strcmp(name,ViewType::kEditor)==0?new View:nullptr;
    }
};
BEGIN_FACTORY_DEF("LVB reference", "https://example.invalid", "")
DEF_CLASS2(INLINE_UID_FROM_FUID(processor_id),PClassInfo::kManyInstances,kVstAudioEffectClass,"Graphics reference",Vst::kDistributable,"Fx","1.0",kVstVersionString,Processor::create)
DEF_CLASS2(INLINE_UID_FROM_FUID(controller_id),PClassInfo::kManyInstances,kVstComponentControllerClass,"Graphics controller",0,"","1.0",kVstVersionString,Controller::create)
END_FACTORY
