use std::num::NonZeroU32;
use std::sync::Arc;

use tiny_skia::{Color as SkiaColor, Paint, Pixmap, Rect, Transform};
use winit::window::Window;

use crate::config::Config;
use crate::geometry::{Color, Point};
use crate::tools::render_text_annotation;

/// Width / height of the settings window in physical pixels.
pub const WIN_W: u32 = 420;
pub const WIN_H: u32 = 520;

// Catppuccin Mocha palette
const BASE: (u8, u8, u8) = (0x1e, 0x1e, 0x2e);
const SURFACE0: (u8, u8, u8) = (0x31, 0x32, 0x44);
const SURFACE1: (u8, u8, u8) = (0x45, 0x47, 0x5a);
const TEXT_RGB: (u8, u8, u8) = (0xcd, 0xd6, 0xf4);
const LAVENDER: (u8, u8, u8) = (0xb4, 0xbe, 0xfe);
const GREEN_RGB: (u8, u8, u8) = (0xa6, 0xe3, 0xa1);
const SUBTEXT0: (u8, u8, u8) = (0xa6, 0xad, 0xc8);

/// Color choices available in settings (name, R, G, B).
const COLOR_CHOICES: &[(&str, u8, u8, u8)] = &[
    ("red", 0xf3, 0x8b, 0xa8),
    ("blue", 0x89, 0xb4, 0xfa),
    ("green", 0xa6, 0xe3, 0xa1),
    ("yellow", 0xf9, 0xe2, 0xaf),
    ("mauve", 0xcb, 0xa6, 0xf7),
];

/// Actions produced by clicking a region.
#[derive(Debug, Clone, Copy)]
pub enum Action {
    SelectColor(usize),
    ThicknessDown,
    ThicknessUp,
    BrowseDir,
    ToggleAutostart,
    ClickShortcut(usize),
    ToggleToolbar(usize),
    SaveClose,
    SwitchTab(usize),
}

/// A clickable rectangle mapped to an action.
struct HitRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    action: Action,
}

pub struct SettingsWindow {
    pub window: Arc<Window>,
    pub surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    pub config: Config,
    pub hovered_button: Option<usize>,
    pub needs_redraw: bool,
    hit_rects: Vec<HitRect>,
    pub cursor_pos: (f32, f32),
    /// Index of the shortcut currently being edited (0-13), or None.
    pub editing_shortcut: Option<usize>,
    /// Active tab index: 0=General, 1=Shortcuts, 2=Toolbar.
    pub active_tab: usize,
}

impl SettingsWindow {
    pub fn new(
        window: Arc<Window>,
        surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
        config: Config,
    ) -> Self {
        Self {
            window,
            surface,
            config,
            hovered_button: None,
            needs_redraw: true,
            hit_rects: Vec::new(),
            cursor_pos: (0.0, 0.0),
            editing_shortcut: None,
            active_tab: 0,
        }
    }

