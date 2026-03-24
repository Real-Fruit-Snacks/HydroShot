use hydroshot::capture::create_capturer;

fn main() {
    let capturer = create_capturer().expect("Failed to create screen capturer");
    let screens = capturer
        .capture_all_screens()
        .expect("Failed to capture screens");

    println!("Captured {} screen(s)", screens.len());

    for (i, screen) in screens.iter().enumerate() {
        println!(
            "  Screen {}: {}x{} at ({}, {}), scale={:.2}",
            i, screen.width, screen.height, screen.x_offset, screen.y_offset, screen.scale_factor
        );

        let path = format!("capture_test_screen_{}.png", i);
        let img = image::RgbaImage::from_raw(screen.width, screen.height, screen.pixels.clone())
            .expect("Failed to create image from pixels");
        img.save(&path).expect("Failed to save PNG");
        println!("  Saved to {}", path);
    }
}
