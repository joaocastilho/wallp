# AGENTS.md - Wallp Development Guide

## Project Overview

**Wallp** is a cross-platform desktop wallpaper manager built in Rust. It fetches random wallpapers from Unsplash and manages automatic cycling, history navigation, and system tray integration.

- **Platforms**: Windows, macOS, Linux
- **Edition**: Rust 2024
- **Binary Name**: `wallp` (or `wallp.exe` on Windows)

---

## Build Instructions

### Prerequisites

| Platform | Dependencies |
|----------|--------------|
| Windows | Visual Studio Build Tools (C++) |
| Linux | `libgtk-3-dev`, `libappindicator3-dev`, `xdotool`, `libxdo-dev` |
| macOS | Xcode Command Line Tools, `libnotify` (via Homebrew: `brew install libnotify`) |

### Build Commands

```bash
# Development build
cargo build

# Release build
cargo build --release

# Check (compile without running)
cargo check

# Run
cargo run --release
```

---

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test integration_tests

# Run clippy lints
cargo clippy --all-targets -- -D warnings -W clippy::pedantic

# Check formatting
cargo fmt -- --check
```

**Test Results**: 51 tests (41 unit + 7 integration + 2 CLI + 1 other)

---

## Project Structure

```
wallp/
├── src/
│   ├── main.rs       # Entry point, CLI parsing, Windows console handling
│   ├── cli.rs        # CLI commands, setup wizard, autostart, PATH management
│   ├── config.rs     # AppData, Config, State, Wallpaper structs + persistence
│   ├── manager.rs    # Wallpaper fetching and setting logic
│   ├── unsplash.rs   # Unsplash API client
│   ├── scheduler.rs  # Background task scheduler
│   └── tray.rs      # System tray implementation
├── tests/
│   ├── integration_tests.rs
│   ├── cli_tests.rs
│   └── cli_repro.rs
├── build.rs          # Windows resource compilation (icon, metadata)
├── Cargo.toml        # Dependencies and metadata
└── icon.ico / icon.png  # App icons
```

---

## Key Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `tao` | 0.34.5 | Window/t tray event loop |
| `tray-icon` | 0.21.3 | System tray |
| `wallpaper` | 3.2.0 | Set desktop wallpaper |
| `reqwest` | 0.13.1 | HTTP client for Unsplash API |
| `tokio` | 1.49.0 | Async runtime |
| `chrono` | 0.4.43 | Date/time handling |
| `dialoguer` | 0.12.0 | Interactive CLI prompts |
| `single-instance` | 0.3.3 | Prevent multiple tray instances |
| `auto-launch` | 0.6.0 | Auto-start on login |
| `directories` | 6.0.0 | Platform-specific config dirs |

### Platform-Specific

| Platform | Dependencies |
|----------|--------------|
| Windows | `windows` 0.62.2, `winreg` 0.55.0, `winres` (build) |
| Unix | `notify-rust` 4.11.3 |

---

## Code Organization

### Entry Point (`main.rs`)

- Parses CLI arguments using `clap`
- Handles Windows console detachment for tray mode
- Routes to setup wizard or command handler

### CLI Commands (`cli.rs`)

- `init_wizard()`: Interactive setup
- `handle_command()`: Route subcommands
- `setup_autostart()`: Platform-specific autostart
- `add_to_path_*` / `remove_from_path_*`: PATH management
- `handle_uninstall()`: Cleanup and self-deletion

### Configuration (`config.rs`)

- `AppData`: Root config structure (config + state + history)
- `Config`: User settings (API key, interval, collections, retention)
- `State`: Runtime state (is_running, next_run_at, current_index)
- `Wallpaper`: Historical wallpaper record

**Config Location**:
- Windows: `%APPDATA%\wallp\wallp.json`
- Linux: `~/.config/wallp/wallp.json`
- macOS: `~/Library/Application Support/wallp/wallp.json`

### Manager (`manager.rs`)

- `new()`: Fetch and set new wallpaper
- `next()`: Navigate forward (or fetch new)
- `prev()`: Navigate backward in history
- `get_current_wallpaper()`: Get current wallpaper info
- `fetch_and_set_new()`: Core logic for fetching from Unsplash

### Unsplash Client (`unsplash.rs`)

- `fetch_random()`: Get random photo from collections
- `download_image()`: Download and save image file

### Scheduler (`scheduler.rs`)

- `start_background_task()`: Runs every 60 seconds
- `check_and_run()`: Checks if it's time for next wallpaper

### System Tray (`tray.rs`)

- Single instance enforcement
- Menu construction with autostart toggle
- Icon loading from embedded resources
- Event handling for menu actions

---

## Common Development Tasks

### Adding a New CLI Command

1. Add to `Commands` enum in `main.rs`
2. Implement handler in `cli.rs::handle_command()`
3. Add tests if appropriate

### Adding Configuration Option

1. Add field to `Config` struct in `config.rs`
2. Add to `set()` method for `config set` command
3. Add to `show()` method for `config` display
4. Add default in `Config::default()`

### Modifying Platform-Specific Code

| Feature | Location |
|---------|-----------|
| Windows autostart | `cli.rs::build_auto_launch()` with `winreg` |
| Linux autostart | `cli.rs::build_auto_launch()` with freedesktop |
| macOS autostart | `cli.rs::build_auto_launch()` with `MacOSLaunchMode::LaunchAgent` |
| Console detachment | `main.rs::win_utils` module |
| PATH modification | `cli.rs::add_to_path_windows()` / `add_to_path_unix()` |
| Tray notifications | `tray.rs` with `notify_rust` |

### Updating Dependencies

Update version in `Cargo.toml`, then run:
```bash
cargo update
cargo build
cargo test
```

---

## Platform-Specific Notes

### Windows

- Uses registry for autostart (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`)
- Uses registry for PATH modification (`HKCU\Environment`)
- Console detachment via `FreeConsole()` API
- Icon compiled from `icon.ico` via `winres`

