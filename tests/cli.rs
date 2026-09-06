use std::process::Command;

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_segzify"))
}

#[test]
fn prints_version() {
    let output = command().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("segzify "));
}

#[test]
fn rejects_check_with_output() {
    let output = command()
        .args(["--check", "--output", "out.txt"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(64));
}

#[test]
fn rejects_unknown_arguments_with_usage_status() {
    let output = command().arg("--bogus").output().unwrap();
    assert_eq!(output.status.code(), Some(64));
}
