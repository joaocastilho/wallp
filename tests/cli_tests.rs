use assert_cmd::prelude::*;
use std::process::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("wallp").unwrap();
    cmd.arg("--help");
    cmd.assert().success();
}

#[test]
fn test_config_load_default() {
    // This depends on the environment, but we can try to mock the data dir
    // For now, just check if we can run status without crashing
    let mut cmd = Command::cargo_bin("wallp").unwrap();
    cmd.arg("status");
    let output = cmd.output().expect("Failed to execute command");
    assert!(output.status.success());
}
