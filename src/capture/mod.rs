#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod x11;

#[cfg(target_os = "linux")]
mod wayland;

/// A captured screen's pixel data and metadata.
#[derive(Debug, Clone)]
pub struct CapturedScreen {
    /// RGBA8 pixels, row-major order.
    pub pixels: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// X offset of this screen in virtual-desktop coordinates.
    pub x_offset: i32,
    /// Y offset of this screen in virtual-desktop coordinates.
    pub y_offset: i32,
    /// Display scale factor (e.g. 1.0, 1.25, 1.5, 2.0).
    pub scale_factor: f64,
}

/// Errors that can occur during screen capture.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("Permission denied")]
    PermissionDenied,
    #[error("No display available")]
    NoDisplay,
    #[error("Platform error: {0}")]
    PlatformError(String),
}

/// Trait for platform-specific screen capture implementations.
pub trait ScreenCapture {
    /// Capture all connected screens and return their pixel data.
    fn capture_all_screens(&self) -> Result<Vec<CapturedScreen>, CaptureError>;
}

/// Create a platform-appropriate screen capturer.
pub fn create_capturer() -> Result<Box<dyn ScreenCapture>, CaptureError> {
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsCapturer::new()))
    }

    #[cfg(target_os = "linux")]
    {
        // TODO: detect Wayland vs X11 at runtime
        Ok(Box::new(x11::X11Capturer::new()?))
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err(CaptureError::PlatformError(
            "Unsupported platform".to_string(),
        ))
    }
}
