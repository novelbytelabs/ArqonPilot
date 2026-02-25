use assert_cmd::Command;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_report_json_output() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;

    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.current_dir(temp.path())
        .arg("--report-json")
        .arg("init")
        .assert()
        .success()
        .stdout(predicates::str::contains("\"command\":\"init\""))
        .stdout(predicates::str::contains("\"success\":true"));

    Ok(())
}
