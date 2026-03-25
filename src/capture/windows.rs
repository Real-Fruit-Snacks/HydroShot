use super::{CaptureError, CapturedScreen, ScreenCapture};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

pub struct WindowsCapturer;

impl WindowsCapturer {
    pub fn new() -> Self {
        Self
    }
}

impl ScreenCapture for WindowsCapturer {
    fn capture_all_screens(&self) -> Result<Vec<CapturedScreen>, CaptureError> {
        unsafe { capture_virtual_desktop() }.map(|screen| vec![screen])
    }
}

unsafe fn capture_virtual_desktop() -> Result<CapturedScreen, CaptureError> {
    let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

    if width <= 0 || height <= 0 {
        return Err(CaptureError::NoDisplay);
    }

    let hdc_screen = unsafe { GetDC(None) };
    if hdc_screen.is_invalid() {
        return Err(CaptureError::PlatformError(
            "Failed to get screen DC".to_string(),
        ));
    }

    let hdc_mem = unsafe { CreateCompatibleDC(hdc_screen) };
    if hdc_mem.is_invalid() {
        unsafe { ReleaseDC(None, hdc_screen) };
        return Err(CaptureError::PlatformError(
            "Failed to create compatible DC".to_string(),
        ));
    }

    let hbm = unsafe { CreateCompatibleBitmap(hdc_screen, width, height) };
    if hbm.is_invalid() {
        unsafe {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);
        }
        return Err(CaptureError::PlatformError(
            "Failed to create bitmap".to_string(),
        ));
    }

    let old_bm = unsafe { SelectObject(hdc_mem, hbm) };

    let blt_result = unsafe { BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, x, y, SRCCOPY) };
    if blt_result.is_err() {
        unsafe {
            SelectObject(hdc_mem, old_bm);
            let _ = DeleteObject(hbm);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);
        }
        return Err(CaptureError::PlatformError("BitBlt failed".to_string()));
    }

    // Extract pixel data via GetDIBits
    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [Default::default()],
    };

    let buf_size = (width * height * 4) as usize;
    let mut bgra_pixels: Vec<u8> = vec![0u8; buf_size];

    let lines = unsafe {
        GetDIBits(
            hdc_mem,
            hbm,
            0,
            height as u32,
            Some(bgra_pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };

    // Clean up GDI resources
    unsafe {
        SelectObject(hdc_mem, old_bm);
        let _ = DeleteObject(hbm);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);
    }

    if lines == 0 {
        return Err(CaptureError::PlatformError(
            "GetDIBits returned 0 lines".to_string(),
        ));
    }

    // Convert BGRA -> RGBA
    for chunk in bgra_pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2); // swap B and R
    }

    Ok(CapturedScreen {
        pixels: bgra_pixels,
        width: width as u32,
        height: height as u32,
        x_offset: x,
        y_offset: y,
        scale_factor: 1.0,
    })
}
