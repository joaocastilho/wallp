use crate::{Commands, ConfigAction};
use crate::config::AppData;
use crate::manager;
use anyhow::{Context, Result};
use dialoguer::{Input, Confirm};
use std::process::Command;
use std::env;

pub fn init_wizard() -> Result<()> {
    println!("Welcome to Wallp Setup Wizard!");
    println!("------------------------------");

    let mut app_data = AppData::load()?; // Load existing or default

    let access_key: String = Input::new()
        .with_prompt("Unsplash Access Key")
        .default(app_data.config.unsplash_access_key.clone())
        .interact_text()?;

    let interval: u64 = Input::new()
        .with_prompt("Update Interval (minutes)")
        .default(app_data.config.interval_minutes)
        .interact_text()?;

    // Default collections
    let collections_str = app_data.config.collections.join(",");
    let collections_input: String = Input::new()
        .with_prompt("Unsplash Collection IDs (comma separated)")
        .default(collections_str)
        .interact_text()?;
    
    let collections: Vec<String> = collections_input.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let enable_autostart = Confirm::new()
        .with_prompt("Enable Autostart on Login?")
        .default(true)
        .interact()?;

    // Update config
    app_data.config.unsplash_access_key = access_key;
    app_data.config.interval_minutes = interval;
    app_data.config.collections = collections;
    app_data.save()?;

    // Setup Autostart
    if enable_autostart {
        setup_autostart(true)?;
        println!("✅ Autostart enabled.");
    } else {
        setup_autostart(false)?;
        println!("ℹ️ Autostart disabled.");
    }

    // Add to PATH
    if cfg!(target_os = "windows") {
        if Confirm::new()
            .with_prompt("Add Wallp directory to system PATH?")
            .default(true)
            .interact()? 
        {
            add_to_path_windows()?;
        }
    }

    println!("✅ Configuration saved!");
    
    // Launch Tray App
    if Confirm::new().with_prompt("Start Wallp now?").default(true).interact()? {
        start_background_process()?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn add_to_path_windows() -> Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let current_exe = env::current_exe()?;
    let install_dir = current_exe.parent().context("Failed to get executable directory")?;
    let install_dir_str = install_dir.to_str().context("Invalid path")?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (env, _) = hkcu.create_subkey("Environment")?; // Create or open
    let path_val: String = env.get_value("Path").unwrap_or_default();

    // Check if already in PATH
    let paths: Vec<&str> = path_val.split(';').collect();
    if paths.iter().any(|p| p.eq_ignore_ascii_case(install_dir_str)) {
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
    println!("✅ Added {} to PATH. Restart your terminal to see changes.", install_dir_str);
    
    // Notify system of env change (broadcast WM_SETTINGCHANGE)
    // This requires unsafe code and user32.dll, skipping for simplicity/safety unless requested.
    // Standard practice is to tell user to restart terminal.

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn add_to_path_windows() -> Result<()> {
    Ok(()) // No-op for now on non-windows
}

fn setup_autostart(enable: bool) -> Result<()> {
    let current_exe = env::current_exe()?;
    let auto = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("Wallp")
        .set_app_path(current_exe.to_str().unwrap())
        .set_use_launch_agent(false) 
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build auto_launch: {}", e))?;

    if enable {
        if !auto.is_enabled().unwrap_or(false) {
             auto.enable().map_err(|e| anyhow::anyhow!("Failed to enable autostart: {}", e))?;
        }
    } else {
        if auto.is_enabled().unwrap_or(false) {
            auto.disable().map_err(|e| anyhow::anyhow!("Failed to disable autostart: {}", e))?;
        }
    }
    Ok(())
}

fn start_background_process() -> Result<()> {
    let current_exe = env::current_exe()?;
    
    // Spawn detached process
    Command::new(current_exe)
        .spawn()
        .context("Failed to start background process")?;
        
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
        },
        Commands::Next => {
            rt.block_on(manager::next())?;
            println!("⏩ Next wallpaper set.");
        },
        Commands::Prev => {
            rt.block_on(manager::prev())?;
            println!("⏪ Previous wallpaper set.");
        },
        Commands::Status => {
            let data = AppData::load()?;
            println!("Status: {}", if data.state.is_running { "Running" } else { "Stopped" });
            println!("Next Run: {}", data.state.next_run_at);
            println!("Last Run: {}", data.state.last_run_at);
            println!("Current Wallpaper ID: {:?}", data.state.current_wallpaper_id);
        },
        Commands::Info => {
            if let Some(w) = manager::get_current_wallpaper()? {
                println!("Title: {}", w.title.unwrap_or_default());
                println!("Author: {}", w.author.unwrap_or_default());
                println!("ID: {}", w.id);
            } else {
                println!("No wallpaper in history.");
            }
        },
        Commands::Open => {
            if let Some(w) = manager::get_current_wallpaper()? {
                if let Some(url) = w.url {
                    open::that(url)?;
                } else {
                    println!("No URL available.");
                }
            }
        },
        Commands::Folder => {
            let path = AppData::get_data_dir()?.join("wallpapers");
            open::that(path)?;
        },
        Commands::Config(args) => {
            match &args.action {
                 Some(ConfigAction::Edit) => {
                     let path = AppData::get_config_path()?;
                     open::that(path)?;
                 },
                 Some(ConfigAction::Set { key, value }) => {
                     println!("Setting {} to {} (Not implemented yet)", key, value);
                 },
                 None => println!("Use 'edit' or 'set'"),
            }
        },
        Commands::List => {
            let data = AppData::load()?;
            for (i, w) in data.history.iter().rev().take(5).enumerate() {
                 println!("{}: {} by {}", i, w.title.clone().unwrap_or_default(), w.author.clone().unwrap_or_default());
            }
        }
    }
    Ok(())
}
