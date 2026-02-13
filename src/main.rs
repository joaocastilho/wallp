// No windows_subsystem attribute - defaults to console subsystem for CLI support.
// We handle window hiding manually in main() for tray mode.


use clap::Parser;
use std::process::ExitCode;

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
        // SAFETY: GetConsoleProcessList is a Windows API that takes a mutable slice
        // and fills it with process IDs. The slice is properly sized and valid.
        let count = unsafe { GetConsoleProcessList(&mut pids) };
        count > 1
    }

    pub fn detach_console() {
        // SAFETY: FreeConsole is a Windows API that detaches the calling process
        // from its console. It's safe to call when a console exists.
        unsafe {
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
    Setup,
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

fn main() -> ExitCode {
    #[cfg(target_os = "windows")]
    let in_terminal = win_utils::is_launched_from_terminal();
    #[cfg(not(target_os = "windows"))]
    let in_terminal = true; 

    // Parse CLI first
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Setup) => {
            if let Err(e) = cli::init_wizard() {
                eprintln!("Error: {}", e);
                return ExitCode::FAILURE;
            }
        }
        Some(cmd) => {
            if let Err(e) = cli::handle_command(cmd) {
                eprintln!("Error: {}", e);
                return ExitCode::FAILURE;
            }
        }
        None => {
            // Check if initialized - if not, run init wizard automatically
            if !cli::is_initialized() {
                println!("First time running Wallp. Running setup...");
                if let Err(e) = cli::init_wizard() {
                    eprintln!("Error during setup: {}", e);
                    return ExitCode::FAILURE;
                }
                return ExitCode::SUCCESS;
            }

            if in_terminal {
                use clap::CommandFactory;
                if let Err(e) = Cli::command().print_help() {
                    eprintln!("Error: {}", e);
                    return ExitCode::FAILURE;
                }
                return ExitCode::SUCCESS;
            }

            #[cfg(target_os = "windows")]
            win_utils::detach_console();

            return tray::run();
        },
    }

    ExitCode::SUCCESS
}
