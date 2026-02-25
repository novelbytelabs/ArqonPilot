use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
#[allow(deprecated)]
fn test_wave6_dry_run_orchestration_and_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let home = temp.path().join("home");
    let repo_a = temp.path().join("repo-a");
    let repo_b = temp.path().join("repo-b");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo_a)?;
    fs::create_dir_all(&repo_b)?;
    fs::write(
        repo_a.join("Cargo.toml"),
        "[package]\nname=\"repo-a\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )?;
    fs::write(
        repo_b.join("Cargo.toml"),
        "[package]\nname=\"repo-b\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )?;

    for repo in [&repo_a, &repo_b] {
        std::process::Command::new("git")
            .arg("init")
            .current_dir(repo)
            .output()?;
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo)
            .output()?;
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo)
            .output()?;
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()?;
        std::process::Command::new("git")
            .args(["commit", "-m", "chore: init"])
            .current_dir(repo)
            .output()?;
    }

    for (name, path) in [("repo-a", &repo_a), ("repo-b", &repo_b)] {
        let mut register = Command::cargo_bin("pilot")?;
        register
            .env("HOME", &home)
            .arg("multi")
            .arg("register")
            .arg("--path")
            .arg(path.to_string_lossy().to_string())
            .arg("--name")
            .arg(name)
            .arg("--group")
            .arg("core")
            .assert()
            .success();
    }

    let mut deps = Command::cargo_bin("pilot")?;
    deps.env("HOME", &home)
        .arg("multi")
        .arg("deps")
        .arg("set")
        .arg("--repo")
        .arg("repo-b")
        .arg("--depends-on")
        .arg("repo-a")
        .arg("--dry-run")
        .assert()
        .success();

    let mut order = Command::cargo_bin("pilot")?;
    order
        .env("HOME", &home)
        .arg("multi")
        .arg("order")
        .arg("--group")
        .arg("core")
        .assert()
        .success()
        .stdout(predicates::str::contains("1. repo-a"));

    let mut branch = Command::cargo_bin("pilot")?;
    branch
        .env("HOME", &home)
        .arg("branch")
        .arg("create")
        .arg("feat/wave6-dryrun")
        .arg("--group")
        .arg("core")
        .arg("--dry-run")
        .assert()
        .success();

    let mut nav = Command::cargo_bin("pilot")?;
    nav.env("HOME", &home)
        .arg("navigate")
        .arg("--multi")
        .arg("--dry-run")
        .arg("--group")
        .arg("core")
        .assert()
        .success()
        .stdout(predicates::str::contains("Coordinated release order"));

    let mut secure = Command::cargo_bin("pilot")?;
    secure
        .env("HOME", &home)
        .arg("secure")
        .arg("fix")
        .arg("--group")
        .arg("core")
        .assert()
        .success();

    let audit = home.join(".pilot").join("audit.jsonl");
    let reports_dir = home.join(".pilot").join("reports");
    assert!(audit.exists());
    assert!(reports_dir.exists());
    let report_count = fs::read_dir(&reports_dir)?.count();
    assert!(report_count > 0);
    Ok(())
}
