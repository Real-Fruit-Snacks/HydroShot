# HydroShot Phase 2A Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add pencil, text, and pixelate annotation tools, global hotkey (Ctrl+Shift+S), and settings persistence to make HydroShot a daily-driver screenshot tool.

**Architecture:** Three new tools follow the existing `AnnotationTool` trait. Text rendering uses `fontdue` for glyph rasterization. Pixelation reads original screenshot pixels and block-averages them. Global hotkey via `global-hotkey` crate polled in the winit event loop alongside tray events. Config stored as TOML at the platform-standard config directory.

**Tech Stack:** Existing (Rust, winit, tiny-skia, softbuffer, tray-icon) + `global-hotkey`, `fontdue`, `toml`, `serde`, `dirs`

**Spec:** `docs/superpowers/specs/2026-03-24-hydroshot-phase2a-design.md`

---

## File Structure

**New files:**
```
hydroshot/
├── assets/
│   └── font.ttf                   # Bundled monospace font (e.g., DejaVu Sans Mono)
├── src/
│   ├── config.rs                  # Config struct, load/save, platform paths
│   ├── hotkey.rs                  # Global hotkey registration
│   ├── tools/
│   │   ├── pencil.rs              # PencilTool — freehand drawing
│   │   ├── text.rs                # TextTool — single-line text placement
│   │   └── pixelate.rs            # PixelateTool — block-average pixelation
```

**Modified files:**
```
├── Cargo.toml                     # Add 5 new dependencies
├── src/
│   ├── lib.rs                     # Add pub mod config, hotkey
│   ├── tools/mod.rs               # 3 new Annotation variants, 3 ToolKind variants, extend render_annotation()
│   ├── overlay/toolbar.rs         # BUTTON_COUNT 9→12
│   ├── state.rs                   # 3 new tool fields, text input state, new() takes &Config
│   ├── renderer.rs                # New button icons, text preview, pixelate rendering, separator indices
│   ├── export.rs                  # offset_annotation for new variants, save_to_file default dir
│   ├── main.rs                    # Config loading, hotkey polling, text input routing, tool shortcuts
```

---

## Task 0: Dependencies & Font Asset

**Files:**
- Modify: `hydroshot/Cargo.toml`
- Create: `hydroshot/assets/font.ttf`

- [ ] **Step 1: Add new dependencies to Cargo.toml**

Add under `[dependencies]`:
```toml
global-hotkey = "0.6"
fontdue = "0.9"
toml = "0.8"
serde = { version = "1", features = ["derive"] }
dirs = "6"
```

- [ ] **Step 2: Bundle a monospace font**

Download DejaVu Sans Mono (open source, permissive license) and place it at `hydroshot/assets/font.ttf`. Alternatively, use any open-source monospace TTF.

```bash
cd hydroshot && curl -L -o assets/font.ttf "https://github.com/dejavu-fonts/dejavu-fonts/raw/main/ttf/DejaVuSansMono.ttf"
```

If curl fails, manually download any monospace TTF and place it at `assets/font.ttf`.

- [ ] **Step 3: Verify build**

```bash
cd hydroshot && cargo check
```

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock assets/font.ttf
git commit -m "chore: add Phase 2A dependencies and bundled font"
```

---

## Task 1: Config Module

**Files:**
- Create: `hydroshot/src/config.rs`
- Create: `hydroshot/tests/config_tests.rs`
- Modify: `hydroshot/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `hydroshot/tests/config_tests.rs`:

```rust
use hydroshot::config::Config;
use hydroshot::geometry::Color;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.general.default_color, "red");
    assert_eq!(config.general.default_thickness, 3.0);
    assert_eq!(config.general.save_directory, "");
    assert_eq!(config.hotkey.capture, "Ctrl+Shift+S");
}

#[test]
fn test_parse_color_valid() {
    let config = Config::default();
    let color = config.default_color();
    assert_eq!(color, Color::red());
}

#[test]
fn test_parse_color_invalid_falls_back() {
    let mut config = Config::default();
    config.general.default_color = "invalid".to_string();
    assert_eq!(config.default_color(), Color::red());
}

#[test]
fn test_serialize_roundtrip() {
    let config = Config::default();
    let toml_str = toml::to_string(&config).unwrap();
    let parsed: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.general.default_color, "red");
    assert_eq!(parsed.hotkey.capture, "Ctrl+Shift+S");
}

#[test]
fn test_thickness_clamped_on_load() {
    let mut config = Config::default();
    config.general.default_thickness = 100.0;
    assert_eq!(config.clamped_thickness(), 20.0);
    config.general.default_thickness = -5.0;
    assert_eq!(config.clamped_thickness(), 1.0);
}
```

- [ ] **Step 2: Run tests — verify failure**

```bash
cd hydroshot && cargo test --test config_tests
```

- [ ] **Step 3: Implement config.rs**

Create `hydroshot/src/config.rs`:

