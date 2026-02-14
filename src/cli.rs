use crate::config::AppData;
use crate::manager;
use crate::Commands;
use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, MultiSelect};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn get_exe_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "wallp.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "wallp"
    }
}

const MIN_INTERVAL_MINUTES: u64 = 30;

fn parse_interval(input: &str) -> Result<u64, String> {
    let input = input.trim().to_lowercase();
    if let Ok(minutes) = input.parse::<u64>() {
        return Ok(minutes);
    }
    let last_char = input.chars().last().ok_or("Empty input")?;
    let number_part = &input[..input.len() - 1];
    let value: u64 = number_part.parse().map_err(|_| "Invalid number")?;
    match last_char {
        'd' => Ok(value * 24 * 60),
        'h' => Ok(value * 60),
        'm' => Ok(value),
        's' => Ok((value as f64 / 60.0).ceil() as u64),
        _ => Err("Use: d (days), h (hours), m (minutes), s (seconds)".to_string()),
    }
}

fn get_default_collections_info() -> Vec<(String, String)> {
    vec![
        ("1065976".to_string(), "Wallpapers".to_string()),
        ("3330448".to_string(), "Nature".to_string()),
        ("894".to_string(), "Earth & Planets".to_string()),
    ]
}

pub fn is_initialized() -> bool {
    // Check if config file exists (primary indicator)
    if let Ok(config_path) = AppData::get_config_path()
        && config_path.exists()
    {
        return true;
    }
    false
}

