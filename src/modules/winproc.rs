use crate::win_fact::WindowType;
use crate::window_controller::{Command, CONTROLLER};
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{VK_ESCAPE, VK_S},
        WindowsAndMessaging::{
            DefWindowProcW, PostQuitMessage, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN,
        },
    },
};

static FIRST_PAINT: AtomicBool = AtomicBool::new(true);

pub extern "system" fn transparent_handler(
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
                if wparam.0 == VK_S.0 as usize {
                    println!("Pressed S");
                    let _ = CONTROLLER.dispatch(WindowType::Opaque, Command::TriggerScreenshot);
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
