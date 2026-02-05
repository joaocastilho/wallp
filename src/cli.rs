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
    // We could make this a MultiSelect if we had names, but for now just input or keep default logic?
    // Let's offer to edit collections as comma separated string?
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

    println!("✅ Configuration saved!");
    
    // Launch Tray App
    if Confirm::new().with_prompt("Start Wallp now?").default(true).interact()? {
        start_background_process()?;
    }

    Ok(())
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
    // Create a tokio runtime for async commands if needed, 
    // BUT main already has no runtime? 
    // Wait, main.rs does NOT initialize a runtime. 
    // We need to create one for async commands.
    
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
