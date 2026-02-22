use crate::config::AppData;
use crate::manager;
use anyhow::{Context, Result};
use chrono::DateTime;
pub use clap::{Parser, Subcommand};
use dialoguer::{Confirm, Input, MultiSelect};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn format_datetime(iso: &str) -> String {
    DateTime::parse_from_rfc3339(iso).map_or_else(
        |_| iso.to_string(),
        |dt| dt.format("%b %d, %Y at %l:%M %p").to_string(),
    )
}

#[derive(Parser)]
#[command(name = "wallp")]
#[command(version = env!("CARGO_PKG_VERSION"), disable_version_flag = true, about = "A cross-platform wallpaper manager.", long_about = None)]
#[command(disable_help_flag = true)]
pub struct Cli {
    #[arg(long, help = "print help")]
    pub help: bool,
    #[arg(short = 'v', long, action = clap::ArgAction::Version)]
    pub version: Option<bool>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
pub enum Commands {
    /// fetch a new random wallpaper
    New,
    /// advance to next wallpaper
    Next,
    /// go back to previous wallpaper
    Prev,
    /// show current wallpaper details
    Info,
    /// set wallpaper by number from history (shows list if no number provided)
    Set {
        /// wallpaper number to set (see 'wallp list')
        index: Option<usize>,
    },

    /// show scheduler status
    Status,
    /// list recent wallpaper history
    List,
    /// show current configuration settings
    Settings,
    /// open wallpapers folder in file manager
    Folder,
    /// open configuration file in default editor
    Config,

    /// run interactive setup wizard
    Setup,
    /// remove wallp and all data
    Uninstall,
}

impl Commands {
    #[must_use]
    pub const fn group_index(&self) -> usize {
        match self {
            Self::New | Self::Next | Self::Prev | Self::Info | Self::Set { .. } => 0,
            Self::Status | Self::List | Self::Settings | Self::Folder | Self::Config => 1,
            Self::Setup | Self::Uninstall => 2,
        }
    }

    #[must_use]
    pub fn all_commands() -> Vec<(String, String, usize)> {
        use clap::CommandFactory;
        let cmd = Cli::command();
        cmd.get_subcommands()
            .map(|sub| {
                let name = sub.get_name().to_string();
                let about = sub
                    .get_about()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default();
                let variant = match name.as_str() {
                    "next" => Self::Next,
                    "prev" => Self::Prev,
                    "info" => Self::Info,
                    "set" => Self::Set { index: None },
                    "status" => Self::Status,
                    "list" => Self::List,
                    "settings" => Self::Settings,
                    "folder" => Self::Folder,
                    "config" => Self::Config,
                    "setup" => Self::Setup,
                    "uninstall" => Self::Uninstall,
                    _ => Self::New,
                };
                (name, about, variant.group_index())
            })
            .collect()
    }
}

pub fn print_grouped_help() {
    use clap::CommandFactory;
    let cmd = Cli::command();
    let bin_name = cmd.get_name();

    println!("\nusage: {bin_name} [<command>] [<options>]");
    println!("\nCommands:");

    let commands = Commands::all_commands();
    let max_name_len = commands
        .iter()
        .map(|(name, _, _)| name.len())
        .max()
        .unwrap_or(10);

    for group_idx in 0..3 {
        for (name, about, cmd_group) in &commands {
            if *cmd_group == group_idx {
                let padding = " ".repeat(max_name_len.saturating_sub(name.len()) + 2);
                println!("  {name}{padding}{about}");
            }
        }
    }

    println!("\nOptions:");
    println!("  -h, --help     print help");
    println!("  -v, --version  print version");
}

const fn get_exe_name() -> &'static str {
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
        _ => Err("Use: d (days), h (hours), m (minutes)".to_string()),
    }
}

fn format_interval_for_display(minutes: u64) -> String {
    if minutes >= 1440 {
        format!("{}d", minutes / 1440)
    } else if minutes >= 60 {
        format!("{}h", minutes / 60)
    } else {
        format!("{minutes}m")
    }
}

fn get_default_collections_info() -> Vec<(String, String)> {
    vec![
        ("1065976".to_string(), "Wallpapers".to_string()),
        ("3330448".to_string(), "Nature".to_string()),
        ("894".to_string(), "Earth & Planets".to_string()),
    ]
}

