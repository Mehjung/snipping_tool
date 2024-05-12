use crate::{
    direct2d::{draw_rectangle, draw_updated_area, render_screen_img},
    errorhandler::{handle_error, throw_error, ExpectedError},
};
use std::{os::raw::c_void, time::Instant};
use windows::{
    core::{w, Error, PCWSTR},
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Direct2D::Common::D2D_POINT_2F,
        Graphics::Gdi::{
            CreateSolidBrush, RedrawWindow, RDW_ERASE, RDW_INVALIDATE, RDW_NOINTERNALPAINT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics, LoadCursorW,
            RegisterClassW, SetForegroundWindow, SetLayeredWindowAttributes, SetWindowPos,
            ShowWindow, CS_HREDRAW, CS_OWNDC, CS_VREDRAW, CW_USEDEFAULT, HMENU, HWND_TOPMOST,
            IDC_ARROW, IDC_CROSS, LWA_ALPHA, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
            SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOW,
            WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW, WS_EX_COMPOSITED, WS_EX_LAYERED, WS_POPUP,
        },
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

    pub fn hide(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn draw_updated_area(&self) {
        draw_updated_area(self.hwnd);
    }
    pub fn reload(&self) {
        self.hide();
        self.show();
    }

    pub fn draw_rectangle(&self, start: D2D_POINT_2F, end: D2D_POINT_2F) {
        draw_rectangle(self.hwnd, start, end);
    }

    pub fn auto_screenshot(&self) {
        let start = Instant::now();
        let _ = render_screen_img(self.hwnd);
        let duration = start.elapsed(); // Get the elapsed time

        println!("Time elapsed in render_screen_img is: {:?}", duration);
    }

    pub fn trigger_screenshot(&self) {
        unsafe {
            println!("Trigger Screenshot");
            let res = RedrawWindow(
                self.hwnd,
                None,
                None,
                RDW_INVALIDATE | RDW_ERASE | RDW_NOINTERNALPAINT,
            );
            match res.as_bool() {
                false => eprintln!("Error in RedrawWindow"),
                _ => println!("RedrawWindow successful"),
            }
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
#[derive(Debug)]
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
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
            nwidth: CW_USEDEFAULT,
            nheight: CW_USEDEFAULT,
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

unsafe extern "system" fn default_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Hier werden keine spezifischen Nachrichten behandelt. Stattdessen wird alles an DefWindowProcW weitergeleitet.
    DefWindowProcW(hwnd, msg, wparam, lparam)
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
