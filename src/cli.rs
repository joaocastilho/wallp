use crate::config::AppData;
use crate::manager;
use crate::{Commands, ConfigAction};
use anyhow::{Context, Result};
use dialoguer::{Confirm, Input};
use std::env;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(windows)]
use windows::Win32::Foundation::WPARAM;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutA, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};

pub fn init_wizard() -> Result<()> {
    println!("Welcome to Wallp Setup Wizard!");
    println!("------------------------------");

    let current_exe = env::current_exe()?;
    let data_dir = AppData::get_data_dir()?;

    // Check if already initialized (data dir exists with config)
    let is_initialized = data_dir.exists() && AppData::get_config_path()?.exists();

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

    // Ensure data directory exists
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).context("Failed to create data directory")?;
    }

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

    let target_exe = data_dir.join(get_exe_name());

    // Copy to AppData if not already there
    let current_exe_canonical = current_exe.canonicalize().unwrap_or(current_exe.clone());
    let target_exe_canonical = target_exe.canonicalize().ok();

    let is_installed = target_exe_canonical.is_some_and(|t| t == current_exe_canonical);

    let final_exe_path = if !is_installed {
        println!("Installing Wallp to {}", target_exe.display());
        // Copy current exe to target
        // We might fail if target is running (shouldn't be, if we are in init)
        // or permission issues.
        match fs::copy(&current_exe, &target_exe) {
            Ok(_) => {
                println!("✅ Copied executable to AppData.");
                // Give the filesystem a moment to settle/scan the new file so metadata is available for reading
                std::thread::sleep(std::time::Duration::from_millis(500));
                target_exe
            }
            Err(e) => {
                println!(
                    "⚠️  Failed to copy executable: {}. Proceeding with current executable.",
                    e
                );
                current_exe
            }
        }
    } else {
        println!("ℹ️  Already running from installation directory.");
        current_exe
    };

    // Canonicalize the final path to ensure we have the absolute system path.
    // This helps with registry keys and ensuring the file is correctly identified.
    let final_exe_path = final_exe_path.canonicalize().unwrap_or(final_exe_path);

    let mut app_data = AppData::load()?; // Load existing or default

    let access_key: String = Input::new()
        .with_prompt("Unsplash Access Key")
        .default(app_data.config.unsplash_access_key.clone())
        .interact()
        .context("Failed to get access key")?;

    let interval: u64 = Input::new()
        .with_prompt("Update Interval (minutes)")
        .default(app_data.config.interval_minutes)
        .interact()
        .context("Failed to get interval")?;

    // Default collections
    let collections_str = app_data.config.collections.join(",");
    let collections_input: String = Input::new()
        .with_prompt("Unsplash Collection IDs (comma separated)")
        .default(collections_str)
        .interact()
        .context("Failed to get collections")?;

    let collections: Vec<String> = collections_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

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

    // Add to PATH
    if cfg!(target_os = "windows")
        && Confirm::new()
            .with_prompt("Add Wallp directory to system PATH?")
            .default(true)
            .interact()?
    {
        add_to_path_windows(&final_exe_path)?;
    }

    println!("✅ Configuration saved!");
    if !is_installed && final_exe_path != env::current_exe()? {
        println!("ℹ️  You can safely delete this executable and the downloaded file.");
        println!(
            "ℹ️  Wallp is now installed at: {}",
            final_exe_path.display()
        );
    }

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
    use winreg::enums::*;
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
        format!("{};{}", path_val, install_dir_str)
    };

    env.set_value("Path", &new_path)?;
    println!("✅ Added {} to PATH.", install_dir_str);

    let _ = broadcast_env_change();
    println!("ℹ️ System notified of PATH change.");

    Ok(())
}

fn broadcast_env_change() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let param = CString::new("Environment").unwrap();
        unsafe {
            let _ = SendMessageTimeoutA(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                WPARAM(0),
                windows::Win32::Foundation::LPARAM(param.as_ptr() as isize),
                SMTO_ABORTIFHUNG,
                5000,
                None,
            );
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn add_to_path_windows(_exe_path: &Path) -> Result<()> {
    Ok(()) // No-op for now on non-windows
}

pub fn setup_autostart(enable: bool, exe_path: &Path) -> Result<()> {
    let app_path = exe_path
        .to_str()
        .context("Failed to get executable path as string")?;

    let auto = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Wallp")
        .set_app_path(app_path)
        .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build auto_launch: {}", e))?;

    if enable {
        auto.enable()
            .map_err(|e| anyhow::anyhow!("Failed to enable autostart: {}", e))?;
    } else {
        auto.disable()
            .map_err(|e| anyhow::anyhow!("Failed to disable autostart: {}", e))?;
    }
    Ok(())
}

fn start_background_process(exe_path: &Path) -> Result<()> {
    let mut cmd = Command::new(exe_path);

    // Detach process on Windows to ensure it survives console close and doesn't inherit console
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(DETACHED_PROCESS);
    }

    cmd.spawn().context("Failed to start background process")?;

    println!("🚀 Wallp started in background.");
    Ok(())
}

