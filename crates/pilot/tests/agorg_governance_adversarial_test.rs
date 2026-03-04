use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;

fn pilot_cmd() -> Result<Command, Box<dyn std::error::Error>> {
    Ok(Command::cargo_bin("pilot")?)
}

fn skip_if_db_env_denied(
    home: &Path,
    pilot_home: &Path,
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
        || stderr.contains("could not create any Unix-domain sockets")
        || stderr.contains("Unix-domain socket path")
    {
        eprintln!("Skipping test: managed Postgres shared-memory denied by runtime environment.");
        return Ok(true);
    }
    Ok(false)
}

fn git_init(path: &Path) {
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
        .expect("git config user.email failed");

    std::process::Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("test")
        .current_dir(path)
        .output()
        .expect("git config user.name failed");

    fs::write(path.join("README.md"), "test").expect("write README failed");
    std::process::Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .expect("git add failed");
    std::process::Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("initial")
        .current_dir(path)
        .output()
        .expect("git commit failed");
}

fn write_repo_with_secret(repo_path: &Path, parent: &str) {
    fs::create_dir_all(repo_path).expect("create repo dir failed");
    git_init(repo_path);
    fs::write(
        repo_path.join("pyproject.toml"),
        format!(
            "[project]\nname=\"{}\"\nversion=\"0.1.0\"\n\n[tool.arqon.relationships]\nparent=\"{}\"\nchildren=[]\n",
            repo_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("repo"),
            parent
        ),
    )
    .expect("write pyproject failed");
    fs::write(
        repo_path.join("app.js"),
        "const SECRET = AKIA1234567890ABCDEF;\n",
    )
    .expect("write app.js failed");
}

fn setup_policy_security(
    home: &Path,
    pilot_home: &Path,
    port: &str,
    policy_file: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        policy_file,
        r#"{
  "kind": "security",
  "version": 1,
  "max_cve_severity": "critical",
  "block_naked_secrets": { "level": "block", "enabled": true }
}"#,
    )?;

    pilot_cmd()?
        .env("HOME", home)
        .env("PILOT_HOME", pilot_home)
        .env("PILOT_DB_PORT", port)
        .env("PILOT_DB_MODE", "unix_socket")
        .args([
            "policy",
            "set-draft",
            "--kind",
            "security",
            "--file",
            policy_file.to_string_lossy().as_ref(),
        ])
        .assert()
        .success();

    pilot_cmd()?
        .env("HOME", home)
        .env("PILOT_HOME", pilot_home)
        .env("PILOT_DB_PORT", port)
        .env("PILOT_DB_MODE", "unix_socket")
        .args(["policy", "activate", "--kind", "security", "--version", "1"])
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_reconcile_requires_active_or_explicit_agorg() -> Result<(), Box<dyn std::error::Error>> {
    let temp_root = Path::new(".pilot_test_tmp");
    fs::create_dir_all(temp_root)?;
    let temp = tempfile::Builder::new()
        .prefix("agorg_p3_adv_no_scope_")
        .tempdir_in(temp_root)?;
    let home = temp.path().join("home");
    let pilot_home_dir = tempfile::Builder::new()
        .prefix("pdb9353_")
        .tempdir_in("/tmp")?;
    let pilot_home = pilot_home_dir.path().to_path_buf();
    fs::create_dir_all(&home)?;

    if skip_if_db_env_denied(&home, &pilot_home, "9353")? {
        return Ok(());
    }

    let out = pilot_cmd()?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9353")
        .env("PILOT_DB_MODE", "unix_socket")
        .args(["agorg", "reconcile"])
        .output()?;

    assert!(!out.status.success(), "reconcile without scope should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("No active AGOrg") || stderr.contains("pass --agorg"),
        "expected no-active-agorg error, got: {stderr}"
    );
    Ok(())
}

