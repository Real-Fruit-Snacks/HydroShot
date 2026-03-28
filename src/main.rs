#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use tray_icon::menu::MenuEvent;
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId, WindowLevel};

use winit::application::ApplicationHandler;

use notify_rust::Notification;

use hydroshot::capture;
use hydroshot::cli::{Cli, Commands};
use hydroshot::config::Config;
use hydroshot::export;
use hydroshot::geometry::{Color, Point};
use hydroshot::history_ui::HistoryWindow;
use hydroshot::overlay::selection::{HitZone, Selection};
use hydroshot::overlay::toolbar::Toolbar;
use hydroshot::renderer::render_overlay;
use hydroshot::settings_ui::SettingsWindow;
use hydroshot::state::{AppState, OverlayState};
use hydroshot::tools::{
    annotation_bounding_box, apply_redo, apply_undo, hit_test_annotation, move_annotation,
    recolor_annotation, record_undo, resize_annotation, Annotation, AnnotationTool, ResizeHandle,
    ToolKind, UndoAction,
};
use hydroshot::tray::{self, TrayState};
use hydroshot::window_detect;

/// Border thickness for pinned windows (Catppuccin themed frame)
const PIN_BORDER: u32 = 3;
/// Shadow offset for pinned windows
const PIN_SHADOW: u32 = 2;

struct PinnedWindow {
    window: Arc<Window>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    pixels: Vec<u8>, // RGBA pixels of the pinned image (includes border)
    width: u32,
    height: u32,
    dragging: bool,
    drag_start: Option<winit::dpi::PhysicalPosition<f64>>,
    temp_path: Option<PathBuf>,
}

struct App {
    config: Config,
    state: AppState,
    tray: Option<TrayState>,
    overlay_window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    pixmap: Option<tiny_skia::Pixmap>,
    modifiers: ModifiersState,
    needs_redraw: bool,
    last_render: Instant,
    capture_at: Option<Instant>,
    _hotkey_manager: Option<global_hotkey::GlobalHotKeyManager>,
    hotkey_id: Option<u32>,
    pinned_windows: Vec<PinnedWindow>,
    immediate_capture: bool,
    cli_only: bool,
    startup_notified: bool,
    countdown_window: Option<Arc<Window>>,
    countdown_surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    countdown_remaining: u32,
    countdown_next_tick: Option<Instant>,
    window_capture_mode: bool,
    window_rects: Vec<window_detect::WinRect>,
    settings_window: Option<SettingsWindow>,
    history_window: Option<HistoryWindow>,
}

const FRAME_INTERVAL: Duration = Duration::from_millis(16); // ~60fps cap

impl App {
    fn new(config: Config) -> Self {
        Self {
            config,
            state: AppState::Idle,
            tray: None,
            overlay_window: None,
            surface: None,
            pixmap: None,
            modifiers: ModifiersState::empty(),
            needs_redraw: false,
            last_render: Instant::now(),
            capture_at: None,
            _hotkey_manager: None,
            hotkey_id: None,
            pinned_windows: Vec::new(),
            immediate_capture: false,
            cli_only: false,
            startup_notified: false,
            countdown_window: None,
            countdown_surface: None,
            countdown_remaining: 0,
            countdown_next_tick: None,
            window_capture_mode: false,
            window_rects: Vec::new(),
            settings_window: None,
            history_window: None,
        }
    }

