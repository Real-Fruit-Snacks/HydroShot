# HydroShot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build HydroShot, a Flameshot-inspired screenshot capture and annotation tool for Windows and Linux.

**Architecture:** System tray daemon using winit's event loop. On capture, a fullscreen borderless window is created. Rendering is done entirely with tiny-skia (CPU), displayed via softbuffer. Window is destroyed after export/cancel. Single rendering path for both interactive display and export.

**Tech Stack:** Rust 2021 edition, winit (windowing), tiny-skia (rendering), softbuffer (pixel display), tray-icon (system tray), arboard (clipboard), rfd (file dialogs), image (PNG encoding), tracing (logging), windows crate (Win32 capture), x11rb (X11 capture)

**Spec:** `docs/superpowers/specs/2026-03-24-hydroshot-design.md`

**Deferred to post-MVP:**
- Multi-monitor (enumerate monitors, per-monitor overlay) — MVP captures primary monitor only
- Wayland overlay — MVP supports X11 only on Linux
- Single-instance enforcement
- Error notifications (tray balloon tips) — errors logged to stderr for MVP
- About dialog

---

## File Structure

```
hydroshot/
├── Cargo.toml
├── assets/
│   └── icon.png                   # Tray icon (32x32 PNG, bundled via include_bytes!)
├── src/
│   ├── main.rs                    # Entry point: winit event loop, tray + overlay orchestration
│   ├── lib.rs                     # Crate root, module declarations
│   ├── tray.rs                    # Tray icon creation, menu setup
│   ├── state.rs                   # AppState enum, OverlayState struct
│   ├── renderer.rs                # Compose full frame: screenshot, dim, selection, annotations, toolbar
│   ├── geometry.rs                # Point, Size, Color + tiny-skia conversions
│   ├── capture/
│   │   ├── mod.rs                 # ScreenCapture trait, CapturedScreen, CaptureError
│   │   ├── windows.rs             # Win32 BitBlt implementation
│   │   ├── x11.rs                 # X11 root window capture
│   │   └── wayland.rs             # Stub
│   ├── overlay/
│   │   ├── mod.rs                 # Re-exports
│   │   ├── selection.rs           # Selection rectangle: create, move, resize, hit-testing
│   │   └── toolbar.rs             # Toolbar positioning, hit-testing, button layout
│   ├── tools/
│   │   ├── mod.rs                 # AnnotationTool trait, Annotation enum, render_annotation()
│   │   ├── arrow.rs               # ArrowTool
│   │   └── rectangle.rs           # RectangleTool
│   └── export.rs                  # Crop, offset annotations, flatten, clipboard, file save
├── tests/
│   ├── geometry_tests.rs
│   ├── tools_tests.rs
│   ├── selection_tests.rs
│   └── export_tests.rs
├── examples/
│   └── capture_test.rs            # Smoke test for screen capture
```

---

## Task 0: Project Scaffold & Spike

Validate that winit + tray-icon + softbuffer + tiny-skia work together. This is the core integration.

**Files:**
- Create: `hydroshot/Cargo.toml`
- Create: `hydroshot/src/main.rs`
- Create: `hydroshot/src/lib.rs`
- Create: `hydroshot/assets/icon.png`

- [ ] **Step 1: Create project and Cargo.toml**

```bash
mkdir -p hydroshot/assets hydroshot/src
```

Create `hydroshot/Cargo.toml`:
```toml
[package]
name = "hydroshot"
version = "0.1.0"
edition = "2021"

[dependencies]
winit = "0.30"
softbuffer = "0.4"
tiny-skia = "0.11"
tray-icon = "0.19"
image = "0.25"
tracing = "0.1"
tracing-subscriber = "0.3"
```

Create `hydroshot/src/lib.rs`:
```rust
pub mod geometry;
```

- [ ] **Step 2: Create a placeholder 32x32 tray icon PNG**

```bash
cd hydroshot && magick convert -size 32x32 xc:dodgerblue assets/icon.png
```

Or create any valid 32x32 PNG.

- [ ] **Step 3: Write the spike**

Create `hydroshot/src/main.rs`:

```rust
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use tray_icon::{TrayIconBuilder, Icon, menu::{Menu, MenuItem, MenuEvent}, TrayIconEvent};
use tracing::info;

fn main() {
    tracing_subscriber::fmt::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    // Create tray icon
    let icon_bytes = include_bytes!("../assets/icon.png");
    let icon_img = image::load_from_memory(icon_bytes).unwrap().to_rgba8();
    let (w, h) = icon_img.dimensions();
    let icon = Icon::from_rgba(icon_img.into_raw(), w, h).unwrap();

    let menu = Menu::new();
    let capture_item = MenuItem::new("Capture", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let capture_id = capture_item.id().clone();
    let quit_id = quit_item.id().clone();
    menu.append(&capture_item).unwrap();
    menu.append(&quit_item).unwrap();

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("HydroShot")
        .with_icon(icon)
        .build()
        .unwrap();

    let mut overlay_window: Option<winit::window::Window> = None;

    event_loop.run(move |event, elwt| {
        // Check tray events
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == capture_id {
                info!("Capture triggered!");
                // Create a test window
                let window = WindowBuilder::new()
                    .with_title("HydroShot Overlay")
                    .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
                    .build(elwt)
                    .unwrap();
                overlay_window = Some(window);
            } else if event.id == quit_id {
                info!("Quit");
                elwt.exit();
            }
        }

        if let Ok(TrayIconEvent::Click { .. }) = TrayIconEvent::receiver().try_recv() {
            info!("Tray left-click — capture!");
        }

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                overlay_window = None;
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event: key_event, .. }, ..
            } => {
                use winit::keyboard::{Key, NamedKey};
                if key_event.state.is_pressed() {
                    if let Key::Named(NamedKey::Escape) = key_event.logical_key {
                        info!("Escape — closing overlay");
                        overlay_window = None;
                    }
                }
            }
            _ => {}
        }
    }).unwrap();
}
```

- [ ] **Step 4: Build and run**

```bash
cd hydroshot && cargo run
```

Expected: Tray icon appears. Right-click → Capture opens a window. Esc closes it. Right-click → Quit exits. Left-click logs "Tray left-click".

- [ ] **Step 5: Verify and document findings**

Note any platform-specific quirks with tray-icon + winit interaction. Verify `TrayIconEvent::Click` variant name matches the tray-icon 0.19 API — adjust if different.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: project scaffold and winit+tray-icon integration spike"
```

---

## Task 1: Geometry Types

**Files:**
- Create: `hydroshot/src/geometry.rs`
- Create: `hydroshot/tests/geometry_tests.rs`
- Modify: `hydroshot/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `hydroshot/tests/geometry_tests.rs`:

```rust
use hydroshot::geometry::{Point, Size, Color};

#[test]
fn test_point_creation() {
    let p = Point::new(10.0, 20.0);
    assert_eq!(p.x, 10.0);
    assert_eq!(p.y, 20.0);
}

#[test]
fn test_size_creation() {
    let s = Size::new(100.0, 200.0);
    assert_eq!(s.width, 100.0);
    assert_eq!(s.height, 200.0);
}

#[test]
fn test_color_red() {
    let c = Color::red();
    assert_eq!(c.r, 1.0);
    assert_eq!(c.g, 0.0);
    assert_eq!(c.b, 0.0);
    assert_eq!(c.a, 1.0);
}

#[test]
fn test_color_presets() {
    let colors = Color::presets();
    assert!(colors.len() >= 4);
    assert_eq!(colors[0], Color::red());
}

#[test]
fn test_color_to_tiny_skia() {
    let c = Color::new(1.0, 0.0, 0.0, 1.0);
    let skia_color: tiny_skia::Color = c.into();
    assert_eq!(skia_color.red(), 1.0);
    assert_eq!(skia_color.green(), 0.0);
}
```

