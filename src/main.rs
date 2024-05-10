mod modules;
use modules::*; // Import all modules

use win_fact::{WindowBuilder, WindowType};
use window_controller::CONTROLLER;
use windows::Win32::UI::WindowsAndMessaging::*;
use winproc::{opaque_handler, transparent_handler};

fn main() {
    let opaque_window = WindowBuilder::new()
        .set_window_type(WindowType::Opaque)
        .set_window_proc(opaque_handler)
        .build();

    let o_win = match opaque_window {
        Ok(window) => window,
        Err(e) => {
            eprintln!("Fehler beim Erstellen des Fensters: {}", e);
            return;
        }
    };

    let trans_window = WindowBuilder::new()
        .set_window_type(WindowType::Transparent)
        .set_window_proc(transparent_handler)
        .build();

    let t_win = match trans_window {
        Ok(window) => window,
        Err(e) => {
            eprintln!("Fehler beim Erstellen des Fensters: {}", e);
            return;
        }
    };
    o_win.show();

    let _ = CONTROLLER.add_window(t_win);
    let _ = CONTROLLER.add_window(o_win);

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
