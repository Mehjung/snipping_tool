use crate::errorhandler::{handle_error, ExpectedError};
use anyhow::Result;

use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::{
        Direct2D::{
            Common::{D2D1_COLOR_F, D2D_POINT_2F, D2D_RECT_F, D2D_SIZE_U},
            D2D1CreateFactory, ID2D1Bitmap, ID2D1Factory, ID2D1HwndRenderTarget,
            D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR, D2D1_BITMAP_PROPERTIES,
            D2D1_FACTORY_OPTIONS, D2D1_FACTORY_TYPE_SINGLE_THREADED,
            D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_PROPERTIES,
        },
        Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, GetDC, ReleaseDC,
            SelectObject, HBITMAP, SRCCOPY,
        },
        Imaging::{
            CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICBitmap,
            IWICFormatConverter, IWICImagingFactory, WICBitmapAlphaChannelOption,
            WICBitmapDitherTypeNone, WICBitmapPaletteTypeMedianCut,
        },
    },
    System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED},
    UI::WindowsAndMessaging::{
        GetClientRect, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    },
};

pub fn capture_screen_to_bitmap() -> Result<HBITMAP> {
    unsafe {
        let hdc = GetDC(HWND(0)); // Get the desktop device context
        handle_error("DC error", ExpectedError::Win32, || hdc.0 == 0)?;

        let h_dest = CreateCompatibleDC(hdc); // Create a device context to use yourself
        handle_error("Comb DC error", ExpectedError::Win32, || h_dest.0 == 0)?;

        // Get the virtual screen coordinates and dimensions
        let x_left = GetSystemMetrics(SM_XVIRTUALSCREEN); // Left position of the virtual screen
        let y_top = GetSystemMetrics(SM_YVIRTUALSCREEN); // Top position of the virtual screen
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN); // Width of the virtual screen
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN); // Height of the virtual screen

        let hb_desktop = CreateCompatibleBitmap(hdc, width, height); // Create a bitmap compatible with the screen device context
        handle_error("Create Bitmap error", ExpectedError::Win32, || {
            hb_desktop.0 == 0
        })?;

        SelectObject(h_dest, hb_desktop); // Select the bitmap into the device context
        BitBlt(h_dest, 0, 0, width, height, hdc, x_left, y_top, SRCCOPY)?; // Capture the screen with offset consideration

        // Cleanup resources
        ReleaseDC(HWND(0), hdc);
        handle_error("Delete DC error", ExpectedError::Win32, || {
            DeleteDC(h_dest).as_bool() == false
        })?;

        Ok(hb_desktop) // Return the handle to the bitmap containing the screenshot
    }
}

fn initialize_com_library() -> Result<(), anyhow::Error> {
    handle_error(
        "Failed to initialize COM library",
        ExpectedError::Win32,
        || unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).is_err() },
    )
}

fn create_wic_imaging_factory() -> Result<IWICImagingFactory, anyhow::Error> {
    let res = unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) };
    res.map_err(|_| anyhow::anyhow!("Failed to create WIC Imaging Factory"))
}
pub fn render_screen_img(win: HWND) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize COM library
    let _ = initialize_com_library();

    let hr = create_wic_imaging_factory()?;
    // Capture screen to bitmap
    let hb_desktop = capture_screen_to_bitmap()?;

    let BitmapConverter {
        converter,
        width,
        height,
        ..
    } = BitmapConverter::new(&hr, hb_desktop)?;

    let Direct2DFactory { rwt, .. } = Direct2DFactory::new(win, None, None)?;

    let bmps = unsafe {
        rwt.CreateBitmapFromWicBitmap(&converter, Some(&D2D1_BITMAP_PROPERTIES::default()))?
    };

    let renderer = Renderer::new(rwt, width, height);

    renderer.draw_bitmap(&bmps)?;

    Ok(())
}

