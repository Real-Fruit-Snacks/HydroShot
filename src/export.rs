use crate::geometry::{Point, Size};
use crate::tools::{render_annotation, Annotation};
use tiny_skia::{Pixmap, PremultipliedColorU8};

/// Flatten annotations onto a pixel buffer.
///
/// Input and output are straight (non-premultiplied) RGBA8 pixels.
/// Internally converts to premultiplied alpha for tiny-skia rendering,
/// then converts back to straight alpha for the output.
pub fn flatten_annotations(
    pixels: &[u8],
    width: u32,
    height: u32,
    annotations: &[Annotation],
) -> Vec<u8> {
    let mut pixmap = match Pixmap::new(width, height) {
        Some(p) => p,
        None => return pixels.to_vec(), // zero-dimension: return input unchanged
    };

    // Copy source pixels into pixmap WITH premultiplication
    let expected_len = (width as usize) * (height as usize) * 4;
    if pixels.len() < expected_len {
        return pixels.to_vec(); // undersized buffer: return input unchanged
    }

    // Fast path: screenshots are fully opaque (a=255), so premultiplication is a
    // no-op — we can bulk-copy instead of per-pixel conversion.
    let all_opaque = pixels.chunks_exact(4).all(|px| px[3] == 255);
    let pm_pixels = pixmap.pixels_mut();
    if all_opaque {
        for (i, chunk) in pixels.chunks_exact(4).enumerate() {
            pm_pixels[i] =
                PremultipliedColorU8::from_rgba(chunk[0], chunk[1], chunk[2], 255).unwrap();
        }
    } else {
        for (i, chunk) in pixels.chunks_exact(4).enumerate() {
            pm_pixels[i] =
                PremultipliedColorU8::from_rgba(chunk[0], chunk[1], chunk[2], chunk[3]).unwrap();
        }
    }

    // Render each annotation
    for annotation in annotations {
        render_annotation(annotation, &mut pixmap, Some(pixels), Some(width));
    }

    // Demultiply back to straight alpha
    let pm_pixels = pixmap.pixels();
    let mut output = vec![0u8; expected_len];
    for (i, px) in pm_pixels.iter().enumerate() {
        let a = px.alpha();
        let base = i * 4;
        if a == 0 {
            // output is already zeroed
        } else if a == 255 {
            // Fully opaque: no division needed
            output[base] = px.red();
            output[base + 1] = px.green();
            output[base + 2] = px.blue();
            output[base + 3] = 255;
        } else {
            output[base] = (px.red() as u16 * 255 / a as u16) as u8;
            output[base + 1] = (px.green() as u16 * 255 / a as u16) as u8;
            output[base + 2] = (px.blue() as u16 * 255 / a as u16) as u8;
            output[base + 3] = a;
        }
    }

    output
}

/// Crop a screenshot to a selection region, offset annotations, and flatten.
///
/// `screenshot_pixels` is straight RGBA8 for the full screenshot.
/// Annotations are in screenshot-absolute coordinates and will be offset
/// by (-sel_x, -sel_y) to become selection-relative.
pub fn crop_and_flatten(
    screenshot_pixels: &[u8],
    screenshot_width: u32,
    sel_x: u32,
    sel_y: u32,
    sel_w: u32,
    sel_h: u32,
    annotations: &[Annotation],
) -> Vec<u8> {
    // Clamp selection to screenshot bounds to prevent out-of-bounds access
    let screenshot_height = screenshot_pixels.len() as u32 / (screenshot_width * 4).max(1);
    let clamped_w = sel_w.min(screenshot_width.saturating_sub(sel_x));
    let clamped_h = sel_h.min(screenshot_height.saturating_sub(sel_y));

    // Crop the pixel buffer
    let mut cropped = Vec::with_capacity((clamped_w as usize) * (clamped_h as usize) * 4);
    for row in sel_y..(sel_y + clamped_h) {
        let start = ((row * screenshot_width + sel_x) * 4) as usize;
        let end = start + (clamped_w * 4) as usize;
        if end <= screenshot_pixels.len() {
            cropped.extend_from_slice(&screenshot_pixels[start..end]);
        }
    }

    // Offset annotations to selection-relative coordinates
    let offset_annotations: Vec<Annotation> = annotations
        .iter()
        .map(|a| offset_annotation(a, sel_x as f32, sel_y as f32))
        .collect();

    flatten_annotations(&cropped, clamped_w, clamped_h, &offset_annotations)
}

