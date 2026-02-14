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

const ASCII_ART: &str = r#"
                        ##\ ##\           
                        ## |## |          
##\  ##\  ##\  ######\  ## |## | ######\  
## | ## | ## | \____##\ ## |## |##  __##\ 
## | ## | ## | ####### |## |## |## /  ## |
## | ## | ## |##  __## |## |## |## |  ## |
\#####\####  |\####### |## |## |#######  |
 \_____\____/  \_______|\__|\__|##  ____/ 
                                ## |      
                                ## |      
                                \__|      
"#;

#[derive(Parser)]
#[command(name = "wallp")]
#[command(version, about = ASCII_ART, long_about = None)]
#[command(help_template = "\nusage: {usage}\n\n{all-args}")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    New,
    /// go to next wallpaper (history or new)
    Next,
    /// go to previous wallpaper
    Prev,
    /// show status
    Status,
    /// show current wallpaper info
    Info,
    /// open wallpaper in browser
    Open,
    /// open local wallpapers folder
    Folder,
    /// edit configuration
    Config(ConfigArgs),
    /// list recent wallpapers
    List,
    /// run the setup wizard to configure wallp
    Setup,
    /// uninstall wallp (remove startup, data, and cleanup)
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
    // Always print ASCII art first
    println!("{}", ASCII_ART);

    #[cfg(target_os = "windows")]
    let in_terminal = win_utils::is_launched_from_terminal();
    #[cfg(not(target_os = "windows"))]
    let in_terminal = std::io::IsTerminal::is_terminal(&std::io::stdin());

    // Parse CLI first
    let cli = Cli::parse();

    match &cli.command {
        Some(cmd) => {
            // Auto-run setup on first install
            if !cli::is_initialized() {
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
                if let Err(e) = cli::setup_wizard() {
                    eprintln!("Error during setup: {e}");
                    return ExitCode::FAILURE;
                }
                return ExitCode::SUCCESS;
            }

            if in_terminal {
                use clap::CommandFactory;
                if let Err(e) = Cli::command().print_help() {
                    eprintln!("Error: {e}");
                    return ExitCode::FAILURE;
                }
                return ExitCode::SUCCESS;
            }

            #[cfg(target_os = "windows")]
            win_utils::detach_console();

            return tray::run();
        }
    }

    ExitCode::SUCCESS
}