#[must_use]
pub fn is_initialized() -> bool {
    let is_installed = {
        #[cfg(target_os = "linux")]
        {
            AppData::get_binary_dir()
                .map(|dir| dir.join(get_exe_name()).exists())
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "linux"))]
        {
            AppData::get_data_dir()
                .map(|dir| dir.join(get_exe_name()).exists())
                .unwrap_or(false)
        }
    };

    if is_installed {
        return true;
    }

    if let Ok(config_path) = AppData::get_config_path()
        && config_path.exists()
    {
        let _ = fs::remove_file(&config_path);
    }

    false
}

#[must_use]
pub fn is_autostart_enabled() -> bool {
    let Ok(current_exe) = std::env::current_exe() else {
        return false;
    };

    let Some(exe_path) = current_exe.to_str() else {
        return false;
    };

    #[cfg(target_os = "macos")]
    let auto = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Wallp")
        .set_app_path(exe_path)
        .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
        .build();

    #[cfg(not(target_os = "macos"))]
    let auto = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Wallp")
        .set_app_path(exe_path)
        .build();

    auto.map(|a| a.is_enabled().unwrap_or(false))
        .unwrap_or(false)
}

/// Runs the interactive setup wizard.
///
/// # Errors
///
/// Returns an error if interacting with the user or saving configuration fails.
#[allow(clippy::too_many_lines)]
pub fn setup_wizard() -> Result<()> {
    let is_installed = is_initialized();

    if is_installed {
        println!("Wallp Setup - Modify Settings");
        println!("-----------------------------");
        println!();

        if !Confirm::new()
            .with_prompt("Wallp is already installed. Do you want to modify settings?")
            .default(false)
            .interact()?
        {
            println!("No changes made.");
            return Ok(());
        }
    } else {
        println!("Welcome to Wallp Setup Wizard!");
        println!("------------------------------");
        println!();

        if !Confirm::new()
            .with_prompt("Do you want to install Wallp?")
            .default(true)
            .interact()?
        {
            println!("Setup cancelled.");
            return Ok(());
        }
    }

    // Load existing config if any
    let mut app_data = AppData::load().unwrap_or_default();

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
            .with_prompt("Update Interval (e.g., 1d, 12h, 30m)")
            .default(format_interval_for_display(
                app_data.config.interval_minutes,
            ))
            .interact()
            .context("Failed to get interval")?;

        match parse_interval(&input) {
            Ok(minutes) if minutes >= MIN_INTERVAL_MINUTES => break minutes,
            Ok(_) => {
                println!(
                    "Interval must be at least 30 minutes to avoid exceeding Unsplash's rate limit of 50 requests per day"
                );
            }
            Err(e) => {
                println!("Invalid input: {e}");
            }
        }
    };

    println!();

    // Collection selection with checkboxes
    let default_collections = get_default_collections_info();
    let current_collections = app_data.config.collections.clone();
    let custom_collections = app_data.config.custom_collections.clone();

    // Build items: defaults + custom + add option
    let mut all_items: Vec<(String, String, bool)> = default_collections
        .iter()
        .map(|(id, desc)| (id.clone(), format!("{desc} - {id}"), false))
        .collect();

    for (id, desc) in &custom_collections {
        all_items.push((id.clone(), format!("Custom: {desc} - {id}"), true));
    }

    let add_custom_index = all_items.len();
    all_items.push((
        String::new(),
        "[+] Add custom collection(s)".to_string(),
        false,
    ));

    // Build display items and defaults
    let items: Vec<String> = all_items.iter().map(|(_, desc, _)| desc.clone()).collect();

    let defaults: Vec<bool> = all_items
        .iter()
        .map(|(id, _, is_custom)| {
            if id.is_empty() {
                false
            } else if *is_custom {
                custom_collections.iter().any(|(cid, _)| cid == id)
            } else {
                current_collections.contains(id)
            }
        })
        .collect();

    println!("Select collections (Space to toggle, Enter to confirm):");
    println!("Find more at: https://unsplash.com/collections");
    println!();

    let selections = MultiSelect::new()
        .items(&items)
        .defaults(&defaults)
        .interact()
        .context("Failed to select collections")?;

    // Handle "Add custom collection(s)" if selected
    let mut new_collections: Vec<String> = Vec::new();
    let mut updated_custom_collections: Vec<(String, String)> = custom_collections;

    for idx in &selections {
        if *idx == add_custom_index {
            let ids_input: String = Input::new()
                .with_prompt("Enter collection IDs (comma-separated, e.g., 1234567,8901234)")
                .interact()
                .context("Failed to get collection IDs")?;

            for id in ids_input
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let desc: String = Input::new()
                    .with_prompt(format!(
                        "Description for {id} (optional, press Enter to skip):"
                    ))
                    .default(format!("Collection {id}"))
                    .interact()
                    .context("Failed to get description")?;

                let final_desc = if desc.is_empty() || desc == format!("Collection {id}") {
                    format!("Collection {id}")
                } else {
                    desc
                };

                updated_custom_collections.push((id.to_string(), final_desc));
                new_collections.push(id.to_string());
            }
        } else {
            new_collections.push(all_items[*idx].0.clone());
        }
    }

    // Retention prompt
    let retention_days: Option<u64> = loop {
        let input: String = Input::new()
            .with_prompt("Keep wallpapers for how many days? (leave empty for forever, 0 to delete immediately)")
            .default("7".to_string())
            .interact()
            .context("Failed to get retention days")?;

        if input.trim().is_empty() {
            break None;
        }

        match input.trim().parse::<u64>() {
            Ok(0) => break Some(0),
            Ok(n) => break Some(n),
            Err(_) => {
                println!("Invalid input. Please enter a number, or leave empty for forever.");
            }
        }
    };

    // Ask about system integration (skip PATH if already installed)
    println!();
    println!("🔧 System Integration");

    let enable_autostart = Confirm::new()
        .with_prompt("Enable Autostart on Login?")
        .default(true)
        .interact()
        .context("Failed to get autostart confirmation")?;

    let add_to_path = if is_installed {
        false
    } else {
        Confirm::new()
            .with_prompt("Add Wallp to PATH?")
            .default(true)
            .interact()?
    };

    // Ask to proceed
    println!();
    if is_installed {
        if !Confirm::new()
            .with_prompt("Save changes?")
            .default(true)
            .interact()?
        {
            println!("No changes made.");
            return Ok(());
        }
    } else {
        println!("Ready to install.");
        if !Confirm::new()
            .with_prompt("Proceed with installation?")
            .default(true)
            .interact()?
        {
            println!("Setup cancelled.");
            return Ok(());
        }
    }

    // ===== ACTUAL CHANGES START HERE =====

    // Skip binary installation if already installed
    if is_installed {
        // Just update settings without reinstallation
        if enable_autostart != is_autostart_enabled()
            && let Ok(current_exe) = env::current_exe()
        {
            setup_autostart(enable_autostart, &current_exe)?;
        }

        // Save configuration
        app_data.config.unsplash_access_key = access_key;
        app_data.config.interval_minutes = interval;
        app_data.config.collections = new_collections;
        app_data.config.custom_collections = updated_custom_collections;
        app_data.config.retention_days = retention_days;
        app_data.save()?;

        println!();
        println!("✅ Settings saved successfully!");
    } else {
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

        // Copy to installation directory if not already there
        let current_exe_canonical = current_exe
            .canonicalize()
            .unwrap_or_else(|_| current_exe.clone());
        let target_exe_canonical = target_exe.canonicalize().ok();

        let is_running_from_install =
            target_exe_canonical.is_some_and(|t| t == current_exe_canonical);

        let final_exe_path = if is_running_from_install {
            println!("ℹ️  Already running from installation directory.");
            current_exe
        } else {
            println!("Installing Wallp to {}", target_exe.display());
            match fs::copy(&current_exe, &target_exe) {
                Ok(_) => {
                    println!("✅ Wallp copied to installation directory.");

                    // Give the filesystem a moment to settle
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    target_exe
                }
                Err(e) => {
                    println!(
                        "⚠️  Failed to copy executable: {e}. Proceeding with current executable."
                    );
                    current_exe
                }
            }
        };

        // Canonicalize the final path
        let final_exe_path = final_exe_path.canonicalize().unwrap_or(final_exe_path);

        // Save configuration
        app_data.config.unsplash_access_key = access_key;
        app_data.config.interval_minutes = interval;
        app_data.config.collections = new_collections;
        app_data.config.custom_collections = updated_custom_collections;
        app_data.config.retention_days = retention_days;
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
        if add_to_path {
            #[cfg(target_os = "linux")]
            {
                add_local_bin_to_path()?;
            }
            #[cfg(target_os = "windows")]
            {
                add_to_path_windows(&final_exe_path)?;
            }
            #[cfg(target_os = "macos")]
            {
                add_to_path_unix(&final_exe_path)?;
            }
        }

        println!();
        println!("✅ Wallp installed successfully!");
        println!("\nUsage:");
        println!("  wallp new     - Get new wallpaper");
        println!("  wallp next    - Next wallpaper");
        println!("  wallp prev    - Previous wallpaper");
        println!("  wallp --help  - See all commands");
        if !is_running_from_install && final_exe_path != env::current_exe()? {
            println!("\nYou can delete this installer file.");
        }

        println!();

        start_background_process(&final_exe_path)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn add_to_path_windows(exe_path: &Path) -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

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
        println!("ℹ️ Directory already in PATH");
        return Ok(());
    }

    // Append
    let new_path = if path_val.is_empty() {
        install_dir_str.to_string()
    } else {
        format!("{path_val};{install_dir_str}")
    };

    env.set_value("Path", &new_path)?;
    println!("✅ Added to PATH (restart terminal to apply changes)");

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

    if shell_exists { shell } else { "sh" }
}

