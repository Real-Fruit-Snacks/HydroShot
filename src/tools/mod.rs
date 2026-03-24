pub mod arrow;
pub mod pencil;
pub mod rectangle;

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
    let _ = (screenshot_pixels, screenshot_width);
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
    }
}