```rust
use crate::geometry::Color;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub hotkey: HotkeyConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub default_color: String,
    pub default_thickness: f32,
    pub save_directory: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HotkeyConfig {
    pub capture: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                default_color: "red".to_string(),
                default_thickness: 3.0,
                save_directory: String::new(),
            },
            hotkey: HotkeyConfig {
                capture: "Ctrl+Shift+S".to_string(),
            },
        }
    }
}

impl Config {
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("hydroshot").join("config.toml"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            tracing::warn!("Could not determine config directory, using defaults");
            return Self::default();
        };

        if !path.exists() {
            let config = Self::default();
            if let Err(e) = config.save() {
                tracing::warn!("Could not save default config: {}", e);
            }
            return config;
        }

        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!("Invalid config file, using defaults: {}", e);
                    Self::default()
                }
            },
            Err(e) => {
                tracing::warn!("Could not read config: {}", e);
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let Some(path) = Self::config_path() else {
            return Err("Could not determine config directory".into());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let contents = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, contents).map_err(|e| e.to_string())
    }

    pub fn default_color(&self) -> Color {
        match self.general.default_color.as_str() {
            "red" => Color::red(),
            "blue" => Color::blue(),
            "green" => Color::green(),
            "yellow" => Color::yellow(),
            "white" => Color::white(),
            _ => Color::red(),
        }
    }

    pub fn clamped_thickness(&self) -> f32 {
        self.general.default_thickness.clamp(1.0, 20.0)
    }

    pub fn save_directory(&self) -> Option<PathBuf> {
        if self.general.save_directory.is_empty() {
            None
        } else {
            let path = PathBuf::from(&self.general.save_directory);
            if path.is_dir() { Some(path) } else { None }
        }
    }
}
```

Add `pub mod config;` to `hydroshot/src/lib.rs`.

- [ ] **Step 4: Run tests — verify pass**

```bash
cd hydroshot && cargo test --test config_tests
```

Expected: All 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs src/lib.rs tests/config_tests.rs
git commit -m "feat: config module — TOML settings persistence"
```

---

## Task 2: Pencil Tool

**Files:**
- Create: `hydroshot/src/tools/pencil.rs`
- Create: `hydroshot/tests/pencil_tests.rs`
- Modify: `hydroshot/src/tools/mod.rs`

- [ ] **Step 1: Write failing tests**

Create `hydroshot/tests/pencil_tests.rs`:

```rust
use hydroshot::geometry::{Point, Color};
use hydroshot::tools::{Annotation, AnnotationTool};
use hydroshot::tools::pencil::PencilTool;

#[test]
fn test_pencil_produces_annotation() {
    let mut tool = PencilTool::new(Color::red(), 3.0);
    tool.on_mouse_down(Point::new(10.0, 10.0));
    tool.on_mouse_move(Point::new(20.0, 20.0));
    tool.on_mouse_move(Point::new(30.0, 15.0));
    let ann = tool.on_mouse_up(Point::new(40.0, 25.0)).unwrap();
    match ann {
        Annotation::Pencil { points, color, thickness } => {
            assert!(points.len() >= 3);
            assert_eq!(color, Color::red());
            assert_eq!(thickness, 3.0);
        }
        _ => panic!("Expected Pencil"),
    }
}

#[test]
fn test_pencil_no_annotation_without_mousedown() {
    let mut tool = PencilTool::new(Color::red(), 3.0);
    assert!(tool.on_mouse_up(Point::new(10.0, 10.0)).is_none());
}

#[test]
fn test_pencil_in_progress() {
    let mut tool = PencilTool::new(Color::red(), 3.0);
    assert!(tool.in_progress_annotation().is_none());
    tool.on_mouse_down(Point::new(0.0, 0.0));
    tool.on_mouse_move(Point::new(10.0, 10.0));
    let preview = tool.in_progress_annotation().unwrap();
    match preview {
        Annotation::Pencil { points, .. } => assert!(points.len() >= 2),
        _ => panic!(),
    }
}

#[test]
fn test_pencil_render() {
    use hydroshot::tools::render_annotation;
    let ann = Annotation::Pencil {
        points: vec![Point::new(10.0, 10.0), Point::new(50.0, 50.0), Point::new(90.0, 10.0)],
        color: Color::red(),
        thickness: 3.0,
    };
    let mut pixmap = tiny_skia::Pixmap::new(100, 100).unwrap();
    render_annotation(&ann, &mut pixmap, None, None);
    // Middle of the path should have colored pixels
    let px = pixmap.pixel(30, 30).unwrap();
    // At least some pixels should be non-zero (path passes nearby)
    // This is a smoke test — exact pixel depends on anti-aliasing
}
```

- [ ] **Step 2: Run tests — verify failure**

```bash
cd hydroshot && cargo test --test pencil_tests
```

- [ ] **Step 3: Add Pencil variant to Annotation and ToolKind**

In `hydroshot/src/tools/mod.rs`, add:

```rust
// In Annotation enum:
Pencil {
    points: Vec<Point>,
    color: Color,
    thickness: f32,
},

// In ToolKind enum:
Pencil,
```

Update `render_annotation` signature to:
```rust
pub fn render_annotation(
    annotation: &Annotation,
    pixmap: &mut tiny_skia::Pixmap,
    screenshot_pixels: Option<&[u8]>,
    screenshot_width: Option<u32>,
)
```

Add Pencil match arm to `render_annotation`:
```rust
Annotation::Pencil { points, color, thickness } => {
    if points.len() < 2 { return; }
    let mut pb = PathBuilder::new();
    pb.move_to(points[0].x, points[0].y);
    for p in &points[1..] {
        pb.line_to(p.x, p.y);
    }
    if let Some(path) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color((*color).into());
        paint.anti_alias = true;
        let stroke = Stroke { width: *thickness, ..Stroke::default() };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}
