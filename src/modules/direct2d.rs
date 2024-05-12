use crate::errorhandler::{handle_error, ExpectedError};

use anyhow::Result;
use core::*;

use windows::core::{Error, Interface};
use windows::Foundation::Numerics::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::{
    Foundation::{COLORREF, FALSE, HWND, RECT, TRUE},
    Graphics::{
        Direct2D::{
            Common::{
                D2D1_ALPHA_MODE_IGNORE, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F,
                D2D1_PIXEL_FORMAT, D2D_POINT_2F, D2D_RECT_F, D2D_SIZE_U,
            },
            D2D1CreateFactory, ID2D1Bitmap, ID2D1Bitmap1, ID2D1DeviceContext, ID2D1Factory,
            ID2D1Factory1, ID2D1HwndRenderTarget, ID2D1RenderTarget,
            D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
            D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_CPU_READ,
            D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES, D2D1_BITMAP_PROPERTIES1,
            D2D1_BRUSH_PROPERTIES, D2D1_DEVICE_CONTEXT_OPTIONS_ENABLE_MULTITHREADED_OPTIMIZATIONS,
            D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_OPTIONS,
            D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_HWND_RENDER_TARGET_PROPERTIES,
            D2D1_RENDER_TARGET_PROPERTIES, D2D1_UNIT_MODE_DIPS, D2D1_UNIT_MODE_PIXELS,
        },
        Dxgi::Common::*,
        Dxgi::*,
        Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, GetDC, ReleaseDC,
            SelectObject, HBITMAP, SRCCOPY,
        },
        Gdi::{CreateRectRgn, GetUpdateRect, SetWindowRgn, ValidateRect},
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

pub fn draw_updated_area(win: HWND) -> Result<(), anyhow::Error> {
    let Direct2DFactory { rwt, .. } = Direct2DFactory::new(win, None, None)?;
    let renderer = Renderer::new(rwt, win, 0, 0);

    renderer?.draw_updated_area(win)
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

    let renderer_result = Renderer::new(rwt, win, width, height);

    if let Ok(renderer) = renderer_result {
        renderer.draw_bitmap(&bmps);
    } else {
        // Handle error
    }

    Ok(())
}

pub fn draw_rectangle(
    win: HWND,
    start: D2D_POINT_2F,
    end: D2D_POINT_2F,
) -> Result<(), anyhow::Error> {
    let start_x = start.x.min(end.x);
    let start_y = start.y.min(end.y);
    let width = (start.x.max(end.x) - start_x) as u32;
    let height = (start.y.max(end.y) - start_y) as u32;

    let Direct2DFactory { rwt, .. } = Direct2DFactory::new(win, Some(width), Some(height))?;
    let renderer = Renderer::new(rwt, win, width, height);

    renderer?.draw_rectangle(start, end)
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
    swapchain: IDXGISwapChain1, // Hinzufügen der Swapchain
    nrwt: ID2D1RenderTarget,
    target: ID2D1DeviceContext,
}
trait Drawable {
    fn draw(&self, rwt: &ID2D1DeviceContext) -> Result<(), anyhow::Error>;
}

impl Renderer {
    fn new(
        rwt: ID2D1HwndRenderTarget,
        window: HWND,
        width: u32,
        height: u32,
    ) -> Result<Self, anyhow::Error> {
        let device = create_device()?;
        let swapchain = create_swapchain(&device, window)?;
        let surface = create_dxgi_surface(&swapchain)?;
        let fac = Direct2DFactory::new(window, Some(width), Some(height))?;
        let factory = fac.factory;
        let nrwt = create_d2d_render_target(&factory, &surface)?;

        let target = create_render_target(&factory.cast()?, &device)?;

        create_swapchain_bitmap(&swapchain, &target)?;

        let srect = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: width as f32,
            bottom: height as f32,
        };

        Ok(Self {
            rwt,
            nrwt,
            target,
            srect,
            swapchain: swapchain.clone(),
        })
    }
    fn draw_updated_area(&self, window: HWND) -> Result<(), anyhow::Error> {
        unsafe {
            let mut rect: RECT = mem::zeroed();
            GetUpdateRect(window, Some(&mut rect), FALSE);

            let invalidated_area = D2D_RECT_F {
                left: rect.left as f32,
                top: rect.top as f32,
                right: rect.right as f32,
                bottom: rect.bottom as f32,
            };

            self.target.BeginDraw();
            self.target
                .PushAxisAlignedClip(&invalidated_area, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            // Führen Sie hier Ihre Zeichenoperationen durch
            self.target.PopAxisAlignedClip();
            self.target.EndDraw(None, None)?;

            ValidateRect(window, Some(&mut rect));
        }

        Ok(())
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
        unsafe { self.target.BeginDraw() };

        let rectangle = Rectangle {
            start,
            end,
            color: D2D1_COLOR_F {
                r: 41.0,
                g: 41.0,
                b: 41.0,
                a: 0.5,
            }, // Standardfarbe, anpassbar
        };

        let col = D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 5.0,
        };

        let rect = D2D_RECT_F {
            left: start.x.min(end.x),
            top: start.y.min(end.y),
            right: start.x.max(end.x),
            bottom: start.y.max(end.y),
        };

        let trans = Matrix3x2::identity();
        let brushprops = D2D1_BRUSH_PROPERTIES {
            opacity: 1.0,
            transform: trans,
        };

        let b_ptr = &brushprops as *const D2D1_BRUSH_PROPERTIES;

        let brush = unsafe { self.target.CreateSolidColorBrush(&col, Some(b_ptr))? };

        let rect_ptr = &rect as *const D2D_RECT_F;
        unsafe { self.target.FillRectangle(rect_ptr, &brush) };
        rectangle.draw(&self.target)?;

        unsafe { self.target.EndDraw(None, None)? };
        let hr = unsafe { self.swapchain.Present(4, 0) };

        Ok(())
    }
}
struct Rectangle {
    start: D2D_POINT_2F,
    end: D2D_POINT_2F,
    color: D2D1_COLOR_F,
}

