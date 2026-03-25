# HydroShot — Design Specification

**Date**: 2026-03-24
**Status**: Draft
**Platform**: Windows + Linux (X11 & Wayland)
**Language**: Rust
**Windowing**: winit
**Rendering**: tiny-skia (CPU software rendering)
**Display**: softbuffer (pixel buffer → window surface)
**Rust Edition**: 2021

---

## 1. Overview

HydroShot is a screenshot capture and annotation tool inspired by Flameshot. It runs as a system tray application, allowing users to capture a screen region, annotate it with arrows and rectangles, and export via clipboard or file save.

### MVP Scope

- System tray daemon with left-click capture and right-click menu
- Fullscreen overlay with region selection
- Two annotation tools: Arrow and Rectangle
- Export via Ctrl+C (clipboard) and Ctrl+S (file save)
- Single-instance enforcement
- Windows + Linux X11 support

### Out of Scope (Future)

- Global hotkeys
- Pencil, text, blur, circle, marker tools
- Pin-to-screen
- Imgur / cloud upload
- CLI interface
- Multi-monitor selection spanning displays
- Settings UI
- Full Wayland overlay support (see Section 9 — Known Limitations)

---

## 2. High-Level Architecture

HydroShot is structured as three layers:

### 2.1 System Tray Daemon

The always-running process that lives in the system tray.

- **Left-click**: trigger a capture
- **Right-click**: context menu (Capture, About, Quit)
- Uses the `tray-icon` crate, which integrates natively with winit's event loop

### 2.2 Screen Capture Engine

Platform-specific screen grabbing, abstracted behind a trait:

```rust
trait ScreenCapture {
    fn capture_all_screens(&self) -> Result<Vec<CapturedScreen>, CaptureError>;
}

struct CapturedScreen {
    /// Pixel data in RGBA8 format (4 bytes per pixel, row-major)
    pixels: Vec<u8>,
    /// Width in physical pixels
    width: u32,
    /// Height in physical pixels
    height: u32,
    /// Monitor X offset in physical pixels (for multi-monitor positioning)
    x_offset: i32,
    /// Monitor Y offset in physical pixels
    y_offset: i32,
    /// Display scale factor (1.0 = no scaling, 1.5 = 150%, 2.0 = HiDPI)
    scale_factor: f64,
}

enum CaptureError {
    PermissionDenied,
    NoDisplay,
    PlatformError(String),
}
```

All coordinates and pixel data operate in **physical pixels**. The `scale_factor` is stored per-screen so the overlay can render at the correct logical size.

| Platform | Implementation | Crate |
|----------|---------------|-------|
| Windows | Win32 Desktop Duplication / BitBlt | `windows` |
| Linux X11 | Root window capture | `x11rb` or `xcb` |
| Linux Wayland | xdg-desktop-portal screenshot portal | `ashpd` |

### 2.3 Overlay + Annotation UI

A fullscreen borderless winit window that displays the captured screenshot. Rendering is done entirely with `tiny-skia` (CPU software rasterizer), and the resulting pixel buffer is pushed to the window surface via `softbuffer`.

**Why winit + tiny-skia instead of a GUI framework:**
- Screenshot overlay windows are not normal GUI windows — they need precise fullscreen control, transparent overlays, and raw input handling. GUI frameworks (iced, egui) are designed for widget-based applications, not this use case.
- `tray-icon` is designed to integrate directly with winit's event loop — no channel hacks or threading needed.
- A single renderer (tiny-skia) is used for both interactive display and export, eliminating dual rendering paths.
- The overlay window is created only when capturing and destroyed afterward — no hidden window state machine.

---

## 3. Capture Flow

