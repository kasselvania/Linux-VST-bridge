/* Source-owned Win32/X11 popup input regression. No vendor code or input synthesis. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <windowsx.h>
#include <stdio.h>

#define FIXTURE_BUTTON 101
#define FIXTURE_HEARTBEAT 102
#define FIXTURE_ORDINARY 103
#define FIXTURE_POPUP_CLASS L"LVBTouchPopup"
#define FIXTURE_MAIN_CLASS L"LVBTouchMain"

static HWND main_window, popup_window;
static FILE *record;
static unsigned int mouse_down, mouse_up, pointer_down, pointer_up;
static unsigned int activations, open_requests, opens, closes, heartbeats, ordinary_changes;
static unsigned int last_pointer_id;
static unsigned int pointer_up_incontact, duplicate_mouse_activation;

static void event(const char *name, HWND hwnd, UINT message, WPARAM wparam)
{
    if (!record) return;
    fprintf(record, "%s\t%lu\t%p\t%u\t%u\t%u\t%p\t%u\t%u\t%u\t%u\t%u\t%u\n",
            name, GetTickCount(), hwnd, message, LOWORD(wparam), HIWORD(wparam),
            GetCapture(), mouse_down, mouse_up, pointer_down, pointer_up,
            activations, heartbeats);
    fflush(record);
}

static void close_popup(void)
{
    HWND old = popup_window;
    if (!old) return;
    if (GetCapture() == old) ReleaseCapture();
    popup_window = NULL;
    DestroyWindow(old);
    closes++;
    event("popup_closed", old, 0, 0);
    InvalidateRect(main_window, NULL, FALSE);
}

static LRESULT CALLBACK popup_proc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp)
{
    switch (msg)
    {
    case WM_POINTERDOWN:
        pointer_down++;
        last_pointer_id = LOWORD(wp);
        event("pointer_down", hwnd, msg, wp);
        break;
    case WM_POINTERUPDATE:
        if (LOWORD(wp) == last_pointer_id) event("pointer_update", hwnd, msg, wp);
        break;
    case WM_POINTERUP:
        pointer_up++;
        if (HIWORD(wp) & POINTER_MESSAGE_FLAG_INCONTACT) pointer_up_incontact++;
        event("pointer_up", hwnd, msg, wp);
        break;
    case WM_LBUTTONDOWN:
        mouse_down++;
        event("mouse_down", hwnd, msg, wp);
        return 0;
    case WM_LBUTTONUP:
        mouse_up++;
        if (GET_X_LPARAM(lp) >= 0 && GET_X_LPARAM(lp) < 180 &&
            GET_Y_LPARAM(lp) >= 0 && GET_Y_LPARAM(lp) < 48)
        {
            if (mouse_up > opens) duplicate_mouse_activation++;
            activations++;
        }
        event("mouse_up", hwnd, msg, wp);
        close_popup();
        return 0;
    case WM_CAPTURECHANGED:
        event("capture_changed", hwnd, msg, wp);
        break;
    case WM_PAINT:
    {
        PAINTSTRUCT ps;
        HDC dc = BeginPaint(hwnd, &ps);
        TextOutW(dc, 8, 15, L"Tap this menu item", 18);
        EndPaint(hwnd, &ps);
        return 0;
    }
    case WM_DESTROY:
        event("popup_destroy", hwnd, msg, wp);
        return 0;
    }
    return DefWindowProcW(hwnd, msg, wp, lp);
}

static void open_popup(HWND hwnd)
{
    RECT origin;
    if (popup_window) return;
    GetWindowRect(GetDlgItem(hwnd, FIXTURE_BUTTON), &origin);
    popup_window = CreateWindowExW(WS_EX_TOOLWINDOW, FIXTURE_POPUP_CLASS, L"",
                                   WS_POPUP | WS_BORDER | WS_VISIBLE,
                                   origin.left, origin.bottom + 4, 180, 48,
                                   hwnd, NULL, GetModuleHandleW(NULL), NULL);
    if (popup_window)
    {
        opens++;
        SetCapture(popup_window);
        event("popup_opened", popup_window, 0, 0);
    }
}

static LRESULT CALLBACK main_proc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp)
{
    switch (msg)
    {
    case WM_CREATE:
        CreateWindowExW(0, L"BUTTON", L"Open menu", WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
                        20, 30, 180, 48, hwnd, (HMENU)(ULONG_PTR)FIXTURE_BUTTON,
                        GetModuleHandleW(NULL), NULL);
        CreateWindowExW(0, L"BUTTON", L"Ordinary touch", WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
                        220, 30, 180, 48, hwnd, (HMENU)(ULONG_PTR)FIXTURE_ORDINARY,
                        GetModuleHandleW(NULL), NULL);
        SetTimer(hwnd, FIXTURE_HEARTBEAT, 100, NULL);
        return 0;
    case WM_COMMAND:
        if (LOWORD(wp) == FIXTURE_BUTTON && HIWORD(wp) == BN_CLICKED)
        {
            open_requests++;
            open_popup(hwnd);
            return 0;
        }
        if (LOWORD(wp) == FIXTURE_ORDINARY && HIWORD(wp) == BN_CLICKED)
        {
            ordinary_changes++;
            event("ordinary_control", hwnd, msg, wp);
            InvalidateRect(hwnd, NULL, FALSE);
            return 0;
        }
        break;
    case WM_TIMER:
        if (wp == FIXTURE_HEARTBEAT)
        {
            heartbeats++;
            if (!(heartbeats % 10)) event("heartbeat", hwnd, msg, wp);
        }
        return 0;
    case WM_PAINT:
    {
        PAINTSTRUCT ps;
        wchar_t text[256];
        HDC dc = BeginPaint(hwnd, &ps);
        wsprintfW(text, L"Open %u / close %u / select %u / ordinary %u / heartbeat %u",
                  opens, closes, activations, ordinary_changes, heartbeats);
        TextOutW(dc, 20, 110, text, lstrlenW(text));
        TextOutW(dc, 20, 145, L"Mouse and touch are physical inputs only.",
                 lstrlenW(L"Mouse and touch are physical inputs only."));
        EndPaint(hwnd, &ps);
        return 0;
    }
    case WM_DESTROY:
        close_popup();
        event("final", hwnd, msg, wp);
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(hwnd, msg, wp, lp);
}

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE previous, PWSTR args, int show)
{
    WNDCLASSW main_class = {0}, popup_class = {0};
    MSG msg;
    (void)previous; (void)args;
    record = fopen("popup-fixture.private.tsv", "wx");
    if (!record) return 10;
    main_class.lpfnWndProc = main_proc;
    main_class.hInstance = instance;
    main_class.lpszClassName = FIXTURE_MAIN_CLASS;
    main_class.hCursor = LoadCursorW(NULL, (LPCWSTR)IDC_ARROW);
    popup_class.lpfnWndProc = popup_proc;
    popup_class.hInstance = instance;
    popup_class.lpszClassName = FIXTURE_POPUP_CLASS;
    popup_class.hCursor = LoadCursorW(NULL, (LPCWSTR)IDC_ARROW);
    if (!RegisterClassW(&main_class) || !RegisterClassW(&popup_class)) return 11;
    main_window = CreateWindowExW(0, FIXTURE_MAIN_CLASS, L"LVB Touch Popup Fixture",
                                   WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                                   140, 110, 560, 240, NULL, NULL, instance, NULL);
    if (!main_window) return 12;
    ShowWindow(main_window, show);
    while (GetMessageW(&msg, NULL, 0, 0) > 0)
    {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    if (record) fclose(record);
    return (opens >= 2 && open_requests == opens && closes == opens &&
            activations >= 1 && ordinary_changes >= 1 &&
            heartbeats >= 10 && mouse_down == mouse_up && pointer_down == pointer_up &&
            pointer_up_incontact == 0 && duplicate_mouse_activation == 0 &&
            !popup_window && !GetCapture()) ? 0 : 13;
}
