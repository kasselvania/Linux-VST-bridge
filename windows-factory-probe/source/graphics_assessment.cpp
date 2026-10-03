#include "graphics_assessment.h"
#include "vendor_view.h"
#include <windows.h>
#include <d3d11.h>
#include <dxgi.h>
#include <dcomp.h>
#include <GL/gl.h>
#include <wrl/client.h>
#include <array>
#include <cctype>
#include <string>
#include <utility>

namespace linux_vst_bridge::wf0 {
namespace {
using Microsoft::WRL::ComPtr;
struct LibraryHandle {
    HMODULE handle;
    explicit LibraryHandle(const wchar_t* name):handle(LoadLibraryW(name)){}
    ~LibraryHandle(){if(handle)FreeLibrary(handle);}
    template<class T>T get(const char* name)const{return handle?reinterpret_cast<T>(GetProcAddress(handle,name)):nullptr;}
};
std::string safe_text(const char* value) {
    if(!value)return "null";
    std::string out;
    for(size_t i=0;i<256;++i){
        auto c=static_cast<unsigned char>(value[i]);
        if(!c)return out.empty()?"null":"\""+out+"\"";
        if(c>127||(!std::isalnum(c)&&std::string(" ._()-:+").find(char(c))==std::string::npos))return "null";
        out+=char(c);
    }
    return "null";
}
std::string safe_text(const wchar_t* value, size_t length) {
    std::array<char,256> buffer{};
    for(size_t i=0;i<buffer.size()&&i<length;++i){
        if(value[i]>127)return "null";
        buffer[i]=char(value[i]);if(!value[i])return safe_text(buffer.data());
    }
    return "null";
}
bool software(std::string value){
    for(auto& c:value)c=char(std::tolower(static_cast<unsigned char>(c)));
    return value.find("llvmpipe")!=std::string::npos||value.find("softpipe")!=std::string::npos
        ||value.find("software rasterizer")!=std::string::npos||value.find("basic render")!=std::string::npos;
}
struct Probe {
    std::string api,status="unavailable",rendering="unknown",renderer="null",vendor="null",device="null",driver="null",level="null";
    explicit Probe(const char* name):api(name){}
    std::string json()const{
        return "{\"api\":\""+api+"\",\"status\":\""+status+"\",\"rendering\":\""+rendering+
            "\",\"renderer\":"+renderer+",\"vendor_id\":"+vendor+",\"device_id\":"+device+
            ",\"driver_version\":"+driver+",\"feature_level\":"+level+"}";
    }
};
bool pixels(const unsigned char* p){return p&&p[0]>=63&&p[0]<=65&&p[1]>=127&&p[1]<=129&&p[2]>=190&&p[2]<=192&&p[3]==255;}
std::pair<Probe,Probe> d3d(D3D_DRIVER_TYPE type){
    Probe result(type==D3D_DRIVER_TYPE_WARP?"d3d11_warp":"d3d11_default"),composition("direct_composition");
    LibraryHandle library(L"d3d11.dll");
    auto create=library.get<PFN_D3D11_CREATE_DEVICE>("D3D11CreateDevice");
    if(!create)return {result,composition};
    ComPtr<ID3D11Device> device;ComPtr<ID3D11DeviceContext> context;D3D_FEATURE_LEVEL level{};
    if(FAILED(create(nullptr,type,nullptr,D3D11_CREATE_DEVICE_BGRA_SUPPORT,nullptr,0,D3D11_SDK_VERSION,&device,&level,&context)))return {result,composition};
    result.level=std::to_string(unsigned(level));
    std::string rendering="unknown";
    ComPtr<IDXGIDevice> dxgi;ComPtr<IDXGIAdapter> adapter;DXGI_ADAPTER_DESC description{};
    if(SUCCEEDED(device.As(&dxgi))&&SUCCEEDED(dxgi->GetAdapter(&adapter))&&SUCCEEDED(adapter->GetDesc(&description))){
        result.renderer=safe_text(description.Description,128);result.vendor=std::to_string(description.VendorId);result.device=std::to_string(description.DeviceId);
        rendering=type==D3D_DRIVER_TYPE_WARP||software(result.renderer)?"reported_software":"reported_hardware";
        LARGE_INTEGER version{};
        if(SUCCEEDED(adapter->CheckInterfaceSupport(__uuidof(ID3D11Device),&version)))
            result.driver="\""+std::to_string(HIWORD(version.HighPart))+"."+std::to_string(LOWORD(version.HighPart))+"."+
                std::to_string(HIWORD(version.LowPart))+"."+std::to_string(LOWORD(version.LowPart))+"\"";
    }
    if(type==D3D_DRIVER_TYPE_WARP)rendering="reported_software";
    D3D11_TEXTURE2D_DESC desc{};desc.Width=2;desc.Height=2;desc.MipLevels=1;desc.ArraySize=1;
    desc.Format=DXGI_FORMAT_R8G8B8A8_UNORM;desc.SampleDesc.Count=1;desc.Usage=D3D11_USAGE_DEFAULT;desc.BindFlags=D3D11_BIND_RENDER_TARGET;
    ComPtr<ID3D11Texture2D> target,readback;ComPtr<ID3D11RenderTargetView> view;
    result.status="readback_failed";
    if(SUCCEEDED(device->CreateTexture2D(&desc,nullptr,&target))&&SUCCEEDED(device->CreateRenderTargetView(target.Get(),nullptr,&view))){
        const float colour[]={.25f,.5f,.75f,1.f};context->ClearRenderTargetView(view.Get(),colour);
        desc.Usage=D3D11_USAGE_STAGING;desc.BindFlags=0;desc.CPUAccessFlags=D3D11_CPU_ACCESS_READ;
        if(SUCCEEDED(device->CreateTexture2D(&desc,nullptr,&readback))){
            context->CopyResource(readback.Get(),target.Get());D3D11_MAPPED_SUBRESOURCE mapped{};
            if(SUCCEEDED(context->Map(readback.Get(),0,D3D11_MAP_READ,0,&mapped))){
                const auto* bytes=static_cast<const unsigned char*>(mapped.pData);
                const bool matched=mapped.RowPitch>=8&&pixels(bytes)&&pixels(bytes+4)&&pixels(bytes+mapped.RowPitch)&&pixels(bytes+mapped.RowPitch+4);
                context->Unmap(readback.Get(),0);
                if(matched){result.status="passed";result.rendering=rendering;}
            }
        }
    }
    if(type!=D3D_DRIVER_TYPE_WARP&&dxgi){
        LibraryHandle comp(L"dcomp.dll");
        using Create=HRESULT(WINAPI*)(IDXGIDevice*,REFIID,void**);
        auto make=comp.get<Create>("DCompositionCreateDevice");ComPtr<IDCompositionDevice> composition_device;
        if(make&&SUCCEEDED(make(dxgi.Get(),__uuidof(IDCompositionDevice),reinterpret_cast<void**>(composition_device.GetAddressOf()))))composition.status="passed";
    }
    return {result,composition};
}
Probe opengl(){
    Probe result("opengl");LibraryHandle library(L"opengl32.dll");
    auto create=library.get<decltype(&wglCreateContext)>("wglCreateContext");
    auto current=library.get<decltype(&wglMakeCurrent)>("wglMakeCurrent");
    auto remove=library.get<decltype(&wglDeleteContext)>("wglDeleteContext");
    auto string=library.get<decltype(&glGetString)>("glGetString");
    auto clear_colour=library.get<decltype(&glClearColor)>("glClearColor");
    auto clear=library.get<decltype(&glClear)>("glClear");
    auto read=library.get<decltype(&glReadPixels)>("glReadPixels");
    auto error=library.get<decltype(&glGetError)>("glGetError");
    if(!create||!current||!remove||!string||!clear_colour||!clear||!read||!error)return result;
    WNDCLASSW wc{};wc.style=CS_OWNDC;wc.lpfnWndProc=DefWindowProcW;wc.hInstance=GetModuleHandleW(nullptr);wc.lpszClassName=L"LVBGraphicsProbe";
    if(!RegisterClassW(&wc)&&GetLastError()!=ERROR_CLASS_ALREADY_EXISTS)return result;
    HWND window=CreateWindowW(wc.lpszClassName,L"Graphics assessment",WS_POPUP,0,0,32,32,nullptr,nullptr,wc.hInstance,nullptr);
    if(!window)return result;
    HDC dc=GetDC(window);HGLRC context=nullptr;
    PIXELFORMATDESCRIPTOR request{};request.nSize=sizeof(request);request.nVersion=1;
    request.dwFlags=PFD_DRAW_TO_WINDOW|PFD_SUPPORT_OPENGL;request.iPixelType=PFD_TYPE_RGBA;request.cColorBits=32;request.cAlphaBits=8;
    const int format=dc?ChoosePixelFormat(dc,&request):0;
    if(format&&SetPixelFormat(dc,format,&request)&&(context=create(dc))&&current(dc,context)){
        result.renderer=safe_text(reinterpret_cast<const char*>(string(GL_RENDERER)));
        result.driver=safe_text(reinterpret_cast<const char*>(string(GL_VERSION)));
        PIXELFORMATDESCRIPTOR actual{};const bool described=DescribePixelFormat(dc,format,sizeof(actual),&actual)!=0;
        clear_colour(.25f,.5f,.75f,1.f);clear(GL_COLOR_BUFFER_BIT);std::array<unsigned char,16> bytes{};read(0,0,2,2,GL_RGBA,GL_UNSIGNED_BYTE,bytes.data());
        result.status="readback_failed";
        if(error()==GL_NO_ERROR&&pixels(bytes.data())&&pixels(bytes.data()+4)&&pixels(bytes.data()+8)&&pixels(bytes.data()+12)){
            result.status="passed";
            if(software(result.renderer)||(described&&(actual.dwFlags&PFD_GENERIC_FORMAT)&&!(actual.dwFlags&PFD_GENERIC_ACCELERATED)))result.rendering="reported_software";
            else if(described)result.rendering="reported_hardware";
        }
        current(nullptr,nullptr);
    }
    if(context)remove(context);if(dc)ReleaseDC(window,dc);DestroyWindow(window);
    return result;
}
std::string loaded(){
    const std::pair<const wchar_t*,const char*> names[]={{L"d3d9.dll","d3d9"},{L"d3d11.dll","d3d11"},{L"d3d12.dll","d3d12"},
        {L"dxgi.dll","dxgi"},{L"d2d1.dll","direct2d"},{L"dcomp.dll","direct_composition"},{L"opengl32.dll","open_gl"},
        {L"vulkan-1.dll","vulkan"},{L"WebView2Loader.dll","web_view2"},{L"libEGL.dll","angle_egl"},{L"libGLESv2.dll","angle_gles"}};
    std::string result="[";
    for(const auto& name:names)if(GetModuleHandleW(name.first)){if(result.size()>1)result+=",";result+="\""+std::string(name.second)+"\"";}
    return result+"]";
}
}
void assess_graphics_editor(Steinberg::Vst::IEditController& controller,EventWriter& events){
    // Observe before independent probes load any graphics libraries. The union
    // is this process's loaded-library fingerprint, never proof of API use.
    const auto before=loaded();VendorView view;
    const bool opened=view.open(controller);
    const auto deadline=GetTickCount64()+750;
    while(opened&&GetTickCount64()<deadline&&VendorView::pump()&&!view.close_requested())Sleep(5);
    const auto during=loaded();
    if(!view.close())ExitProcess(92);
    events.lifecycle("graphics_assessment",",\"schema\":1,\"editor\":{\"status\":\""+std::string(opened?"opened":"unavailable")+
        "\",\"before\":"+before+",\"during\":"+during+",\"closed\":true}");
}
void assess_graphics_runtime(EventWriter& events){
    const auto hardware=d3d(D3D_DRIVER_TYPE_HARDWARE);
    const auto warp=d3d(D3D_DRIVER_TYPE_WARP);
    const auto gl=opengl();
    events.lifecycle("graphics_runtime",",\"schema\":1,\"probes\":["+hardware.first.json()+","+warp.first.json()+","+gl.json()+","+hardware.second.json()+"]");
}
}
