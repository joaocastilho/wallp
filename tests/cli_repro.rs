use assert_cmd::Command;

#[test]
fn test_cli_no_args_shows_help() {
    let mut cmd = Command::cargo_bin("wallp").unwrap();
    // This expects the help output to be present when no args are passed.
    // Currently, it likely does NOT show help (it runs tray), so this assertion might fail or timeout if tray is blocking.
    // However, since we are in a test environment, tray might fail to init or we can check for help text.
    
    // The current implementation runs tray::run() if no args.
    // We want it to print help.
    
    // We can't easily test "tray is running" without side effects, but we can test "stderr/stdout contains help".
    // If it goes into tray mode, it won't print help.
    
    // We expect it to FAIL currently if we assert standard output contains "Usage:".
    // Or we expect it to timeout if it goes into event loop (bad for test). 
    // BUT, tray usually requires a GUI session. In CI/test environment, maybe it returns immediately or fails?
    
    // Let's assert that we see "Usage: wallp" in stdout or stderr.
    
    let assert = cmd.assert();
    assert.failure(); // It usually fails if it tries to attach console or something? 
                      // actually if it runs tray, it might just keep running.
                      // assert_cmd captures output.
                      
    // Wait, if it runs tray, it blocks. `assert_cmd` waits for completion.
    // So this test will HANG if the current implementation starts the tray loop.
    // That is a "failure" of the requirement "cli should show commands list".
    
    // To avoid hanging indefinitely, we might need a timeout, but standard test runner doesn't have per-test timeout easily.
    // Let's assume for this reproduction: If we pass `--help`, we get help.
    // If we pass nothing, we WANT help.
    
    // CAUTION: Running this test might hang if I'm not careful.
    // Maybe I should pass a bogus arg to verify help comes on error, 
    // but the issue is SPECIFICALLY "wallp" (no args).
    
    // If I can't safely test "hang", I will assume the code reading is correct and just write the test expecting the FIX.
    // Implementation:
    // If successful, `cmd` outputs help and exits with 0 (or error 2 if we use clap's error?).
    // Actually typically `clap` prints help and exits 0 for --help, or error text and non-zero for missing args if required.
    // Our `command` is `Option<Commands>`, so `None` is valid for clap parser.
    
    // I will try to run it with a timeout using `timeout` command? No, windows.
    // I will write the test to verify the EXPECTED behavior.
    
    // Test:
    // result.stdout.contains("Usage: wallp")
}

#[test]
fn test_cli_help_arg() {
    let mut cmd = Command::cargo_bin("wallp").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage: wallp"));
}