    /// Full render of the settings UI into the softbuffer surface.
    pub fn render(&mut self) {
        let mut pixmap = match Pixmap::new(WIN_W, WIN_H) {
            Some(p) => p,
            None => return,
        };

        // Background
        fill_rect_rgb(&mut pixmap, 0.0, 0.0, WIN_W as f32, WIN_H as f32, BASE);

        self.hit_rects.clear();

        let mut y: f32 = 20.0;
        let left: f32 = 24.0;

        // ── Title ──
        draw_label(&mut pixmap, left, y, "HydroShot Settings", 16.0, TEXT_RGB);
        y += 30.0;

        // ── Tab bar ──
        let tab_names = ["General", "Shortcuts", "Toolbar"];
        let tab_w = (WIN_W as f32 - left * 2.0) / 3.0;
        let tab_h: f32 = 32.0;
        let tab_y = y;

        for (i, name) in tab_names.iter().enumerate() {
            let tx = left + i as f32 * tab_w;

            if i == self.active_tab {
                fill_rect_rgb(&mut pixmap, tx, tab_y, tab_w, tab_h, SURFACE0);
                draw_label(&mut pixmap, tx + 12.0, tab_y + 8.0, name, 14.0, LAVENDER);
            } else {
                if self.is_hovered(tx, tab_y, tab_w, tab_h) {
                    fill_rect_rgb(
                        &mut pixmap,
                        tx,
                        tab_y,
                        tab_w,
                        tab_h,
                        (0x25, 0x25, 0x38),
                    );
                }
                draw_label(&mut pixmap, tx + 12.0, tab_y + 8.0, name, 14.0, SUBTEXT0);
            }

            self.hit_rects.push(HitRect {
                x: tx,
                y: tab_y,
                w: tab_w,
                h: tab_h,
                action: Action::SwitchTab(i),
            });
        }

        // Separator line below tabs
        y = tab_y + tab_h + 2.0;
        fill_rect_rgb(
            &mut pixmap,
            left,
            y,
            WIN_W as f32 - left * 2.0,
            1.0,
            SURFACE0,
        );
        y += 16.0;

        // ── Tab content ──
        match self.active_tab {
            0 => {
                self.render_general_tab(&mut pixmap, left, y);
            }
            1 => {
                self.render_shortcuts_tab(&mut pixmap, left, y);
            }
            _ => {
                self.render_toolbar_tab(&mut pixmap, left, y);
            }
        }

        // ── Save & Close button (always at bottom) ──
        let sc_w: f32 = 140.0;
        let sc_h: f32 = 34.0;
        let sc_x = (WIN_W as f32 - sc_w) / 2.0;
        let sc_y = WIN_H as f32 - sc_h - 20.0;
        draw_button(
            &mut pixmap,
            sc_x,
            sc_y,
            sc_w,
            sc_h,
            "Save & Close",
            self.is_hovered(sc_x, sc_y, sc_w, sc_h),
        );
        self.hit_rects.push(HitRect {
            x: sc_x,
            y: sc_y,
            w: sc_w,
            h: sc_h,
            action: Action::SaveClose,
        });

        // ── Present to softbuffer surface ──
        if let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(WIN_W), NonZeroU32::new(WIN_H)) {
            if let Err(e) = self.surface.resize(nz_w, nz_h) {
                tracing::error!("Settings surface resize failed: {e}");
                return;
            }
        }

        if let Ok(mut buffer) = self.surface.buffer_mut() {
            let src = pixmap.data();
            let pixel_count = (WIN_W * WIN_H) as usize;
            for (i, chunk) in src.chunks_exact(4).take(pixel_count).enumerate() {
                buffer[i] =
                    ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
            }
            let _ = buffer.present();
        }

