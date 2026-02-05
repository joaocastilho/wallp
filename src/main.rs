#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;

mod cli;
mod config;
mod manager;
mod scheduler;
mod tray;
mod unsplash;

#[derive(Parser)]
#[command(name = "Wallp")]
#[command(version, about = "Wallp - Wallpaper Changer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Initialize Wallp (Interactive Setup)
    Init,
    /// Force fetch a new wallpaper
    New,
    /// Go to next wallpaper (History or New)
    Next,
    /// Go to previous wallpaper
    Prev,
    /// Show status
    Status,
    /// Show current wallpaper info
    Info,
    /// Open wallpaper in browser
    Open,
    /// Open local wallpapers folder
    Folder,
    /// Edit configuration
    Config(ConfigArgs),
    /// List recent wallpapers
    List,
}

#[derive(clap::Args)]
struct ConfigArgs {
    #[command(subcommand)]
    action: Option<ConfigAction>,
}

#[derive(clap::Subcommand)]
enum ConfigAction {
    Edit,
    Set { key: String, value: String },
}

fn main() -> anyhow::Result<()> {
    // On Windows, if we are in GUI mode (no console), we need to attach to the parent console
    // if we want to print to stdout (e.g. for CLI commands).
    // If no parent console, we are likely double-clicked, so we stay silent.
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        unsafe {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }

    // Initialize logging (File only mainly, but stdout if attached)
    let file_appender = tracing_appender::rolling::daily(
        directories::ProjectDirs::from("com", "user", "wallp")
            .unwrap()
            .data_dir()
            .join("logs"),
        "wallp.log",
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    let subscriber = tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .finish();
        
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Init) => cli::init_wizard()?,
        Some(cmd) => cli::handle_command(cmd)?,
        None => tray::run()?,
    }

    Ok(())
}
