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
            let mut stroke = Stroke::default();
            stroke.width = 1.0;
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // 5. Finalized annotations
    for annotation in &state.annotations {
        render_annotation(annotation, pixmap);
    }

    // 6. In-progress annotation preview
    let in_progress = match state.active_tool {
        ToolKind::Arrow => state.arrow_tool.in_progress_annotation(),
        ToolKind::Rectangle => state.rectangle_tool.in_progress_annotation(),
    };
    if let Some(ref ann) = in_progress {
        render_annotation(ann, pixmap);
    }

    // 7. Toolbar (only if there is a selection)
    if let Some(sel) = &state.selection {
        render_toolbar(state, sel, pixmap);
    }
}

/// Swatch colors matching Color::presets() order.
const SWATCH_COLORS: [(f32, f32, f32); 5] = [
    (1.0, 0.0, 0.0),   // red
    (0.0, 0.4, 1.0),   // blue
    (0.0, 0.8, 0.0),   // green
    (1.0, 0.9, 0.0),   // yellow
    (1.0, 1.0, 1.0),   // white
];

fn render_toolbar(state: &OverlayState, selection: &Selection, pixmap: &mut tiny_skia::Pixmap) {
    let toolbar = Toolbar::position_for(selection, pixmap.height() as f32);

    // Toolbar background
    if let Some(rect) = tiny_skia::Rect::from_xywh(toolbar.x, toolbar.y, toolbar.width, toolbar.height) {
        let mut paint = Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba(0.15, 0.15, 0.15, 0.85).unwrap());
        paint.anti_alias = false;
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }

    let presets = Color::presets();

    for i in 0..9usize {
        let (bx, by, bw, bh) = toolbar.button_rect(i);

        // Determine if this button is "active"
        let is_active = match i {
            0 => state.active_tool == ToolKind::Arrow,
            1 => state.active_tool == ToolKind::Rectangle,
            2..=6 => {
                let idx = i - 2;
                idx < presets.len() && state.current_color == presets[idx]
            }
            _ => false,
        };

        // Button background
        let bg_color = if is_active {
            tiny_skia::Color::from_rgba(0.3, 0.6, 1.0, 0.8).unwrap()
        } else {
            tiny_skia::Color::from_rgba(0.3, 0.3, 0.3, 0.8).unwrap()
        };

        if let Some(rect) = tiny_skia::Rect::from_xywh(bx, by, bw, bh) {
            let mut paint = Paint::default();
            paint.set_color(bg_color);
            paint.anti_alias = false;
            pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }

        // Button-specific content
        match i {
            0 => {
                // Arrow icon: diagonal line
                let mut pb = PathBuilder::new();
                pb.move_to(bx + 6.0, by + bh - 6.0);
                pb.line_to(bx + bw - 6.0, by + 6.0);
                if let Some(path) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color(tiny_skia::Color::WHITE);
                    paint.anti_alias = true;
                    let mut stroke = Stroke::default();
                    stroke.width = 2.0;
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
            }
            1 => {
                // Rectangle icon: small outlined rect
                let inset = 6.0;
                if let Some(rect) = tiny_skia::Rect::from_xywh(
                    bx + inset,
                    by + inset,
                    bw - inset * 2.0,
                    bh - inset * 2.0,
                ) {
                    let path = PathBuilder::from_rect(rect);
                    let mut paint = Paint::default();
                    paint.set_color(tiny_skia::Color::WHITE);
                    paint.anti_alias = true;
                    let mut stroke = Stroke::default();
                    stroke.width = 2.0;
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
            }
            2..=6 => {
                // Color swatch: filled rect with 4px inset
                let idx = i - 2;
                if idx < SWATCH_COLORS.len() {
                    let (r, g, b) = SWATCH_COLORS[idx];
                    let inset = 4.0;
                    if let Some(rect) = tiny_skia::Rect::from_xywh(
                        bx + inset,
                        by + inset,
                        bw - inset * 2.0,
                        bh - inset * 2.0,
                    ) {
                        let mut paint = Paint::default();
                        paint.set_color(tiny_skia::Color::from_rgba(r, g, b, 1.0).unwrap());
                        paint.anti_alias = false;
                        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                }
            }
            7 => {
                // Copy button: simple icon — two overlapping rects
                let inset = 7.0;
                let s = bw - inset * 2.0;
                let half = s * 0.6;
                // Back rect
                if let Some(rect) = tiny_skia::Rect::from_xywh(bx + inset, by + inset, half, half) {
                    let path = PathBuilder::from_rect(rect);
                    let mut paint = Paint::default();
                    paint.set_color(tiny_skia::Color::WHITE);
                    paint.anti_alias = true;
                    let mut stroke = Stroke::default();
                    stroke.width = 1.5;
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
                // Front rect (offset)
                let off = s * 0.4;
                if let Some(rect) = tiny_skia::Rect::from_xywh(bx + inset + off, by + inset + off, half, half) {
                    let path = PathBuilder::from_rect(rect);
                    let mut paint = Paint::default();
                    paint.set_color(tiny_skia::Color::WHITE);
                    paint.anti_alias = true;
                    let mut stroke = Stroke::default();
                    stroke.width = 1.5;
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
            }
            8 => {
                // Save button: downward arrow icon
                let cx = bx + bw / 2.0;
                let mut pb = PathBuilder::new();
                pb.move_to(cx, by + 7.0);
                pb.line_to(cx, by + bh - 10.0);
                // Arrowhead
                pb.move_to(cx - 5.0, by + bh - 14.0);
                pb.line_to(cx, by + bh - 9.0);
                pb.line_to(cx + 5.0, by + bh - 14.0);
                if let Some(path) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color(tiny_skia::Color::WHITE);
                    paint.anti_alias = true;
                    let mut stroke = Stroke::default();
                    stroke.width = 2.0;
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
                // Base line
                let mut pb2 = PathBuilder::new();
                pb2.move_to(bx + 7.0, by + bh - 7.0);
                pb2.line_to(bx + bw - 7.0, by + bh - 7.0);
                if let Some(path) = pb2.finish() {
                    let mut paint = Paint::default();
                    paint.set_color(tiny_skia::Color::WHITE);
                    paint.anti_alias = true;
                    let mut stroke = Stroke::default();
                    stroke.width = 2.0;
                    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
            }
            _ => {}
        }
    }
}