- [ ] **Step 2: Run tests to verify failure**

```bash
cd hydroshot && cargo test --test geometry_tests
```

Expected: Compilation error.

- [ ] **Step 3: Implement geometry.rs**

Create `hydroshot/src/geometry.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point { pub x: f32, pub y: f32 }

impl Point {
    pub fn new(x: f32, y: f32) -> Self { Self { x, y } }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size { pub width: f32, pub height: f32 }

impl Size {
    pub fn new(width: f32, height: f32) -> Self { Self { width, height } }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self { Self { r, g, b, a } }
    pub fn red() -> Self { Self::new(1.0, 0.0, 0.0, 1.0) }
    pub fn blue() -> Self { Self::new(0.0, 0.4, 1.0, 1.0) }
    pub fn green() -> Self { Self::new(0.0, 0.8, 0.0, 1.0) }
    pub fn yellow() -> Self { Self::new(1.0, 0.9, 0.0, 1.0) }
    pub fn white() -> Self { Self::new(1.0, 1.0, 1.0, 1.0) }
    pub fn presets() -> Vec<Self> {
        vec![Self::red(), Self::blue(), Self::green(), Self::yellow(), Self::white()]
    }
}

impl From<Color> for tiny_skia::Color {
    fn from(c: Color) -> Self {
        tiny_skia::Color::from_rgba(c.r, c.g, c.b, c.a).unwrap_or(tiny_skia::Color::BLACK)
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cd hydroshot && cargo test --test geometry_tests
```

Expected: All 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/geometry.rs src/lib.rs tests/geometry_tests.rs
git commit -m "feat: geometry types with tiny-skia conversions"
```

---

## Task 2: Screen Capture Trait & Windows Implementation

**Files:**
- Create: `hydroshot/src/capture/mod.rs`
- Create: `hydroshot/src/capture/windows.rs`
- Create: `hydroshot/src/capture/x11.rs`
- Create: `hydroshot/src/capture/wayland.rs`
- Create: `hydroshot/examples/capture_test.rs`
- Modify: `hydroshot/src/lib.rs`, `hydroshot/Cargo.toml`

- [ ] **Step 1: Add platform dependencies**

Add to `Cargo.toml`:
```toml
thiserror = "2"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Graphics_Gdi",
    "Win32_UI_WindowsAndMessaging",
] }

[target.'cfg(target_os = "linux")'.dependencies]
x11rb = { version = "0.13", features = ["image"] }
```

- [ ] **Step 2: Create trait and types**

Create `hydroshot/src/capture/mod.rs`:

```rust
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod x11;

#[derive(Debug)]
pub struct CapturedScreen {
    pub pixels: Vec<u8>,       // RGBA8, row-major
    pub width: u32,
    pub height: u32,
    pub x_offset: i32,
    pub y_offset: i32,
    pub scale_factor: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("Permission denied")]
    PermissionDenied,
    #[error("No display available")]
    NoDisplay,
    #[error("Platform error: {0}")]
    PlatformError(String),
}

pub trait ScreenCapture {
    fn capture_all_screens(&self) -> Result<Vec<CapturedScreen>, CaptureError>;
}

pub fn create_capturer() -> Box<dyn ScreenCapture> {
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsCapture) }
    #[cfg(target_os = "linux")]
    { Box::new(x11::X11Capture) }
}
```

- [ ] **Step 3: Implement Windows capture**

Create `hydroshot/src/capture/windows.rs`:

```rust
use super::{CaptureError, CapturedScreen, ScreenCapture};

pub struct WindowsCapture;

impl ScreenCapture for WindowsCapture {
    fn capture_all_screens(&self) -> Result<Vec<CapturedScreen>, CaptureError> {
        use windows::Win32::Graphics::Gdi::*;
        use windows::Win32::UI::WindowsAndMessaging::*;

        unsafe {
            let screen_dc = GetDC(None);
            if screen_dc.is_invalid() {
                return Err(CaptureError::NoDisplay);
            }

            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);
            let mem_dc = CreateCompatibleDC(screen_dc);
            let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
            let old_bitmap = SelectObject(mem_dc, bitmap);

            if BitBlt(mem_dc, 0, 0, width, height, screen_dc, 0, 0, SRCCOPY).is_err() {
                ReleaseDC(None, screen_dc);
                return Err(CaptureError::PlatformError("BitBlt failed".into()));
            }

            let mut buffer = vec![0u8; (width * height) as usize * 4];
            let bitmap_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width, biHeight: -height, biPlanes: 1,
                    biBitCount: 32, biCompression: BI_RGB.0, ..Default::default()
                },
                ..Default::default()
            };

            GetDIBits(mem_dc, bitmap, 0, height as u32,
                Some(buffer.as_mut_ptr() as *mut _),
                &bitmap_info as *const _ as *mut _, DIB_RGB_COLORS);

            for chunk in buffer.chunks_exact_mut(4) { chunk.swap(0, 2); } // BGRA → RGBA

            SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);

            Ok(vec![CapturedScreen {
                pixels: buffer, width: width as u32, height: height as u32,
                x_offset: 0, y_offset: 0, scale_factor: 1.0,
            }])
        }
    }
}
```

- [ ] **Step 4: Create X11 and Wayland stubs**

Create `hydroshot/src/capture/x11.rs`:
```rust
use super::{CaptureError, CapturedScreen, ScreenCapture};

pub struct X11Capture;

impl ScreenCapture for X11Capture {
    fn capture_all_screens(&self) -> Result<Vec<CapturedScreen>, CaptureError> {
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::*;

        let (conn, screen_num) = x11rb::connect(None)
            .map_err(|e| CaptureError::PlatformError(e.to_string()))?;
        let screen = &conn.setup().roots[screen_num];
        let (w, h) = (screen.width_in_pixels, screen.height_in_pixels);

        let image = conn.get_image(ImageFormat::Z_PIXMAP, screen.root, 0, 0, w, h, !0)
            .map_err(|e| CaptureError::PlatformError(e.to_string()))?
            .reply()
            .map_err(|e| CaptureError::PlatformError(e.to_string()))?;

        let mut pixels = image.data;
        for chunk in pixels.chunks_exact_mut(4) { chunk.swap(0, 2); }

        Ok(vec![CapturedScreen {
            pixels, width: w as u32, height: h as u32,
            x_offset: 0, y_offset: 0, scale_factor: 1.0,
        }])
    }
}
```

Create `hydroshot/src/capture/wayland.rs`:
```rust
// Wayland capture — deferred to post-MVP. See spec Section 9.
```

- [ ] **Step 5: Smoke test**

Create `hydroshot/examples/capture_test.rs`:
```rust
fn main() {
    let capturer = hydroshot::capture::create_capturer();
    match capturer.capture_all_screens() {
        Ok(screens) => {
            for (i, s) in screens.iter().enumerate() {
                println!("Screen {}: {}x{}", i, s.width, s.height);
                let img = image::RgbaImage::from_raw(s.width, s.height, s.pixels.clone()).unwrap();
                img.save(format!("capture_test_{}.png", i)).unwrap();
                println!("Saved capture_test_{}.png", i);
            }
        }
        Err(e) => eprintln!("Failed: {}", e),
    }
}
```

- [ ] **Step 6: Build and run**

```bash
cd hydroshot && cargo run --example capture_test
```

Expected: Saves a valid screenshot PNG.

- [ ] **Step 7: Commit**

```bash
git add src/capture/ src/lib.rs examples/ Cargo.toml
git commit -m "feat: screen capture trait with Windows and X11 implementations"
```

---

## Task 3: Annotation Tools & Shared Renderer

**Files:**
- Create: `hydroshot/src/tools/mod.rs`
- Create: `hydroshot/src/tools/rectangle.rs`
- Create: `hydroshot/src/tools/arrow.rs`
- Create: `hydroshot/tests/tools_tests.rs`
- Modify: `hydroshot/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `hydroshot/tests/tools_tests.rs`:

