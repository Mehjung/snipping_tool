use anyhow::{anyhow, Result};

use crate::direct2d::{capture_screen_to_bitmap, render_screen_img};
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::core::Error;
use windows::{
    core::*, Foundation::Numerics::*, Win32::Foundation::*, Win32::Graphics::Direct2D::Common::*,
    Win32::Graphics::Direct2D::*, Win32::Graphics::Direct3D::*, Win32::Graphics::Direct3D11::*,
    Win32::Graphics::Dxgi::Common::*, Win32::Graphics::Dxgi::*, Win32::Graphics::Gdi::*,
    Win32::System::Com::*, Win32::System::LibraryLoader::*, Win32::System::Performance::*,
    Win32::System::SystemInformation::GetLocalTime, Win32::UI::Animation::*,
    Win32::UI::WindowsAndMessaging::*,
};
use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*,
    Win32::System::Com::COINIT_MULTITHREADED, Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::UI::Input::KeyboardAndMouse::*, Win32::UI::WindowsAndMessaging::*,
};

use lazy_static::lazy_static;
use std::sync::{mpsc, Arc};

mod direct2d;
mod errorhandler;
mod win_fact;
mod window_controller;

use errorhandler::{handle_error, ExpectedError};
use std::time::Instant;
use win_fact::{Window, WindowBuilder, WindowType};
use window_controller::{Command, WindowController};

static FIRST_PAINT: AtomicBool = AtomicBool::new(true);
static DRAWING_DATA: Lazy<Mutex<DrawingData>> = Lazy::new(|| {
    Mutex::new(DrawingData {
        is_drawing: false,
        start_x: 0,
        start_y: 0,
        end_x: 0,
        end_y: 0,
    })
});

static CONTROLLER: Lazy<Arc<window_controller::WindowController>> =
    Lazy::new(|| Arc::new(window_controller::WindowController::new()));

lazy_static! {
    static ref TRANSPARENT_WINDOW_HANDLE: Mutex<Option<HWND>> = Mutex::new(None);
}
struct DrawingData {
    is_drawing: bool,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
}

fn main() {
    let opaque_window = WindowBuilder::new()
        .set_window_type(WindowType::Opaque)
        .set_window_proc(opaque_handler)
        .build();

    let trans_window = WindowBuilder::new()
        .set_window_type(WindowType::Transparent)
        .set_window_proc(transparent_handler)
        .build();

    let o_win = match opaque_window {
        Ok(window) => window,
        Err(e) => {
            eprintln!("Fehler beim Erstellen des Fensters: {}", e);
            return;
        }
    };

    let t_win = match trans_window {
        Ok(window) => {
            let mut handle = TRANSPARENT_WINDOW_HANDLE.lock().unwrap();
            *handle = Some(window.get_hwnd());
            window
        }
        Err(e) => {
            eprintln!("Fehler beim Erstellen des Fensters: {}", e);
            return;
        }
    };
    CONTROLLER.add_window(t_win);
    o_win.show();

    let mut msg: MSG = MSG::default();
    unsafe {
        let _ = GetMessageW(&mut msg, None, 0, 0);
        while msg.message != WM_QUIT {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
            let _ = GetMessageW(&mut msg, None, 0, 0);
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
                if FIRST_PAINT.swap(false, Ordering::SeqCst) {
                    let start = Instant::now();
                    let _ = handle_error::<(), _>("Render error", ExpectedError::Other, || {
                        render_screen_img(window).is_err()
                    });
                    let duration = start.elapsed(); // Get the elapsed time
                    println!("Time elapsed in render_screen_img is: {:?}", duration);
                    CONTROLLER.dispatch(WindowType::Transparent, Command::Show);
                    /*
                    let t_win_handle = TRANSPARENT_WINDOW_HANDLE.lock().unwrap();
                    if let Some(handle) = *t_win_handle {
                        PostMessageW(handle, WM_SHOW_T_WIN, WPARAM(0), LPARAM(0));
                    } */
                }
                LRESULT(0)
            }
            WM_CREATE => LRESULT(0),

            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

const WM_SHOW_T_WIN: u32 = windows::Win32::UI::WindowsAndMessaging::WM_USER + 1;
const WM_RENDER_O_WIN: u32 = windows::Win32::UI::WindowsAndMessaging::WM_USER + 2;
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
                LRESULT(0)
            }
            WM_DESTROY => {
                println!("WM_DESTROY");
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_SHOW_T_WIN => {
                let _ = ShowWindow(window, SW_SHOW);
                LRESULT(0)
            }

            WM_CREATE => LRESULT(0),

            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}
