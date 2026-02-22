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
    use windows::Win32::System::Console::FreeConsole;
    use windows::Win32::System::Console::GetConsoleProcessList;

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

const ASCII_ART: &str = concat!(
    r#"
                        ██\ ██\           
                        ██ |██ |          
██\  ██\  ██\  ██████\  ██ |██ | ██████\  
██ | ██ | ██ | \____██\ ██ |██ |██  __██\ 
██ | ██ | ██ | ███████ |██ |██ |██ /  ██ |
██ | ██ | ██ |██  __██ |██ |██ |██ |  ██ |
\█████\████  |\███████ |██ |██ |██████  |
 \_____\____/  \_______|\__|\__|██  ____/ 
                                 ██ |      
                                 ██ |      
                                 \__|      

  wallp v1.0.0
  Built: "#,
    env!("BUILD_DATETIME"),
    r#"
  A cross-platform wallpaper manager that fetches random
  wallpapers from Unsplash and manages automatic cycling.
"#
);

use cli::{Cli, Commands, print_grouped_help};

fn main() -> ExitCode {
    #[allow(clippy::single_match_else)]
    #[cfg(target_os = "windows")]
    let in_terminal = win_utils::is_launched_from_terminal();
    #[cfg(not(target_os = "windows"))]
    let in_terminal = std::io::IsTerminal::is_terminal(&std::io::stdin());

    // Parse CLI first
    let cli = Cli::parse();

    // Handle --help flag
    if cli.help {
        println!("{ASCII_ART}");
        print_grouped_help();
        return ExitCode::SUCCESS;
    }

    #[allow(clippy::single_match_else)]
    match &cli.command {
        Some(cmd) => {
            // Allow settings/info/status/list/config/folder commands without initialization
            let needs_init = !matches!(
                cmd,
                Commands::Settings
                    | Commands::Info
                    | Commands::Status
                    | Commands::List
                    | Commands::Config
                    | Commands::Folder
            );
            // Auto-run setup on first install
            if needs_init && !cli::is_initialized() {
                println!("{ASCII_ART}");
                if let Err(e) = cli::setup_wizard() {
                    eprintln!("Error during setup: {e}");
                    return ExitCode::FAILURE;
                }
                return ExitCode::SUCCESS;
            }

            if let Err(e) = cli::handle_command(cmd) {
                eprintln!("Error: {e}");
                return ExitCode::FAILURE;
            }
        }
        None => {
            // Auto-run setup on first install
            if !cli::is_initialized() {
                println!("{ASCII_ART}");
                if let Err(e) = cli::setup_wizard() {
                    eprintln!("Error during setup: {e}");
                    return ExitCode::FAILURE;
                }
                return ExitCode::SUCCESS;
            }

            if in_terminal {
                // Print ASCII art only when showing the menu (no command)
                println!("{ASCII_ART}");
                print_grouped_help();
                return ExitCode::SUCCESS;
            }

            #[cfg(target_os = "windows")]
            win_utils::detach_console();

            return tray::run();
        }
    }

    ExitCode::SUCCESS
}