```rust
use hydroshot::geometry::{Point, Color};
use hydroshot::tools::{Annotation, AnnotationTool};
use hydroshot::tools::rectangle::RectangleTool;
use hydroshot::tools::arrow::ArrowTool;

#[test]
fn test_rectangle_produces_annotation() {
    let mut tool = RectangleTool::new(Color::red(), 3.0);
    tool.on_mouse_down(Point::new(10.0, 10.0));
    let ann = tool.on_mouse_up(Point::new(100.0, 80.0)).unwrap();
    match ann {
        Annotation::Rectangle { top_left, size, .. } => {
            assert_eq!(top_left.x, 10.0);
            assert_eq!(size.width, 90.0);
            assert_eq!(size.height, 70.0);
        }
        _ => panic!("Expected Rectangle"),
    }
}

#[test]
fn test_rectangle_normalizes() {
    let mut tool = RectangleTool::new(Color::red(), 3.0);
    tool.on_mouse_down(Point::new(100.0, 80.0));
    let ann = tool.on_mouse_up(Point::new(10.0, 10.0)).unwrap();
    match ann {
        Annotation::Rectangle { top_left, .. } => {
            assert_eq!(top_left.x, 10.0);
            assert_eq!(top_left.y, 10.0);
        }
        _ => panic!(),
    }
}

#[test]
fn test_arrow_produces_annotation() {
    let mut tool = ArrowTool::new(Color::red(), 3.0);
    tool.on_mouse_down(Point::new(10.0, 10.0));
    let ann = tool.on_mouse_up(Point::new(100.0, 80.0)).unwrap();
    match ann {
        Annotation::Arrow { start, end, .. } => {
            assert_eq!(start, Point::new(10.0, 10.0));
            assert_eq!(end, Point::new(100.0, 80.0));
        }
        _ => panic!(),
    }
}

#[test]
fn test_no_annotation_without_mousedown() {
    let mut tool = RectangleTool::new(Color::red(), 3.0);
    assert!(tool.on_mouse_up(Point::new(100.0, 80.0)).is_none());
}

#[test]
fn test_in_progress_annotation() {
    let mut tool = ArrowTool::new(Color::red(), 3.0);
    assert!(tool.in_progress_annotation().is_none());
    tool.on_mouse_down(Point::new(0.0, 0.0));
    tool.on_mouse_move(Point::new(50.0, 50.0));
    let preview = tool.in_progress_annotation().unwrap();
    match preview {
        Annotation::Arrow { end, .. } => assert_eq!(end, Point::new(50.0, 50.0)),
        _ => panic!(),
    }
}

#[test]
fn test_thickness_clamping() {
    let mut tool = RectangleTool::new(Color::red(), 3.0);
    tool.set_thickness(0.0);
    tool.on_mouse_down(Point::new(0.0, 0.0));
    match tool.on_mouse_up(Point::new(10.0, 10.0)).unwrap() {
        Annotation::Rectangle { thickness, .. } => assert_eq!(thickness, 1.0),
        _ => panic!(),
    }
}

#[test]
fn test_arrowhead_points() {
    use hydroshot::tools::arrow::arrowhead_points;
    let pts = arrowhead_points(Point::new(0.0, 0.0), Point::new(100.0, 0.0), 3.0);
    assert_eq!(pts.len(), 2);
    assert!(pts[0].x < 100.0);
    assert!(pts[0].y > 0.0);
    assert!(pts[1].y < 0.0);
}

#[test]
fn test_render_annotation_rect() {
    use hydroshot::tools::render_annotation;
    let mut pixmap = tiny_skia::Pixmap::new(100, 100).unwrap();
    let ann = Annotation::Rectangle {
        top_left: Point::new(10.0, 10.0),
        size: hydroshot::geometry::Size::new(80.0, 80.0),
        color: Color::red(),
        thickness: 3.0,
    };
    render_annotation(&mut pixmap, &ann);
    // Pixel at (10, 10) should no longer be transparent
    let px = pixmap.pixel(10, 10).unwrap();
    assert!(px.alpha() > 0);
}
```

- [ ] **Step 2: Run tests to verify failure**

```bash
cd hydroshot && cargo test --test tools_tests
```

- [ ] **Step 3: Implement tools/mod.rs with shared render function**

Create `hydroshot/src/tools/mod.rs`:

```rust
pub mod arrow;
pub mod rectangle;

use crate::geometry::{Color, Point, Size};

#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    Arrow { start: Point, end: Point, color: Color, thickness: f32 },
    Rectangle { top_left: Point, size: Size, color: Color, thickness: f32 },
}

pub trait AnnotationTool {
    fn on_mouse_down(&mut self, pos: Point);
    fn on_mouse_move(&mut self, pos: Point);
    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation>;
    fn is_drawing(&self) -> bool;
    fn in_progress_annotation(&self) -> Option<Annotation>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolKind { Arrow, Rectangle }

/// Single rendering function used for BOTH interactive display and export.
/// Draws an annotation onto a tiny-skia Pixmap.
pub fn render_annotation(pixmap: &mut tiny_skia::Pixmap, annotation: &Annotation) {
    match annotation {
        Annotation::Rectangle { top_left, size, color, thickness } => {
            let mut paint = tiny_skia::Paint::default();
            paint.set_color((*color).into());
            paint.anti_alias = true;
            let mut stroke = tiny_skia::Stroke::default();
            stroke.width = *thickness;

            if let Some(rect) = tiny_skia::Rect::from_xywh(
                top_left.x, top_left.y, size.width, size.height
            ) {
                let path = tiny_skia::PathBuilder::from_rect(rect);
                pixmap.stroke_path(&path, &paint, &stroke,
                    tiny_skia::Transform::identity(), None);
            }
        }
        Annotation::Arrow { start, end, color, thickness } => {
            let mut paint = tiny_skia::Paint::default();
            paint.set_color((*color).into());
            paint.anti_alias = true;
            let mut stroke = tiny_skia::Stroke::default();
            stroke.width = *thickness;

            // Shaft
            let mut pb = tiny_skia::PathBuilder::new();
            pb.move_to(start.x, start.y);
            pb.line_to(end.x, end.y);
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &stroke,
                    tiny_skia::Transform::identity(), None);
            }

            // Arrowhead (filled triangle)
            let wings = arrow::arrowhead_points(*start, *end, *thickness);
            if wings.len() == 2 {
                let mut pb = tiny_skia::PathBuilder::new();
                pb.move_to(end.x, end.y);
                pb.line_to(wings[0].x, wings[0].y);
                pb.line_to(wings[1].x, wings[1].y);
                pb.close();
                if let Some(path) = pb.finish() {
                    pixmap.fill_path(&path, &paint,
                        tiny_skia::FillRule::Winding, tiny_skia::Transform::identity(), None);
                }
            }
        }
    }
}
```

