use assert_cmd::Command;

#[test]
#[allow(deprecated)]
fn test_oracle_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("oracle")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("scan"))
        .stdout(predicates::str::contains("query"));

    Ok(())
}
