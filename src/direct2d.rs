use crate::errorhandler::{handle_error, ExpectedError};
use anyhow::Result;
use windows::Win32::Graphics::Imaging::{CLSID_WICImagingFactory, IWICImagingFactory};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::{
    core::*,
    Foundation::Numerics::*,
    Win32::Foundation::*,
    Win32::Graphics::Direct2D::Common::D2D_SIZE_U,
    Win32::Graphics::Direct2D::Common::*,
    Win32::Graphics::Direct2D::Common::*,
    Win32::Graphics::Direct2D::*,
    Win32::Graphics::Direct3D::*,
    Win32::Graphics::Direct3D11::*,
    Win32::Graphics::Direct3D11::*,
    Win32::Graphics::Dxgi::Common::*,
    Win32::Graphics::Dxgi::*,
    Win32::Graphics::Gdi::*,
    Win32::Graphics::Imaging::*,
    Win32::Graphics::*,
    Win32::System::Com::*,
    Win32::System::DataExchange::*,
    Win32::System::LibraryLoader::*,
    Win32::System::Performance::*,
    Win32::System::SystemInformation::GetLocalTime,
    Win32::UI::Animation::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::{
        Foundation::*, Graphics::Direct2D::Common::*, Graphics::Direct2D::*,
        Graphics::Dxgi::Common::*, Graphics::Imaging::*, System::Com::*,
        UI::WindowsAndMessaging::GetSystemMetrics,
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

pub fn render_screen_img(win: HWND) -> std::result::Result<(), Box<dyn std::error::Error>> {
    unsafe {
        //initialize:
        handle_error("Delete DC error", ExpectedError::Win32, || {
            CoInitializeEx(None, COINIT_MULTITHREADED).is_err()
        })?;

        let hr: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

        // Capture screen to bitmap
        let hb_desktop = capture_screen_to_bitmap()?;

        // Create a WIC bitmap from the HBITMAP
        let bitmap =
            hr.CreateBitmapFromHBITMAP(hb_desktop, None, WICBitmapAlphaChannelOption(0))?;

        let mut width: u32 = 0;
        let mut height: u32 = 0;
        bitmap.GetSize(&mut width, &mut height)?;

        // get format converter to build a bitmap:
        let converter = hr.CreateFormatConverter()?;
        converter.Initialize(
            &bitmap,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeMedianCut,
        )?;

        //get Direct2D Factroy necessary to render target:
        let factory: ID2D1Factory = D2D1CreateFactory(
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            Some(&D2D1_FACTORY_OPTIONS::default()),
        )?;
        let prop1 = D2D1_RENDER_TARGET_PROPERTIES {
            dpiX: Default::default(),
            dpiY: Default::default(),
            ..Default::default()
        };
        let prop2 = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: win, //win is the parameter of this function providing a window to draw on
            pixelSize: D2D_SIZE_U { width, height },
            ..Default::default()
        };
        let rwt = factory.CreateHwndRenderTarget(&prop1, &prop2)?;

        let bmps =
            rwt.CreateBitmapFromWicBitmap(&converter, Some(&D2D1_BITMAP_PROPERTIES::default()))?;

        let srect = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: width as _,
            bottom: height as _,
        };
        rwt.BeginDraw();

        rwt.DrawBitmap(
            &bmps,
            Some(&srect),
            1.0,
            D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
            Some(&srect),
        );

        rwt.EndDraw(None, None)?;
    };
    Ok(())
}
