// No windows_subsystem attribute - defaults to console subsystem for CLI support.
// We handle window hiding manually in main() for tray mode.


use clap::Parser;

mod cli;
mod config;
mod manager;
mod scheduler;
mod tray;
mod unsplash;

#[cfg(target_os = "windows")]
mod win_utils {
    use windows::Win32::System::Console::GetConsoleProcessList;
    use windows::Win32::System::Console::FreeConsole;

    pub fn is_launched_from_terminal() -> bool {
        let mut pids = [0u32; 1];
        let count = unsafe { GetConsoleProcessList(&mut pids) };
        count > 1
    }

    pub fn detach_console() {
        unsafe {
            // Detach this process from the current console window.
            // This effectively makes the console disappearance "permanent" for this process,
            // preventing it from closing if the original console is closed (if started from there)
            // or just removing the window if it spawned one.
            let _ = FreeConsole();
        }
    }
}

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
    /// Uninstall Wallp (Remove startup, data, and cleanup)
    Uninstall,
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
    #[cfg(target_os = "windows")]
    let in_terminal = win_utils::is_launched_from_terminal();
    #[cfg(not(target_os = "windows"))]
    let in_terminal = true; 

    // Initialize logging
    let args: Vec<String> = std::env::args().collect();
    let is_uninstall = args.iter().any(|arg| arg == "uninstall");

    if !is_uninstall {
        let file_appender = tracing_appender::rolling::Builder::new()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("wallp")
            .filename_suffix("log")
            .build(
                directories::BaseDirs::new()
                    .unwrap()
                    .config_dir()
                    .join("wallp")
                    .join("logs")
            )
            .expect("failed to initialize rolling file appender");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        
        let subscriber = tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_ansi(false)
            .finish();
            
        tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    }

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Init) => cli::init_wizard()?,
        Some(cmd) => cli::handle_command(cmd)?,
        None => {
            if in_terminal {
                use clap::CommandFactory;
                Cli::command().print_help()?;
                return Ok(());
            }

            #[cfg(target_os = "windows")]
            win_utils::detach_console();

            tray::run()?;
        },
    }

    Ok(())
}
