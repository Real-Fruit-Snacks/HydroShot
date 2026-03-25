# HydroShot Phase 2A — Design Specification

**Date**: 2026-03-24
**Status**: Draft
**Builds on**: `docs/superpowers/specs/2026-03-24-hydroshot-design.md` (MVP)

---

## 1. Overview

Phase 2A adds five features to make HydroShot a viable daily-driver screenshot tool:

1. **Pencil tool** — freehand drawing
2. **Text tool** — single-line text annotations
3. **Pixelate tool** — block-average pixelation for redacting sensitive content
4. **Global hotkey** — Ctrl+Shift+S to trigger capture without clicking the tray
5. **Settings persistence** — TOML config file for default color, thickness, save directory, hotkey

### Out of Scope

- Multi-line text / word wrap
- Gaussian blur
- Settings UI (edit config file manually)
- Cursor feedback (crosshair, resize arrows)
- Selection dimensions display
- Annotation re-selection / move after placement

---

## 2. New Annotation Tools

All three tools follow the existing `AnnotationTool` trait pattern from the MVP.

### 2.1 Pencil Tool

**Behavior:** Click and drag to draw freehand. Mouse movement records points into a path. On release, the path is finalized as an annotation.

**Data model:**
```rust
Annotation::Pencil {
    points: Vec<Point>,
    color: Color,
    thickness: f32,
}
```

**Rendering:** Connected line segments via `tiny_skia::PathBuilder` — `move_to` first point, `line_to` each subsequent point. Stroked with the annotation's color and thickness. Anti-aliased.

**In-progress preview:** Same as finalized rendering — draw the accumulated points so far.

**File:** `src/tools/pencil.rs`

### 2.2 Text Tool

**Behavior:** Click to place a text cursor. Type characters (printable ASCII/Unicode). Scroll wheel adjusts font size. Enter confirms the text as a finalized annotation. Esc cancels text entry (discards typed text, returns to previous tool).

**Text input state** (on `OverlayState`, not the tool itself):
- `text_input_active: bool` — whether we're in text editing mode
- `text_input_buffer: String` — the text being typed
- `text_input_position: Point` — where the text will be placed
- `text_input_font_size: f32` — current font size (default from config, adjustable 10-72px)

**Data model:**
```rust
Annotation::Text {
    position: Point,
    text: String,
    color: Color,
    font_size: f32,
}
```

**Rendering:** Use the `fontdue` crate to rasterize glyphs. Load a bundled monospace font (e.g., embedded TTF via `include_bytes!`). For each character:
1. Rasterize the glyph at the annotation's font_size
2. Write the glyph's alpha coverage into the pixmap, tinted with the annotation's color
3. Advance the x position by the glyph's advance width

**In-progress preview:** Render the current `text_input_buffer` at `text_input_position` with a blinking cursor (or static cursor — simpler). The cursor is a vertical line at the end of the text.

**Keyboard routing:** When `text_input_active` is true, keyboard input is handled as follows:
- **Printable characters** → append to `text_input_buffer`
- **Backspace** → delete last character from buffer
- **Enter** → confirm: create `Annotation::Text` from buffer, clear buffer, set `text_input_active = false`
- **Esc** → cancel: discard buffer, set `text_input_active = false`, return to previous tool
- **All other keys (including Ctrl+C, Ctrl+S, Ctrl+Z)** → ignored while text input is active. This guard must be the **very first check** in the keyboard handler, before any shortcut processing.

**Scroll wheel while text input active:** Adjusts `text_input_font_size` (range 10-72px). When text input is NOT active but Text tool is selected, scroll wheel does nothing (thickness is irrelevant for text).

