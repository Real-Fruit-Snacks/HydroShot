use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use tiny_skia::{Color as SkiaColor, Paint, Pixmap, Rect, Transform};
use winit::window::Window;

use crate::geometry::{Color, Point};
use crate::tools::render_text_annotation;

/// Width / height of the history window in physical pixels.
pub const WIN_W: u32 = 600;
pub const WIN_H: u32 = 400;

/// Thumbnail grid layout
const COLS: usize = 4;
const THUMB_W: u32 = 120;
const THUMB_H: u32 = 80;
const PADDING: f32 = 16.0;
const GAP: f32 = 12.0;
const HEADER_H: f32 = 44.0;

// Catppuccin Mocha palette
const BASE: (u8, u8, u8) = (0x1e, 0x1e, 0x2e);
const SURFACE0: (u8, u8, u8) = (0x31, 0x32, 0x44);
const LAVENDER: (u8, u8, u8) = (0xb4, 0xbe, 0xfe);
const TEXT_RGB: (u8, u8, u8) = (0xcd, 0xd6, 0xf4);
const SUBTEXT0: (u8, u8, u8) = (0xa6, 0xad, 0xc8);

/// A loaded thumbnail: path, RGBA pixels, width, height.
struct Thumbnail {
    path: PathBuf,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

pub struct HistoryWindow {
    pub window: Arc<Window>,
    pub surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    thumbnails: Vec<Thumbnail>,
    pub hovered: Option<usize>,
    pub needs_redraw: bool,
    pub cursor_pos: (f32, f32),
}

impl HistoryWindow {
    pub fn new(
        window: Arc<Window>,
        surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    ) -> Self {
        // Load thumbnails from history
        let entries = crate::history::list_history();
        let thumbnails: Vec<Thumbnail> = entries
            .into_iter()
            .filter_map(|path| {
                let img = image::open(&path).ok()?;
                let thumb = img.thumbnail(THUMB_W, THUMB_H);
                let rgba = thumb.to_rgba8();
                let (w, h) = rgba.dimensions();
                Some(Thumbnail {
                    path,
                    pixels: rgba.into_raw(),
                    width: w,
                    height: h,
                })
            })
            .collect();

        Self {
            window,
            surface,
            thumbnails,
            hovered: None,
            needs_redraw: true,
            cursor_pos: (0.0, 0.0),
        }
    }

    /// Returns (x, y, w, h) of the thumbnail at the given index.
    fn thumb_rect(&self, idx: usize) -> (f32, f32, f32, f32) {
        let col = idx % COLS;
        let row = idx / COLS;
        let cell_w = THUMB_W as f32 + GAP;
        let cell_h = THUMB_H as f32 + GAP;
        let x = PADDING + col as f32 * cell_w;
        let y = HEADER_H + PADDING + row as f32 * cell_h;
        if let Some(thumb) = self.thumbnails.get(idx) {
            (x, y, thumb.width as f32, thumb.height as f32)
        } else {
            (x, y, THUMB_W as f32, THUMB_H as f32)
        }
    }

    /// Full render of the history UI into the softbuffer surface.
    pub fn render(&mut self) {
        let mut pixmap = match Pixmap::new(WIN_W, WIN_H) {
            Some(p) => p,
            None => return,
        };

        // Background
        fill_rect_rgb(&mut pixmap, 0.0, 0.0, WIN_W as f32, WIN_H as f32, BASE);

        // Title
        draw_label(&mut pixmap, PADDING, 14.0, "Recent Captures", 16.0, TEXT_RGB);

        // Separator
        fill_rect_rgb(
            &mut pixmap,
            PADDING,
            HEADER_H - 4.0,
            WIN_W as f32 - PADDING * 2.0,
            1.0,
            SURFACE0,
        );

        if self.thumbnails.is_empty() {
            draw_label(
                &mut pixmap,
                PADDING,
                HEADER_H + PADDING + 20.0,
                "No captures yet.",
                14.0,
                SUBTEXT0,
            );
        } else {
            for (i, thumb) in self.thumbnails.iter().enumerate() {
                let (x, y, tw, th) = self.thumb_rect(i);

                // Border (highlight on hover)
                let border_color = if self.hovered == Some(i) {
                    LAVENDER
                } else {
                    SURFACE0
                };
                fill_rect_rgb(
                    &mut pixmap,
                    x - 2.0,
                    y - 2.0,
                    tw + 4.0,
                    th + 4.0,
                    border_color,
                );

                // Render thumbnail pixels directly onto the pixmap
                blit_rgba(
                    &mut pixmap,
                    x as u32,
                    y as u32,
                    &thumb.pixels,
                    thumb.width,
                    thumb.height,
                );
            }
        }

        // Present to softbuffer surface
        if let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(WIN_W), NonZeroU32::new(WIN_H)) {
            if let Err(e) = self.surface.resize(nz_w, nz_h) {
                tracing::error!("History surface resize failed: {e}");
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

    /// Update cursor position. Returns true if hover state changed.
    pub fn on_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        self.cursor_pos = (x, y);
        let old_hovered = self.hovered;
        self.hovered = None;
        for i in 0..self.thumbnails.len() {
            let (tx, ty, tw, th) = self.thumb_rect(i);
            if x >= tx && x <= tx + tw && y >= ty && y <= ty + th {
                self.hovered = Some(i);
                break;
            }
        }
        old_hovered != self.hovered
    }

    /// Handle a click. Returns true if an image was copied to clipboard.
    pub fn on_click(&self, x: f32, y: f32) -> bool {
        for i in 0..self.thumbnails.len() {
            let (tx, ty, tw, th) = self.thumb_rect(i);
            if x >= tx && x <= tx + tw && y >= ty && y <= ty + th {
                // Load the full-resolution image and copy to clipboard
                if let Some(thumb) = self.thumbnails.get(i) {
                    if let Ok(img) = image::open(&thumb.path) {
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        let pixels = rgba.into_raw();
                        match crate::export::copy_to_clipboard(&pixels, w, h) {
                            Ok(()) => {
                                tracing::info!("Copied history image to clipboard");
                                let _ = notify_rust::Notification::new()
                                    .summary("HydroShot")
                                    .body("Copied to clipboard")
                                    .timeout(2000)
                                    .show();
                                return true;
                            }
                            Err(e) => {
                                tracing::error!("Failed to copy history image: {e}");
                            }
                        }
                    }
                }
            }
        }
        false
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

/// Blit RGBA pixels onto a pixmap at the given position.
fn blit_rgba(pixmap: &mut Pixmap, dst_x: u32, dst_y: u32, pixels: &[u8], w: u32, h: u32) {
    let pm_w = pixmap.width();
    let pm_h = pixmap.height();
    let data = pixmap.data_mut();
    for y in 0..h {
        for x in 0..w {
            let px = dst_x + x;
            let py = dst_y + y;
            if px >= pm_w || py >= pm_h {
                continue;
            }
            let src_i = ((y * w + x) * 4) as usize;
            let dst_i = ((py * pm_w + px) * 4) as usize;
            if src_i + 3 < pixels.len() && dst_i + 3 < data.len() {
                // Premultiply alpha for tiny-skia
                let a = pixels[src_i + 3] as u32;
                data[dst_i] = ((pixels[src_i] as u32 * a) / 255) as u8;
                data[dst_i + 1] = ((pixels[src_i + 1] as u32 * a) / 255) as u8;
                data[dst_i + 2] = ((pixels[src_i + 2] as u32 * a) / 255) as u8;
                data[dst_i + 3] = pixels[src_i + 3];
            }
        }
    }
}