#[allow(clippy::too_many_lines)]
pub fn setup_wizard() -> Result<()> {
    println!("Welcome to Wallp Setup Wizard!");
    println!("------------------------------");

    let current_exe = env::current_exe()?;

    // Platform-specific installation paths
    #[cfg(target_os = "linux")]
    let (install_dir, target_exe) = {
        let binary_dir = AppData::get_binary_dir()?;
        let target = binary_dir.join(get_exe_name());
        (binary_dir, target)
    };

    #[cfg(not(target_os = "linux"))]
    let (install_dir, target_exe) = {
        let data_dir = AppData::get_data_dir()?;
        let target = data_dir.join(get_exe_name());
        (data_dir, target)
    };

    // Ensure all necessary directories exist
    if !install_dir.exists() {
        fs::create_dir_all(&install_dir).context("Failed to create installation directory")?;
    }

    // Ensure config directory exists
    let config_dir = AppData::get_config_dir()?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
    }

    // Ensure data directory exists
    let data_dir = AppData::get_data_dir()?;
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).context("Failed to create data directory")?;
    }

    // Check if already initialized
    let config_path = AppData::get_config_path()?;
    let is_initialized = config_path.exists();

    // If already initialized, confirm before overwriting
    if is_initialized
        && !Confirm::new()
            .with_prompt("Wallp appears to be already installed. Run setup anyway?")
            .default(false)
            .interact()?
    {
        println!("Setup cancelled.");
        return Ok(());
    }

    // Copy to installation directory if not already there
    let current_exe_canonical = current_exe.canonicalize().unwrap_or(current_exe.clone());
    let target_exe_canonical = target_exe.canonicalize().ok();

    let is_installed = target_exe_canonical.is_some_and(|t| t == current_exe_canonical);

    let final_exe_path = if is_installed {
        println!("ℹ️  Already running from installation directory.");
        current_exe
    } else {
        println!("Installing Wallp to {}", target_exe.display());
        match fs::copy(&current_exe, &target_exe) {
            Ok(_) => {
                #[cfg(target_os = "linux")]
                println!("✅ Copied executable to ~/.local/bin/.");
                #[cfg(target_os = "windows")]
                println!("✅ Copied executable to Local AppData.");
                #[cfg(target_os = "macos")]
                println!("✅ Copied executable to Application Support.");

                // Give the filesystem a moment to settle
                std::thread::sleep(std::time::Duration::from_millis(500));
                target_exe
            }
            Err(e) => {
                println!("⚠️  Failed to copy executable: {e}. Proceeding with current executable.");
                current_exe
            }
        }
    };

    println!();

    // Canonicalize the final path
    let final_exe_path = final_exe_path.canonicalize().unwrap_or(final_exe_path);

    let mut app_data = AppData::load()?; // Load existing or default

    println!();
    println!("📋 Configuration");
    println!("Get your API key at: https://unsplash.com/oauth/applications");

    let access_key: String = Input::new()
        .with_prompt("Unsplash Access Key")
        .default(app_data.config.unsplash_access_key.clone())
        .interact()
        .context("Failed to get access key")?;

    // Parse interval with validation
    let interval = loop {
        let input: String = Input::new()
            .with_prompt("Update Interval (e.g., 1d, 12h, 30m, 500s, or minutes)")
            .default(app_data.config.interval_minutes.to_string())
            .interact()
            .context("Failed to get interval")?;

        match parse_interval(&input) {
            Ok(minutes) if minutes >= MIN_INTERVAL_MINUTES => break minutes,
            Ok(_) => {
                println!("Interval must be at least 30 minutes to avoid exceeding Unsplash's rate limit of 50 requests per day");
            }
            Err(e) => {
                println!("Invalid input: {}", e);
            }
        }
    };

    println!();

    // Collection selection with checkboxes
    let default_collections = get_default_collections_info();
    let current_collections = app_data.config.collections.clone();

    // Prepare items for MultiSelect
    let items: Vec<String> = default_collections
        .iter()
        .map(|(id, desc)| format!("{} - {}", desc, id))
        .collect();

    // Determine which items are currently selected
    let defaults: Vec<bool> = default_collections
        .iter()
        .map(|(id, _)| current_collections.contains(id))
        .collect();

    println!("Select collections (Space to toggle, Enter to confirm):");
    println!("Find more at: https://unsplash.com/collections");
    println!();

    let selections = MultiSelect::new()
        .items(&items)
        .defaults(&defaults)
        .interact()
        .context("Failed to select collections")?;

    let collections: Vec<String> = selections
        .iter()
        .map(|&idx| default_collections[idx].0.clone())
        .collect();

    println!();
    println!("🔧 System Integration");

    let enable_autostart = Confirm::new()
        .with_prompt("Enable Autostart on Login?")
        .default(true)
        .interact()
        .context("Failed to get autostart confirmation")?;

    // Update config
    app_data.config.unsplash_access_key = access_key;
    app_data.config.interval_minutes = interval;
    app_data.config.collections = collections;
    app_data.save()?;

    // Setup Autostart
    if enable_autostart {
        setup_autostart(true, &final_exe_path)?;
        println!("✅ Autostart enabled.");
    } else {
        setup_autostart(false, &final_exe_path)?;
        println!("ℹ️ Autostart disabled.");
    }

    // Add to PATH (Linux only - binary is already in PATH location)
    #[cfg(target_os = "linux")]
    if Confirm::new()
        .with_prompt("Add ~/.local/bin to PATH?")
        .default(true)
        .interact()?
    {
        add_local_bin_to_path()?;
    }

    // Add to PATH (Windows/macOS - add install directory)
    #[cfg(target_os = "windows")]
    if Confirm::new()
        .with_prompt("Add Wallp directory to system PATH?")
        .default(true)
        .interact()?
    {
        add_to_path_windows(&final_exe_path)?;
    }

    #[cfg(target_os = "macos")]
    if Confirm::new()
        .with_prompt("Add Wallp directory to PATH?")
        .default(true)
        .interact()?
    {
        add_to_path_unix(&final_exe_path)?;
    }

    println!();
    println!("✅ Wallp installed successfully!");
    println!("\nUsage:");
    println!("  wallp new     - Get new wallpaper");
    println!("  wallp next    - Next wallpaper");
    println!("  wallp prev    - Previous wallpaper");
    println!("  wallp --help  - See all commands");
    if !is_installed && final_exe_path != env::current_exe()? {
        println!("\nYou can delete this installer file.");
    }

    println!();

    // Launch Tray App
    if Confirm::new()
        .with_prompt("Start Wallp now?")
        .default(true)
        .interact()?
    {
        start_background_process(&final_exe_path)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn add_to_path_windows(exe_path: &Path) -> Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let install_dir = exe_path
        .parent()
        .context("Failed to get executable directory")?;
    let install_dir_str = install_dir.to_str().context("Invalid path")?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (env, _) = hkcu.create_subkey("Environment")?; // Create or open
    let path_val: String = env.get_value("Path").unwrap_or_default();

    // Check if already in PATH
    let paths: Vec<&str> = path_val.split(';').collect();
    if paths
        .iter()
        .any(|p| p.eq_ignore_ascii_case(install_dir_str))
    {
        println!("ℹ️ Directory already in PATH.");
        return Ok(());
    }

    // Append
    let new_path = if path_val.is_empty() {
        install_dir_str.to_string()
    } else {
        format!("{path_val};{install_dir_str}")
    };

    env.set_value("Path", &new_path)?;
    println!("✅ Added {install_dir_str} to PATH.");

    #[cfg(target_os = "windows")]
    {
        use std::ffi::CString;
        let param =
            CString::new("Environment").context("Failed to create CString for broadcast")?;
        // SAFETY: SendMessageTimeoutA is a Windows API that broadcasts a message to all top-level windows.
        // The LPARAM is a valid pointer to a null-terminated CString that lives for the duration of the call.
        // This is the standard Windows mechanism for notifying applications of environment changes.
        unsafe {
            let result = windows::Win32::UI::WindowsAndMessaging::SendMessageTimeoutA(
                windows::Win32::UI::WindowsAndMessaging::HWND_BROADCAST,
                windows::Win32::UI::WindowsAndMessaging::WM_SETTINGCHANGE,
                windows::Win32::Foundation::WPARAM(0),
                windows::Win32::Foundation::LPARAM(param.as_ptr() as isize),
                windows::Win32::UI::WindowsAndMessaging::SMTO_ABORTIFHUNG,
                5000,
                None,
            );
            if result.0 == 0 {
                eprintln!("Warning: Could not notify system of PATH change");
            }
        }
        println!("ℹ️ System notified of PATH change.");
    }

    Ok(())
}

