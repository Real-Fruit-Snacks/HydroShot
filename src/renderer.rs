use crate::geometry::Color;
use crate::overlay::selection::Selection;
use crate::overlay::toolbar::Toolbar;
use crate::state::OverlayState;
use crate::tools::{render_annotation, AnnotationTool, ToolKind};
use tiny_skia::{Paint, PathBuilder, PremultipliedColorU8, Stroke, Transform};

/// Render the full overlay frame: screenshot background, dim, selection highlight,
/// annotations, and toolbar.
pub fn render_overlay(state: &OverlayState, pixmap: &mut tiny_skia::Pixmap) {
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    let screenshot = &state.screenshot;

    // 1. Copy screenshot pixels as background (premultiplied)
    let pixels = pixmap.pixels_mut();
    let src = &screenshot.pixels;
    let src_w = screenshot.width as usize;
    let src_h = screenshot.height as usize;
    let copy_w = width.min(src_w);
    let copy_h = height.min(src_h);

    for y in 0..copy_h {
        for x in 0..copy_w {
            let si = (y * src_w + x) * 4;
            if si + 3 < src.len() {
                let r = src[si];
                let g = src[si + 1];
                let b = src[si + 2];
                let a = src[si + 3];
                if let Some(c) = PremultipliedColorU8::from_rgba(r, g, b, a) {
                    pixels[y * width + x] = c;
                }
            }
        }
    }

    // 2. Dim the entire screen with semi-transparent black overlay
    if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 0.0, width as f32, height as f32) {
        let mut paint = Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.4).unwrap());
        paint.anti_alias = false;
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }

    // 3. If selection exists, restore brightness within selection bounds
    if let Some(sel) = &state.selection {
        let sx = (sel.x as usize).min(copy_w);
        let sy = (sel.y as usize).min(copy_h);
        let sx2 = ((sel.x + sel.width) as usize).min(copy_w);
        let sy2 = ((sel.y + sel.height) as usize).min(copy_h);

        let pixels = pixmap.pixels_mut();
        for y in sy..sy2 {
            for x in sx..sx2 {
                let si = (y * src_w + x) * 4;
                if si + 3 < src.len() {
                    let r = src[si];
                    let g = src[si + 1];
                    let b = src[si + 2];
                    let a = src[si + 3];
                    if let Some(c) = PremultipliedColorU8::from_rgba(r, g, b, a) {
                        pixels[y * width + x] = c;
                    }
                }
            }
        }

        // 4. Selection border — white 1px stroke
        if let Some(rect) = tiny_skia::Rect::from_xywh(sel.x, sel.y, sel.width, sel.height) {
            let path = PathBuilder::from_rect(rect);
            let mut paint = Paint::default();
            paint.set_color(tiny_skia::Color::WHITE);
            paint.anti_alias = true;
            let stroke = Stroke {
                width: 1.0,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // 5. Finalized annotations
    let ss_pixels = Some(state.screenshot.pixels.as_slice());
    let ss_width = Some(state.screenshot.width);
    for annotation in &state.annotations {
        render_annotation(annotation, pixmap, ss_pixels, ss_width);
    }

    // 6. In-progress annotation preview
    let in_progress = match state.active_tool {
        ToolKind::Arrow => state.arrow_tool.in_progress_annotation(),
        ToolKind::Rectangle => state.rectangle_tool.in_progress_annotation(),
        ToolKind::Pencil => state.pencil_tool.in_progress_annotation(),
        ToolKind::Text => state.text_tool.in_progress_annotation(),
        ToolKind::Pixelate => state.pixelate_tool.in_progress_annotation(),
    };
    if let Some(ref ann) = in_progress {
        render_annotation(ann, pixmap, ss_pixels, ss_width);
    }

    // 7. Toolbar (only if there is a selection)
    if let Some(sel) = &state.selection {
        render_toolbar(state, sel, pixmap);
    }
}

/// Swatch colors matching Color::presets() order.
const SWATCH_COLORS: [(f32, f32, f32); 5] = [
    (1.0, 0.0, 0.0), // red
    (0.0, 0.4, 1.0), // blue
    (0.0, 0.8, 0.0), // green
    (1.0, 0.9, 0.0), // yellow
    (1.0, 1.0, 1.0), // white
];

/// Build a rounded rectangle path with given corner radius.
fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    // Top edge (starting after top-left corner)
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    // Top-right corner
    pb.quad_to(x + w, y, x + w, y + r);
    // Right edge
    pb.line_to(x + w, y + h - r);
    // Bottom-right corner
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    // Bottom edge
    pb.line_to(x + r, y + h);
    // Bottom-left corner
    pb.quad_to(x, y + h, x, y + h - r);
    // Left edge
    pb.line_to(x, y + r);
    // Top-left corner
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

fn render_toolbar(state: &OverlayState, selection: &Selection, pixmap: &mut tiny_skia::Pixmap) {
    let toolbar = Toolbar::position_for(selection, pixmap.height() as f32);
    let presets = Color::presets();

    // --- Toolbar background: rounded rect with subtle border ---
    if let Some(bg_path) = rounded_rect_path(toolbar.x, toolbar.y, toolbar.width, toolbar.height, 8.0) {
        // Shadow (offset dark rect behind)
        if let Some(shadow_path) = rounded_rect_path(toolbar.x + 1.0, toolbar.y + 2.0, toolbar.width, toolbar.height, 8.0) {
            let mut shadow_paint = Paint::default();
            shadow_paint.set_color(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.4).unwrap());
            shadow_paint.anti_alias = true;
            pixmap.fill_path(&shadow_path, &shadow_paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
        }
        // Background fill
        let mut bg_paint = Paint::default();
        bg_paint.set_color(tiny_skia::Color::from_rgba(0.12, 0.12, 0.14, 0.92).unwrap());
        bg_paint.anti_alias = true;
        pixmap.fill_path(&bg_path, &bg_paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
        // Subtle border
        let mut border_paint = Paint::default();
        border_paint.set_color(tiny_skia::Color::from_rgba(1.0, 1.0, 1.0, 0.1).unwrap());
        border_paint.anti_alias = true;
        let border_stroke = Stroke { width: 1.0, ..Stroke::default() };
        pixmap.stroke_path(&bg_path, &border_paint, &border_stroke, Transform::identity(), None);
    }

    // --- Separators between button groups (tools | colors | actions) ---
    let sep_color = tiny_skia::Color::from_rgba(1.0, 1.0, 1.0, 0.15).unwrap();
    for &after_btn in &[1usize, 6] {
        let (bx, _, bw, _) = toolbar.button_rect(after_btn);
        let sep_x = bx + bw + TOOLBAR_PADDING / 2.0;
        let sep_y1 = toolbar.y + 8.0;
        let sep_y2 = toolbar.y + toolbar.height - 8.0;
        let mut pb = PathBuilder::new();
        pb.move_to(sep_x, sep_y1);
        pb.line_to(sep_x, sep_y2);
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(sep_color);
            paint.anti_alias = true;
            let stroke = Stroke { width: 1.0, ..Stroke::default() };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // --- Render each button ---
    for i in 0..9usize {
        let (bx, by, bw, bh) = toolbar.button_rect(i);

        let is_active = match i {
            0 => state.active_tool == ToolKind::Arrow,
            1 => state.active_tool == ToolKind::Rectangle,
            2..=6 => {
                let idx = i - 2;
                idx < presets.len() && state.current_color == presets[idx]
            }
            _ => false,
        };

        // Button background: rounded rect
        let btn_radius = 5.0;
        if let Some(btn_path) = rounded_rect_path(bx, by, bw, bh, btn_radius) {
            if is_active {
                // Active: filled highlight
                let mut fill = Paint::default();
                fill.set_color(tiny_skia::Color::from_rgba(0.25, 0.52, 0.95, 0.35).unwrap());
                fill.anti_alias = true;
                pixmap.fill_path(&btn_path, &fill, tiny_skia::FillRule::Winding, Transform::identity(), None);
                // Active border glow
                let mut glow = Paint::default();
                glow.set_color(tiny_skia::Color::from_rgba(0.4, 0.65, 1.0, 0.7).unwrap());
                glow.anti_alias = true;
                let glow_stroke = Stroke { width: 1.5, ..Stroke::default() };
                pixmap.stroke_path(&btn_path, &glow, &glow_stroke, Transform::identity(), None);
            } else {
                // Inactive: subtle hover-ready background
                let mut fill = Paint::default();
                fill.set_color(tiny_skia::Color::from_rgba(1.0, 1.0, 1.0, 0.06).unwrap());
                fill.anti_alias = true;
                pixmap.fill_path(&btn_path, &fill, tiny_skia::FillRule::Winding, Transform::identity(), None);
            }
        }

        let icon_color = if is_active {
            tiny_skia::Color::WHITE
        } else {
            tiny_skia::Color::from_rgba(0.8, 0.8, 0.8, 1.0).unwrap()
        };

        match i {
            0 => {
                // Arrow icon: diagonal line with proper arrowhead
                let x1 = bx + 8.0;
                let y1 = by + bh - 8.0;
                let x2 = bx + bw - 8.0;
                let y2 = by + 8.0;

                // Shaft
                let mut pb = PathBuilder::new();
                pb.move_to(x1, y1);
                pb.line_to(x2, y2);
                if let Some(path) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color(icon_color);
                    paint.anti_alias = true;
                    let stroke = Stroke { width: 2.0, ..Stroke::default() };
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }

                // Arrowhead triangle at (x2, y2)
                let dx = x2 - x1;
                let dy = y2 - y1;
                let len = (dx * dx + dy * dy).sqrt();
                let ux = dx / len;
                let uy = dy / len;
                let arrow_len = 8.0;
                let arrow_half_w = 4.0;
                let base_x = x2 - ux * arrow_len;
                let base_y = y2 - uy * arrow_len;
                let perp_x = -uy;
                let perp_y = ux;

                let mut pb = PathBuilder::new();
                pb.move_to(x2, y2);
                pb.line_to(base_x + perp_x * arrow_half_w, base_y + perp_y * arrow_half_w);
                pb.line_to(base_x - perp_x * arrow_half_w, base_y - perp_y * arrow_half_w);
                pb.close();
                if let Some(path) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color(icon_color);
                    paint.anti_alias = true;
                    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
                }
            }
            1 => {
                // Rectangle icon: rounded outlined rect
                let inset = 7.0;
                if let Some(path) = rounded_rect_path(bx + inset, by + inset, bw - inset * 2.0, bh - inset * 2.0, 2.0) {
                    let mut paint = Paint::default();
                    paint.set_color(icon_color);
                    paint.anti_alias = true;
                    let stroke = Stroke { width: 2.0, ..Stroke::default() };
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
            }
            2..=6 => {
                // Color swatch: rounded filled rect with border
                let idx = i - 2;
                if idx < SWATCH_COLORS.len() {
                    let (r, g, b) = SWATCH_COLORS[idx];
                    let inset = 6.0;
                    if let Some(swatch_path) = rounded_rect_path(bx + inset, by + inset, bw - inset * 2.0, bh - inset * 2.0, 3.0) {
                        // Fill with color
                        let mut paint = Paint::default();
                        paint.set_color(tiny_skia::Color::from_rgba(r, g, b, 1.0).unwrap());
                        paint.anti_alias = true;
                        pixmap.fill_path(&swatch_path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
                        // Border (darker for light colors, lighter for dark)
                        let border_alpha = if r + g + b > 2.0 { 0.3 } else { 0.5 };
                        let mut border = Paint::default();
                        border.set_color(tiny_skia::Color::from_rgba(1.0, 1.0, 1.0, border_alpha).unwrap());
                        border.anti_alias = true;
                        let stroke = Stroke { width: 1.0, ..Stroke::default() };
                        pixmap.stroke_path(&swatch_path, &border, &stroke, Transform::identity(), None);
                    }
                }
            }
            7 => {
                // Copy icon: two overlapping rounded rects
                let s = 10.0;
                let off = 4.0;
                let cx = bx + bw / 2.0;
                let cy = by + bh / 2.0;
                // Back rect (top-left)
                if let Some(path) = rounded_rect_path(cx - s / 2.0 - off / 2.0, cy - s / 2.0 - off / 2.0, s, s, 2.0) {
                    let mut paint = Paint::default();
                    paint.set_color(icon_color);
                    paint.anti_alias = true;
                    let stroke = Stroke { width: 1.5, ..Stroke::default() };
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
                // Front rect (bottom-right, slightly filled)
                if let Some(path) = rounded_rect_path(cx - s / 2.0 + off / 2.0, cy - s / 2.0 + off / 2.0, s, s, 2.0) {
                    // Slight fill to distinguish from back
                    let mut fill = Paint::default();
                    fill.set_color(tiny_skia::Color::from_rgba(0.12, 0.12, 0.14, 0.92).unwrap());
                    fill.anti_alias = true;
                    pixmap.fill_path(&path, &fill, tiny_skia::FillRule::Winding, Transform::identity(), None);
                    let mut paint = Paint::default();
                    paint.set_color(icon_color);
                    paint.anti_alias = true;
                    let stroke = Stroke { width: 1.5, ..Stroke::default() };
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
            }
            8 => {
                // Save/download icon: arrow pointing down into a tray
                let cx = bx + bw / 2.0;
                let top = by + 8.0;
                let arrow_tip = by + bh - 12.0;
                let tray_y = by + bh - 8.0;

                // Vertical stem
                let mut pb = PathBuilder::new();
                pb.move_to(cx, top);
                pb.line_to(cx, arrow_tip);
                if let Some(path) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color(icon_color);
                    paint.anti_alias = true;
                    let stroke = Stroke { width: 2.0, ..Stroke::default() };
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }

                // Arrowhead (filled triangle)
                let mut pb = PathBuilder::new();
                pb.move_to(cx, arrow_tip + 2.0);
                pb.line_to(cx - 5.0, arrow_tip - 4.0);
                pb.line_to(cx + 5.0, arrow_tip - 4.0);
                pb.close();
                if let Some(path) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color(icon_color);
                    paint.anti_alias = true;
                    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
                }

                // Tray: U-shape
                let mut pb = PathBuilder::new();
                let tray_left = bx + 8.0;
                let tray_right = bx + bw - 8.0;
                pb.move_to(tray_left, tray_y - 4.0);
                pb.line_to(tray_left, tray_y);
                pb.line_to(tray_right, tray_y);
                pb.line_to(tray_right, tray_y - 4.0);
                if let Some(path) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color(icon_color);
                    paint.anti_alias = true;
                    let stroke = Stroke { width: 2.0, ..Stroke::default() };
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
            }
            _ => {}
        }
    }
}

use crate::overlay::toolbar::TOOLBAR_PADDING;
