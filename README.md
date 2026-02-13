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
2. Place it in a directory that's in your PATH (or run `wallp init` to auto-install)
3. Run `wallp init` to set up your Unsplash API key

#### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/joaocastilho/wallp
cd wallp

# Build and install
cargo install --path .
```

The executable will be available at `target/release/wallp` (or `wallp.exe` on Windows).

---

### First-Time Setup

Run the interactive wizard to configure your Unsplash API key and preferences:

```bash
wallp init
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
| `wallp` | Start the System Tray application (runs in background) | `wallp` |
| `wallp init` | Run the setup wizard | `wallp init` |
| `wallp next` | Go to next wallpaper (history-aware) | `wallp next` |
| `wallp prev` | Go to previous wallpaper | `wallp prev` |
| `wallp new` | Force fetch a brand new wallpaper | `wallp new` |
| `wallp info` | Show metadata for current wallpaper | `wallp info` |
| `wallp open` | Open current wallpaper in browser | `wallp open` |
| `wallp folder` | Open local wallpaper storage folder | `wallp folder` |
| `wallp status` | Check background scheduler status | `wallp status` |
| `wallp config` | View or edit configuration | `wallp config` |
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
| 📂 **Open Folder** | View downloaded wallpapers |
| ℹ️ **Info** | See current wallpaper details |
| 🔗 **Open in Browser** | View on Unsplash.com |
| ⏹️ **Pause/Resume** | Toggle automatic cycling |
| 🔃 **Autostart** | Toggle launch on login |
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
    "retention_days": 7,
    "logging_enabled": true,
    "autostart": true
  },
  "state": {
    "current_index": 0
  },
  "history": []
}
```

### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `unsplash_access_key` | string | — | Your Unsplash API access key (required) |
| `collections` | array | `[]` | Unsplash collection IDs to pull from |
| `interval_minutes` | integer | 120 | Auto-cycle interval (0 = disabled) |
| `aspect_ratio_tolerance` | float | 0.1 | Screen aspect ratio matching tolerance |
| `retention_days` | integer | 7 | Days to keep old wallpapers |
| `logging_enabled` | boolean | true | Enable debug logging |
| `autostart` | boolean | true | Launch on system startup |

---

## 🛠️ Development

### Prerequisites

- **Rust** 1.70+ ([Install](https://rustup.rs/))

### Platform-Specific Dependencies

| Platform | Dependencies |
|----------|--------------|
| 🪟 **Windows** | Visual Studio C++ Build Tools |
| 🐧 **Linux** | `libgtk-3-dev`, `libappindicator3-dev`, `xdotool` |
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

### Logs

Debug logs are stored in the `logs/` subdirectory of your config folder:

```bash
# Windows
%APPDATA%\wallp\logs\

# Linux
~/.config/wallp/logs/

# macOS
~/Library/Application Support/wallp/logs/
```

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