#[cfg(not(target_family = "unix"))]
#[allow(dead_code)]
fn add_to_path_unix(_exe_path: &Path) {
    // Stub for non-unix platforms
}

#[allow(dead_code)]
fn get_shell_name() -> &'static str {
    let shell = std::env::var("SHELL")
        .map(|s| if s.contains("zsh") { "zsh" } else { "bash" })
        .unwrap_or("bash");

    // Validate shell exists
    let shell_paths = ["/bin", "/usr/bin", "/usr/local/bin"];
    let shell_exists = shell_paths
        .iter()
        .any(|path| PathBuf::from(format!("{path}/{shell}")).exists());

    if shell_exists {
        shell
    } else {
        "sh"
    }
}

#[cfg(test)]
pub fn get_shell_files(shell: &str) -> (String, String) {
    if shell == "zsh" {
        (".zshrc".to_string(), ".zprofile".to_string())
    } else {
        (".bashrc".to_string(), ".bash_profile".to_string())
    }
}

#[allow(dead_code)]
fn shell_escape(s: &str) -> String {
    s.replace('"', "\\\"").replace('$', "\\$")
}

#[cfg(target_os = "windows")]
fn powershell_escape(s: &str) -> String {
    // PowerShell escaping: escape quotes, backticks, and dollar signs
    s.replace('"', "\"\"").replace('`', "``").replace('$', "`$")
}

