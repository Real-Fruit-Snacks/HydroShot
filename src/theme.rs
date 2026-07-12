//! Terminal Workbench palette and the process-global active mode.
//! Every rendered surface reads its colors through these accessors so a
//! single `set_mode` call re-themes the whole app.

use std::sync::atomic::{AtomicU8, Ordering};

type Rgb = (u8, u8, u8);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Dark,
    Light,
}

static ACTIVE: AtomicU8 = AtomicU8::new(0); // 0 = Dark, 1 = Light

pub fn set_mode(mode: ThemeMode) {
    let v = match mode {
        ThemeMode::Dark => 0,
        ThemeMode::Light => 1,
    };
    ACTIVE.store(v, Ordering::Relaxed);
}

pub fn mode() -> ThemeMode {
    if ACTIVE.load(Ordering::Relaxed) == 1 {
        ThemeMode::Light
    } else {
        ThemeMode::Dark
    }
}

struct Palette {
    bg_0: Rgb,
    bg_1: Rgb,
    bg_2: Rgb,
    bg_3: Rgb,
    bg_4: Rgb,
    border: Rgb,
    border_strong: Rgb,
    text_normal: Rgb,
    text_soft: Rgb,
    text_muted: Rgb,
    text_faint: Rgb,
    text_on_accent: Rgb,
    accent: Rgb,
    accent_alt: Rgb,
    warm: Rgb,
    red: Rgb,
    orange: Rgb,
    violet: Rgb,
}

// Terminal Workbench palette — dark (default).
const DARK: Palette = Palette {
    bg_0: (0x09, 0x0c, 0x0d),
    bg_1: (0x0e, 0x12, 0x14),
    bg_2: (0x13, 0x19, 0x1c),
    bg_3: (0x18, 0x20, 0x24),
    bg_4: (0x20, 0x2a, 0x2f),
    border: (0x2a, 0x36, 0x3d),
    border_strong: (0x39, 0x48, 0x4f),
    text_normal: (0xdc, 0xe4, 0xdf),
    text_soft: (0xb4, 0xc3, 0xbd),
    text_muted: (0x87, 0x99, 0x94),
    text_faint: (0x63, 0x73, 0x6f),
    text_on_accent: (0x07, 0x10, 0x0d),
    accent: (0x63, 0xf2, 0xab),
    accent_alt: (0x6b, 0xdc, 0xff),
    warm: (0xf0, 0xc6, 0x74),
    red: (0xff, 0x6e, 0x7a),
    orange: (0xf7, 0xa3, 0x5c),
    violet: (0xb7, 0x8c, 0xff),
};

// Terminal Workbench palette — light.
const LIGHT: Palette = Palette {
    bg_0: (0xf5, 0xf7, 0xf4),
    bg_1: (0xed, 0xf2, 0xee),
    bg_2: (0xe2, 0xea, 0xe5),
    bg_3: (0xd6, 0xe1, 0xdb),
    bg_4: (0xc8, 0xd5, 0xcf),
    border: (0xbf, 0xcb, 0xc5),
    border_strong: (0x9d, 0xae, 0xa7),
    text_normal: (0x17, 0x20, 0x1d),
    text_soft: (0x34, 0x44, 0x3f),
    text_muted: (0x60, 0x70, 0x6a),
    text_faint: (0x81, 0x91, 0x8a),
    text_on_accent: (0xf9, 0xfb, 0xf8),
    accent: (0x00, 0x7a, 0x4d),
    accent_alt: (0x00, 0x6f, 0x9e),
    warm: (0xa4, 0x66, 0x00),
    red: (0xc8, 0x32, 0x4c),
    orange: (0xb6, 0x58, 0x00),
    violet: (0x73, 0x57, 0xb8),
};

fn active() -> &'static Palette {
    match mode() {
        ThemeMode::Dark => &DARK,
        ThemeMode::Light => &LIGHT,
    }
}

macro_rules! token {
    ($name:ident) => {
        pub fn $name() -> Rgb {
            active().$name
        }
    };
}
token!(bg_0);
token!(bg_1);
token!(bg_2);
token!(bg_3);
token!(bg_4);
token!(border);
token!(border_strong);
token!(text_normal);
token!(text_soft);
token!(text_muted);
token!(text_faint);
token!(text_on_accent);
token!(accent);
token!(accent_alt);
token!(warm);
token!(red);
token!(orange);
token!(violet);

/// Convert a token to a `tiny_skia::Color` at the given alpha (0.0..=1.0).
pub fn skia(rgb: Rgb, alpha: f32) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(
        rgb.0 as f32 / 255.0,
        rgb.1 as f32 / 255.0,
        rgb.2 as f32 / 255.0,
        alpha,
    )
    .unwrap_or(tiny_skia::Color::WHITE)
}

/// Convert a token to the crate's own `geometry::Color` at the given alpha.
pub fn gcolor(rgb: Rgb, alpha: f32) -> crate::geometry::Color {
    crate::geometry::Color::new(
        rgb.0 as f32 / 255.0,
        rgb.1 as f32 / 255.0,
        rgb.2 as f32 / 255.0,
        alpha,
    )
}

/// Lowercase `#rrggbb` string (for the SVG icon renderer).
pub fn hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_differ_and_match_spec() {
        set_mode(ThemeMode::Dark);
        assert_eq!(accent(), (0x63, 0xf2, 0xab));
        assert_eq!(bg_0(), (0x09, 0x0c, 0x0d));
        assert_eq!(text_normal(), (0xdc, 0xe4, 0xdf));

        set_mode(ThemeMode::Light);
        assert_eq!(accent(), (0x00, 0x7a, 0x4d));
        assert_eq!(bg_0(), (0xf5, 0xf7, 0xf4));
        assert_eq!(text_normal(), (0x17, 0x20, 0x1d));

        set_mode(ThemeMode::Dark);
    }

    #[test]
    fn hex_formats_lowercase_six_digits() {
        assert_eq!(hex((0x63, 0xf2, 0xab)), "#63f2ab");
    }

    #[test]
    fn skia_applies_alpha() {
        let c = skia((0xff, 0x00, 0x00), 0.5);
        assert!((c.red() - 1.0).abs() < 0.01);
        assert!((c.alpha() - 0.5).abs() < 0.01);
    }
}
