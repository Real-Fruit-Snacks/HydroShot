#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: a.clamp(0.0, 1.0),
        }
    }

    // Terminal Workbench palette
    pub fn red() -> Self {
        Self::new(1.0, 0.431, 0.478, 1.0) // #ff6e7a
    }

    pub fn blue() -> Self {
        Self::new(0.420, 0.863, 1.0, 1.0) // #6bdcff (accent-alt)
    }

    pub fn green() -> Self {
        Self::new(0.388, 0.949, 0.671, 1.0) // #63f2ab (accent)
    }

    pub fn yellow() -> Self {
        Self::new(0.941, 0.776, 0.455, 1.0) // #f0c674 (warm)
    }

    pub fn white() -> Self {
        Self::new(0.863, 0.894, 0.875, 1.0) // #dce4df (text-normal)
    }

    pub fn mauve() -> Self {
        Self::new(0.718, 0.549, 1.0, 1.0) // #b78cff (violet)
    }

    pub fn peach() -> Self {
        Self::new(0.969, 0.639, 0.361, 1.0) // #f7a35c (orange)
    }

    pub fn teal() -> Self {
        Self::new(0.420, 0.863, 1.0, 1.0) // #6bdcff (accent-alt)
    }

    pub fn sky() -> Self {
        Self::new(0.420, 0.863, 1.0, 1.0) // #6bdcff (accent-alt)
    }

    pub fn lavender() -> Self {
        Self::new(0.388, 0.949, 0.671, 1.0) // #63f2ab (accent)
    }

    pub fn presets() -> &'static [Self] {
        static PRESETS: &[Color] = &[
            Color {
                r: 1.0,
                g: 0.431,
                b: 0.478,
                a: 1.0,
            }, // red #ff6e7a
            Color {
                r: 0.420,
                g: 0.863,
                b: 1.0,
                a: 1.0,
            }, // blue #6bdcff
            Color {
                r: 0.388,
                g: 0.949,
                b: 0.671,
                a: 1.0,
            }, // green #63f2ab
            Color {
                r: 0.941,
                g: 0.776,
                b: 0.455,
                a: 1.0,
            }, // yellow #f0c674
            Color {
                r: 0.718,
                g: 0.549,
                b: 1.0,
                a: 1.0,
            }, // mauve #b78cff
        ];
        PRESETS
    }
}

impl From<Color> for tiny_skia::Color {
    fn from(c: Color) -> Self {
        tiny_skia::Color::from_rgba(c.r, c.g, c.b, c.a).unwrap_or(tiny_skia::Color::BLACK)
    }
}
