<div align="center">

# 🖼️ Wallp

[![CI](https://github.com/joaocastilho/wallp/actions/workflows/ci.yml/badge.svg)](https://github.com/joaocastilho/wallp/actions/workflows/ci.yml)
[![Release](https://github.com/joaocastilho/wallp/actions/workflows/release.yml/badge.svg)](https://github.com/joaocastilho/wallp/releases)
[![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)]()

### 🎨 A lightweight, cross-platform wallpaper manager for your desktop

</div>

Wallp is a powerful yet minimal CLI and System Tray application that automatically manages and cycles through stunning desktop wallpapers from Unsplash. Built with **Rust** for blazing-fast performance and minimal resource usage.

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🎛️ **System Tray** | Control wallpapers directly from your taskbar with an intuitive menu |
| ⏪ **Smart History** | Unlimited undo/redo with session persistence across restarts |
| ⏰ **Auto-Cycling** | Set custom intervals (1 minute to 24 hours) for automatic wallpaper changes |
| 🖼️ **Unsplash Integration** | Access millions of high-quality photos from curated collections |
| 🖥️ **Cross-Platform** | Native support for Windows, macOS, and Linux |
| 🚀 **Auto-Start** | Silently launches on system boot with no UI interruption |
| 📦 **Zero Dependencies** | Single binary with no external runtime requirements |
| 💾 **Smart Caching** | Automatic cleanup with configurable retention policies |

---

## 📥 Download

Get the latest pre-built binaries from [GitHub Releases](https://github.com/joaocastilho/wallp/releases/latest):

| Platform | Download | Arch |
|----------|----------|------|
| 🪟 **Windows** | [wallp-windows-x64.exe](https://github.com/joaocastilho/wallp/releases/latest/download/wallp-windows-x64.exe) | x64 |
| 🍎 **macOS** | [wallp-macos-arm64](https://github.com/joaocastilho/wallp/releases/latest/download/wallp-macos-arm64) | Apple Silicon |
| 🐧 **Linux** | [wallp-linux-x64](https://github.com/joaocastilho/wallp/releases/latest/download/wallp-linux-x64) | x64 |

Or install from source with `cargo install --git https://github.com/joaocastilho/wallp`

---

## 🚀 Quick Start

### Installation

#### Option 1: Download Pre-built Binary (Recommended)

1. Download the appropriate binary for your platform from the [releases page](https://github.com/joaocastilho/wallp/releases/latest)
2. Place it in a directory that's in your PATH
3. Run `wallp` - on first run, the setup wizard will start automatically

#### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/joaocastilho/wallp
cd wallp

# Build and install
cargo install --path .
```

The executable will be installed to your Cargo bin directory (`~/.cargo/bin/` or `%USERPROFILE%\.cargo\bin\`).

---

### First-Time Setup

On first run, Wallp will automatically launch the interactive setup wizard to configure your Unsplash API key and preferences. You can also run it manually at any time:

```bash
wallp setup
```

The wizard will:
1. 🔑 Prompt for your Unsplash Access Key ([Get one free](https://unsplash.com/developers))
2. 🎯 Configure collection preferences
3. ⏱️ Set cycling intervals
4. 🚀 Enable autostart
5. ▶️ Launch the System Tray app

---

### 📋 CLI Commands

| Command | Description | Example |
|---------|-------------|---------|
| `wallp` | Start the System Tray application (runs in background, or runs setup on first use) | `wallp` |
| `wallp setup` | Run the interactive setup wizard | `wallp setup` |
| `wallp next` | Go to next wallpaper (history-aware) | `wallp next` |
| `wallp prev` | Go to previous wallpaper | `wallp prev` |
| `wallp new` | Force fetch a brand new wallpaper | `wallp new` |
| `wallp info` | Show metadata for current wallpaper | `wallp info` |
| `wallp open` | Open current wallpaper in browser | `wallp open` |
| `wallp folder` | Open local wallpapers folder | `wallp folder` |
| `wallp status` | Check background scheduler status | `wallp status` |
| `wallp config` | View configuration | `wallp config` |
| `wallp config edit` | Open config file in default editor | `wallp config edit` |
| `wallp config set <key> <value>` | Set a config value | `wallp config set interval_minutes 60` |
| `wallp list` | Show recent wallpaper history | `wallp list` |
| `wallp uninstall` | Remove Wallp and all data | `wallp uninstall` |

---

### 🎛️ System Tray Menu

Right-click the Wallp icon in your system tray to access:

| Menu Item | Action |
|-----------|--------|
| ✨ **New Wallpaper** | Fetch a random image from Unsplash |
| ⏭️ **Next** | Navigate forward in history |
| ⏮️ **Previous** | Navigate backward in history |
| 📂 **Open Folder** | View downloaded wallpapers folder |
| ⚙️ **Open Config** | Open the configuration file |
| ⬜ **Run at Startup** | Toggle automatic launch on login (checkbox) |
| ❌ **Quit** | Exit the background process |

---

## ⚙️ Configuration

Configuration is stored in JSON format at your platform's standard data directory:

| Platform | Config Path |
|----------|-------------|
| 🪟 **Windows** | `%APPDATA%\wallp\wallp.json` |
| 🐧 **Linux** | `~/.config/wallp/wallp.json` |
| 🍎 **macOS** | `~/Library/Application Support/wallp/wallp.json` |

### Example Configuration

```json
{
  "config": {
    "unsplash_access_key": "YOUR_UNSPLASH_ACCESS_KEY",
    "collections": [
      "1053828",
      "3330448",
      "894"
    ],
    "interval_minutes": 120,
    "aspect_ratio_tolerance": 0.1,
    "retention_days": 7
  },
  "state": {
    "is_running": true,
    "next_run_at": "2024-01-01T00:00:00Z",
    "last_run_at": "2024-01-01T00:00:00Z",
    "current_wallpaper_id": null,
    "current_history_index": 0
  },
  "history": []
}
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `unsplash_access_key` | string | — | Your Unsplash API access key (required) |
| `collections` | array | `["1053828", "3330448", "327760", "894"]` | Unsplash collection IDs to pull from |
| `interval_minutes` | integer | 120 | Auto-cycle interval (0 = disabled) |
| `aspect_ratio_tolerance` | float | 0.1 | Screen aspect ratio matching tolerance |
| `retention_days` | integer | 7 | Days to keep old wallpapers (0 = keep forever) |

---

## 🛠️ Development

### Prerequisites

- **Rust** 1.70+ ([Install](https://rustup.rs/))

### Platform-Specific Dependencies

| Platform | Dependencies |
|----------|--------------|
| 🪟 **Windows** | Visual Studio C++ Build Tools |
| 🐧 **Linux** | `libgtk-3-dev`, `libappindicator3-dev`, `xdotool`, `libxdo-dev` |
| 🍎 **macOS** | Xcode Command Line Tools, `libnotify` (optional) |

### Build Commands

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt
```

---

## 🐛 Troubleshooting

| Issue | Solution |
|-------|----------|
| **Build fails on Windows** | Install "Desktop development with C++" via Visual Studio Build Tools |
| **Build fails on Linux** | Install `libgtk-3-dev`, `libappindicator3-dev`, `xdotool`, `libxdo-dev` |
| **Build fails on macOS** | Install Xcode Command Line Tools: `xcode-select --install` |
| **System tray not visible** | Check if your desktop environment supports system tray icons |
| **API rate limit exceeded** | Ensure you have a valid Unsplash Access Key |
| **Wallpaper not changing** | Check if Wallp has permission to change desktop background |
| **macOS notifications not working** | Install libnotify: `brew install libnotify` |
| **First run doesn't start setup** | Run `wallp setup` manually |

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please ensure your code passes `cargo fmt` and `cargo clippy` before submitting.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<div align="center">

**Made with ❤️ and 🦀 Rust**

[Report Bug](https://github.com/joaocastilho/wallp/issues) · [Request Feature](https://github.com/joaocastilho/wallp/issues) · [Releases](https://github.com/joaocastilho/wallp/releases)

</div>
