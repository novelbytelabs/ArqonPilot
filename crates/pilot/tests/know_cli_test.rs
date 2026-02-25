use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_know_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("know")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("record"))
        .stdout(predicates::str::contains("query"));
    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_know_record_and_query() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    fs::create_dir_all(&home)?;

    let mut record = Command::cargo_bin("pilot")?;
    record
        .env("HOME", &home)
        .arg("know")
        .arg("record")
        .arg("--title")
        .arg("Adopt wave5 modules")
        .arg("--context")
        .arg("Need faster cross-repo operations")
        .arg("--decision")
        .arg("Implement plan/create/know now")
        .arg("--tag")
        .arg("wave5")
        .assert()
        .success()
        .stdout(predicates::str::contains("Recorded decision"));

    let mut query = Command::cargo_bin("pilot")?;
    query
        .env("HOME", &home)
        .arg("know")
        .arg("query")
        .arg("--query")
        .arg("wave5")
        .assert()
        .success()
        .stdout(predicates::str::contains("Adopt wave5 modules"));

    Ok(())
}
