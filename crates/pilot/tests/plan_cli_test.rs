use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_plan_help_surface() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("pilot")?;
    cmd.arg("plan")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("issues"))
        .stdout(predicates::str::contains("score"))
        .stdout(predicates::str::contains("roadmap"));
    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_plan_issues_score_roadmap_flow() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let input = temp.path().join("issues.json");
    let issues = r#"
[
  {"id":1,"title":"Security hardening","body":"critical auth","labels":["security"],"state":"open","html_url":"https://example.com/1"},
  {"id":2,"title":"Cleanup docs","body":"small","labels":["size/s"],"state":"open","html_url":"https://example.com/2"}
]
"#;
    fs::write(&input, issues)?;

    let scored = temp.path().join("scored.json");
    let roadmap = temp.path().join("roadmap.md");

    let mut cmd_issues = Command::cargo_bin("pilot")?;
    cmd_issues
        .arg("plan")
        .arg("issues")
        .arg("--input")
        .arg(input.to_string_lossy().to_string())
        .arg("--output")
        .arg(
            temp.path()
                .join("cached_issues.json")
                .to_string_lossy()
                .to_string(),
        )
        .assert()
        .success()
        .stdout(predicates::str::contains("Cached 2 issues"));

    let mut cmd_score = Command::cargo_bin("pilot")?;
    cmd_score
        .arg("plan")
        .arg("score")
        .arg("--input")
        .arg(input.to_string_lossy().to_string())
        .arg("--output")
        .arg(scored.to_string_lossy().to_string())
        .assert()
        .success()
        .stdout(predicates::str::contains("Wrote 2 scored items"));

    let mut cmd_roadmap = Command::cargo_bin("pilot")?;
    cmd_roadmap
        .arg("plan")
        .arg("roadmap")
        .arg("--input")
        .arg(scored.to_string_lossy().to_string())
        .arg("--output")
        .arg(roadmap.to_string_lossy().to_string())
        .arg("--top-n")
        .arg("1")
        .assert()
        .success()
        .stdout(predicates::str::contains("Wrote roadmap with 1 items"));

    assert!(roadmap.exists());
    Ok(())
}
