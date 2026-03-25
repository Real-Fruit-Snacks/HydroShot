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

    // Catppuccin Mocha palette
    pub fn red() -> Self {
        Self::new(0.953, 0.545, 0.659, 1.0) // #f38ba8
    }

    pub fn blue() -> Self {
        Self::new(0.537, 0.706, 0.980, 1.0) // #89b4fa
    }

    pub fn green() -> Self {
        Self::new(0.651, 0.890, 0.631, 1.0) // #a6e3a1
    }

    pub fn yellow() -> Self {
        Self::new(0.976, 0.886, 0.686, 1.0) // #f9e2af
    }

    pub fn white() -> Self {
        Self::new(0.804, 0.839, 0.957, 1.0) // #cdd6f4 (Mocha Text)
    }

    pub fn mauve() -> Self {
        Self::new(0.796, 0.651, 0.969, 1.0) // #cba6f7
    }

    pub fn peach() -> Self {
        Self::new(0.980, 0.702, 0.529, 1.0) // #fab387
    }

    pub fn teal() -> Self {
        Self::new(0.580, 0.886, 0.835, 1.0) // #94e2d5
    }

    pub fn sky() -> Self {
        Self::new(0.537, 0.863, 0.922, 1.0) // #89dceb
    }

    pub fn lavender() -> Self {
        Self::new(0.706, 0.745, 0.996, 1.0) // #b4befe
    }

    pub fn presets() -> &'static [Self] {
        static PRESETS: &[Color] = &[
            Color { r: 0.953, g: 0.545, b: 0.659, a: 1.0 }, // red
            Color { r: 0.537, g: 0.706, b: 0.980, a: 1.0 }, // blue
            Color { r: 0.651, g: 0.890, b: 0.631, a: 1.0 }, // green
            Color { r: 0.976, g: 0.886, b: 0.686, a: 1.0 }, // yellow
            Color { r: 0.796, g: 0.651, b: 0.969, a: 1.0 }, // mauve
        ];
        PRESETS
    }
}

impl From<Color> for tiny_skia::Color {
    fn from(c: Color) -> Self {
        tiny_skia::Color::from_rgba(c.r, c.g, c.b, c.a).unwrap_or(tiny_skia::Color::BLACK)
    }
}
