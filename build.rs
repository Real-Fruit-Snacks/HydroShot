fn main() {
    #[cfg(target_os = "windows")]
    {
        // Generate .ico from .png (ICO is just a small header + embedded PNG data)
        let png_data = std::fs::read("assets/icon.png").expect("Failed to read assets/icon.png");
        let mut ico = Vec::new();
        // ICO header: reserved(2) + type=1(2) + count=1(2)
        ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
        // Directory entry: width=0(256), height=0(256), colors=0, reserved=0
        ico.extend_from_slice(&[0, 0, 0, 0]);
        // planes=1, bpp=32
        ico.extend_from_slice(&[1, 0, 32, 0]);
        // PNG data size (4 bytes LE)
        ico.extend_from_slice(&(png_data.len() as u32).to_le_bytes());
        // Offset to PNG data: 6 (header) + 16 (entry) = 22
        ico.extend_from_slice(&22u32.to_le_bytes());
        // PNG data
        ico.extend_from_slice(&png_data);
        std::fs::write("assets/icon.ico", &ico).expect("Failed to write assets/icon.ico");

        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "HydroShot");
        res.set("FileDescription", "Screenshot capture and annotation tool");
        res.set("LegalCopyright", "Copyright 2026 Matt");
        if let Err(e) = res.compile() {
            eprintln!("winres compile error: {}", e);
        }
    }
}
