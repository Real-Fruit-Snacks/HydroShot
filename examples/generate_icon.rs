//! Generate the HydroShot icon (Concept D: Drop + Capture Frame) using tiny-skia.
//! Run: cargo run --example generate_icon
//!
//! Dark rounded badge with cyan selection brackets framing a gradient water droplet.

fn main() {
    generate_icon(256, "icon_256.png");
    generate_icon(256, "assets/icon.png");
    generate_icon(48, "icon_48.png");
    generate_icon(16, "icon_16.png");
    println!("Icons generated.");
}

fn generate_icon(size: u32, path: &str) {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).unwrap();
    let s = size as f32 / 128.0; // design is 128x128

    // ─── 1. Rounded square background ───
    let corner_r = 26.0 * s;
    if let Some(bg_path) = rounded_rect(0.0, 0.0, size as f32, size as f32, corner_r) {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba(0.047, 0.071, 0.133, 1.0).unwrap()); // #0c1222
        paint.anti_alias = true;
        pixmap.fill_path(
            &bg_path,
            &paint,
            tiny_skia::FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );
    }

    // ─── 2. Subtle inner glow / vignette ───
    // A slightly lighter rectangle inset to give depth
    let inset = 3.0 * s;
    if let Some(inner_path) = rounded_rect(
        inset,
        inset,
        size as f32 - inset * 2.0,
        size as f32 - inset * 2.0,
        corner_r - inset,
    ) {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba(0.06, 0.09, 0.16, 1.0).unwrap()); // slightly lighter
        paint.anti_alias = true;
        pixmap.fill_path(
            &inner_path,
            &paint,
            tiny_skia::FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );
    }

    // ─── 3. Selection brackets (corners) ───
    let bracket_inset = 18.0 * s;
    let bracket_len = 22.0 * s;
    // Scale bracket width for small sizes
    let bracket_w = if size <= 32 { 5.0 * s } else { 3.5 * s };

    let bracket_color = tiny_skia::Color::from_rgba(0.22, 0.74, 0.97, 1.0).unwrap(); // #38bdf8
    let mut bp = tiny_skia::Paint::default();
    bp.set_color(bracket_color);
    bp.anti_alias = true;
    let bs = tiny_skia::Stroke {
        width: bracket_w,
        line_cap: tiny_skia::LineCap::Round,
        line_join: tiny_skia::LineJoin::Round,
        ..tiny_skia::Stroke::default()
    };

    let x1 = bracket_inset;
    let y1 = bracket_inset;
    let x2 = size as f32 - bracket_inset;
    let y2 = size as f32 - bracket_inset;

    // Top-left
    draw_bracket(&mut pixmap, x1, y1, bracket_len, true, true, &bp, &bs);
    // Top-right
    draw_bracket(&mut pixmap, x2, y1, bracket_len, false, true, &bp, &bs);
    // Bottom-left
    draw_bracket(&mut pixmap, x1, y2, bracket_len, true, false, &bp, &bs);
    // Bottom-right
    draw_bracket(&mut pixmap, x2, y2, bracket_len, false, false, &bp, &bs);

    // ─── 4. Water droplet ───
    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;

    // Droplet dimensions
    let drop_top = cy - 32.0 * s;
    let drop_bottom = cy + 20.0 * s;
    let drop_radius = 20.0 * s;
    let drop_wide_y = cy + 2.0 * s; // where the drop is widest

    // Main droplet path
    let drop_path = build_droplet(cx, drop_top, drop_bottom, drop_radius, drop_wide_y, s);

    if let Some(ref dp) = drop_path {
        // Base fill: deep blue
        let mut base = tiny_skia::Paint::default();
        base.set_color(tiny_skia::Color::from_rgba(0.01, 0.41, 0.64, 1.0).unwrap()); // #0369a1
        base.anti_alias = true;
        pixmap.fill_path(
            dp,
            &base,
            tiny_skia::FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );

        // Upper gradient overlay: lighter cyan on top portion
        let upper = build_droplet_upper(cx, drop_top, drop_radius, drop_wide_y, cy - 2.0 * s, s);
        if let Some(ref up) = upper {
            let mut light = tiny_skia::Paint::default();
            light.set_color(tiny_skia::Color::from_rgba(0.13, 0.83, 0.93, 0.8).unwrap()); // #22d3ee
            light.anti_alias = true;
            pixmap.fill_path(
                up,
                &light,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }

        // Tip highlight: brightest at the very top
        let tip = build_droplet_upper(
            cx,
            drop_top,
            drop_radius * 0.5,
            drop_wide_y,
            cy - 14.0 * s,
            s,
        );
        if let Some(ref tp) = tip {
            let mut bright = tiny_skia::Paint::default();
            bright.set_color(tiny_skia::Color::from_rgba(0.4, 0.92, 0.98, 0.5).unwrap()); // bright cyan
            bright.anti_alias = true;
            pixmap.fill_path(
                tp,
                &bright,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }

        // Glass shine: elliptical highlight
        if size >= 48 {
            let shine_cx = cx - 6.0 * s;
            let shine_cy = cy - 10.0 * s;
            let shine_rx = 4.5 * s;
            let shine_ry = 8.0 * s;
            if let Some(shine) = ellipse_path(shine_cx, shine_cy, shine_rx, shine_ry) {
                let mut sp = tiny_skia::Paint::default();
                sp.set_color(tiny_skia::Color::from_rgba(1.0, 1.0, 1.0, 0.28).unwrap());
                sp.anti_alias = true;
                pixmap.fill_path(
                    &shine,
                    &sp,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
        }

        // Small bubble highlight (only at larger sizes)
        if size >= 64 {
            let bub_cx = cx - 9.0 * s;
            let bub_cy = cy - 18.0 * s;
            let bub_r = 2.5 * s;
            if let Some(bub) = ellipse_path(bub_cx, bub_cy, bub_r, bub_r) {
                let mut bp2 = tiny_skia::Paint::default();
                bp2.set_color(tiny_skia::Color::from_rgba(1.0, 1.0, 1.0, 0.2).unwrap());
                bp2.anti_alias = true;
                pixmap.fill_path(
                    &bub,
                    &bp2,
                    tiny_skia::FillRule::Winding,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
        }

        // Droplet outline: subtle cyan border
        let mut outline = tiny_skia::Paint::default();
        outline.set_color(tiny_skia::Color::from_rgba(0.22, 0.74, 0.97, 0.6).unwrap()); // #38bdf8
        outline.anti_alias = true;
        let outline_w = if size <= 32 { 2.5 * s } else { 1.8 * s };
        let os = tiny_skia::Stroke {
            width: outline_w,
            ..tiny_skia::Stroke::default()
        };
        pixmap.stroke_path(dp, &outline, &os, tiny_skia::Transform::identity(), None);
    }

    // ─── 5. Outer badge border (very subtle) ───
    if size >= 48 {
        if let Some(badge_path) =
            rounded_rect(0.5, 0.5, size as f32 - 1.0, size as f32 - 1.0, corner_r)
        {
            let mut border = tiny_skia::Paint::default();
            border.set_color(tiny_skia::Color::from_rgba(0.22, 0.74, 0.97, 0.15).unwrap());
            border.anti_alias = true;
            let bord_stroke = tiny_skia::Stroke {
                width: 1.0 * s,
                ..tiny_skia::Stroke::default()
            };
            pixmap.stroke_path(
                &badge_path,
                &border,
                &bord_stroke,
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }

    pixmap.save_png(path).unwrap();
}

// ─── Helper: rounded rectangle path ───
fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

// ─── Helper: L-shaped bracket ───
fn draw_bracket(
    pixmap: &mut tiny_skia::Pixmap,
    x: f32,
    y: f32,
    len: f32,
    left: bool,
    top: bool,
    paint: &tiny_skia::Paint,
    stroke: &tiny_skia::Stroke,
) {
    let dx = if left { len } else { -len };
    let dy = if top { len } else { -len };
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(x + dx, y);
    pb.line_to(x, y);
    pb.line_to(x, y + dy);
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, paint, stroke, tiny_skia::Transform::identity(), None);
    }
}

// ─── Helper: full droplet path ───
fn build_droplet(
    cx: f32,
    top: f32,
    bottom: f32,
    radius: f32,
    wide_y: f32,
    s: f32,
) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(cx, top);
    // Right side: tip down to widest
    pb.cubic_to(
        cx + 7.0 * s,
        top + 16.0 * s,
        cx + radius,
        wide_y - 10.0 * s,
        cx + radius,
        wide_y,
    );
    // Right side: widest to bottom
    let k = 0.5522848;
    let ry = bottom - wide_y;
    pb.cubic_to(
        cx + radius,
        wide_y + ry * k,
        cx + radius * k,
        bottom,
        cx,
        bottom,
    );
    // Left side: bottom to widest
    pb.cubic_to(
        cx - radius * k,
        bottom,
        cx - radius,
        wide_y + ry * k,
        cx - radius,
        wide_y,
    );
    // Left side: widest up to tip
    pb.cubic_to(
        cx - radius,
        wide_y - 10.0 * s,
        cx - 7.0 * s,
        top + 16.0 * s,
        cx,
        top,
    );
    pb.close();
    pb.finish()
}

// ─── Helper: upper portion of droplet (for gradient simulation) ───
fn build_droplet_upper(
    cx: f32,
    top: f32,
    radius: f32,
    wide_y: f32,
    cut_y: f32,
    s: f32,
) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(cx, top);
    pb.cubic_to(
        cx + 7.0 * s,
        top + 16.0 * s,
        cx + radius,
        wide_y - 10.0 * s,
        cx + radius,
        cut_y,
    );
    pb.line_to(cx - radius, cut_y);
    pb.cubic_to(
        cx - radius,
        wide_y - 10.0 * s,
        cx - 7.0 * s,
        top + 16.0 * s,
        cx,
        top,
    );
    pb.close();
    pb.finish()
}

// ─── Helper: ellipse path ───
fn ellipse_path(cx: f32, cy: f32, rx: f32, ry: f32) -> Option<tiny_skia::Path> {
    let k: f32 = 0.5522848;
    let kx = rx * k;
    let ky = ry * k;
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(cx + rx, cy);
    pb.cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry);
    pb.cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy);
    pb.cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry);
    pb.cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy);
    pb.close();
    pb.finish()
}
