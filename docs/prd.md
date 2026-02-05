# Wallp - Product Requirements Document (Rust Rewrite)

## 1. Overview
**Wallp** is a lightweight, single-executable CLI and System Tray application (Cross-Platform: Windows, macOS, Linux) that automatically manages and cycles through desktop wallpapers using the Unsplash API.

**Primary Goal**: Rewrite the existing TypeScript application in **Rust** to achieve a truly native, single-executable distribution with zero external dependencies and minimal resource footprint, compatible with all major OSs.

## 2. Architecture & Tech Stack
- **Language**: Rust (latest stable)
- **Distribution**: Single binary.

### 2.1. Critical Crates
- **System Tray**: `tray-icon` + `tao` (Tauri ecosystem). *Do not use legacy `systray` crates.*
    - *Requirement*: Helper binary must NOT be used. The application must run its own event loop.
- **Image Handling**: Download to a local cache directory; use `wallpaper` crate to set desktop background (cross-platform).
- **Autostart**: `auto-launch` crate (Registry based).
- **HTTP**: `reqwest` (async, JSON support).
- **Serialization**: `serde` + `serde_json`.
- **Async Runtime**: `tokio` (full features).
- **Logging**: `tracing` + `tracing-subscriber` + `tracing-appender` (for file rotation/non-blocking logging).
- **Paths**: `directories` (standard XDG/Roaming paths).
- **Locking**: `single-instance` (prevent multiple tray instances).

## 3. Data Storage
All application data is consolidated into a single file: `wallp.json`.