1. **Trigger** — user clicks the tray icon (left-click or right-click → Capture)
2. **Grab** — `ScreenCapture` captures all monitors as pixel buffers
3. **Overlay opens** — a new fullscreen borderless winit window is created, the screenshot is rendered as background with a dim/tint overlay via tiny-skia
4. **Region selection** — user clicks and drags to draw a selection rectangle. The selected area shows original brightness (undimmed), creating contrast. The selection can be moved by dragging inside it, and resized via corner/edge hit-test zones of 8 logical pixels (no visible handles, cursor changes to resize indicator on hover).
5. **Toolbar appears** — a small floating toolbar is rendered below the selection (or above if the selection is near the bottom of the screen). Contains: Arrow tool, Rectangle tool, color swatches, and action buttons (Save, Copy, Cancel).
6. **Annotation mode** — user clicks a tool and draws on the selected region. Annotations are stored as a stack of objects for undo/redo support.
7. **Export** — Ctrl+C: flatten annotations onto the cropped region, copy to clipboard. Ctrl+S: same but open a file save dialog (default filename: `hydroshot_YYYY-MM-DD_HHMMSS.png`, PNG only). If the user cancels the save dialog, the overlay remains open.
8. **Cancel** — Esc at any point destroys the overlay window, discarding everything. The tray daemon continues running.

### Multi-Monitor

For the MVP, one overlay window is created **per monitor**. Each overlay covers its respective display. Selection is constrained to a single monitor. The captured screenshot for each monitor is rendered as the background of its respective overlay window.

### DPI / HiDPI Scaling

All internal coordinates are in **physical pixels** to match the captured screenshot 1:1. The `scale_factor` from `CapturedScreen` is used when creating the overlay window so that winit reports correct logical sizes. Annotations are drawn and stored in physical pixel coordinates, ensuring they align correctly with the underlying screenshot regardless of display scaling.

---

## 4. Annotation Tools Architecture

### 4.1 Geometry Types

Annotations use custom geometry types:

```rust
/// Physical pixel coordinate
struct Point { x: f32, y: f32 }
struct Size { width: f32, height: f32 }
/// RGBA color, 0.0–1.0 per channel
struct Color { r: f32, g: f32, b: f32, a: f32 }
```

Conversion functions (`From`/`Into` impls) bridge these to `tiny_skia` equivalents.

### 4.2 Core Trait

```rust
trait AnnotationTool {
    fn on_mouse_down(&mut self, pos: Point);
    fn on_mouse_move(&mut self, pos: Point);
    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation>;
    /// Returns the in-progress annotation for preview rendering during drag
    fn in_progress_annotation(&self) -> Option<Annotation>;
}
```

### 4.3 Tool Lifecycle

Each tool struct holds **temporary state** during a drag operation (e.g., `start_point` after mouse down, current `end_point` during drag). While the user is dragging:

1. `on_mouse_down` — stores the start position, tool enters "drawing" state
2. `on_mouse_move` — updates the current end position
3. `on_mouse_up` — finalizes the shape, returns `Some(Annotation)` which is pushed onto the annotation stack. Tool returns to idle state.

**Rendering:** Both in-progress previews and finalized annotations use the same `render_annotation()` function that draws to a `tiny-skia::Pixmap`. There is only one rendering path — the same code used for interactive display is used for export. This eliminates the dual-renderer problem entirely.

### 4.4 MVP Tools

- **RectangleTool** — click to set one corner, drag to opposite corner, release to finalize. Renders as a colored outlined rectangle (no fill).
- **ArrowTool** — click for tail, drag to head, release to finalize. Renders as a line with an arrowhead. The arrowhead is an isosceles triangle with side length equal to 4x the stroke thickness, at a 30-degree half-angle from the shaft.

Both tools support configurable color and thickness.

### 4.5 Annotation Stack

Each completed drawing becomes an `Annotation` enum variant:

```rust
enum Annotation {
    Arrow { start: Point, end: Point, color: Color, thickness: f32 },
    Rectangle { top_left: Point, size: Size, color: Color, thickness: f32 },
}
```

- Stored in a `Vec<Annotation>` — the annotation stack
- **Undo** (Ctrl+Z): pop the last annotation into a redo buffer
- **Redo** (Ctrl+Shift+Z): push it back from the redo buffer
- Redo buffer clears when a new annotation is drawn

### 4.6 Color & Thickness

- Default color: red
- Small color picker with preset swatches in the toolbar
- Thickness adjustable via mouse scroll wheel while a tool is selected (range: 1px–20px, default 3px, increment 1px per scroll step)

