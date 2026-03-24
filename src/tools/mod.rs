pub mod arrow;
pub mod pencil;
pub mod pixelate;
pub mod rectangle;
pub mod text;

use crate::geometry::{Color, Point, Size};
use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Debug, Clone, PartialEq)]
pub enum Annotation {
    Arrow {
        start: Point,
        end: Point,
        color: Color,
        thickness: f32,
    },
    Rectangle {
        top_left: Point,
        size: Size,
        color: Color,
        thickness: f32,
    },
    Pencil {
        points: Vec<Point>,
        color: Color,
        thickness: f32,
    },
    Text {
        position: Point,
        text: String,
        color: Color,
        font_size: f32,
    },
    Pixelate {
        top_left: Point,
        size: Size,
        block_size: u8,
    },
}

pub trait AnnotationTool {
    fn on_mouse_down(&mut self, pos: Point);
    fn on_mouse_move(&mut self, pos: Point);
    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation>;
    fn is_drawing(&self) -> bool;
    fn in_progress_annotation(&self) -> Option<Annotation>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolKind {
    Arrow,
    Rectangle,
    Pencil,
    Text,
    Pixelate,
}

/// Compute the three vertices of an arrowhead triangle.
///
/// The tip is at `end`. The two base vertices are placed at `4 * thickness`
/// distance from the tip along the shaft, spread at +/- 30 degrees.
pub fn arrowhead_points(start: Point, end: Point, thickness: f32) -> Vec<Point> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return vec![end, end, end];
    }

    let ux = dx / len;
    let uy = dy / len;

    let side_length = 4.0 * thickness;
    let half_angle: f32 = std::f32::consts::PI / 6.0; // 30 degrees
    let back_dist = side_length * half_angle.cos();
    let perp_offset = side_length * half_angle.sin();

    // Perpendicular direction (rotate unit vector 90 degrees)
    let px = -uy;
    let py = ux;

    let base_x = end.x - ux * back_dist;
    let base_y = end.y - uy * back_dist;

    let p1 = Point::new(base_x + px * perp_offset, base_y + py * perp_offset);
    let p2 = Point::new(base_x - px * perp_offset, base_y - py * perp_offset);

    vec![end, p1, p2]
}