### Linux

- Uses freedesktop autostart (`~/.config/autostart/wallp.desktop`)
- Modifies shell profiles (`.bashrc`, `.bash_profile`, `.zshrc`, `.zprofile`)
- Requires `libgtk-3-dev` and `libappindicator3-dev` for tray

### macOS

- Uses LaunchAgent for autostart
- Modifies shell profiles
- Requires `libnotify` for notifications

---

## Testing Guidelines

1. **Unit Tests**: Test pure functions in `src/.../tests` modules
2. **Integration Tests**: Test file I/O, config serialization in `tests/`
3. **CLI Tests**: Test CLI parsing in `tests/cli_*.rs`
4. **Platform Testing**: Test on all 3 platforms before merging

### Key Test Patterns

- Use `tempfile::TempDir` for file operations
- Mock external services (Unsplash API) - tests use deserialization
- Test edge cases: empty collections, invalid JSON, missing files

---

## CI/CD

GitHub Actions workflow in `.github/workflows/ci.yml`:

1. Runs on: Ubuntu, Windows, macOS
2. Installs platform-specific dependencies
3. Checks formatting (`cargo fmt`)
4. Runs clippy with strict warnings
5. Builds and tests

---

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Build fails on Linux | Install: `sudo apt-get install libgtk-3-dev libappindicator3-dev xdotool libxdo-dev` |
| Build fails on macOS | Install Xcode: `xcode-select --install` |
| Notifications not working on macOS | Install libnotify: `brew install libnotify` |
| Tray icon missing | Check desktop environment support |
| Tests fail | Ensure dependencies installed for your platform |

---

## Release Process

1. Update version in `Cargo.toml`
2. Update `build.rs` version strings
3. Run full test suite on all platforms
4. Build release: `cargo build --release`
5. Create GitHub release with binary artifacts
