use crate::win_fact::WindowType;
use crate::window_controller::{Command, CONTROLLER};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use windows::Win32::Graphics::Gdi::{DeleteObject, RedrawWindow, UpdateWindow};

use windows::Win32::{
    Foundation::{COLORREF, FALSE, HWND, LPARAM, LRESULT, RECT, TRUE, WPARAM},
    Graphics::Direct2D::Common::{D2D_POINT_2F, D2D_RECT_F},
    Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, EndPaint, FillRect, InvalidateRect, SetBkMode, HBRUSH,
        PAINTSTRUCT, RDW_INTERNALPAINT, RDW_INVALIDATE, TRANSPARENT,
    },
    System::SystemServices::MK_LBUTTON,
    UI::{
        Input::KeyboardAndMouse::{VK_ESCAPE, VK_R, VK_S},
        WindowsAndMessaging::{
            DefWindowProcW, PostQuitMessage, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN,
            WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT,
        },
    },
};
static INVALIDATED_RECT: Mutex<Option<D2D_RECT_F>> = Mutex::new(None);
static START_POINT: Mutex<Option<D2D_POINT_2F>> = Mutex::new(None);

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

                if message == WM_LBUTTONDOWN {
                    let mut start_point = START_POINT.lock().unwrap();
                    *start_point = Some(D2D_POINT_2F {
                        x: x as f32,
                        y: y as f32,
                    });
                } else if message == WM_MOUSEMOVE && (wparam.0 & MK_LBUTTON.0 as usize) != 0 {
                    let start_point = START_POINT.lock().unwrap();
                    if let Some(start) = *start_point {
                        let end = D2D_POINT_2F {
                            x: x as f32,
                            y: y as f32,
                        };

                        let mut invalidated_rect = INVALIDATED_RECT.lock().unwrap();
                        *invalidated_rect = Some(D2D_RECT_F {
                            left: start.x.min(end.x),
                            top: start.y.min(end.y),
                            right: start.x.max(end.x),
                            bottom: start.y.max(end.y),
                        });

                        if let Some(rect) = invalidated_rect.as_ref() {
                            let rect = RECT {
                                left: rect.left as _,
                                top: rect.top as _,
                                right: rect.right as _,
                                bottom: rect.bottom as _,
                            };
                            //InvalidateRect(window, Some(&rect), FALSE);
                            RedrawWindow(window, Some(&rect), None, RDW_INTERNALPAINT);
                        }
                    }
                }

                LRESULT(0)
            }
            WM_PAINT => {
                let invalidated_rect = INVALIDATED_RECT.lock().unwrap().clone();
                let rect = match invalidated_rect {
                    Some(rect) => RECT {
                        left: rect.left as _,
                        top: rect.top as _,
                        right: rect.right as _,
                        bottom: rect.bottom as _,
                    },
                    None => RECT::default(),
                };

                let mut ps = PAINTSTRUCT {
                    rcPaint: rect,
                    ..PAINTSTRUCT::default()
                };

                let hdc = BeginPaint(window, &mut ps);

                if let Some(rect) = invalidated_rect {
                    let _ = CONTROLLER.dispatch(
                        WindowType::Transparent,
                        Command::DrawRectangle {
                            start: D2D_POINT_2F {
                                x: rect.left,
                                y: rect.top,
                            },
                            end: D2D_POINT_2F {
                                x: rect.right,
                                y: rect.bottom,
                            },
                        },
                    );
                }
                _ = EndPaint(window, &ps);
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