- [ ] **Step 4: Implement RectangleTool**

Create `hydroshot/src/tools/rectangle.rs`:

```rust
use crate::geometry::{Color, Point, Size};
use super::{Annotation, AnnotationTool};

pub struct RectangleTool {
    color: Color, thickness: f32,
    start: Option<Point>, current: Option<Point>,
}

impl RectangleTool {
    pub fn new(color: Color, thickness: f32) -> Self {
        Self { color, thickness: thickness.clamp(1.0, 20.0), start: None, current: None }
    }
    pub fn set_color(&mut self, color: Color) { self.color = color; }
    pub fn set_thickness(&mut self, t: f32) { self.thickness = t.clamp(1.0, 20.0); }

    pub fn normalize(a: Point, b: Point) -> (Point, Size) {
        (Point::new(a.x.min(b.x), a.y.min(b.y)),
         Size::new((a.x - b.x).abs(), (a.y - b.y).abs()))
    }
}

impl AnnotationTool for RectangleTool {
    fn on_mouse_down(&mut self, pos: Point) { self.start = Some(pos); self.current = Some(pos); }
    fn on_mouse_move(&mut self, pos: Point) { if self.start.is_some() { self.current = Some(pos); } }
    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation> {
        let start = self.start.take()?;
        self.current = None;
        let (tl, sz) = Self::normalize(start, pos);
        Some(Annotation::Rectangle { top_left: tl, size: sz, color: self.color, thickness: self.thickness })
    }
    fn is_drawing(&self) -> bool { self.start.is_some() }
    fn in_progress_annotation(&self) -> Option<Annotation> {
        let (tl, sz) = Self::normalize(self.start?, self.current?);
        Some(Annotation::Rectangle { top_left: tl, size: sz, color: self.color, thickness: self.thickness })
    }
}
```

- [ ] **Step 5: Implement ArrowTool**

Create `hydroshot/src/tools/arrow.rs`:

```rust
use crate::geometry::{Color, Point};
use super::{Annotation, AnnotationTool};

pub fn arrowhead_points(start: Point, end: Point, thickness: f32) -> Vec<Point> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 { return vec![end, end]; }
    let arrow_len = thickness * 4.0;
    let half_angle: f32 = 30.0_f32.to_radians();
    let (ux, uy) = (-dx / len, -dy / len);
    let (cos_a, sin_a) = (half_angle.cos(), half_angle.sin());
    vec![
        Point::new(end.x + arrow_len * (ux*cos_a - uy*sin_a), end.y + arrow_len * (ux*sin_a + uy*cos_a)),
        Point::new(end.x + arrow_len * (ux*cos_a + uy*sin_a), end.y + arrow_len * (-ux*sin_a + uy*cos_a)),
    ]
}

pub struct ArrowTool {
    color: Color, thickness: f32,
    start: Option<Point>, current: Option<Point>,
}

impl ArrowTool {
    pub fn new(color: Color, thickness: f32) -> Self {
        Self { color, thickness: thickness.clamp(1.0, 20.0), start: None, current: None }
    }
    pub fn set_color(&mut self, color: Color) { self.color = color; }
    pub fn set_thickness(&mut self, t: f32) { self.thickness = t.clamp(1.0, 20.0); }
}

impl AnnotationTool for ArrowTool {
    fn on_mouse_down(&mut self, pos: Point) { self.start = Some(pos); self.current = Some(pos); }
    fn on_mouse_move(&mut self, pos: Point) { if self.start.is_some() { self.current = Some(pos); } }
    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation> {
        let start = self.start.take()?;
        self.current = None;
        Some(Annotation::Arrow { start, end: pos, color: self.color, thickness: self.thickness })
    }
    fn is_drawing(&self) -> bool { self.start.is_some() }
    fn in_progress_annotation(&self) -> Option<Annotation> {
        Some(Annotation::Arrow { start: self.start?, end: self.current?, color: self.color, thickness: self.thickness })
    }
}
```

- [ ] **Step 6: Run tests**

```bash
cd hydroshot && cargo test --test tools_tests
```

Expected: All 8 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/tools/ tests/tools_tests.rs src/lib.rs
git commit -m "feat: annotation tools with shared tiny-skia render_annotation"
```

---

## Task 4: Selection Rectangle

**Files:**
- Create: `hydroshot/src/overlay/mod.rs`
- Create: `hydroshot/src/overlay/selection.rs`
- Create: `hydroshot/src/overlay/toolbar.rs`
- Create: `hydroshot/tests/selection_tests.rs`
- Modify: `hydroshot/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `hydroshot/tests/selection_tests.rs`:

```rust
use hydroshot::geometry::Point;
use hydroshot::overlay::selection::{Selection, HitZone};

#[test]
fn test_from_points() {
    let s = Selection::from_points(Point::new(10.0, 10.0), Point::new(110.0, 80.0));
    assert_eq!((s.x, s.y, s.width, s.height), (10.0, 10.0, 100.0, 70.0));
}

#[test]
fn test_normalizes_reverse() {
    let s = Selection::from_points(Point::new(110.0, 80.0), Point::new(10.0, 10.0));
    assert_eq!((s.x, s.y), (10.0, 10.0));
}

#[test]
fn test_hit_inside() {
    let s = Selection { x: 100.0, y: 100.0, width: 200.0, height: 150.0 };
    assert_eq!(s.hit_test(Point::new(200.0, 175.0), 8.0), Some(HitZone::Inside));
}

#[test]
fn test_hit_outside() {
    let s = Selection { x: 100.0, y: 100.0, width: 200.0, height: 150.0 };
    assert_eq!(s.hit_test(Point::new(50.0, 50.0), 8.0), None);
}

#[test]
fn test_hit_corner() {
    let s = Selection { x: 100.0, y: 100.0, width: 200.0, height: 150.0 };
    assert_eq!(s.hit_test(Point::new(103.0, 103.0), 8.0), Some(HitZone::TopLeft));
}

#[test]
fn test_move() {
    let mut s = Selection { x: 100.0, y: 100.0, width: 200.0, height: 150.0 };
    s.move_by(10.0, -5.0);
    assert_eq!((s.x, s.y, s.width), (110.0, 95.0, 200.0));
}
```

- [ ] **Step 2: Run, verify failure, implement**

Implement `Selection` and `HitZone` in `hydroshot/src/overlay/selection.rs` (same logic as before — `from_points`, `contains`, `hit_test`, `move_by`, `resize`).

Create `hydroshot/src/overlay/mod.rs`:
```rust
pub mod selection;
pub mod toolbar;
```

Create `hydroshot/src/overlay/toolbar.rs` with `Toolbar` struct (same as before — `position_for`, `hit_test`, `button_rect`).

- [ ] **Step 3: Run tests, verify pass**

```bash
cd hydroshot && cargo test --test selection_tests
```

- [ ] **Step 4: Commit**

```bash
git add src/overlay/ tests/selection_tests.rs src/lib.rs
git commit -m "feat: selection rectangle and toolbar positioning"
```

---

## Task 5: Export Module

