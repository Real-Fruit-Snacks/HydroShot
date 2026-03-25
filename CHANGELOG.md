# Changelog

All notable changes to HydroShot will be documented in this file.

## [0.5.1] - 2026-03-25

### Fixed
- Fixed command injection vulnerability in OCR PowerShell integration
- Externalized Imgur client ID to environment variable
- Fixed integer overflow in screen capture buffer sizing on multi-monitor setups
- Replaced panicking unwrap/expect calls with graceful error handling
- Moved Imgur upload to background thread to prevent UI freezing
- Fixed string slicing panics on non-ASCII text (CJK, emoji)
- Fixed selection resize allowing negative dimensions
- Fixed alpha blending precision loss in text rendering
- Added bounds checking in crop_and_flatten to prevent panics
- Fixed toolbar positioning going off-screen
- Fixed hardcoded DPI scale factor (now queries actual system DPI)
- Fixed version mismatches across CHANGELOG, webpage, and installer
- Fixed README: correct tool count (14, not 16), accurate project structure, matching config example, correct Linux build deps
- Fixed webpage: correct tool count, removed non-tool from tools grid, clarified Linux Wayland support
- Fixed installer help URL pointing to wrong GitHub repository
- Added `rust-version` to Cargo.toml to enforce MSRV 1.75

## [0.5.0] - 2026-03-24

### Added

#### New Annotation Tools
- Rounded Rectangle tool (O key) — rectangle with adjustable corner radius
- Spotlight tool (F key) — draw rectangles that dim everything outside them
- Measurement tool (M key) — click two points to show pixel distance
- Color Eyedropper (I key) — pick any color from the screenshot

#### Annotation Improvements
- Annotation resize — drag corner handles on selected annotations to resize
- Command-pattern undo/redo — covers all operations (add, delete, move, resize, recolor)

#### Export & Sharing
- Imgur upload — toolbar Upload button with confirmation click
- OCR text extraction — extract text from selected region using Windows OCR
- Recent captures history — tray History menu shows thumbnails, click to re-copy

#### Configuration
- Customizable keyboard shortcuts — rebind all tool shortcuts in Settings UI
- Configurable toolbar — hide/show individual tools in Settings Toolbar tab
- Tabbed Settings UI — General, Shortcuts, and Toolbar tabs

#### Interface & UX
- In-overlay toast notifications — feedback shown directly on the overlay
- Pin window improvements — right-click to reveal in Explorer, middle-click to copy, draggable

#### Build & Distribution
- Windows MSI installer via CI
- GitHub Actions CI — automated builds, tests, and releases
- Embedded exe icon — HydroShot icon in Windows Explorer and taskbar

#### Performance
- Cached font rendering
- Cached pixmaps for annotation tools
- 60fps frame rate cap
- Professional Lucide SVG icons via resvg

## [0.2.0] - [0.4.0] — Internal development releases

## [0.1.0] - 2026-03-25

### Added
- System tray application with left-click capture
- Global hotkey (Ctrl+Shift+S) for instant capture
- Fullscreen overlay with region selection (drag, resize, move)
- 10 annotation tools: Select/Move, Arrow, Rectangle, Circle, Line, Pencil, Highlight, Text, Pixelate, Step Markers
- Catppuccin Mocha color presets with native color picker (right-click swatch)
- Quick crop mode (Enter key)
- Copy to clipboard (Ctrl+C) and save to file (Ctrl+S)
- Pin captures to screen as always-on-top floating windows
- Window capture mode (highlight and click a window)
- Delay capture (3s/5s/10s) with visible countdown overlay
- Multi-monitor support (captures entire virtual desktop)
- CLI interface (`hydroshot capture --clipboard/--save/--delay`)
- In-app Settings UI
- TOML configuration persistence
- Auto-start on login
- Cursor feedback, selection size overlay, tooltips
- Post-action notifications
- Undo/redo for annotations
- Annotation re-selection (move, delete, recolor existing annotations)
- Keyboard shortcuts for all tools
- Performance optimized rendering (cached pixmaps, 60fps cap)
