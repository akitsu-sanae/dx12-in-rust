extern crate widestring;
extern crate winapi;

mod direct3d;
mod math;
mod msg;
mod util;
mod window;

fn enable_debug_layer() {
    use std::ptr::null_mut;
    use winapi::ctypes::c_void;
    use winapi::um::d3d12::D3D12GetDebugInterface;
    use winapi::um::d3d12sdklayers::ID3D12Debug;
    use winapi::Interface;
    let mut debug_layer: *mut ID3D12Debug = null_mut();
    let result = unsafe {
        D3D12GetDebugInterface(
            &ID3D12Debug::uuidof(),
            &mut debug_layer as *mut *mut _ as *mut *mut c_void,
        )
    };
    if util::is_succeeded(result) {
        eprintln!("enable debug layer");
        unsafe {
            (*debug_layer).EnableDebugLayer();
            (*debug_layer).Release();
        }
    }
}

fn main() {
    let mut window = window::Window::create("dx12 test in rust", 640, 480);
    enable_debug_layer();

    let mut direct3d = direct3d::Direct3D::create(&window).unwrap();

    window.show();

    loop {
        if let Some(msg) = msg::peek() {
            if msg.0.message == winapi::um::winuser::WM_QUIT {
                break;
            }
        }

        direct3d.update();
    }
}
