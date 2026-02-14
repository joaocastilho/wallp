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

#[derive(Parser)]
#[command(name = "wallp")]
#[command(version = env!("CARGO_PKG_VERSION"), disable_version_flag = true, about = ASCII_ART, long_about = None)]
#[command(disable_help_flag = true)]
struct Cli {
    #[arg(long, help = "print help")]
    help: bool,
    #[arg(short = 'v', long, action = clap::ArgAction::Version)]
    version: Option<bool>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
#[allow(clippy::enum_variant_names)]
enum Commands {
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
    fn group_index(&self) -> usize {
        match self {
            Commands::New
            | Commands::Next
            | Commands::Prev
            | Commands::Info
            | Commands::Set { .. } => 0,
            Commands::Status
            | Commands::List
            | Commands::Settings
            | Commands::Folder
            | Commands::Config => 1,
            Commands::Setup | Commands::Uninstall => 2,
        }
    }

    fn all_commands() -> Vec<(String, String, usize)> {
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
                    "next" => Commands::Next,
                    "prev" => Commands::Prev,
                    "info" => Commands::Info,
                    "set" => Commands::Set { index: None },
                    "status" => Commands::Status,
                    "list" => Commands::List,
                    "settings" => Commands::Settings,
                    "folder" => Commands::Folder,
                    "config" => Commands::Config,
                    "setup" => Commands::Setup,
                    "uninstall" => Commands::Uninstall,
                    _ => Commands::New,
                };
                (name, about, variant.group_index())
            })
            .collect()
    }
}

fn print_grouped_help() {
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
            // Auto-run setup on first install
            if !cli::is_initialized() {
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
