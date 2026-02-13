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

pub fn is_initialized() -> bool {
    if let Ok(data_dir) = AppData::get_data_dir() {
        if let Ok(config_path) = AppData::get_config_path() {
            return data_dir.exists() && config_path.exists();
        }
    }
    false
}
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
    let config_path = AppData::get_config_path()?;

    // Check if already initialized (data dir exists with config)
    let is_initialized = data_dir.exists() && config_path.exists();

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
    #[cfg(target_os = "windows")]
    if cfg!(target_os = "windows")
        && Confirm::new()
            .with_prompt("Add Wallp directory to system PATH?")
            .default(true)
            .interact()?
    {
        add_to_path_windows(&final_exe_path)?;
    }

    #[cfg(not(target_os = "windows"))]
    if Confirm::new()
        .with_prompt("Add Wallp directory to PATH?")
        .default(true)
        .interact()?
    {
        add_to_path_unix(&final_exe_path)?;
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
        let param =
            CString::new("Environment").context("Failed to create CString for broadcast")?;
        unsafe {
            let result = SendMessageTimeoutA(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                WPARAM(0),
                windows::Win32::Foundation::LPARAM(param.as_ptr() as isize),
                SMTO_ABORTIFHUNG,
                5000,
                None,
            );
            if result.0 == 0 {
                eprintln!("Warning: Could not notify system of PATH change");
            }
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn add_to_path_unix(_exe_path: &Path) -> Result<()> {
    Ok(())
}

fn get_shell_name() -> &'static str {
    let shell = std::env::var("SHELL")
        .map(|s| {
            if s.contains("zsh") {
                "zsh"
            } else if s.contains("bash") {
                "bash"
            } else {
                "bash"
            }
        })
        .unwrap_or("bash");

    // Validate shell exists
    let shell_paths = ["/bin", "/usr/bin", "/usr/local/bin"];
    let shell_exists = shell_paths
        .iter()
        .any(|path| PathBuf::from(format!("{}/{}", path, shell)).exists());

    if !shell_exists {
        // Fallback to sh which should always exist
        return "sh";
    }

    shell
}

#[cfg(test)]
pub fn get_shell_files(shell: &str) -> (String, String) {
    if shell == "zsh" {
        (".zshrc".to_string(), ".zprofile".to_string())
    } else {
        (".bashrc".to_string(), ".bash_profile".to_string())
    }
}

fn shell_escape(s: &str) -> String {
    s.replace('"', "\\\"").replace('$', "\\$")
}

#[cfg(test)]
pub fn create_export_line(install_dir: &str) -> String {
    let escaped = shell_escape(install_dir);
    format!(r#"export PATH="$PATH:{}""#, escaped)
}

#[cfg(test)]
pub fn add_path_to_profile_content(content: &str, install_dir: &str) -> String {
    let export_line = create_export_line(install_dir);
    // Use exact line matching to avoid false positives
    if content.lines().any(|line| line.trim() == export_line) {
        return content.to_string();
    }
    format!("{}\n# Wallp\n{}\n", content, export_line)
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

    let export_line = format!(r#"export PATH="$PATH:{}")"#, escaped_path);

    for profile_name in &[&rc_file, &profile_file] {
        let profile_path = home_dir.join(profile_name);

        // Check permissions if file exists
        if profile_path.exists() {
            let metadata = fs::metadata(&profile_path)?;
            if metadata.permissions().readonly() {
                println!("⚠️  Profile {} is read-only, skipping", profile_name);
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
            println!("ℹ️ Directory already in PATH ({})", profile_name);
            continue;
        }

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&profile_path)
            .context(format!("Failed to open {}", profile_name))?;

        use std::io::Write;
        writeln!(file, r#"\n# Wallp\nexport PATH="$PATH:{}""#, escaped_path)
            .context(format!("Failed to write to {}", profile_name))?;
    }

    println!("✅ Added {} to PATH.", install_dir_str);
    println!(
        "ℹ️ Restart your terminal or run 'source {}' to apply changes.",
        rc_file
    );

    Ok(())
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
        Commands::Setup => unreachable!(), // Handled in main
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
    #[cfg(unix)]
    {
        // Use more specific pattern and current user only to avoid killing unrelated processes
        let _ = Command::new("pkill")
            .args(&["-f", "-u", &std::process::id().to_string(), "^/.*/wallp$"])
            .output();
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
    #[cfg(not(target_os = "windows"))]
    {
        if let Err(e) = remove_from_path_unix() {
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

        #[cfg(unix)]
        {
            let script = format!(
                r#"for i in 1 2 3 4 5; do
  sleep 1
  if rm -f "{}" 2>/dev/null; then
    break
  fi
done"#,
                exe_path
            );
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

    let export_line = format!(r#"export PATH="$PATH:{}")"#, escaped_path);

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
    println!(
        "ℹ️ Restart your terminal or run 'source {}' to apply changes.",
        rc_file
    );

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn remove_from_path_unix() -> Result<()> {
    Ok(())
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
        assert_eq!(escaped, r#"/path/\$HOME/wallp"#);
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
