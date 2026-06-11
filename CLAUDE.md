# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

HydroShot (package/binary: `hydroshot`) — a native screenshot capture and annotation tool in pure Rust. No Electron, no GPU: winit for windowing, softbuffer for surface presentation, tiny-skia for all 2D drawing on the CPU. Windows is the primary platform; Linux (X11, partial Wayland) is secondary. Rust 1.80+, edition 2021. GitHub repo: Real-Fruit-Snacks/HydroShot.

## Commands

```bash
cargo build                     # debug build
cargo build --release           # → target/release/hydroshot(.exe)
cargo run                       # run tray app in debug mode
cargo test                      # run all tests
cargo test --test undo_tests    # run one test file (tests/undo_tests.rs)
cargo test test_undo_add        # run a single test by name
cargo clippy -- -D warnings     # lint — CI fails on any warning
cargo fmt                       # format (CI runs cargo fmt --check)
cargo run --example capture_test  # capture all screens, save PNGs (manual smoke test)
```

CI (`.github/workflows/ci.yml`) runs build/test/clippy/fmt on both windows-latest and ubuntu-latest. Linux builds need: `libxkbcommon-dev libwayland-dev libglib2.0-dev libgtk-3-dev libxdo-dev`.

CLI subcommands (see `src/cli.rs`): `hydroshot capture [--clipboard] [--save path] [--delay N]`, `hydroshot install`, `hydroshot uninstall`. No subcommand = tray-resident app.

**Do not remove the `[profile.dev]` overrides in Cargo.toml.** Dependencies are built at opt-level 2 even in dev because pixel operations (tiny-skia, fontdue, softbuffer) are 10–50x slower unoptimized, making the GUI unusable in debug builds.

## Architecture

### lib/bin split — testability boundary

`src/lib.rs` exposes every module; `src/main.rs` is the binary: the `App` struct, the winit `ApplicationHandler` event loop, tray/hotkey wiring, and export actions (`do_copy`, `do_save`, `do_upload`, `do_ocr`, `do_pin`). Pinned windows live in `src/pin.rs` (`PinnedWindow` owns its window/surface/temp file; `Drop` deletes the temp file) and the delay countdown in `src/countdown.rs`. All tests are integration tests in `tests/` importing via `hydroshot::…` — pure logic (geometry, selection math, undo, annotation ops, export flattening, config, hotkey parsing) deliberately lives in library modules so it is testable headlessly. When adding logic, put the testable core in a lib module, not main.rs.

### State machine

`src/state.rs`: `AppState::Idle` ↔ `AppState::Capturing(Box<OverlayState>)`. `OverlayState` owns everything for a capture session: the screenshot plus two pre-converted pixmaps (raw and dimmed — computed once at capture time, never per frame), the selection, annotations, undo/redo stacks, one instance of each tool, and all interaction state (drag, resize handle, text input buffer, toast, visible toolbar buttons).

### Annotation pipeline — one render path

- Each tool in `src/tools/*.rs` implements the `AnnotationTool` trait (`on_mouse_down/move/up`, `in_progress_annotation` for live preview). Tools produce `Annotation` enum variants (`tools/mod.rs`) which are pure data — all behavior lives in free functions.
- `render_annotation()` in `tools/mod.rs` is the **single rendering path** used for both the interactive overlay preview and final export. Pixelate/Spotlight take the optional `screenshot_pixels` args since they sample the underlying image.
- `tools/mod.rs` also holds the shared annotation ops used by the Select tool: `hit_test_annotation`, `move_annotation`, `resize_annotation`, `recolor_annotation`, `annotation_bounding_box`.
- Export (`src/export.rs`): `flatten_annotations` / `crop_and_flatten` burn annotations into pixels via the same `render_annotation`, then the result goes to clipboard (arboard), file, pin window, Imgur (`upload.rs`), or Windows OCR (`ocr.rs`).

### Undo/redo — command pattern

`UndoAction::{Add(idx), Delete(idx, Annotation), Modify(idx, old)}` with `record_undo` / `apply_undo` / `apply_redo` in `tools/mod.rs`. Invariants: `Modify` stores the OLD annotation; `record_undo` clears the redo stack; stacks are capped at 50 entries. Drag/resize undo uses `OverlayState.pre_drag_annotation` to snapshot the annotation before the gesture starts, recording a single `Modify` when it ends.

