# Contributing to HydroShot

Thanks for your interest in contributing to HydroShot!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/<your-username>/HydroShot.git`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Run lints: `cargo clippy -- -D warnings`
7. Format: `cargo fmt`
8. Commit and push
9. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.80+ (install via [rustup](https://rustup.rs))
- Windows 10+ or Linux (X11/Wayland experimental)

### Building

```bash
cargo build          # debug build
cargo build --release # optimized build
cargo run            # run in debug mode
```

### Running Tests

```bash
cargo test           # run all tests
cargo clippy         # check for common mistakes
cargo fmt --check    # check formatting
```

## Guidelines

- Follow existing code style and patterns
- Add tests for new functionality
- Keep PRs focused — one feature or fix per PR
- Update documentation for user-facing changes

## Architecture

HydroShot uses a simple layered architecture:

- `main.rs` — winit event loop, tray integration, window management
- `renderer.rs` — tiny-skia frame composition
- `tools/` — annotation tools (each tool is a separate file)
- `capture/` — platform-specific screen capture
- `overlay/` — selection and toolbar logic
- `export.rs` — clipboard, file save, annotation flattening
- `config.rs` — TOML settings persistence
- `icons.rs` — SVG icon rendering via resvg

## Reporting Issues

- Use GitHub Issues
- Include your OS version and HydroShot version
- Steps to reproduce
- Expected vs actual behavior

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
