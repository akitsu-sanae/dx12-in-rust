extern crate widestring;
extern crate winapi;

mod msg;
mod window;

fn main() {
    let mut window = window::Window::create("dx12 test in rust", 640, 480);
    window.show();

    loop {
        if let Some(msg) = msg::peek() {
            if msg.0.message == winapi::um::winuser::WM_QUIT {
                break;
            }
        }
    }
}
