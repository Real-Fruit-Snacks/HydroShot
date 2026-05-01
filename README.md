<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/Real-Fruit-Snacks/HydroShot/main/docs/assets/logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/Real-Fruit-Snacks/HydroShot/main/docs/assets/logo-light.svg">
  <img alt="HydroShot" src="https://raw.githubusercontent.com/Real-Fruit-Snacks/HydroShot/main/docs/assets/logo-dark.svg" width="100%">
</picture>

> [!IMPORTANT]
> **Fast, lightweight screenshot capture and annotation tool built with Rust.** Region selection, window capture, delay timer, 14 annotation tools, clipboard and file export, pin-to-screen, Imgur upload, OCR text extraction, and recent captures history. Catppuccin Mocha themed with customizable shortcuts and toolbar.

> *No Electron, no browser engine — just `winit` for windowing and `tiny-skia` for 2D drawing. Felt fitting for a tool that should appear, capture, and disappear without bringing a browser along.*

---

## §1 / Premise

HydroShot is a **native screenshot tool** in pure Rust. Region capture by drag, window capture by click, timed capture with 3/5/10-second countdown, multi-monitor coverage with a fullscreen overlay that dims inactive areas. Captured shots flow into an annotation surface with 14 tools backed by command-pattern undo/redo. Pin captures as always-on-top reference windows; copy to clipboard, save to file, upload anonymously to Imgur, or extract text via the Windows OCR API.

Configuration lives in TOML. Tray-resident with a global hotkey (`Ctrl+Shift+S` by default).

---

## §2 / Specs

| KEY        | VALUE                                                                       |
|------------|-----------------------------------------------------------------------------|
| BINARY     | **Pure Rust** · winit (windowing) · tiny-skia (2D) · resvg (icons)          |
| CAPTURE    | Region · window · delay (3/5/10 s) · multi-monitor                          |
| BACKENDS   | **Windows** · **X11** · **Wayland** (window capture: X11 only)              |
| TOOLS      | **14 annotation tools** · select · arrow · rect · circle · rounded rect · line · pencil · highlight · spotlight · text · pixelate · step markers · eyedropper · measurement |
| EXPORT     | Clipboard · file · pin-to-screen · anonymous Imgur upload · OCR (Windows)   |
| THEME      | **Catppuccin Mocha** · 5 palette presets · native color picker · Lucide icons |
| CONFIG     | TOML · `%APPDATA%\hydroshot\config.toml` · `~/.config/hydroshot/config.toml` |
| HOTKEY     | Global · `Ctrl+Shift+S` default · customizable                              |
| STACK      | **Rust 1.80+** · Cargo · MIT licensed                                       |

Architecture in §5 below.

---

## §3 / Quickstart

### Pre-built binaries

Download the latest release from the [Releases](https://github.com/Real-Fruit-Snacks/HydroShot/releases) page:

| Platform | Format |
|----------|--------|
| Windows  | `.exe` portable · `.msi` installer |
| Linux    | `hydroshot-linux` binary |

### Build from source

Prerequisites: **Rust 1.80+**.

```bash
git clone https://github.com/Real-Fruit-Snacks/HydroShot.git
cd HydroShot
cargo build --release
# → target/release/hydroshot(.exe)
```

```bash
cargo test                    # run tests
cargo clippy                  # lint
cargo fmt --check             # format check
```

CLI usage:

```bash
hydroshot capture --clipboard           # capture and copy
hydroshot capture --save output.png     # capture and save
hydroshot capture --delay 3             # 3-second countdown
hydroshot capture --delay 5 --clipboard
```

---

## §4 / Reference

```
ANNOTATION TOOLS

  V   Select / Move        A   Arrow              R   Rectangle
  C   Circle               O   Rounded Rect       L   Line
  P   Pencil               H   Highlight          F   Spotlight
  T   Text                 B   Pixelate           N   Step Markers
  I   Eyedropper           M   Measurement

KEYBOARD

  Ctrl+Shift+S            Start capture (global hotkey)
  Enter                   Crop selection
  Ctrl+C                  Copy to clipboard
  Ctrl+S                  Save to file
  Ctrl+Z                  Undo annotation
  Ctrl+Shift+Z            Redo annotation
  Escape                  Cancel capture
  Scroll wheel            Adjust tool size

CONFIG (TOML)

  [general]
  default_color = "blue"
  default_thickness = 3.0
  save_directory = ""

  [hotkey]
  capture = "Ctrl+Shift+S"

  [shortcuts]
  arrow = "a"     rectangle = "r"     circle = "c"
  text  = "t"     pixelate  = "b"

EXPORT PATHS

  Ctrl+C     Copy to clipboard
  Ctrl+S     Save to file
  Pin        Always-on-top floating window
  Upload     Anonymous Imgur upload
  OCR        Extract text (Windows OCR API)
  History    Recent captures with thumbnails
```

---

## §5 / Architecture

```
src/
  main.rs             Entry point · event loop
  cli.rs              CLI argument parsing (clap)
  tray.rs             System tray integration
  hotkey.rs           Global hotkey registration
  config.rs           TOML configuration
  renderer.rs         Core rendering pipeline
  export.rs           Clipboard and file export
  upload.rs           Imgur anonymous upload
  ocr.rs              OCR text extraction (Windows)
  capture/
    windows.rs        Windows capture
    x11.rs            X11 capture
    wayland.rs        Wayland capture
  overlay/            Overlay window and selection
  tools/              14 annotation tool implementations
```

| Layer        | Implementation                                                  |
|--------------|-----------------------------------------------------------------|
| **Window**   | winit · cross-platform native windowing                         |
| **Render**   | tiny-skia · 2D vector + bitmap rendering                        |
| **Icons**    | Lucide SVG rendered with resvg                                  |
| **Capture**  | Per-platform backend (Windows / X11 / Wayland)                  |
| **Overlay**  | Fullscreen dimming overlay · click-and-drag selection           |
| **Tools**    | Command-pattern undo/redo · scroll-wheel sizing                 |
| **Tray**     | Native system tray · auto-start on login (Registry / XDG)       |

**Key patterns:** No browser engine, no Electron, no JavaScript runtime. The annotation surface is a `tiny-skia` canvas; every tool emits commands that the undo/redo stack composes. Capture backends are split per-platform so the rest of the codebase stays platform-agnostic.

---

## §6 / Platform support

| Capability | Windows | Linux |
|------------|---------|-------|
| Region Capture | Full | Full (X11 + Wayland) |
| Window Capture | Full | X11 only |
| Delay Capture | Full | Full |
| Multi-Monitor | Full | Full |
| 14 Annotation Tools | Full | Full |
| Clipboard Copy | Full | Full |
| System Tray | Full | Full |
| Global Hotkey | Full | Full |
| OCR | Windows OCR API | Not supported |
| Imgur Upload | Full | Full |
| Auto-Start | Registry | XDG autostart |

---

[License: MIT](LICENSE) · Part of [Real-Fruit-Snacks](https://github.com/Real-Fruit-Snacks) — building offensive security tools, one wave at a time.