**Files:**
- Create: `hydroshot/src/export.rs`
- Create: `hydroshot/tests/export_tests.rs`
- Modify: `hydroshot/src/lib.rs`, `hydroshot/Cargo.toml`

- [ ] **Step 1: Add dependencies**

```toml
arboard = { version = "3", features = ["image-data"] }
rfd = "0.15"
chrono = "0.4"
```

- [ ] **Step 2: Write failing tests**

Create `hydroshot/tests/export_tests.rs`:

```rust
use hydroshot::geometry::{Point, Size, Color};
use hydroshot::tools::Annotation;
use hydroshot::export::flatten_annotations;

#[test]
fn test_flatten_empty_preserves() {
    let pixels: Vec<u8> = (0..10*10).flat_map(|_| [255, 0, 0, 255]).collect();
    let result = flatten_annotations(&pixels, 10, 10, &[]);
    assert_eq!(&result[0..4], &[255, 0, 0, 255]);
}

#[test]
fn test_flatten_with_rect_modifies() {
    let pixels: Vec<u8> = (0..100*100).flat_map(|_| [255, 255, 255, 255]).collect();
    let annotations = vec![Annotation::Rectangle {
        top_left: Point::new(10.0, 10.0),
        size: Size::new(80.0, 80.0),
        color: Color::red(), thickness: 3.0,
    }];
    let result = flatten_annotations(&pixels, 100, 100, &annotations);
    assert_ne!(result, pixels);
}
```

- [ ] **Step 3: Implement export.rs**

Create `hydroshot/src/export.rs`:

```rust
use crate::geometry::Point;
use crate::tools::{Annotation, render_annotation};

/// Flatten annotations onto pixel buffer. Input & output: straight RGBA8.
pub fn flatten_annotations(pixels: &[u8], width: u32, height: u32, annotations: &[Annotation]) -> Vec<u8> {
    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("Failed to create pixmap");
    // Copy pixels with premultiplication
    for (i, chunk) in pixels.chunks_exact(4).enumerate() {
        pixmap.pixels_mut()[i] = tiny_skia::PremultipliedColorU8::from_rgba(
            chunk[0], chunk[1], chunk[2], chunk[3]
        ).into();
    }

    for ann in annotations { render_annotation(&mut pixmap, ann); }

    // Demultiply back to straight alpha
    pixmap.pixels().iter().flat_map(|p| {
        let d = p.demultiply();
        [d.red(), d.green(), d.blue(), d.alpha()]
    }).collect()
}

/// Crop screenshot to selection, offset annotations, flatten.
pub fn crop_and_flatten(
    screenshot_pixels: &[u8], screenshot_width: u32,
    sel_x: u32, sel_y: u32, sel_w: u32, sel_h: u32,
    annotations: &[Annotation],
) -> Vec<u8> {
    let mut cropped = vec![0u8; (sel_w * sel_h * 4) as usize];
    for row in 0..sel_h {
        let src = ((sel_y + row) * screenshot_width + sel_x) as usize * 4;
        let dst = (row * sel_w) as usize * 4;
        let len = (sel_w * 4) as usize;
        if src + len <= screenshot_pixels.len() {
            cropped[dst..dst + len].copy_from_slice(&screenshot_pixels[src..src + len]);
        }
    }

    let offset_anns: Vec<Annotation> = annotations.iter().map(|a| {
        let dx = sel_x as f32;
        let dy = sel_y as f32;
        match a {
            Annotation::Arrow { start, end, color, thickness } => Annotation::Arrow {
                start: Point::new(start.x - dx, start.y - dy),
                end: Point::new(end.x - dx, end.y - dy),
                color: *color, thickness: *thickness,
            },
            Annotation::Rectangle { top_left, size, color, thickness } => Annotation::Rectangle {
                top_left: Point::new(top_left.x - dx, top_left.y - dy),
                size: *size, color: *color, thickness: *thickness,
            },
        }
    }).collect();

    flatten_annotations(&cropped, sel_w, sel_h, &offset_anns)
}

pub fn copy_to_clipboard(pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_image(ImageData {
        width: width as usize, height: height as usize,
        bytes: std::borrow::Cow::Borrowed(pixels),
    }).map_err(|e| e.to_string())
}

pub fn save_to_file(pixels: &[u8], width: u32, height: u32) -> Result<Option<String>, String> {
    let default_name = chrono::Local::now().format("hydroshot_%Y-%m-%d_%H%M%S.png").to_string();
    let path = rfd::FileDialog::new()
        .set_file_name(&default_name)
        .add_filter("PNG Image", &["png"])
        .save_file();
    match path {
        Some(p) => {
            let img = image::RgbaImage::from_raw(width, height, pixels.to_vec())
                .ok_or("Invalid image data")?;
            img.save(&p).map_err(|e| e.to_string())?;
            Ok(Some(p.to_string_lossy().to_string()))
        }
        None => Ok(None),
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cd hydroshot && cargo test --test export_tests
```

Expected: Both pass.

- [ ] **Step 5: Commit**

```bash
git add src/export.rs tests/export_tests.rs src/lib.rs Cargo.toml
git commit -m "feat: export — crop, flatten, clipboard, file save"
```

---

## Task 6: App State & Tray Module

**Files:**
- Create: `hydroshot/src/state.rs`
- Create: `hydroshot/src/tray.rs`
- Modify: `hydroshot/src/lib.rs`

- [ ] **Step 1: Implement state.rs**

```rust
use crate::capture::CapturedScreen;
use crate::geometry::{Color, Point};
use crate::overlay::selection::{Selection, HitZone};
use crate::tools::{Annotation, AnnotationTool, ToolKind};
use crate::tools::arrow::ArrowTool;
use crate::tools::rectangle::RectangleTool;

pub enum AppState {
    Idle,
    Capturing(OverlayState),
}

pub struct OverlayState {
    pub screenshot: CapturedScreen,
    pub selection: Option<Selection>,
    pub annotations: Vec<Annotation>,
    pub redo_buffer: Vec<Annotation>,
    pub active_tool: ToolKind,
    pub arrow_tool: ArrowTool,
    pub rectangle_tool: RectangleTool,
    pub current_color: Color,
    pub current_thickness: f32,
    pub is_selecting: bool,
    pub drag_start: Option<Point>,
    pub drag_zone: Option<HitZone>,
    pub last_mouse_pos: Point,
}

impl OverlayState {
    pub fn new(screenshot: CapturedScreen) -> Self {
        let color = Color::red();
        Self {
            screenshot, selection: None,
            annotations: Vec::new(), redo_buffer: Vec::new(),
            active_tool: ToolKind::Arrow,
            arrow_tool: ArrowTool::new(color, 3.0),
            rectangle_tool: RectangleTool::new(color, 3.0),
            current_color: color, current_thickness: 3.0,
            is_selecting: false, drag_start: None, drag_zone: None,
            last_mouse_pos: Point::new(0.0, 0.0),
        }
    }
}
```

- [ ] **Step 2: Implement tray.rs**