/// Offset an annotation's coordinates by (-dx, -dy).
fn offset_annotation(annotation: &Annotation, dx: f32, dy: f32) -> Annotation {
    match annotation {
        Annotation::Arrow {
            start,
            end,
            color,
            thickness,
        } => Annotation::Arrow {
            start: Point::new(start.x - dx, start.y - dy),
            end: Point::new(end.x - dx, end.y - dy),
            color: *color,
            thickness: *thickness,
        },
        Annotation::Rectangle {
            top_left,
            size,
            color,
            thickness,
        } => Annotation::Rectangle {
            top_left: Point::new(top_left.x - dx, top_left.y - dy),
            size: Size::new(size.width, size.height),
            color: *color,
            thickness: *thickness,
        },
        Annotation::Pencil {
            points,
            color,
            thickness,
        } => Annotation::Pencil {
            points: points
                .iter()
                .map(|p| Point::new(p.x - dx, p.y - dy))
                .collect(),
            color: *color,
            thickness: *thickness,
        },
        Annotation::Text {
            position,
            text,
            color,
            font_size,
        } => Annotation::Text {
            position: Point::new(position.x - dx, position.y - dy),
            text: text.clone(),
            color: *color,
            font_size: *font_size,
        },
        Annotation::Pixelate {
            top_left,
            size,
            block_size,
        } => Annotation::Pixelate {
            top_left: Point::new(top_left.x - dx, top_left.y - dy),
            size: Size::new(size.width, size.height),
            block_size: *block_size,
        },
        Annotation::Ellipse {
            center,
            radius_x,
            radius_y,
            color,
            thickness,
        } => Annotation::Ellipse {
            center: Point::new(center.x - dx, center.y - dy),
            radius_x: *radius_x,
            radius_y: *radius_y,
            color: *color,
            thickness: *thickness,
        },
        Annotation::Line {
            start,
            end,
            color,
            thickness,
        } => Annotation::Line {
            start: Point::new(start.x - dx, start.y - dy),
            end: Point::new(end.x - dx, end.y - dy),
            color: *color,
            thickness: *thickness,
        },
        Annotation::Highlight {
            top_left,
            size,
            color,
        } => Annotation::Highlight {
            top_left: Point::new(top_left.x - dx, top_left.y - dy),
            size: Size::new(size.width, size.height),
            color: *color,
        },
        Annotation::StepMarker {
            position,
            number,
            color,
            size,
        } => Annotation::StepMarker {
            position: Point::new(position.x - dx, position.y - dy),
            number: *number,
            color: *color,
            size: *size,
        },
        Annotation::RoundedRect {
            top_left,
            size,
            color,
            thickness,
            radius,
        } => Annotation::RoundedRect {
            top_left: Point::new(top_left.x - dx, top_left.y - dy),
            size: Size::new(size.width, size.height),
            color: *color,
            thickness: *thickness,
            radius: *radius,
        },
        Annotation::Spotlight { top_left, size } => Annotation::Spotlight {
            top_left: Point::new(top_left.x - dx, top_left.y - dy),
            size: Size::new(size.width, size.height),
        },
        Annotation::Measurement { start, end, color } => Annotation::Measurement {
            start: Point::new(start.x - dx, start.y - dy),
            end: Point::new(end.x - dx, end.y - dy),
            color: *color,
        },
    }
}

/// Copy pixel data to the system clipboard.
///
/// `pixels` is straight RGBA8 data.
pub fn copy_to_clipboard(pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};

    let mut clipboard = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {e}"))?;
    let img = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: std::borrow::Cow::Borrowed(pixels),
    };
    clipboard
        .set_image(img)
        .map_err(|e| format!("Failed to copy image to clipboard: {e}"))
}

/// Save pixel data to an image file (accepts owned Vec to avoid redundant copy).
///
/// When `default_dir` is `Some`, auto-saves to that directory with a
/// timestamped PNG filename (no dialog). When `None`, opens a native save
/// dialog offering PNG, JPEG, and WebP formats.
///
/// Returns `Ok(Some(path))` on success, `Ok(None)` if the user cancelled,
/// or `Err` on failure.
pub fn save_to_file(
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    default_dir: Option<&std::path::Path>,
) -> Result<Option<String>, String> {
    let now = chrono::Local::now();
    let default_name = now.format("hydroshot_%Y-%m-%d_%H%M%S.png").to_string();

    if let Some(dir) = default_dir {
        // Auto-save (no dialog) — always PNG
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create save directory: {e}"))?;
        let path = dir.join(&default_name);
        let img = image::RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| "Failed to create image from pixel data".to_string())?;
        img.save(&path)
            .map_err(|e| format!("Failed to save image: {e}"))?;
        return Ok(Some(path.to_string_lossy().into_owned()));
    }

    // Show dialog with format options
    let path = rfd::FileDialog::new()
        .set_file_name(&default_name)
        .add_filter("PNG Image", &["png"])
        .add_filter("JPEG Image", &["jpg", "jpeg"])
        .add_filter("WebP Image", &["webp"])
        .save_file();

    match path {
        Some(p) => {
            let img = image::RgbaImage::from_raw(width, height, pixels)
                .ok_or_else(|| "Failed to create image from pixel data".to_string())?;

            // Detect format from extension
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png")
                .to_lowercase();

            match ext.as_str() {
                "jpg" | "jpeg" => {
                    // Convert RGBA to RGB for JPEG (JPEG doesn't support alpha)
                    let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
                    rgb_img
                        .save(&p)
                        .map_err(|e| format!("Failed to save image: {e}"))?;
                }
                _ => {
                    // PNG, WebP, and any other extension — image crate infers
                    // the format from the extension automatically.
                    img.save(&p)
                        .map_err(|e| format!("Failed to save image: {e}"))?;
                }
            }

            Ok(Some(p.to_string_lossy().into_owned()))
        }
        None => Ok(None),
    }
}