    fn trigger_capture(&mut self, event_loop: &ActiveEventLoop) {
        if self.overlay_window.is_some() {
            tracing::info!("Overlay already open, ignoring capture trigger");
            return;
        }

        let capturer = match capture::create_capturer() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create capturer: {e}");
                return;
            }
        };

        let screens = match capturer.capture_all_screens() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Screen capture failed: {e}");
                return;
            }
        };

        if screens.is_empty() {
            tracing::error!("No screens captured");
            return;
        }

        // Use the first (primary) screen
        let screenshot = screens.into_iter().next().unwrap();
        tracing::info!(
            "Captured screen: {}x{} at ({}, {})",
            screenshot.width,
            screenshot.height,
            screenshot.x_offset,
            screenshot.y_offset
        );

        let attrs = WindowAttributes::default()
            .with_position(winit::dpi::PhysicalPosition::new(
                screenshot.x_offset,
                screenshot.y_offset,
            ))
            .with_inner_size(winit::dpi::PhysicalSize::new(
                screenshot.width,
                screenshot.height,
            ))
            .with_decorations(false)
            .with_visible(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_title("HydroShot");

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("Failed to create overlay window: {e}");
                return;
            }
        };

        let context = match softbuffer::Context::new(Arc::clone(&window)) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create softbuffer context: {e}");
                return;
            }
        };

        let surface = match softbuffer::Surface::new(&context, Arc::clone(&window)) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create softbuffer surface: {e}");
                return;
            }
        };

        let pixmap = tiny_skia::Pixmap::new(screenshot.width, screenshot.height);
        if pixmap.is_none() {
            tracing::error!(
                "Failed to create pixmap ({}x{})",
                screenshot.width,
                screenshot.height
            );
            return;
        }

        self.state = AppState::Capturing(Box::new(OverlayState::new(screenshot, &self.config)));
        self.surface = Some(surface);
        self.pixmap = pixmap;
        self.overlay_window = Some(window);
        self.needs_redraw = true;

        if let Some(w) = &self.overlay_window {
            w.request_redraw();
        }
    }

    fn open_settings(&mut self, event_loop: &ActiveEventLoop) {
        if self.settings_window.is_some() {
            tracing::info!("Settings window already open");
            return;
        }

        let config = Config::load();

        // Load window icon from embedded PNG
        let win_icon = {
            let icon_bytes = include_bytes!("../assets/icon.png");
            let img = image::load_from_memory(icon_bytes)
                .ok()
                .map(|i| i.to_rgba8());
            img.and_then(|i| {
                let (w, h) = i.dimensions();
                winit::window::Icon::from_rgba(i.into_raw(), w, h).ok()
            })
        };

        let mut attrs = WindowAttributes::default()
            .with_title("HydroShot Settings")
            .with_inner_size(winit::dpi::LogicalSize::new(
                hydroshot::settings_ui::WIN_W,
                hydroshot::settings_ui::WIN_H,
            ))
            .with_resizable(false)
            .with_decorations(true);

        if let Some(icon) = win_icon {
            attrs = attrs.with_window_icon(Some(icon));
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("Failed to create settings window: {e}");
                return;
            }
        };

        let context = match softbuffer::Context::new(Arc::clone(&window)) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create settings context: {e}");
                return;
            }
        };

        let surface = match softbuffer::Surface::new(&context, Arc::clone(&window)) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create settings surface: {e}");
                return;
            }
        };

        let mut sw = SettingsWindow::new(window, surface, config);
        sw.render();
        tracing::info!("Settings window opened");
        self.settings_window = Some(sw);
    }

    fn close_settings(&mut self, apply: bool) {
        if apply {
            self.config = Config::load(); // reload saved config
        }
        self.settings_window = None;
        tracing::info!("Settings window closed");
    }

    fn open_history(&mut self, event_loop: &ActiveEventLoop) {
        if self.history_window.is_some() {
            tracing::info!("History window already open");
            return;
        }

        // Load window icon from embedded PNG
        let win_icon = {
            let icon_bytes = include_bytes!("../assets/icon.png");
            let img = image::load_from_memory(icon_bytes)
                .ok()
                .map(|i| i.to_rgba8());
            img.and_then(|i| {
                let (w, h) = i.dimensions();
                winit::window::Icon::from_rgba(i.into_raw(), w, h).ok()
            })
        };

        let mut attrs = WindowAttributes::default()
            .with_title("HydroShot \u{2014} History")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                hydroshot::history_ui::WIN_W,
                hydroshot::history_ui::WIN_H,
            ))
            .with_resizable(false)
            .with_decorations(true);

        if let Some(icon) = win_icon {
            attrs = attrs.with_window_icon(Some(icon));
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("Failed to create history window: {e}");
                return;
            }
        };

        let context = match softbuffer::Context::new(Arc::clone(&window)) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create history context: {e}");
                return;
            }
        };

        let surface = match softbuffer::Surface::new(&context, Arc::clone(&window)) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create history surface: {e}");
                return;
            }
        };

        let mut hw = HistoryWindow::new(window, surface);
        hw.render();
        tracing::info!("History window opened");
        self.history_window = Some(hw);
    }

    fn close_history(&mut self) {
        self.history_window = None;
        tracing::info!("History window closed");
    }

    fn close_overlay(&mut self) {
        self.surface = None;
        self.pixmap = None;
        self.overlay_window = None;
        self.state = AppState::Idle;
        self.modifiers = ModifiersState::empty();
        self.window_capture_mode = false;
        self.window_rects.clear();
        tracing::info!("Overlay closed");
    }

    fn do_copy(&mut self) {
        // Collect data from overlay, then close and do heavy work on a background thread
        let copy_data = if let AppState::Capturing(ref overlay) = self.state {
            if let Some(ref sel) = overlay.selection {
                let r = sel.clamped(overlay.screenshot.width, overlay.screenshot.height);
                let pixels = export::crop_and_flatten(
                    &overlay.screenshot.pixels,
                    overlay.screenshot.width,
                    r.x,
                    r.y,
                    r.w,
                    r.h,
                    &overlay.annotations,
                );
                Some((pixels, r.w, r.h))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((pixels, w, h)) = copy_data {
            self.close_overlay();
            std::thread::spawn(move || match export::copy_to_clipboard(&pixels, w, h) {
                Ok(()) => {
                    let _ = hydroshot::history::save_to_history(&pixels, w, h);
                    tracing::info!("Copied to clipboard");
                    let _ = Notification::new()
                        .summary("HydroShot")
                        .body("Copied to clipboard")
                        .timeout(2000)
                        .show();
                }
                Err(e) => {
                    tracing::error!("Clipboard copy failed: {e}");
                    let _ = Notification::new()
                        .summary("HydroShot")
                        .body(&format!("Copy failed: {e}"))
                        .timeout(3000)
                        .show();
                }
            });
        }
    }

    fn do_save(&mut self) {
        // Collect data from overlay, then close and do heavy work on a background thread
        let save_data = if let AppState::Capturing(ref overlay) = self.state {
            if let Some(ref sel) = overlay.selection {
                let r = sel.clamped(overlay.screenshot.width, overlay.screenshot.height);
                let pixels = export::crop_and_flatten(
                    &overlay.screenshot.pixels,
                    overlay.screenshot.width,
                    r.x,
                    r.y,
                    r.w,
                    r.h,
                    &overlay.annotations,
                );
                let save_dir = self.config.save_directory();
                Some((pixels, r.w, r.h, save_dir))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((pixels, w, h, save_dir)) = save_data {
            self.close_overlay();
            std::thread::spawn(move || {
                match export::save_to_file(pixels, w, h, save_dir.as_deref()) {
                    Ok(Some(path)) => {
                        // Copy saved file to history instead of re-encoding
                        let _ = hydroshot::history::save_to_history_from_file(
                            std::path::Path::new(&path),
                        );
                        tracing::info!("Saved to {path}");
                        let _ = Notification::new()
                            .summary("HydroShot")
                            .body(&format!("Saved to {path}"))
                            .timeout(2000)
                            .show();
                    }
                    Ok(None) => {
                        tracing::info!("Save cancelled by user");
                    }
                    Err(e) => {
                        tracing::error!("Save failed: {e}");
                        let _ = Notification::new()
                            .summary("HydroShot")
                            .body(&format!("Save failed: {e}"))
                            .timeout(3000)
                            .show();
                    }
                }
            });
        }
    }

    fn do_upload(&mut self) {
        // Check if Imgur is configured before entering the confirmation flow
        let has_client_id = !self.config.general.imgur_client_id.is_empty()
            || std::env::var("HYDROSHOT_IMGUR_CLIENT_ID")
                .map(|v| !v.is_empty())
                .unwrap_or(false);

        if !has_client_id {
            if let AppState::Capturing(ref mut o) = self.state {
                o.show_toast(
                    "Set imgur_client_id in config.toml to enable uploads".into(),
                    4000,
                );
            }
            self.needs_redraw = true;
            return;
        }

        // First click: show confirmation toast
        // Second click: actually upload
        let confirmed = if let AppState::Capturing(ref overlay) = self.state {
            overlay.upload_confirmed
        } else {
            false
        };

        if !confirmed {
            // First click — ask for confirmation
            if let AppState::Capturing(ref mut o) = self.state {
                o.upload_confirmed = true;
                o.show_toast("Click Upload again to share to Imgur (public)".into(), 4000);
            }
            self.needs_redraw = true;
            return;
        }

        // Second click — confirmed, proceed with upload
        let upload_data = if let AppState::Capturing(ref mut o) = self.state {
            o.upload_confirmed = false; // reset
            if let Some(ref sel) = o.selection {
                let r = sel.clamped(o.screenshot.width, o.screenshot.height);
                let pixels = export::crop_and_flatten(
                    &o.screenshot.pixels,
                    o.screenshot.width,
                    r.x,
                    r.y,
                    r.w,
                    r.h,
                    &o.annotations,
                );
                let _ = hydroshot::history::save_to_history(&pixels, r.w, r.h);
                Some((pixels, r.w, r.h))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((pixels, w, h)) = upload_data {
            // Show uploading toast
            if let AppState::Capturing(ref mut o) = self.state {
                o.show_toast("Uploading to Imgur...".into(), 10000);
            }
            self.needs_redraw = true;

            // Encode to PNG bytes
            let img = match image::RgbaImage::from_raw(w, h, pixels) {
                Some(img) => img,
                None => {
                    tracing::error!("Failed to create image for upload");
                    if let AppState::Capturing(ref mut o) = self.state {
                        o.show_toast("Upload failed: invalid image data".into(), 3000);
                    }
                    return;
                }
            };
            let mut png_bytes = Vec::new();
            if let Err(e) = img.write_to(
                &mut std::io::Cursor::new(&mut png_bytes),
                image::ImageFormat::Png,
            ) {
                tracing::error!("PNG encode failed: {}", e);
                if let AppState::Capturing(ref mut o) = self.state {
                    o.show_toast(format!("Upload failed: {}", e), 3000);
                }
                return;
            }

            // Close overlay immediately, upload in background thread
            let imgur_id = self.config.general.imgur_client_id.clone();
            self.close_overlay();

            std::thread::spawn(move || {
                let toast_msg = match hydroshot::upload::upload_to_imgur(&png_bytes, &imgur_id) {
                    Ok(url) => {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&url);
                        }
                        tracing::info!("Uploaded to Imgur: {}", url);
                        format!("Uploaded! URL copied: {}", url)
                    }
                    Err(e) => {
                        tracing::error!("Imgur upload failed: {}", e);
                        format!("Upload failed: {}", e)
                    }
                };

                let _ = Notification::new()
                    .summary("HydroShot")
                    .body(&toast_msg)
                    .timeout(3000)
                    .show();
            });
        }
    }

    fn do_ocr(&mut self) {
        // Extract cropped pixels (immutable borrow of state)
        let ocr_data = if let AppState::Capturing(ref overlay) = self.state {
            if let Some(ref sel) = overlay.selection {
                let r = sel.clamped(overlay.screenshot.width, overlay.screenshot.height);
                if r.w == 0 || r.h == 0 {
                    None
                } else {
                    let mut cropped = vec![0u8; (r.w as usize) * (r.h as usize) * 4];
                    let src_w = overlay.screenshot.width;
                    for row in 0..r.h {
                        let src_row = r.y + row;
                        if src_row >= overlay.screenshot.height {
                            break;
                        }
                        let src_offset = ((src_row * src_w + r.x) * 4) as usize;
                        let dst_offset = (row * r.w * 4) as usize;
                        let copy_w = r.w.min(src_w.saturating_sub(r.x)) as usize * 4;
                        let src_end = (src_offset + copy_w).min(overlay.screenshot.pixels.len());
                        let actual = src_end - src_offset;
                        cropped[dst_offset..dst_offset + actual]
                            .copy_from_slice(&overlay.screenshot.pixels[src_offset..src_end]);
                    }
                    Some((cropped, r.w, r.h))
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some((cropped, w, h)) = ocr_data {
            self.close_overlay();
            std::thread::spawn(move || {
                let toast_msg = match hydroshot::ocr::extract_text(&cropped, w, h) {
                    Ok(text) => {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                        }
                        tracing::info!("OCR extracted {} chars", text.len());
                        if text.len() > 80 {
                            format!(
                                "Copied {} chars: {}...",
                                text.len(),
                                text.chars().take(80).collect::<String>()
                            )
                        } else {
                            format!("Copied: {}", text)
                        }
                    }
                    Err(e) => {
                        tracing::error!("OCR failed: {e}");
                        format!("OCR failed: {e}")
                    }
                };
                let _ = notify_rust::Notification::new()
                    .summary("HydroShot")
                    .body(&toast_msg)
                    .timeout(3000)
                    .show();
            });
        }
    }

    fn do_pin(&mut self, event_loop: &ActiveEventLoop) {
        if let AppState::Capturing(ref overlay) = self.state {
            if let Some(ref sel) = overlay.selection {
                let cr = sel.clamped(overlay.screenshot.width, overlay.screenshot.height);
                if cr.w == 0 || cr.h == 0 {
                    return;
                }

                let pixels = export::crop_and_flatten(
                    &overlay.screenshot.pixels,
                    overlay.screenshot.width,
                    cr.x,
                    cr.y,
                    cr.w,
                    cr.h,
                    &overlay.annotations,
                );

                let _ = hydroshot::history::save_to_history(&pixels, cr.w, cr.h);

                // Add a Catppuccin-themed border + shadow around the image
                let border = PIN_BORDER;
                let shadow = PIN_SHADOW;
                let pin_w = cr.w;
                let pin_h = cr.h;
                let total_w = pin_w + border * 2 + shadow;
                let total_h = pin_h + border * 2 + shadow;

                // Build the framed pixel buffer
                let mut framed = vec![0u8; (total_w * total_h * 4) as usize];

                // Shadow fill (dark, offset bottom-right)
                for y in shadow..total_h {
                    for x in shadow..total_w {
                        let i = ((y * total_w + x) * 4) as usize;
                        if i + 3 < framed.len() {
                            framed[i] = 17; // Crust R
                            framed[i + 1] = 17;
                            framed[i + 2] = 27;
                            framed[i + 3] = 100; // semi-transparent
                        }
                    }
                }

                // Border fill (Lavender #b4befe)
                for y in 0..total_h - shadow {
                    for x in 0..total_w - shadow {
                        let i = ((y * total_w + x) * 4) as usize;
                        if i + 3 < framed.len() {
                            framed[i] = 180; // Lavender R
                            framed[i + 1] = 190;
                            framed[i + 2] = 254;
                            framed[i + 3] = 255;
                        }
                    }
                }

                // Copy the actual image inside the border
                for y in 0..pin_h {
                    for x in 0..pin_w {
                        let src = ((y * pin_w + x) * 4) as usize;
                        let dst = (((y + border) * total_w + x + border) * 4) as usize;
                        if src + 3 < pixels.len() && dst + 3 < framed.len() {
                            framed[dst] = pixels[src];
                            framed[dst + 1] = pixels[src + 1];
                            framed[dst + 2] = pixels[src + 2];
                            framed[dst + 3] = pixels[src + 3];
                        }
                    }
                }

                let pixels = framed;
                let pin_w = total_w;
                let pin_h = total_h;

                // Position the pin window near the selection's screen position
                let pin_x = sel.x as i32 - border as i32;
                let pin_y = sel.y as i32 - border as i32;

                let attrs = WindowAttributes::default()
                    .with_title("HydroShot Pin")
                    .with_inner_size(winit::dpi::PhysicalSize::new(pin_w, pin_h))
                    .with_position(winit::dpi::PhysicalPosition::new(pin_x, pin_y))
                    .with_window_level(WindowLevel::AlwaysOnTop)
                    .with_decorations(false);

                let window = match event_loop.create_window(attrs) {
                    Ok(w) => Arc::new(w),
                    Err(e) => {
                        tracing::error!("Failed to create pin window: {e}");
                        return;
                    }
                };

                let context = match softbuffer::Context::new(Arc::clone(&window)) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Failed to create pin softbuffer context: {e}");
                        return;
                    }
                };

                let mut surface = match softbuffer::Surface::new(&context, Arc::clone(&window)) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to create pin softbuffer surface: {e}");
                        return;
                    }
                };

                // Render the pinned image to the surface
                if let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(pin_w), NonZeroU32::new(pin_h)) {
                    if let Err(e) = surface.resize(nz_w, nz_h) {
                        tracing::error!("Pin surface resize failed: {e}");
                        return;
                    }
                }

                if let Ok(mut buffer) = surface.buffer_mut() {
                    let pixel_count = (pin_w * pin_h) as usize;
                    for (i, chunk) in pixels.chunks_exact(4).take(pixel_count).enumerate() {
                        buffer[i] = ((chunk[0] as u32) << 16)
                            | ((chunk[1] as u32) << 8)
                            | (chunk[2] as u32);
                    }
                    let _ = buffer.present();
                }

                window.request_redraw();

                // Save the pin image to a temp file for drag-and-drop support
                let temp_path = {
                    let temp_dir = std::env::temp_dir();
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let path = temp_dir.join(format!("hydroshot_pin_{ts}.png"));
                    if let Some(img) = image::RgbaImage::from_raw(pin_w, pin_h, pixels.clone()) {
                        match img.save(&path) {
                            Ok(()) => {
                                tracing::info!("Pin temp file saved to {}", path.display());
                                Some(path)
                            }
                            Err(e) => {
                                tracing::warn!("Failed to save pin temp file: {e}");
                                None
                            }
                        }
                    } else {
                        None
                    }
                };

                self.pinned_windows.push(PinnedWindow {
                    window,
                    surface,
                    pixels,
                    width: pin_w,
                    height: pin_h,
                    dragging: false,
                    drag_start: None,
                    temp_path,
                });

                // Set grab cursor to indicate the pin is draggable
                if let Some(pin) = self.pinned_windows.last() {
                    pin.window.set_cursor(CursorIcon::Grab);
                }
                tracing::info!("Pinned {}x{} capture to screen", pin_w, pin_h);
            }
        }
        self.close_overlay();
    }

    fn start_countdown(&mut self, event_loop: &ActiveEventLoop, seconds: u32) {
        const CD_SIZE: u32 = 120;

        // Find center of primary monitor
        let (x, y) = if let Some(monitor) = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
        {
            let size = monitor.size();
            (
                (size.width as i32 - CD_SIZE as i32) / 2,
                (size.height as i32 - CD_SIZE as i32) / 2,
            )
        } else {
            (800, 400)
        };

        let attrs = WindowAttributes::default()
            .with_title("HydroShot Countdown")
            .with_inner_size(winit::dpi::PhysicalSize::new(CD_SIZE, CD_SIZE))
            .with_position(PhysicalPosition::new(x, y))
            .with_decorations(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_resizable(false);

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("Failed to create countdown window: {e}");
                return;
            }
        };

        let context = match softbuffer::Context::new(Arc::clone(&window)) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to create countdown context: {e}");
                return;
            }
        };

        let surface = match softbuffer::Surface::new(&context, Arc::clone(&window)) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create countdown surface: {e}");
                return;
            }
        };

        self.countdown_window = Some(window);
        self.countdown_surface = Some(surface);
        self.countdown_remaining = seconds;
        self.countdown_next_tick = Some(Instant::now() + Duration::from_secs(1));
        self.render_countdown();
    }

    fn render_countdown(&mut self) {
        const CD_SIZE: u32 = 120;

        let surface = match self.countdown_surface.as_mut() {
            Some(s) => s,
            None => return,
        };

        // Create a pixmap and draw the countdown number
        let mut pixmap = match tiny_skia::Pixmap::new(CD_SIZE, CD_SIZE) {
            Some(p) => p,
            None => return,
        };

        // Background: Catppuccin Crust #11111b at 90% opacity
        let bg_r = 0x11;
        let bg_g = 0x11;
        let bg_b = 0x1b;
        let bg_a: u8 = 230; // ~90%
        let pixels_data = pixmap.data_mut();
        for chunk in pixels_data.chunks_exact_mut(4) {
            chunk[0] = bg_r;
            chunk[1] = bg_g;
            chunk[2] = bg_b;
            chunk[3] = bg_a;
        }

        // Render the number using render_text_annotation
        let num_str = self.countdown_remaining.to_string();
        let text_color = Color {
            r: 0xb4 as f32 / 255.0,
            g: 0xbe as f32 / 255.0,
            b: 0xfe as f32 / 255.0,
            a: 1.0,
        }; // Lavender #b4befe
        let font_size = 60.0_f32;

        // Center the text: estimate width ~36px per char at 60px font, height ~60px
        let text_w = num_str.len() as f32 * 36.0;
        let text_x = (CD_SIZE as f32 - text_w) / 2.0;
        let text_y = (CD_SIZE as f32 - font_size) / 2.0;
        let text_pos = Point::new(text_x, text_y);

        hydroshot::tools::render_text_annotation(
            &mut pixmap,
            &text_pos,
            &num_str,
            &text_color,
            font_size,
        );

        // Copy pixmap to softbuffer surface
        if let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(CD_SIZE), NonZeroU32::new(CD_SIZE)) {
            if let Err(e) = surface.resize(nz_w, nz_h) {
                tracing::error!("Countdown surface resize failed: {e}");
                return;
            }
        }

        if let Ok(mut buffer) = surface.buffer_mut() {
            let src = pixmap.data();
            let pixel_count = (CD_SIZE * CD_SIZE) as usize;
            for (i, chunk) in src.chunks_exact(4).take(pixel_count).enumerate() {
                buffer[i] =
                    ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
            }
            let _ = buffer.present();
        }

        if let Some(ref w) = self.countdown_window {
            w.request_redraw();
        }
    }

    fn close_countdown(&mut self) {
        // Hide the window immediately before dropping — set_visible(false) is
        // processed synchronously by the OS, guaranteeing the window is gone
        // from the screen before the next screenshot is taken.
        if let Some(ref window) = self.countdown_window {
            window.set_visible(false);
        }
        self.countdown_window = None;
        self.countdown_surface = None;
        self.countdown_remaining = 0;
        self.countdown_next_tick = None;
    }

    fn process_tray_events(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                tracing::info!("Tray icon clicked — triggering capture");
                self.trigger_capture(event_loop);
            }
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(ref tray) = self.tray {
                if event.id == tray.capture_id {
                    tracing::info!("Capture menu item clicked");
                    self.trigger_capture(event_loop);
                } else if event.id == tray.window_capture_id {
                    tracing::info!("Window capture menu item clicked");
                    self.window_capture_mode = true;
                    self.window_rects = window_detect::enumerate_window_rects();
                    tracing::info!("Enumerated {} windows", self.window_rects.len());
                    self.trigger_capture(event_loop);
                } else if event.id == tray.delay_3_id {
                    tracing::info!("Capturing in 3 seconds...");
                    // Don't set capture_at — countdown reaching 0 triggers the capture
                    self.start_countdown(event_loop, 3);
                } else if event.id == tray.delay_5_id {
                    tracing::info!("Capturing in 5 seconds...");
                    self.start_countdown(event_loop, 5);
                } else if event.id == tray.delay_10_id {
                    tracing::info!("Capturing in 10 seconds...");
                    self.start_countdown(event_loop, 10);
                } else if event.id == tray.autostart_id {
                    let new_state = !hydroshot::autostart::is_enabled();
                    if let Err(e) = hydroshot::autostart::set_enabled(new_state) {
                        tracing::error!("Auto-start toggle failed: {}", e);
                    } else {
                        tracing::info!(
                            "Auto-start {}",
                            if new_state { "enabled" } else { "disabled" }
                        );
                        tray.autostart_check.set_checked(new_state);
                    }
                } else if event.id == tray.history_id {
                    tracing::info!("History menu item clicked");
                    self.open_history(event_loop);
                } else if event.id == tray.quit_id {
                    tracing::info!("Quit requested");
                    event_loop.exit();
                } else if event.id == tray.settings_id {
                    tracing::info!("Settings menu item clicked");
                    self.open_settings(event_loop);
                } else if event.id == tray.about_id {
                    let version = env!("CARGO_PKG_VERSION");
                    let _ = Notification::new()
                        .summary("HydroShot")
                        .body(&format!(
                            "HydroShot v{}\nScreenshot capture & annotation tool\ngithub.com/Real-Fruit-Snacks/HydroShot",
                            version
                        ))
                        .timeout(5000)
                        .show();
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("cmd")
                            .args([
                                "/C",
                                "start",
                                "https://github.com/Real-Fruit-Snacks/HydroShot",
                            ])
                            .spawn();
                    }
                    #[cfg(target_os = "linux")]
                    {
                        let _ = std::process::Command::new("xdg-open")
                            .arg("https://github.com/Real-Fruit-Snacks/HydroShot")
                            .spawn();
                    }
                }
            }
        }
    }

    fn render(&mut self) {
        if !self.needs_redraw {
            return;
        }

        let overlay = match &mut self.state {
            AppState::Capturing(o) => o,
            AppState::Idle => return,
        };

        let pixmap = match self.pixmap.as_mut() {
            Some(p) => p,
            None => return,
        };

        let surface = match self.surface.as_mut() {
            Some(s) => s,
            None => return,
        };

        let window = match self.overlay_window.as_ref() {
            Some(w) => w,
            None => return,
        };

        render_overlay(overlay, pixmap);

        let size = window.inner_size();
        let w = size.width;
        let h = size.height;

        if w == 0 || h == 0 {
            return;
        }

        if let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
            if let Err(e) = surface.resize(nz_w, nz_h) {
                tracing::error!("Surface resize failed: {e}");
                return;
            }
        }

        let mut buffer = match surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to get surface buffer: {e}");
                return;
            }
        };

        // Fast pixmap → softbuffer copy using raw bytes.
        // Screenshot pixels are fully opaque (alpha=255), and tiny-skia composites
        // onto an opaque background, so all pixels remain opaque after rendering.
        // For opaque pixels, premultiplied == straight — skip demultiply entirely.
        // Pixel format: tiny-skia RGBA bytes → softbuffer 0x00RRGGBB u32
        let src_data = pixmap.data(); // &[u8], RGBA order
        let pixel_count = (pixmap.width() * pixmap.height()) as usize;
        let buf_len = buffer.len();
        let copy_count = pixel_count.min(buf_len);

        for (i, chunk) in src_data.chunks_exact(4).take(copy_count).enumerate() {
            buffer[i] = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        }

        if let Err(e) = buffer.present() {
            tracing::error!("Buffer present failed: {e}");
        }

        // Show window after first frame is rendered (avoids white flash)
        if let Some(ref window) = self.overlay_window {
            if !window.is_visible().unwrap_or(true) {
                window.set_visible(true);
            }
        }

        self.needs_redraw = false;
    }
}