#[cfg(test)]
#[must_use]
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
#[must_use]
pub fn create_export_line(install_dir: &str) -> String {
    let escaped = shell_escape(install_dir);
    format!(r#"export PATH="$PATH:{escaped}""#)
}

#[cfg(test)]
#[must_use]
pub fn add_path_to_profile_content(content: &str, install_dir: &str) -> String {
    let export_line = create_export_line(install_dir);
    // Use exact line matching to avoid false positives
    if content.lines().any(|line| line.trim() == export_line) {
        return content.to_string();
    }
    format!("{content}\n# Wallp\n{export_line}\n")
}

#[cfg(test)]
#[must_use]
pub fn remove_path_from_profile_content(content: &str, install_dir: &str) -> String {
    let export_line = create_export_line(install_dir);
    content
        .lines()
        .filter(|line| line.trim() != export_line && !line.contains("# Wallp"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
#[must_use]
pub fn is_path_in_profile(content: &str, install_dir: &str) -> bool {
    let export_line = create_export_line(install_dir);
    content.lines().any(|line| line.trim() == export_line)
}

#[cfg(unix)]
#[allow(dead_code)]
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

    println!("✅ Added to PATH (restart terminal to apply changes)");

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
            println!("ℹ️ Directory already in PATH");
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

    println!("✅ Added to PATH (restart terminal to apply changes)");

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

/// Setup autostart for the application.
///
/// # Errors
///
/// Returns an error if the auto-launch builder fails or if enabling/disabling fails.
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

/// Handle the parsed CLI command.
///
/// # Errors
///
/// Returns an error if the command fails to execute or if the tokio runtime fails to create.
#[allow(clippy::too_many_lines)]
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
            println!("Next Run: {}", format_datetime(&data.state.next_run_at));
            println!("Last Run: {}", format_datetime(&data.state.last_run_at));
            if let Some(w) = manager::get_current_wallpaper()? {
                let title = w.title.unwrap_or_default();
                let author = w.author.unwrap_or_default();
                if !title.is_empty() && !author.is_empty() {
                    println!("Current: {title} by {author}");
                }
            }
        }
        Commands::Info => {
            if let Some(w) = manager::get_current_wallpaper()? {
                println!("Title: {}", w.title.unwrap_or_default());
                println!("Author: {}", w.author.unwrap_or_default());
                if let Some(url) = w.url {
                    println!();
                    println!("View: {url}");
                }
            } else {
                println!("No wallpaper in history.");
            }
        }
        Commands::Set { index } => {
            let data = AppData::load()?;
            let history_len = data.history.len();

            if history_len == 0 {
                println!("No wallpaper in history.");
                return Ok(());
            }

            if let Some(idx) = index {
                rt.block_on(manager::set_by_index(*idx))?;
                println!("✅ Wallpaper set to index {idx}");
            } else {
                println!("Select a wallpaper (most recent is 0):");
                println!();

                let mut shown = 0;
                let mut total_shown = 0;
                let max_initial = 5;
                let max_more = 10;

                loop {
                    let to_show = if shown == 0 { max_initial } else { max_more };
                    let items: Vec<String> = data
                        .history
                        .iter()
                        .rev()
                        .skip(shown)
                        .take(to_show)
                        .enumerate()
                        .map(|(i, w)| {
                            let idx = shown + i;
                            format!(
                                "{}: {} by {}",
                                idx,
                                w.title.clone().unwrap_or_default(),
                                w.author.clone().unwrap_or_default()
                            )
                        })
                        .collect();

                    for item in &items {
                        println!("{item}");
                    }

                    shown += items.len();
                    total_shown += items.len();

                    if shown >= history_len {
                        break;
                    }

                    let prompt = if total_shown == max_initial {
                        "Enter number (or 'm' for more):"
                    } else {
                        "Enter number (or 'm' for more, 'q' to quit):"
                    };

                    let input: String = Input::new()
                        .with_prompt(prompt)
                        .interact()
                        .context("Failed to get input")?;

                    if input.eq_ignore_ascii_case("q") {
                        println!("Cancelled.");
                        return Ok(());
                    }

                    if input.eq_ignore_ascii_case("m") {
                        continue;
                    }

                    match input.trim().parse::<usize>() {
                        Ok(idx) => {
                            rt.block_on(manager::set_by_index(idx))?;
                            println!("✅ Wallpaper set to index {idx}");
                            break;
                        }
                        Err(_) => {
                            println!(
                                "Invalid input. Enter a number, 'm' for more, or 'q' to quit."
                            );
                        }
                    }
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
        Commands::Settings => {
            let data = AppData::load()?;
            let config = &data.config;

            // Format API key (masked)
            let api_key = if config.unsplash_access_key.is_empty() {
                "Not set".to_string()
            } else {
                format!(
                    "****{}",
                    &config.unsplash_access_key[config.unsplash_access_key.len() - 4..]
                )
            };

            // Build collections list with descriptions
            let default_collections = get_default_collections_info();
            let mut collection_lines = Vec::new();

            for col_id in &config.collections {
                let desc = default_collections
                    .iter()
                    .find(|(id, _)| id == col_id)
                    .map(|(_, d)| d.clone())
                    .or_else(|| {
                        config
                            .custom_collections
                            .iter()
                            .find(|(id, _)| id == col_id)
                            .map(|(_, d)| d.clone())
                    })
                    .unwrap_or_else(|| "Unknown".to_string());

                collection_lines.push(format!("  - {desc} ({col_id})"));
            }

            // Format interval
            let interval_str = format_interval_for_display(config.interval_minutes);

            // Format retention
            let retention_str = match config.retention_days {
                Some(0) => "Delete immediately".to_string(),
                Some(n) => format!("{n} days"),
                None => "Forever".to_string(),
            };

            // Autostart status
            let autostart_str = if is_autostart_enabled() {
                "Enabled"
            } else {
                "Disabled"
            };

            // PATH status
            let path_str = if which::which("wallp").is_ok() {
                "Yes"
            } else {
                "No"
            };

            println!();
            println!("API Key: {api_key}");
            println!("Collections:");
            for line in collection_lines {
                println!("{line}");
            }
            println!("Update Interval: {interval_str}");
            println!("Retention: {retention_str}");
            println!("Autostart: {autostart_str}");
            println!("Application in PATH: {path_str}");
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
    // Kill other wallp instances (Tray app)
    #[cfg(target_os = "windows")]
    {
        let my_pid = std::process::id();
        let output = Command::new("taskkill")
            .args(["/F", "/IM", "wallp.exe", "/FI", &format!("PID ne {my_pid}")])
            .output();

        if output.map(|o| o.status.success()).unwrap_or(false) {
            println!("[  OK  ] Stopped background processes");
            std::thread::sleep(std::time::Duration::from_secs(2));
        } else {
            println!("[WARNING] Could not stop all Wallp processes - please close Wallp manually");
        }
    }
    #[cfg(unix)]
    {
        let output = Command::new("pkill").args(["-x", "wallp"]).output();
        if output.map(|o| o.status.success()).unwrap_or(false) {
            println!("[  OK  ] Stopped background processes");
            std::thread::sleep(std::time::Duration::from_secs(2));
        } else {
            println!("[WARNING] Could not stop all Wallp processes - please close Wallp manually");
        }
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
                    Ok(()) => println!("[  OK  ] Removed binary"),
                    Err(e) => println!("[FAILED] Failed to remove binary: {e}"),
                }
            }
        }

        if let Ok(config_dir) = AppData::get_config_dir()
            && config_dir.exists()
        {
            if std::fs::remove_dir_all(&config_dir).is_ok() {
                println!("[  OK  ] Removed configuration");
            } else {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let _ = std::fs::remove_dir_all(&config_dir);
            }
        }

        if let Ok(data_dir) = AppData::get_data_dir()
            && data_dir.exists()
        {
            if std::fs::remove_dir_all(&data_dir).is_ok() {
                println!("[  OK  ] Removed data directory");
            } else {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let _ = std::fs::remove_dir_all(&data_dir);
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        if let Ok(data_dir) = AppData::get_data_dir()
            && data_dir.exists()
        {
            if std::fs::remove_dir_all(&data_dir).is_ok() {
                println!("[  OK  ] Removed data and configuration");
            } else {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let _ = std::fs::remove_dir_all(&data_dir);
            }
        }
    }

    // Check if running from installation directory
    let is_running_from_install = AppData::get_data_dir().is_ok_and(|data_dir| {
        let data_dir_canonical = data_dir.canonicalize().unwrap_or_else(|_| data_dir.clone());
        let current_exe_canonical = current_exe
            .canonicalize()
            .unwrap_or_else(|_| current_exe.clone());
        current_exe_canonical.starts_with(&data_dir_canonical)
    });

    #[cfg(target_os = "linux")]
    let is_running_from_install = is_running_from_install || {
        AppData::get_binary_dir().is_ok_and(|binary_dir| {
            let binary_dir_canonical = binary_dir
                .canonicalize()
                .unwrap_or_else(|_| binary_dir.clone());
            let current_exe_canonical = current_exe
                .canonicalize()
                .unwrap_or_else(|_| current_exe.clone());
            current_exe_canonical.starts_with(&binary_dir_canonical)
        })
    };

    if is_running_from_install {
        // Self-delete: spawn to delete exe and directories after we exit
        println!("[INFO] Running from installation directory - scheduling self-deletion");

        #[allow(unused_variables)]
        let exe_path = current_exe.display().to_string();

        #[cfg(target_os = "windows")]
        {
            let data_dir = AppData::get_data_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default();

            let escaped_exe_path = powershell_escape(&exe_path);
            let escaped_data_dir = powershell_escape(&data_dir);

            let ps_script = format!(
                r#"Start-Sleep -Seconds 2
Set-Location $env:TEMP
Remove-Item -LiteralPath "{escaped_exe_path}" -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath "{escaped_data_dir}" -Recurse -Force -ErrorAction SilentlyContinue"#
            );
            let _ = Command::new("powershell")
                .args(["-WindowStyle", "Hidden", "-Command", &ps_script])
                .spawn();
        }

        #[cfg(target_os = "linux")]
        {
            let exe_path = current_exe.display().to_string();
            let data_dir = AppData::get_data_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let config_dir = AppData::get_config_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            let binary_dir = AppData::get_binary_dir()
                .map(|p| p.join("wallp").display().to_string())
                .unwrap_or_default();

            let escaped_exe = shell_escape(&exe_path);
            let escaped_data = shell_escape(&data_dir);
            let escaped_config = shell_escape(&config_dir);
            let escaped_binary = shell_escape(&binary_dir);

            let script = format!(
                r#"for i in 1 2 3 4 5; do
  sleep 1
  rm -f "{escaped_exe}" 2>/dev/null
  rm -rf "{escaped_data}" 2>/dev/null
  rm -rf "{escaped_config}" 2>/dev/null
  rm -f "{escaped_binary}" 2>/dev/null
done"#
            );
            let _ = Command::new("sh").args(["-c", &script]).spawn();
        }

        #[cfg(target_os = "macos")]
        {
            let exe_path = current_exe.display().to_string();
            let data_dir = AppData::get_data_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default();

            let escaped_exe = shell_escape(&exe_path);
            let escaped_data = shell_escape(&data_dir);

            let script = format!(
                r#"for i in 1 2 3 4 5; do
  sleep 1
  rm -f "{escaped_exe}" 2>/dev/null
  rm -rf "{escaped_data}" 2>/dev/null
done"#
            );
            let _ = Command::new("sh").args(["-c", &script]).spawn();
        }

        println!();
        println!("Uninstall complete. All installed files will be removed shortly.");
        std::process::exit(0);
    }

    println!();
    println!("Uninstall complete. You can now delete this executable.");
    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_from_path_windows() -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

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
#[allow(dead_code)]
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
