<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/Real-Fruit-Snacks/HydroShot/main/docs/assets/logo-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/Real-Fruit-Snacks/HydroShot/main/docs/assets/logo-light.svg">
  <img alt="HydroShot" src="https://raw.githubusercontent.com/Real-Fruit-Snacks/HydroShot/main/docs/assets/logo-dark.svg" width="520">
</picture>

![Rust](https://img.shields.io/badge/language-Rust-orange.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

**Fast, lightweight screenshot capture and annotation tool built with Rust, winit, and tiny-skia**

Region selection, window capture, delay timer, 14 annotation tools, clipboard/file export, pin-to-screen, Imgur upload, OCR text extraction, and recent captures history. Catppuccin Mocha themed with customizable shortcuts and toolbar.

[Features](#features) • [Quick Start](#quick-start) • [Usage](#usage) • [Configuration](#configuration) • [Architecture](#architecture) • [Security](#security)

</div>

---

## Highlights

<table>
<tr>
<td width="50%">

**Region & Window Capture**
Click and drag to capture any area. Highlight and click any window. Timed captures (3s, 5s, 10s) with visible countdown. Multi-monitor support across all connected displays. Fullscreen overlay dims inactive areas for precise selection.

**14 Annotation Tools**
Arrow, rectangle, circle, rounded rectangle, line, pencil, highlight, spotlight, text, pixelate, step markers, eyedropper, measurement, and select/move. All with keyboard shortcuts, scroll-wheel sizing, and command-pattern undo/redo.

**Export & Sharing**
Clipboard copy (Ctrl+C), file save (Ctrl+S), quick crop (Enter), pin to screen as always-on-top window, Imgur upload, OCR text extraction, recent captures history with thumbnails, and toast notifications.

</td>
<td width="50%">

**System Tray Integration**
Lives in your system tray. Left-click to start capture, global hotkey (Ctrl+Shift+S) from anywhere. Pin windows as floating always-on-top references. Optional auto-start on login.

**Catppuccin Mocha Theme**
Beautiful dark theme with consistent styling throughout. 5 color presets from the Catppuccin palette, plus native color picker via right-click. Lucide SVG icons rendered via resvg.

**Fully Configurable**
Tabbed settings UI with General, Shortcuts, and Toolbar tabs. Customizable keyboard shortcuts for all tools. Configurable toolbar visibility. TOML config with persistent preferences between sessions.

</td>
</tr>
</table>

---

## Quick Start

### Prerequisites

<table>
<tr>
<th>Requirement</th>
<th>Version</th>
<th>Purpose</th>
</tr>
<tr>
<td>Rust</td>
<td>1.80+</td>
<td>Compiler toolchain</td>
</tr>
<tr>
<td>Cargo</td>
<td>1.80+</td>
<td>Build system and package manager</td>
</tr>
</table>

### Build

```bash
# Clone repository
git clone https://github.com/Real-Fruit-Snacks/HydroShot.git
cd HydroShot

# Build release binary
cargo build --release

# Binary location
# target/release/hydroshot(.exe)
```

### Pre-built Binaries

<table>
<tr>
<th>Platform</th>
<th>Download</th>
</tr>
<tr>
<td>Windows (exe)</td>
<td><a href="https://github.com/Real-Fruit-Snacks/HydroShot/releases/latest"><code>hydroshot.exe</code></a></td>
</tr>
<tr>
<td>Windows (MSI)</td>
<td><a href="https://github.com/Real-Fruit-Snacks/HydroShot/releases/latest"><code>HydroShot.msi</code></a></td>
</tr>
<tr>
<td>Linux</td>
<td><a href="https://github.com/Real-Fruit-Snacks/HydroShot/releases/latest"><code>hydroshot-linux</code></a></td>
</tr>
</table>

### Verification

```bash
# Launch -- sits in system tray
./target/release/hydroshot

# Capture and copy to clipboard
hydroshot capture --clipboard

# Capture with 3-second delay
hydroshot capture --delay 3
```

---

## Features

### Annotation Tools (14 tools)

<table>
<tr>
<th>Shortcut</th>
<th>Tool</th>
<th>Description</th>
</tr>
<tr><td><code>V</code></td><td><strong>Select / Move</strong></td><td>Reposition and resize annotations after placement</td></tr>
<tr><td><code>A</code></td><td><strong>Arrow</strong></td><td>Draw directional arrows to highlight points of interest</td></tr>
<tr><td><code>R</code></td><td><strong>Rectangle</strong></td><td>Draw outlined or filled rectangles for emphasis</td></tr>
<tr><td><code>C</code></td><td><strong>Circle</strong></td><td>Draw outlined or filled circles and ellipses</td></tr>
<tr><td><code>O</code></td><td><strong>Rounded Rectangle</strong></td><td>Rectangle with adjustable corner radius</td></tr>
<tr><td><code>L</code></td><td><strong>Line</strong></td><td>Draw straight lines between two points</td></tr>
<tr><td><code>P</code></td><td><strong>Pencil</strong></td><td>Freehand drawing for quick marks and sketches</td></tr>
<tr><td><code>H</code></td><td><strong>Highlight</strong></td><td>Semi-transparent marker for emphasizing text or regions</td></tr>
<tr><td><code>F</code></td><td><strong>Spotlight</strong></td><td>Dim everything outside a region, focusing attention</td></tr>
<tr><td><code>T</code></td><td><strong>Text</strong></td><td>Add text labels with configurable size</td></tr>
<tr><td><code>B</code></td><td><strong>Pixelate</strong></td><td>Blur sensitive information with a pixelation effect</td></tr>
<tr><td><code>N</code></td><td><strong>Step Markers</strong></td><td>Numbered markers for sequential instructions</td></tr>
<tr><td><code>I</code></td><td><strong>Eyedropper</strong></td><td>Pick any color from the screenshot</td></tr>
<tr><td><code>M</code></td><td><strong>Measurement</strong></td><td>Click two points to measure pixel distance</td></tr>
</table>

### Keyboard Shortcuts

<table>
<tr>
<th>Shortcut</th>
<th>Action</th>
</tr>
<tr><td><code>Ctrl+Shift+S</code></td><td>Start capture (global)</td></tr>
<tr><td><code>Enter</code></td><td>Crop selection</td></tr>
<tr><td><code>Ctrl+C</code></td><td>Copy to clipboard</td></tr>
<tr><td><code>Ctrl+S</code></td><td>Save to file</td></tr>
<tr><td><code>Ctrl+Z</code></td><td>Undo</td></tr>
<tr><td><code>Ctrl+Shift+Z</code></td><td>Redo</td></tr>
<tr><td><code>Escape</code></td><td>Cancel capture</td></tr>
<tr><td><code>Scroll Wheel</code></td><td>Adjust tool size</td></tr>
<tr><td><code>Right-click color</code></td><td>Open native color picker</td></tr>
</table>

All tool shortcuts can be customized in Settings > Shortcuts.

---

## Usage

### Quick Start

1. Launch HydroShot -- it sits in your system tray.
2. **Left-click** the tray icon or press **Ctrl+Shift+S** to start a capture.
3. **Click and drag** to select a region.
4. Use the toolbar to annotate your screenshot.
5. Press **Enter** to crop, **Ctrl+C** to copy, or **Ctrl+S** to save.

### CLI Usage

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

---

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

---

## Architecture

```
HydroShot/
├── Cargo.toml                      # Dependencies, features, release profile
│
├── src/
│   ├── main.rs                     # Entry point, event loop
│   ├── lib.rs                      # Library root
│   ├── cli.rs                      # CLI argument parsing (clap)
│   ├── tray.rs                     # System tray integration
│   ├── hotkey.rs                   # Global hotkey registration
│   ├── config.rs                   # TOML configuration
│   ├── state.rs                    # Application state management
│   ├── renderer.rs                 # Core rendering pipeline
│   ├── icons.rs                    # Lucide SVG icon loading
│   ├── geometry.rs                 # Geometry utilities
│   ├── export.rs                   # Clipboard and file export
│   ├── upload.rs                   # Imgur anonymous upload
│   ├── history.rs                  # Recent captures history
│   ├── history_ui.rs               # History panel rendering
│   ├── ocr.rs                      # OCR text extraction (Windows)
│   ├── font.rs                     # Font loading and caching
│   ├── color_picker.rs             # Color selection UI
│   ├── settings_ui.rs              # Settings window (tabbed)
│   ├── autostart.rs                # Auto-start on login
│   ├── window_detect.rs            # Window detection for capture
│   │
│   ├── capture/                    # ── Screen Capture ──
│   │   ├── mod.rs                  # Capture abstraction
│   │   ├── windows.rs              # Windows screen capture
│   │   ├── x11.rs                  # X11 screen capture
│   │   └── wayland.rs              # Wayland screen capture
│   │
│   ├── overlay/                    # ── Overlay Window ──
│   │   ├── mod.rs                  # Overlay window management
│   │   ├── selection.rs            # Region selection logic
│   │   └── toolbar.rs              # Annotation toolbar
│   │
│   └── tools/                      # ── Annotation Tools ──
│       ├── mod.rs                  # Tool trait and registry
│       ├── arrow.rs                # Arrow tool
│       ├── circle.rs               # Circle/ellipse tool
│       ├── highlight.rs            # Highlight marker tool
│       ├── line.rs                 # Line tool
│       ├── measurement.rs          # Measurement tool
│       ├── pencil.rs               # Freehand pencil tool
│       ├── pixelate.rs             # Pixelation tool
│       ├── rectangle.rs            # Rectangle tool
│       ├── rounded_rect.rs         # Rounded rectangle tool
│       ├── spotlight.rs            # Spotlight tool
│       ├── step_marker.rs          # Numbered step markers
│       └── text.rs                 # Text annotation tool
│
├── assets/                          # Static assets
├── tests/                           # Integration tests
│
├── docs/                            # ── GitHub Pages ──
│   ├── index.html                  # Project website
│   └── assets/
│       ├── logo-dark.svg           # Logo for dark theme
│       └── logo-light.svg          # Logo for light theme
│
└── .github/
    └── workflows/
        └── ci.yml                  # CI pipeline
```

---

## Development

### Prerequisites

- [Rust](https://rustup.rs/) 1.80 or later
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

---

## Platform Support

<table>
<tr>
<th>Capability</th>
<th>Windows</th>
<th>Linux</th>
</tr>
<tr>
<td>Region Capture</td>
<td>Full</td>
<td>Full (X11 + Wayland)</td>
</tr>
<tr>
<td>Window Capture</td>
<td>Full</td>
<td>X11 only</td>
</tr>
<tr>
<td>Delay Capture</td>
<td>Full</td>
<td>Full</td>
</tr>
<tr>
<td>Multi-Monitor</td>
<td>Full</td>
<td>Full</td>
</tr>
<tr>
<td>14 Annotation Tools</td>
<td>Full</td>
<td>Full</td>
</tr>
<tr>
<td>Clipboard Copy</td>
<td>Full</td>
<td>Full</td>
</tr>
<tr>
<td>System Tray</td>
<td>Full</td>
<td>Full</td>
</tr>
<tr>
<td>Global Hotkey</td>
<td>Full</td>
<td>Full</td>
</tr>
<tr>
<td>OCR</td>
<td>Windows OCR API</td>
<td>Not supported</td>
</tr>
<tr>
<td>Imgur Upload</td>
<td>Full</td>
<td>Full</td>
</tr>
<tr>
<td>Auto-Start</td>
<td>Registry</td>
<td>XDG autostart</td>
</tr>
</table>

---

## Security

### Vulnerability Reporting

**Report security issues via:**
- GitHub Security Advisories (preferred)
- Private disclosure to maintainers
- Responsible disclosure timeline (90 days)

**Do NOT:**
- Open public GitHub issues for vulnerabilities
- Disclose before coordination with maintainers

### What HydroShot Does NOT Do

HydroShot is a **screenshot capture and annotation tool**, not a security tool:

- **Not a keylogger** -- Captures only when explicitly triggered by the user
- **Not spyware** -- No telemetry, no analytics, no phone-home
- **Not a screen recorder** -- Single-frame capture only
- **Imgur uploads are opt-in** -- Only when explicitly triggered via toolbar button

---

## License

MIT License

Copyright &copy; 2026 Real-Fruit-Snacks

```
THIS SOFTWARE IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND.
THE AUTHORS ARE NOT LIABLE FOR ANY DAMAGES ARISING FROM USE.
```

---

## Resources

- **GitHub**: [github.com/Real-Fruit-Snacks/HydroShot](https://github.com/Real-Fruit-Snacks/HydroShot)
- **Releases**: [Latest Release](https://github.com/Real-Fruit-Snacks/HydroShot/releases/latest)
- **Issues**: [Report a Bug](https://github.com/Real-Fruit-Snacks/HydroShot/issues)
- **Security**: [SECURITY.md](SECURITY.md)
- **Contributing**: [CONTRIBUTING.md](CONTRIBUTING.md)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)

---

<div align="center">

**Part of the Real-Fruit-Snacks water-themed security toolkit**

[Aquifer](https://github.com/Real-Fruit-Snacks/Aquifer) • [Cascade](https://github.com/Real-Fruit-Snacks/Cascade) • [Conduit](https://github.com/Real-Fruit-Snacks/Conduit) • [Flux](https://github.com/Real-Fruit-Snacks/Flux) • **HydroShot** • [Riptide](https://github.com/Real-Fruit-Snacks/Riptide) • [Runoff](https://github.com/Real-Fruit-Snacks/Runoff) • [Seep](https://github.com/Real-Fruit-Snacks/Seep) • [Slipstream](https://github.com/Real-Fruit-Snacks/Slipstream) • [Tidepool](https://github.com/Real-Fruit-Snacks/Tidepool) • [Undertow](https://github.com/Real-Fruit-Snacks/Undertow) • [Whirlpool](https://github.com/Real-Fruit-Snacks/Whirlpool)

*Remember: With great power comes great responsibility.*

</div>