```

Update ALL existing `render_annotation` call sites to pass the two new parameters (`None, None` for now — pixelate will use them later). Call sites to update:
- `hydroshot/src/renderer.rs` (lines 85, 94 — the two `render_annotation` calls)
- `hydroshot/src/export.rs` (line 30 — inside `flatten_annotations`)

Add `pub mod pencil;` to `tools/mod.rs`.

Also add the `Pencil` arm to `offset_annotation` in `export.rs`:
```rust
Annotation::Pencil { points, color, thickness } => Annotation::Pencil {
    points: points.iter().map(|p| Point::new(p.x - dx, p.y - dy)).collect(),
    color: *color, thickness: *thickness,
},
```

- [ ] **Step 4: Implement PencilTool**

Create `hydroshot/src/tools/pencil.rs`:

```rust
use crate::geometry::{Color, Point};
use super::{Annotation, AnnotationTool};

pub struct PencilTool {
    color: Color,
    thickness: f32,
    points: Vec<Point>,
    drawing: bool,
}

impl PencilTool {
    pub fn new(color: Color, thickness: f32) -> Self {
        Self { color, thickness: thickness.clamp(1.0, 20.0), points: Vec::new(), drawing: false }
    }
    pub fn set_color(&mut self, color: Color) { self.color = color; }
    pub fn set_thickness(&mut self, t: f32) { self.thickness = t.clamp(1.0, 20.0); }
}

impl AnnotationTool for PencilTool {
    fn on_mouse_down(&mut self, pos: Point) {
        self.points.clear();
        self.points.push(pos);
        self.drawing = true;
    }

    fn on_mouse_move(&mut self, pos: Point) {
        if self.drawing {
            self.points.push(pos);
        }
    }

    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation> {
        if !self.drawing { return None; }
        self.points.push(pos);
        self.drawing = false;
        let points = std::mem::take(&mut self.points);
        if points.len() < 2 { return None; }
        Some(Annotation::Pencil { points, color: self.color, thickness: self.thickness })
    }

    fn is_drawing(&self) -> bool { self.drawing }

    fn in_progress_annotation(&self) -> Option<Annotation> {
        if !self.drawing || self.points.len() < 2 { return None; }
        Some(Annotation::Pencil { points: self.points.clone(), color: self.color, thickness: self.thickness })
    }
}
```

- [ ] **Step 5: Run tests — verify pass**

```bash
cd hydroshot && cargo test --test pencil_tests
```

- [ ] **Step 6: Commit**

```bash
git add src/tools/pencil.rs src/tools/mod.rs tests/pencil_tests.rs
git commit -m "feat: pencil tool — freehand drawing"
```

---

## Task 3: Text Tool

**Files:**
- Create: `hydroshot/src/tools/text.rs`
- Create: `hydroshot/tests/text_tests.rs`
- Modify: `hydroshot/src/tools/mod.rs`
- Modify: `hydroshot/src/state.rs`

- [ ] **Step 1: Write failing tests**

Create `hydroshot/tests/text_tests.rs`:

```rust
use hydroshot::geometry::{Point, Color};
use hydroshot::tools::text::TextTool;
use hydroshot::tools::AnnotationTool;

#[test]
fn test_text_tool_sets_pending_position() {
    let mut tool = TextTool::new(Color::red(), 16.0);
    tool.on_mouse_down(Point::new(100.0, 200.0));
    let pos = tool.take_pending_position();
    assert_eq!(pos, Some(Point::new(100.0, 200.0)));
}

#[test]
fn test_text_tool_pending_consumed_once() {
    let mut tool = TextTool::new(Color::red(), 16.0);
    tool.on_mouse_down(Point::new(100.0, 200.0));
    tool.take_pending_position();
    assert_eq!(tool.take_pending_position(), None);
}

#[test]
fn test_text_tool_no_annotation_from_mouse() {
    let mut tool = TextTool::new(Color::red(), 16.0);
    tool.on_mouse_down(Point::new(100.0, 200.0));
    assert!(tool.on_mouse_up(Point::new(100.0, 200.0)).is_none());
}

#[test]
fn test_text_tool_in_progress_is_none() {
    let tool = TextTool::new(Color::red(), 16.0);
    assert!(tool.in_progress_annotation().is_none());
}

#[test]
fn test_text_render() {
    use hydroshot::tools::{Annotation, render_annotation};
    let ann = Annotation::Text {
        position: Point::new(10.0, 10.0),
        text: "Hello".to_string(),
        color: Color::red(),
        font_size: 20.0,
    };
    let mut pixmap = tiny_skia::Pixmap::new(200, 50).unwrap();
    render_annotation(&ann, &mut pixmap, None, None);
    // Some pixels should be non-transparent where text was drawn
    let has_content = pixmap.pixels().iter().any(|p| p.alpha() > 0);
    assert!(has_content, "Text should render visible pixels");
}
```

- [ ] **Step 2: Run tests — verify failure**

```bash
cd hydroshot && cargo test --test text_tests
```

- [ ] **Step 3: Add Text variant to Annotation and ToolKind**

In `hydroshot/src/tools/mod.rs`, add:

```rust
// In Annotation enum:
Text {
    position: Point,
    text: String,
    color: Color,
    font_size: f32,
},

