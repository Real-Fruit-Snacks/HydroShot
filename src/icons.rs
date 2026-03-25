//! SVG icon rendering using Lucide Icons (MIT license) via resvg.
//!
//! Each icon is stored as an SVG string constant. Icons are rendered to tiny_skia::Pixmap
//! on first use and cached for subsequent frames.

use std::collections::HashMap;

/// Wrap SVG path content in a complete Lucide-style SVG document.
/// Lucide icons use a 24x24 viewBox with stroke-based rendering.
fn wrap_svg(inner: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{}</svg>"#,
        inner
    )
}

/// Return the SVG string for a given icon name.
fn get_svg(name: &str) -> Option<String> {
    let inner = match name {
        "select" => {
            r#"<path d="M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2"/><path d="M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2"/><path d="M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8"/><path d="M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8H12a8 8 0 0 1-8-8V8a2 2 0 1 1 4 0"/>"#
        }
        "arrow" => r#"<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>"#,
        "rectangle" => r#"<rect width="18" height="18" x="3" y="3" rx="2"/>"#,
        "circle" => r#"<circle cx="12" cy="12" r="10"/>"#,
        "line" => r#"<path d="M4 20 L20 4"/>"#,
        "pencil" => {
            r#"<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/>"#
        }
        "highlight" => {
            r#"<path d="m9 11-6 6v3h9l3-3"/><path d="m22 12-4.6 4.6a2 2 0 0 1-2.8 0l-5.2-5.2a2 2 0 0 1 0-2.8L14 4"/>"#
        }
        "text" => {
            r#"<polyline points="4 7 4 4 20 4 20 7"/><line x1="9" x2="15" y1="20" y2="20"/><line x1="12" x2="12" y1="4" y2="20"/>"#
        }
        "pixelate" => {
            r#"<rect width="7" height="7" x="2" y="2" rx="1"/><rect width="7" height="7" x="15" y="2" rx="1"/><rect width="7" height="7" x="2" y="15" rx="1"/><rect width="7" height="7" x="15" y="15" rx="1"/>"#
        }
        "step-marker" => {
            r#"<line x1="4" x2="20" y1="9" y2="9"/><line x1="4" x2="20" y1="15" y2="15"/><line x1="10" x2="8" y1="3" y2="21"/><line x1="16" x2="14" y1="3" y2="21"/>"#
        }
        "eyedropper" => {
            r#"<path d="m2 22 1-1h3l9-9"/><path d="M3 21v-3l9-9"/><path d="m15 6 3.4-3.4a2.1 2.1 0 1 1 3 3L18 9l.4.4a2.1 2.1 0 1 1-3 3l-3.8-3.8a2.1 2.1 0 1 1 3-3l.4.4Z"/>"#
        }
        "pin" => {
            r#"<path d="M12 17v5"/><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/>"#
        }
        "copy" => {
            r#"<rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>"#
        }
        "upload" => {
            r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/>"#
        }
        "save" => {
            r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>"#
        }
        "rounded-rect" => r#"<rect x="3" y="3" width="18" height="18" rx="5"/>"#,
        "ocr" => {
            r#"<path d="M4 7V4h3"/><path d="M17 4h3v3"/><path d="M4 17v3h3"/><path d="M17 20h3v-3"/><path d="M7 8h10"/><path d="M7 12h10"/><path d="M7 16h10"/>"#
        }
        _ => return None,
    };
    Some(wrap_svg(inner))
}

/// Render an SVG string to a tiny_skia::Pixmap at the given size and color.
fn render_icon_svg(svg_data: &str, size: u32, color: &str) -> Option<tiny_skia::Pixmap> {
    let colored_svg = svg_data.replace("currentColor", color);

    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&colored_svg, &options).ok()?;

    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let scale = size as f32 / 24.0;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    Some(pixmap)
}

/// Cache for rendered icon pixmaps, keyed by (name_color, size).
#[derive(Default)]
pub struct IconCache {
    icons: HashMap<(String, u32), tiny_skia::Pixmap>,
}

impl IconCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a cached icon or render it on first access.
    pub fn get_or_render(
        &mut self,
        name: &str,
        size: u32,
        color: &str,
    ) -> Option<&tiny_skia::Pixmap> {
        let key = (format!("{}_{}", name, color), size);
        if !self.icons.contains_key(&key) {
            let svg_data = get_svg(name)?;
            let pixmap = render_icon_svg(&svg_data, size, color)?;
            self.icons.insert(key.clone(), pixmap);
        }
        self.icons.get(&key)
    }
}

/// Alpha-blend `src` pixmap onto `dst` pixmap at the given offset.
pub fn blend_pixmap(
    dst: &mut tiny_skia::Pixmap,
    src: &tiny_skia::Pixmap,
    offset_x: i32,
    offset_y: i32,
) {
    let dst_w = dst.width() as i32;
    let dst_h = dst.height() as i32;
    let src_w = src.width() as i32;
    let src_h = src.height() as i32;

    let dst_data = dst.data_mut();
    let src_data = src.data();

    for sy in 0..src_h {
        let dy = offset_y + sy;
        if dy < 0 || dy >= dst_h {
            continue;
        }
        for sx in 0..src_w {
            let dx = offset_x + sx;
            if dx < 0 || dx >= dst_w {
                continue;
            }

            let si = ((sy * src_w + sx) * 4) as usize;
            let di = ((dy * dst_w + dx) * 4) as usize;

            let sa = src_data[si + 3] as u32;
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                dst_data[di..di + 4].copy_from_slice(&src_data[si..si + 4]);
                continue;
            }

            // Standard alpha compositing (src is premultiplied from resvg)
            let inv_sa = 255 - sa;
            for c in 0..3 {
                let s = src_data[si + c] as u32;
                let d = dst_data[di + c] as u32;
                dst_data[di + c] = (s + (d * inv_sa) / 255).min(255) as u8;
            }
            let da = dst_data[di + 3] as u32;
            dst_data[di + 3] = (sa + (da * inv_sa) / 255).min(255) as u8;
        }
    }
}
