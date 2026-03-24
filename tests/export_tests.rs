use hydroshot::export::{crop_and_flatten, flatten_annotations};
use hydroshot::geometry::{Color, Point, Size};
use hydroshot::tools::Annotation;

#[test]
fn flatten_empty_annotations_preserves_pixels() {
    let width: u32 = 4;
    let height: u32 = 4;
    // Create a simple test image: all red pixels (straight RGBA)
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        pixels.push(255); // R
        pixels.push(0);   // G
        pixels.push(0);   // B
        pixels.push(255); // A
    }

    let annotations: Vec<Annotation> = vec![];
    let result = flatten_annotations(&pixels, width, height, &annotations);

    // With no annotations, the output should match the input
    assert_eq!(result.len(), pixels.len());
    for i in 0..pixels.len() {
        assert_eq!(
            result[i], pixels[i],
            "Pixel mismatch at byte index {}: expected {}, got {}",
            i, pixels[i], result[i]
        );
    }
}

#[test]
fn flatten_with_rectangle_modifies_pixels() {
    let width: u32 = 64;
    let height: u32 = 64;
    // Create a fully white image (straight RGBA)
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        pixels.push(255); // R
        pixels.push(255); // G
        pixels.push(255); // B
        pixels.push(255); // A
    }

    let annotations = vec![Annotation::Rectangle {
        top_left: Point::new(10.0, 10.0),
        size: Size::new(40.0, 40.0),
        color: Color::red(),
        thickness: 3.0,
    }];

    let result = flatten_annotations(&pixels, width, height, &annotations);

    assert_eq!(result.len(), pixels.len());

    // The result should differ from the input since a red rectangle was drawn
    let differs = result.iter().zip(pixels.iter()).any(|(a, b)| a != b);
    assert!(
        differs,
        "Expected pixel buffer to change after drawing rectangle annotation"
    );
}

#[test]
fn crop_and_flatten_crops_correctly() {
    let width: u32 = 10;
    let height: u32 = 10;
    // Create a gradient-like image so we can verify cropping
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.push(x as u8 * 25);  // R varies by column
            pixels.push(y as u8 * 25);  // G varies by row
            pixels.push(128);           // B constant
            pixels.push(255);           // A opaque
        }
    }

    let annotations: Vec<Annotation> = vec![];
    let result = crop_and_flatten(&pixels, width, 2, 3, 4, 4, &annotations);

    // Output should be 4x4 pixels = 64 bytes
    assert_eq!(result.len(), (4 * 4 * 4) as usize);

    // Check first pixel of cropped region corresponds to (2,3) in original
    assert_eq!(result[0], 2 * 25);   // R = x=2 * 25
    assert_eq!(result[1], 3 * 25);   // G = y=3 * 25
    assert_eq!(result[2], 128);      // B
    assert_eq!(result[3], 255);      // A
}

#[test]
fn crop_and_flatten_offsets_annotations() {
    let width: u32 = 100;
    let height: u32 = 100;
    // White image
    let pixels = vec![255u8; (width * height * 4) as usize];

    // Annotation at absolute coordinates
    let annotations = vec![Annotation::Rectangle {
        top_left: Point::new(30.0, 30.0),
        size: Size::new(20.0, 20.0),
        color: Color::red(),
        thickness: 2.0,
    }];

    // Crop a region that includes the annotation
    let sel_x = 20;
    let sel_y = 20;
    let sel_w = 60;
    let sel_h = 60;

    let result = crop_and_flatten(&pixels, width, sel_x, sel_y, sel_w, sel_h, &annotations);
    assert_eq!(result.len(), (sel_w * sel_h * 4) as usize);

    // The rectangle should have been rendered (offset to 10,10 in the cropped image)
    // so pixels should differ from all-white
    let differs = result.iter().enumerate().any(|(i, &v)| {
        // Only check non-alpha channels
        i % 4 != 3 && v != 255
    });
    assert!(
        differs,
        "Expected annotation to be rendered in the cropped output"
    );
}
