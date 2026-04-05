<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/Real-Fruit-Snacks/HydroShot/main/docs/assets/logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/Real-Fruit-Snacks/HydroShot/main/docs/assets/logo-light.svg">
  <img alt="HydroShot" src="https://raw.githubusercontent.com/Real-Fruit-Snacks/HydroShot/main/docs/assets/logo-dark.svg" width="520">
</picture>

![Rust](https://img.shields.io/badge/Rust-orange?style=flat&logo=rust&logoColor=white)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

**Fast, lightweight screenshot capture and annotation tool built with Rust**

Region selection, window capture, delay timer, 14 annotation tools, clipboard and file export,
pin-to-screen, Imgur upload, OCR text extraction, and recent captures history. Catppuccin Mocha
themed with customizable shortcuts and toolbar.

</div>

---

## Quick Start

### Pre-built Binaries

Download the latest release from the [Releases](https://github.com/Real-Fruit-Snacks/HydroShot/releases) page:

| Platform | Format |
|----------|--------|
| Windows  | `.exe` portable / `.msi` installer |
| Linux    | `hydroshot-linux` binary |

### Build from Source

Prerequisites: Rust 1.80+.

```bash
git clone https://github.com/Real-Fruit-Snacks/HydroShot.git
cd HydroShot

cargo build --release
# Binary: target/release/hydroshot(.exe)
```

### Verify

```bash
cargo test              # run tests
cargo clippy            # lint
cargo fmt --check       # format check
```

---

## Features

### Region and Window Capture

Click and drag to capture any screen area. Highlight and click to capture a specific window. Timed captures with 3s, 5s, or 10s countdown. Multi-monitor support across all connected displays with a fullscreen overlay that dims inactive areas.

```bash
# CLI usage
hydroshot capture --clipboard          # capture and copy
hydroshot capture --save output.png    # capture and save
hydroshot capture --delay 3            # 3-second delay
hydroshot capture --delay 5 --clipboard
```

### 14 Annotation Tools

Full annotation toolkit with keyboard shortcuts, scroll-wheel sizing, and command-pattern undo/redo:

```
V  Select/Move       A  Arrow           R  Rectangle
C  Circle            O  Rounded Rect    L  Line
P  Pencil            H  Highlight       F  Spotlight
T  Text              B  Pixelate        N  Step Markers
I  Eyedropper        M  Measurement
```

All tool shortcuts are customizable through Settings > Shortcuts.

### Export and Sharing

Multiple export paths for captured screenshots:

```
Ctrl+C    →  copy to clipboard
Ctrl+S    →  save to file
Enter     →  quick crop selection
Pin       →  always-on-top floating window
Upload    →  anonymous Imgur upload
OCR       →  extract text (Windows OCR API)
```

Recent captures history with thumbnails for quick access to previous screenshots.

### System Tray Integration

HydroShot lives in the system tray. Left-click the tray icon or press `Ctrl+Shift+S` from anywhere to start a capture. Pin captured screenshots as floating always-on-top reference windows. Optional auto-start on login.

### Catppuccin Mocha Theme

Consistent dark theme with 5 color presets from the Catppuccin palette. Native color picker available via right-click on any color swatch. Lucide SVG icons rendered with resvg throughout the interface.

```
Color presets:  5 Catppuccin palette colors
Custom colors:  native OS color picker via right-click
Icons:          Lucide SVG rendered with resvg
```

### Configuration

TOML-based configuration with persistent preferences. Tabbed settings UI for General, Shortcuts, and Toolbar customization.

```toml
# Windows: %APPDATA%\hydroshot\config.toml
# Linux:   ~/.config/hydroshot/config.toml

[general]
default_color = "blue"
default_thickness = 3.0
save_directory = ""

[hotkey]
capture = "Ctrl+Shift+S"

[shortcuts]
arrow = "a"
rectangle = "r"
circle = "c"
text = "t"
pixelate = "b"
```

### Keyboard Shortcuts

```
Ctrl+Shift+S   →  start capture (global hotkey)
Enter          →  crop selection
Ctrl+C         →  copy to clipboard
Ctrl+S         →  save to file
Ctrl+Z         →  undo annotation
Ctrl+Shift+Z   →  redo annotation
Escape         →  cancel capture
Scroll Wheel   →  adjust tool size
```

### OCR Text Extraction

Extract text from screenshots using the Windows OCR API. Select a region and use the OCR tool to copy recognized text to the clipboard. Available on Windows only.

---

## Architecture

```
HydroShot/
├── src/
│   ├── main.rs              # Entry point, event loop
│   ├── cli.rs               # CLI argument parsing (clap)
│   ├── tray.rs              # System tray integration
│   ├── hotkey.rs            # Global hotkey registration
│   ├── config.rs            # TOML configuration
│   ├── renderer.rs          # Core rendering pipeline
│   ├── export.rs            # Clipboard and file export
│   ├── upload.rs            # Imgur anonymous upload
│   ├── ocr.rs               # OCR text extraction
│   ├── capture/             # Screen capture backends
│   │   ├── windows.rs       # Windows capture
│   │   ├── x11.rs           # X11 capture
│   │   └── wayland.rs       # Wayland capture
│   ├── overlay/             # Overlay window and selection
│   └── tools/               # 14 annotation tool implementations
├── assets/                  # Static assets
├── tests/                   # Integration tests
└── docs/                    # GitHub Pages site
```

Pure Rust implementation using winit for windowing, tiny-skia for 2D rendering, and platform-specific capture backends. No Electron, no browser engine -- just native graphics.

---

## Platform Support

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

## License

[MIT](LICENSE) — Copyright 2026 Real-Fruit-Snacks