pub fn handle_command(cmd: &Commands) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().unwrap();

    match cmd {
        Commands::Init => unreachable!(), // Handled in main
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
        Commands::Config(args) => match &args.action {
            Some(ConfigAction::Edit) => {
                let path = AppData::get_config_path()?;
                open::that(path)?;
            }
            Some(ConfigAction::Set { key, value }) => {
                println!("Setting {} to {} (Not implemented yet)", key, value);
            }
            None => println!("Use 'edit' or 'set'"),
        },
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

fn handle_uninstall() -> Result<()> {
    println!("⚠️  WARNING: This will remove Wallp from startup, delete all configuration/data, and remove it from PATH.");

    if !Confirm::new()
        .with_prompt("Are you sure you want to uninstall Wallp?")
        .default(false)
        .interact()?
    {
        println!("Uninstall cancelled.");
        return Ok(());
    }

    println!("Stopping background processes...");
    // Kill other wallp instances (Tray app)
    #[cfg(target_os = "windows")]
    {
        let my_pid = std::process::id();
        let _ = Command::new("taskkill")
            .args([
                "/F",
                "/IM",
                "wallp.exe",
                "/FI",
                &format!("PID ne {}", my_pid),
            ])
            .output();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("pkill").args(&["-f", "wallp"]).output();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("pkill").args(&["-f", "wallp"]).output();
    }

    println!("Removing from startup...");
    // We try to remove whatever registered path implies.
    // AutoLaunch typically keys off app name, but we might have registered different paths?
    // Let's assume current exe path or installed path.
    // If we installed to AppData, we should point there.
    if let Ok(data_dir) = AppData::get_data_dir() {
        let exe_name = if cfg!(target_os = "windows") {
            "wallp.exe"
        } else {
            "wallp"
        };
        let installed_exe = data_dir.join(exe_name);
        if let Err(e) = setup_autostart(false, &installed_exe) {
            println!("⚠️  Failed to remove installed autostart: {}", e);
        }
    }
    // Also try current exe just in case
    if let Ok(current_exe) = env::current_exe() {
        let _ = setup_autostart(false, &current_exe);
    }

    println!("Removing from PATH...");
    #[cfg(target_os = "windows")]
    {
        if let Err(e) = remove_from_path_windows() {
            println!("⚠️  Failed to remove from PATH: {}", e);
        }
    }

    println!("Removing data and configuration...");
    let data_dir = AppData::get_data_dir()?;
    let current_exe = env::current_exe()?;

    // Check if running from installation directory (AppData)
    let data_dir_canonical = data_dir.canonicalize().unwrap_or_else(|_| data_dir.clone());
    let current_exe_canonical = current_exe
        .canonicalize()
        .unwrap_or_else(|_| current_exe.clone());
    let is_running_from_install = current_exe_canonical.starts_with(&data_dir_canonical);

    // First, try to delete all files in data directory
    if data_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&data_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let _ = if path.is_dir() {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
            }
        }
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    if is_running_from_install {
        // Self-delete: spawn to delete exe after we exit
        println!("ℹ️  Running from installation directory. Scheduling self-deletion...");

        let exe_path = current_exe.display().to_string();

        #[cfg(target_os = "windows")]
        {
            let ps_script = format!(
                r#"Start-Sleep -Seconds 2; Set-Location $env:TEMP; Remove-Item -Path "{}" -Force -ErrorAction SilentlyContinue"#,
                exe_path
            );
            let _ = Command::new("powershell")
                .args(["-WindowStyle", "Hidden", "-Command", &ps_script])
                .spawn();
        }

        #[cfg(target_os = "macos")]
        {
            let script = format!("sleep 2 && rm -f \"{}\"", exe_path);
            let _ = Command::new("sh").args(&["-c", &script]).spawn();
        }

        #[cfg(target_os = "linux")]
        {
            let script = format!("sleep 2 && rm -f \"{}\"", exe_path);
            let _ = Command::new("sh").args(&["-c", &script]).spawn();
        }

        println!("✅ Uninstall complete. The executable will be removed shortly.");
        std::process::exit(0);
    }

    println!("✅ Uninstall complete. You can now delete this executable.");
    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_from_path_windows() -> Result<()> {
    use winreg::enums::*;
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

    let _ = broadcast_env_change();

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn remove_from_path_windows() -> Result<()> {
    Ok(())
}
