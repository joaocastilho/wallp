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
    use windows::Win32::System::Console::{GetConsoleProcessList, GetConsoleWindow};
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    pub fn is_launched_from_terminal() -> bool {
        let mut pids = [0u32; 1];
        let count = unsafe { GetConsoleProcessList(&mut pids) };
        // If count > 1, it means there are other processes (like cmd.exe or powershell.exe)
        // attached to this console.
        count > 1
    }

    pub fn hide_console_window() {
        let window = unsafe { GetConsoleWindow() };
        if !window.is_invalid() {
            unsafe {
                let _ = ShowWindow(window, SW_HIDE);
            }
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
        let file_appender = tracing_appender::rolling::daily(
            directories::ProjectDirs::from("", "", "wallp")
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
            win_utils::hide_console_window();

            tray::run()?;
        },
    }

    Ok(())
}
