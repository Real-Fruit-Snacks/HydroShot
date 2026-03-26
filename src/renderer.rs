use crate::geometry::Color;
use crate::icons::blend_pixmap;
use crate::overlay::selection::Selection;
use crate::overlay::toolbar::Toolbar;
use crate::state::OverlayState;
use crate::tools::{
    annotation_bounding_box, render_annotation, Annotation, AnnotationTool, ToolKind,
};
use tiny_skia::{Paint, PathBuilder, Stroke, Transform};

/// Render the full overlay frame: screenshot background, dim, selection highlight,
/// annotations, and toolbar.
pub fn render_overlay(state: &mut OverlayState, pixmap: &mut tiny_skia::Pixmap) {
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;

    // 1+2. Bulk copy pre-computed dimmed screenshot (fast memcpy, done once at capture time)
    let dimmed_data = state.dimmed_pixmap.data();
    let pixmap_data = pixmap.data_mut();
    let copy_len = dimmed_data.len().min(pixmap_data.len());
    pixmap_data[..copy_len].copy_from_slice(&dimmed_data[..copy_len]);

    // 2b. Crosshair guide lines through cursor for precise alignment
    let show_crosshair = state.selection.is_none() || state.is_selecting;
    if show_crosshair {
        let mx = state.last_mouse_pos.x;
        let my = state.last_mouse_pos.y;
        let pw = pixmap.width() as f32;
        let ph = pixmap.height() as f32;

        let mut paint = Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba(0.706, 0.745, 0.996, 0.5).unwrap()); // Lavender #b4befe 50%
        paint.anti_alias = true;
        let stroke = Stroke {
            width: 1.0,
            ..Stroke::default()
        };

        // Vertical line
        let mut pb = PathBuilder::new();
        pb.move_to(mx, 0.0);
        pb.line_to(mx, ph);
        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        // Horizontal line
        let mut pb = PathBuilder::new();
        pb.move_to(0.0, my);
        pb.line_to(pw, my);
        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // 3. If selection exists, restore brightness by copying from cached screenshot pixmap
    if let Some(sel) = &state.selection {
        let src_w = state.screenshot_pixmap.width() as usize;
        let sx = (sel.x as usize).min(width);
        let sy = (sel.y as usize).min(height);
        let sx2 = ((sel.x + sel.width) as usize).min(width).min(src_w);
        let sy2 = ((sel.y + sel.height) as usize).min(height);
        let row_bytes = (sx2.saturating_sub(sx)) * 4; // 4 bytes per pixel

        if row_bytes > 0 {
            let screenshot_data = state.screenshot_pixmap.data();
            let pixmap_data = pixmap.data_mut();
            for y in sy..sy2 {
                let src_offset = (y * src_w + sx) * 4;
                let dst_offset = (y * width + sx) * 4;
                if src_offset + row_bytes <= screenshot_data.len()
                    && dst_offset + row_bytes <= pixmap_data.len()
                {
                    pixmap_data[dst_offset..dst_offset + row_bytes]
                        .copy_from_slice(&screenshot_data[src_offset..src_offset + row_bytes]);
                }
            }
        }

        // 4. Selection border — white 1px stroke
        if let Some(rect) = tiny_skia::Rect::from_xywh(sel.x, sel.y, sel.width, sel.height) {
            let path = PathBuilder::from_rect(rect);
            let mut paint = Paint::default();
            paint.set_color(tiny_skia::Color::from_rgba(0.706, 0.745, 0.996, 0.9).unwrap()); // Lavender #b4befe
            paint.anti_alias = true;
            let stroke = Stroke {
                width: 1.5,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        // 4b. Selection size label
        render_size_label(pixmap, sel, width as f32, height as f32);
    }

    // 5. Finalized annotations
    let ss_pixels = Some(state.screenshot.pixels.as_slice());
    let ss_width = Some(state.screenshot.width);
    for annotation in &state.annotations {
        render_annotation(annotation, pixmap, ss_pixels, ss_width);
    }

    // 6. In-progress annotation preview
    let in_progress = match state.active_tool {
        ToolKind::Select => None,
        ToolKind::Arrow => state.arrow_tool.in_progress_annotation(),
        ToolKind::Rectangle => state.rectangle_tool.in_progress_annotation(),
        ToolKind::Circle => state.circle_tool.in_progress_annotation(),
        ToolKind::Line => state.line_tool.in_progress_annotation(),
        ToolKind::Pencil => state.pencil_tool.in_progress_annotation(),
        ToolKind::Highlight => state.highlight_tool.in_progress_annotation(),
        ToolKind::Text => state.text_tool.in_progress_annotation(),
        ToolKind::Pixelate => state.pixelate_tool.in_progress_annotation(),
        ToolKind::StepMarker => state.step_marker_tool.in_progress_annotation(),
        ToolKind::Eyedropper => None,
        ToolKind::RoundedRect => state.rounded_rect_tool.in_progress_annotation(),
        ToolKind::Spotlight => state.spotlight_tool.in_progress_annotation(),
        ToolKind::Measurement => state.measurement_tool.in_progress_annotation(),
    };
    if let Some(ref ann) = in_progress {
        render_annotation(ann, pixmap, ss_pixels, ss_width);
    }

    // 6a. Spotlight effect — dim everything outside spotlight rectangles
    {
        let mut spotlights: Vec<(crate::geometry::Point, crate::geometry::Size)> = state
            .annotations
            .iter()
            .filter_map(|a| match a {
                Annotation::Spotlight { top_left, size } => Some((*top_left, *size)),
                _ => None,
            })
            .collect();
        // Include in-progress spotlight
        if let Some(Annotation::Spotlight { top_left, size }) = in_progress.as_ref() {
            spotlights.push((*top_left, *size));
        }
        if !spotlights.is_empty() {
            // Dim the entire pixmap with 50% black overlay
            let pw = pixmap.width() as f32;
            let ph = pixmap.height() as f32;
            if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, pw, ph) {
                let mut dim = Paint::default();
                dim.set_color(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.5).unwrap());
                dim.anti_alias = false;
                pixmap.fill_rect(r, &dim, Transform::identity(), None);
            }

            // Restore the bright areas (spotlight cutouts) from screenshot_pixmap
            let src_w = state.screenshot_pixmap.width();
            let screenshot_data = state.screenshot_pixmap.data();
            for (tl, sz) in &spotlights {
                let sx = tl.x.max(0.0) as u32;
                let sy = tl.y.max(0.0) as u32;
                let sw = sz.width as u32;
                let sh = sz.height as u32;
                let pm_w = pixmap.width();
                let pm_h = pixmap.height();
                let pixmap_data = pixmap.data_mut();
                for y in sy..(sy + sh).min(pm_h) {
                    let row_start = ((y * src_w + sx) * 4) as usize;
                    let copy_w = sw
                        .min(src_w.saturating_sub(sx))
                        .min(pm_w.saturating_sub(sx));
                    let row_end = row_start + (copy_w * 4) as usize;
                    let dst_start = ((y * pm_w + sx) * 4) as usize;
                    let dst_end = dst_start + (copy_w * 4) as usize;
                    if row_end <= screenshot_data.len() && dst_end <= pixmap_data.len() {
                        pixmap_data[dst_start..dst_end]
                            .copy_from_slice(&screenshot_data[row_start..row_end]);
                    }
                }
            }

            // Re-draw non-spotlight annotations so they remain visible over dimmed areas
            for ann in &state.annotations {
                if !matches!(ann, Annotation::Spotlight { .. }) {
                    render_annotation(ann, pixmap, ss_pixels, ss_width);
                }
            }
            if let Some(ref ann) = in_progress {
                if !matches!(ann, Annotation::Spotlight { .. }) {
                    render_annotation(ann, pixmap, ss_pixels, ss_width);
                }
            }
        }
    }

    // 6b. Selection highlight around selected annotation
    if let Some(idx) = state.selected_index {
        if let Some(ann) = state.annotations.get(idx) {
            if let Some((bx, by, bw, bh)) = annotation_bounding_box(ann) {
                render_selection_highlight(pixmap, bx - 4.0, by - 4.0, bw + 8.0, bh + 8.0);
            }
        }
    }

    // 7. Text input preview (in-progress text being typed)
    if state.text_input_active && !state.text_input_buffer.is_empty() {
        let preview = Annotation::Text {
            position: state.text_input_position,
            text: state.text_input_buffer.clone(),
            color: state.current_color,
            font_size: state.text_input_font_size,
        };
        render_annotation(&preview, pixmap, ss_pixels, ss_width);
    }
    // Text cursor (vertical white line after text)
    if state.text_input_active {
        let font = &*crate::font::FONT;
        let cursor_x = state.text_input_position.x
            + state
                .text_input_buffer
                .chars()
                .map(|ch| {
                    font.rasterize(ch, state.text_input_font_size)
                        .0
                        .advance_width
                })
                .sum::<f32>();
        let cursor_y = state.text_input_position.y;
        let cursor_h = state.text_input_font_size;
        let mut pb = PathBuilder::new();
        pb.move_to(cursor_x, cursor_y);
        pb.line_to(cursor_x, cursor_y + cursor_h);
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(tiny_skia::Color::WHITE);
            paint.anti_alias = true;
            let stroke = Stroke {
                width: 1.5,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // 8. Toolbar (only if there is a selection)
    if let Some(sel) = state.selection {
        render_toolbar(state, &sel, pixmap);
    }

    // 9. Eyedropper preview (color swatch + hex near cursor)
    if state.active_tool == ToolKind::Eyedropper {
        if let Some(color) = state.eyedropper_preview {
            let mx = state.last_mouse_pos.x;
            let my = state.last_mouse_pos.y;

            // Small filled square showing the color
            let swatch_size = 24.0_f32;
            let swatch_x = mx + 16.0;
            let swatch_y = my + 16.0;

            if let Some(rect) =
                tiny_skia::Rect::from_xywh(swatch_x, swatch_y, swatch_size, swatch_size)
            {
                let mut paint = Paint::default();
                if let Some(c) = tiny_skia::Color::from_rgba(color.r, color.g, color.b, 1.0) {
                    paint.set_color(c);
                }
                paint.anti_alias = false;
                pixmap.fill_rect(rect, &paint, Transform::identity(), None);

                // White border around swatch
                let path = PathBuilder::from_rect(rect);
                let mut border = Paint::default();
                border.set_color(tiny_skia::Color::WHITE);
                border.anti_alias = true;
                let stroke = Stroke {
                    width: 1.5,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &border, &stroke, Transform::identity(), None);
            }

            // Hex code label below swatch
            let r8 = (color.r * 255.0) as u8;
            let g8 = (color.g * 255.0) as u8;
            let b8 = (color.b * 255.0) as u8;
            let hex = format!("#{:02x}{:02x}{:02x}", r8, g8, b8);

            let font_size = 12.0_f32;
            let font = &*crate::font::FONT;
            let text_width: f32 = hex
                .chars()
                .map(|ch| font.rasterize(ch, font_size).0.advance_width)
                .sum();

            let pad_x = 4.0_f32;
            let pad_y = 2.0_f32;
            let pill_w = text_width + pad_x * 2.0;
            let pill_h = font_size + pad_y * 2.0;
            let pill_x = swatch_x;
            let pill_y = swatch_y + swatch_size + 2.0;

            if let Some(bg) = rounded_rect_path(pill_x, pill_y, pill_w, pill_h, 3.0) {
                let mut bg_paint = Paint::default();
                bg_paint.set_color(tiny_skia::Color::from_rgba(0.067, 0.067, 0.094, 0.9).unwrap());
                bg_paint.anti_alias = true;
                pixmap.fill_path(
                    &bg,
                    &bg_paint,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }

            use crate::tools::render_text_annotation;
            let text_pos = crate::geometry::Point::new(pill_x + pad_x, pill_y + pad_y);
            let white = crate::geometry::Color::new(0.804, 0.839, 0.957, 1.0);
            render_text_annotation(pixmap, &text_pos, &hex, &white, font_size);
        }
    }

    // 10. In-overlay toast notification
    state.clear_expired_toast();
    if let Some(ref msg) = state.toast_message {
        let font_size = 14.0_f32;
        let font = &*crate::font::FONT;
        let text_width: f32 = msg
            .chars()
            .map(|ch| font.rasterize(ch, font_size).0.advance_width)
            .sum();

        let pad_x = 16.0_f32;
        let pad_y = 10.0_f32;
        let toast_w = text_width + pad_x * 2.0;
        let toast_h = font_size + pad_y * 2.0;
        let toast_x = (pixmap.width() as f32 - toast_w) / 2.0;
        let toast_y = pixmap.height() as f32 - 80.0;

        // Background pill
        if let Some(bg) = rounded_rect_path(toast_x, toast_y, toast_w, toast_h, 8.0) {
            let mut bg_paint = Paint::default();
            bg_paint.set_color(tiny_skia::Color::from_rgba(0.067, 0.067, 0.094, 0.92).unwrap());
            bg_paint.anti_alias = true;
            pixmap.fill_path(
                &bg,
                &bg_paint,
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                None,
            );
            // Border
            let mut border = Paint::default();
            border.set_color(tiny_skia::Color::from_rgba(0.651, 0.890, 0.631, 0.6).unwrap()); // green tint
            border.anti_alias = true;
            let stroke = Stroke {
                width: 1.0,
                ..Stroke::default()
            };
            pixmap.stroke_path(&bg, &border, &stroke, Transform::identity(), None);
        }

        // Text
        use crate::tools::render_text_annotation;
        let text_pos = crate::geometry::Point::new(toast_x + pad_x, toast_y + pad_y);
        let text_color = crate::geometry::Color::new(0.804, 0.839, 0.957, 1.0);
        render_text_annotation(pixmap, &text_pos, msg, &text_color, font_size);
    }
}

/// Catppuccin Mocha swatch colors matching Color::presets() order.
const SWATCH_COLORS: [(f32, f32, f32); 5] = [
    (0.953, 0.545, 0.659), // Red (#f38ba8)
    (0.537, 0.706, 0.980), // Blue (#89b4fa)
    (0.651, 0.890, 0.631), // Green (#a6e3a1)
    (0.976, 0.886, 0.686), // Yellow (#f9e2af)
    (0.796, 0.651, 0.969), // Mauve (#cba6f7)
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

/// Draw a dashed selection highlight rectangle around a selected annotation,
/// plus resize handles at the four corners.
fn render_selection_highlight(pixmap: &mut tiny_skia::Pixmap, x: f32, y: f32, w: f32, h: f32) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let lavender_color = tiny_skia::Color::from_rgba(0.706, 0.745, 0.996, 0.8).unwrap();
    if let Some(rect) = tiny_skia::Rect::from_xywh(x, y, w, h) {
        let path = PathBuilder::from_rect(rect);
        let mut paint = Paint::default();
        paint.set_color(lavender_color);
        paint.anti_alias = true;
        let stroke = Stroke {
            width: 1.5,
            dash: tiny_skia::StrokeDash::new(vec![4.0, 4.0], 0.0),
            ..Stroke::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    // Draw resize handles at the four corners
    let handles = [(x, y), (x + w, y), (x, y + h), (x + w, y + h)];
    for (hx, hy) in &handles {
        let hs = 4.0; // half-size
        if let Some(rect) = tiny_skia::Rect::from_xywh(hx - hs, hy - hs, hs * 2.0, hs * 2.0) {
            // White filled square
            let mut fill = Paint::default();
            fill.set_color(tiny_skia::Color::WHITE);
            fill.anti_alias = false;
            pixmap.fill_rect(rect, &fill, Transform::identity(), None);

            // Lavender border to match the dashed selection border
            let path = PathBuilder::from_rect(rect);
            let mut border = Paint::default();
            border.set_color(lavender_color);
            border.anti_alias = true;
            let stroke = Stroke {
                width: 1.0,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &border, &stroke, Transform::identity(), None);
        }
    }
}

fn render_toolbar(state: &mut OverlayState, selection: &Selection, pixmap: &mut tiny_skia::Pixmap) {
    let visible = &state.visible_buttons;
    let visible_count = visible.len();
    let toolbar = Toolbar::position_for_dynamic(selection, pixmap.height() as f32, visible_count);
    let presets = Color::presets();

    // --- Toolbar background: rounded rect with subtle border ---
    if let Some(bg_path) =
        rounded_rect_path(toolbar.x, toolbar.y, toolbar.width, toolbar.height, 8.0)
    {
        // Shadow (offset dark rect behind)
        if let Some(shadow_path) = rounded_rect_path(
            toolbar.x + 1.0,
            toolbar.y + 2.0,
            toolbar.width,
            toolbar.height,
            8.0,
        ) {
            let mut shadow_paint = Paint::default();
            shadow_paint.set_color(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.4).unwrap());
            shadow_paint.anti_alias = true;
            pixmap.fill_path(
                &shadow_path,
                &shadow_paint,
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        // Background fill
        let mut bg_paint = Paint::default();
        bg_paint.set_color(tiny_skia::Color::from_rgba(0.067, 0.067, 0.094, 0.95).unwrap());
        bg_paint.anti_alias = true;
        pixmap.fill_path(
            &bg_path,
            &bg_paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
        // Subtle border
        let mut border_paint = Paint::default();
        border_paint.set_color(tiny_skia::Color::from_rgba(0.804, 0.839, 0.957, 0.12).unwrap());
        border_paint.anti_alias = true;
        let border_stroke = Stroke {
            width: 1.0,
            ..Stroke::default()
        };
        pixmap.stroke_path(
            &bg_path,
            &border_paint,
            &border_stroke,
            Transform::identity(),
            None,
        );
    }

    // --- Separators between button groups (tools | colors | actions) ---
    // Find the visible positions where group boundaries occur (after last tool, after last color)
    let sep_color = tiny_skia::Color::from_rgba(0.804, 0.839, 0.957, 0.15).unwrap();
    // Separator after the last tool button (orig 0-13) before colors (orig 14)
    // and after the last color button (orig 18) before actions (orig 19)
    let sep_after_orig = [13usize, 18];
    for &boundary in &sep_after_orig {
        // Find the visible index of this boundary button
        if let Some(vis_idx) = visible.iter().position(|&orig| orig == boundary) {
            let (bx, _, bw, _) = toolbar.button_rect(vis_idx);
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
                let stroke = Stroke {
                    width: 1.0,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        } else if boundary == 13 {
            // If button 13 is hidden, find the last visible tool button (0-13)
            // and draw separator after it
            if let Some(vis_idx) = visible.iter().rposition(|&orig| orig <= 13) {
                // Only draw if there's actually a color/action after it
                if vis_idx + 1 < visible_count {
                    let (bx, _, bw, _) = toolbar.button_rect(vis_idx);
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
                        let stroke = Stroke {
                            width: 1.0,
                            ..Stroke::default()
                        };
                        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                    }
                }
            }
        }
    }

    // --- Render each button ---
    for (vis_idx, &i) in visible.iter().enumerate() {
        let (bx, by, bw, bh) = toolbar.button_rect(vis_idx);

        let is_active = match i {
            0 => state.active_tool == ToolKind::Select,
            1 => state.active_tool == ToolKind::Arrow,
            2 => state.active_tool == ToolKind::Rectangle,
            3 => state.active_tool == ToolKind::Circle,
            4 => state.active_tool == ToolKind::RoundedRect,
            5 => state.active_tool == ToolKind::Line,
            6 => state.active_tool == ToolKind::Pencil,
            7 => state.active_tool == ToolKind::Highlight,
            8 => state.active_tool == ToolKind::Spotlight,
            9 => state.active_tool == ToolKind::Text,
            10 => state.active_tool == ToolKind::Pixelate,
            11 => state.active_tool == ToolKind::StepMarker,
            12 => state.active_tool == ToolKind::Eyedropper,
            13 => state.active_tool == ToolKind::Measurement,
            14..=18 => {
                let idx = i - 14;
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
                fill.set_color(tiny_skia::Color::from_rgba(0.537, 0.706, 0.980, 0.3).unwrap());
                fill.anti_alias = true;
                pixmap.fill_path(
                    &btn_path,
                    &fill,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );
                // Active border glow
                let mut glow = Paint::default();
                glow.set_color(tiny_skia::Color::from_rgba(0.706, 0.745, 0.996, 0.7).unwrap());
                glow.anti_alias = true;
                let glow_stroke = Stroke {
                    width: 1.5,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&btn_path, &glow, &glow_stroke, Transform::identity(), None);
            } else {
                // Inactive: subtle hover-ready background
                let mut fill = Paint::default();
                fill.set_color(tiny_skia::Color::from_rgba(0.804, 0.839, 0.957, 0.08).unwrap());
                fill.anti_alias = true;
                pixmap.fill_path(
                    &btn_path,
                    &fill,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }

        // --- SVG icon rendering for tool buttons and action buttons ---
        let icon_name = match i {
            0 => Some("select"),
            1 => Some("arrow"),
            2 => Some("rectangle"),
            3 => Some("circle"),
            4 => Some("rounded-rect"),
            5 => Some("line"),
            6 => Some("pencil"),
            7 => Some("highlight"),
            8 => Some("spotlight"),
            9 => Some("text"),
            10 => Some("pixelate"),
            11 => Some("step-marker"),
            12 => Some("eyedropper"),
            13 => Some("measurement"),
            19 => Some("ocr"),
            20 => Some("upload"),
            21 => Some("pin"),
            22 => Some("copy"),
            23 => Some("save"),
            _ => None,
        };

        if let Some(name) = icon_name {
            let icon_size = (bw - 12.0).max(12.0) as u32;
            let color_hex = if is_active { "#cdd6f4" } else { "#a6adc8" };
            if let Some(icon_pixmap) = state.icon_cache.get_or_render(name, icon_size, color_hex) {
                let icon_x = (bx + (bw - icon_size as f32) / 2.0) as i32;
                let icon_y = (by + (bh - icon_size as f32) / 2.0) as i32;
                blend_pixmap(pixmap, icon_pixmap, icon_x, icon_y);
            }
        }

        if let 14..=18 = i {
            {
                // Color swatch: rounded filled rect with border
                let idx = i - 14;
                if idx < SWATCH_COLORS.len() {
                    let (r, g, b) = SWATCH_COLORS[idx];
                    let inset = 6.0;
                    if let Some(swatch_path) = rounded_rect_path(
                        bx + inset,
                        by + inset,
                        bw - inset * 2.0,
                        bh - inset * 2.0,
                        3.0,
                    ) {
                        // Fill with color
                        let mut paint = Paint::default();
                        paint.set_color(tiny_skia::Color::from_rgba(r, g, b, 1.0).unwrap());
                        paint.anti_alias = true;
                        pixmap.fill_path(
                            &swatch_path,
                            &paint,
                            tiny_skia::FillRule::Winding,
                            Transform::identity(),
                            None,
                        );
                        // Border (darker for light colors, lighter for dark)
                        let border_alpha = if r + g + b > 2.0 { 0.3 } else { 0.5 };
                        let mut border = Paint::default();
                        border.set_color(
                            tiny_skia::Color::from_rgba(1.0, 1.0, 1.0, border_alpha).unwrap(),
                        );
                        border.anti_alias = true;
                        let stroke = Stroke {
                            width: 1.0,
                            ..Stroke::default()
                        };
                        pixmap.stroke_path(
                            &swatch_path,
                            &border,
                            &stroke,
                            Transform::identity(),
                            None,
                        );
                    }
                }
            }
        }
    }

    // --- Tooltip for hovered button ---
    let mouse = crate::geometry::Point::new(state.last_mouse_pos.x, state.last_mouse_pos.y);
    if let Some(vis_idx) = toolbar.hit_test_dynamic(mouse, visible_count) {
        let btn_idx = visible[vis_idx];
        let label = match btn_idx {
            0 => "Select (V)",
            1 => "Arrow (A)",
            2 => "Rectangle (R)",
            3 => "Circle (C)",
            4 => "Rounded Rect (O)",
            5 => "Line (L)",
            6 => "Pencil (P)",
            7 => "Highlight (H)",
            8 => "Spotlight (F)",
            9 => "Text (T)",
            10 => "Pixelate (B)",
            11 => "Step Marker (N)",
            12 => "Eyedropper (I)",
            13 => "Measurement (M)",
            14 => "Red #f38ba8 (right-click: pick)",
            15 => "Blue #89b4fa (right-click: pick)",
            16 => "Green #a6e3a1 (right-click: pick)",
            17 => "Yellow #f9e2af (right-click: pick)",
            18 => "Mauve #cba6f7 (right-click: pick)",
            19 => "OCR (Extract Text)",
            20 => "Upload (Imgur)",
            21 => "Pin",
            22 => "Copy (Ctrl+C)",
            23 => "Save (Ctrl+S)",
            _ => "",
        };

        if !label.is_empty() {
            let (btn_x, _btn_y, btn_w, _btn_h) = toolbar.button_rect(vis_idx);

            // Measure text width
            let font = &*crate::font::FONT;
            let font_size = 12.0;
            let text_width: f32 = label
                .chars()
                .map(|ch| font.rasterize(ch, font_size).0.advance_width)
                .sum();

            let pad_x = 6.0;
            let pad_y = 4.0;
            let tip_w = text_width + pad_x * 2.0;
            let tip_h = font_size + pad_y * 2.0;

            // Center tooltip above the button
            let tip_x = (btn_x + btn_w / 2.0 - tip_w / 2.0)
                .max(toolbar.x)
                .min(toolbar.x + toolbar.width - tip_w);
            let tip_y = toolbar.y - tip_h - 6.0;

            // Background pill
            if let Some(bg) = rounded_rect_path(tip_x, tip_y, tip_w, tip_h, 4.0) {
                let mut bg_paint = Paint::default();
                bg_paint.set_color(tiny_skia::Color::from_rgba(0.067, 0.067, 0.094, 0.95).unwrap());
                bg_paint.anti_alias = true;
                pixmap.fill_path(
                    &bg,
                    &bg_paint,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );

                // Border
                let mut border_paint = Paint::default();
                border_paint
                    .set_color(tiny_skia::Color::from_rgba(0.804, 0.839, 0.957, 0.2).unwrap());
                border_paint.anti_alias = true;
                let border_stroke = Stroke {
                    width: 0.5,
                    ..Stroke::default()
                };
                pixmap.stroke_path(
                    &bg,
                    &border_paint,
                    &border_stroke,
                    Transform::identity(),
                    None,
                );
            }

            // Text
            use crate::tools::render_text_annotation;
            let text_pos = crate::geometry::Point::new(tip_x + pad_x, tip_y + pad_y);
            let white = crate::geometry::Color::new(0.804, 0.839, 0.957, 1.0);
            render_text_annotation(pixmap, &text_pos, label, &white, font_size);
        }
    }
}

use crate::overlay::toolbar::TOOLBAR_PADDING;

/// Render a small label showing the selection dimensions (e.g. "1920 × 1080")
/// near the bottom-right corner of the selection, flipping position if near screen edges.
fn render_size_label(
    pixmap: &mut tiny_skia::Pixmap,
    sel: &Selection,
    screen_w: f32,
    screen_h: f32,
) {
    let font = &*crate::font::FONT;
    let font_size = 14.0;

    let label = format!("{} \u{00D7} {}", sel.width as u32, sel.height as u32);

    // Measure text width
    let mut text_width: f32 = 0.0;
    for ch in label.chars() {
        let (metrics, _) = font.rasterize(ch, font_size);
        text_width += metrics.advance_width;
    }

    let pad_x: f32 = 8.0;
    let pad_y: f32 = 4.0;
    let pill_w = text_width + pad_x * 2.0;
    let pill_h = font_size + pad_y * 2.0;
    let margin = 4.0;

    // Default: bottom-right, offset outside selection
    let mut pill_x = sel.x + sel.width - pill_w + margin;
    let mut pill_y = sel.y + sel.height + margin;

    // Flip left if near right edge
    if pill_x + pill_w > screen_w {
        pill_x = sel.x - margin;
    }
    // Flip above if near bottom edge
    if pill_y + pill_h > screen_h {
        pill_y = sel.y - pill_h - margin;
    }

    // Clamp to screen
    if pill_x < 0.0 {
        pill_x = 0.0;
    }
    if pill_y < 0.0 {
        pill_y = 0.0;
    }

    // Draw background pill (dark semi-transparent rounded rect)
    if let Some(bg_path) = rounded_rect_path(pill_x, pill_y, pill_w, pill_h, 4.0) {
        let mut paint = Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.7).unwrap());
        paint.anti_alias = true;
        pixmap.fill_path(
            &bg_path,
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    // Render text on top
    let line_metrics = font.horizontal_line_metrics(font_size);
    let ascent = line_metrics.map(|m| m.ascent).unwrap_or(font_size * 0.8);
    let text_x = pill_x + pad_x;
    let baseline_y = pill_y + pad_y + ascent;

    let pw = pixmap.width() as i32;
    let ph = pixmap.height() as i32;
    let pixels = pixmap.data_mut();

    let mut cursor_x = text_x;
    for ch in label.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        if metrics.width == 0 || metrics.height == 0 {
            cursor_x += metrics.advance_width;
            continue;
        }

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
                if coverage < 1.0 / 255.0 {
                    continue;
                }

                let idx = ((px_y as u32 * pw as u32 + px_x as u32) * 4) as usize;

                // White text, alpha-blended
                let src_val = (255.0 * coverage) as u8;
                let src_a = coverage;
                let inv_a = 1.0 - src_a;

                let dst_r = pixels[idx];
                let dst_g = pixels[idx + 1];
                let dst_b = pixels[idx + 2];
                let dst_a = pixels[idx + 3];

                pixels[idx] = (src_val as f32 + dst_r as f32 * inv_a).min(255.0) as u8;
                pixels[idx + 1] = (src_val as f32 + dst_g as f32 * inv_a).min(255.0) as u8;
                pixels[idx + 2] = (src_val as f32 + dst_b as f32 * inv_a).min(255.0) as u8;
                pixels[idx + 3] = (src_a * 255.0 + dst_a as f32 * inv_a).min(255.0) as u8;
            }
        }

        cursor_x += metrics.advance_width;
    }
}
