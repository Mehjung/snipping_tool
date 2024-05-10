use crate::errorhandler::{handle_error, throw_error, ExpectedError};
use std::os::raw::c_void;
use windows::core::Error;
use windows::{
    core::{w, PCWSTR},
    Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    Win32::Graphics::Gdi::CreateSolidBrush,
    Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetSystemMetrics, LoadCursorW, RegisterClassW,
        SetForegroundWindow, SetLayeredWindowAttributes, SetWindowPos, ShowWindow, CS_HREDRAW,
        CS_OWNDC, CS_VREDRAW, HMENU, HWND_TOPMOST, IDC_ARROW, IDC_CROSS, LWA_ALPHA,
        SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOMOVE,
        SWP_NOSIZE, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW, WS_EX_COMPOSITED,
        WS_EX_LAYERED, WS_POPUP,
    },
};
pub struct Window {
    pub hwnd: HWND,
    pub window_type: WindowType,
}

impl Window {
    pub fn get_hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn set_foreground(&self) {
        unsafe {
            SetForegroundWindow(self.hwnd);
        }
    }

    pub fn set_position(&self) {
        unsafe {
            SetWindowPos(self.hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        }
    }

    pub fn make_transparent(&self) -> Result<(), Error> {
        unsafe {
            SetLayeredWindowAttributes(
                self.hwnd,
                COLORREF(0x000000),
                (0.55 * 255.0) as u8,
                LWA_ALPHA,
            )
        }
    }

    pub fn show(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_SHOW);
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if let Err(e) = DestroyWindow(self.hwnd) {
                let error = anyhow::anyhow!("Error destroying window: {:?}", e);
                eprintln!("{}", error);
            }
        }
    }
}

pub struct WINDOWPROPS {
    pub dwexstyle: WINDOW_EX_STYLE,
    pub lpclassname: PCWSTR,
    pub lpwindowname: PCWSTR,
    pub dwstyle: WINDOW_STYLE,
    pub x: i32,
    pub y: i32,
    pub nwidth: i32,
    pub nheight: i32,
    pub hwndparent: HWND,
    pub hmenu: HMENU,
    pub hinstance: HINSTANCE,
    pub lpparam: Option<*const c_void>,
}

impl Default for WINDOWPROPS {
    fn default() -> Self {
        WINDOWPROPS {
            dwexstyle: WINDOW_EX_STYLE(0),
            lpclassname: w!("win_template_class"),
            lpwindowname: w!("win_template_window"),
            dwstyle: WINDOW_STYLE(0),
            x: 0,
            y: 0,
            nwidth: 0,
            nheight: 0,
            hwndparent: HWND::default(),
            hmenu: HMENU::default(),
            hinstance: HINSTANCE::default(),
            lpparam: None,
        }
    }
}

pub struct WindowTemplate {
    pub windowprops: WINDOWPROPS,
    pub classprops: WNDCLASSW,
}

impl WindowTemplate {
    fn new() -> Self {
        WindowTemplate {
            windowprops: WINDOWPROPS::default(),
            classprops: WNDCLASSW::default(),
        }
    }
    fn create_window(&self, builder: &WindowBuilder) -> Result<Window, anyhow::Error> {
        let hwnd: HWND;
        unsafe {
            let instance = GetModuleHandleW(None)?;

            handle_error("Failed to get module handle", ExpectedError::Win32, || {
                instance.0 == 0
            })?;

            let wc = WNDCLASSW {
                hInstance: instance.into(),
                lpszClassName: self.windowprops.lpclassname,
                lpfnWndProc: Some(builder.window_proc),
                ..self.classprops
            };

            handle_error(
                "Failed to register window class",
                ExpectedError::Win32,
                || RegisterClassW(&wc) == 0,
            )?;

            handle_error("No window procedure set", ExpectedError::Other, || {
                wc.lpfnWndProc.is_none()
            })?;

            hwnd = CreateWindowExW(
                self.windowprops.dwexstyle,
                self.windowprops.lpclassname,
                self.windowprops.lpwindowname,
                self.windowprops.dwstyle,
                self.windowprops.x,
                self.windowprops.y,
                self.windowprops.nwidth,
                self.windowprops.nheight,
                self.windowprops.hwndparent,
                self.windowprops.hmenu,
                self.windowprops.hinstance,
                self.windowprops.lpparam,
            );

            handle_error("Window creation failed", ExpectedError::Win32, || {
                hwnd.0 == 0
            })?;
        };
        Ok(Window {
            hwnd,
            window_type: builder.window_type,
        })
    }
}

