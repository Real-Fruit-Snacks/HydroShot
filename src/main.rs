use std::num::NonZeroU32;
use std::sync::Arc;

use tray_icon::menu::MenuEvent;
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
use winit::event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

use winit::application::ApplicationHandler;

use hydroshot::capture;
use hydroshot::export;
use hydroshot::geometry::{Color, Point};
use hydroshot::overlay::selection::{HitZone, Selection};
use hydroshot::overlay::toolbar::Toolbar;
use hydroshot::renderer::render_overlay;
use hydroshot::state::{AppState, OverlayState};
use hydroshot::tools::{AnnotationTool, ToolKind};
use hydroshot::tray::{self, TrayState};

struct App {
    state: AppState,
    tray: Option<TrayState>,
    overlay_window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    pixmap: Option<tiny_skia::Pixmap>,
    modifiers: ModifiersState,
    needs_redraw: bool,
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState::Idle,
            tray: None,
            overlay_window: None,
            surface: None,
            pixmap: None,
            modifiers: ModifiersState::empty(),
            needs_redraw: false,
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
            .with_fullscreen(Some(Fullscreen::Borderless(None)))
            .with_decorations(false)
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

        self.state = AppState::Capturing(Box::new(OverlayState::new(screenshot)));
        self.surface = Some(surface);
        self.pixmap = pixmap;
        self.overlay_window = Some(window);
        self.needs_redraw = true;

