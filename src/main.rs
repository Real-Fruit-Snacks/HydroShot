#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use tray_icon::menu::MenuEvent;
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
use winit::event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId, WindowLevel};

use winit::application::ApplicationHandler;

use notify_rust::Notification;

use hydroshot::capture;
use hydroshot::cli::{Cli, Commands};
use hydroshot::config::Config;
use hydroshot::countdown::Countdown;
use hydroshot::export;
use hydroshot::geometry::{Color, Point};
use hydroshot::history_ui::HistoryWindow;
use hydroshot::overlay::selection::{HitZone, Selection};
use hydroshot::overlay::toolbar::{ButtonAction, Toolbar, BUTTONS};
use hydroshot::pin::PinnedWindow;
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

struct App {
    config: Config,
    state: AppState,
    tray: Option<TrayState>,
    overlay_window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    pixmap: Option<tiny_skia::Pixmap>,
    modifiers: ModifiersState,
    /// Modifier state tracked separately for the settings window (used while
    /// capturing a new global-hotkey combination).
    settings_modifiers: ModifiersState,
    needs_redraw: bool,
    last_render: Instant,
    capture_at: Option<Instant>,
    _hotkey_manager: Option<global_hotkey::GlobalHotKeyManager>,
    hotkey_id: Option<u32>,
    pinned_windows: Vec<PinnedWindow>,
    immediate_capture: bool,
    cli_only: bool,
    startup_notified: bool,
    countdown: Option<Countdown>,
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
            settings_modifiers: ModifiersState::empty(),
            needs_redraw: false,
            last_render: Instant::now(),
            capture_at: None,
            _hotkey_manager: None,
            hotkey_id: None,
            pinned_windows: Vec::new(),
            immediate_capture: false,
            cli_only: false,
            startup_notified: false,
            countdown: None,
            window_capture_mode: false,
            window_rects: Vec::new(),
            settings_window: None,
            history_window: None,
        }
    }

    /// Register the global capture hotkey from config, falling back to the
    /// default binding when the configured one fails to parse or register.
    fn register_hotkey_with_fallback(&mut self) {
        // Drop any previous registration first.
        self._hotkey_manager = None;
        self.hotkey_id = None;

        let binding = self.config.hotkey.capture.clone();
        match hydroshot::hotkey::register_hotkey(&binding) {
            Ok((manager, id)) => {
                self._hotkey_manager = Some(manager);
                self.hotkey_id = Some(id);
                tracing::info!("Global hotkey registered: {}", binding);
            }
            Err(e) => {
                tracing::warn!("Failed to register hotkey '{}': {}", binding, e);
                const DEFAULT: &str = "Ctrl+Shift+S";
                if binding != DEFAULT {
                    match hydroshot::hotkey::register_hotkey(DEFAULT) {
                        Ok((manager, id)) => {
                            self._hotkey_manager = Some(manager);
                            self.hotkey_id = Some(id);
                            tracing::info!("Fell back to default hotkey {DEFAULT}");
                            let _ = Notification::new()
                                .summary("HydroShot")
                                .body(&format!(
                                    "Hotkey '{binding}' could not be registered — using {DEFAULT}"
                                ))
                                .timeout(4000)
                                .show();
                        }
                        Err(e2) => tracing::warn!("Default hotkey also failed: {e2}"),
                    }
                }
            }
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
            let old_binding = self.config.hotkey.capture.clone();
            self.config = Config::load(); // reload saved config
            if self.config.hotkey.capture != old_binding && !self.cli_only {
                self.register_hotkey_with_fallback();
            }
        }
        // Settings can toggle autostart — keep the tray checkbox in sync.
        if let Some(ref tray) = self.tray {
            tray.autostart_check
                .set_checked(hydroshot::autostart::is_enabled());
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
            .with_inner_size(winit::dpi::LogicalSize::new(
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
            overlay.flattened_selection()
        } else {
            None
        };

        if let Some((pixels, w, h)) = copy_data {
            let history_enabled = self.config.general.history_enabled;
            self.close_overlay();
            std::thread::spawn(move || match export::copy_to_clipboard(&pixels, w, h) {
                Ok(()) => {
                    if history_enabled {
                        let _ = hydroshot::history::save_to_history(&pixels, w, h);
                    }
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
            overlay
                .flattened_selection()
                .map(|(pixels, w, h)| (pixels, w, h, self.config.save_directory()))
        } else {
            None
        };

        if let Some((pixels, w, h, save_dir)) = save_data {
            let history_enabled = self.config.general.history_enabled;
            self.close_overlay();
            std::thread::spawn(move || {
                match export::save_to_file(pixels, w, h, save_dir.as_deref()) {
                    Ok(Some(path)) => {
                        // Copy saved file to history instead of re-encoding
                        if history_enabled {
                            let _ = hydroshot::history::save_to_history_from_file(
                                std::path::Path::new(&path),
                            );
                        }
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
            o.flattened_selection()
        } else {
            None
        };

        if let Some((pixels, w, h)) = upload_data {
            if self.config.general.history_enabled {
                let _ = hydroshot::history::save_to_history(&pixels, w, h);
            }

            // Close overlay immediately; PNG encoding and the upload both run
            // on a background thread so the UI never stalls on large captures.
            let imgur_id = self.config.general.imgur_client_id.clone();
            self.close_overlay();

            std::thread::spawn(move || {
                let png_bytes = match encode_png(pixels, w, h) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        tracing::error!("PNG encode failed: {e}");
                        let _ = Notification::new()
                            .summary("HydroShot")
                            .body(&format!("Upload failed: {e}"))
                            .timeout(3000)
                            .show();
                        return;
                    }
                };

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
        // OCR reads the raw screenshot pixels (annotations would confuse it)
        let ocr_data = if let AppState::Capturing(ref overlay) = self.state {
            overlay.raw_selection()
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
                        let char_count = text.chars().count();
                        tracing::info!("OCR extracted {} chars", char_count);
                        if char_count > 80 {
                            format!(
                                "Copied {} chars: {}...",
                                char_count,
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
            if let Some((pixels, w, h)) = overlay.flattened_selection() {
                if self.config.general.history_enabled {
                    let _ = hydroshot::history::save_to_history(&pixels, w, h);
                }

                // Position the pin where the selection is on screen, accounting
                // for the virtual-desktop offset of the captured area (the
                // selection is in screenshot coordinates, which can start at a
                // negative virtual-desktop position on multi-monitor setups).
                if let Some(ref sel) = overlay.selection {
                    let screen_x = sel.x as i32 + overlay.screenshot.x_offset;
                    let screen_y = sel.y as i32 + overlay.screenshot.y_offset;
                    if let Some(pin) =
                        PinnedWindow::create(event_loop, &pixels, w, h, screen_x, screen_y)
                    {
                        self.pinned_windows.push(pin);
                    }
                }
            }
        }
        self.close_overlay();
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
                    self.countdown = Countdown::start(event_loop, 3);
                } else if event.id == tray.delay_5_id {
                    tracing::info!("Capturing in 5 seconds...");
                    self.countdown = Countdown::start(event_loop, 5);
                } else if event.id == tray.delay_10_id {
                    tracing::info!("Capturing in 10 seconds...");
                    self.countdown = Countdown::start(event_loop, 10);
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
                        // CREATE_NO_WINDOW — cmd.exe would otherwise flash a console
                        use std::os::windows::process::CommandExt;
                        const CREATE_NO_WINDOW: u32 = 0x08000000;
                        let _ = std::process::Command::new("cmd")
                            .args([
                                "/C",
                                "start",
                                "https://github.com/Real-Fruit-Snacks/HydroShot",
                            ])
                            .creation_flags(CREATE_NO_WINDOW)
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

/// Encode RGBA pixels to PNG bytes.
fn encode_png(pixels: Vec<u8>, w: u32, h: u32) -> Result<Vec<u8>, String> {
    let img = image::RgbaImage::from_raw(w, h, pixels).ok_or("invalid image data")?;
    let mut png_bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .map_err(|e| e.to_string())?;
    Ok(png_bytes)
}

/// Build a hotkey binding string (e.g. "Ctrl+Shift+S") from the current
/// modifier state and a pressed key. Returns None for keys that can't be a
/// global hotkey or when no Ctrl/Alt/Super modifier is held.
fn hotkey_binding_from_key(mods: ModifiersState, key: &Key) -> Option<String> {
    let key_part: String = match key {
        Key::Character(ch) => {
            let s = ch.as_str();
            let c = s.chars().next()?;
            if s.chars().count() == 1 && (c.is_ascii_alphabetic() || c.is_ascii_digit()) {
                c.to_ascii_uppercase().to_string()
            } else {
                return None;
            }
        }
        Key::Named(named) => match named {
            NamedKey::F1 => "F1".into(),
            NamedKey::F2 => "F2".into(),
            NamedKey::F3 => "F3".into(),
            NamedKey::F4 => "F4".into(),
            NamedKey::F5 => "F5".into(),
            NamedKey::F6 => "F6".into(),
            NamedKey::F7 => "F7".into(),
            NamedKey::F8 => "F8".into(),
            NamedKey::F9 => "F9".into(),
            NamedKey::F10 => "F10".into(),
            NamedKey::F11 => "F11".into(),
            NamedKey::F12 => "F12".into(),
            NamedKey::PrintScreen => "PrintScreen".into(),
            _ => return None,
        },
        _ => return None,
    };

    // Require a strong modifier so the hotkey can't swallow normal typing.
    if !(mods.control_key() || mods.alt_key() || mods.super_key()) {
        return None;
    }

    let mut parts: Vec<String> = Vec::new();
    if mods.control_key() {
        parts.push("Ctrl".into());
    }
    if mods.shift_key() {
        parts.push("Shift".into());
    }
    if mods.alt_key() {
        parts.push("Alt".into());
    }
    if mods.super_key() {
        parts.push("Super".into());
    }
    parts.push(key_part);
    let binding = parts.join("+");

    // Final validation through the real parser
    hydroshot::hotkey::parse_binding(&binding).ok()?;
    Some(binding)
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
                self.register_hotkey_with_fallback();
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
            .position(|p| p.window_id() == _window_id)
        {
            if self.pinned_windows[pin_idx].handle_event(&event) {
                // PinnedWindow::drop removes the backing temp file.
                self.pinned_windows.remove(pin_idx);
                tracing::info!("Pinned window closed");
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
                        if let Some(ref mut hw) = self.history_window {
                            if hw.on_click() {
                                hw.render();
                            }
                        }
                        return;
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let scroll = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y,
                            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 20.0,
                        };
                        if let Some(ref mut hw) = self.history_window {
                            if hw.on_scroll(scroll) {
                                hw.window.request_redraw();
                            }
                        }
                        return;
                    }
                    WindowEvent::KeyboardInput { ref event, .. }
                        if event.state == ElementState::Pressed =>
                    {
                        if let Key::Named(NamedKey::Escape) = &event.logical_key {
                            self.close_history();
                            return;
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
                    WindowEvent::ModifiersChanged(mods) => {
                        self.settings_modifiers = mods.state();
                        return;
                    }
                    WindowEvent::KeyboardInput { ref event, .. }
                        if event.state == ElementState::Pressed =>
                    {
                        // If rebinding the global hotkey, capture the combo
                        if let Some(ref mut sw) = self.settings_window {
                            if sw.editing_hotkey {
                                if let Key::Named(NamedKey::Escape) = &event.logical_key {
                                    sw.editing_hotkey = false;
                                    sw.needs_redraw = true;
                                    sw.window.request_redraw();
                                    return;
                                }
                                if let Some(binding) = hotkey_binding_from_key(
                                    self.settings_modifiers,
                                    &event.logical_key,
                                ) {
                                    sw.config.hotkey.capture = binding;
                                    sw.editing_hotkey = false;
                                    sw.needs_redraw = true;
                                    sw.window.request_redraw();
                                }
                                // Bare keys / unsupported keys keep edit mode active
                                return;
                            }
                        }

                        // If editing a shortcut, capture the key
                        if let Some(ref mut sw) = self.settings_window {
                            if sw.editing_shortcut.is_some() {
                                let key_str = match &event.logical_key {
                                    Key::Character(ch) => Some(ch.as_str().to_lowercase()),
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
                    _ => {}
                }
                return;
            }
        }

        // Check if event is for the countdown window
        if let Some(ref mut cd) = self.countdown {
            if cd.window_id() == _window_id {
                match event {
                    WindowEvent::CloseRequested => {
                        self.countdown = None;
                        self.capture_at = None; // cancel the delayed capture
                        return;
                    }
                    WindowEvent::RedrawRequested => {
                        cd.render();
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
                    let ctrl = self.modifiers.control_key();
                    match &event.logical_key {
                        Key::Character(ch) if ctrl && ch.as_str().eq_ignore_ascii_case("v") => {
                            // Ctrl+V: paste clipboard text into the buffer
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                if let Ok(text) = clipboard.get_text() {
                                    // Single-line annotation — fold newlines
                                    let text = text.replace(['\r', '\n'], " ");
                                    overlay.text_input_buffer.push_str(&text);
                                    self.needs_redraw = true;
                                }
                            }
                        }
                        Key::Character(_) if ctrl => {
                            // Swallow other Ctrl chords so they don't insert
                            // literal characters while typing.
                        }
                        Key::Character(ch) => {
                            overlay.text_input_buffer.push_str(ch.as_str());
                            self.needs_redraw = true;
                        }
                        Key::Named(NamedKey::Space) => {
                            overlay.text_input_buffer.push(' ');
                            self.needs_redraw = true;
                        }
                        Key::Named(NamedKey::Backspace) => {
                            overlay.text_input_buffer.pop();
                            self.needs_redraw = true;
                        }
                        Key::Named(NamedKey::Enter) => {
                            let buffer = overlay.text_input_buffer.clone();
                            if let Some((idx, old_ann)) = overlay.text_edit_origin.take() {
                                // Re-edit of an existing annotation: one Modify
                                // step (or a Delete when the text was cleared).
                                let idx = idx.min(overlay.annotations.len());
                                if !buffer.is_empty() {
                                    let ann = Annotation::Text {
                                        position: overlay.text_input_position,
                                        text: buffer,
                                        color: overlay.current_color,
                                        font_size: overlay.text_input_font_size,
                                    };
                                    overlay.annotations.insert(idx, ann);
                                    record_undo(
                                        &mut overlay.undo_stack,
                                        &mut overlay.redo_stack,
                                        UndoAction::Modify(idx, old_ann),
                                    );
                                } else {
                                    record_undo(
                                        &mut overlay.undo_stack,
                                        &mut overlay.redo_stack,
                                        UndoAction::Delete(idx, old_ann),
                                    );
                                }
                            } else if !buffer.is_empty() {
                                let ann = Annotation::Text {
                                    position: overlay.text_input_position,
                                    text: buffer,
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
                            // Cancel text input; restore the original annotation
                            // if this was a re-edit.
                            if let Some((idx, old_ann)) = overlay.text_edit_origin.take() {
                                let idx = idx.min(overlay.annotations.len());
                                overlay.annotations.insert(idx, old_ann);
                            }
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
                                overlay.sync_step_counter();
                                self.needs_redraw = true;
                            }
                        }
                    }
                    Key::Named(NamedKey::Enter) if overlay.selection.is_some() => {
                        // Quick copy — same path as Ctrl+C / the Copy button,
                        // annotations included, clipboard work off-thread.
                        self.do_copy();
                        return;
                    }
                    Key::Character(ch) => {
                        let ctrl = self.modifiers.control_key();
                        let shift = self.modifiers.shift_key();
                        // Compare case-insensitively so Caps Lock / Shift don't
                        // break shortcuts ("Z" vs "z"); redo is decided by the
                        // actual Shift modifier, not the produced character.
                        let key_lower = ch.as_str().to_lowercase();
                        match key_lower.as_str() {
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
                                    overlay.sync_step_counter();
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
                                    overlay.sync_step_counter();
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
                            // Tool keyboard shortcuts (no Ctrl) — config-driven,
                            // case-insensitive so Caps Lock doesn't disable them.
                            key_str if !ctrl => {
                                let shortcuts = &self.config.shortcuts;
                                let eq = |bound: &str| key_str.eq_ignore_ascii_case(bound);
                                let new_tool = if eq(&shortcuts.select) {
                                    Some(ToolKind::Select)
                                } else if eq(&shortcuts.arrow) {
                                    Some(ToolKind::Arrow)
                                } else if eq(&shortcuts.rectangle) {
                                    Some(ToolKind::Rectangle)
                                } else if eq(&shortcuts.circle) {
                                    Some(ToolKind::Circle)
                                } else if eq(&shortcuts.rounded_rect) {
                                    Some(ToolKind::RoundedRect)
                                } else if eq(&shortcuts.line) {
                                    Some(ToolKind::Line)
                                } else if eq(&shortcuts.pencil) {
                                    Some(ToolKind::Pencil)
                                } else if eq(&shortcuts.highlight) {
                                    Some(ToolKind::Highlight)
                                } else if eq(&shortcuts.spotlight) {
                                    Some(ToolKind::Spotlight)
                                } else if eq(&shortcuts.text) {
                                    Some(ToolKind::Text)
                                } else if eq(&shortcuts.pixelate) {
                                    Some(ToolKind::Pixelate)
                                } else if eq(&shortcuts.step_marker) {
                                    Some(ToolKind::StepMarker)
                                } else if eq(&shortcuts.eyedropper) {
                                    Some(ToolKind::Eyedropper)
                                } else if eq(&shortcuts.measurement) {
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
                        let action = BUTTONS.get(btn).map(|d| d.action);
                        // Reset upload confirmation if clicking anything other than Upload
                        if action != Some(ButtonAction::Upload) {
                            overlay.upload_confirmed = false;
                        }
                        match action {
                            Some(ButtonAction::Tool(kind)) => {
                                overlay.active_tool = kind;
                                if kind != ToolKind::Select {
                                    overlay.selected_index = None;
                                }
                                self.needs_redraw = true;
                            }
                            Some(ButtonAction::Color(idx)) => {
                                let presets = Color::presets();
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
                                        overlay.set_color_all(presets[idx]);
                                    }
                                    self.needs_redraw = true;
                                }
                            }
                            Some(ButtonAction::Ocr) => {
                                self.do_ocr();
                                return;
                            }
                            Some(ButtonAction::Upload) => {
                                self.do_upload();
                                return;
                            }
                            Some(ButtonAction::Pin) => {
                                self.do_pin(_event_loop);
                                return;
                            }
                            Some(ButtonAction::Copy) => {
                                self.do_copy();
                                return;
                            }
                            Some(ButtonAction::Save) => {
                                self.do_save();
                                return;
                            }
                            None => {}
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
                                                // Take the annotation out and remember it: committing
                                                // records ONE Modify undo step; Escape restores it.
                                                let removed = overlay.annotations.remove(idx);
                                                overlay.text_edit_origin = Some((idx, removed));
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
                                        overlay.set_color_all(color);
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
                            if let Some(ButtonAction::Color(swatch_idx)) =
                                BUTTONS.get(btn).map(|d| d.action)
                            {
                                let current = Color::presets()
                                    .get(swatch_idx)
                                    .copied()
                                    .unwrap_or_else(Color::red);
                                if let Some(ref w) = self.overlay_window {
                                    w.set_visible(false);
                                }
                                let picked = hydroshot::color_picker::pick_color(&current);
                                if let Some(ref w) = self.overlay_window {
                                    w.set_visible(true);
                                }
                                if let Some(new_color) = picked {
                                    overlay.set_color_all(new_color);
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
                    overlay.set_thickness_all(overlay.current_thickness + scroll);
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

        // Drain ALL pending hotkey events, not just one per wakeup
        while let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            if Some(event.id) == self.hotkey_id {
                tracing::info!("Global hotkey pressed — triggering capture");
                self.trigger_capture(event_loop);
            }
        }

        // Apply async folder-picker results for the settings window.
        if let Some(ref mut sw) = self.settings_window {
            if sw.poll_browse() {
                sw.window.request_redraw();
            }
        }

        // Update countdown overlay
        if let Some(ref mut cd) = self.countdown {
            let now = Instant::now();
            if cd.tick(now) {
                // Hide before dropping so the window is gone from the screen
                // before the screenshot is taken.
                cd.hide();
                self.countdown = None;
                // Schedule capture 300ms from now — gives the OS time to
                // fully remove the countdown window from the screen
                self.capture_at = Some(now + Duration::from_millis(300));
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(cd.next_tick()));
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
            // Wake exactly at toast expiry to clear it — don't re-render before
            if let AppState::Capturing(ref overlay) = self.state {
                if let Some(until) = overlay.toast_until {
                    if Instant::now() >= until {
                        self.needs_redraw = true; // expired — render to clear it
                    } else {
                        event_loop.set_control_flow(ControlFlow::WaitUntil(until));
                    }
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

        // While a folder-picker dialog is open we must self-schedule wakeups —
        // the dialog produces no winit events. This is set LAST so the idle
        // ControlFlow::Wait above can't overwrite it (last write wins).
        if self
            .settings_window
            .as_ref()
            .is_some_and(|sw| sw.browse_pending())
        {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(100),
            ));
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

    // --clipboard and --save are independent; both can be given at once.
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
    }
    if let Some(path) = save {
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
            // If not running from the install location, OFFER to install (the
            // installer spawns the installed copy and we exit). Declining runs
            // the app portably from its current location. needs_install() is
            // always false in debug builds and on non-Windows platforms.
            if hydroshot::installer::needs_install() && hydroshot::installer::confirm_install() {
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