```rust
use tray_icon::{TrayIconBuilder, Icon, menu::{Menu, MenuItem}};

pub struct TrayState {
    pub capture_id: tray_icon::menu::MenuId,
    pub about_id: tray_icon::menu::MenuId,
    pub quit_id: tray_icon::menu::MenuId,
    pub _tray: tray_icon::TrayIcon,
}

pub fn create_tray() -> Result<TrayState, String> {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let icon_img = image::load_from_memory(icon_bytes).map_err(|e| e.to_string())?.to_rgba8();
    let (w, h) = icon_img.dimensions();
    let icon = Icon::from_rgba(icon_img.into_raw(), w, h).map_err(|e| e.to_string())?;

    let menu = Menu::new();
    let capture_item = MenuItem::new("Capture", true, None);
    let about_item = MenuItem::new("About", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let capture_id = capture_item.id().clone();
    let about_id = about_item.id().clone();
    let quit_id = quit_item.id().clone();
    menu.append(&capture_item).map_err(|e| e.to_string())?;
    menu.append(&about_item).map_err(|e| e.to_string())?;
    menu.append(&quit_item).map_err(|e| e.to_string())?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("HydroShot")
        .with_icon(icon)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(TrayState { capture_id, about_id, quit_id, _tray: tray })
}
```

- [ ] **Step 3: Verify compilation**

```bash
cd hydroshot && cargo build
```

- [ ] **Step 4: Commit**

```bash
git add src/state.rs src/tray.rs src/lib.rs
git commit -m "feat: app state and tray module"
```

---

## Task 7: Renderer

**Files:**
- Create: `hydroshot/src/renderer.rs`
- Modify: `hydroshot/src/lib.rs`

- [ ] **Step 1: Implement the full-frame renderer**

Create `hydroshot/src/renderer.rs`:

```rust
use crate::geometry::{Color, Point};
use crate::overlay::selection::Selection;
use crate::overlay::toolbar::Toolbar;
use crate::state::OverlayState;
use crate::tools::{render_annotation, Annotation, AnnotationTool, ToolKind};

/// Render the entire overlay frame into a tiny-skia Pixmap.
/// Called on every redraw (mouse move, key press, etc).
pub fn render_overlay(state: &OverlayState, pixmap: &mut tiny_skia::Pixmap) {
    let (w, h) = (pixmap.width(), pixmap.height());

    // 1. Draw screenshot as background
    let screenshot = &state.screenshot;
    for (i, chunk) in screenshot.pixels.chunks_exact(4).enumerate() {
        if i < pixmap.pixels().len() {
            pixmap.pixels_mut()[i] = tiny_skia::PremultipliedColorU8::from_rgba(
                chunk[0], chunk[1], chunk[2], chunk[3]
            ).into();
        }
    }

    // 2. Dim the entire screen
    let mut dim_paint = tiny_skia::Paint::default();
    dim_paint.set_color(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.4).unwrap());
    let dim_rect = tiny_skia::Rect::from_xywh(0.0, 0.0, w as f32, h as f32).unwrap();
    pixmap.fill_rect(dim_rect, &dim_paint, tiny_skia::Transform::identity(), None);

    // 3. If selection exists, restore brightness in selected area
    if let Some(sel) = &state.selection {
        // Redraw the screenshot pixels in the selection area (overwriting the dim)
        let sx = sel.x.max(0.0) as u32;
        let sy = sel.y.max(0.0) as u32;
        let sw = sel.width as u32;
        let sh = sel.height as u32;
        for row in 0..sh {
            for col in 0..sw {
                let px = sx + col;
                let py = sy + row;
                if px < w && py < h {
                    let src_idx = (py * screenshot.width + px) as usize;
                    let dst_idx = (py * w + px) as usize;
                    if src_idx * 4 + 3 < screenshot.pixels.len() && dst_idx < pixmap.pixels().len() {
                        let s = &screenshot.pixels[src_idx * 4..src_idx * 4 + 4];
                        pixmap.pixels_mut()[dst_idx] = tiny_skia::PremultipliedColorU8::from_rgba(
                            s[0], s[1], s[2], s[3]
                        ).into();
                    }
                }
            }
        }

        // 4. Selection border (white, 1px)
        let mut border_paint = tiny_skia::Paint::default();
        border_paint.set_color(tiny_skia::Color::WHITE);
        border_paint.anti_alias = true;
        let mut stroke = tiny_skia::Stroke::default();
        stroke.width = 1.0;
        if let Some(rect) = tiny_skia::Rect::from_xywh(sel.x, sel.y, sel.width, sel.height) {
            let path = tiny_skia::PathBuilder::from_rect(rect);
            pixmap.stroke_path(&path, &border_paint, &stroke,
                tiny_skia::Transform::identity(), None);
        }

        // 5. Draw finalized annotations
        for ann in &state.annotations {
            render_annotation(pixmap, ann);
        }

        // 6. Draw in-progress annotation preview
        let preview = match state.active_tool {
            ToolKind::Arrow => state.arrow_tool.in_progress_annotation(),
            ToolKind::Rectangle => state.rectangle_tool.in_progress_annotation(),
        };
        if let Some(ref ann) = preview {
            render_annotation(pixmap, ann);
        }

        // 7. Draw toolbar
        render_toolbar(pixmap, sel, h as f32, &state.active_tool, &state.current_color);
    }
}

fn render_toolbar(
    pixmap: &mut tiny_skia::Pixmap, sel: &Selection, screen_h: f32,
    active_tool: &ToolKind, current_color: &Color,
) {
    let toolbar = Toolbar::position_for(sel, screen_h);

    // Background
    let mut bg_paint = tiny_skia::Paint::default();
    bg_paint.set_color(tiny_skia::Color::from_rgba(0.15, 0.15, 0.15, 0.85).unwrap());
    if let Some(rect) = tiny_skia::Rect::from_xywh(toolbar.x, toolbar.y, toolbar.width, toolbar.height) {
        pixmap.fill_rect(rect, &bg_paint, tiny_skia::Transform::identity(), None);
    }

    // Buttons: 0=Arrow, 1=Rectangle, 2-6=color swatches, 7=Copy, 8=Save
    let color_presets = Color::presets();
    for i in 0..9 {
        let (bx, by, bw, bh) = toolbar.button_rect(i);

        // Button background
        let mut btn_paint = tiny_skia::Paint::default();
        let is_active = (i == 0 && *active_tool == ToolKind::Arrow)
            || (i == 1 && *active_tool == ToolKind::Rectangle)
            || (i >= 2 && i <= 6 && color_presets.get(i - 2) == Some(current_color));

        if is_active {
            btn_paint.set_color(tiny_skia::Color::from_rgba(0.3, 0.6, 1.0, 0.8).unwrap());
        } else {
            btn_paint.set_color(tiny_skia::Color::from_rgba(0.3, 0.3, 0.3, 0.8).unwrap());
        }

        if let Some(rect) = tiny_skia::Rect::from_xywh(bx, by, bw, bh) {
            pixmap.fill_rect(rect, &btn_paint, tiny_skia::Transform::identity(), None);
        }

        // Color swatch fill for buttons 2-6
        if i >= 2 && i <= 6 {
            if let Some(c) = color_presets.get(i - 2) {
                let mut swatch_paint = tiny_skia::Paint::default();
                swatch_paint.set_color((*c).into());
                let inset = 4.0;
                if let Some(rect) = tiny_skia::Rect::from_xywh(bx + inset, by + inset, bw - inset*2.0, bh - inset*2.0) {
                    pixmap.fill_rect(rect, &swatch_paint, tiny_skia::Transform::identity(), None);
                }
            }
        }

        // Tool icons: simple shapes for Arrow (line) and Rectangle (outline)
        if i == 0 {
            // Arrow icon: diagonal line with arrowhead
            let mut p = tiny_skia::Paint::default();
            p.set_color(tiny_skia::Color::WHITE);
            let mut s = tiny_skia::Stroke::default();
            s.width = 2.0;
            let mut pb = tiny_skia::PathBuilder::new();
            pb.move_to(bx + 6.0, by + bh - 6.0);
            pb.line_to(bx + bw - 6.0, by + 6.0);
            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &p, &s, tiny_skia::Transform::identity(), None);
            }
        } else if i == 1 {
            // Rectangle icon: small outlined rect
            let mut p = tiny_skia::Paint::default();
            p.set_color(tiny_skia::Color::WHITE);
            let mut s = tiny_skia::Stroke::default();
            s.width = 2.0;
            if let Some(rect) = tiny_skia::Rect::from_xywh(bx + 6.0, by + 6.0, bw - 12.0, bh - 12.0) {
                let path = tiny_skia::PathBuilder::from_rect(rect);
                pixmap.stroke_path(&path, &p, &s, tiny_skia::Transform::identity(), None);
            }
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd hydroshot && cargo build
```

