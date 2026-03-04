use assert_cmd::Command;
use chrono::{Duration, Utc};
use serde_json::Value;
use std::fs;
use std::path::Path;
use tokio_postgres::NoTls;
use uuid::Uuid;

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

fn agorg_id_from_show(
    home: &Path,
    pilot_home: &Path,
    port: &str,
) -> Result<Uuid, Box<dyn std::error::Error>> {
    let out = pilot_cmd()?
        .env("HOME", home)
        .env("PILOT_HOME", pilot_home)
        .env("PILOT_DB_PORT", port)
        .env("PILOT_DB_MODE", "unix_socket")
        .args(["agorg", "show"])
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "agorg show failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    let v: Value = serde_json::from_slice(&out.stdout)?;
    let id = v
        .get("id")
        .and_then(Value::as_str)
        .ok_or("agorg show missing id")?;
    Ok(Uuid::parse_str(id)?)
}

fn db_dsn_from_status(
    home: &Path,
    pilot_home: &Path,
    port: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let out = pilot_cmd()?
        .env("HOME", home)
        .env("PILOT_HOME", pilot_home)
        .env("PILOT_DB_PORT", port)
        .env("PILOT_DB_MODE", "unix_socket")
        .args(["db", "status"])
        .output()?;
    if !out.status.success() {
        return Err(format!("db status failed: {}", String::from_utf8_lossy(&out.stderr)).into());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let dsn = stdout
        .lines()
        .find_map(|line| line.strip_prefix("DSN: ").map(|v| v.trim().to_string()))
        .ok_or_else(|| format!("db status missing DSN line. stdout={stdout}"))?;
    Ok(dsn)
}

fn run_reconcile(
    home: &Path,
    pilot_home: &Path,
    port: &str,
    agorg: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let out = pilot_cmd()?
        .env("HOME", home)
        .env("PILOT_HOME", pilot_home)
        .env("PILOT_DB_PORT", port)
        .env("PILOT_DB_MODE", "unix_socket")
        .args(["agorg", "reconcile", "--agorg", agorg])
        .output()?;

    if !out.status.success() {
        return Err(format!(
            "agorg reconcile failed. stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }

    Ok(serde_json::from_slice(&out.stdout)?)
}

#[test]
#[allow(deprecated)]
fn test_reconcile_surfaces_security_policy_violations_in_fleet_report(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp_root = Path::new(".pilot_test_tmp");
    fs::create_dir_all(temp_root)?;
    let temp = tempfile::Builder::new()
        .prefix("agorg_p3_reconcile_")
        .tempdir_in(temp_root)?;

    let home = temp.path().join("home");
    let pilot_home_dir = tempfile::Builder::new()
        .prefix("pdb9351_")
        .tempdir_in("/tmp")?;
    let pilot_home = pilot_home_dir.path().to_path_buf();
    let master_root = temp.path().join("master");
    let repo_root = master_root.join("RepoA");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&master_root)?;

    if skip_if_db_env_denied(&home, &pilot_home, "9351")? {
        return Ok(());
    }

    write_repo_with_secret(&repo_root, "OrgA");

    // Create AGOrg and import AGO candidate.
    pilot_cmd()?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9351")
        .env("PILOT_DB_MODE", "unix_socket")
        .args([
            "agorg",
            "create-project",
            "--name",
            "OrgA",
            "--root",
            master_root.to_string_lossy().as_ref(),
            "--autoscan",
            "--import",
            "--default-scope",
        ])
        .assert()
        .success();

    // Activate security policy with naked-secret block.
    let policy_file = temp.path().join("security_policy.json");
    setup_policy_security(&home, &pilot_home, "9351", &policy_file)?;

    let agorg_id = agorg_id_from_show(&home, &pilot_home, "9351")?;
    let report = run_reconcile(&home, &pilot_home, "9351", &agorg_id.to_string())?;

    let governance_issues = report
        .get("governance_issues")
        .and_then(Value::as_array)
        .ok_or("missing governance_issues")?;
    assert!(
        governance_issues.iter().any(|i| {
            i.get("policy_kind") == Some(&Value::String("security".to_string()))
                && i.get("issue_type") == Some(&Value::String("policy_violation".to_string()))
        }),
        "expected security policy_violation in governance_issues, got: {governance_issues:?}"
    );

    let fleet_report = report.get("fleet_report").ok_or("missing fleet_report")?;
    let statuses = fleet_report
        .get("ago_statuses")
        .and_then(Value::as_array)
        .ok_or("fleet_report missing ago_statuses")?;
    assert!(!statuses.is_empty(), "expected non-empty ago_statuses");

    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_reconcile_reports_orphan_and_expired_overrides() -> Result<(), Box<dyn std::error::Error>> {
    let temp_root = Path::new(".pilot_test_tmp");
    fs::create_dir_all(temp_root)?;
    let temp = tempfile::Builder::new()
        .prefix("agorg_p3_override_")
        .tempdir_in(temp_root)?;

    let home = temp.path().join("home");
    let pilot_home_dir = tempfile::Builder::new()
        .prefix("pdb9352_")
        .tempdir_in("/tmp")?;
    let pilot_home = pilot_home_dir.path().to_path_buf();
    let master_root = temp.path().join("master");
    let repo_root = master_root.join("RepoB");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&master_root)?;

    if skip_if_db_env_denied(&home, &pilot_home, "9352")? {
        return Ok(());
    }

    write_repo_with_secret(&repo_root, "OrgB");

    pilot_cmd()?
        .env("HOME", &home)
        .env("PILOT_HOME", &pilot_home)
        .env("PILOT_DB_PORT", "9352")
        .env("PILOT_DB_MODE", "unix_socket")
        .args([
            "agorg",
            "create-project",
            "--name",
            "OrgB",
            "--root",
            master_root.to_string_lossy().as_ref(),
            "--autoscan",
            "--import",
            "--default-scope",
        ])
        .assert()
        .success();

    // Need a parent active policy version for override registry metadata.
    let policy_file = temp.path().join("security_policy_b.json");
    setup_policy_security(&home, &pilot_home, "9352", &policy_file)?;

    let agorg_id = agorg_id_from_show(&home, &pilot_home, "9352")?;
    let dsn = db_dsn_from_status(&home, &pilot_home, "9352")?;

    // Insert an orphan + expired override row directly.
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let cfg: tokio_postgres::Config = dsn.parse().map_err(|e| format!("bad dsn: {e}"))?;
        let (client, conn) = cfg.connect(NoTls).await.map_err(|e| e.to_string())?;
        tokio::spawn(async move {
            let _ = conn.await;
        });

        client
            .execute(
                "INSERT INTO agorg_policy_overrides \
                 (id, agorg_id, ago_path, policy_kind, reason, ticket_ref, owner, expires_at, parent_policy_version, override_policy_version) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
                &[
                    &Uuid::new_v4(),
                    &agorg_id,
                    &"/tmp/nonexistent/RepoGhost".to_string(),
                    &"security".to_string(),
                    &"temp exception".to_string(),
                    &Some("TICKET-1".to_string()),
                    &"tester".to_string(),
                    &(Utc::now() - Duration::hours(2)),
                    &1i32,
                    &1i32,
                ],
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok::<(), String>(())
    })
    .map_err(|e| format!("override insert failed: {e}"))?;

    let report = run_reconcile(&home, &pilot_home, "9352", &agorg_id.to_string())?;
    let governance_issues = report
        .get("governance_issues")
        .and_then(Value::as_array)
        .ok_or("missing governance_issues")?;

    assert!(
        governance_issues.iter().any(|i| {
            i.get("issue_type") == Some(&Value::String("orphan_override".to_string()))
        }),
        "expected orphan_override in governance_issues, got: {governance_issues:?}"
    );
    assert!(
        governance_issues.iter().any(|i| {
            i.get("issue_type") == Some(&Value::String("expired_override".to_string()))
        }),
        "expected expired_override in governance_issues, got: {governance_issues:?}"
    );

    Ok(())
}
