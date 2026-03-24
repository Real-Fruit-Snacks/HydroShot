// X11 screen capture implementation (Linux only).
//
// This module is only compiled on Linux targets. It will use x11rb to
// capture the root window. Currently a stub awaiting full implementation.

use super::{CaptureError, CapturedScreen, ScreenCapture};

pub struct X11Capturer;

impl X11Capturer {
    pub fn new() -> Result<Self, CaptureError> {
        Err(CaptureError::PlatformError(
            "X11 capture not yet implemented".to_string(),
        ))
    }
}

impl ScreenCapture for X11Capturer {
    fn capture_all_screens(&self) -> Result<Vec<CapturedScreen>, CaptureError> {
        Err(CaptureError::PlatformError(
            "X11 capture not yet implemented".to_string(),
        ))
    }
}