**Location**: Standard User Data Directory + `wallp` subdirectory.
- **Windows**: `%APPDATA%\wallp\`
- **macOS**: `~/Library/Application Support/wallp/`
- **Linux**: `~/.local/share/wallp/` (or `$XDG_DATA_HOME`)

**Directory Structure**:
- `{data_dir}\wallp.json` (Config/State/History)
- `{data_dir}\wallpapers\` (Image Cache)
- `{data_dir}\logs\` (Optional)

**Schema (`wallp.json`)**:
```json
{
  "config": {
    "unsplashAccessKey": "string",
    "collections": ["1053828", "3330448", "327760", "894"], // Default IDs: Nature, Architecture, Minimal, Travel
    "interval": 120, // Minutes (default)
    "aspectRatioTolerance": 0.1, // 10%
    "retentionDays": 7,
    "notifications": false,
    "loggingEnabled": false
  },
  "state": {
    "isRunning": true,
    "nextRunAt": "ISO-8601 Timestamp",
    "lastRunAt": "ISO-8601 Timestamp",
    "currentWallpaperId": "string",
    "currentHistoryIndex": 0 // Pointer for "Redo" functionality
  },
  "history": [
    {
      "id": "string",
      "filename": "string", // Relative to wallpapers dir
      "appliedAt": "ISO-8601 Timestamp"
    }
  ]
}
```

## 4. Functionality

### 4.1. Core Logic
- **Scheduler**: Checks every minute if `current_time >= nextRunAt`.
- **Defaults**:
    - Interval: 120 minutes (2 hours).
    - Collections:
        - `1053828`: Nature
        - `3330448`: Architecture
        - `327760`: Minimal
        - `894`: Travel
- **Global Toggle**: `config.loggingEnabled`. If false, no log files are written.
- **Unsplash API Logic**:
    - Endpoint: `/photos/random`
    - Params:
        - `collections`: Join `config.collections` with commas (e.g., "123,456").
        - `orientation`: `landscape`.
        - `client_id`: `config.unsplashAccessKey`.
        - `count`: 1.
- **Image Management**: 
    - Downloads images from Unsplash.
    - Saves with filename format: `wallpaper_{ID}.jpg`.
    - Prunes images older than `retentionDays`.

### 4.2. Smart Navigation (History)
- **Next Logic**:
    - If `currentHistoryIndex < history.length - 1`: Move forward (Redo). Restores wallpaper from local cache.
    - Else: Fetch **NEW** wallpaper from Unsplash, append to history, increment index.
- **Prev Logic**:
    - If `currentHistoryIndex > 0`: Move backward (Undo). Restores wallpaper from local cache.
    - Decrement index.
- **New Logic**:
    - Ignore current index/history.
    - Force fetch from Unsplash.
    - Append to history.
    - Set `currentHistoryIndex` to new end.

### 4.3. Initialization (`init`)
- **First Run**:
    - Create Data Directory (platform-specific) and `wallpapers` subdirectory.
    - Check for existing `wallp.json`.
    - If missing, create `wallp.json` with **Default Configuration** (see Schema).
    - Prompt user to open config file if API key is missing.
    - Enable **Autostart** (Platform-native method).
    - Launch Tray application.

### 4.4. Autostart
- Application must be able to register itself to start with the OS.
- **Method**: Utilizes `auto-launch` crate which handles:
    - **Windows**: Registry (`HKCU\...\Run`).
    - **macOS**: Launch Agent.
    - **Linux**: XDG Autostart (`~/.config/autostart`).
- **Requirement**: Must start *silently* (no visible console window) when triggered by autostart.

## 5. CLI Commands
The executable runs as a CLI tool.

| Command | Description |
| :--- | :--- |
| `wallp init` | Runs setup wizard, creates files, enables autostart, starts tray. |
| `wallp tray` | Starts the system tray process (long-running). |
| `wallp next` | Sets next wallpaper (Smart: History Redo or New). |
| `wallp new` | Force fetches a fresh wallpaper from Unsplash. |
| `wallp prev` | Sets previous wallpaper from history. |
| `wallp info` | Displays metadata of current wallpaper (Title, Author, Resolution, Location). |
| `wallp open` | Opens the current wallpaper's Unsplash page in browser. |
| `wallp folder` | Opens the local wallpapers directory in File Explorer. |
| `wallp config` | Subcommands: `edit` (open default editor), `set <key> <value>`. |
| `wallp status` | Shows running status, next scheduled run time. |
| `wallp list` | Lists recent wallpapers (date, description, author). |

## 6. System Tray Application
The tray application is the long-running process (`wallp tray`) that handles the scheduler and user interaction.

**Tray Menu Items**:
1.  **✨ New Wallpaper**: User action -> `wallp new` equivalent.
2.  **⏭️ Next**: User action -> `wallp next` equivalent.
3.  **⏮️ Previous**: User action -> `wallp prev` equivalent.
4.  `<Separator>`
5.  **🚀 Start on Login**: Checkbox/Toggle. Reflects current autostart status.
6.  `<Separator>`
7.  **ℹ️ View Info**: Opens URL in browser.
8.  **📂 Open Folder**: Opens local folder.
9.  **⚙️ Edit Config**: Opens `wallp.json`.
10. `<Separator>`
11. **❌ Quit**: Terminates the process.

**Icon**: Custom `.ico` file embedded in the executable.
**Tooltip**: "Wallp - Wallpaper Changer".



## 8. Technical Best Practices (Strict)
1.  **Strict Typing**: All configuration and API responses must be strongly typed with `serde`.
2.  **Error Handling**: Use `anyhow` or `thiserror` for comprehensive error context. Do not silence errors; log them to the file.
3.  **Async**: The main loop and all network/IO operations must be non-blocking (`tokio`).
4.  **Graceful Shutdown**: The application must handle `Ctrl+C` and Tray Quit signals gracefully, determining scheduler thread termination.
5.  **Logging**: check `config.loggingEnabled`.
    - If `true`: initialize `tracing_appender` with file rotation (daily or size-based) in `{data_dir}/logs`.
    - If `false`: initialize a no-op subscriber or only log to stdout (if in dev mode).
6.  **Single Instance**: Use `single-instance` crate to ensure only one process runs. If a second instance starts:
    - If it's `wallp tray`: Exit immediately (silent).
    - If it's a CLI command (e.g., `wallp next`): Execute the command and exit (allowed).
    - *Clarification*: The "single instance" rule applies primarily to the long-running Tray process. CLI commands are ephemeral and can run concurrently to trigger actions.
7.  **Metadata**: The generic `cargo build` is not enough. The final executable **must** have:
    - Embedded `.ico` icon.
    - Version info (`FileVersion`, `ProductVersion`).
    - Metadata (`ProductName`: "Wallp", `FileDescription`: "Wallp - Wallpaper Changer").
    - *Tool*: Use `winres` crate or `tauri-build` attributes to inject this resource file.
