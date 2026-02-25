use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_regression_report_json_on_plan_input_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let bad_input = temp.path().join("issues.json");
    fs::write(&bad_input, "{ this-is-not-valid-json ]")?;

    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("--report-json")
        .arg("plan")
        .arg("issues")
        .arg("--input")
        .arg(bad_input.to_string_lossy().to_string())
        .assert()
        .failure()
        .stdout(predicates::str::contains("\"command\":\"plan.issues\""))
        .stdout(predicates::str::contains("\"success\":false"));

    Ok(())
}