// In ToolKind:
Text,
```

Add Text match arm to `render_annotation` using `fontdue`:

```rust
Annotation::Text { position, text, color, font_size } => {
    let font_data = include_bytes!("../../assets/font.ttf");
    let font = fontdue::Font::from_bytes(font_data as &[u8], fontdue::FontSettings::default())
        .expect("Failed to load font");

    let mut x_offset = 0.0;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, *font_size);
        let gx = position.x as i32 + x_offset as i32 + metrics.xmin;
        let gy = position.y as i32 + *font_size as i32 - metrics.height as i32 - metrics.ymin;

        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let alpha = bitmap[row * metrics.width + col];
                if alpha == 0 { continue; }
                let px = gx + col as i32;
                let py = gy + row as i32;
                if px >= 0 && py >= 0 && (px as u32) < pixmap.width() && (py as u32) < pixmap.height() {
                    let a_f = alpha as f32 / 255.0;
                    let r = (color.r * 255.0 * a_f) as u8;
                    let g = (color.g * 255.0 * a_f) as u8;
                    let b = (color.b * 255.0 * a_f) as u8;
                    let a = (color.a * a_f * 255.0) as u8;
                    if let Some(c) = tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, a) {
                        // Alpha-blend onto existing pixel
                        let idx = (py as u32 * pixmap.width() + px as u32) as usize;
                        if idx < pixmap.pixels().len() {
                            let dst = pixmap.pixels()[idx];
                            let blended = alpha_blend(c, dst);
                            pixmap.pixels_mut()[idx] = blended;
                        }
                    }
                }
            }
        }
        x_offset += metrics.advance_width;
    }
}
```

Add a helper `alpha_blend` function in `tools/mod.rs`:
```rust
fn alpha_blend(src: tiny_skia::PremultipliedColorU8, dst: tiny_skia::PremultipliedColorU8) -> tiny_skia::PremultipliedColorU8 {
    let sa = src.alpha() as u16;
    let da = dst.alpha() as u16;
    let inv_sa = 255 - sa;
    let a = sa + (da * inv_sa / 255);
    let r = (src.red() as u16 + dst.red() as u16 * inv_sa / 255) as u8;
    let g = (src.green() as u16 + dst.green() as u16 * inv_sa / 255) as u8;
    let b = (src.blue() as u16 + dst.blue() as u16 * inv_sa / 255) as u8;
    tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, a as u8)
        .unwrap_or(dst)
}
```

Add `pub mod text;`.

Also add the `Text` arm to `offset_annotation` in `export.rs`:
```rust
Annotation::Text { position, text, color, font_size } => Annotation::Text {
    position: Point::new(position.x - dx, position.y - dy),
    text: text.clone(), color: *color, font_size: *font_size,
},
```

- [ ] **Step 4: Implement TextTool**

Create `hydroshot/src/tools/text.rs`:

```rust
use crate::geometry::{Color, Point};
use super::{Annotation, AnnotationTool};

pub struct TextTool {
    color: Color,
    font_size: f32,
    pending_position: Option<Point>,
}

impl TextTool {
    pub fn new(color: Color, font_size: f32) -> Self {
        Self { color, font_size: font_size.clamp(10.0, 72.0), pending_position: None }
    }
    pub fn set_color(&mut self, color: Color) { self.color = color; }
    pub fn set_font_size(&mut self, size: f32) { self.font_size = size.clamp(10.0, 72.0); }
    pub fn font_size(&self) -> f32 { self.font_size }
    pub fn color(&self) -> Color { self.color }

    /// Consume the pending click position. Called by the event loop
    /// to activate text input mode.
    pub fn take_pending_position(&mut self) -> Option<Point> {
        self.pending_position.take()
    }
}

impl AnnotationTool for TextTool {
    fn on_mouse_down(&mut self, pos: Point) {
        self.pending_position = Some(pos);
    }
    fn on_mouse_move(&mut self, _pos: Point) {} // no-op
    fn on_mouse_up(&mut self, _pos: Point) -> Option<Annotation> { None } // text confirmed via Enter
    fn is_drawing(&self) -> bool { false }
    fn in_progress_annotation(&self) -> Option<Annotation> { None } // preview handled by renderer
}
```

- [ ] **Step 5: Add text input state to OverlayState**

In `hydroshot/src/state.rs`, add fields:

```rust
pub text_tool: TextTool,
pub text_input_active: bool,
pub text_input_buffer: String,
pub text_input_position: Point,
pub text_input_font_size: f32,
```

Initialize in `OverlayState::new`:
```rust
text_tool: TextTool::new(color, 16.0),
text_input_active: false,
text_input_buffer: String::new(),
text_input_position: Point::new(0.0, 0.0),
text_input_font_size: 16.0,
```

- [ ] **Step 6: Run tests — verify pass**

```bash
cd hydroshot && cargo test --test text_tests
```

- [ ] **Step 7: Commit**

```bash
git add src/tools/text.rs src/tools/mod.rs src/state.rs tests/text_tests.rs
git commit -m "feat: text tool — single-line text annotations with fontdue"
```

---

## Task 4: Pixelate Tool

**Files:**
- Create: `hydroshot/src/tools/pixelate.rs`
- Create: `hydroshot/tests/pixelate_tests.rs`
- Modify: `hydroshot/src/tools/mod.rs`

- [ ] **Step 1: Write failing tests**

Create `hydroshot/tests/pixelate_tests.rs`:

```rust
use hydroshot::geometry::{Point, Size, Color};
use hydroshot::tools::{Annotation, AnnotationTool};
use hydroshot::tools::pixelate::PixelateTool;

