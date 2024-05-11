use crate::win_fact::WindowType;
use crate::window_controller::{Command, CONTROLLER};
use once_cell::sync::Lazy;
use std::sync::Mutex;

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Direct2D::Common::D2D_POINT_2F,
    System::SystemServices::MK_LBUTTON,
    UI::{
        Input::KeyboardAndMouse::{VK_ESCAPE, VK_R, VK_S},
        WindowsAndMessaging::{
            DefWindowProcW, PostQuitMessage, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN,
            WM_LBUTTONUP, WM_MOUSEMOVE,
        },
    },
};

static START: Lazy<Mutex<D2D_POINT_2F>> = Lazy::new(|| Mutex::new(D2D_POINT_2F { x: 0.0, y: 0.0 }));

macro_rules! get_x_lparam {
    ($lparam:expr) => {
        ($lparam & 0xFFFF) as i16 as i32 // Cast to i16 first to handle negative coordinates correctly
    };
}

macro_rules! get_y_lparam {
    ($lparam:expr) => {
        (($lparam >> 16) & 0xFFFF) as i16 as i32 // Shift right 16 bits and then cast to i16
    };
}

pub extern "system" fn transparent_handler(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP => {
                let x = get_x_lparam!(lparam.0);
                let y = get_y_lparam!(lparam.0);

                //println!("Mouse position: ({}, {})", x, y);
                //println!("Message: {} {}", message, WM_LBUTTONDOWN);
                if message == WM_LBUTTONDOWN {
                    print!("Mouse down at ({}, {})", x, y);
                    let mut start = START.lock().unwrap();
                    *start = D2D_POINT_2F {
                        x: x as f32,
                        y: y as f32,
                    };
                } else if message == WM_MOUSEMOVE && (wparam.0 & MK_LBUTTON.0 as usize) != 0 {
                    //println!("Mouse move (with button down) at ({}, {})", x, y);
                    let _ = CONTROLLER.dispatch(
                        WindowType::Transparent,
                        Command::DrawRectangle {
                            start: *START.lock().unwrap(),
                            end: D2D_POINT_2F {
                                x: x as f32,
                                y: y as f32,
                            },
                        },
                    );
                } else if message == WM_LBUTTONUP {
                    //println!("Mouse up at ({}, {})", x, y);
                    // Beende das Zeichnen
                }
                LRESULT(0)
            }
            WM_KEYDOWN => {
                if wparam.0 == VK_ESCAPE.0 as usize {
                    PostQuitMessage(0);
                }
                if wparam.0 == VK_S.0 as usize {
                    println!("Pressed S");
                    let _ = CONTROLLER.dispatch(WindowType::Opaque, Command::TriggerScreenshot);
                }
                if wparam.0 == VK_R.0 as usize {
                    println!("Reloaded");
                    let _ = CONTROLLER.dispatch(WindowType::Transparent, Command::Hide);
                    let _ = CONTROLLER.dispatch(WindowType::Opaque, Command::Reload);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                println!("WM_DESTROY");
                PostQuitMessage(0);
                LRESULT(0)
            }

            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

pub extern "system" fn opaque_handler(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_KEYDOWN => {
                if wparam.0 == VK_ESCAPE.0 as usize {
                    PostQuitMessage(0);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                println!("WM_DESTROY");
                PostQuitMessage(0);
                LRESULT(0)
            }

            WM_ERASEBKGND => {
                //if FIRST_PAINT.swap(false, Ordering::SeqCst) {
                let _ = CONTROLLER.dispatch(WindowType::Opaque, Command::AutoScreenshot);
                let _ = CONTROLLER.dispatch(WindowType::Transparent, Command::Show);
                //}
                LRESULT(0)
            }

            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}
