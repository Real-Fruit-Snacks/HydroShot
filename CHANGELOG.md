# Changelog

All notable changes to HydroShot will be documented in this file.

## [0.5.4] - 2026-03-25

### Fixed
- Upload button now shows a helpful message immediately when Imgur is not configured, instead of going through the confirmation flow first
- Fixed README config example using hex color code (`#89b4fa`) that silently fell back to red — now uses named color `blue`
- Imgur client ID is now a config.toml setting (`imgur_client_id` under `[general]`) — no environment variable needed for normal use
- Settings UI shows Imgur upload configuration status
- Fixed flaky history tests on Linux CI by serializing with a mutex

## [0.5.3] - 2026-03-25

### Security
- Removed hardcoded Imgur client ID from source code — now configured via Settings/config.toml (env var `HYDROSHOT_IMGUR_CLIENT_ID` as override)
- Replaced hand-rolled JSON parser with `serde_json` to eliminate panic on malformed Imgur responses
- Fixed OCR temp file race condition with unique filenames and Drop-guard cleanup
- Added `-ExecutionPolicy Bypass` to PowerShell OCR invocation for restricted systems
- Pinned all GitHub Actions to SHA hashes to prevent supply chain attacks
- Added `permissions: contents: read` to CI workflow (principle of least privilege)

### Fixed
- Fixed negative selection coordinates wrapping to huge values when cast to u32 — added `Selection::clamped()` helper
- Fixed silent undo/redo action loss when indices become stale — actions are now validated before popping
- Fixed text cursor positioning using hardcoded 0.6 char width — now measures actual font advance widths
- Fixed 1px glyph jitter from float-to-int truncation — now uses rounding
- Fixed color precision loss in float-to-u8 conversion — now rounds instead of truncating
- Fixed settings UI triggering redraws on every sub-pixel mouse move — now only redraws when hovered element changes
- Fixed undo stack `remove(0)` O(n) performance — replaced with `drain()`
- Made `Config::save()` atomic via temp file + rename to prevent corruption on crash
- Fixed `StepMarkerTool` u32 overflow panic at `MAX` — uses `saturating_add`
- Fixed `Color::new()` accepting out-of-range values — now clamps to `[0.0, 1.0]`
- Fixed `Color::presets()` allocating a new Vec every frame — now returns `&'static [Color]`
- Fixed `IconCache::get_or_render` double HashMap lookup — uses entry API
- Fixed state.rs silently skipping pixel copy on truncated buffers — now logs warnings
- Fixed state.rs potential integer overflow in pixel buffer sizing on 32-bit — uses `checked_mul`
- Added minimum size checks to 7 annotation tools to prevent invisible zero-size annotations on click
- Fixed hotkey `letter_to_code` silently mapping unknown characters to KeyA — now returns an error
- Added warning when registering global hotkey without modifier keys
- Added F7-F12 support to hotkey parser
- Added 30-second HTTP timeout to Imgur upload requests

### Changed
- MSRV aligned to 1.80 across Cargo.toml, CONTRIBUTING.md, and CHANGELOG
- Added clippy and format checks to Linux CI job
- Generated proper UUID for WiX installer UpgradeCode (was a placeholder)
- Fixed `cd hydroshot` to `cd HydroShot` in webpage build instructions (case-sensitive Linux)
- Fixed footer copyright to match LICENSE
- Fixed querySelector("#") console error on brand logo click
- Updated Cargo.toml description to be platform-neutral

## [0.5.2] - 2026-03-25

### Fixed
- Fixed command injection vulnerability in OCR PowerShell integration
- Imgur client ID now required via `HYDROSHOT_IMGUR_CLIENT_ID` environment variable (removed hardcoded default)
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
- Added `rust-version` to Cargo.toml to enforce MSRV 1.80

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
