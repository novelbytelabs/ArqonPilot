use assert_cmd::Command;

#[test]
#[allow(deprecated)]
fn test_serve_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("serve")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("--ws-url"))
        .stdout(predicates::str::contains("--channel"))
        .stdout(predicates::str::contains("--telemetry-channel"))
        .stdout(predicates::str::contains("--once"));

    Ok(())
}