- [ ] **Step 3: Commit**

```bash
git add src/renderer.rs src/lib.rs
git commit -m "feat: overlay renderer — screenshot, dim, selection, annotations, toolbar"
```

---

## Task 8: Main Event Loop — Wire Everything Together

**Files:**
- Modify: `hydroshot/src/main.rs`

- [ ] **Step 1: Implement the full event loop**

Rewrite `hydroshot/src/main.rs`:

```rust
use winit::event::{Event, WindowEvent, ElementState, MouseButton};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{WindowBuilder, Fullscreen};
use winit::keyboard::{Key, NamedKey, ModifiersState};
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;
use tracing::{info, error};

use hydroshot::capture;
use hydroshot::geometry::{Color, Point};
use hydroshot::overlay::selection::{Selection, HitZone};
use hydroshot::overlay::toolbar::Toolbar;
use hydroshot::state::{AppState, OverlayState};
use hydroshot::tools::{AnnotationTool, ToolKind};
use hydroshot::renderer::render_overlay;

fn main() {
    tracing_subscriber::fmt::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let tray = hydroshot::tray::create_tray().expect("Failed to create tray icon");

    let mut state = AppState::Idle;
    let mut overlay_window: Option<winit::window::Window> = None;
    let mut surface: Option<softbuffer::Surface<_, _>> = None; // simplified type
    let mut pixmap: Option<tiny_skia::Pixmap> = None;
    let mut modifiers = ModifiersState::empty();

    event_loop.run(move |event, elwt| {
        // --- Tray events ---
        if let Ok(menu_event) = MenuEvent::receiver().try_recv() {
            if menu_event.id == tray.capture_id {
                if matches!(state, AppState::Idle) {
                    trigger_capture(&mut state, &mut overlay_window, &mut surface, &mut pixmap, elwt);
                }
            } else if menu_event.id == tray.quit_id {
                elwt.exit();
            } else if menu_event.id == tray.about_id {
                info!("HydroShot v{}", env!("CARGO_PKG_VERSION"));
            }
        }

        if let Ok(TrayIconEvent::Click { .. }) = TrayIconEvent::receiver().try_recv() {
            if matches!(state, AppState::Idle) {
                trigger_capture(&mut state, &mut overlay_window, &mut surface, &mut pixmap, elwt);
            }
        }

        // --- Window events ---
        match event {
            Event::WindowEvent { event: window_event, .. } => {
                if let AppState::Capturing(ref mut overlay) = state {
                    match window_event {
                        WindowEvent::ModifiersChanged(new_mods) => {
                            modifiers = new_mods.state();
                        }
                        WindowEvent::KeyboardInput { event: key_event, .. }
                            if key_event.state == ElementState::Pressed =>
                        {
                            match key_event.logical_key {
                                Key::Named(NamedKey::Escape) => {
                                    close_overlay(&mut state, &mut overlay_window, &mut surface, &mut pixmap);
                                }
                                Key::Character(ref c) if modifiers.control_key() => {
                                    match c.as_str() {
                                        "c" => {
                                            if overlay.selection.is_some() {
                                                do_export_clipboard(overlay);
                                                close_overlay(&mut state, &mut overlay_window, &mut surface, &mut pixmap);
                                            }
                                        }
                                        "s" => {
                                            if overlay.selection.is_some() {
                                                let saved = do_export_save(overlay);
                                                if saved {
                                                    close_overlay(&mut state, &mut overlay_window, &mut surface, &mut pixmap);
                                                }
                                            }
                                        }
                                        "z" => {
                                            if modifiers.shift_key() {
                                                if let Some(a) = overlay.redo_buffer.pop() {
                                                    overlay.annotations.push(a);
                                                }
                                            } else {
                                                if let Some(a) = overlay.annotations.pop() {
                                                    overlay.redo_buffer.push(a);
                                                }
                                            }
                                            request_redraw(&overlay_window);
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {}
                            }
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            let pos = Point::new(position.x as f32, position.y as f32);
                            let dx = pos.x - overlay.last_mouse_pos.x;
                            let dy = pos.y - overlay.last_mouse_pos.y;
                            overlay.last_mouse_pos = pos;

                            if overlay.is_selecting {
                                if let Some(start) = overlay.drag_start {
                                    overlay.selection = Some(Selection::from_points(start, pos));
                                }
                            } else if let Some(zone) = overlay.drag_zone {
                                if let Some(ref mut sel) = overlay.selection {
                                    sel.resize(zone, dx, dy);
                                }
                            } else {
                                match overlay.active_tool {
                                    ToolKind::Arrow => overlay.arrow_tool.on_mouse_move(pos),
                                    ToolKind::Rectangle => overlay.rectangle_tool.on_mouse_move(pos),
                                }
                            }
                            request_redraw(&overlay_window);
                        }
                        WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                            let pos = overlay.last_mouse_pos;

                            // Toolbar hit?
                            if let Some(sel) = &overlay.selection {
                                let tb = Toolbar::position_for(sel, overlay.screenshot.height as f32);
                                if let Some(btn) = tb.hit_test(pos) {
                                    let colors = Color::presets();
                                    match btn {
                                        0 => overlay.active_tool = ToolKind::Arrow,
                                        1 => overlay.active_tool = ToolKind::Rectangle,
                                        2..=6 => {
                                            overlay.current_color = colors[btn - 2];
                                            overlay.arrow_tool.set_color(overlay.current_color);
                                            overlay.rectangle_tool.set_color(overlay.current_color);
                                        }
                                        7 => {
                                            do_export_clipboard(overlay);
                                            close_overlay(&mut state, &mut overlay_window, &mut surface, &mut pixmap);
                                            return;
                                        }
                                        8 => {
                                            if do_export_save(overlay) {
                                                close_overlay(&mut state, &mut overlay_window, &mut surface, &mut pixmap);
                                            }
                                            return;
                                        }
                                        _ => {}
                                    }
                                    request_redraw(&overlay_window);
                                    return;
                                }
                            }

                            if overlay.selection.is_none() {
                                overlay.is_selecting = true;
                                overlay.drag_start = Some(pos);
                            } else if let Some(ref sel) = overlay.selection {
                                match sel.hit_test(pos, 8.0) {
                                    Some(HitZone::Inside) => {
                                        match overlay.active_tool {
                                            ToolKind::Arrow => overlay.arrow_tool.on_mouse_down(pos),
                                            ToolKind::Rectangle => overlay.rectangle_tool.on_mouse_down(pos),
                                        }
                                    }
                                    Some(zone) => { overlay.drag_zone = Some(zone); }
                                    None => {
                                        // Click outside selection — start new selection
                                        overlay.selection = None;
                                        overlay.annotations.clear();
                                        overlay.redo_buffer.clear();
                                        overlay.is_selecting = true;
                                        overlay.drag_start = Some(pos);
                                    }
                                }
                            }
                        }
                        WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => {
                            let pos = overlay.last_mouse_pos;
                            if overlay.is_selecting {
                                overlay.is_selecting = false;
                                overlay.drag_start = None;
                            } else if overlay.drag_zone.is_some() {
                                overlay.drag_zone = None;
                            } else {
                                let ann = match overlay.active_tool {
                                    ToolKind::Arrow => overlay.arrow_tool.on_mouse_up(pos),
                                    ToolKind::Rectangle => overlay.rectangle_tool.on_mouse_up(pos),
                                };
                                if let Some(a) = ann {
                                    overlay.annotations.push(a);
                                    overlay.redo_buffer.clear();
                                }
                            }
                            request_redraw(&overlay_window);
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            let scroll = match delta {
                                winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                                winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 20.0,
                            };
                            overlay.current_thickness = (overlay.current_thickness + scroll).clamp(1.0, 20.0);
                            overlay.arrow_tool.set_thickness(overlay.current_thickness);
                            overlay.rectangle_tool.set_thickness(overlay.current_thickness);
                        }
                        _ => {}
                    }
                }
            }
            Event::AboutToWait => {
                // Render if we have an overlay
                if let (AppState::Capturing(ref overlay), Some(ref mut pm), Some(ref mut surf), Some(ref win)) =
                    (&state, &mut pixmap, &mut surface, &overlay_window)
                {
                    render_overlay(overlay, pm);
                    // Copy pixmap to softbuffer surface
                    let (w, h) = (pm.width(), pm.height());
                    surf.resize(
                        std::num::NonZeroU32::new(w).unwrap(),
                        std::num::NonZeroU32::new(h).unwrap(),
                    ).unwrap();
                    let mut buf = surf.buffer_mut().unwrap();
                    for (i, pixel) in pm.pixels().iter().enumerate() {
                        let d = pixel.demultiply();
                        buf[i] = ((d.alpha() as u32) << 24)
                            | ((d.red() as u32) << 16)
                            | ((d.green() as u32) << 8)
                            | (d.blue() as u32);
                    }
                    buf.present().unwrap();
                }
            }
            _ => {}
        }
    }).unwrap();
}

fn trigger_capture(
    state: &mut AppState,
    window: &mut Option<winit::window::Window>,
    surface: &mut Option<softbuffer::Surface<_, _>>,
    pixmap: &mut Option<tiny_skia::Pixmap>,
    elwt: &winit::event_loop::ActiveEventLoop,
) {
    let capturer = capture::create_capturer();
    match capturer.capture_all_screens() {
        Ok(mut screens) if !screens.is_empty() => {
            let screenshot = screens.remove(0);
            let (w, h) = (screenshot.width, screenshot.height);

            let win = WindowBuilder::new()
                .with_title("HydroShot")
                .with_fullscreen(Some(Fullscreen::Borderless(None)))
                .with_decorations(false)
                .build(elwt)
                .expect("Failed to create overlay window");

            let ctx = softbuffer::Context::new(&win).unwrap();
            let surf = softbuffer::Surface::new(&ctx, &win).unwrap();

            *pixmap = Some(tiny_skia::Pixmap::new(w, h).unwrap());
            *surface = Some(surf);
            *state = AppState::Capturing(OverlayState::new(screenshot));
            *window = Some(win);
        }
        Ok(_) => error!("No screens captured"),
        Err(e) => error!("Capture failed: {}", e),
    }
}

fn close_overlay(
    state: &mut AppState,
    window: &mut Option<winit::window::Window>,
    surface: &mut Option<softbuffer::Surface<_, _>>,
    pixmap: &mut Option<tiny_skia::Pixmap>,
) {
    *window = None; // drop the window
    *surface = None;
    *pixmap = None;
    *state = AppState::Idle;
}

fn request_redraw(window: &Option<winit::window::Window>) {
    if let Some(w) = window { w.request_redraw(); }
}

fn do_export_clipboard(overlay: &OverlayState) {
    if let Some(sel) = &overlay.selection {
        let pixels = hydroshot::export::crop_and_flatten(
            &overlay.screenshot.pixels, overlay.screenshot.width,
            sel.x as u32, sel.y as u32, sel.width as u32, sel.height as u32,
            &overlay.annotations,
        );
        if let Err(e) = hydroshot::export::copy_to_clipboard(&pixels, sel.width as u32, sel.height as u32) {
            error!("Clipboard error: {}", e);
        }
    }
}

fn do_export_save(overlay: &OverlayState) -> bool {
    if let Some(sel) = &overlay.selection {
        let pixels = hydroshot::export::crop_and_flatten(
            &overlay.screenshot.pixels, overlay.screenshot.width,
            sel.x as u32, sel.y as u32, sel.width as u32, sel.height as u32,
            &overlay.annotations,
        );
        match hydroshot::export::save_to_file(&pixels, sel.width as u32, sel.height as u32) {
            Ok(Some(_)) => return true,
            Ok(None) => {} // cancelled
            Err(e) => error!("Save error: {}", e),
        }
    }
    false
}
```