impl Drawable for Rectangle {
    fn draw(&self, rwt: &ID2D1DeviceContext) -> Result<(), anyhow::Error> {
        let rect = D2D_RECT_F {
            left: self.start.x.min(self.end.x),
            top: self.start.y.min(self.end.y),
            right: self.start.x.max(self.end.x),
            bottom: self.start.y.max(self.end.y),
        };

        let brush_props = D2D1_BRUSH_PROPERTIES {
            opacity: 1.0,

            transform: Default::default(),
        };

        let brush_props_ptr: *const D2D1_BRUSH_PROPERTIES = &brush_props;
        let brush = unsafe { rwt.CreateSolidColorBrush(&self.color, Some(brush_props_ptr))? };

        unsafe {
            rwt.DrawRectangle(&rect, &brush, 2.0, None);
        }

        Ok(())
    }
}

fn create_device_with_type(drive_type: D3D_DRIVER_TYPE) -> Result<ID3D11Device> {
    let mut flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;

    if cfg!(debug_assertions) {
        flags |= D3D11_CREATE_DEVICE_DEBUG;
    }

    let mut device = None;

    unsafe {
        Ok(D3D11CreateDevice(
            None,
            drive_type,
            None,
            flags,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )
        .map(|()| device.unwrap())?)
    }
}

fn create_device() -> Result<ID3D11Device> {
    let mut result = create_device_with_type(D3D_DRIVER_TYPE_HARDWARE);

    if let Err(err) = &result {
        if let Some(directx_error) = err.downcast_ref::<Error>() {
            if directx_error.code() == DXGI_ERROR_UNSUPPORTED {
                result = create_device_with_type(D3D_DRIVER_TYPE_WARP);
            }
        }
    }

    result
}

fn create_swapchain(device: &ID3D11Device, window: HWND) -> Result<IDXGISwapChain1> {
    let factory = get_dxgi_factory(device)?;

    let props = DXGI_SWAP_CHAIN_DESC1 {
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        AlphaMode: DXGI_ALPHA_MODE_IGNORE,
        ..Default::default()
    };

    unsafe { Ok(factory.CreateSwapChainForHwnd(device, window, &props, None, None)?) }
}

fn get_dxgi_factory(device: &ID3D11Device) -> Result<IDXGIFactory2> {
    let dxdevice = device.cast::<IDXGIDevice>()?;
    let adapter = unsafe { dxdevice.GetAdapter()? };
    let factory = unsafe { adapter.GetParent::<IDXGIFactory2>()? };
    Ok(factory)
}

fn create_dxgi_surface(swap_chain: &IDXGISwapChain1) -> Result<IDXGISurface, anyhow::Error> {
    let surface: IDXGISurface = unsafe { swap_chain.GetBuffer(0)? }; // Index 0 für den ersten Back Buffer
    Ok(surface)
}

fn create_d2d_render_target(
    factory: &ID2D1Factory,
    surface: &IDXGISurface,
) -> Result<ID2D1RenderTarget, anyhow::Error> {
    let props = D2D1_RENDER_TARGET_PROPERTIES {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },

        ..Default::default()
    };

    let render_target = unsafe { factory.CreateDxgiSurfaceRenderTarget(surface, &props)? };

    Ok(render_target)
}

fn create_swapchain_bitmap(swapchain: &IDXGISwapChain1, target: &ID2D1DeviceContext) -> Result<()> {
    let surface: IDXGISurface = unsafe { swapchain.GetBuffer(0)? };

    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_UNKNOWN,
            alphaMode: D2D1_ALPHA_MODE_IGNORE,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        ..Default::default()
    };

    unsafe {
        let bitmap = target.CreateBitmapFromDxgiSurface(&surface, Some(&props))?;
        target.SetTarget(&bitmap);
    };

    Ok(())
}

fn create_render_target(
    factory: &ID2D1Factory1,
    device: &ID3D11Device,
) -> Result<ID2D1DeviceContext> {
    unsafe {
        let d2device = factory.CreateDevice(&device.cast::<IDXGIDevice>()?)?;

        let target = d2device
            .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_ENABLE_MULTITHREADED_OPTIMIZATIONS)?;

        target.SetUnitMode(D2D1_UNIT_MODE_DIPS);

        Ok(target)
    }
}
