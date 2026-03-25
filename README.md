<div align="center">

<img src="docs/icon_256.png" alt="HydroShot" width="128">

# HydroShot

A fast, lightweight screenshot capture and annotation tool.
Built with Rust, winit, and tiny-skia.

<p>
  <a href="#features">Features</a> &bull;
  <a href="#installation">Installation</a> &bull;
  <a href="#usage">Usage</a> &bull;
  <a href="#configuration">Configuration</a> &bull;
  <a href="#development">Development</a> &bull;
  <a href="#license">License</a>
</p>

</div>

---

## Features

### Capture

- **Region selection** — Click and drag to capture any area of your screen. Resize and reposition selections before committing.
- **Window capture** — Highlight and click any window to capture it instantly.
- **Delay capture** — Timed captures (3s, 5s, 10s) with a visible countdown overlay, giving you time to set up the shot.
- **Multi-monitor support** — Captures the entire virtual desktop across all connected monitors.
- **Fullscreen overlay** — Semi-transparent overlay dims inactive areas for precise region selection.

### Annotation Tools

- **Select/Move** — Reposition and resize annotations after placement.
- **Arrow** — Draw directional arrows to highlight points of interest.
- **Rectangle** — Draw outlined or filled rectangles for emphasis.
- **Circle** — Draw outlined or filled circles and ellipses.
- **Line** — Draw straight lines between two points.
- **Pencil** — Freehand drawing for quick marks and sketches.
- **Highlight** — Semi-transparent marker for emphasizing text or regions.
- **Text** — Add text labels with configurable size.
- **Pixelate** — Blur sensitive information with a pixelation effect.
- **Step Markers** — Numbered markers for sequential instructions.

### Export & Sharing

- **Clipboard copy** — `Ctrl+C` to copy directly to clipboard for instant pasting.
- **File save** — `Ctrl+S` to save with a file picker dialog.
- **Quick crop** — Press `Enter` to crop and export immediately.
- **Pin to screen** — Float captures as always-on-top windows for reference.
- **Post-action notifications** — Visual feedback confirming successful exports.

### Window Management

- **System tray** — Lives in your system tray; left-click to start a capture.
- **Global hotkey** — `Ctrl+Shift+S` triggers capture from anywhere.
- **Pin windows** — Pin captures as floating always-on-top windows.
- **Auto-start** — Optionally launch HydroShot on login.

### Interface

- **Catppuccin Mocha theme** — Beautiful dark theme with consistent styling throughout.
- **5 color presets** — Catppuccin Mocha palette colors, plus right-click for a native color picker.
- **Scroll wheel sizing** — Adjust tool thickness and size with the scroll wheel.
- **Undo/Redo** — Full undo and redo support for annotations.
- **Lucide SVG icons** — Professional vector icons rendered via resvg.
- **Tooltips** — Contextual tooltips for all toolbar actions.
- **Selection size overlay** — Live dimensions shown while selecting a region.
- **Cursor feedback** — Context-appropriate cursor changes.

### Configuration

- **Settings UI** — In-app settings window for easy configuration.
- **TOML config** — Human-readable settings file with sensible defaults.
- **Persistent preferences** — Colors, thickness, save paths, and more are remembered between sessions.

## Installation

### Pre-built Binaries

| Platform | Download |
|----------|----------|
| Windows  | [`hydroshot.exe`](https://github.com/Real-Fruit-Snacks/HydroShot/releases/latest) |
| Linux    | [`hydroshot-linux`](https://github.com/Real-Fruit-Snacks/HydroShot/releases/latest) |

Download the latest release from the [Releases](https://github.com/Real-Fruit-Snacks/HydroShot/releases) page.

### Build from Source

```bash
# Clone the repository
git clone https://github.com/Real-Fruit-Snacks/HydroShot.git
cd hydroshot

# Build in release mode
cargo build --release

# The binary will be at target/release/hydroshot(.exe)
```

## Usage

### Quick Start

1. Launch HydroShot — it sits in your system tray.
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
| `Ctrl+Y` | Redo |
| `Escape` | Cancel capture |
| `Scroll Wheel` | Adjust tool size |
| `Right-click color` | Open native color picker |

### CLI Usage

HydroShot includes a command-line interface for scripting and automation.

```bash
# Capture and copy to clipboard
hydroshot capture --clipboard

# Capture and save to file
hydroshot capture --save

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
[capture]
save_path = "~/Pictures/Screenshots"
copy_to_clipboard = true
show_notifications = true

[appearance]
default_color = "#89b4fa"
default_thickness = 3.0

[behavior]
auto_start = false
global_hotkey = "Ctrl+Shift+S"
```

Settings can also be changed through the in-app settings UI accessible from the tray menu.

## Development

### Prerequisites

- [Rust](https://rustup.rs/) 1.75 or later
- Platform-specific dependencies:
  - **Windows:** No additional dependencies
  - **Linux:** `libx11-dev`, `libxrandr-dev`, `libxcomposite-dev`

### Building

```bash
# Debug build (with optimized dependencies for GUI responsiveness)
cargo build

# Release build
cargo build --release

# Run tests
cargo test
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
│   ├── color_picker.rs      # Color selection UI
│   ├── settings_ui.rs       # Settings window
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
│       ├── pencil.rs        # Freehand pencil tool
│       ├── pixelate.rs      # Pixelation tool
│       ├── rectangle.rs     # Rectangle tool
│       ├── step_marker.rs   # Numbered step markers
│       └── text.rs          # Text annotation tool
├── assets/                  # Static assets
├── tests/                   # Integration tests
├── docs/                    # GitHub Pages site
└── Cargo.toml
```

## License

MIT License. See [LICENSE](LICENSE) for details.