        if let Some(w) = &self.overlay_window {
            w.request_redraw();
        }
    }

    fn close_overlay(&mut self) {
        self.surface = None;
        self.pixmap = None;
        self.overlay_window = None;
        self.state = AppState::Idle;
        self.modifiers = ModifiersState::empty();
        tracing::info!("Overlay closed");
    }

    fn do_copy(&mut self) {
        if let AppState::Capturing(ref overlay) = self.state {
            if let Some(ref sel) = overlay.selection {
                let pixels = export::crop_and_flatten(
                    &overlay.screenshot.pixels,
                    overlay.screenshot.width,
                    sel.x as u32,
                    sel.y as u32,
                    sel.width as u32,
                    sel.height as u32,
                    &overlay.annotations,
                );
                match export::copy_to_clipboard(&pixels, sel.width as u32, sel.height as u32) {
                    Ok(()) => tracing::info!("Copied to clipboard"),
                    Err(e) => tracing::error!("Clipboard copy failed: {e}"),
                }
                self.close_overlay();
            }
        }
    }

    fn do_save(&mut self) {
        if let AppState::Capturing(ref overlay) = self.state {
            if let Some(ref sel) = overlay.selection {
                let pixels = export::crop_and_flatten(
                    &overlay.screenshot.pixels,
                    overlay.screenshot.width,
                    sel.x as u32,
                    sel.y as u32,
                    sel.width as u32,
                    sel.height as u32,
                    &overlay.annotations,
                );
                match export::save_to_file(&pixels, sel.width as u32, sel.height as u32) {
                    Ok(Some(path)) => {
                        tracing::info!("Saved to {path}");
                        self.close_overlay();
                    }
                    Ok(None) => {
                        tracing::info!("Save cancelled by user");
                        // Don't close overlay on cancel
                    }
                    Err(e) => tracing::error!("Save failed: {e}"),
                }
            }
        }
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
                } else if event.id == tray.quit_id {
                    tracing::info!("Quit requested");
                    event_loop.exit();
                } else if event.id == tray.about_id {
                    tracing::info!(
                        "HydroShot v{} — a screenshot annotation tool",
                        env!("CARGO_PKG_VERSION")
                    );
                }
            }
        }
    }

    fn render(&mut self) {
        if !self.needs_redraw {
            return;
        }

        let overlay = match &self.state {
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

        let pm_pixels = pixmap.pixels();
        let pm_w = pixmap.width() as usize;
        let pm_h = pixmap.height() as usize;
        let buf_w = w as usize;
        let buf_h = h as usize;
        let copy_w = pm_w.min(buf_w);
        let copy_h = pm_h.min(buf_h);

        // Clear buffer first (black)
        for pixel in buffer.iter_mut() {
            *pixel = 0;
        }

        // Copy pixmap to softbuffer, converting premultiplied RGBA to 0x00RRGGBB
        for y in 0..copy_h {
            for x in 0..copy_w {
                let px = pm_pixels[y * pm_w + x];
                let d = px.demultiply();
                buffer[y * buf_w + x] =
                    ((d.red() as u32) << 16) | ((d.green() as u32) << 8) | (d.blue() as u32);
            }
        }

        if let Err(e) = buffer.present() {
            tracing::error!("Buffer present failed: {e}");
        }

        self.needs_redraw = false;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        tracing::info!("Application resumed");
        event_loop.set_control_flow(ControlFlow::Wait);

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
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
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

                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.close_overlay();
                        return;
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
                                if let Some(ann) = overlay.redo_buffer.pop() {
                                    overlay.annotations.push(ann);
                                    self.needs_redraw = true;
                                }
                            }
                            "z" if ctrl => {
                                // Undo
                                let overlay = match &mut self.state {
                                    AppState::Capturing(o) => o,
                                    _ => return,
                                };
                                if let Some(ann) = overlay.annotations.pop() {
                                    overlay.redo_buffer.push(ann);
                                    self.needs_redraw = true;
                                }
                            }
                            "Z" if ctrl => {
                                // Ctrl+Shift+Z on some platforms sends uppercase Z
                                let overlay = match &mut self.state {
                                    AppState::Capturing(o) => o,
                                    _ => return,
                                };
                                if let Some(ann) = overlay.redo_buffer.pop() {
                                    overlay.annotations.push(ann);
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
                        ToolKind::Pencil => {
                            if overlay.pencil_tool.is_drawing() {
                                overlay.pencil_tool.on_mouse_move(pos);
                                self.needs_redraw = true;
                            }
                        }
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: WinitMouseButton::Left,
                ..
            } => {
                let pos = overlay.last_mouse_pos;

                // 1. Check toolbar hit first (only if selection exists)
                if let Some(ref sel) = overlay.selection {
                    let toolbar = Toolbar::position_for(sel, overlay.screenshot.height as f32);
                    if let Some(btn) = toolbar.hit_test(pos) {
                        match btn {
                            0 => {
                                overlay.active_tool = ToolKind::Arrow;
                                self.needs_redraw = true;
                            }
                            1 => {
                                overlay.active_tool = ToolKind::Rectangle;
                                self.needs_redraw = true;
                            }
                            2..=6 => {
                                let presets = Color::presets();
                                let idx = btn - 2;
                                if idx < presets.len() {
                                    overlay.current_color = presets[idx];
                                    overlay.arrow_tool.set_color(presets[idx]);
                                    overlay.rectangle_tool.set_color(presets[idx]);
                                    overlay.pencil_tool.set_color(presets[idx]);
                                    self.needs_redraw = true;
                                }
                            }
                            7 => {
                                // Copy button
                                self.do_copy();
                                return;
                            }
                            8 => {
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
                                ToolKind::Arrow => overlay.arrow_tool.on_mouse_down(pos),
                                ToolKind::Rectangle => overlay.rectangle_tool.on_mouse_down(pos),
                                ToolKind::Pencil => overlay.pencil_tool.on_mouse_down(pos),
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
                        ToolKind::Arrow => overlay.arrow_tool.on_mouse_up(pos),
                        ToolKind::Rectangle => overlay.rectangle_tool.on_mouse_up(pos),
                        ToolKind::Pencil => overlay.pencil_tool.on_mouse_up(pos),
                    };
                    if let Some(ann) = annotation {
                        overlay.annotations.push(ann);
                        overlay.redo_buffer.clear();
                        self.needs_redraw = true;
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 20.0,
                };
                let new_thickness = (overlay.current_thickness + scroll).clamp(1.0, 20.0);
                overlay.current_thickness = new_thickness;
                overlay.arrow_tool.set_thickness(new_thickness);
                overlay.rectangle_tool.set_thickness(new_thickness);
                overlay.pencil_tool.set_thickness(new_thickness);
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
        self.render();
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("HydroShot starting");

    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}
