# Wallp

**Wallp** is a lightweight, cross-platform (Windows, macOS, Linux) CLI and System Tray application that automatically manages and cycles through desktop wallpapers using the Unsplash API.

Written in **Rust** for performance, zero dependencies, and a minimal resource footprint.

## Features

- **System Tray Integration**: Manage wallpapers directly from your taskbar.
- **Smart History**: Undo/Redo wallpaper changes; keeps a history of your sessions.
- **Automatic Cycling**: Set an interval (e.g., every 2 hours) to get a fresh wallpaper.
- **Unsplash Powered**: High-quality headers from curated collections (Nature, Architecture, Minimal, Travel).
- **Cross-Platform**: Works natively on Windows, macOS, and Linux.
- **Autostart**: Automatically launches silently on system login.

## Installation

### Prerequisites
- **Rust Toolchain**: [Install Rust](https://rustup.rs/)
- **Build Tools**:
    - **Windows**: Visual Studio C++ Build Tools.
    - **Linux**: `libgtk-3-dev`, `libappindicator3-dev` (depending on distro).

### Build

```powershell
git clone https://github.com/your-username/wallp
cd wallp
cargo install --path .
```

The executable `wallp.exe` will be compiled to `target/release/` or installed to your Cargo bin path.

## Usage

### Initialization
First time setup? Run the wizard to configure your API key and preferences:

```powershell
wallp init
```

This will:
1. Prompt for your Unsplash Access Key.
2. Configure intervals and collections.
3. Enable autostart.
4. Launch the System Tray app.

### CLI Commands

Wallp functions as both a Tray app and a CLI controller.

| Command | Description |
| :--- | :--- |
| `wallp` | Starts the System Tray application (long-running). |
| `wallp init` | Runs the setup wizard. |
| `wallp next` | Smart forward: Goes to next history item OR fetches a new one. |
| `wallp prev` | Undo: Go back to the previous wallpaper. |
| `wallp new` | Force fetches a brand new wallpaper from Unsplash. |
| `wallp info` | Shows metadata about the current wallpaper. |
| `wallp open` | Opens the current wallpaper's Unsplash page in your browser. |
| `wallp folder` | Opens the local directory where wallpapers are saved. |
| `wallp status` | Checks if the background scheduler is running. |

### System Tray
- **✨ New Wallpaper**: Fetch a new random image.
- **⏭️ Next**: Go forward in history/new.
- **⏮️ Previous**: Go back in history.
- **📂 Open Folder**: View downloaded files.
- **❌ Quit**: Stop the background process.

## Configuration

Configuration is stored in `wallp.json` in your standard data directory:
- **Windows**: `%APPDATA%\wallp\wallp.json`
- **Linux**: `~/.local/share/wallp/wallp.json`
- **macOS**: `~/Library/Application Support/wallp/wallp.json`

### Example Config
```json
{
  "config": {
    "unsplash_access_key": "YOUR_ACCESS_KEY",
    "collections": [
      "1053828",
      "3330448"
    ],
    "interval_minutes": 120,
    "aspect_ratio_tolerance": 0.1,
    "retention_days": 7,
    "logging_enabled": true
  },
  "state": { ... },
  "history": [ ... ]
}
```

## Troubleshooting

**Logs**: If enabled, logs are found in the `logs/` subdirectory of the config folder.
**Build Errors on Windows**: Ensure you have "Desktop environment with C++" installed via Visual Studio Build Tools if you see errors regarding `link.exe` or `msvc`.