### 4.7 Extensibility

Adding a new tool requires:
1. Implement `AnnotationTool` for the new tool struct
2. Add a variant to the `Annotation` enum
3. Add a match arm to `render_annotation()`
4. Add a toolbar button

No changes needed to capture, export, or windowing logic.

---

## 5. System Tray & Application Lifecycle

### 5.1 Startup

- HydroShot launches as a system tray application with no windows
- Single instance enforced:
  - **Windows**: named mutex
  - **Linux**: lockfile or Unix domain socket

### 5.2 Tray Behavior

- **Left-click**: immediately trigger a capture
- **Right-click menu**: Capture, About (shows version in a small dialog), Quit

### 5.3 Lifecycle States

1. **Idle** — tray icon visible, no windows, event loop running via winit
2. **Capturing** — overlay window exists, user selecting/annotating
3. **Idle** — overlay window destroyed after export or cancel

### 5.4 Process Architecture — winit + tray-icon Integration

**winit owns the event loop. tray-icon hooks into it directly.**

`tray-icon` is built to work with winit's event loop. On each iteration of the event loop, we check for tray menu events and tray icon click events. No channels, no background threads, no hacks.

```rust
event_loop.run(move |event, event_loop_window_target| {
    // Check for tray menu events on every loop iteration
    if let Ok(event) = MenuEvent::receiver().try_recv() {
        // handle Capture / About / Quit
    }
    if let Ok(event) = TrayIconEvent::receiver().try_recv() {
        // handle left-click → Capture
    }

    // Handle winit window events (mouse, keyboard, redraw)
    match event {
        Event::WindowEvent { .. } => { /* selection, annotation, keyboard */ }
        Event::RedrawRequested(_) => { /* re-render with tiny-skia, push to softbuffer */ }
        _ => {}
    }
});
```

**When a capture is triggered:**
1. Screen grab happens synchronously (milliseconds)
2. A new fullscreen borderless winit window is created
3. A `softbuffer::Surface` is attached to the window
4. A `tiny_skia::Pixmap` is created at the window's size — this is the render target
5. The render loop draws: screenshot → dim overlay → selection → annotations → toolbar
6. The pixmap's pixels are copied to the softbuffer surface and presented

**When capture ends (export or cancel):**
1. The overlay window is dropped/closed
2. The softbuffer surface and pixmap are dropped
3. The event loop continues running (tray only, no windows)

No hidden windows, no state machine toggling visibility, no channel threading.

### 5.5 Graceful Shutdown

- Quit from tray menu sets a `running = false` flag; event loop exits via `ControlFlow::Exit`
- OS signal (SIGTERM on Linux, close on Windows) also triggers shutdown
- Any in-progress capture is discarded on quit (window dropped)
- Tray icon is the embedded application icon (bundled as a PNG resource in the binary)

---

## 6. Export

### 6.1 Annotation Flattening

Since tiny-skia is the sole renderer, flattening for export uses the **exact same** `render_annotation()` function used for interactive display. The only difference is the target pixmap — instead of the full-screen overlay pixmap, annotations are rendered onto a cropped pixmap matching the selection region.

Process:
1. Crop the `CapturedScreen` pixel buffer to the selection region
2. Create a `tiny-skia::Pixmap` from the cropped pixels (handling premultiplied alpha correctly)
3. Offset each annotation's coordinates by (-selection.x, -selection.y)
4. Call `render_annotation()` for each annotation onto the cropped pixmap
5. Demultiply alpha and encode the final pixmap as PNG

### 6.2 Clipboard (Ctrl+C)

1. Flatten annotations (as above)
2. Copy the image to clipboard via `arboard` crate
3. Destroy the overlay window

### 6.3 File Save (Ctrl+S)

1. Flatten annotations (as above)
2. Open a native save dialog via `rfd` crate
3. Default filename: `hydroshot_YYYY-MM-DD_HHMMSS.png` (PNG format only)
4. If user cancels the dialog, the overlay stays open
5. If save succeeds, destroy the overlay window

---

## 7. Error Handling