#[test]
fn test_pixelate_produces_annotation() {
    let mut tool = PixelateTool::new();
    tool.on_mouse_down(Point::new(10.0, 10.0));
    let ann = tool.on_mouse_up(Point::new(60.0, 60.0)).unwrap();
    match ann {
        Annotation::Pixelate { top_left, size, block_size } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
            assert_eq!(size.width, 50.0);
            assert_eq!(size.height, 50.0);
            assert_eq!(block_size, 10);
        }
        _ => panic!("Expected Pixelate"),
    }
}

#[test]
fn test_pixelate_normalizes() {
    let mut tool = PixelateTool::new();
    tool.on_mouse_down(Point::new(60.0, 60.0));
    let ann = tool.on_mouse_up(Point::new(10.0, 10.0)).unwrap();
    match ann {
        Annotation::Pixelate { top_left, .. } => {
            assert_eq!(top_left, Point::new(10.0, 10.0));
        }
        _ => panic!(),
    }
}

#[test]
fn test_pixelate_render() {
    use hydroshot::tools::render_annotation;

    // Create a 100x100 white image
    let pixels: Vec<u8> = (0..100 * 100).flat_map(|_| [255u8, 255, 255, 255]).collect();
    // Draw red in top-left 50x50
    let mut pixels = pixels;
    for y in 0..50u32 {
        for x in 0..50u32 {
            let i = ((y * 100 + x) * 4) as usize;
            pixels[i] = 255;     // R
            pixels[i + 1] = 0;   // G
            pixels[i + 2] = 0;   // B
        }
    }

    let ann = Annotation::Pixelate {
        top_left: Point::new(0.0, 0.0),
        size: Size::new(50.0, 50.0),
        block_size: 10,
    };

    let mut pixmap = tiny_skia::Pixmap::new(100, 100).unwrap();
    // Fill pixmap with the source pixels first
    for (i, chunk) in pixels.chunks_exact(4).enumerate() {
        if let Some(c) = tiny_skia::PremultipliedColorU8::from_rgba(chunk[0], chunk[1], chunk[2], chunk[3]) {
            pixmap.pixels_mut()[i] = c;
        }
    }

    render_annotation(&ann, &mut pixmap, Some(&pixels), Some(100));
    // The pixelated region should still have content (blocks of averaged color)
    let px = pixmap.pixel(5, 5).unwrap();
    assert!(px.alpha() > 0);
}
```

- [ ] **Step 2: Run tests — verify failure**

```bash
cd hydroshot && cargo test --test pixelate_tests
```

- [ ] **Step 3: Add Pixelate variant and implement**

In `hydroshot/src/tools/mod.rs`, add:

```rust
// In Annotation enum:
Pixelate {
    top_left: Point,
    size: Size,
    block_size: u8,
},

// In ToolKind:
Pixelate,
```

Add Pixelate match arm to `render_annotation`:
```rust
Annotation::Pixelate { top_left, size, block_size } => {
    let bs = *block_size as u32;
    if bs == 0 { return; }

    let (src_pixels, src_width) = match (screenshot_pixels, screenshot_width) {
        (Some(p), Some(w)) => (p, w),
        _ => return, // can't pixelate without source pixels
    };

    let x0 = top_left.x.max(0.0) as u32;
    let y0 = top_left.y.max(0.0) as u32;
    let x1 = (top_left.x + size.width).min(pixmap.width() as f32) as u32;
    let y1 = (top_left.y + size.height).min(pixmap.height() as f32) as u32;

    let mut block_y = y0;
    while block_y < y1 {
        let mut block_x = x0;
        let by_end = (block_y + bs).min(y1);
        while block_x < x1 {
            let bx_end = (block_x + bs).min(x1);
            // Average pixels in this block from source
            let mut r_sum: u32 = 0;
            let mut g_sum: u32 = 0;
            let mut b_sum: u32 = 0;
            let mut count: u32 = 0;

            for py in block_y..by_end {
                for px in block_x..bx_end {
                    let si = ((py * src_width + px) * 4) as usize;
                    if si + 3 < src_pixels.len() {
                        r_sum += src_pixels[si] as u32;
                        g_sum += src_pixels[si + 1] as u32;
                        b_sum += src_pixels[si + 2] as u32;
                        count += 1;
                    }
                }
            }

            if count > 0 {
                let avg_r = (r_sum / count) as u8;
                let avg_g = (g_sum / count) as u8;
                let avg_b = (b_sum / count) as u8;

                if let Some(rect) = tiny_skia::Rect::from_xywh(
                    block_x as f32, block_y as f32,
                    (bx_end - block_x) as f32, (by_end - block_y) as f32,
                ) {
                    let mut paint = Paint::default();
                    paint.set_color(tiny_skia::Color::from_rgba(
                        avg_r as f32 / 255.0, avg_g as f32 / 255.0,
                        avg_b as f32 / 255.0, 1.0
                    ).unwrap());
                    paint.anti_alias = false;
                    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                }
            }
            block_x += bs;
        }
        block_y += bs;
    }
}
```

Add `pub mod pixelate;`.

Also add the `Pixelate` arm to `offset_annotation` in `export.rs`:
```rust
Annotation::Pixelate { top_left, size, block_size } => Annotation::Pixelate {
    top_left: Point::new(top_left.x - dx, top_left.y - dy),
    size: *size, block_size: *block_size,
},
```

- [ ] **Step 4: Implement PixelateTool**

Create `hydroshot/src/tools/pixelate.rs`:

```rust
use crate::geometry::{Point, Size};
use super::{Annotation, AnnotationTool};

