<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="docs/assets/logo-light.svg">
  <img alt="HydroShot" src="docs/assets/logo-dark.svg" width="120" height="120">
</picture>

<br><br>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/banner.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/banner.svg">
  <img alt="HydroShot" src="assets/banner.svg" width="800">
</picture>

<br>

**Fast, lightweight screenshot capture and annotation tool -- built with Rust, winit, and tiny-skia.**

[![Rust](https://img.shields.io/badge/Rust-1.80+-DEA584?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/Platform-Windows-0078D6?style=flat-square&logo=windows&logoColor=white)](.)
[![License](https://img.shields.io/badge/License-MIT-a6e3a1?style=flat-square)](LICENSE)
[![Release](https://img.shields.io/github/v/release/Real-Fruit-Snacks/HydroShot?style=flat-square&color=cba6f7&label=Latest)](https://github.com/Real-Fruit-Snacks/HydroShot/releases/latest)

<p>
  <a href="#features">Features</a> &bull;
  <a href="#installation">Installation</a> &bull;
  <a href="#usage">Usage</a> &bull;
  <a href="#configuration">Configuration</a> &bull;
  <a href="#development">Development</a> &bull;
  <a href="#contributing">Contributing</a> &bull;
  <a href="#license">License</a>
</p>

</div>

---

## Features

### Capture

- **Region selection** -- Click and drag to capture any area of your screen. Resize and reposition selections before committing.
- **Window capture** -- Highlight and click any window to capture it instantly.
- **Delay capture** -- Timed captures (3s, 5s, 10s) with a visible countdown overlay, giving you time to set up the shot.
- **Multi-monitor support** -- Captures the entire virtual desktop across all connected monitors.
- **Fullscreen overlay** -- Semi-transparent overlay dims inactive areas for precise region selection.

### Annotation Tools (14 tools)

| Shortcut | Tool | Description |
|----------|------|-------------|
| `V` | **Select / Move** | Reposition and resize annotations after placement |
| `A` | **Arrow** | Draw directional arrows to highlight points of interest |
| `R` | **Rectangle** | Draw outlined or filled rectangles for emphasis |
| `C` | **Circle** | Draw outlined or filled circles and ellipses |
| `O` | **Rounded Rectangle** | Rectangle with adjustable corner radius |
| `L` | **Line** | Draw straight lines between two points |
| `P` | **Pencil** | Freehand drawing for quick marks and sketches |
| `H` | **Highlight** | Semi-transparent marker for emphasizing text or regions |
| `F` | **Spotlight** | Dim everything outside a region, focusing attention |
| `T` | **Text** | Add text labels with configurable size |
| `B` | **Pixelate** | Blur sensitive information with a pixelation effect |
| `N` | **Step Markers** | Numbered markers for sequential instructions |
| `I` | **Eyedropper** | Pick any color from the screenshot |
| `M` | **Measurement** | Click two points to measure pixel distance |

### Export & Sharing

- **Clipboard copy** -- `Ctrl+C` to copy directly to clipboard for instant pasting.
- **File save** -- `Ctrl+S` to save with a file picker dialog.
- **Quick crop** -- Press `Enter` to crop and export immediately.
- **Pin to screen** -- Float captures as always-on-top windows for reference. Right-click to reveal in Explorer, middle-click to copy.
- **Imgur upload** -- Upload screenshots directly via the toolbar. Set your Imgur client ID in Settings or `config.toml`.
- **OCR text extraction** -- Extract text from a selected region using Windows OCR.
- **Recent captures history** -- Access previous captures from the tray History menu with thumbnails.
- **Toast notifications** -- Visual feedback shown directly on the overlay.

### Window Management

- **System tray** -- Lives in your system tray; left-click to start a capture.
- **Global hotkey** -- `Ctrl+Shift+S` triggers capture from anywhere.
- **Pin windows** -- Pin captures as floating always-on-top windows.
- **Auto-start** -- Optionally launch HydroShot on login.

### Interface

- **Catppuccin Mocha theme** -- Beautiful dark theme with consistent styling throughout.
- **5 color presets** -- Catppuccin palette colors, plus right-click for a native color picker.
- **Scroll wheel sizing** -- Adjust tool thickness and size with the scroll wheel.
- **Command-pattern undo/redo** -- Full undo and redo covering all operations: add, delete, move, resize, and recolor.
- **Annotation resize** -- Drag corner handles on selected annotations.
- **Lucide SVG icons** -- Professional vector icons rendered via resvg.
- **Tooltips** -- Contextual tooltips for all toolbar actions.
- **Selection size overlay** -- Live dimensions shown while selecting a region.

### Configuration

- **Tabbed Settings UI** -- In-app settings with General, Shortcuts, and Toolbar tabs.
- **Customizable keyboard shortcuts** -- Rebind all tool shortcuts in the Settings Shortcuts tab.
- **Configurable toolbar** -- Hide or show individual tools.
- **TOML config** -- Human-readable settings file with sensible defaults.
- **Persistent preferences** -- Colors, thickness, save paths, and more are remembered between sessions.

## Installation

### Pre-built Binaries

| Platform | Download |
|----------|----------|
| Windows (exe)  | [`hydroshot.exe`](https://github.com/Real-Fruit-Snacks/HydroShot/releases/latest) |
| Windows (MSI)  | [`HydroShot.msi`](https://github.com/Real-Fruit-Snacks/HydroShot/releases/latest) |
| Linux    | [`hydroshot-linux`](https://github.com/Real-Fruit-Snacks/HydroShot/releases/latest) |

Download the latest release from the [Releases](https://github.com/Real-Fruit-Snacks/HydroShot/releases) page.

### Build from Source

```bash
git clone https://github.com/Real-Fruit-Snacks/HydroShot.git
cd HydroShot
cargo build --release
# Binary: target/release/hydroshot(.exe)
```

## Usage

### Quick Start

1. Launch HydroShot -- it sits in your system tray.
2. **Left-click** the tray icon or press **Ctrl+Shift+S** to start a capture.
3. **Click and drag** to select a region.
4. Use the toolbar to annotate your screenshot.
5. Press **Enter** to crop, **Ctrl+C** to copy, or **Ctrl+S** to save.

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+S` | Start capture (global) |
| `Enter` | Crop selection |
| `Ctrl+C` | Copy to clipboard |
| `Ctrl+S` | Save to file |
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` | Redo |
| `Escape` | Cancel capture |
| `Scroll Wheel` | Adjust tool size |
| `Right-click color` | Open native color picker |

All tool shortcuts can be customized in Settings > Shortcuts.

### CLI Usage

HydroShot includes a command-line interface for scripting and automation. Running `hydroshot` with no subcommand launches the tray application.

```bash
# Capture and copy to clipboard
hydroshot capture --clipboard

# Capture and save to file
hydroshot capture --save screenshot.png

# Capture with a 3-second delay
hydroshot capture --delay 3

# Capture with delay and copy
hydroshot capture --delay 5 --clipboard
```

## Configuration

HydroShot stores its configuration in a TOML file:

- **Windows:** `%APPDATA%\hydroshot\config.toml`
- **Linux:** `~/.config/hydroshot/config.toml`

### Example Configuration

```toml
[general]
default_color = "blue"
default_thickness = 3.0
save_directory = ""
imgur_client_id = ""

[hotkey]
capture = "Ctrl+Shift+S"

[shortcuts]
select = "v"
arrow = "a"
rectangle = "r"
circle = "c"
rounded_rect = "o"
line = "l"
pencil = "p"
highlight = "h"
spotlight = "f"
text = "t"
pixelate = "b"
step_marker = "n"
eyedropper = "i"
measurement = "m"

[toolbar]
arrow = true
rectangle = true
circle = true
# ... all tools default to true
```

Settings can also be changed through the in-app Settings UI accessible from the tray menu.

## Development

### Prerequisites

- [Rust](https://rustup.rs/) 1.80 or later
- Platform-specific dependencies:
  - **Windows:** No additional dependencies
  - **Linux:** `libxkbcommon-dev`, `libwayland-dev`, `libglib2.0-dev`, `libgtk-3-dev`, `libxdo-dev`

### Building

```bash
cargo build            # Debug build
cargo build --release  # Release build
cargo test             # Run tests
cargo clippy           # Lint
cargo fmt --check      # Format check
```

### Project Structure

```
hydroshot/
├── src/
│   ├── main.rs              # Entry point, event loop
│   ├── lib.rs               # Library root
│   ├── cli.rs               # CLI argument parsing (clap)
│   ├── tray.rs              # System tray integration
│   ├── hotkey.rs            # Global hotkey registration
│   ├── config.rs            # TOML configuration
│   ├── state.rs             # Application state management
│   ├── renderer.rs          # Core rendering pipeline
│   ├── icons.rs             # Lucide SVG icon loading
│   ├── geometry.rs          # Geometry utilities
│   ├── export.rs            # Clipboard and file export
│   ├── upload.rs            # Imgur anonymous upload
│   ├── history.rs           # Recent captures history
│   ├── history_ui.rs        # History panel rendering
│   ├── ocr.rs               # OCR text extraction (Windows)
│   ├── font.rs              # Font loading and caching
│   ├── color_picker.rs      # Color selection UI
│   ├── settings_ui.rs       # Settings window (tabbed)
│   ├── autostart.rs         # Auto-start on login
│   ├── window_detect.rs     # Window detection for capture
│   ├── capture/
│   │   ├── mod.rs           # Capture abstraction
│   │   ├── windows.rs       # Windows screen capture
│   │   ├── x11.rs           # X11 screen capture
│   │   └── wayland.rs       # Wayland screen capture
│   ├── overlay/
│   │   ├── mod.rs           # Overlay window management
│   │   ├── selection.rs     # Region selection logic
│   │   └── toolbar.rs       # Annotation toolbar
│   └── tools/
│       ├── mod.rs           # Tool trait and registry
│       ├── arrow.rs         # Arrow tool
│       ├── circle.rs        # Circle/ellipse tool
│       ├── highlight.rs     # Highlight marker tool
│       ├── line.rs          # Line tool
│       ├── measurement.rs   # Measurement tool
│       ├── pencil.rs        # Freehand pencil tool
│       ├── pixelate.rs      # Pixelation tool
│       ├── rectangle.rs     # Rectangle tool
│       ├── rounded_rect.rs  # Rounded rectangle tool
│       ├── spotlight.rs     # Spotlight tool
│       ├── step_marker.rs   # Numbered step markers
│       └── text.rs          # Text annotation tool
├── assets/                  # Static assets
├── tests/                   # Integration tests
├── docs/                    # GitHub Pages site
└── Cargo.toml
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Security

To report a vulnerability, please see our [Security Policy](SECURITY.md).

## License

MIT License. See [LICENSE](LICENSE) for details.
