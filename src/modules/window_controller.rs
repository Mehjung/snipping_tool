use crate::errorhandler::throw_error;
use crate::win_fact::{Window, WindowType};
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};
use windows::Win32::Graphics::Direct2D::Common::D2D_POINT_2F;

pub static CONTROLLER: Lazy<Arc<WindowController>> =
    Lazy::new(|| Arc::new(WindowController::new()));

pub enum Command {
    Show,
    AutoScreenshot,
    TriggerScreenshot,
    Reload,
    Hide,
    DrawRectangle {
        start: D2D_POINT_2F,
        end: D2D_POINT_2F,
    },
}

pub struct WindowController {
    transparent_window: Mutex<Option<Window>>,
    opaque_window: Mutex<Option<Window>>,
    main_window: Mutex<Option<Window>>,
}

impl WindowController {
    pub fn new() -> Self {
        WindowController {
            transparent_window: Mutex::new(None),
            opaque_window: Mutex::new(None),
            main_window: Mutex::new(None),
        }
    }
    fn window_ref(&self, window_type: WindowType) -> Option<&Mutex<Option<Window>>> {
        match window_type {
            WindowType::Transparent => Some(&self.transparent_window),
            WindowType::Opaque => Some(&self.opaque_window),
            WindowType::Main => Some(&self.main_window),
            _ => None,
        }
    }

    fn locked_window(
        &self,
        window_type: WindowType,
    ) -> Result<MutexGuard<Option<Window>>, anyhow::Error> {
        self.window_ref(window_type)
            .ok_or_else(|| throw_error::<()>("Invalid window type").unwrap_err()) // korrekter Gebrauch von throw_error
            .and_then(|mutex| {
                mutex
                    .lock()
                    .map_err(|_| throw_error::<()>("Failed to lock window mutex").unwrap_err())
            })
    }

    pub fn add_window(&self, window: Window) -> Result<(), anyhow::Error> {
        let mut locked_window = self.locked_window(window.window_type)?;
        *locked_window = Some(window);
        Ok(())
    }

    pub fn dispatch(&self, window_type: WindowType, command: Command) -> Result<(), anyhow::Error> {
        if let Some(window) = &*self.locked_window(window_type)? {
            match command {
                Command::Show => window.show(),
                Command::AutoScreenshot => window.auto_screenshot(),
                Command::TriggerScreenshot => window.trigger_screenshot(),
                Command::Reload => window.reload(),
                Command::Hide => window.hide(),
                Command::DrawRectangle { start, end } => window.draw_rectangle(start, end),
            }
        }
        Ok(())
    }
}