pub struct PixelateTool {
    start: Option<Point>,
    current: Option<Point>,
    block_size: u8,
}

impl PixelateTool {
    pub fn new() -> Self {
        Self { start: None, current: None, block_size: 10 }
    }

    fn normalize(a: Point, b: Point) -> (Point, Size) {
        (Point::new(a.x.min(b.x), a.y.min(b.y)),
         Size::new((a.x - b.x).abs(), (a.y - b.y).abs()))
    }
}

impl AnnotationTool for PixelateTool {
    fn on_mouse_down(&mut self, pos: Point) {
        self.start = Some(pos);
        self.current = Some(pos);
    }
    fn on_mouse_move(&mut self, pos: Point) {
        if self.start.is_some() { self.current = Some(pos); }
    }
    fn on_mouse_up(&mut self, pos: Point) -> Option<Annotation> {
        let start = self.start.take()?;
        self.current = None;
        let (tl, sz) = Self::normalize(start, pos);
        Some(Annotation::Pixelate { top_left: tl, size: sz, block_size: self.block_size })
    }
    fn is_drawing(&self) -> bool { self.start.is_some() }
    fn in_progress_annotation(&self) -> Option<Annotation> {
        let (tl, sz) = Self::normalize(self.start?, self.current?);
        Some(Annotation::Pixelate { top_left: tl, size: sz, block_size: self.block_size })
    }
}
```

- [ ] **Step 5: Run tests — verify pass**

```bash
cd hydroshot && cargo test --test pixelate_tests
```

- [ ] **Step 6: Commit**

```bash
git add src/tools/pixelate.rs src/tools/mod.rs tests/pixelate_tests.rs
git commit -m "feat: pixelate tool — block-average redaction"
```

---

## Task 5: Update Export for New Annotation Variants

**Files:**
- Modify: `hydroshot/src/export.rs`

- [ ] **Step 1: Extend offset_annotation for new variants**

Add match arms for Pencil, Text, and Pixelate in the `offset_annotation` function in `export.rs`:

```rust
Annotation::Pencil { points, color, thickness } => Annotation::Pencil {
    points: points.iter().map(|p| Point::new(p.x - dx, p.y - dy)).collect(),
    color: *color, thickness: *thickness,
},
Annotation::Text { position, text, color, font_size } => Annotation::Text {
    position: Point::new(position.x - dx, position.y - dy),
    text: text.clone(), color: *color, font_size: *font_size,
},
Annotation::Pixelate { top_left, size, block_size } => Annotation::Pixelate {
    top_left: Point::new(top_left.x - dx, top_left.y - dy),
    size: *size, block_size: *block_size,
},
```

- [ ] **Step 2: Update flatten_annotations to pass screenshot pixels**

Update the `flatten_annotations` call to `render_annotation` to pass the source pixels:

```rust
for ann in annotations {
    render_annotation(ann, &mut pixmap, Some(pixels), Some(width));
}
```

- [ ] **Step 3: Add save_directory parameter to save_to_file**

Add an optional default directory to `save_to_file`:

```rust
pub fn save_to_file(
    pixels: &[u8], width: u32, height: u32,
    default_dir: Option<&std::path::Path>,
) -> Result<Option<String>, String> {
    let default_name = chrono::Local::now().format("hydroshot_%Y-%m-%d_%H%M%S.png").to_string();

    if let Some(dir) = default_dir {
        // Auto-save without dialog
        let path = dir.join(&default_name);
        let img = image::RgbaImage::from_raw(width, height, pixels.to_vec())
            .ok_or("Invalid image data")?;
        img.save(&path).map_err(|e| e.to_string())?;
        return Ok(Some(path.to_string_lossy().to_string()));
    }

    // Show dialog (existing behavior)
    let path = rfd::FileDialog::new()
        .set_file_name(&default_name)
        .add_filter("PNG Image", &["png"])
        .save_file();
    // ... rest unchanged
}
```

- [ ] **Step 4: Update all callers of save_to_file**

In `main.rs`, pass `None` for the directory for now (Task 7 will update this to use the config):

```rust
export::save_to_file(&pixels, w, h, None)
```

- [ ] **Step 5: Verify build**

```bash
cd hydroshot && cargo check
```

- [ ] **Step 6: Run all tests**

```bash
cd hydroshot && cargo test
```

- [ ] **Step 7: Commit**

```bash
git add src/export.rs src/main.rs
git commit -m "feat: export handles new annotation variants and auto-save directory"
```

---

## Task 6: Global Hotkey

**Files:**
- Create: `hydroshot/src/hotkey.rs`
- Modify: `hydroshot/src/lib.rs`
- Modify: `hydroshot/src/main.rs`

- [ ] **Step 1: Implement hotkey.rs**

Create `hydroshot/src/hotkey.rs`:

```rust
use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};

pub fn register_hotkey(binding: &str) -> Result<(GlobalHotKeyManager, u32), String> {
    let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;

    let hotkey = parse_hotkey(binding)?;
    let id = hotkey.id();
    manager.register(hotkey).map_err(|e| format!("Failed to register hotkey '{}': {}", binding, e))?;

    Ok((manager, id))
}

