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
        Self { r, g, b, a }
    }

    pub fn red() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0)
    }

    pub fn blue() -> Self {
        Self::new(0.0, 0.4, 1.0, 1.0)
    }

    pub fn green() -> Self {
        Self::new(0.0, 0.8, 0.0, 1.0)
    }

    pub fn yellow() -> Self {
        Self::new(1.0, 0.9, 0.0, 1.0)
    }

    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    pub fn presets() -> Vec<Self> {
        vec![
            Self::red(),
            Self::blue(),
            Self::green(),
            Self::yellow(),
            Self::white(),
        ]
    }
}

impl From<Color> for tiny_skia::Color {
    fn from(c: Color) -> Self {
        tiny_skia::Color::from_rgba(c.r, c.g, c.b, c.a).unwrap_or(tiny_skia::Color::BLACK)
    }
}
