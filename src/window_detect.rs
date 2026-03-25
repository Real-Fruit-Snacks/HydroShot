//! Window detection for "Capture Window" mode.
//!
//! Enumerates all visible, non-minimised windows and returns their screen-space
//! rectangles in front-to-back Z-order.  The overlay window is excluded by
//! filtering out windows whose title is "HydroShot".

/// A screen-space rectangle: (x, y, width, height).
pub type WinRect = (i32, i32, i32, i32);

/// Return visible window rects in Z-order (front-to-back).
#[cfg(target_os = "windows")]
pub fn enumerate_window_rects() -> Vec<WinRect> {
    use windows::Win32::Foundation::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let rects = &mut *(lparam.0 as *mut Vec<WinRect>);

        unsafe {
            if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
                return BOOL(1);
            }

            // Skip our own overlay window
            let mut buf = [0u16; 64];
            let len = GetWindowTextW(hwnd, &mut buf) as usize;
            if len > 0 {
                let title = String::from_utf16_lossy(&buf[..len]);
                if title == "HydroShot" {
                    return BOOL(1);
                }
            }

            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;
                // Skip tiny/invisible windows
                if w > 1 && h > 1 {
                    rects.push((rect.left, rect.top, w, h));
                }
            }
        }

        BOOL(1) // continue enumeration
    }

    let mut rects: Vec<WinRect> = Vec::new();

    unsafe {
        let _ = EnumWindows(
            Some(enum_callback),
            LPARAM(&mut rects as *mut Vec<WinRect> as isize),
        );
    }

    rects
}

/// Find the front-most window rect that contains the given screen-space point.
pub fn window_at_point(rects: &[WinRect], screen_x: i32, screen_y: i32) -> Option<WinRect> {
    rects.iter().copied().find(|&(wx, wy, ww, wh)| {
        screen_x >= wx && screen_x < wx + ww && screen_y >= wy && screen_y < wy + wh
    })
}

#[cfg(not(target_os = "windows"))]
pub fn enumerate_window_rects() -> Vec<WinRect> {
    Vec::new()
}