pub trait WindowFactory {
    fn create_window(&self, builder: &WindowBuilder) -> Result<Window, anyhow::Error>;
}

pub struct TransparentWindowFactory;
pub struct OpaqueWindowFactory;

impl WindowFactory for TransparentWindowFactory {
    fn create_window(&self, builder: &WindowBuilder) -> Result<Window, anyhow::Error> {
        let window;
        unsafe {
            let mut template = WindowTemplate::new();

            template.windowprops = WINDOWPROPS {
                lpclassname: w!("TransparentWindowClass"),
                lpwindowname: w!("TransparentWindow"),
                dwexstyle: WS_EX_LAYERED | WS_EX_COMPOSITED,
                dwstyle: WS_POPUP,
                x: GetSystemMetrics(SM_XVIRTUALSCREEN),
                y: GetSystemMetrics(SM_YVIRTUALSCREEN),
                nwidth: GetSystemMetrics(SM_CXVIRTUALSCREEN),
                nheight: GetSystemMetrics(SM_CYVIRTUALSCREEN),
                ..Default::default()
            };

            template.classprops = WNDCLASSW {
                style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
                hbrBackground: CreateSolidBrush(COLORREF(0x000000)),
                hCursor: LoadCursorW(None, IDC_CROSS)?,
                lpfnWndProc: Some(builder.window_proc),
                lpszClassName: template.windowprops.lpclassname,
                ..Default::default()
            };

            window = template.create_window(builder)?;

            handle_error(
                "Changing Window Attributes failed",
                ExpectedError::Win32,
                || window.make_transparent().is_err(),
            )?;
        }
        Ok(window)
    }
}

impl WindowFactory for OpaqueWindowFactory {
    fn create_window(&self, builder: &WindowBuilder) -> Result<Window, anyhow::Error> {
        let res;
        unsafe {
            let mut template = WindowTemplate::new();

            template.windowprops = WINDOWPROPS {
                lpclassname: w!("OpaqueWindowClass"),
                lpwindowname: w!("OpaqueWindow"),
                dwexstyle: WS_EX_COMPOSITED,
                dwstyle: WS_POPUP,
                x: GetSystemMetrics(SM_XVIRTUALSCREEN),
                y: GetSystemMetrics(SM_YVIRTUALSCREEN),
                nwidth: GetSystemMetrics(SM_CXVIRTUALSCREEN),
                nheight: GetSystemMetrics(SM_CYVIRTUALSCREEN),
                ..Default::default()
            };

            template.classprops = WNDCLASSW {
                style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                lpszClassName: template.windowprops.lpclassname,
                lpfnWndProc: Some(builder.window_proc),
                ..Default::default()
            };

            res = template.create_window(builder)?;
        }
        Ok(res)
    }
}

pub struct WindowBuilder {
    window_proc: unsafe extern "system" fn(
        param0: HWND,
        param1: u32,
        param2: WPARAM,
        param3: LPARAM,
    ) -> LRESULT,

    window_type: WindowType,
}

#[derive(Clone, Copy)]
pub enum WindowType {
    Transparent,
    Opaque,
    Main,
    None,
}

unsafe extern "system" fn default_window_proc(_: HWND, _: u32, _: WPARAM, _: LPARAM) -> LRESULT {
    LRESULT(0)
}
impl WindowBuilder {
    pub fn new() -> WindowBuilder {
        WindowBuilder {
            window_proc: default_window_proc,
            window_type: WindowType::None,
        }
    }

    pub fn set_window_type(&mut self, window_type: WindowType) -> &mut Self {
        self.window_type = window_type;
        self
    }

    pub fn set_window_proc(
        &mut self,
        window_proc: unsafe extern "system" fn(
            param0: HWND,
            param1: u32,
            param2: WPARAM,
            param3: LPARAM,
        ) -> LRESULT,
    ) -> &mut Self {
        self.window_proc = window_proc;
        self
    }

    pub fn build(&self) -> Result<Window, anyhow::Error> {
        let factory: Box<dyn WindowFactory> = match self.window_type {
            WindowType::Transparent => Box::new(TransparentWindowFactory),
            WindowType::Opaque => Box::new(OpaqueWindowFactory),
            WindowType::Main => return throw_error("Main window not implemented"),
            _ => return throw_error("No window type set"),
        };

        let window = factory.create_window(self)?;

        Ok(window)
    }
}
