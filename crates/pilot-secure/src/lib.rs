use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureFinding {
    pub category: String,
    pub severity: String,
    pub rule: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub message: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureScanReport {
    pub repo_path: PathBuf,
    pub findings: Vec<SecureFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureFixAction {
    pub repo_path: PathBuf,
    pub command: String,
    pub applied: bool,
    pub success: bool,
    pub message: String,
}

pub fn scan_repo(repo_path: &Path) -> Result<SecureScanReport> {
    let mut findings = Vec::new();
    findings.extend(scan_secrets(repo_path)?);
    findings.extend(scan_dependencies(repo_path));

    Ok(SecureScanReport {
        repo_path: repo_path.to_path_buf(),
        findings,
    })
}

pub fn fix_repo(repo_path: &Path, dry_run: bool) -> Result<Vec<SecureFixAction>> {
    let mut actions = Vec::new();
    let has_cargo = repo_path.join("Cargo.toml").exists();
    let has_python_requirements = repo_path.join("requirements.txt").exists();

    if has_cargo {
        actions.push(run_fix_step(
            repo_path,
            "cargo update",
            vec!["cargo", "update"],
            dry_run,
        ));
        actions.push(run_fix_step(
            repo_path,
            "cargo check",
            vec!["cargo", "check"],
            dry_run,
        ));
    }

    if has_python_requirements {
        actions.push(run_fix_step(
            repo_path,
            "pip-audit --fix -r requirements.txt",
            vec!["pip-audit", "--fix", "-r", "requirements.txt"],
            dry_run,
        ));
    }

    if !has_cargo && !has_python_requirements {
        actions.push(SecureFixAction {
            repo_path: repo_path.to_path_buf(),
            command: "none".to_string(),
            applied: false,
            success: true,
            message: "No supported dependency manifest found".to_string(),
        });
    }

    Ok(actions)
}

fn run_fix_step(repo_path: &Path, display: &str, cmd: Vec<&str>, dry_run: bool) -> SecureFixAction {
    if dry_run {
        return SecureFixAction {
            repo_path: repo_path.to_path_buf(),
            command: display.to_string(),
            applied: false,
            success: true,
            message: format!("[DRY RUN] Would run: {}", display),
        };
    }

    if !git_clean(repo_path).unwrap_or(false) {
        return SecureFixAction {
            repo_path: repo_path.to_path_buf(),
            command: display.to_string(),
            applied: false,
            success: false,
            message: "Repo is not clean; refusing to mutate".to_string(),
        };
    }

    let mut process = Command::new(cmd[0]);
    process.args(&cmd[1..]).current_dir(repo_path);
    match process.output() {
        Ok(out) if out.status.success() => SecureFixAction {
            repo_path: repo_path.to_path_buf(),
            command: display.to_string(),
            applied: true,
            success: true,
            message: "Applied successfully".to_string(),
        },
        Ok(out) => SecureFixAction {
            repo_path: repo_path.to_path_buf(),
            command: display.to_string(),
            applied: true,
            success: false,
            message: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        },
        Err(err) => SecureFixAction {
            repo_path: repo_path.to_path_buf(),
            command: display.to_string(),
            applied: true,
            success: false,
            message: err.to_string(),
        },
    }
}

fn scan_dependencies(repo_path: &Path) -> Vec<SecureFinding> {
    let mut findings = Vec::new();
    let cargo_toml = repo_path.join("Cargo.toml");
    if cargo_toml.exists() {
        findings.extend(scan_cargo_audit(repo_path));
    }

    let requirements = repo_path.join("requirements.txt");
    if requirements.exists() {
        findings.extend(scan_pip_audit(repo_path, &requirements));
    }

    findings
}

fn scan_cargo_audit(repo_path: &Path) -> Vec<SecureFinding> {
    let out = Command::new("cargo")
        .args(["audit", "--json"])
        .current_dir(repo_path)
        .output();

    match out {
        Ok(o) if o.status.success() => parse_cargo_audit_json(&o.stdout),
        Ok(_) => vec![SecureFinding {
            category: "dependency".to_string(),
            severity: "info".to_string(),
            rule: "cargo-audit-unavailable".to_string(),
            file: Some("Cargo.toml".to_string()),
            line: None,
            message: "cargo audit failed; run manually for advisory details".to_string(),
            recommendation: "Install cargo-audit and run `cargo audit --json`".to_string(),
        }],
        Err(_) => vec![SecureFinding {
            category: "dependency".to_string(),
            severity: "info".to_string(),
            rule: "cargo-audit-not-installed".to_string(),
            file: Some("Cargo.toml".to_string()),
            line: None,
            message: "cargo-audit not available in PATH".to_string(),
            recommendation: "Install cargo-audit for Rust CVE scanning".to_string(),
        }],
    }
}

fn parse_cargo_audit_json(raw: &[u8]) -> Vec<SecureFinding> {
    let value: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let mut findings = Vec::new();
    if let Some(list) = value
        .get("vulnerabilities")
        .and_then(|v| v.get("list"))
        .and_then(|v| v.as_array())
    {
        for item in list {
            let advisory = item.get("advisory");
            let id = advisory
                .and_then(|a| a.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("RUSTSEC-UNKNOWN");
            let title = advisory
                .and_then(|a| a.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("Rust dependency vulnerability");
            let pkg = item
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            findings.push(SecureFinding {
                category: "dependency".to_string(),
                severity: "high".to_string(),
                rule: id.to_string(),
                file: Some("Cargo.lock".to_string()),
                line: None,
                message: format!("{} ({})", title, pkg),
                recommendation: format!("Run `cargo update -p {}` and retest", pkg),
            });
        }
    }
    findings
}

fn scan_pip_audit(repo_path: &Path, requirements: &Path) -> Vec<SecureFinding> {
    let out = Command::new("pip-audit")
        .args([
            "-r",
            requirements.to_string_lossy().as_ref(),
            "--format",
            "json",
        ])
        .current_dir(repo_path)
        .output();

    match out {
        Ok(o) if o.status.success() => parse_pip_audit_json(&o.stdout),
        Ok(_) => vec![SecureFinding {
            category: "dependency".to_string(),
            severity: "info".to_string(),
            rule: "pip-audit-unavailable".to_string(),
            file: Some("requirements.txt".to_string()),
            line: None,
            message: "pip-audit failed; run manually for advisory details".to_string(),
            recommendation: "Install pip-audit and run `pip-audit -r requirements.txt`".to_string(),
        }],
        Err(_) => vec![SecureFinding {
            category: "dependency".to_string(),
            severity: "info".to_string(),
            rule: "pip-audit-not-installed".to_string(),
            file: Some("requirements.txt".to_string()),
            line: None,
            message: "pip-audit not available in PATH".to_string(),
            recommendation: "Install pip-audit for Python CVE scanning".to_string(),
        }],
    }
}

fn parse_pip_audit_json(raw: &[u8]) -> Vec<SecureFinding> {
    let value: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let mut findings = Vec::new();
    if let Some(deps) = value.as_array() {
        for dep in deps {
            let name = dep
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            if let Some(vulns) = dep.get("vulns").and_then(|v| v.as_array()) {
                for vuln in vulns {
                    let id = vuln
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("PYSEC-UNKNOWN");
                    let fix = vuln
                        .get("fix_versions")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                        .unwrap_or("latest");
                    findings.push(SecureFinding {
                        category: "dependency".to_string(),
                        severity: "high".to_string(),
                        rule: id.to_string(),
                        file: Some("requirements.txt".to_string()),
                        line: None,
                        message: format!("Vulnerability in Python dependency {}", name),
                        recommendation: format!("Upgrade {} to {}", name, fix),
                    });
                }
            }
        }
    }
    findings
}

fn scan_secrets(repo_path: &Path) -> Result<Vec<SecureFinding>> {
    let mut findings = Vec::new();
    let skip_dirs: HashSet<&str> = [".git", "target", "node_modules", ".venv", "venv"]
        .into_iter()
        .collect();

    let patterns = vec![
        (
            "secret.aws_access_key",
            "high",
            Regex::new(r"AKIA[0-9A-Z]{16}")?,
            "Potential AWS access key exposed",
        ),
        (
            "secret.github_token",
            "high",
            Regex::new(r"ghp_[A-Za-z0-9]{36}")?,
            "Potential GitHub token exposed",
        ),
        (
            "secret.openai_key",
            "high",
            Regex::new(r"sk-[A-Za-z0-9]{20,}")?,
            "Potential API key exposed",
        ),
        (
            "secret.private_key",
            "critical",
            Regex::new(r"-----BEGIN (RSA|EC|OPENSSH|PRIVATE) KEY-----")?,
            "Private key material detected",
        ),
    ];

    for entry in WalkDir::new(repo_path)
        .into_iter()
        .filter_entry(|e| {
            if let Some(name) = e.file_name().to_str() {
                return !skip_dirs.contains(name);
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() || !is_scannable_file(path) {
            continue;
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed reading {}", path.display()));
        let content = match content {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (line_no, line) in content.lines().enumerate() {
            for (rule, severity, regex, msg) in &patterns {
                if regex.is_match(line) {
                    let rel = path
                        .strip_prefix(repo_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();
                    findings.push(SecureFinding {
                        category: "secret".to_string(),
                        severity: (*severity).to_string(),
                        rule: (*rule).to_string(),
                        file: Some(rel),
                        line: Some(line_no + 1),
                        message: (*msg).to_string(),
                        recommendation: "Move secret to env/config vault and rotate credential"
                            .to_string(),
                    });
                }
            }
        }
    }

    Ok(findings)
}

fn is_scannable_file(path: &Path) -> bool {
    let ext = match path.extension().and_then(|s| s.to_str()) {
        Some(e) => e,
        None => return false,
    };
    matches!(
        ext,
        "rs" | "py"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "toml"
            | "yaml"
            | "yml"
            | "json"
            | "env"
            | "txt"
            | "md"
            | "sh"
    )
}

fn git_clean(repo_path: &Path) -> Result<bool> {
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .with_context(|| format!("Failed to run git status in {}", repo_path.display()))?;
    Ok(out.status.success() && out.stdout.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_scan_finds_key_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("lib.rs");
        std::fs::write(&src, "const K: &str = \"AKIAABCDEFGHIJKLMNOP\";").unwrap();
        let findings = scan_secrets(dir.path()).unwrap();
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule == "secret.aws_access_key"));
    }
}