pub fn draw_rectangle(
    win: HWND,
    start: D2D_POINT_2F,
    end: D2D_POINT_2F,
) -> Result<(), anyhow::Error> {
    let Direct2DFactory { rwt, .. } = Direct2DFactory::new(win, None, None)?;
    let renderer = Renderer::new(rwt, 0, 0);

    renderer.draw_rectangle(start, end)
}

struct Direct2DFactory {
    factory: ID2D1Factory,
    prop1: D2D1_RENDER_TARGET_PROPERTIES,
    prop2: D2D1_HWND_RENDER_TARGET_PROPERTIES,
    rwt: ID2D1HwndRenderTarget,
}

impl Direct2DFactory {
    fn new(win: HWND, width: Option<u32>, height: Option<u32>) -> Result<Self, anyhow::Error> {
        let factory: ID2D1Factory = unsafe {
            D2D1CreateFactory(
                D2D1_FACTORY_TYPE_SINGLE_THREADED,
                Some(&D2D1_FACTORY_OPTIONS::default()),
            )?
        };

        let (width, height) = match (width, height) {
            (Some(w), Some(h)) => (w, h),
            _ => {
                // Abfrage der Fenstergröße
                let mut rect = RECT::default();
                unsafe {
                    let _ = GetClientRect(win, &mut rect)?;
                }
                (
                    (rect.right - rect.left) as u32,
                    (rect.bottom - rect.top) as u32,
                )
            }
        };

        let prop1 = D2D1_RENDER_TARGET_PROPERTIES::default();
        let prop2 = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: win,
            pixelSize: D2D_SIZE_U { width, height },
            ..Default::default()
        };

        let rwt = unsafe { factory.CreateHwndRenderTarget(&prop1, &prop2)? };

        Ok(Self {
            factory,
            prop1,
            prop2,
            rwt,
        })
    }
}

struct BitmapConverter {
    bitmap: IWICBitmap,
    converter: IWICFormatConverter,
    width: u32,
    height: u32,
}

impl BitmapConverter {
    fn new(hr: &IWICImagingFactory, hb_desktop: HBITMAP) -> Result<Self, anyhow::Error> {
        let bitmap = unsafe {
            hr.CreateBitmapFromHBITMAP(hb_desktop, None, WICBitmapAlphaChannelOption(0))?
        };

        let (mut width, mut height) = (0u32, 0u32);
        unsafe { bitmap.GetSize(&mut width, &mut height)? };

        let converter = unsafe { hr.CreateFormatConverter()? };
        unsafe {
            converter.Initialize(
                &bitmap,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeMedianCut,
            )?
        };

        Ok(Self {
            bitmap,
            converter,
            width,
            height,
        })
    }
}

struct Renderer {
    rwt: ID2D1HwndRenderTarget,
    srect: D2D_RECT_F,
}

impl Renderer {
    fn new(rwt: ID2D1HwndRenderTarget, width: u32, height: u32) -> Self {
        let srect = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: width as _,
            bottom: height as _,
        };

        Self { rwt, srect }
    }

    fn draw_bitmap(&self, bmps: &ID2D1Bitmap) -> Result<(), anyhow::Error> {
        unsafe { self.rwt.BeginDraw() };

        unsafe {
            self.rwt.DrawBitmap(
                bmps,
                Some(&self.srect),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
                Some(&self.srect),
            )
        };

        unsafe { self.rwt.EndDraw(None, None)? };

        Ok(())
    }
    fn draw_rectangle(&self, start: D2D_POINT_2F, end: D2D_POINT_2F) -> Result<(), anyhow::Error> {
        unsafe { self.rwt.BeginDraw() };

        let rect = D2D_RECT_F {
            left: start.x.min(end.x),
            top: start.y.min(end.y),
            right: start.x.max(end.x),
            bottom: start.y.max(end.y),
        };

        let color = D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };

        let brush = unsafe { self.rwt.CreateSolidColorBrush(&color, None)? };

        unsafe {
            self.rwt.DrawRectangle(&rect, &brush, 1.0, None);
            self.rwt.EndDraw(None, None)?;
        }

        Ok(())
    }
}