**Note:** The exact `softbuffer::Surface` generic types depend on the softbuffer version. The implementer should adjust type signatures to match. The core pattern is correct: create a surface from the window, resize it, write pixels, present.

- [ ] **Step 2: Build and test**

```bash
cd hydroshot && cargo run
```

Expected: Full working flow — tray icon → capture → overlay → select → annotate → export.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: main event loop — complete tray + overlay + annotation pipeline"
```

---

## Task 9: End-to-End Testing & Polish

- [ ] **Step 1: Full manual test**

1. Tray icon visible
2. Left-click → fullscreen overlay with dimmed screenshot
3. Click+drag selects region (brightens)
4. Toolbar appears with tool/color buttons
5. Arrow tool: draw arrows
6. Rectangle tool: draw rectangles
7. Color swatches change annotation color
8. Scroll wheel adjusts thickness
9. Ctrl+Z undoes, Ctrl+Shift+Z redoes
10. Ctrl+C copies to clipboard — paste to verify
11. Ctrl+S saves — open file to verify
12. Esc closes overlay, tray continues

- [ ] **Step 2: Fix bugs**

- [ ] **Step 3: Run all tests**

```bash
cd hydroshot && cargo test
```

- [ ] **Step 4: Add clippy and fmt**

```bash
cd hydroshot && cargo fmt && cargo clippy -- -D warnings
```

Fix any warnings.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat: HydroShot MVP — screenshot capture and annotation tool"
```

---

## Task Dependencies

```
Task 0: Spike (winit + tray-icon + softbuffer + tiny-skia)
  └── Task 1: Geometry types
       ├── Task 2: Screen capture ──┐
       ├── Task 3: Annotation tools ┤  (all parallel)
       ├── Task 4: Selection logic  ┤
       ├── Task 5: Export module ───┤
       └── Task 6: State + Tray ───┘
            └── Task 7: Renderer
                 └── Task 8: Main event loop (wires everything)
                      └── Task 9: Integration test & polish
```

Tasks 2–6 are fully independent and can run in parallel.
