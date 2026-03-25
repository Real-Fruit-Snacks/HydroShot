use crate::geometry::Color;
use crate::icons::blend_pixmap;
use crate::overlay::selection::Selection;
use crate::overlay::toolbar::Toolbar;
use crate::state::OverlayState;
use crate::tools::{annotation_bounding_box, render_annotation, Annotation, AnnotationTool, ToolKind};
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
    };
    if let Some(ref ann) = in_progress {
        render_annotation(ann, pixmap, ss_pixels, ss_width);
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
        let cursor_x = state.text_input_position.x
            + state.text_input_buffer.len() as f32 * state.text_input_font_size * 0.6;
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

            if let Some(rect) = tiny_skia::Rect::from_xywh(swatch_x, swatch_y, swatch_size, swatch_size) {
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
                let stroke = Stroke { width: 1.5, ..Stroke::default() };
                pixmap.stroke_path(&path, &border, &stroke, Transform::identity(), None);
            }

            // Hex code label below swatch
            let r8 = (color.r * 255.0) as u8;
            let g8 = (color.g * 255.0) as u8;
            let b8 = (color.b * 255.0) as u8;
            let hex = format!("#{:02x}{:02x}{:02x}", r8, g8, b8);

            let font_size = 12.0_f32;
            static FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");
            let font = fontdue::Font::from_bytes(FONT_DATA, fontdue::FontSettings::default())
                .expect("font");
            let text_width: f32 = hex.chars()
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
                pixmap.fill_path(&bg, &bg_paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
            }

            use crate::tools::render_text_annotation;
            let text_pos = crate::geometry::Point::new(pill_x + pad_x, pill_y + pad_y);
            let white = crate::geometry::Color::new(0.804, 0.839, 0.957, 1.0);
            render_text_annotation(pixmap, &text_pos, &hex, &white, font_size);
        }
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
    let handles = [
        (x, y),
        (x + w, y),
        (x, y + h),
        (x + w, y + h),
    ];
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
    let toolbar = Toolbar::position_for(selection, pixmap.height() as f32);
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
    let sep_color = tiny_skia::Color::from_rgba(0.804, 0.839, 0.957, 0.15).unwrap();
    for &after_btn in &[10usize, 15] {
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
            let stroke = Stroke {
                width: 1.0,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // --- Render each button ---
    for i in 0..20usize {
        let (bx, by, bw, bh) = toolbar.button_rect(i);

        let is_active = match i {
            0 => state.active_tool == ToolKind::Select,
            1 => state.active_tool == ToolKind::Arrow,
            2 => state.active_tool == ToolKind::Rectangle,
            3 => state.active_tool == ToolKind::Circle,
            4 => state.active_tool == ToolKind::Line,
            5 => state.active_tool == ToolKind::Pencil,
            6 => state.active_tool == ToolKind::Highlight,
            7 => state.active_tool == ToolKind::Text,
            8 => state.active_tool == ToolKind::Pixelate,
            9 => state.active_tool == ToolKind::StepMarker,
            10 => state.active_tool == ToolKind::Eyedropper,
            11..=15 => {
                let idx = i - 11;
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
            4 => Some("line"),
            5 => Some("pencil"),
            6 => Some("highlight"),
            7 => Some("text"),
            8 => Some("pixelate"),
            9 => Some("step-marker"),
            10 => Some("eyedropper"),
            16 => Some("upload"),
            17 => Some("pin"),
            18 => Some("copy"),
            19 => Some("save"),
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

        if let 11..=15 = i {
            {
                // Color swatch: rounded filled rect with border
                let idx = i - 11;
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
    if let Some(btn_idx) = toolbar.hit_test(mouse) {
        let label = match btn_idx {
            0 => "Select (V)",
            1 => "Arrow (A)",
            2 => "Rectangle (R)",
            3 => "Circle (C)",
            4 => "Line (L)",
            5 => "Pencil (P)",
            6 => "Highlight (H)",
            7 => "Text (T)",
            8 => "Pixelate (B)",
            9 => "Step Marker (N)",
            10 => "Eyedropper (I)",
            11 => "Red (right-click: pick)",
            12 => "Blue (right-click: pick)",
            13 => "Green (right-click: pick)",
            14 => "Yellow (right-click: pick)",
            15 => "Mauve (right-click: pick)",
            16 => "Upload (Imgur)",
            17 => "Pin",
            18 => "Copy (Ctrl+C)",
            19 => "Save (Ctrl+S)",
            _ => "",
        };

        if !label.is_empty() {
            let (btn_x, _btn_y, btn_w, _btn_h) = toolbar.button_rect(btn_idx);

            // Measure text width
            static FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");
            let font = fontdue::Font::from_bytes(
                FONT_DATA, fontdue::FontSettings::default()
            ).expect("font");
            let font_size = 12.0;
            let text_width: f32 = label.chars()
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
                pixmap.fill_path(&bg, &bg_paint, tiny_skia::FillRule::Winding, Transform::identity(), None);

                // Border
                let mut border_paint = Paint::default();
                border_paint.set_color(tiny_skia::Color::from_rgba(0.804, 0.839, 0.957, 0.2).unwrap());
                border_paint.anti_alias = true;
                let border_stroke = Stroke { width: 0.5, ..Stroke::default() };
                pixmap.stroke_path(&bg, &border_paint, &border_stroke, Transform::identity(), None);
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
    use fontdue::{Font, FontSettings};

    static FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");

    let font = Font::from_bytes(FONT_DATA, FontSettings::default()).expect("failed to load font");
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