/// Determine the appropriate cursor icon based on the current overlay state and mouse position.
fn determine_cursor(overlay: &OverlayState, pos: Point) -> CursorIcon {
    // Text input active — always show text cursor
    if overlay.text_input_active {
        return CursorIcon::Text;
    }

    // Check toolbar hover (only if selection exists)
    if let Some(ref sel) = overlay.selection {
        let visible_count = overlay.visible_buttons.len();
        let toolbar =
            Toolbar::position_for_dynamic(sel, overlay.screenshot.height as f32, visible_count);
        if toolbar.hit_test_dynamic(pos, visible_count).is_some() {
            return CursorIcon::Pointer;
        }
    }

    // Currently dragging a resize handle on an annotation
    if let Some(handle) = overlay.resize_handle {
        return match handle {
            ResizeHandle::TopLeft | ResizeHandle::BottomRight => CursorIcon::NwseResize,
            ResizeHandle::TopRight | ResizeHandle::BottomLeft => CursorIcon::NeswResize,
        };
    }

    // Hovering over a resize handle on a selected annotation
    if overlay.active_tool == ToolKind::Select {
        if let Some(idx) = overlay.selected_index {
            if let Some(ann) = overlay.annotations.get(idx) {
                if let Some((bx, by, bw, bh)) = annotation_bounding_box(ann) {
                    let handles = [
                        (Point::new(bx, by), ResizeHandle::TopLeft),
                        (Point::new(bx + bw, by), ResizeHandle::TopRight),
                        (Point::new(bx, by + bh), ResizeHandle::BottomLeft),
                        (Point::new(bx + bw, by + bh), ResizeHandle::BottomRight),
                    ];
                    for (hp, handle) in &handles {
                        if (pos.x - hp.x).abs() < 8.0 && (pos.y - hp.y).abs() < 8.0 {
                            return match handle {
                                ResizeHandle::TopLeft | ResizeHandle::BottomRight => {
                                    CursorIcon::NwseResize
                                }
                                ResizeHandle::TopRight | ResizeHandle::BottomLeft => {
                                    CursorIcon::NeswResize
                                }
                            };
                        }
                    }
                }
            }
        }
    }

    // Currently dragging to create selection
    if overlay.is_selecting {
        return CursorIcon::Crosshair;
    }

    // Currently dragging a resize/move zone
    if let Some(zone) = overlay.drag_zone {
        return hitzone_to_cursor(zone, overlay);
    }

    // Selection exists — hit-test it
    if let Some(ref sel) = overlay.selection {
        if let Some(zone) = sel.hit_test(pos, 8.0) {
            return hitzone_to_cursor(zone, overlay);
        }
    }

    // No selection yet (idle) — crosshair
    CursorIcon::Crosshair
}