#[test]
fn test_reconcile_malformed_agorg_id_fails_cleanly() -> Result<(), Box<dyn std::error::Error>> {
    let out = pilot_cmd()?
        .args(["agorg", "reconcile", "--agorg", "not-a-valid-uuid"])
        .output()?;
    assert!(
        !out.status.success(),
        "reconcile with invalid agorg-id should fail; got exit 0"
    );
    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_reconcile_enforces_inherited_parent_policy() -> Result<(), Box<dyn std::error::Error>> {
    let temp_root = Path::new(".pilot_test_tmp");
    fs::create_dir_all(temp_root)?;
    let temp = tempfile::Builder::new()
        .prefix("agorg_p3_adv_inherit_")
        .tempdir_in(temp_root)?;

    let home = temp.path().join("home");
    let pilot_home_dir = tempfile::Builder::new()
        .prefix("pdb9354_")
        .tempdir_in("/tmp")?;
    let pilot_home = pilot_home_dir.path().to_path_buf();
    let parent_root = temp.path().join("parent_root");
    let child_root = temp.path().join("child_root");
    let child_repo = child_root.join("RepoChild");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&parent_root)?;
    fs::create_dir_all(&child_root)?;

    if skip_if_db_env_denied(&home, &pilot_home, "9354")? {
        return Ok(());
    }

    write_repo_with_secret(&child_repo, "ChildOrg");

    // Create parent and child AGOrgs.
    pilot_cmd()?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9354")
        .env("PILOT_DB_MODE", "unix_socket")
        .args([
            "agorg",
            "create",
            "--name",
            "ParentOrg",
            "--root",
            parent_root.to_string_lossy().as_ref(),
        ])
        .assert()
        .success();

    pilot_cmd()?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9354")
        .env("PILOT_DB_MODE", "unix_socket")
        .args([
            "agorg",
            "create-project",
            "--name",
            "ChildOrg",
            "--root",
            child_root.to_string_lossy().as_ref(),
            "--parent",
            "ParentOrg",
            "--autoscan",
            "--import",
            "--default-scope",
        ])
        .assert()
        .success();

    // Parent defines active security policy.
    pilot_cmd()?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9354")
        .env("PILOT_DB_MODE", "unix_socket")
        .args(["agorg", "use", "ParentOrg"])
        .assert()
        .success();

    let policy_file = temp.path().join("security_parent_policy.json");
    setup_policy_security(&home, &pilot_home, "9354", &policy_file)?;

    // Switch to child and reconcile: inherited parent policy should still flag secret.
    let out = pilot_cmd()?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9354")
        .env("PILOT_DB_MODE", "unix_socket")
        .args(["agorg", "use", "ChildOrg"])
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "agorg use ChildOrg failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }

    let reconcile = pilot_cmd()?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9354")
        .env("PILOT_DB_MODE", "unix_socket")
        .args(["agorg", "reconcile", "--agorg", "ChildOrg"])
        .output()?;

    if !reconcile.status.success() {
        return Err(format!(
            "reconcile failed. stdout={} stderr={}",
            String::from_utf8_lossy(&reconcile.stdout),
            String::from_utf8_lossy(&reconcile.stderr)
        )
        .into());
    }

    let v: Value = serde_json::from_slice(&reconcile.stdout)?;
    let governance_issues = v
        .get("governance_issues")
        .and_then(Value::as_array)
        .ok_or("missing governance_issues")?;

    assert!(
        governance_issues.iter().any(|i| {
            i.get("policy_kind") == Some(&Value::String("security".to_string()))
                && i.get("issue_type") == Some(&Value::String("policy_violation".to_string()))
        }),
        "expected inherited parent security violation in child reconcile: {governance_issues:?}"
    );

    Ok(())
}

#[test]
fn test_policy_invalid_kind_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let out = pilot_cmd()?
        .args(["policy", "scan", "--kind", "zzz_invalid_kind_xyz"])
        .output()?;
    assert!(
        !out.status.success(),
        "policy scan invalid kind should fail; got exit 0"
    );
    Ok(())
}
