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
    let mut pixmap = Pixmap::new(width, height).expect("failed to create Pixmap");

    // Copy source pixels into pixmap WITH premultiplication
    let pm_pixels = pixmap.pixels_mut();
    for i in 0..(width * height) as usize {
        let r = pixels[i * 4];
        let g = pixels[i * 4 + 1];
        let b = pixels[i * 4 + 2];
        let a = pixels[i * 4 + 3];
        pm_pixels[i] = PremultipliedColorU8::from_rgba(r, g, b, a).unwrap();
    }

    // Render each annotation
    for annotation in annotations {
        render_annotation(annotation, &mut pixmap, Some(pixels), Some(width));
    }

    // Demultiply back to straight alpha
    let pm_pixels = pixmap.pixels();
    let mut output = Vec::with_capacity((width * height * 4) as usize);
    for px in pm_pixels {
        let a = px.alpha();
        if a == 0 {
            output.push(0);
            output.push(0);
            output.push(0);
            output.push(0);
        } else {
            let r = (px.red() as u16 * 255 / a as u16) as u8;
            let g = (px.green() as u16 * 255 / a as u16) as u8;
            let b = (px.blue() as u16 * 255 / a as u16) as u8;
            output.push(r);
            output.push(g);
            output.push(b);
            output.push(a);
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
    // Crop the pixel buffer
    let mut cropped = Vec::with_capacity((sel_w * sel_h * 4) as usize);
    for row in sel_y..(sel_y + sel_h) {
        let start = ((row * screenshot_width + sel_x) * 4) as usize;
        let end = start + (sel_w * 4) as usize;
        cropped.extend_from_slice(&screenshot_pixels[start..end]);
    }

    // Offset annotations to selection-relative coordinates
    let offset_annotations: Vec<Annotation> = annotations
        .iter()
        .map(|a| offset_annotation(a, sel_x as f32, sel_y as f32))
        .collect();

    flatten_annotations(&cropped, sel_w, sel_h, &offset_annotations)
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

/// Save pixel data as PNG.
///
/// When `default_dir` is `Some`, auto-saves to that directory with a
/// timestamped filename (no dialog). When `None`, opens a native save dialog.
///
/// Returns `Ok(Some(path))` on success, `Ok(None)` if the user cancelled,
/// or `Err` on failure.
pub fn save_to_file(
    pixels: &[u8],
    width: u32,
    height: u32,
    default_dir: Option<&std::path::Path>,
) -> Result<Option<String>, String> {
    let now = chrono::Local::now();
    let default_name = now.format("hydroshot_%Y-%m-%d_%H%M%S.png").to_string();

    let path = if let Some(dir) = default_dir {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create save directory: {e}"))?;
        dir.join(&default_name)
    } else {
        let chosen = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("PNG Image", &["png"])
            .save_file();

        match chosen {
            Some(p) => p,
            None => return Ok(None),
        }
    };

    let img = image::RgbaImage::from_raw(width, height, pixels.to_vec())
        .ok_or_else(|| "Failed to create image from pixel data".to_string())?;

    img.save(&path)
        .map_err(|e| format!("Failed to save image: {e}"))?;

    Ok(Some(path.to_string_lossy().into_owned()))
}