**TextTool and the AnnotationTool trait:** The `TextTool` struct implements `AnnotationTool` but only uses `on_mouse_down` — which records the click position and signals the event loop to activate text input mode. `on_mouse_move` and `on_mouse_up` are no-ops. `in_progress_annotation()` returns `None` (the text preview is rendered separately by the renderer using `OverlayState`'s text input fields, not through the trait). `on_mouse_down` returns a signal value — it sets internal state that the event loop checks to know it should activate text input mode.

```rust
pub struct TextTool {
    color: Color,
    font_size: f32,
    pending_position: Option<Point>,  // set by on_mouse_down, consumed by event loop
}

impl TextTool {
    pub fn take_pending_position(&mut self) -> Option<Point> {
        self.pending_position.take()
    }
}
```

The event loop, after calling `on_mouse_down` on the text tool, checks `take_pending_position()`. If `Some(pos)`, it sets `text_input_active = true`, `text_input_position = pos`, `text_input_buffer = String::new()` on `OverlayState`.

**File:** `src/tools/text.rs`

**Asset:** A bundled TTF font file at `assets/font.ttf` (e.g., JetBrains Mono, DejaVu Sans Mono, or any open-source monospace font). Embedded via `include_bytes!`.

### 2.3 Pixelate Tool

**Behavior:** Click and drag a rectangle over the area to pixelate. On release, the region is finalized. The pixelation effect reads from the **original screenshot pixels** (not the annotation layer), so it always pixelates the underlying content regardless of other annotations.

**Data model:**
```rust
Annotation::Pixelate {
    top_left: Point,
    size: Size,
    block_size: u8,
}
```

**Default block_size:** 10 pixels. Not user-adjustable in Phase 2A (future: scroll wheel).

**Rendering:** For the rectangle defined by `top_left` and `size`:
1. Divide into blocks of `block_size x block_size` pixels
2. For each block, read the screenshot's RGBA pixels within that block
3. Average the R, G, B, A values across all pixels in the block
4. Fill the entire block with the averaged color

**Important:** The renderer needs access to the original screenshot pixels, not just the pixmap being drawn on. The `render_overlay` function already receives `&OverlayState` which contains `screenshot`, so this is accessible.

**In-progress preview:** Show the pixelation effect in real-time as the user drags.

**Coordinate normalization:** Same as RectangleTool — handle reverse drag (bottom-right to top-left).

**File:** `src/tools/pixelate.rs`

### 2.4 Tool Integration

**ToolKind extension:**
```rust
pub enum ToolKind {
    Arrow,
    Rectangle,
    Pencil,    // new
    Text,      // new
    Pixelate,  // new
}
```

**Keyboard shortcuts (when text input is NOT active):**
- A = Arrow
- R = Rectangle
- P = Pencil
- T = Text
- B = Pixelate (B for "blur" — intuitive mnemonic)

**Toolbar layout (12 buttons):**
```
[Arrow] [Rect] [Pencil] [Text] [Pixelate] | [Red] [Blue] [Green] [Yellow] [White] | [Copy] [Save]
```

Button indices: 0=Arrow, 1=Rectangle, 2=Pencil, 3=Text, 4=Pixelate, 5-9=colors, 10=Copy, 11=Save

**Toolbar constants update:** `BUTTON_COUNT` changes from 9 to 12.

**Toolbar separator positions:** The renderer currently draws separators after button indices `[1, 6]`. These must be updated to `[4, 9]` to match the new layout (separator after Pixelate, separator after White).

**Keyboard shortcuts are all new:** The MVP has no keyboard shortcuts for tool switching — only toolbar clicks. A, R, P, T, B are all additions in Phase 2A.

**render_annotation() extension:** Add match arms for `Pencil`, `Text`, and `Pixelate` in the shared `render_annotation()` function in `tools/mod.rs`. The `Pixelate` variant needs the screenshot pixels passed in — extend the function signature:

```rust
pub fn render_annotation(
    annotation: &Annotation,
    pixmap: &mut tiny_skia::Pixmap,
    screenshot_pixels: Option<&[u8]>,  // needed for Pixelate
    screenshot_width: Option<u32>,
)
```

For non-Pixelate annotations, `screenshot_pixels` is ignored. Callers pass `Some(...)` when rendering the overlay (screenshot is available) and `Some(...)` during export (cropped pixels are available, with offset coordinates).

**Export handling for Pixelate:** During export in `flatten_annotations`, the `screenshot_pixels` parameter receives the **cropped** pixel buffer (pre-annotation, straight RGBA). The Pixelate annotation's coordinates are already selection-relative after `offset_annotation` has been applied. So the pixelation reads from the cropped buffer at the offset coordinates — no special handling needed beyond passing the cropped buffer through.

**`offset_annotation` must be extended** for all three new variants in `export.rs`:
- `Pencil` — offset every `Point` in the `points` vec by `(-sel_x, -sel_y)`
- `Text` — offset the `position` point
- `Pixelate` — offset the `top_left` point

This is required because `offset_annotation` contains an exhaustive `match` on `Annotation` — adding new variants without match arms will be a compile error.

---

## 3. Global Hotkey

### 3.1 Crate

Use `global-hotkey` crate (by the same author as `tray-icon` and `muda`). Designed to work with winit's event loop.

### 3.2 Registration

On startup, after creating the tray icon:
1. Parse the hotkey string from config (default: `"Ctrl+Shift+S"`)
2. Create a `GlobalHotKeyManager`
3. Register the hotkey
4. If registration fails (key combo already taken by another app), log a warning via `tracing::warn!` and continue — the tray icon still works

The `GlobalHotKeyManager` must be kept alive for the duration of the app.

### 3.3 Event Handling

In `about_to_wait`, poll `GlobalHotKeyEvent::receiver().try_recv()`. When the registered hotkey fires, trigger capture the same way as a tray click — call the same `trigger_capture()` function.

### 3.4 File

`src/hotkey.rs`:
```rust
pub fn register_hotkey(binding: &str) -> Result<global_hotkey::GlobalHotKeyManager, String> {
    // Parse binding string into modifiers + key code
    // Register with GlobalHotKeyManager
    // Return manager (caller keeps it alive)
}
```

---

## 4. Settings Persistence

### 4.1 Config File Location

- **Windows:** `%APPDATA%/hydroshot/config.toml`
- **Linux:** `~/.config/hydroshot/config.toml`

Use the `dirs` crate for `dirs::config_dir()`.

### 4.2 Format

```toml
[general]
default_color = "red"
default_thickness = 3.0
save_directory = ""

[hotkey]
capture = "Ctrl+Shift+S"
```

### 4.3 Data Model

```rust
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub hotkey: HotkeyConfig,
}

#[derive(Serialize, Deserialize)]
pub struct GeneralConfig {
    pub default_color: String,       // "red", "blue", "green", "yellow", "white"
    pub default_thickness: f32,      // 1.0 - 20.0
    pub save_directory: String,      // empty = use file dialog
}

#[derive(Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub capture: String,             // e.g. "Ctrl+Shift+S"
}
```

### 4.4 Behavior

- **First run:** If config file doesn't exist, create it with defaults
- **Load:** Parse TOML, fall back to defaults for any missing/invalid fields
- **Color parsing:** Map string names to `Color` values. Invalid names fall back to red.
- **Save directory:** If non-empty and the directory exists, `save_to_file` auto-saves there with the timestamped filename (no file dialog). If empty, show the file dialog as before.

### 4.5 File

`src/config.rs`:
```rust
impl Config {
    pub fn load() -> Self { /* load from file or create defaults */ }
    pub fn default_color(&self) -> Color { /* parse string to Color */ }
    pub fn config_path() -> PathBuf { /* dirs::config_dir() + "hydroshot/config.toml" */ }
}
```

### 4.6 Integration Points

- `main.rs` — `App` struct gets a `config: Config` field, loaded at startup in `main()` before `run_app`. Passed to `OverlayState::new()` in `trigger_capture()` and to `register_hotkey()` in `resumed()`
- `state.rs` — `OverlayState::new(screenshot, &config)` uses config for initial color/thickness/font_size
- `export.rs` — `save_to_file` accepts `Option<&Path>` for auto-save directory; `offset_annotation` extended with Pencil/Text/Pixelate arms

---

## 5. Crate Dependencies (New)

| Purpose | Crate | Notes |
|---------|-------|-------|
| Global hotkey | `global-hotkey` | Same author as tray-icon |
| Font rasterization | `fontdue` | Lightweight, no system deps |
| Config serialization | `toml` | TOML parsing/writing |
| Serde | `serde = { features = ["derive"] }` | Serialize/Deserialize derive macros |
| Config directory | `dirs` | Platform-standard paths |

---

## 6. Modified Files Summary

| File | Changes |
|------|---------|
| `Cargo.toml` | Add 5 new dependencies |
| `src/tools/mod.rs` | 3 new Annotation variants, 3 new ToolKind variants, extend render_annotation() signature |
| `src/tools/pencil.rs` | New: PencilTool |
| `src/tools/text.rs` | New: TextTool + font loading/rasterization |
| `src/tools/pixelate.rs` | New: PixelateTool |
| `src/overlay/toolbar.rs` | BUTTON_COUNT 9→12 |
| `src/renderer.rs` | New toolbar button icons (pencil, T, grid), text preview with cursor, pixelate preview, update render_annotation calls |
| `src/state.rs` | 3 new tool fields, text input state fields, OverlayState::new takes &Config |
| `src/config.rs` | New: Config loading/saving/defaults |
| `src/hotkey.rs` | New: Global hotkey registration |
| `src/main.rs` | Load config, register hotkey, poll hotkey events, keyboard tool shortcuts, text input routing |
| `src/export.rs` | save_to_file accepts optional default directory |
| `src/lib.rs` | Add pub mod config, hotkey |
| `assets/font.ttf` | Bundled monospace font |

---

## 7. Project Structure (Phase 2A additions)

```
hydroshot/
├── assets/
│   ├── icon.png
│   └── font.ttf              # NEW: bundled monospace font
├── src/
│   ├── config.rs              # NEW: settings persistence
│   ├── hotkey.rs              # NEW: global hotkey registration
│   ├── tools/
│   │   ├── pencil.rs          # NEW: freehand drawing
│   │   ├── text.rs            # NEW: text annotation + font rendering
│   │   └── pixelate.rs        # NEW: block-average pixelation
│   └── (existing files modified as noted above)
```