        self.needs_redraw = false;
    }

    /// Render the General tab content.
    fn render_general_tab(&mut self, pixmap: &mut Pixmap, left: f32, mut y: f32) {
        // ── Default Color ──
        draw_label(pixmap, left, y, "Default Color", 14.0, SUBTEXT0);
        y += 24.0;

        let swatch_size: f32 = 28.0;
        let swatch_gap: f32 = 10.0;
        for (i, &(name, r, g, b)) in COLOR_CHOICES.iter().enumerate() {
            let sx = left + i as f32 * (swatch_size + swatch_gap);
            let sy = y;

            if self.config.general.default_color == name {
                fill_rect_rgb(
                    pixmap,
                    sx - 2.0,
                    sy - 2.0,
                    swatch_size + 4.0,
                    swatch_size + 4.0,
                    LAVENDER,
                );
            }

            if self.is_hovered(sx, sy, swatch_size, swatch_size) {
                fill_rect_rgb(
                    pixmap,
                    sx - 1.0,
                    sy - 1.0,
                    swatch_size + 2.0,
                    swatch_size + 2.0,
                    SURFACE1,
                );
            }

            fill_rect_rgb(pixmap, sx, sy, swatch_size, swatch_size, (r, g, b));

            self.hit_rects.push(HitRect {
                x: sx,
                y: sy,
                w: swatch_size,
                h: swatch_size,
                action: Action::SelectColor(i),
            });
        }
        y += swatch_size + 20.0;

        // ── Default Thickness ──
        draw_label(pixmap, left, y, "Default Thickness", 14.0, SUBTEXT0);
        let thickness_label = format!("{:.1}", self.config.general.default_thickness);
        draw_label(pixmap, left + 160.0, y, &thickness_label, 14.0, TEXT_RGB);
        y += 26.0;

        let btn_w: f32 = 36.0;
        let btn_h: f32 = 28.0;

        let bx = left;
        draw_button(
            pixmap,
            bx,
            y,
            btn_w,
            btn_h,
            "-",
            self.is_hovered(bx, y, btn_w, btn_h),
        );
        self.hit_rects.push(HitRect {
            x: bx,
            y,
            w: btn_w,
            h: btn_h,
            action: Action::ThicknessDown,
        });

        let bx2 = bx + btn_w + 12.0;
        draw_button(
            pixmap,
            bx2,
            y,
            btn_w,
            btn_h,
            "+",
            self.is_hovered(bx2, y, btn_w, btn_h),
        );
        self.hit_rects.push(HitRect {
            x: bx2,
            y,
            w: btn_w,
            h: btn_h,
            action: Action::ThicknessUp,
        });
        y += btn_h + 20.0;

        // ── Save Directory ──
        draw_label(pixmap, left, y, "Save Directory", 14.0, SUBTEXT0);
        y += 22.0;

        let dir_display = if self.config.general.save_directory.is_empty() {
            "(default \u{2014} file picker)"
        } else {
            &self.config.general.save_directory
        };
        let dir_short: String = if dir_display.len() > 34 {
            format!("...{}", &dir_display[dir_display.len() - 31..])
        } else {
            dir_display.to_string()
        };
        draw_label(pixmap, left, y, &dir_short, 12.0, TEXT_RGB);

        let browse_w: f32 = 64.0;
        let browse_x = WIN_W as f32 - left - browse_w;
        let browse_y = y - 2.0;
        let btn_h: f32 = 28.0;
        draw_button(
            pixmap,
            browse_x,
            browse_y,
            browse_w,
            btn_h,
            "Browse",
            self.is_hovered(browse_x, browse_y, browse_w, btn_h),
        );
        self.hit_rects.push(HitRect {
            x: browse_x,
            y: browse_y,
            w: browse_w,
            h: btn_h,
            action: Action::BrowseDir,
        });
        y += btn_h + 20.0;

        // ── Hotkey ──
        let hotkey_label = format!("Hotkey: {}", self.config.hotkey.capture);
        draw_label(pixmap, left, y, &hotkey_label, 14.0, SUBTEXT0);
        y += 30.0;

        // ── Auto-start ──
        let autostart_on = crate::autostart::is_enabled();
        draw_label(pixmap, left, y, "Auto-start:", 14.0, SUBTEXT0);

        let toggle_x = left + 110.0;
        let toggle_w: f32 = 56.0;
        let toggle_h: f32 = 26.0;
        let toggle_bg = if autostart_on { GREEN_RGB } else { SURFACE0 };
        let hovered = self.is_hovered(toggle_x, y - 2.0, toggle_w, toggle_h);
        if hovered {
            fill_rect_rgb(pixmap, toggle_x, y - 2.0, toggle_w, toggle_h, SURFACE1);
        }
        fill_rect_rgb(
            pixmap,
            toggle_x + 1.0,
            y - 1.0,
            toggle_w - 2.0,
            toggle_h - 2.0,
            toggle_bg,
        );

        let toggle_label = if autostart_on { "ON" } else { "OFF" };
        let tl_x = toggle_x + if autostart_on { 18.0 } else { 14.0 };
        draw_label(pixmap, tl_x, y + 2.0, toggle_label, 12.0, TEXT_RGB);

        self.hit_rects.push(HitRect {
            x: toggle_x,
            y: y - 2.0,
            w: toggle_w,
            h: toggle_h,
            action: Action::ToggleAutostart,
        });
    }

    /// Render the Shortcuts tab content.
    fn render_shortcuts_tab(&mut self, pixmap: &mut Pixmap, left: f32, y: f32) {
        let row_h: f32 = 22.0;
        let key_btn_w: f32 = 50.0;
        let key_btn_h: f32 = 20.0;
        let key_btn_x = WIN_W as f32 - left - key_btn_w;

        let entries = self.config.shortcuts.entries();
        for (i, (symbol, label, key_val)) in entries.iter().enumerate() {
            let ry = y + i as f32 * row_h;

            if self.editing_shortcut == Some(i) {
                fill_rect_rgb(
                    pixmap,
                    left - 4.0,
                    ry - 2.0,
                    WIN_W as f32 - (left - 4.0) * 2.0,
                    row_h,
                    SURFACE0,
                );
            }

            draw_label(pixmap, left, ry, symbol, 12.0, LAVENDER);
            draw_label(pixmap, left + 36.0, ry, label, 12.0, TEXT_RGB);

            let display = if self.editing_shortcut == Some(i) {
                "..."
            } else {
                key_val
            };
            let btn_hovered = self.is_hovered(key_btn_x, ry - 1.0, key_btn_w, key_btn_h);
            draw_button(
                pixmap,
                key_btn_x,
                ry - 1.0,
                key_btn_w,
                key_btn_h,
                &display.to_uppercase(),
                btn_hovered,
            );

            self.hit_rects.push(HitRect {
                x: key_btn_x,
                y: ry - 1.0,
                w: key_btn_w,
                h: key_btn_h,
                action: Action::ClickShortcut(i),
            });
        }
    }

    /// Render the Toolbar tab content.
    fn render_toolbar_tab(&mut self, pixmap: &mut Pixmap, left: f32, y: f32) {
        let row_h: f32 = 22.0;
        let toggle_w: f32 = 50.0;
        let toggle_h: f32 = 20.0;
        let toggle_x = WIN_W as f32 - left - toggle_w;

        let toolbar_entries = self.config.toolbar.entries();
        for (i, (symbol, label, enabled)) in toolbar_entries.iter().enumerate() {
            let ry = y + i as f32 * row_h;

            draw_label(pixmap, left, ry, symbol, 12.0, LAVENDER);
            draw_label(pixmap, left + 36.0, ry, label, 12.0, TEXT_RGB);

            let toggle_label = if *enabled { "ON" } else { "OFF" };
            let toggle_bg = if *enabled { GREEN_RGB } else { SURFACE0 };
            let btn_hovered = self.is_hovered(toggle_x, ry - 1.0, toggle_w, toggle_h);

            if btn_hovered {
                fill_rect_rgb(pixmap, toggle_x, ry - 1.0, toggle_w, toggle_h, SURFACE1);
            }
            fill_rect_rgb(
                pixmap,
                toggle_x + 1.0,
                ry,
                toggle_w - 2.0,
                toggle_h - 2.0,
                toggle_bg,
            );

            let tl_x = toggle_x + if *enabled { 14.0 } else { 12.0 };
            draw_label(pixmap, tl_x, ry + 2.0, toggle_label, 12.0, TEXT_RGB);

            self.hit_rects.push(HitRect {
                x: toggle_x,
                y: ry - 1.0,
                w: toggle_w,
                h: toggle_h,
                action: Action::ToggleToolbar(i),
            });
        }
    }

    /// Update cursor position and return whether a redraw is needed.
    pub fn on_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        let old = self.cursor_pos;
        self.cursor_pos = (x, y);
        // Check if hover state changed for any hit rect
        old != self.cursor_pos
    }

    /// Hit-test a click and return the action, if any.
    pub fn on_click(&self, x: f32, y: f32) -> Option<Action> {
        for hr in &self.hit_rects {
            if x >= hr.x && x <= hr.x + hr.w && y >= hr.y && y <= hr.y + hr.h {
                return Some(hr.action);
            }
        }
        None
    }

    /// Handle an action, returning true if the window should close.
    pub fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::SelectColor(idx) => {
                if let Some(&(name, _, _, _)) = COLOR_CHOICES.get(idx) {
                    self.config.general.default_color = name.to_string();
                    self.needs_redraw = true;
                }
                false
            }
            Action::ThicknessDown => {
                self.config.general.default_thickness =
                    (self.config.general.default_thickness - 0.5).max(1.0);
                self.needs_redraw = true;
                false
            }
            Action::ThicknessUp => {
                self.config.general.default_thickness =
                    (self.config.general.default_thickness + 0.5).min(20.0);
                self.needs_redraw = true;
                false
            }
            Action::BrowseDir => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.config.general.save_directory = path.to_string_lossy().to_string();
                    self.needs_redraw = true;
                }
                false
            }
            Action::ToggleAutostart => {
                let new_state = !crate::autostart::is_enabled();
                if let Err(e) = crate::autostart::set_enabled(new_state) {
                    tracing::error!("Auto-start toggle failed: {e}");
                }
                self.needs_redraw = true;
                false
            }
            Action::ClickShortcut(idx) => {
                self.editing_shortcut = Some(idx);
                self.needs_redraw = true;
                false
            }
            Action::ToggleToolbar(idx) => {
                self.config.toolbar.toggle_by_index(idx);
                self.needs_redraw = true;
                false
            }
            Action::SaveClose => {
                if let Err(e) = self.config.save() {
                    tracing::error!("Failed to save config: {e}");
                }
                true
            }
            Action::SwitchTab(idx) => {
                self.active_tab = idx;
                self.editing_shortcut = None;
                self.needs_redraw = true;
                false
            }
        }
    }

    /// Handle a key press while a shortcut is being edited.
    /// Returns `true` if the key was consumed (a shortcut was being edited).
    pub fn on_key_press(&mut self, key: &str) -> bool {
        if let Some(idx) = self.editing_shortcut {
            if !key.is_empty() {
                self.config.shortcuts.set_by_index(idx, key.to_string());
            }
            self.editing_shortcut = None;
            self.needs_redraw = true;
            true
        } else {
            false
        }
    }

    fn is_hovered(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        let (cx, cy) = self.cursor_pos;
        cx >= x && cx <= x + w && cy >= y && cy <= y + h
    }
}

// ── Drawing helpers ──

fn fill_rect_rgb(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, (r, g, b): (u8, u8, u8)) {
    let rect = match Rect::from_xywh(x, y, w, h) {
        Some(r) => r,
        None => return,
    };
    let mut paint = Paint::default();
    paint.set_color(SkiaColor::from_rgba8(r, g, b, 255));
    paint.anti_alias = true;
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

fn draw_label(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    text: &str,
    font_size: f32,
    (r, g, b): (u8, u8, u8),
) {
    let color = Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0);
    let pos = Point::new(x, y);
    render_text_annotation(pixmap, &pos, text, &color, font_size);
}

fn draw_button(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, label: &str, hovered: bool) {
    let bg = if hovered { SURFACE1 } else { SURFACE0 };
    fill_rect_rgb(pixmap, x, y, w, h, bg);

    // Center label approximately
    let char_w = font_size_char_width(12.0);
    let text_w = label.len() as f32 * char_w;
    let tx = x + (w - text_w) / 2.0;
    let ty = y + (h - 12.0) / 2.0;
    draw_label(pixmap, tx, ty, label, 12.0, TEXT_RGB);
}

fn font_size_char_width(size: f32) -> f32 {
    size * 0.55
}