### Capture backends

`src/capture/mod.rs` defines the `ScreenCapture` trait and `CapturedScreen` (RGBA8 pixels + virtual-desktop x/y offsets + scale factor). Platform impls are cfg-gated. `windows.rs` (GDI) captures the entire virtual desktop as ONE `CapturedScreen` spanning all monitors; `x11.rs` does the same via x11rb root-window GetImage (works under XWayland too); `wayland.rs` is still a stub (native portal capture TODO). Capture backends must force `alpha = 255` — downstream code (premultiplied pixmaps, `flatten_annotations`) relies on opaque input. Selection/pin math is in screenshot coordinates; convert to screen coordinates by adding `x_offset`/`y_offset` (they're negative when a monitor sits left/above the primary).

### Toolbar — single source of truth

`overlay/toolbar.rs` defines `const BUTTONS: [ButtonDef; 24]` — each entry carries its `ButtonAction` (Tool/Color/Ocr/Upload/Pin/Copy/Save), Lucide icon name, and tooltip. Array order defines the "original" indices (0–13 tools, 14–18 colors, 19–23 actions) that `ToolbarConfig::visible_button_indices()` refers to. Renderer and click handling both dispatch off this table — add/change a button HERE, not in per-site matches. Hit-testing returns a *visible* index; `OverlayState.visible_buttons[vis_idx]` maps back to the original index.

### Adding a new annotation tool

New tool file in `src/tools/` + variant in `Annotation` + `ToolKind` + arm in `render_annotation` (and hit-test/move/resize/bounding-box fns) + tool instance field in `OverlayState` (and its `set_color_all`/`set_thickness_all` helpers) + a `BUTTONS` entry in toolbar.rs + icon SVG in `icons.rs` + `ShortcutsConfig`/`ToolbarConfig` in `config.rs` + key/mouse dispatch in main.rs + a `tests/<tool>_tests.rs` file. Follow an existing tool (e.g. `rounded_rect.rs`) end to end.

### Theming

Catppuccin Mocha is hardcoded throughout. Named palette colors live in `geometry::Color` (`red()`, `blue()`, … with hex comments); ad-hoc UI colors in renderer/main are raw `tiny_skia::Color::from_rgba` calls with hex comments. Stay on-palette when adding UI.

### Config

TOML via serde at `%APPDATA%\hydroshot\config.toml` (Windows) / `~/.config/hydroshot/config.toml` (Linux). Sections: `[general]`, `[hotkey]`, `[shortcuts]`, `[toolbar]`. Every field carries a per-field `#[serde(default = ...)]` so configs missing any key still parse — keep that invariant for new fields. History PNGs live in the LOCAL data dir (`history.rs`), not the roaming config dir, deliberately.

### Windows specifics

`main.rs` sets `windows_subsystem = "windows"` (no console; CLI subcommands re-attach to the parent console; spawned console tools need `CREATE_NO_WINDOW` or they flash a window). `build.rs` generates `assets/icon.ico` from `assets/icon.png` and embeds winres metadata. `installer.rs` implements self-install/uninstall (AppData copy, Start Menu shortcut, PATH, single-instance enforcement) — it PROMPTS before installing, and `needs_install()` is hardwired false in debug builds and on non-Windows, so `cargo run` always runs portably. `wix/main.wxs` is the MSI definition. Autostart: Registry on Windows, XDG autostart on Linux (`autostart.rs`) — exe paths are quoted in both.

## Conventions

- CHANGELOG.md is maintained per release (Keep-a-Changelog style with Fixed/Added sections); update it for user-facing changes.
- Avoid panics in runtime paths: pixel-buffer code uses checked math, bounds-clamped copies, and `tracing::warn!` fallbacks rather than unwraps (see `OverlayState::new`, `renderer.rs`). `tracing` is the logging facility.
- Long-running work triggered from the overlay (OCR, upload, clipboard for pins) goes to background threads to keep the event loop responsive.
