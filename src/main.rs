extern crate winapi;
extern crate widestring;

use std::ptr::{null, null_mut};

use widestring::U16String;

fn main() {
    use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
    use winapi::shared::windef::{HWND, RECT};

    extern "system" fn procedure(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        use winapi::um::winuser::{DefWindowProcW, PostQuitMessage, WM_DESTROY};
        if msg == WM_DESTROY {
            unsafe { PostQuitMessage(0) };
            return 0;
        }
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    use winapi::um::{
        libloaderapi::GetModuleHandleW,
        winuser::{
            AdjustWindowRect, CreateWindowExW, DispatchMessageW, PeekMessageW, RegisterClassExW,
            ShowWindow, TranslateMessage, UnregisterClassW, CW_USEDEFAULT, MSG, PM_REMOVE,
            SW_SHOW, WM_QUIT, WNDCLASSEXW, WS_EX_LEFT, WS_OVERLAPPEDWINDOW,
        },
    };

    let name = U16String::from_str("dx12 in rust");

    use std::mem;
    let window_class = WNDCLASSEXW {
        cbSize: mem::size_of::<WNDCLASSEXW>() as UINT,
        lpfnWndProc: Some(procedure),
        lpszClassName: name.as_ptr(),
        hInstance: unsafe { GetModuleHandleW(null()) },
        cbClsExtra: 0,
        cbWndExtra: 0,
        style: 0,
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        hIconSm: null_mut(),
    };

    unsafe { RegisterClassExW(&window_class) };

    let window_width = 640;
    let window_height = 480;

    let mut window_rect = RECT {
        left: 0,
        top: 0,
        right: window_width,
        bottom: window_height,
    };

    unsafe {
        AdjustWindowRect(&mut window_rect, WS_OVERLAPPEDWINDOW, 0);
    }

    let window_handle = unsafe {
        CreateWindowExW(
            WS_EX_LEFT,
            name.as_ptr(),
            name.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            window_rect.right - window_rect.left,
            window_rect.bottom - window_rect.top,
            null_mut(),
            null_mut(),
            window_class.hInstance,
            null_mut(),
        )
    };

    unsafe { ShowWindow(window_handle, SW_SHOW) };

    let mut msg: MSG = unsafe { std::mem::zeroed() };

    loop {
        if unsafe { PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe { TranslateMessage(&mut msg) };
            unsafe { DispatchMessageW(&mut msg) };
        }

        if msg.message == WM_QUIT {
            break;
        }
    }

    unsafe { UnregisterClassW(window_class.lpszClassName, window_class.hInstance) };
}