fn parse_hotkey(binding: &str) -> Result<HotKey, String> {
    let parts: Vec<&str> = binding.split('+').map(|s| s.trim()).collect();
    let mut modifiers = Modifiers::empty();
    let mut key_code = None;

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            "super" | "meta" | "win" => modifiers |= Modifiers::SUPER,
            other => {
                key_code = Some(match other {
                    "a" => Code::KeyA, "b" => Code::KeyB, "c" => Code::KeyC,
                    "d" => Code::KeyD, "e" => Code::KeyE, "f" => Code::KeyF,
                    "g" => Code::KeyG, "h" => Code::KeyH, "i" => Code::KeyI,
                    "j" => Code::KeyJ, "k" => Code::KeyK, "l" => Code::KeyL,
                    "m" => Code::KeyM, "n" => Code::KeyN, "o" => Code::KeyO,
                    "p" => Code::KeyP, "q" => Code::KeyQ, "r" => Code::KeyR,
                    "s" => Code::KeyS, "t" => Code::KeyT, "u" => Code::KeyU,
                    "v" => Code::KeyV, "w" => Code::KeyW, "x" => Code::KeyX,
                    "y" => Code::KeyY, "z" => Code::KeyZ,
                    "printscreen" => Code::PrintScreen,
                    "f1" => Code::F1, "f2" => Code::F2, "f3" => Code::F3,
                    "f4" => Code::F4, "f5" => Code::F5, "f6" => Code::F6,
                    _ => return Err(format!("Unknown key: {}", other)),
                });
            }
        }
    }

    let code = key_code.ok_or("No key specified in binding")?;
    Ok(HotKey::new(Some(modifiers), code))
}
```

Add `pub mod hotkey;` to `lib.rs`.

- [ ] **Step 2: Wire hotkey into main.rs**

In `App` struct, add:
```rust
_hotkey_manager: Option<GlobalHotKeyManager>,
hotkey_id: Option<u32>,
```

In `resumed()`, register the hotkey:
```rust
match hotkey::register_hotkey(&self.config.hotkey.capture) {
    Ok((manager, id)) => {
        self._hotkey_manager = Some(manager);
        self.hotkey_id = Some(id);
        tracing::info!("Global hotkey registered: {}", self.config.hotkey.capture);
    }
    Err(e) => {
        tracing::warn!("Failed to register hotkey: {}", e);
    }
}
```

In `about_to_wait`, poll hotkey events:
```rust
if let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
    if Some(event.id()) == self.hotkey_id {
        if matches!(self.state, AppState::Idle) {
            self.trigger_capture(event_loop);
        }
    }
}
```

- [ ] **Step 3: Verify build**

```bash
cd hydroshot && cargo check
```

- [ ] **Step 4: Commit**

```bash
git add src/hotkey.rs src/lib.rs src/main.rs
git commit -m "feat: global hotkey — Ctrl+Shift+S to capture"
```

---

## Task 7: Update State, Renderer, Toolbar & Main Event Loop

This task wires the new tools into the overlay state, renderer, toolbar, and event loop.

**Files:**
- Modify: `hydroshot/src/state.rs`
- Modify: `hydroshot/src/overlay/toolbar.rs`
- Modify: `hydroshot/src/renderer.rs`
- Modify: `hydroshot/src/main.rs`

- [ ] **Step 1: Update OverlayState**

Add to `OverlayState`:
```rust
pub pencil_tool: PencilTool,
pub text_tool: TextTool,
pub pixelate_tool: PixelateTool,
pub text_input_active: bool,
pub text_input_buffer: String,
pub text_input_position: Point,
pub text_input_font_size: f32,
```

Update `OverlayState::new` to accept `&Config` and initialize:
```rust
pub fn new(screenshot: CapturedScreen, config: &Config) -> Self {
    let color = config.default_color();
    let thickness = config.clamped_thickness();
    Self {
        // ... existing fields using color/thickness ...
        pencil_tool: PencilTool::new(color, thickness),
        text_tool: TextTool::new(color, 16.0),
        pixelate_tool: PixelateTool::new(),
        text_input_active: false,
        text_input_buffer: String::new(),
        text_input_position: Point::new(0.0, 0.0),
        text_input_font_size: 16.0,
    }
}
```

- [ ] **Step 2: Update toolbar**

In `hydroshot/src/overlay/toolbar.rs`, change:
```rust
pub const BUTTON_COUNT: usize = 12;
```

- [ ] **Step 3: Update renderer**

In `hydroshot/src/renderer.rs`:

1. Update separator positions from `[1, 6]` to `[4, 9]`
2. Update button rendering loop for 12 buttons
3. Add new button icons:
   - Button 2 (Pencil): wavy line icon
   - Button 3 (Text): "T" letter icon
   - Button 4 (Pixelate): small grid icon
4. Update color swatch indices from `2..=6` to `5..=9`
5. Update copy/save indices from `7, 8` to `10, 11`
6. Add text preview rendering when `text_input_active`:
   - Render `text_input_buffer` at `text_input_position` using `render_annotation` with a temporary `Annotation::Text`
   - Draw a cursor (vertical white line) at the end of the text
7. Update all `render_annotation` calls to pass `Some(&state.screenshot.pixels)` and `Some(state.screenshot.width)` for pixelate support

- [ ] **Step 4: Update main.rs event loop**

1. Add `config: Config` to `App` struct, load in `main()`
2. Pass `&config` to `OverlayState::new()` in `trigger_capture()`
3. Add text input guard as FIRST check in keyboard handler:
```rust
// If text input active, route all keyboard to text buffer
if let AppState::Capturing(ref mut o) = self.state {
    if o.text_input_active {
        match key_event.logical_key {
            Key::Named(NamedKey::Enter) => {
                // Confirm: create annotation
                if !o.text_input_buffer.is_empty() {
                    o.annotations.push(Annotation::Text {
                        position: o.text_input_position,
                        text: o.text_input_buffer.clone(),
                        color: o.current_color,
                        font_size: o.text_input_font_size,
                    });
                    o.redo_buffer.clear();
                }
                o.text_input_active = false;
                o.text_input_buffer.clear();
            }
            Key::Named(NamedKey::Escape) => {
                o.text_input_active = false;
                o.text_input_buffer.clear();
            }
            Key::Named(NamedKey::Backspace) => {
                o.text_input_buffer.pop();
            }
            Key::Character(ref c) => {
                o.text_input_buffer.push_str(c.as_str());
            }
            _ => {}
        }
        self.needs_redraw = true;
        return; // Don't process other shortcuts
    }
}
```

4. Add tool keyboard shortcuts (after the text input guard):
```rust
Key::Character(ref c) if !modifiers.control_key() => {
    match c.as_str() {
        "a" => o.active_tool = ToolKind::Arrow,
        "r" => o.active_tool = ToolKind::Rectangle,
        "p" => o.active_tool = ToolKind::Pencil,
        "t" => o.active_tool = ToolKind::Text,
        "b" => o.active_tool = ToolKind::Pixelate,
        _ => {}
    }
}
```

5. Extend mouse down/move/up routing for new tools (Pencil, Text, Pixelate)
6. After text tool mouse_down, check `take_pending_position()`:
```rust
ToolKind::Text => {
    o.text_tool.on_mouse_down(pos);
    if let Some(p) = o.text_tool.take_pending_position() {
        o.text_input_active = true;
        o.text_input_position = p;
        o.text_input_buffer.clear();
        o.text_input_font_size = o.text_tool.font_size();
    }
}
```

7. Scroll wheel: when text_input_active, adjust font_size; otherwise adjust thickness

- [ ] **Step 5: Verify build**

```bash
cd hydroshot && cargo build
```

- [ ] **Step 6: Run all tests**

```bash
cd hydroshot && cargo test
```

- [ ] **Step 7: Commit**

```bash
git add src/state.rs src/overlay/toolbar.rs src/renderer.rs src/main.rs
git commit -m "feat: wire new tools, hotkey, and config into event loop"
```

---

## Task 8: Polish & Integration Test

**Files:**
- All (bug fixes)

- [ ] **Step 1: Run clippy and fmt**

```bash
cd hydroshot && cargo fmt && cargo clippy -- -D warnings
```

Fix any issues.

- [ ] **Step 2: Full manual test**

```bash
cd hydroshot && cargo run
```

Test:
1. Tray icon appears
2. Ctrl+Shift+S triggers capture (global hotkey)
3. Select region
4. Press P → pencil tool, draw freehand path
5. Press A → arrow tool, draw arrow
6. Press R → rectangle tool, draw rectangle
7. Press T → text tool, click to place, type text, Enter to confirm
8. Press B → pixelate tool, drag over sensitive area
9. Ctrl+Z undoes, Ctrl+Shift+Z redoes (for all tool types)
10. Color swatches change annotation color
11. Scroll wheel adjusts thickness (or font size when typing)
12. Ctrl+C copies to clipboard with all annotations
13. Ctrl+S saves to file
14. Esc cancels
15. Config file created at `%APPDATA%/hydroshot/config.toml`

- [ ] **Step 3: Fix bugs found during testing**

- [ ] **Step 4: Final test run**

```bash
cd hydroshot && cargo test && cargo clippy -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: HydroShot Phase 2A — pencil, text, pixelate, hotkey, settings"
```

---

## Task Dependencies

```
Task 0: Dependencies & Font
  └── Task 1: Config module
       └── Task 2: Pencil tool (changes render_annotation signature — must be first)
            └── Task 3: Text tool (adds text_tool + text input state to OverlayState)
                 └── Task 4: Pixelate tool
                      └── Task 5: Update export save_to_file + verify all offset arms
                           └── Task 6: Global hotkey (independent but sequential for simplicity)
                                └── Task 7: Wire everything (state, renderer, toolbar, main)
                                     └── Task 8: Polish & integration test
