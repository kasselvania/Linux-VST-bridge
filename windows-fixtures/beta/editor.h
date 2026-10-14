// Original Win32 SDK edge for the stateful first-party editor variant.
#include "public.sdk/source/common/pluginview.h"
#include <windows.h>
#include <cwchar>

class ReferenceEditor final : public CPluginView {
    ReferenceController& controller;
    HWND window = nullptr;
    HINSTANCE instance = nullptr;
    ATOM windowClass = 0;
    bool dragging = false;
    static constexpr int width = 320, height = 140;
    void finish() {
        if (!dragging) return;
        dragging = false;
        controller.endEdit(0);
        if (GetCapture() == window) ReleaseCapture();
    }
    void value(double selected) {
        const auto before = controller.getParamNormalized(0);
        if (controller.setParamNormalized(0, selected) != kResultOk) return;
        if (controller.performEdit(0, selected) != kResultOk)
            controller.setParamNormalized(0, before);
        InvalidateRect(window, nullptr, FALSE);
    }
    void pointer(LPARAM position) {
        const auto x = static_cast<short>(LOWORD(position));
        value(std::clamp(double(x - 20) / 280., 0., 1.));
    }
    void paint() {
        PAINTSTRUCT paint{};
        const auto dc = BeginPaint(window, &paint);
        RECT client{}; GetClientRect(window, &client);
        FillRect(dc, &client, static_cast<HBRUSH>(GetStockObject(BLACK_BRUSH)));
        SetBkMode(dc, TRANSPARENT); SetTextColor(dc, RGB(235, 235, 235));
        wchar_t text[64]{};
        std::swprintf(text, 64, L"Level  %.0f%%", controller.getParamNormalized(0) * 100.);
        RECT label{20, 14, 300, 42}; DrawTextW(dc, text, -1, &label, DT_LEFT);
        RECT bar{20, 58, 300, 90};
        FillRect(dc, &bar, static_cast<HBRUSH>(GetStockObject(GRAY_BRUSH)));
        bar.right = 20 + int(controller.getParamNormalized(0) * 280.);
        auto brush = CreateSolidBrush(RGB(70, 180, 130));
        FillRect(dc, &bar, brush); DeleteObject(brush);
        RECT hint{20, 106, 310, 130};
        DrawTextW(dc, L"Drag Level; arrow keys adjust", -1, &hint, DT_LEFT);
        EndPaint(window, &paint);
    }
    static LRESULT CALLBACK procedure(HWND hwnd, UINT message, WPARAM key, LPARAM position) {
        auto* view = reinterpret_cast<ReferenceEditor*>(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
        if (message == WM_NCCREATE) {
            view = static_cast<ReferenceEditor*>(reinterpret_cast<CREATESTRUCTW*>(position)->lpCreateParams);
            view->window = hwnd;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(view));
        }
        if (!view) return DefWindowProcW(hwnd, message, key, position);
        switch (message) {
        case WM_LBUTTONDOWN:
            if (!view->dragging && static_cast<short>(HIWORD(position)) >= 58 && static_cast<short>(HIWORD(position)) <= 90
                && view->controller.beginEdit(0) == kResultOk) {
                view->dragging = true; SetFocus(hwnd); SetCapture(hwnd); view->pointer(position);
            }
            return 0;
        case WM_MOUSEMOVE: if (view->dragging) view->pointer(position); return 0;
        case WM_LBUTTONUP: view->finish(); return 0;
        case WM_CAPTURECHANGED: case WM_KILLFOCUS: view->finish(); return 0;
        case WM_KEYDOWN:
            if ((key == VK_LEFT || key == VK_RIGHT) && !view->dragging
                && view->controller.beginEdit(0) == kResultOk) {
                view->value(std::clamp(view->controller.getParamNormalized(0)
                    + (key == VK_RIGHT ? .05 : -.05), 0., 1.));
                view->controller.endEdit(0);
            }
            return 0;
        case WM_PAINT: view->paint(); return 0;
        case WM_TIMER: InvalidateRect(hwnd, nullptr, FALSE); return 0;
        case WM_NCDESTROY:
            view->finish(); view->window = nullptr;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0); break;
        }
        return DefWindowProcW(hwnd, message, key, position);
    }
public:
    explicit ReferenceEditor(ReferenceController& owner) : controller(owner) {
        controller.addRef(); rect = {0, 0, width, height};
    }
    ~ReferenceEditor() override { removed(); controller.release(); }
    tresult PLUGIN_API isPlatformTypeSupported(FIDString type) override {
        return type && std::strcmp(type, kPlatformTypeHWND) == 0 ? kResultOk : kResultFalse;
    }
    tresult PLUGIN_API attached(void* parent, FIDString type) override {
        if (window || !parent || isPlatformTypeSupported(type) != kResultOk) return kResultFalse;
        if (!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            reinterpret_cast<LPCWSTR>(&procedure), &instance)) return kResultFalse;
        wchar_t name[64]{}; std::swprintf(name, 64, L"LVBStatefulEditor_%p", static_cast<void*>(this));
        WNDCLASSW wc{}; wc.lpfnWndProc = procedure;
        wc.hInstance = instance; wc.lpszClassName = name;
        wc.hCursor = LoadCursorW(nullptr, IDC_ARROW);
        windowClass = RegisterClassW(&wc);
        if (!windowClass) return kResultFalse;
        window = CreateWindowW(MAKEINTATOM(windowClass), L"Level", WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            0, 0, width, height, static_cast<HWND>(parent), nullptr, wc.hInstance, this);
        if (!window) { removed(); return kResultFalse; }
        if (!SetTimer(window, 1, 50, nullptr)) { removed(); return kResultFalse; }
        systemWindow = parent; return kResultOk;
    }
    tresult PLUGIN_API removed() override {
        finish();
        if (window) { KillTimer(window, 1); if (!DestroyWindow(window)) return kResultFalse; }
        if (windowClass) {
            if (!UnregisterClassW(MAKEINTATOM(windowClass), instance)) return kResultFalse;
            windowClass = 0;
        }
        systemWindow = nullptr; return kResultOk;
    }
    tresult PLUGIN_API onFocus(TBool state) override {
        if (!window) return kResultFalse;
        if (state) SetFocus(window); else finish();
        return kResultOk;
    }
    tresult PLUGIN_API canResize() override { return kResultFalse; }
    tresult PLUGIN_API checkSizeConstraint(ViewRect* size) override {
        if (!size) return kInvalidArgument;
        *size = {0, 0, width, height}; return kResultOk;
    }
    tresult PLUGIN_API onSize(ViewRect* size) override {
        if (!size || size->getWidth() != width || size->getHeight() != height) return kResultFalse;
        rect = *size;
        if (window) MoveWindow(window, 0, 0, width, height, TRUE);
        return kResultOk;
    }
};
IPlugView* PLUGIN_API ReferenceController::createView(FIDString name) {
    return name && std::strcmp(name, ViewType::kEditor) == 0 ? new ReferenceEditor(*this) : nullptr;
}
