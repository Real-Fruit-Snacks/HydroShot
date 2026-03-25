use std::sync::LazyLock;

pub static FONT: LazyLock<fontdue::Font> = LazyLock::new(|| {
    static FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");
    fontdue::Font::from_bytes(FONT_DATA as &[u8], fontdue::FontSettings::default())
        .expect("Failed to load embedded font")
});