/// Render any Annotation variant onto a tiny_skia::Pixmap.
///
/// This is the single rendering path used for both interactive preview and export.
pub fn render_annotation(
    annotation: &Annotation,
    pixmap: &mut Pixmap,
    screenshot_pixels: Option<&[u8]>,
    screenshot_width: Option<u32>,
) {
    match annotation {
        Annotation::Rectangle {
            top_left,
            size,
            color,
            thickness,
        } => {
            let mut paint = Paint::default();
            let c: tiny_skia::Color = (*color).into();
            paint.set_color(c);
            paint.anti_alias = true;

            let rect = tiny_skia::Rect::from_xywh(top_left.x, top_left.y, size.width, size.height);
            if let Some(rect) = rect {
                let path = PathBuilder::from_rect(rect);
                let stroke = Stroke {
                    width: *thickness,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
        Annotation::Arrow {
            start,
            end,
            color,
            thickness,
        } => {
            let mut paint = Paint::default();
            let c: tiny_skia::Color = (*color).into();
            paint.set_color(c);
            paint.anti_alias = true;

            // Draw shaft
            let mut pb = PathBuilder::new();
            pb.move_to(start.x, start.y);
            pb.line_to(end.x, end.y);
            if let Some(path) = pb.finish() {
                let stroke = Stroke {
                    width: *thickness,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }

            // Draw arrowhead (filled triangle)
            let points = arrowhead_points(*start, *end, *thickness);
            if points.len() == 3 {
                let mut pb = PathBuilder::new();
                pb.move_to(points[0].x, points[0].y);
                pb.line_to(points[1].x, points[1].y);
                pb.line_to(points[2].x, points[2].y);
                pb.close();
                if let Some(path) = pb.finish() {
                    pixmap.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }
        }
        Annotation::Pencil {
            points,
            color,
            thickness,
        } => {
            if points.len() < 2 {
                return;
            }
            let mut pb = PathBuilder::new();
            pb.move_to(points[0].x, points[0].y);
            for p in &points[1..] {
                pb.line_to(p.x, p.y);
            }
            if let Some(path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color((*color).into());
                paint.anti_alias = true;
                let stroke = Stroke {
                    width: *thickness,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
        Annotation::Text {
            position,
            text,
            color,
            font_size,
        } => {
            render_text_annotation(pixmap, position, text, color, *font_size);
        }
        Annotation::Pixelate {
            top_left,
            size,
            block_size,
        } => {
            let src_pixels = match screenshot_pixels {
                Some(p) => p,
                None => return,
            };
            let src_width = match screenshot_width {
                Some(w) => w as usize,
                None => return,
            };

            let bs = (*block_size).max(1) as usize;
            let rx = top_left.x.max(0.0) as usize;
            let ry = top_left.y.max(0.0) as usize;
            let rw = size.width as usize;
            let rh = size.height as usize;
            let pm_w = pixmap.width() as usize;
            let pm_h = pixmap.height() as usize;

            let mut by = 0;
            while by < rh {
                let bh = bs.min(rh - by);
                let mut bx = 0;
                while bx < rw {
                    let bw = bs.min(rw - bx);
                    let px_x = rx + bx;
                    let px_y = ry + by;

                    // Average source pixels in this block
                    let mut sum_r: u64 = 0;
                    let mut sum_g: u64 = 0;
                    let mut sum_b: u64 = 0;
                    let mut count: u64 = 0;

                    for row in 0..bh {
                        for col in 0..bw {
                            let sx = px_x + col;
                            let sy = px_y + row;
                            if sx < src_width {
                                let si = (sy * src_width + sx) * 4;
                                if si + 3 < src_pixels.len() {
                                    sum_r += src_pixels[si] as u64;
                                    sum_g += src_pixels[si + 1] as u64;
                                    sum_b += src_pixels[si + 2] as u64;
                                    count += 1;
                                }
                            }
                        }
                    }

                    if count > 0 {
                        let avg_r = (sum_r / count) as f32 / 255.0;
                        let avg_g = (sum_g / count) as f32 / 255.0;
                        let avg_b = (sum_b / count) as f32 / 255.0;

                        // Clamp fill rect to pixmap bounds
                        let fill_x = px_x.min(pm_w);
                        let fill_y = px_y.min(pm_h);
                        let fill_w = bw.min(pm_w.saturating_sub(fill_x));
                        let fill_h = bh.min(pm_h.saturating_sub(fill_y));

                        if fill_w > 0 && fill_h > 0 {
                            if let Some(rect) = tiny_skia::Rect::from_xywh(
                                fill_x as f32,
                                fill_y as f32,
                                fill_w as f32,
                                fill_h as f32,
                            ) {
                                let mut paint = Paint::default();
                                if let Some(c) =
                                    tiny_skia::Color::from_rgba(avg_r, avg_g, avg_b, 1.0)
                                {
                                    paint.set_color(c);
                                } else {
                                    paint.set_color(tiny_skia::Color::BLACK);
                                }
                                paint.anti_alias = false;
                                pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                            }
                        }
                    }

                    bx += bs;
                }
                by += bs;
            }
        }
    }
}

/// Rasterize a single-line text annotation onto a pixmap using fontdue.
fn render_text_annotation(
    pixmap: &mut Pixmap,
    position: &Point,
    text: &str,
    color: &Color,
    font_size: f32,
) {
    use fontdue::{Font, FontSettings};

    static FONT_DATA: &[u8] = include_bytes!("../../assets/font.ttf");

    let font = Font::from_bytes(FONT_DATA, FontSettings::default()).expect("failed to load font");

    let pw = pixmap.width() as i32;
    let ph = pixmap.height() as i32;

    let sr = (color.r * 255.0) as u8;
    let sg = (color.g * 255.0) as u8;
    let sb = (color.b * 255.0) as u8;
    let sa = color.a;

    let mut cursor_x = position.x;
    let baseline_y = position.y;

    let pixels = pixmap.data_mut();

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        if metrics.width == 0 || metrics.height == 0 {
            cursor_x += metrics.advance_width;
            continue;
        }

        // fontdue metrics: ymin is distance from baseline to bottom of glyph (can be negative)
        // The glyph origin y = baseline_y - metrics.height as offset from top
        let glyph_x = cursor_x as i32 + metrics.xmin;
        let glyph_y = baseline_y as i32 - metrics.height as i32 - metrics.ymin;

        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let px_x = glyph_x + col as i32;
                let px_y = glyph_y + row as i32;

                if px_x < 0 || px_x >= pw || px_y < 0 || px_y >= ph {
                    continue;
                }

                let coverage = bitmap[row * metrics.width + col] as f32 / 255.0;
                let alpha = coverage * sa;
                if alpha < 1.0 / 255.0 {
                    continue;
                }

                let idx = ((px_y as u32 * pw as u32 + px_x as u32) * 4) as usize;

                // Alpha-blend with premultiplied destination.
                // Source is straight; destination is premultiplied.
                let src_r = (sr as f32 * alpha) as u8;
                let src_g = (sg as f32 * alpha) as u8;
                let src_b = (sb as f32 * alpha) as u8;
                let src_a = (alpha * 255.0) as u8;

                let dst_r = pixels[idx];
                let dst_g = pixels[idx + 1];
                let dst_b = pixels[idx + 2];
                let dst_a = pixels[idx + 3];

                let inv_alpha = 1.0 - alpha;
                let out_r = src_r as f32 + dst_r as f32 * inv_alpha;
                let out_g = src_g as f32 + dst_g as f32 * inv_alpha;
                let out_b = src_b as f32 + dst_b as f32 * inv_alpha;
                let out_a = src_a as f32 + dst_a as f32 * inv_alpha;

                pixels[idx] = out_r.min(255.0) as u8;
                pixels[idx + 1] = out_g.min(255.0) as u8;
                pixels[idx + 2] = out_b.min(255.0) as u8;
                pixels[idx + 3] = out_a.min(255.0) as u8;
            }
        }

        cursor_x += metrics.advance_width;
    }
}
