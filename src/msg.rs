use std::ptr::null_mut;
use winapi::um::winuser::{DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE};

pub struct Msg(pub MSG);

pub fn peek() -> Option<Msg> {
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    if unsafe { PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) } != 0 {
        unsafe { TranslateMessage(&mut msg) };
        unsafe { DispatchMessageW(&mut msg) };
        Some(Msg(msg))
    } else {
        None
    }
}