/// Map a HitZone to the appropriate cursor icon.
fn hitzone_to_cursor(zone: HitZone, overlay: &OverlayState) -> CursorIcon {
    match zone {
        HitZone::TopLeft | HitZone::BottomRight => CursorIcon::NwseResize,
        HitZone::TopRight | HitZone::BottomLeft => CursorIcon::NeswResize,
        HitZone::Top | HitZone::Bottom => CursorIcon::NsResize,
        HitZone::Left | HitZone::Right => CursorIcon::EwResize,
        HitZone::Inside => {
            // Inside selection — cursor depends on active tool
            match overlay.active_tool {
                ToolKind::Select => CursorIcon::Default,
                ToolKind::Text => CursorIcon::Text,
                ToolKind::StepMarker => CursorIcon::Cell,
                ToolKind::Eyedropper => CursorIcon::Crosshair,
                ToolKind::Arrow | ToolKind::Line | ToolKind::Pencil | ToolKind::Measurement => {
                    CursorIcon::Crosshair
                }
                ToolKind::Rectangle
                | ToolKind::RoundedRect
                | ToolKind::Circle
                | ToolKind::Highlight
                | ToolKind::Pixelate
                | ToolKind::Spotlight => CursorIcon::Crosshair,
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        tracing::info!("Application resumed");
        event_loop.set_control_flow(ControlFlow::Wait);

        if !self.cli_only {
            if self.tray.is_none() {
                match tray::create_tray() {
                    Ok(t) => {
                        tracing::info!("Tray icon created");
                        self.tray = Some(t);
                    }
                    Err(e) => {
                        tracing::error!("Failed to create tray icon: {e}");
                    }
                }
            }

            if self._hotkey_manager.is_none() {
                match hydroshot::hotkey::register_hotkey(&self.config.hotkey.capture) {
                    Ok((manager, id)) => {
                        self._hotkey_manager = Some(manager);
                        self.hotkey_id = Some(id);
                        tracing::info!("Global hotkey registered: {}", self.config.hotkey.capture);
                    }
                    Err(e) => tracing::warn!("Failed to register hotkey: {}", e),
                }
            }
        }

        // Show startup notification (only once, not in CLI mode)
        if !self.cli_only
            && self.tray.is_some()
            && !self.immediate_capture
            && !self.startup_notified
        {
            self.startup_notified = true;
            let hotkey = &self.config.hotkey.capture;
            let _ = Notification::new()
                .summary("HydroShot")
                .body(&format!("Ready — {} to capture", hotkey))
                .timeout(3000)
                .show();
        }

        if self.immediate_capture {
            self.immediate_capture = false;
            self.trigger_capture(event_loop);
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Check if event is for a pinned window
        if let Some(pin_idx) = self
            .pinned_windows
            .iter()
            .position(|p| p.window.id() == _window_id)
        {
            match event {
                WindowEvent::CloseRequested => {
                    let pin = self.pinned_windows.remove(pin_idx);
                    if let Some(ref path) = pin.temp_path {
                        let _ = std::fs::remove_file(path);
                    }
                    tracing::info!("Pinned window closed");
                    return;
                }
                WindowEvent::RedrawRequested => {
                    let pin = &mut self.pinned_windows[pin_idx];
                    let w = pin.width;
                    let h = pin.height;
                    if let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
                        let _ = pin.surface.resize(nz_w, nz_h);
                        if let Ok(mut buffer) = pin.surface.buffer_mut() {
                            let pixel_count = (w * h) as usize;
                            for (i, chunk) in
                                pin.pixels.chunks_exact(4).take(pixel_count).enumerate()
                            {
                                buffer[i] = ((chunk[0] as u32) << 16)
                                    | ((chunk[1] as u32) << 8)
                                    | (chunk[2] as u32);
                            }
                            let _ = buffer.present();
                        }
                    }
                    return;
                }
                WindowEvent::KeyboardInput { ref event, .. } => {
                    if event.state == ElementState::Pressed {
                        if let Key::Named(NamedKey::Escape) = &event.logical_key {
                            let pin = self.pinned_windows.remove(pin_idx);
                            if let Some(ref path) = pin.temp_path {
                                let _ = std::fs::remove_file(path);
                            }
                            tracing::info!("Pinned window closed via Escape");
                            return;
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state: btn_state,
                    button: WinitMouseButton::Left,
                    ..
                } => {
                    let pin = &mut self.pinned_windows[pin_idx];
                    match btn_state {
                        ElementState::Pressed => {
                            pin.dragging = true;
                            pin.drag_start = None;
                            pin.window.set_cursor(CursorIcon::Grabbing);
                        }
                        ElementState::Released => {
                            pin.dragging = false;
                            pin.drag_start = None;
                            pin.window.set_cursor(CursorIcon::Grab);
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: WinitMouseButton::Right,
                    ..
                } => {
                    let pin = &self.pinned_windows[pin_idx];
                    if let Some(ref path) = pin.temp_path {
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("explorer")
                                .arg("/select,")
                                .arg(path)
                                .spawn();
                        }
                        #[cfg(target_os = "linux")]
                        {
                            let _ = std::process::Command::new("xdg-open")
                                .arg(path.parent().unwrap_or(path))
                                .spawn();
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: WinitMouseButton::Middle,
                    ..
                } => {
                    let pin = &self.pinned_windows[pin_idx];
                    let pixels = pin.pixels.clone();
                    let w = pin.width;
                    let h = pin.height;
                    std::thread::spawn(move || match export::copy_to_clipboard(&pixels, w, h) {
                        Ok(()) => {
                            let _ = Notification::new()
                                .summary("HydroShot")
                                .body("Pin image copied to clipboard")
                                .timeout(2000)
                                .show();
                        }
                        Err(e) => {
                            tracing::error!("Pin clipboard copy failed: {e}");
                        }
                    });
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pin = &mut self.pinned_windows[pin_idx];
                    if pin.dragging {
                        if let Some(start) = pin.drag_start {
                            let dx = position.x - start.x;
                            let dy = position.y - start.y;
                            if let Ok(current_pos) = pin.window.outer_position() {
                                let new_x = current_pos.x + dx as i32;
                                let new_y = current_pos.y + dy as i32;
                                pin.window
                                    .set_outer_position(winit::dpi::PhysicalPosition::new(
                                        new_x, new_y,
                                    ));
                            }
                            // Don't update drag_start — cursor position is relative to window
                        } else {
                            pin.drag_start = Some(position);
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // Check if event is for the history window
        if let Some(ref hw) = self.history_window {
            if hw.window.id() == _window_id {
                match event {
                    WindowEvent::CloseRequested => {
                        self.close_history();
                        return;
                    }
                    WindowEvent::RedrawRequested => {
                        if let Some(ref mut hw) = self.history_window {
                            hw.render();
                        }
                        return;
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(ref mut hw) = self.history_window {
                            if hw.on_cursor_moved(position.x as f32, position.y as f32) {
                                hw.needs_redraw = true;
                                hw.window.request_redraw();
                            }
                        }
                        return;
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: WinitMouseButton::Left,
                        ..
                    } => {
                        if let Some(ref hw) = self.history_window {
                            let (cx, cy) = hw.cursor_pos;
                            hw.on_click(cx, cy);
                        }
                        return;
                    }
                    WindowEvent::KeyboardInput { ref event, .. } => {
                        if event.state == ElementState::Pressed {
                            if let Key::Named(NamedKey::Escape) = &event.logical_key {
                                self.close_history();
                                return;
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
        }

        // Check if event is for the settings window
        if let Some(ref sw) = self.settings_window {
            if sw.window.id() == _window_id {
                match event {
                    WindowEvent::CloseRequested => {
                        self.close_settings(false);
                        return;
                    }
                    WindowEvent::RedrawRequested => {
                        if let Some(ref mut sw) = self.settings_window {
                            sw.render();
                        }
                        return;
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(ref mut sw) = self.settings_window {
                            if sw.on_cursor_moved(position.x as f32, position.y as f32) {
                                sw.needs_redraw = true;
                                sw.window.request_redraw();
                            }
                        }
                        return;
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: WinitMouseButton::Left,
                        ..
                    } => {
                        let (cx, cy) = if let Some(ref sw) = self.settings_window {
                            (sw.cursor_pos.0, sw.cursor_pos.1)
                        } else {
                            return;
                        };
                        let action = if let Some(ref sw) = self.settings_window {
                            sw.on_click(cx, cy)
                        } else {
                            None
                        };
                        if let Some(action) = action {
                            let should_close = if let Some(ref mut sw) = self.settings_window {
                                sw.handle_action(action)
                            } else {
                                false
                            };
                            if should_close {
                                self.close_settings(true);
                            } else if let Some(ref sw) = self.settings_window {
                                sw.window.request_redraw();
                            }
                        }
                        return;
                    }
                    WindowEvent::KeyboardInput { ref event, .. } => {
                        if event.state == ElementState::Pressed {
                            // If editing a shortcut, capture the key
                            if let Some(ref mut sw) = self.settings_window {
                                if sw.editing_shortcut.is_some() {
                                    let key_str = match &event.logical_key {
                                        Key::Character(ch) => Some(ch.as_str().to_string()),
                                        Key::Named(NamedKey::Escape) => {
                                            // Cancel editing
                                            sw.editing_shortcut = None;
                                            sw.needs_redraw = true;
                                            sw.window.request_redraw();
                                            None
                                        }
                                        _ => None,
                                    };
                                    if let Some(key) = key_str {
                                        sw.on_key_press(&key);
                                        sw.window.request_redraw();
                                    }
                                    return;
                                }
                            }

                            if let Key::Named(NamedKey::Escape) = &event.logical_key {
                                self.close_settings(false);
                                return;
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
        }

        // Check if event is for the countdown window
        if let Some(ref cd_win) = self.countdown_window {
            if cd_win.id() == _window_id {
                match event {
                    WindowEvent::CloseRequested => {
                        self.close_countdown();
                        self.capture_at = None; // cancel the delayed capture
                        return;
                    }
                    WindowEvent::RedrawRequested => {
                        self.render_countdown();
                        return;
                    }
                    _ => {}
                }
                return;
            }
        }

        // Check if event is for the overlay window
        if let Some(ref overlay_win) = self.overlay_window {
            if overlay_win.id() != _window_id {
                return; // Unknown window, ignore
            }
        }

        // Only process events when we're capturing
        let overlay = match &mut self.state {
            AppState::Capturing(o) => o,
            AppState::Idle => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                self.close_overlay();
                return;
            }

            WindowEvent::RedrawRequested => {
                self.needs_redraw = true;
                self.render();
                return;
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }

                // --- Text input guard: MUST be first ---
                if overlay.text_input_active {
                    match &event.logical_key {
                        Key::Character(ch) => {
                            overlay.text_input_buffer.push_str(ch.as_str());
                            self.needs_redraw = true;
                        }
                        Key::Named(NamedKey::Backspace) => {
                            overlay.text_input_buffer.pop();
                            self.needs_redraw = true;
                        }
                        Key::Named(NamedKey::Enter) => {
                            // Confirm: create annotation from buffer
                            if !overlay.text_input_buffer.is_empty() {
                                let ann = Annotation::Text {
                                    position: overlay.text_input_position,
                                    text: overlay.text_input_buffer.clone(),
                                    color: overlay.current_color,
                                    font_size: overlay.text_input_font_size,
                                };
                                overlay.annotations.push(ann);
                                record_undo(
                                    &mut overlay.undo_stack,
                                    &mut overlay.redo_stack,
                                    UndoAction::Add(overlay.annotations.len() - 1),
                                );
                            }
                            overlay.text_input_buffer.clear();
                            overlay.text_input_active = false;
                            self.needs_redraw = true;
                        }
                        Key::Named(NamedKey::Escape) => {
                            // Cancel text input
                            overlay.text_input_buffer.clear();
                            overlay.text_input_active = false;
                            self.needs_redraw = true;
                        }
                        _ => {
                            // Ignore all other keys while typing
                        }
                    }
                    // Return early — don't fall through to other handlers
                    if self.needs_redraw {
                        if let Some(w) = &self.overlay_window {
                            w.request_redraw();
                        }
                    }
                    return;
                }

                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        let overlay = match &mut self.state {
                            AppState::Capturing(o) => o,
                            _ => return,
                        };
                        if overlay.selected_index.is_some() {
                            overlay.selected_index = None;
                            overlay.select_drag_start = None;
                            self.needs_redraw = true;
                        } else {
                            self.close_overlay();
                            return;
                        }
                    }
                    Key::Named(NamedKey::Delete) | Key::Named(NamedKey::Backspace) => {
                        let overlay = match &mut self.state {
                            AppState::Capturing(o) => o,
                            _ => return,
                        };
                        if let Some(idx) = overlay.selected_index {
                            if idx < overlay.annotations.len() {
                                let removed = overlay.annotations.remove(idx);
                                record_undo(
                                    &mut overlay.undo_stack,
                                    &mut overlay.redo_stack,
                                    UndoAction::Delete(idx, removed),
                                );
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                        }
                    }
                    Key::Named(NamedKey::Enter) => {
                        if let Some(ref sel) = overlay.selection {
                            // Quick crop: copy raw screenshot pixels (no annotations) to clipboard
                            let cr =
                                sel.clamped(overlay.screenshot.width, overlay.screenshot.height);
                            let mut cropped = vec![0u8; (cr.w as usize) * (cr.h as usize) * 4];
                            for row in 0..cr.h {
                                let src_offset =
                                    ((cr.y + row) * overlay.screenshot.width + cr.x) as usize * 4;
                                let dst_offset = (row * cr.w) as usize * 4;
                                let len = (cr.w * 4) as usize;
                                if src_offset + len <= overlay.screenshot.pixels.len() {
                                    cropped[dst_offset..dst_offset + len].copy_from_slice(
                                        &overlay.screenshot.pixels[src_offset..src_offset + len],
                                    );
                                }
                            }
                            if let Err(e) =
                                hydroshot::export::copy_to_clipboard(&cropped, cr.w, cr.h)
                            {
                                tracing::error!("Quick crop clipboard error: {}", e);
                            } else {
                                let _ = Notification::new()
                                    .summary("HydroShot")
                                    .body("Copied to clipboard")
                                    .timeout(2000)
                                    .show();
                            }
                            self.close_overlay();
                            return;
                        }
                    }
                    Key::Character(ch) => {
                        let ctrl = self.modifiers.control_key();
                        let shift = self.modifiers.shift_key();
                        match ch.as_str() {
                            "c" if ctrl => {
                                self.do_copy();
                                return;
                            }
                            "s" if ctrl => {
                                self.do_save();
                                return;
                            }
                            "z" if ctrl && shift => {
                                // Redo
                                let overlay = match &mut self.state {
                                    AppState::Capturing(o) => o,
                                    _ => return,
                                };
                                if apply_redo(
                                    &mut overlay.annotations,
                                    &mut overlay.undo_stack,
                                    &mut overlay.redo_stack,
                                ) {
                                    overlay.selected_index = None;
                                    self.needs_redraw = true;
                                }
                            }
                            "z" if ctrl => {
                                // Undo
                                let overlay = match &mut self.state {
                                    AppState::Capturing(o) => o,
                                    _ => return,
                                };
                                if apply_undo(
                                    &mut overlay.annotations,
                                    &mut overlay.undo_stack,
                                    &mut overlay.redo_stack,
                                ) {
                                    overlay.selected_index = None;
                                    self.needs_redraw = true;
                                }
                            }
                            "Z" if ctrl => {
                                // Ctrl+Shift+Z on some platforms sends uppercase Z
                                let overlay = match &mut self.state {
                                    AppState::Capturing(o) => o,
                                    _ => return,
                                };
                                if apply_redo(
                                    &mut overlay.annotations,
                                    &mut overlay.undo_stack,
                                    &mut overlay.redo_stack,
                                ) {
                                    overlay.selected_index = None;
                                    self.needs_redraw = true;
                                }
                            }
                            "a" if ctrl => {
                                // Ctrl+A: select entire screen
                                let overlay = match &mut self.state {
                                    AppState::Capturing(o) => o,
                                    _ => return,
                                };
                                overlay.selection = Some(Selection {
                                    x: 0.0,
                                    y: 0.0,
                                    width: overlay.screenshot.width as f32,
                                    height: overlay.screenshot.height as f32,
                                });
                                self.needs_redraw = true;
                            }
                            // Tool keyboard shortcuts (no Ctrl) — config-driven
                            key_str if !ctrl => {
                                let shortcuts = &self.config.shortcuts;
                                let new_tool = if key_str == shortcuts.select {
                                    Some(ToolKind::Select)
                                } else if key_str == shortcuts.arrow {
                                    Some(ToolKind::Arrow)
                                } else if key_str == shortcuts.rectangle {
                                    Some(ToolKind::Rectangle)
                                } else if key_str == shortcuts.circle {
                                    Some(ToolKind::Circle)
                                } else if key_str == shortcuts.rounded_rect {
                                    Some(ToolKind::RoundedRect)
                                } else if key_str == shortcuts.line {
                                    Some(ToolKind::Line)
                                } else if key_str == shortcuts.pencil {
                                    Some(ToolKind::Pencil)
                                } else if key_str == shortcuts.highlight {
                                    Some(ToolKind::Highlight)
                                } else if key_str == shortcuts.spotlight {
                                    Some(ToolKind::Spotlight)
                                } else if key_str == shortcuts.text {
                                    Some(ToolKind::Text)
                                } else if key_str == shortcuts.pixelate {
                                    Some(ToolKind::Pixelate)
                                } else if key_str == shortcuts.step_marker {
                                    Some(ToolKind::StepMarker)
                                } else if key_str == shortcuts.eyedropper {
                                    Some(ToolKind::Eyedropper)
                                } else if key_str == shortcuts.measurement {
                                    Some(ToolKind::Measurement)
                                } else {
                                    None
                                };

                                if let Some(tool) = new_tool {
                                    let overlay = match &mut self.state {
                                        AppState::Capturing(o) => o,
                                        _ => return,
                                    };
                                    overlay.active_tool = tool;
                                    overlay.selected_index = None;
                                    self.needs_redraw = true;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let pos = Point::new(position.x as f32, position.y as f32);
                let prev = overlay.last_mouse_pos;
                overlay.last_mouse_pos = pos;
                let dx = pos.x - prev.x;
                let dy = pos.y - prev.y;

                // Window capture mode: highlight the window under the cursor
                if self.window_capture_mode {
                    let x_off = overlay.screenshot.x_offset;
                    let y_off = overlay.screenshot.y_offset;
                    let screen_x = pos.x as i32 + x_off;
                    let screen_y = pos.y as i32 + y_off;

                    if let Some((wx, wy, ww, wh)) =
                        window_detect::window_at_point(&self.window_rects, screen_x, screen_y)
                    {
                        let sel_x = (wx - x_off) as f32;
                        let sel_y = (wy - y_off) as f32;
                        overlay.selection = Some(Selection {
                            x: sel_x,
                            y: sel_y,
                            width: ww as f32,
                            height: wh as f32,
                        });
                    } else {
                        overlay.selection = None;
                    }
                    self.needs_redraw = true;

                    // Update cursor
                    if let Some(ref window) = self.overlay_window {
                        window.set_cursor(CursorIcon::Crosshair);
                    }
                    return;
                }

                if overlay.is_selecting {
                    // Update selection while dragging
                    if let Some(start) = overlay.drag_start {
                        overlay.selection = Some(Selection::from_points(start, pos));
                        self.needs_redraw = true;
                    }
                } else if let Some(zone) = overlay.drag_zone {
                    // Resizing or moving selection
                    if let Some(ref mut sel) = overlay.selection {
                        if zone == HitZone::Inside {
                            sel.move_by(dx, dy);
                        } else {
                            sel.resize(zone, dx, dy);
                        }
                        self.needs_redraw = true;
                    }
                } else {
                    // Forward to active tool if drawing
                    match overlay.active_tool {
                        ToolKind::Select => {
                            // Resize drag takes priority
                            if let (Some(idx), Some(handle)) =
                                (overlay.selected_index, overlay.resize_handle)
                            {
                                if let Some(ann) = overlay.annotations.get_mut(idx) {
                                    resize_annotation(ann, handle, pos);
                                }
                                self.needs_redraw = true;
                            } else if let (Some(idx), Some(drag_start)) =
                                (overlay.selected_index, overlay.select_drag_start)
                            {
                                let dx = pos.x - drag_start.x;
                                let dy = pos.y - drag_start.y;
                                if let Some(ann) = overlay.annotations.get_mut(idx) {
                                    move_annotation(ann, dx, dy);
                                }
                                overlay.select_drag_start = Some(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Arrow => {
                            if overlay.arrow_tool.is_drawing() {
                                overlay.arrow_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Rectangle => {
                            if overlay.rectangle_tool.is_drawing() {
                                overlay.rectangle_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::RoundedRect => {
                            if overlay.rounded_rect_tool.is_drawing() {
                                overlay.rounded_rect_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Circle => {
                            if overlay.circle_tool.is_drawing() {
                                overlay.circle_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Line => {
                            if overlay.line_tool.is_drawing() {
                                overlay.line_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Measurement => {
                            if overlay.measurement_tool.is_drawing() {
                                overlay.measurement_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Pencil => {
                            if overlay.pencil_tool.is_drawing() {
                                overlay.pencil_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Highlight => {
                            if overlay.highlight_tool.is_drawing() {
                                overlay.highlight_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Spotlight => {
                            if overlay.spotlight_tool.is_drawing() {
                                overlay.spotlight_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::Text => {
                            // Text tool does not use mouse move
                            overlay.text_tool.on_mouse_move(pos);
                        }
                        ToolKind::Pixelate => {
                            if overlay.pixelate_tool.is_drawing() {
                                overlay.pixelate_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                        ToolKind::StepMarker => {
                            overlay.step_marker_tool.on_mouse_move(pos);
                        }
                        ToolKind::Eyedropper => {
                            // Read color from screenshot pixels at cursor position
                            let px = pos.x as u32;
                            let py = pos.y as u32;
                            if px < overlay.screenshot.width && py < overlay.screenshot.height {
                                let idx = ((py * overlay.screenshot.width + px) * 4) as usize;
                                if idx + 3 < overlay.screenshot.pixels.len() {
                                    let r = overlay.screenshot.pixels[idx] as f32 / 255.0;
                                    let g = overlay.screenshot.pixels[idx + 1] as f32 / 255.0;
                                    let b = overlay.screenshot.pixels[idx + 2] as f32 / 255.0;
                                    overlay.eyedropper_preview = Some(Color::new(r, g, b, 1.0));
                                }
                            }
                            self.needs_redraw = true;
                        }
                    }
                }

                // Update cursor icon based on current state
                if let Some(ref window) = self.overlay_window {
                    let cursor = determine_cursor(overlay, pos);
                    window.set_cursor(cursor);
                }

                // Always redraw on mouse move for tooltips and cursor feedback
                // (60fps cap prevents this from being wasteful)
                self.needs_redraw = true;
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: WinitMouseButton::Left,
                ..
            } => {
                // Window capture mode: click confirms the highlighted window
                if self.window_capture_mode {
                    // Selection is already set to the window under cursor
                    self.window_capture_mode = false;
                    self.window_rects.clear();
                    self.needs_redraw = true;
                    tracing::info!("Window captured");
                    return;
                }

                let pos = overlay.last_mouse_pos;

                // 1. Check toolbar hit first (only if selection exists)
                if let Some(ref sel) = overlay.selection {
                    let visible_count = overlay.visible_buttons.len();
                    let toolbar = Toolbar::position_for_dynamic(
                        sel,
                        overlay.screenshot.height as f32,
                        visible_count,
                    );
                    if let Some(vis_idx) = toolbar.hit_test_dynamic(pos, visible_count) {
                        let btn = overlay.visible_buttons[vis_idx];
                        // Reset upload confirmation if clicking anything other than Upload
                        if btn != 19 {
                            overlay.upload_confirmed = false;
                        }
                        match btn {
                            0 => {
                                overlay.active_tool = ToolKind::Select;
                                self.needs_redraw = true;
                            }
                            1 => {
                                overlay.active_tool = ToolKind::Arrow;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            2 => {
                                overlay.active_tool = ToolKind::Rectangle;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            3 => {
                                overlay.active_tool = ToolKind::Circle;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            4 => {
                                overlay.active_tool = ToolKind::RoundedRect;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            5 => {
                                overlay.active_tool = ToolKind::Line;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            6 => {
                                overlay.active_tool = ToolKind::Pencil;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            7 => {
                                overlay.active_tool = ToolKind::Highlight;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            8 => {
                                overlay.active_tool = ToolKind::Spotlight;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            9 => {
                                overlay.active_tool = ToolKind::Text;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            10 => {
                                overlay.active_tool = ToolKind::Pixelate;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            11 => {
                                overlay.active_tool = ToolKind::StepMarker;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            12 => {
                                overlay.active_tool = ToolKind::Eyedropper;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            13 => {
                                overlay.active_tool = ToolKind::Measurement;
                                overlay.selected_index = None;
                                self.needs_redraw = true;
                            }
                            14..=18 => {
                                let presets = Color::presets();
                                let idx = btn - 14;
                                if idx < presets.len() {
                                    // If an annotation is selected, recolor it
                                    if let Some(sel_idx) = overlay.selected_index {
                                        if sel_idx < overlay.annotations.len() {
                                            let old_ann = overlay.annotations[sel_idx].clone();
                                            recolor_annotation(
                                                &mut overlay.annotations[sel_idx],
                                                presets[idx],
                                            );
                                            record_undo(
                                                &mut overlay.undo_stack,
                                                &mut overlay.redo_stack,
                                                UndoAction::Modify(sel_idx, old_ann),
                                            );
                                        }
                                    } else {
                                        overlay.current_color = presets[idx];
                                        overlay.arrow_tool.set_color(presets[idx]);
                                        overlay.rectangle_tool.set_color(presets[idx]);
                                        overlay.rounded_rect_tool.set_color(presets[idx]);
                                        overlay.circle_tool.set_color(presets[idx]);
                                        overlay.line_tool.set_color(presets[idx]);
                                        overlay.pencil_tool.set_color(presets[idx]);
                                        overlay.highlight_tool.set_color(presets[idx]);
                                        overlay.text_tool.set_color(presets[idx]);
                                        overlay.step_marker_tool.set_color(presets[idx]);
                                        overlay.measurement_tool.set_color(presets[idx]);
                                    }
                                    self.needs_redraw = true;
                                }
                            }
                            19 => {
                                // OCR button
                                self.do_ocr();
                                return;
                            }
                            20 => {
                                // Upload button
                                self.do_upload();
                                return;
                            }
                            21 => {
                                // Pin button
                                self.do_pin(_event_loop);
                                return;
                            }
                            22 => {
                                // Copy button
                                self.do_copy();
                                return;
                            }
                            23 => {
                                // Save button
                                self.do_save();
                                return;
                            }
                            _ => {}
                        }
                        return;
                    }
                }

                // 2. No selection yet — start selecting
                if overlay.selection.is_none() {
                    overlay.is_selecting = true;
                    overlay.drag_start = Some(pos);
                    return;
                }

                // 3. Selection exists — hit-test it
                if let Some(ref sel) = overlay.selection {
                    match sel.hit_test(pos, 8.0) {
                        Some(HitZone::Inside) => {
                            // Start annotation with active tool
                            match overlay.active_tool {
                                ToolKind::Select => {
                                    // Check resize handles first (if an annotation is selected)
                                    if let Some(idx) = overlay.selected_index {
                                        if let Some(ann) = overlay.annotations.get(idx) {
                                            if let Some((bx, by, bw, bh)) =
                                                annotation_bounding_box(ann)
                                            {
                                                let handles = [
                                                    (Point::new(bx, by), ResizeHandle::TopLeft),
                                                    (
                                                        Point::new(bx + bw, by),
                                                        ResizeHandle::TopRight,
                                                    ),
                                                    (
                                                        Point::new(bx, by + bh),
                                                        ResizeHandle::BottomLeft,
                                                    ),
                                                    (
                                                        Point::new(bx + bw, by + bh),
                                                        ResizeHandle::BottomRight,
                                                    ),
                                                ];
                                                for (hp, handle) in &handles {
                                                    if (pos.x - hp.x).abs() < 8.0
                                                        && (pos.y - hp.y).abs() < 8.0
                                                    {
                                                        overlay.resize_handle = Some(*handle);
                                                        // Snapshot annotation before resize for undo
                                                        overlay.pre_drag_annotation = Some((
                                                            idx,
                                                            overlay.annotations[idx].clone(),
                                                        ));
                                                        self.needs_redraw = true;
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Check annotations in REVERSE order (top-most first)
                                    let mut found = None;
                                    for (idx, ann) in overlay.annotations.iter().enumerate().rev() {
                                        if hit_test_annotation(ann, &pos, 8.0) {
                                            found = Some(idx);
                                            break;
                                        }
                                    }
                                    if let Some(idx) = found {
                                        // If clicking an already-selected Text annotation, re-enter edit mode
                                        if overlay.selected_index == Some(idx) {
                                            if let Some(Annotation::Text {
                                                position,
                                                text,
                                                color,
                                                font_size,
                                            }) = overlay.annotations.get(idx).cloned()
                                            {
                                                // Remove the annotation and enter text edit mode with its content
                                                let removed = overlay.annotations.remove(idx);
                                                record_undo(
                                                    &mut overlay.undo_stack,
                                                    &mut overlay.redo_stack,
                                                    UndoAction::Delete(idx, removed),
                                                );
                                                overlay.selected_index = None;
                                                overlay.text_input_active = true;
                                                overlay.text_input_position = position;
                                                overlay.text_input_buffer = text;
                                                overlay.text_input_font_size = font_size;
                                                overlay.current_color = color;
                                                self.needs_redraw = true;
                                                return;
                                            }
                                        }
                                        overlay.selected_index = Some(idx);
                                        overlay.select_drag_start = Some(pos);
                                        // Snapshot annotation before move for undo
                                        overlay.pre_drag_annotation =
                                            Some((idx, overlay.annotations[idx].clone()));
                                    } else {
                                        overlay.selected_index = None;
                                    }
                                }
                                ToolKind::Arrow => overlay.arrow_tool.on_mouse_down(pos),
                                ToolKind::Rectangle => overlay.rectangle_tool.on_mouse_down(pos),
                                ToolKind::RoundedRect => {
                                    overlay.rounded_rect_tool.on_mouse_down(pos)
                                }
                                ToolKind::Circle => overlay.circle_tool.on_mouse_down(pos),
                                ToolKind::Line => overlay.line_tool.on_mouse_down(pos),
                                ToolKind::Measurement => {
                                    overlay.measurement_tool.on_mouse_down(pos)
                                }
                                ToolKind::Pencil => overlay.pencil_tool.on_mouse_down(pos),
                                ToolKind::Highlight => overlay.highlight_tool.on_mouse_down(pos),
                                ToolKind::Text => {
                                    overlay.text_tool.on_mouse_down(pos);
                                    if let Some(p) = overlay.text_tool.take_pending_position() {
                                        overlay.text_input_active = true;
                                        overlay.text_input_position = p;
                                        overlay.text_input_buffer.clear();
                                        overlay.text_input_font_size =
                                            overlay.text_tool.font_size();
                                    }
                                }
                                ToolKind::Pixelate => overlay.pixelate_tool.on_mouse_down(pos),
                                ToolKind::StepMarker => overlay.step_marker_tool.on_mouse_down(pos),
                                ToolKind::Spotlight => overlay.spotlight_tool.on_mouse_down(pos),
                                ToolKind::Eyedropper => {
                                    if let Some(color) = overlay.eyedropper_preview {
                                        overlay.current_color = color;
                                        overlay.arrow_tool.set_color(color);
                                        overlay.rectangle_tool.set_color(color);
                                        overlay.rounded_rect_tool.set_color(color);
                                        overlay.circle_tool.set_color(color);
                                        overlay.line_tool.set_color(color);
                                        overlay.pencil_tool.set_color(color);
                                        overlay.highlight_tool.set_color(color);
                                        overlay.text_tool.set_color(color);
                                        overlay.step_marker_tool.set_color(color);
                                        overlay.measurement_tool.set_color(color);
                                        overlay.active_tool = ToolKind::Arrow;
                                    }
                                }
                            }
                            self.needs_redraw = true;
                        }
                        Some(zone) => {
                            // Start resize
                            overlay.drag_zone = Some(zone);
                        }
                        None => {
                            // Outside selection — clear and start new
                            overlay.selection = None;
                            overlay.is_selecting = true;
                            overlay.drag_start = Some(pos);
                            self.needs_redraw = true;
                        }
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: WinitMouseButton::Left,
                ..
            } => {
                let pos = overlay.last_mouse_pos;

                if overlay.is_selecting {
                    overlay.is_selecting = false;
                    // Finalize selection — ensure minimum size
                    if let Some(start) = overlay.drag_start.take() {
                        let sel = Selection::from_points(start, pos);
                        if sel.width > 2.0 && sel.height > 2.0 {
                            overlay.selection = Some(sel);
                        }
                    }
                    self.needs_redraw = true;
                } else if overlay.drag_zone.is_some() {
                    overlay.drag_zone = None;
                } else {
                    // Finalize annotation
                    let annotation = match overlay.active_tool {
                        ToolKind::Select => {
                            // Record undo for move/resize if annotation changed
                            if let Some((idx, old_ann)) = overlay.pre_drag_annotation.take() {
                                if idx < overlay.annotations.len()
                                    && overlay.annotations[idx] != old_ann
                                {
                                    record_undo(
                                        &mut overlay.undo_stack,
                                        &mut overlay.redo_stack,
                                        UndoAction::Modify(idx, old_ann),
                                    );
                                }
                            }
                            overlay.select_drag_start = None;
                            overlay.resize_handle = None;
                            None
                        }
                        ToolKind::Arrow => overlay.arrow_tool.on_mouse_up(pos),
                        ToolKind::Rectangle => overlay.rectangle_tool.on_mouse_up(pos),
                        ToolKind::RoundedRect => overlay.rounded_rect_tool.on_mouse_up(pos),
                        ToolKind::Circle => overlay.circle_tool.on_mouse_up(pos),
                        ToolKind::Line => overlay.line_tool.on_mouse_up(pos),
                        ToolKind::Measurement => overlay.measurement_tool.on_mouse_up(pos),
                        ToolKind::Pencil => overlay.pencil_tool.on_mouse_up(pos),
                        ToolKind::Highlight => overlay.highlight_tool.on_mouse_up(pos),
                        ToolKind::Text => overlay.text_tool.on_mouse_up(pos),
                        ToolKind::Pixelate => overlay.pixelate_tool.on_mouse_up(pos),
                        ToolKind::StepMarker => overlay.step_marker_tool.on_mouse_up(pos),
                        ToolKind::Spotlight => overlay.spotlight_tool.on_mouse_up(pos),
                        ToolKind::Eyedropper => None,
                    };
                    if let Some(ann) = annotation {
                        overlay.annotations.push(ann);
                        record_undo(
                            &mut overlay.undo_stack,
                            &mut overlay.redo_stack,
                            UndoAction::Add(overlay.annotations.len() - 1),
                        );
                        self.needs_redraw = true;
                    }
                }
            }

            // Right-click: color picker on swatch, otherwise close overlay
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: WinitMouseButton::Right,
                ..
            } => {
                if let AppState::Capturing(ref mut overlay) = self.state {
                    if let Some(ref sel) = overlay.selection {
                        let visible_count = overlay.visible_buttons.len();
                        let toolbar = Toolbar::position_for_dynamic(
                            sel,
                            overlay.screenshot.height as f32,
                            visible_count,
                        );
                        let pos = overlay.last_mouse_pos;
                        if let Some(vis_idx) = toolbar.hit_test_dynamic(pos, visible_count) {
                            let btn = overlay.visible_buttons[vis_idx];
                            if (14..=18).contains(&btn) {
                                let swatch_idx = btn - 14;
                                let current = Color::presets()[swatch_idx];
                                if let Some(ref w) = self.overlay_window {
                                    w.set_visible(false);
                                }
                                let picked = hydroshot::color_picker::pick_color(&current);
                                if let Some(ref w) = self.overlay_window {
                                    w.set_visible(true);
                                }
                                if let Some(new_color) = picked {
                                    overlay.current_color = new_color;
                                    overlay.arrow_tool.set_color(new_color);
                                    overlay.rectangle_tool.set_color(new_color);
                                    overlay.rounded_rect_tool.set_color(new_color);
                                    overlay.circle_tool.set_color(new_color);
                                    overlay.line_tool.set_color(new_color);
                                    overlay.pencil_tool.set_color(new_color);
                                    overlay.highlight_tool.set_color(new_color);
                                    overlay.text_tool.set_color(new_color);
                                    overlay.step_marker_tool.set_color(new_color);
                                    overlay.measurement_tool.set_color(new_color);
                                    if let Some(idx) = overlay.selected_index {
                                        if idx < overlay.annotations.len() {
                                            let old_ann = overlay.annotations[idx].clone();
                                            recolor_annotation(
                                                &mut overlay.annotations[idx],
                                                new_color,
                                            );
                                            record_undo(
                                                &mut overlay.undo_stack,
                                                &mut overlay.redo_stack,
                                                UndoAction::Modify(idx, old_ann),
                                            );
                                        }
                                    }
                                    self.needs_redraw = true;
                                }
                                return;
                            }
                        }
                    }
                }
                if let AppState::Capturing(ref overlay) = self.state {
                    if overlay.selection.is_some() {
                        return;
                    }
                }
                self.close_overlay();
                return;
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 20.0,
                };
                if overlay.text_input_active {
                    // Adjust font size while typing
                    overlay.text_input_font_size =
                        (overlay.text_input_font_size + scroll).clamp(10.0, 72.0);
                } else if overlay.active_tool == ToolKind::StepMarker {
                    let new_size = overlay.step_marker_tool.size() + scroll * 2.0;
                    overlay.step_marker_tool.set_size(new_size);
                } else if overlay.active_tool == ToolKind::RoundedRect {
                    let new_radius = overlay.rounded_rect_tool.radius() + scroll * 2.0;
                    overlay.rounded_rect_tool.set_radius(new_radius);
                } else {
                    let new_thickness = (overlay.current_thickness + scroll).clamp(1.0, 20.0);
                    overlay.current_thickness = new_thickness;
                    overlay.arrow_tool.set_thickness(new_thickness);
                    overlay.rectangle_tool.set_thickness(new_thickness);
                    overlay.rounded_rect_tool.set_thickness(new_thickness);
                    overlay.circle_tool.set_thickness(new_thickness);
                    overlay.line_tool.set_thickness(new_thickness);
                    overlay.pencil_tool.set_thickness(new_thickness);
                }
                self.needs_redraw = true;
            }

            _ => {}
        }

        if self.needs_redraw {
            if let Some(w) = &self.overlay_window {
                w.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_tray_events(event_loop);

        if let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            if Some(event.id) == self.hotkey_id {
                tracing::info!("Global hotkey pressed — triggering capture");
                self.trigger_capture(event_loop);
            }
        }

        // Update countdown overlay
        if let Some(next_tick) = self.countdown_next_tick {
            if Instant::now() >= next_tick {
                self.countdown_remaining = self.countdown_remaining.saturating_sub(1);
                if self.countdown_remaining > 0 {
                    self.render_countdown();
                    self.countdown_next_tick = Some(Instant::now() + Duration::from_secs(1));
                } else {
                    self.close_countdown();
                    // Schedule capture 300ms from now — gives the OS time to
                    // fully remove the countdown window from the screen
                    self.capture_at = Some(Instant::now() + Duration::from_millis(300));
                }
            }
            if let Some(next) = self.countdown_next_tick {
                event_loop.set_control_flow(ControlFlow::WaitUntil(next));
            }
        }

        if let Some(capture_time) = self.capture_at {
            if Instant::now() >= capture_time {
                self.capture_at = None;
                self.trigger_capture(event_loop);
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(capture_time));
            }
        } else if !self.needs_redraw {
            // Check if there's an active toast that needs to expire
            if let AppState::Capturing(ref overlay) = self.state {
                if let Some(until) = overlay.toast_until {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(until));
                    self.needs_redraw = true;
                } else {
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }

        // Frame rate cap: only render if enough time has passed since last frame
        if self.needs_redraw {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_render);
            if elapsed >= FRAME_INTERVAL {
                self.render();
                self.last_render = now;
            } else {
                // Schedule wake-up for next frame
                let next_frame = self.last_render + FRAME_INTERVAL;
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_frame));
            }
        }
    }
}

fn run_tray_app(config: Config) {
    // Single-instance enforcement: only one tray app at a time
    #[cfg(target_os = "windows")]
    {
        use windows::core::w;
        use windows::Win32::Foundation::GetLastError;
        use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
        use windows::Win32::System::Threading::CreateMutexW;
        unsafe {
            if let Ok(m) = CreateMutexW(None, false, w!("HydroShot.SingleInstance")) {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    let _ = windows::Win32::Foundation::CloseHandle(m);
                    let _ = notify_rust::Notification::new()
                        .summary("HydroShot")
                        .body("HydroShot is already running in the system tray")
                        .timeout(3000)
                        .show();
                    return;
                }
                // Mutex handle is Copy — kernel keeps it alive for the process lifetime
                let _ = m;
            }
        }
    }

    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            hydroshot::installer::show_error(&format!("Failed to start: {e}"));
            std::process::exit(1);
        }
    };
    let mut app = App::new(config);
    if let Err(e) = event_loop.run_app(&mut app) {
        hydroshot::installer::show_error(&format!("Event loop error: {e}"));
        std::process::exit(1);
    }
}

fn run_tray_app_with_immediate_capture(config: Config) {
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            hydroshot::installer::show_error(&format!("Failed to start: {e}"));
            std::process::exit(1);
        }
    };
    let mut app = App::new(config);
    app.immediate_capture = true;
    app.cli_only = true;
    if let Err(e) = event_loop.run_app(&mut app) {
        hydroshot::installer::show_error(&format!("Event loop error: {e}"));
        std::process::exit(1);
    }
}

fn run_cli_capture(clipboard: bool, save: Option<String>, delay: u64) {
    if delay > 0 {
        tracing::info!("Waiting {} seconds...", delay);
        std::thread::sleep(std::time::Duration::from_secs(delay));
    }

    // If neither --clipboard nor --save: open interactive overlay
    if !clipboard && save.is_none() {
        tracing::info!("Opening interactive capture...");
        let config = Config::load();
        run_tray_app_with_immediate_capture(config);
        return;
    }

    // Non-interactive: capture full screen directly
    let capturer = match capture::create_capturer() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create capturer: {}", e);
            std::process::exit(1);
        }
    };

    let screens = match capturer.capture_all_screens() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Capture failed: {}", e);
            std::process::exit(1);
        }
    };

    if screens.is_empty() {
        eprintln!("No screens captured");
        std::process::exit(1);
    }

    let screen = &screens[0];

    if clipboard {
        match export::copy_to_clipboard(&screen.pixels, screen.width, screen.height) {
            Ok(_) => {
                println!(
                    "Copied {}x{} screenshot to clipboard",
                    screen.width, screen.height
                );
            }
            Err(e) => {
                eprintln!("Clipboard error: {}", e);
                std::process::exit(1);
            }
        }
    } else if let Some(path) = save {
        let img =
            match image::RgbaImage::from_raw(screen.width, screen.height, screen.pixels.clone()) {
                Some(img) => img,
                None => {
                    eprintln!("Invalid image data ({}x{})", screen.width, screen.height);
                    std::process::exit(1);
                }
            };
        match img.save(&path) {
            Ok(_) => println!("Saved to {}", path),
            Err(e) => {
                eprintln!("Save error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("HydroShot starting");

    // Set Windows Application User Model ID so toast notifications display
    #[cfg(target_os = "windows")]
    {
        use windows::core::w;
        use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
        unsafe {
            let _ = SetCurrentProcessExplicitAppUserModelID(w!("HydroShot.HydroShot"));
        }
    }

    let cli = Cli::parse();

    // For CLI subcommands, attach to the parent console so println!/eprintln!
    // produce visible output (windows_subsystem = "windows" detaches stdout).
    #[cfg(target_os = "windows")]
    if cli.command.is_some() {
        use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        unsafe {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }

    match cli.command {
        None => {
            // If not running from the install location, install first (then the
            // installer spawns the installed copy and we exit).
            if hydroshot::installer::needs_install() {
                if let Err(e) = hydroshot::installer::install() {
                    hydroshot::installer::show_error(&format!("Install failed: {e}"));
                    std::process::exit(1);
                }
                return;
            }

            let config = Config::load();
            tracing::info!(
                "Config loaded: hotkey={}, color={}, thickness={}",
                config.hotkey.capture,
                config.general.default_color,
                config.general.default_thickness
            );
            run_tray_app(config);
        }
        Some(Commands::Capture {
            clipboard,
            save,
            delay,
        }) => {
            run_cli_capture(clipboard, save, delay);
        }
        Some(Commands::Install) => {
            if let Err(e) = hydroshot::installer::install() {
                hydroshot::installer::show_error(&format!("Install failed: {e}"));
                std::process::exit(1);
            }
        }
        Some(Commands::Uninstall) => {
            if let Err(e) = hydroshot::installer::uninstall() {
                hydroshot::installer::show_error(&format!("Uninstall failed: {e}"));
                std::process::exit(1);
            }
        }
    }
}