```

**Tasks 2, 3, 4 are sequential** (not parallel) because each adds a variant to the `Annotation` enum, which requires updating the exhaustive `match` in `offset_annotation` and `render_annotation` immediately. Task 2 also changes the `render_annotation` signature, which all subsequent tasks depend on.

**Task 7 notes:**
- `pencil_tool` and `pixelate_tool` are added to `OverlayState` in Task 7 (not in Tasks 2/4)
- `text_tool` and text input fields are already on `OverlayState` from Task 3
- `OverlayState::new` signature change to accept `&Config` happens in Task 7; the `trigger_capture` caller is updated in the same step
- `save_to_file` caller passes `None` for default_dir until Task 7 adds `config` to `App`, then updated to `config.save_directory().as_deref()`
- Color/thickness changes in toolbar handler must propagate to `pencil_tool.set_color()`, `text_tool.set_color()`, and `pencil_tool.set_thickness()` in addition to existing arrow/rectangle tools
- Mouse routing for Pencil and Pixelate follows the exact same pattern as Arrow/Rectangle: call `on_mouse_down`/`on_mouse_move`/`on_mouse_up` on the respective tool in the existing `match overlay.active_tool` blocks (3 locations in main.rs)
- `global_hotkey::GlobalHotKeyEvent` field access is `event.id` (field, not method)