Errors are logged to stderr via the `tracing` crate. Critical user-facing errors use system tray notifications (balloon tips on Windows, desktop notifications on Linux via `notify-rust`).

| Error | Behavior |
|-------|----------|
| Screen capture fails (permission denied) | Log error, show tray notification, remain idle |
| Screen capture fails (no display) | Log error, show tray notification, remain idle |
| Clipboard write fails | Log error, overlay stays open so user can try Ctrl+S instead |
| File save fails (I/O error) | Log error, overlay stays open so user can retry |
| Save dialog cancelled | Overlay stays open, no error |
| Wayland portal denied | Log error, show tray notification |

---

## 8. Crate Dependencies

| Purpose | Crate | Notes |
|---------|-------|-------|
| Windowing / event loop | `winit` | Window creation, input events, event loop |
| Pixel buffer display | `softbuffer` | Push pixel buffers to window surface |
| 2D rendering | `tiny-skia` | All rendering (interactive + export) |
| System tray | `tray-icon` | Integrates with winit's event loop |
| Screen capture (Windows) | `windows` | Win32 Desktop Duplication / BitBlt |
| Screen capture (Linux X11) | `x11rb` or `xcb` | Root window capture |
| Screen capture (Linux Wayland) | `ashpd` | xdg-desktop-portal D-Bus bindings |
| Clipboard | `arboard` | Cross-platform clipboard (image support) |
| Image encoding | `image` or `png` | Encode final output as PNG |
| File dialog | `rfd` | Native save dialog on both platforms |
| Single instance | `interprocess` or manual | Prevent duplicate launches |
| Logging | `tracing` + `tracing-subscriber` | Structured logging |
| Tray notifications | `notify-rust` (Linux), `tray-icon` balloon tips (Windows) | Error notifications |

---

## 9. Known Limitations & Risks

### Wayland Overlay

On Wayland, applications cannot create fullscreen overlay windows that sit above everything and capture global input. The compositor controls window layering. Options for future Wayland support:

- **wlr-layer-shell** protocol (supported by wlroots-based compositors like Sway, Hyprland) — allows overlay-like windows. Not supported on GNOME/KDE.
- **xdg-desktop-portal** can capture the screenshot, but the annotation overlay would need compositor cooperation.

**MVP decision**: Wayland screen capture (via `ashpd`) is supported for grabbing pixels, but the interactive overlay may not work correctly on all Wayland compositors. X11/XWayland is the reliable path on Linux. This is documented and accepted as a known limitation.

### Software Rendering Performance

tiny-skia is a CPU software renderer. For a screenshot overlay (static image + a few annotations), performance is more than sufficient. The render pipeline runs once per user interaction (mouse move, key press), not in a continuous animation loop. Profiling may be needed if the overlay feels sluggish on very high resolution displays (4K+).

---

## 10. Project Structure

```
hydroshot/
├── Cargo.toml
├── assets/
│   └── icon.png             # Tray icon (bundled into binary)
├── src/
│   ├── main.rs              # Entry point: winit event loop, tray setup, state management
│   ├── lib.rs               # Crate root, module declarations
│   ├── tray.rs              # Tray icon creation, menu, event types
│   ├── state.rs             # AppState enum, overlay state struct
│   ├── renderer.rs          # Render the overlay: screenshot, dim, selection, annotations, toolbar
│   ├── geometry.rs          # Point, Size, Color types + tiny-skia conversions
│   ├── capture/
│   │   ├── mod.rs           # ScreenCapture trait, CapturedScreen, CaptureError
│   │   ├── windows.rs       # Win32 implementation
│   │   ├── x11.rs           # X11 implementation
│   │   └── wayland.rs       # Stub — deferred to post-MVP
│   ├── overlay/
│   │   ├── selection.rs     # Selection rectangle: create, move, resize, hit-testing
│   │   └── toolbar.rs       # Toolbar positioning and hit-testing
│   ├── tools/
│   │   ├── mod.rs           # AnnotationTool trait, Annotation enum
│   │   ├── arrow.rs         # Arrow tool
│   │   └── rectangle.rs     # Rectangle tool
│   └── export.rs            # Crop, flatten annotations, clipboard copy, file save
```
