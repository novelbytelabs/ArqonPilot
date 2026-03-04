use assert_cmd::Command;
use std::fs;

fn skip_if_db_env_denied(
    home: &std::path::Path,
    pilot_home: &std::path::Path,
    port: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let out = Command::cargo_bin("pilot")?
        .env("HOME", home)
        .env("PILOT_HOME", pilot_home)
        .env("PILOT_DB_PORT", port)
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("db")
        .arg("start")
        .output()?;
    if out.status.success() {
        let _ = Command::cargo_bin("pilot")?
            .env("HOME", home)
            .env("PILOT_HOME", pilot_home)
            .env("PILOT_DB_PORT", port)
            .env("PILOT_DB_MODE", "unix_socket")
            .arg("db")
            .arg("stop")
            .output();
        return Ok(false);
    }
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if stderr.contains("Operation not permitted")
        || (stderr.contains("Permission denied") && stderr.contains("shared memory"))
        || stderr.contains("could not open shared memory segment")
    {
        eprintln!("Skipping test: managed Postgres socket/tcp bind denied by runtime environment.");
        return Ok(true);
    }
    Ok(false)
}

fn git_init(path: &std::path::Path) {
    std::process::Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .expect("Failed to git init");

    std::process::Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("test")
        .current_dir(path)
        .output()
        .unwrap();

    fs::write(path.join("README.md"), "test").unwrap();
    std::process::Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial")
        .current_dir(path)
        .output()
        .unwrap();
}

#[test]
#[allow(deprecated)]
fn test_policy_workflow_draft_preview_scan() -> Result<(), Box<dyn std::error::Error>> {
    let temp_root = std::path::Path::new(".pilot_test_tmp");
    fs::create_dir_all(temp_root)?;
    let temp = tempfile::Builder::new()
        .prefix("policy_e2e_")
        .tempdir_in(temp_root)?;
    let suffix = temp
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("x");
    let org_name = format!("WorkflowOrg-{}", suffix);
    let home = temp.path().join("home");
    let pilot_home = std::path::PathBuf::from("/tmp/pilotdb_a9342");
    let repo_path = temp.path().join("workflow-repo");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&repo_path)?;
    if skip_if_db_env_denied(&home, &pilot_home, "9342")? {
        return Ok(());
    }

    git_init(&repo_path);

    // 1. Setup AGOrg
    let create_out = Command::cargo_bin("pilot")?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9342")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("agorg")
        .arg("create")
        .arg("--name")
        .arg(&org_name)
        .arg("--root")
        .arg(temp.path().to_string_lossy().to_string())
        .output()?;
    if !create_out.status.success() {
        let stderr = String::from_utf8_lossy(&create_out.stderr);
        if (stderr.contains("Permission denied") && stderr.contains("shared memory"))
            || stderr.contains("could not open shared memory segment")
            || stderr.contains("Operation not permitted")
        {
            eprintln!(
                "Skipping test: managed Postgres shared-memory denied by runtime environment."
            );
            return Ok(());
        }
        let full = format!(
            "agorg create failed unexpectedly.\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&create_out.stdout),
            stderr
        );
        return Err(full.into());
    }

    let mut use_cmd = Command::cargo_bin("pilot")?;
    use_cmd
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9342")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("agorg")
        .arg("use")
        .arg(&org_name)
        .assert()
        .success();

    // 2. Register repo
    let mut reg_cmd = Command::cargo_bin("pilot")?;
    reg_cmd
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9342")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("multi")
        .arg("register")
        .arg("--path")
        .arg(repo_path.to_string_lossy().to_string())
        .arg("--name")
        .arg("workflow-repo")
        .assert()
        .success();

    // 3. Insert a violation (naked secret)
    fs::write(
        repo_path.join("app.js"),
        "const SECRET = 'AKIA1234567890ABCDEF';",
    )?;

    // 4. Set Security Policy Draft
    let policy_file = temp.path().join("security_policy.json");
    fs::write(
        &policy_file,
        r#"{
        "kind": "security",
        "version": 1,
        "max_cve_severity": "critical",
        "block_naked_secrets": { "level": "block", "enabled": true }
    }"#,
    )?;

    let mut set_cmd = Command::cargo_bin("pilot")?;
    set_cmd
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9342")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("set-draft")
        .arg("--kind")
        .arg("security")
        .arg("--file")
        .arg(&policy_file)
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Saved draft policy security version 1",
        ));

    // 5. Preview draft v1
    let mut preview_cmd = Command::cargo_bin("pilot")?;
    preview_cmd
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9342")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("preview")
        .arg("--kind")
        .arg("security")
        .arg("--version")
        .arg("1")
        .assert()
        .success()
        .stdout(predicates::str::contains("\"status\": \"blocked\""))
        .stdout(predicates::str::contains("\"violations\": 1"))
        .stdout(predicates::str::contains("Artifact: "));

    // 6. Activate policy
    let mut act_cmd = Command::cargo_bin("pilot")?;
    act_cmd
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9342")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("activate")
        .arg("--kind")
        .arg("security")
        .arg("--version")
        .arg("1")
        .assert()
        .success()
        .stdout(predicates::str::contains("Activated security v1"));

    // 7. Scan active policy
    let mut scan_cmd = Command::cargo_bin("pilot")?;
    scan_cmd
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9342")
        .env("PILOT_DB_MODE", "unix_socket")
        .arg("policy")
        .arg("scan")
        .arg("--kind")
        .arg("security")
        .assert()
        .success()
        .stdout(predicates::str::contains("\"violations\": 1"));

    Ok(())
}
