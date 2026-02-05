use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("wallp").unwrap();
    cmd.arg("--help");
    cmd.assert().success();
}

#[test]
fn test_init_interactive_simulated() {
    let mut cmd = Command::cargo_bin("wallp").unwrap();
    cmd.arg("init");
    
    // Simulate interactive input:
    // 1. Access Key (Enter for default)
    // 2. Interval (Enter for default)
    // 3. Collections (Enter for default)
    // 4. Autostart (Enter for default Y)
    // 5. PATH (Enter for default Y)
    // 6. Start now (Enter for default Y - will try to spawn)
    
    // Using simple newlines to simulate "Enter" key presses for default values.
    cmd.write_stdin("\n\n\n\n\n\n");
    
    let output = cmd.output().expect("Failed to execute command");
    
    // Even if it fails because of permissions or something, it should at least HAVE run the prompts.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Unsplash Access Key") || stdout.contains("Welcome to Wallp Setup Wizard!"));
}

#[test]
fn test_uninstall_interactive_cancel() {
    let mut cmd = Command::cargo_bin("wallp").unwrap();
    cmd.arg("uninstall");
    
    // Answer 'n' to "Are you sure?"
    cmd.write_stdin("n\n");
    
    let output = cmd.output().expect("Failed to execute command");
    let combined_output = format!("{}\n{}", 
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("DEBUG OUTPUT:\n{}", combined_output);
    assert!(combined_output.contains("Uninstall cancelled."));
}
