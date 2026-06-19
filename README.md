<div align="center">

  # HydroShot

  **Fast, lightweight screenshot capture and annotation tool built with Rust. Region selection, window capture, delay timer, 14 annotation tools, clipboard and file export, pin-to-screen, Imgur upload, OCR text extraction, and recent captures history. Catppuccin Mocha themed with customizable shortcuts and toolbar.**

  [![License: MIT](https://img.shields.io/badge/License-MIT-cba6f7.svg)](LICENSE)
  [![Version](https://img.shields.io/badge/version-1.0.0-89b4fa)](https://github.com/Real-Fruit-Snacks/HydroShot/releases)
  
  [Website](https://real-fruit-snacks.github.io/HydroShot/) • [Report Issue](https://github.com/Real-Fruit-Snacks/HydroShot/issues)

</div>

---

## Overview

HydroShot is a **native screenshot tool** in pure Rust. Region capture by drag, window capture by click, timed capture with 3/5/10-second countdown, multi-monitor coverage with a fullscreen overlay that dims inactive areas. Captured shots flow into an annotation surface with 14 tools backed by command-pattern undo/redo. Pin captures as always-on-top reference windows; copy to clipboard, save to file, upload anonymously to Imgur, or extract text via the Windows OCR API.

No Electron, no browser engine — just `winit` for windowing and `tiny-skia` for 2D drawing. Configuration lives in TOML. Tray-resident with a global hotkey (`Ctrl+Shift+S` by default).

---

## Key Features

- **Native Binary:** Pure Rust utilizing `winit` for cross-platform native windowing and `tiny-skia` for lightning-fast 2D rendering. No web views.
- **Versatile Capture Modes:** Region selection, full window capture, multi-monitor support, and delayed capture with 3/5/10-second intervals.
- **14 Annotation Tools:** Select, arrow, rectangle, circle, rounded rect, line, pencil, highlight, spotlight, text, pixelate, step markers, eyedropper, and measurement tools.
- **Advanced Exporting:** Save directly to file, copy to clipboard, upload anonymously to Imgur, pin captures to screen, or extract text via Windows OCR.
- **Premium Aesthetics:** Fully themed with Catppuccin Mocha colors, including 5 palette presets, a native color picker, and Lucide icons.
- **System Tray Integration:** Runs in the background with auto-start capabilities (Registry/XDG) and a global hotkey trigger.
- **Command-Pattern Undo/Redo:** Non-destructive editing allows rolling back and re-applying annotations endlessly.
- **Cross-Platform:** Available for Windows (GDI) and Linux (X11 experimental).

---

## Getting Started / Installation

### Pre-built Binaries

Download the latest release from the [Releases](https://github.com/Real-Fruit-Snacks/HydroShot/releases) page:

- **Windows:** `.exe` portable or `.msi` installer
- **Linux:** `hydroshot-linux` binary

### Build From Source

**Prerequisites:** Rust 1.80+.

```bash
git clone https://github.com/Real-Fruit-Snacks/HydroShot.git
cd HydroShot

# Build the release binary
cargo build --release

# The compiled binary will be located at:
# -> target/release/hydroshot(.exe)
```

**Development Commands:**
```bash
cargo test                    # run tests
cargo clippy                  # lint
cargo fmt --check             # format check
```

---

## Usage

### CLI Usage

```bash
hydroshot capture --clipboard           # capture and copy
hydroshot capture --save output.png     # capture and save
hydroshot capture --delay 3             # wait 3 seconds, then open interactive capture
hydroshot capture --delay 5 --clipboard # wait, then capture straight to clipboard
```
*(The on-screen countdown window is shown for tray-menu delays; CLI `--delay` waits silently.)*

### Keyboard Shortcuts

- `Ctrl+Shift+S`: Start capture (global hotkey)
- `Enter`: Copy selection to clipboard (annotations included)
- `Ctrl+C`: Copy to clipboard
- `Ctrl+S`: Save to file
- `Ctrl+Z`: Undo annotation
- `Ctrl+Shift+Z`: Redo annotation
- `Ctrl+V`: Paste clipboard text (while typing a Text annotation)
- `Escape`: Cancel capture
- `Scroll wheel`: Adjust tool size

**Tool Shortcuts:**
`V` (Select), `A` (Arrow), `R` (Rectangle), `C` (Circle), `O` (Rounded Rect), `L` (Line), `P` (Pencil), `H` (Highlight), `F` (Spotlight), `T` (Text), `B` (Pixelate), `N` (Step Markers), `I` (Eyedropper), `M` (Measurement).

### Configuration (TOML)

HydroShot generates a config file at `%APPDATA%\hydroshot\config.toml` (Windows) or `~/.config/hydroshot/config.toml` (Linux).

```toml
[general]
default_color = "blue"        # named Catppuccin color or "#rrggbb"
default_thickness = 3.0
save_directory = ""
history_enabled = true        # recent-captures history (toggle in Settings)

[hotkey]
capture = "Ctrl+Shift+S"      # rebind in Settings > General
```

---

## Architecture / File Structure

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

**Key patterns:** No browser engine, no Electron, no JavaScript runtime. The annotation surface is a `tiny-skia` canvas; every tool emits commands that the undo/redo stack composes. Capture backends are split per-platform so the rest of the codebase stays platform-agnostic.

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to help improve the project. Be sure to also review our [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

---

## License

This project is licensed under the [MIT License](LICENSE).
