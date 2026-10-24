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

#[test]
fn fails_on_candidate_bunka_replacements_without_changing_the_output() {
    let output = command()
        .args([
            "--report",
            "none",
            "--fail-on-unresolved",
            "tests/fixtures/candidate-bunka.txt",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "膨大な資料\n");
}

#[test]
fn reports_candidate_bunka_replacements_as_json_without_failing_by_default() {
    let output = command()
        .args(["--report", "json", "tests/fixtures/candidate-bunka.txt"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "膨大な資料\n");
    let report: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        report["unresolved_bunka_replacements"],
        serde_json::json!([{"source": "膨大", "target": "厖大", "count": 1}])
    );
}