#[cfg(test)]
pub fn create_export_line(install_dir: &str) -> String {
    let escaped = shell_escape(install_dir);
    format!(r#"export PATH="$PATH:{escaped}""#)
}

#[cfg(test)]
pub fn add_path_to_profile_content(content: &str, install_dir: &str) -> String {
    let export_line = create_export_line(install_dir);
    // Use exact line matching to avoid false positives
    if content.lines().any(|line| line.trim() == export_line) {
        return content.to_string();
    }
    format!("{content}\n# Wallp\n{export_line}\n")
}

#[cfg(test)]
pub fn remove_path_from_profile_content(content: &str, install_dir: &str) -> String {
    let export_line = create_export_line(install_dir);
    content
        .lines()
        .filter(|line| line.trim() != export_line && !line.contains("# Wallp"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
pub fn is_path_in_profile(content: &str, install_dir: &str) -> bool {
    let export_line = create_export_line(install_dir);
    content.lines().any(|line| line.trim() == export_line)
}

#[cfg(unix)]
fn add_to_path_unix(exe_path: &Path) -> Result<()> {
    use std::io::Write;

    let install_dir = exe_path
        .parent()
        .context("Failed to get executable directory")?;
    let install_dir_str = install_dir.to_str().context("Invalid path")?;
    let escaped_path = shell_escape(install_dir_str);

    let shell = get_shell_name();
    let (rc_file, profile_file) = if shell == "zsh" {
        (".zshrc".to_string(), ".zprofile".to_string())
    } else {
        (".bashrc".to_string(), ".bash_profile".to_string())
    };

    let base_dirs = directories::BaseDirs::new().context("Failed to get home directory")?;
    let home_dir = base_dirs.home_dir();

    let export_line = format!(r#"export PATH="$PATH:{escaped_path}""#);

    for profile_name in &[&rc_file, &profile_file] {
        let profile_path = home_dir.join(profile_name);

        // Check permissions if file exists
        if profile_path.exists() {
            let metadata = fs::metadata(&profile_path)?;
            if metadata.permissions().readonly() {
                println!("⚠️  Profile {profile_name} is read-only, skipping");
                continue;
            }
        }

        let profile_content = if profile_path.exists() {
            fs::read_to_string(&profile_path).unwrap_or_default()
        } else {
            String::new()
        };

        // Use exact line matching to avoid false positives
        if profile_content
            .lines()
            .any(|line| line.trim() == export_line)
        {
            println!("ℹ️ Directory already in PATH ({profile_name})");
            continue;
        }

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&profile_path)
            .context(format!("Failed to open {profile_name}"))?;

        writeln!(file, "\n# Wallp\nexport PATH=\"$PATH:{escaped_path}\"")
            .context(format!("Failed to write to {profile_name}"))?;
    }

    println!("✅ Added {install_dir_str} to PATH.");
    println!("ℹ️ Restart your terminal or run 'source {rc_file}' to apply changes.");

    Ok(())
}

#[cfg(target_os = "linux")]
fn add_local_bin_to_path() -> Result<()> {
    use std::io::Write;

    let binary_dir = AppData::get_binary_dir()?;
    let binary_dir_str = binary_dir.to_str().context("Invalid path")?;
    let escaped_path = shell_escape(binary_dir_str);

    let shell = get_shell_name();
    let (rc_file, profile_file) = if shell == "zsh" {
        (".zshrc".to_string(), ".zprofile".to_string())
    } else {
        (".bashrc".to_string(), ".bash_profile".to_string())
    };

    let base_dirs = directories::BaseDirs::new().context("Failed to get home directory")?;
    let home_dir = base_dirs.home_dir();

    let export_line = format!(r#"export PATH="$PATH:{escaped_path}"")"#);

    for profile_name in &[&rc_file, &profile_file] {
        let profile_path = home_dir.join(profile_name);

        // Check permissions if file exists
        if profile_path.exists() {
            let metadata = fs::metadata(&profile_path)?;
            if metadata.permissions().readonly() {
                println!("⚠️  Profile {profile_name} is read-only, skipping");
                continue;
            }
        }

        let profile_content = if profile_path.exists() {
            fs::read_to_string(&profile_path).unwrap_or_default()
        } else {
            String::new()
        };

        // Use exact line matching to avoid false positives
        if profile_content
            .lines()
            .any(|line| line.trim() == export_line)
        {
            println!("ℹ️ Directory already in PATH ({profile_name})");
            continue;
        }

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&profile_path)
            .context(format!("Failed to open {profile_name}"))?;

        writeln!(file, "\n# Wallp\nexport PATH=\"$PATH:{escaped_path}\"")
            .context(format!("Failed to write to {profile_name}"))?;
    }

    println!("✅ Added {binary_dir_str} to PATH.");
    println!("ℹ️ Restart your terminal or run 'source {rc_file}' to apply changes.");

    Ok(())
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
fn add_local_bin_to_path() -> Result<()> {
    anyhow::bail!("add_local_bin_to_path is only applicable on Linux")
}

#[cfg(target_os = "macos")]
fn build_auto_launch(app_path: &str) -> Result<auto_launch::AutoLaunch> {
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Wallp")
        .set_app_path(app_path)
        .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build auto_launch: {e}"))
}

#[cfg(not(target_os = "macos"))]
fn build_auto_launch(app_path: &str) -> Result<auto_launch::AutoLaunch> {
    auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Wallp")
        .set_app_path(app_path)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build auto_launch: {e}"))
}

pub fn setup_autostart(enable: bool, exe_path: &Path) -> Result<()> {
    let app_path = exe_path
        .to_str()
        .context("Failed to get executable path as string")?;

    let auto = build_auto_launch(app_path)?;

    if enable {
        auto.enable()
            .map_err(|e| anyhow::anyhow!("Failed to enable autostart: {e}"))?;
    } else {
        auto.disable()
            .map_err(|e| anyhow::anyhow!("Failed to disable autostart: {e}"))?;
    }
    Ok(())
}

fn start_background_process(exe_path: &Path) -> Result<()> {
    let mut cmd = Command::new(exe_path);

    // Detach process on Windows to ensure it survives console close and doesn't inherit console
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(DETACHED_PROCESS);
    }

    cmd.spawn().context("Failed to start background process")?;

    println!("🚀 Wallp started in background.");
    Ok(())
}

pub fn handle_command(cmd: &Commands) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    match cmd {
        Commands::Setup => {
            setup_wizard()?;
        }
        Commands::New => {
            rt.block_on(manager::new())?;
            println!("✨ New wallpaper set.");
        }
        Commands::Next => {
            rt.block_on(manager::next())?;
            println!("⏩ Next wallpaper set.");
        }
        Commands::Prev => {
            rt.block_on(manager::prev())?;
            println!("⏪ Previous wallpaper set.");
        }
        Commands::Status => {
            let data = AppData::load()?;
            println!(
                "Status: {}",
                if data.state.is_running {
                    "Running"
                } else {
                    "Stopped"
                }
            );
            println!("Next Run: {}", data.state.next_run_at);
            println!("Last Run: {}", data.state.last_run_at);
            println!(
                "Current Wallpaper ID: {:?}",
                data.state.current_wallpaper_id
            );
        }
        Commands::Info => {
            if let Some(w) = manager::get_current_wallpaper()? {
                println!("Title: {}", w.title.unwrap_or_default());
                println!("Author: {}", w.author.unwrap_or_default());
                println!("ID: {}", w.id);
            } else {
                println!("No wallpaper in history.");
            }
        }
        Commands::Open => {
            if let Some(w) = manager::get_current_wallpaper()? {
                if let Some(url) = w.url {
                    open::that(url)?;
                } else {
                    println!("No URL available.");
                }
            }
        }
        Commands::Folder => {
            let path = AppData::get_data_dir()?.join("wallpapers");
            open::that(path)?;
        }
        Commands::Config => {
            let path = AppData::get_config_path()?;
            open::that(path)?;
        }
        Commands::List => {
            let data = AppData::load()?;
            for (i, w) in data.history.iter().rev().take(5).enumerate() {
                println!(
                    "{}: {} by {}",
                    i,
                    w.title.clone().unwrap_or_default(),
                    w.author.clone().unwrap_or_default()
                );
            }
        }
        Commands::Uninstall => handle_uninstall()?,
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn handle_uninstall() -> Result<()> {
    println!("[WARNING] This will permanently remove Wallp and all associated data:");
    println!("          - Remove from system startup");
    println!("          - Delete configuration and wallpaper history");
    println!("          - Remove from PATH environment variable");
    println!();

    if !Confirm::new()
        .with_prompt("Are you sure you want to uninstall Wallp?")
        .default(false)
        .interact()?
    {
        println!();
        println!("Uninstall cancelled.");
        return Ok(());
    }

    println!();
    println!("Uninstalling Wallp...");
    println!("[  OK  ] Stopped background processes");
    // Kill other wallp instances (Tray app)
    #[cfg(target_os = "windows")]
    {
        let my_pid = std::process::id();
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "wallp.exe", "/FI", &format!("PID ne {my_pid}")])
            .output();
    }
    #[cfg(unix)]
    {
        // Use exact match to avoid killing unrelated processes
        let _ = Command::new("pkill").args(["-x", "wallp"]).output();
    }

    // Remove from autostart using the appropriate paths for each platform
    #[cfg(target_os = "linux")]
    {
        if let Ok(binary_dir) = AppData::get_binary_dir() {
            let installed_exe = binary_dir.join("wallp");
            let _ = setup_autostart(false, &installed_exe);
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        if let Ok(data_dir) = AppData::get_data_dir() {
            let exe_name = if cfg!(target_os = "windows") {
                "wallp.exe"
            } else {
                "wallp"
            };
            let installed_exe = data_dir.join(exe_name);
            let _ = setup_autostart(false, &installed_exe);
        }
    }
    // Also try current exe just in case
    if let Ok(current_exe) = env::current_exe() {
        let _ = setup_autostart(false, &current_exe);
    }

    #[cfg(target_os = "windows")]
    {
        if let Err(e) = remove_from_path_windows() {
            println!("[FAILED] Failed to remove from PATH: {e}");
        } else {
            println!("[  OK  ] Removed from PATH");
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Err(e) = remove_local_bin_from_path() {
            println!("[FAILED] Failed to remove from PATH: {e}");
        } else {
            println!("[  OK  ] Removed from PATH");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Err(e) = remove_from_path_unix() {
            println!("[FAILED] Failed to remove from PATH: {e}");
        } else {
            println!("[  OK  ] Removed from PATH");
        }
    }

    // Platform-specific cleanup
    let current_exe = env::current_exe()?;

    #[cfg(target_os = "linux")]
    {
        if let Ok(binary_dir) = AppData::get_binary_dir() {
            let binary_path = binary_dir.join("wallp");
            if binary_path.exists() {
                match std::fs::remove_file(&binary_path) {
                    Ok(_) => println!("[  OK  ] Removed binary"),
                    Err(e) => println!("[FAILED] Failed to remove binary: {e}"),
                }
            }
        }

        if let Ok(config_dir) = AppData::get_config_dir() {
            if config_dir.exists() {
                match std::fs::remove_dir_all(&config_dir) {
                    Ok(_) => println!("[  OK  ] Removed configuration"),
                    Err(e) => println!("[FAILED] Failed to remove configuration: {e}"),
                }
            }
        }

        if let Ok(data_dir) = AppData::get_data_dir() {
            if data_dir.exists() {
                match std::fs::remove_dir_all(&data_dir) {
                    Ok(_) => println!("[  OK  ] Removed data directory"),
                    Err(e) => println!("[FAILED] Failed to remove data directory: {e}"),
                }
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        if let Ok(data_dir) = AppData::get_data_dir()
            && data_dir.exists()
        {
            match std::fs::remove_dir_all(&data_dir) {
                Ok(_) => println!("[  OK  ] Removed data and configuration"),
                Err(e) => println!("[FAILED] Failed to remove data directory: {e}"),
            }
        }
    }

    // Check if running from installation directory
    let is_running_from_install = if let Ok(data_dir) = AppData::get_data_dir() {
        let data_dir_canonical = data_dir.canonicalize().unwrap_or_else(|_| data_dir.clone());
        let current_exe_canonical = current_exe
            .canonicalize()
            .unwrap_or_else(|_| current_exe.clone());
        current_exe_canonical.starts_with(&data_dir_canonical)
    } else {
        false
    };

    #[cfg(target_os = "linux")]
    let is_running_from_install = is_running_from_install || {
        if let Ok(binary_dir) = AppData::get_binary_dir() {
            let binary_dir_canonical = binary_dir
                .canonicalize()
                .unwrap_or_else(|_| binary_dir.clone());
            let current_exe_canonical = current_exe
                .canonicalize()
                .unwrap_or_else(|_| current_exe.clone());
            current_exe_canonical.starts_with(&binary_dir_canonical)
        } else {
            false
        }
    };

    if is_running_from_install {
        // Self-delete: spawn to delete exe after we exit
        println!("[INFO] Running from installation directory - scheduling self-deletion");

        let exe_path = current_exe.display().to_string();

        #[cfg(target_os = "windows")]
        {
            let escaped_exe_path = powershell_escape(&exe_path);
            let ps_script = format!(
                r#"Start-Sleep -Seconds 2; Set-Location $env:TEMP; Remove-Item -LiteralPath "{escaped_exe_path}" -Force -ErrorAction SilentlyContinue"#
            );
            let _ = Command::new("powershell")
                .args(["-WindowStyle", "Hidden", "-Command", &ps_script])
                .spawn();
        }

        #[cfg(unix)]
        {
            let escaped_exe_path = shell_escape(&exe_path);
            let script = format!(
                r#"for i in 1 2 3 4 5; do
  sleep 1
  if rm -f "{escaped_exe_path}" 2>/dev/null; then
    break
  fi
done"#
            );
            let _ = Command::new("sh").args(["-c", &script]).spawn();
        }

        println!();
        println!("Uninstall complete. The executable will be removed shortly.");
        std::process::exit(0);
    }

    println!();
    println!("Uninstall complete. You can now delete this executable.");
    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_from_path_windows() -> Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    // Remove BOTH current dir and installed dir if present, just to be sure
    let current_exe = env::current_exe()?;
    let current_dir = current_exe.parent().unwrap_or_else(|| Path::new(""));

    let data_dir = AppData::get_data_dir()?; // roaming/wallp

    let paths_to_remove = [current_dir.to_path_buf(), data_dir];

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (env, _) = hkcu.create_subkey("Environment")?;
    let path_val: String = env.get_value("Path").unwrap_or_default();

    let mut paths: Vec<&str> = path_val.split(';').collect();
    let original_len = paths.len();

    paths.retain(|p| {
        let p_path = PathBuf::from(p);
        !paths_to_remove
            .iter()
            .any(|r| p_path == *r || p.eq_ignore_ascii_case(r.to_str().unwrap_or("")))
            && !p.is_empty()
    });

    if paths.len() == original_len {
        println!("ℹ️ Directory was not in PATH.");
        return Ok(());
    }

    let new_path = paths.join(";");
    env.set_value("Path", &new_path)?;
    println!("✅ Removed from PATH.");

    #[cfg(target_os = "windows")]
    {
        use std::ffi::CString;
        if let Ok(param) = CString::new("Environment") {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::SendMessageTimeoutA(
                    windows::Win32::UI::WindowsAndMessaging::HWND_BROADCAST,
                    windows::Win32::UI::WindowsAndMessaging::WM_SETTINGCHANGE,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(param.as_ptr() as isize),
                    windows::Win32::UI::WindowsAndMessaging::SMTO_ABORTIFHUNG,
                    5000,
                    None,
                );
            }
        }
    }

    Ok(())
}

#[cfg(unix)]
fn remove_from_path_unix() -> Result<()> {
    let data_dir = AppData::get_data_dir()?;
    let install_dir_str = data_dir.to_str().context("Invalid path")?;
    let escaped_path = shell_escape(install_dir_str);

    let shell = get_shell_name();
    let (rc_file, profile_file) = if shell == "zsh" {
        (".zshrc".to_string(), ".zprofile".to_string())
    } else {
        (".bashrc".to_string(), ".bash_profile".to_string())
    };

    let base_dirs = directories::BaseDirs::new().context("Failed to get home directory")?;
    let home_dir = base_dirs.home_dir().to_path_buf();

    let export_line = format!(r#"export PATH="$PATH:{escaped_path}""#);

    for profile_name in &[&rc_file, &profile_file] {
        let profile_path = home_dir.join(profile_name);
        if !profile_path.exists() {
            continue;
        }

        let profile_content =
            fs::read_to_string(&profile_path).context("Failed to read shell profile")?;

        // Use exact line matching to avoid false positives
        if !profile_content
            .lines()
            .any(|line| line.trim() == export_line)
        {
            continue;
        }

        let new_content: String = profile_content
            .lines()
            .filter(|line| line.trim() != export_line && !line.contains("# Wallp"))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(&profile_path, new_content).context("Failed to write shell profile")?;
    }

    println!("✅ Removed from PATH.");
    println!("ℹ️ Restart your terminal or run 'source {rc_file}' to apply changes.");

    Ok(())
}

#[cfg(target_os = "linux")]
fn remove_local_bin_from_path() -> Result<()> {
    let binary_dir = AppData::get_binary_dir()?;
    let binary_dir_str = binary_dir.to_str().context("Invalid path")?;
    let escaped_path = shell_escape(binary_dir_str);

    let shell = get_shell_name();
    let (rc_file, profile_file) = if shell == "zsh" {
        (".zshrc".to_string(), ".zprofile".to_string())
    } else {
        (".bashrc".to_string(), ".bash_profile".to_string())
    };

    let base_dirs = directories::BaseDirs::new().context("Failed to get home directory")?;
    let home_dir = base_dirs.home_dir().to_path_buf();

    let export_line = format!(r#"export PATH="$PATH:{escaped_path}""#);

    for profile_name in &[&rc_file, &profile_file] {
        let profile_path = home_dir.join(profile_name);
        if !profile_path.exists() {
            continue;
        }

        let profile_content =
            fs::read_to_string(&profile_path).context("Failed to read shell profile")?;

        // Use exact line matching to avoid false positives
        if !profile_content
            .lines()
            .any(|line| line.trim() == export_line)
        {
            continue;
        }

        let new_content: String = profile_content
            .lines()
            .filter(|line| line.trim() != export_line && !line.contains("# Wallp"))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(&profile_path, new_content).context("Failed to write shell profile")?;
    }

    println!("✅ Removed from PATH.");
    println!("ℹ️ Restart your terminal or run 'source {rc_file}' to apply changes.");

    Ok(())
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
fn remove_local_bin_from_path() -> Result<()> {
    anyhow::bail!("remove_local_bin_from_path is only applicable on Linux")
}

#[cfg(not(target_family = "unix"))]
#[allow(dead_code)]
fn remove_from_path_unix() {
    // Stub for non-unix platforms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_shell_files_bash() {
        let (rc, profile) = get_shell_files("bash");
        assert_eq!(rc, ".bashrc");
        assert_eq!(profile, ".bash_profile");
    }

    #[test]
    fn test_get_shell_files_zsh() {
        let (rc, profile) = get_shell_files("zsh");
        assert_eq!(rc, ".zshrc");
        assert_eq!(profile, ".zprofile");
    }

    #[test]
    fn test_create_export_line() {
        let line = create_export_line("/home/user/.config/wallp");
        assert_eq!(line, r#"export PATH="$PATH:/home/user/.config/wallp""#);
    }

    #[test]
    fn test_create_export_line_with_spaces() {
        let line = create_export_line("/home/user/My Documents/wallp");
        assert_eq!(line, r#"export PATH="$PATH:/home/user/My Documents/wallp""#);
    }

    #[test]
    fn test_add_path_to_profile_empty() {
        let result = add_path_to_profile_content("", "/home/user/.config/wallp");
        assert!(result.contains(r#"export PATH="$PATH:/home/user/.config/wallp""#));
        assert!(result.contains("# Wallp"));
    }

    #[test]
    fn test_add_path_to_profile_existing() {
        let existing = r#"export PATH="$PATH:/usr/bin"
export EDITOR=vim"#;
        let result = add_path_to_profile_content(existing, "/home/user/.config/wallp");
        assert!(result.contains(r#"export PATH="$PATH:/usr/bin""#));
        assert!(result.contains(r#"export PATH="$PATH:/home/user/.config/wallp""#));
        assert!(result.contains("# Wallp"));
    }

    #[test]
    fn test_add_path_to_profile_already_exists() {
        let existing = r#"export PATH="$PATH:/home/user/.config/wallp"
export EDITOR=vim"#;
        let result = add_path_to_profile_content(existing, "/home/user/.config/wallp");
        let count = result
            .matches("export PATH=\"$PATH:/home/user/.config/wallp\"")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_remove_path_from_profile() {
        let existing = r#"export PATH="$PATH:/usr/bin"
# Wallp
export PATH="$PATH:/home/user/.config/wallp"
export EDITOR=vim"#;
        let result = remove_path_from_profile_content(existing, "/home/user/.config/wallp");
        assert!(!result.contains("/home/user/.config/wallp"));
        assert!(!result.contains("# Wallp"));
        assert!(result.contains("/usr/bin"));
        assert!(result.contains("EDITOR=vim"));
    }

    #[test]
    fn test_remove_path_not_present() {
        let existing = r#"export PATH="$PATH:/usr/bin"
export EDITOR=vim"#;
        let result = remove_path_from_profile_content(existing, "/home/user/.config/wallp");
        assert_eq!(result, existing);
    }

    #[test]
    fn test_is_path_in_profile_true() {
        let content = r#"export PATH="$PATH:/home/user/.config/wallp"
export EDITOR=vim"#;
        assert!(is_path_in_profile(content, "/home/user/.config/wallp"));
    }

    #[test]
    fn test_is_path_in_profile_false() {
        let content = r#"export PATH="$PATH:/usr/bin"
export EDITOR=vim"#;
        assert!(!is_path_in_profile(content, "/home/user/.config/wallp"));
    }

    #[test]
    fn test_is_path_in_profile_partial_match() {
        let content = r#"export PATH="$PATH:/home/user/.config/wallp2""#;
        assert!(!is_path_in_profile(content, "/home/user/.config/wallp"));
    }

    #[test]
    fn test_path_with_spaces() {
        let line = create_export_line("/home/user/My Documents/wallp");
        let content = "";
        let result = add_path_to_profile_content(content, "/home/user/My Documents/wallp");
        assert!(result.contains(&line));
        // Verify the space is in the path (not escaped as it's within quotes)
        assert!(line.contains("My Documents"));
    }

    #[test]
    fn test_path_with_quotes() {
        // Test that quotes in path are escaped
        let escaped = shell_escape("/path/with\"quote");
        assert_eq!(escaped, r#"/path/with\"quote"#);
    }

    #[test]
    fn test_path_with_dollar() {
        // Test that dollar signs in path are escaped
        let escaped = shell_escape("/path/$HOME/wallp");
        assert_eq!(escaped, r"/path/\$HOME/wallp");
    }

    #[test]
    fn test_multiple_wallp_entries() {
        let existing = r#"# Wallp
export PATH="$PATH:/home/user/.config/wallp"
# Wallp
export PATH="$PATH:/home/user/.config/wallp"
export EDITOR=vim"#;
        let result = remove_path_from_profile_content(existing, "/home/user/.config/wallp");
        assert!(!result.contains("/home/user/.config/wallp"));
        assert!(!result.contains("# Wallp"));
        assert!(result.contains("EDITOR=vim"));
    }

    #[test]
    fn test_shell_escape_preserves_slashes() {
        let escaped = shell_escape("/home/user/.config/wallp");
        assert!(escaped.contains("/home/user/.config/wallp"));
    }
}
