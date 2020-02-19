use std::ptr::{null, null_mut};
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HWND, RECT};
use winapi::um::winuser::WNDCLASSEXW;

use widestring::U16CString;

pub struct Window {
    pub handle: HWND,
    class: WNDCLASSEXW,
}

impl Window {
    pub fn create(name: &str, width: usize, height: usize) -> Window {
        let name = U16CString::from_str(name).unwrap();
        use winapi::um::{
            libloaderapi::GetModuleHandleW,
            winuser::{
                AdjustWindowRect, CreateWindowExW, RegisterClassExW, CW_USEDEFAULT, WS_EX_LEFT,
                WS_OVERLAPPEDWINDOW,
            },
        };

        use std::mem;
        let class = WNDCLASSEXW {
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

        unsafe { RegisterClassExW(&class) };

        let mut window_rect = RECT {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };

        unsafe {
            AdjustWindowRect(&mut window_rect, WS_OVERLAPPEDWINDOW, 0);
        }

        let handle = unsafe {
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
                class.hInstance,
                null_mut(),
            )
        };
        Window {
            handle: handle,
            class: class,
        }
    }

    pub fn show(&mut self) {
        use winapi::um::winuser::{ShowWindow, SW_SHOW};
        unsafe { ShowWindow(self.handle, SW_SHOW) };
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        use winapi::um::winuser::UnregisterClassW;
        unsafe { UnregisterClassW(self.class.lpszClassName, self.class.hInstance) };
    }
}

extern "system" fn procedure(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    use winapi::um::winuser::{DefWindowProcW, PostQuitMessage, WM_DESTROY};
    if msg == WM_DESTROY {
        unsafe { PostQuitMessage(0) };
        return 0;
    }
    return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
}
