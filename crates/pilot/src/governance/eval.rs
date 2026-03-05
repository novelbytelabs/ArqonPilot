use super::model::*;
use crate::agorg::AgorgStore;
use crate::governance::store::GovernanceStore;
use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub fn evaluate_branch_policy(
    policy: &BranchPolicy,
    action: &str, // "create", "sync", "prune"
    branch_name: &str,
    exceptions: &[PolicyException],
    current_ago_path: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) -> PolicyEvalReport {
    let mut report = PolicyEvalReport::default();
    // naming policy
    if action == "create" || action == "sync" {
        eval_naming(
            &policy.naming,
            branch_name,
            exceptions,
            &mut report,
            current_ago_path,
            source_name,
            source_id,
        );
        eval_protected(
            &policy.protected_branches,
            branch_name,
            exceptions,
            &mut report,
            current_ago_path,
            source_name,
            source_id,
        );
    }

    if action == "prune" {
        eval_protected_prune(
            &policy.protected_branches,
            branch_name,
            exceptions,
            &mut report,
            current_ago_path,
            source_name,
            source_id,
        );
    }

    report.blocked = report
        .violations
        .iter()
        .any(|v| v.level == EnforcementLevel::Block);
    report
}

fn eval_naming(
    policy: &NamingPolicy,
    branch_name: &str,
    exceptions: &[PolicyException],
    report: &mut PolicyEvalReport,
    current_ago_path: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) {
    if policy.level == EnforcementLevel::Off {
        return;
    }

    if is_excepted("naming", exceptions, current_ago_path) {
        return;
    }

    let n = branch_name.trim();
    let mut parts = n.splitn(2, &policy.separator);
    let prefix = parts.next().unwrap_or_default();
    let rest = parts.next().unwrap_or_default();

    if !policy.required_prefix.iter().any(|p| p == prefix) || rest.is_empty() {
        let allowed = policy.required_prefix.join(", ");
        let msg = format!(
            "Missing required prefix. Got '{}', expected one of: {}",
            branch_name, allowed
        );
        add_result(
            report,
            "naming.required_prefix",
            &policy.level,
            branch_name,
            &msg,
            &format!(
                "Rename to '{}{}{}'",
                policy
                    .required_prefix
                    .first()
                    .unwrap_or(&"feat".to_string()),
                policy.separator,
                branch_name
            ),
            source_name,
            source_id,
        );
        return; // stop evaluating naming if prefix is bad
    }

    // Body format check: assuming kebab-case for now based on legacy
    if policy.body_format == "kebab-case" {
        let valid = rest
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !rest.starts_with('-')
            && !rest.ends_with('-')
            && !rest.contains("--");

        if !valid {
            add_result(
                report,
                "naming.body_format",
                &policy.level,
                branch_name,
                &format!("Body '{}' does not match {}", rest, policy.body_format),
                "Use only lowercase letters, numbers, and single hyphens",
                source_name,
                source_id,
            );
        }
    }

    if branch_name.len() > policy.max_length {
        add_result(
            report,
            "naming.max_length",
            &policy.level,
            branch_name,
            &format!(
                "Branch name is {} chars, max allowed is {}",
                branch_name.len(),
                policy.max_length
            ),
            "Shorten the branch name",
            source_name,
            source_id,
        );
    }
}

fn eval_protected(
    policy: &ProtectedBranchesPolicy,
    branch_name: &str,
    exceptions: &[PolicyException],
    report: &mut PolicyEvalReport,
    current_ago_path: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) {
    if policy.level == EnforcementLevel::Off {
        return;
    }

    if is_excepted("protected_branches", exceptions, current_ago_path) {
        return;
    }

    for pattern in &policy.patterns {
        if match_pattern(pattern, branch_name) {
            add_result(
                report,
                "protected_branches.patterns",
                &policy.level,
                branch_name,
                &format!(
                    "Branch '{}' is protected by pattern '{}'",
                    branch_name, pattern
                ),
                "You cannot mutate a protected branch. Use a feature branch instead.",
                source_name,
                source_id,
            );
            return;
        }
    }
}

fn eval_protected_prune(
    policy: &ProtectedBranchesPolicy,
    branch_name: &str,
    exceptions: &[PolicyException],
    report: &mut PolicyEvalReport,
    current_ago_path: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) {
    if policy.level == EnforcementLevel::Off {
        return;
    }
    if is_excepted("protected_branches", exceptions, current_ago_path) {
        return;
    }

    // Prune checks the same protected patterns
    for pattern in &policy.patterns {
        if match_pattern(pattern, branch_name) {
            add_result(
                report,
                "protected_branches.patterns",
                &policy.level,
                branch_name,
                &format!(
                    "Cannot prune protected branch '{}' (matches '{}')",
                    branch_name, pattern
                ),
                "Protected branches cannot be pruned.",
                source_name,
                source_id,
            );
            return;
        }
    }
}

fn match_pattern(pattern: &str, value: &str) -> bool {
    if pattern.ends_with("/*") {
        let prefix = &pattern[0..pattern.len() - 1]; // keep the slash
        value.starts_with(prefix) || value == &pattern[0..pattern.len() - 2]
    } else {
        pattern == value
    }
}

fn is_excepted(rule_prefix: &str, exceptions: &[PolicyException], ago_path: &str) -> bool {
    let now = Utc::now();
    exceptions.iter().any(|e| {
        let path_matches = match &e.ago_path {
            Some(p) => p == ago_path,
            None => true, // AGOrg-wide exception
        };
        path_matches && e.rule_path.starts_with(rule_prefix) && e.expires_at > now
    })
}

fn add_result(
    report: &mut PolicyEvalReport,
    rule: &str,
    level: &EnforcementLevel,
    input: &str,
    violation: &str,
    fix: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) {
    let res = PolicyEvalResult {
        rule: rule.to_string(),
        level: level.clone(),
        input: input.to_string(),
        violation: violation.to_string(),
        fix_suggestion: fix.to_string(),
        policy_source: "Evaluated".to_string(), // Legacy field, keeping for now
        policy_source_id: source_id,
        policy_source_name: source_name.to_string(),
        override_available: *level == EnforcementLevel::Warn,
    };

    match level {
        EnforcementLevel::Off => {}
        EnforcementLevel::Info => report.infos.push(res),
        EnforcementLevel::Warn => report.warnings.push(res),
        EnforcementLevel::Block => report.violations.push(res),
        EnforcementLevel::AutoFix => report.auto_fixes.push(res),
    }
}

// -----------------------------------------------------------------------------
// NEW FAMILIES EVALUATION
// -----------------------------------------------------------------------------

pub fn evaluate_dependency_policy(
    policy: &DependencyPolicy,
    repo_path: &Path,
    exceptions: &[PolicyException],
    current_ago_path: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) -> PolicyEvalReport {
    let mut report = PolicyEvalReport::default();

    if policy.require_lockfile.level != EnforcementLevel::Off {
        if !is_excepted("dependency.require_lockfile", exceptions, current_ago_path) {
            let has_cargo_lock = repo_path.join("Cargo.lock").exists();
            let has_package_lock = repo_path.join("package-lock.json").exists();
            let has_poetry_lock = repo_path.join("poetry.lock").exists();
            let has_pnpm_lock = repo_path.join("pnpm-lock.yaml").exists();

            if !(has_cargo_lock || has_package_lock || has_poetry_lock || has_pnpm_lock) {
                add_result(
                    &mut report,
                    "require_lockfile",
                    &policy.require_lockfile.level,
                    "Lockfile check",
                    "No recognized lockfile found (Cargo.lock, package-lock.json, poetry.lock, pnpm-lock.yaml)",
                    "Generate and commit a lockfile",
                    source_name,
                    source_id,
                );
            }
        }
    }

    // DEP-001: Banned Packages & DEP-002: Allowed Licenses
    if (policy.banned_packages.level != EnforcementLevel::Off
        && !policy.banned_packages.items.is_empty())
        || (policy.allowed_licenses.level != EnforcementLevel::Off
            && !policy.allowed_licenses.items.is_empty())
    {
        // Scan Cargo.toml
        let cargo_toml = repo_path.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                match toml::from_str::<toml::Value>(&content) {
                    Ok(doc) => {
                        // Check License
                        if policy.allowed_licenses.level != EnforcementLevel::Off
                            && !is_excepted("dependency.DEP-002", exceptions, current_ago_path)
                        {
                            let license = doc
                                .get("package")
                                .and_then(|p| p.get("license"))
                                .and_then(|l| l.as_str());
                            if let Some(l) = license {
                                if !policy.allowed_licenses.items.iter().any(|item| item == l) {
                                    add_result(
                                        &mut report,
                                        "DEP-002",
                                        &policy.allowed_licenses.level,
                                        l,
                                        &format!("Disallowed license '{}'", l),
                                        "Use an approved license (MIT, Apache-2.0, etc.)",
                                        source_name,
                                        source_id,
                                    );
                                }
                            }
                        }
                        // Check Dependencies
                        if policy.banned_packages.level != EnforcementLevel::Off
                            && !is_excepted("dependency.DEP-001", exceptions, current_ago_path)
                        {
                            let deps = ["dependencies", "dev-dependencies", "build-dependencies"];
                            for group in deps {
                                if let Some(d) = doc.get(group).and_then(|v| v.as_table()) {
                                    for (name, _) in d {
                                        if policy
                                            .banned_packages
                                            .items
                                            .iter()
                                            .any(|b| name.contains(b))
                                        {
                                            add_result(
                                                &mut report,
                                                "DEP-001",
                                                &policy.banned_packages.level,
                                                name,
                                                &format!(
                                                    "Banned package '{}' detected in {}",
                                                    name, group
                                                ),
                                                "Remove the banned dependency",
                                                source_name,
                                                source_id,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Silent fail for malformed TOML in production, but we could log to tracing
                    }
                }
            }
        }

        // Scan package.json
        let pkg_json = repo_path.join("package.json");
        if pkg_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                if let Ok(json) = serde_json::from_str::<Value>(&content) {
                    // Check License
                    if policy.allowed_licenses.level != EnforcementLevel::Off
                        && !is_excepted("dependency.DEP-002", exceptions, current_ago_path)
                    {
                        if let Some(l) = json.get("license").and_then(|v| v.as_str()) {
                            if !policy.allowed_licenses.items.iter().any(|item| item == l) {
                                add_result(
                                    &mut report,
                                    "DEP-002",
                                    &policy.allowed_licenses.level,
                                    l,
                                    &format!("Disallowed license '{}' in package.json", l),
                                    "Update to an approved license",
                                    source_name,
                                    source_id,
                                );
                            }
                        }
                    }
                    // Check Dependencies
                    if policy.banned_packages.level != EnforcementLevel::Off
                        && !is_excepted("dependency.DEP-001", exceptions, current_ago_path)
                    {
                        let groups = ["dependencies", "devDependencies", "peerDependencies"];
                        for group in groups {
                            if let Some(d) = json.get(group).and_then(|v| v.as_object()) {
                                for (name, _) in d {
                                    if policy
                                        .banned_packages
                                        .items
                                        .iter()
                                        .any(|b| name.contains(b))
                                    {
                                        add_result(
                                            &mut report,
                                            "DEP-001",
                                            &policy.banned_packages.level,
                                            name,
                                            &format!(
                                                "Banned package '{}' detected in {}",
                                                name, group
                                            ),
                                            "Remove or replace the banned dependency",
                                            source_name,
                                            source_id,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    report.blocked = report
        .violations
        .iter()
        .any(|v| v.level == EnforcementLevel::Block);
    report
}

pub fn evaluate_release_policy(
    policy: &ReleasePolicy,
    repo_path: &Path,
    exceptions: &[PolicyException],
    current_ago_path: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) -> PolicyEvalReport {
    let mut report = PolicyEvalReport::default();

    if policy.require_changelog.level != EnforcementLevel::Off {
        if !is_excepted("release.require_changelog", exceptions, current_ago_path) {
            if !repo_path.join("CHANGELOG.md").exists() {
                add_result(
                    &mut report,
                    "require_changelog",
                    &policy.require_changelog.level,
                    "Changelog check",
                    "CHANGELOG.md file is missing",
                    "Create a CHANGELOG.md file in the repository root",
                    source_name,
                    source_id,
                );
            }
        }
    }

    // REL-001: Forbidden Days (UTC)
    if policy.forbidden_days.level != EnforcementLevel::Off
        && !policy.forbidden_days.items.is_empty()
    {
        if !is_excepted("release.REL-001", exceptions, current_ago_path) {
            let today = Utc::now().weekday().to_string(); // e.g. "Friday"
            if policy
                .forbidden_days
                .items
                .iter()
                .any(|d| d.eq_ignore_ascii_case(&today))
            {
                add_result(
                    &mut report,
                    "REL-001",
                    &policy.forbidden_days.level,
                    &today,
                    &format!("Deployment forbidden on {}", today),
                    "Wait for a permitted deployment window",
                    source_name,
                    source_id,
                );
            }
        }
    }

    // REL-002: SemVer requirement
    if policy.require_semver.level != EnforcementLevel::Off && policy.require_semver.enabled {
        if !is_excepted("release.REL-002", exceptions, current_ago_path) {
            let cargo_toml = repo_path.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                    if let Ok(doc) = content.parse::<toml::Value>() {
                        let version = doc
                            .get("package")
                            .and_then(|p| p.get("version"))
                            .and_then(|v| v.as_str());
                        if let Some(v) = version {
                            // Simple semver regex check
                            if !v.starts_with(|c: char| c.is_ascii_digit())
                                || v.split('.').count() < 2
                            {
                                add_result(
                                    &mut report,
                                    "REL-002",
                                    &policy.require_semver.level,
                                    v,
                                    &format!("Version '{}' does not follow SemVer", v),
                                    "Use x.y.z versioning format",
                                    source_name,
                                    source_id,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    report.blocked = report
        .violations
        .iter()
        .any(|v| v.level == EnforcementLevel::Block);
    report
}

pub fn evaluate_security_policy(
    policy: &SecurityPolicy,
    repo_path: &Path,
    exceptions: &[PolicyException],
    current_ago_path: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) -> PolicyEvalReport {
    let mut report = PolicyEvalReport::default();

    // SEC-001: Vulnerability Threshold
    if let Ok(scan) = pilot_secure::scan_repo(repo_path) {
        let max_severity_rank = severity_rank(&policy.max_cve_severity);

        for finding in scan.findings {
            let finding_rank = severity_rank(&finding.severity);

            // Only report if it meets the category or exceeds threshold
            let is_secret = finding.category == "secret";
            let exceeds_threshold = finding_rank >= max_severity_rank && finding_rank > 0;

            if exceeds_threshold {
                if !is_excepted("security.SEC-001", exceptions, current_ago_path) {
                    let file = finding
                        .file
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string());
                    add_result(
                        &mut report,
                        "SEC-001",
                        &EnforcementLevel::Block, // Vulnerability threshold violations are fixed at Block
                        &file,
                        &finding.message,
                        &finding.recommendation,
                        source_name,
                        source_id,
                    );
                }
            } else if is_secret
                && policy.block_naked_secrets.enabled
                && policy.block_naked_secrets.level != EnforcementLevel::Off
            {
                if !is_excepted("security.SEC-002", exceptions, current_ago_path) {
                    let file = finding
                        .file
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string());
                    add_result(
                        &mut report,
                        "SEC-002",
                        &policy.block_naked_secrets.level,
                        &file,
                        &finding.message,
                        &finding.recommendation,
                        source_name,
                        source_id,
                    );
                }
            }
        }
    }

    report.blocked = report
        .violations
        .iter()
        .any(|v| v.level == EnforcementLevel::Block);
    report
}

fn severity_rank(s: &str) -> i32 {
    match s.to_lowercase().as_str() {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

pub fn evaluate_quality_policy(
    policy: &QualityPolicy,
    repo_path: &Path,
    exceptions: &[PolicyException],
    current_ago_path: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) -> PolicyEvalReport {
    let mut report = PolicyEvalReport::default();

    // QUAL-001: Coverage check
    if policy.require_coverage.level != EnforcementLevel::Off && policy.require_coverage.enabled {
        if !is_excepted("quality.QUAL-001", exceptions, current_ago_path) {
            let coverage_file = repo_path.join("coverage.xml");
            if coverage_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&coverage_file) {
                    // Crude regex-like search for line-rate="0.X"
                    // e.g. <coverage line-rate="0.85" ...>
                    if let Some(pos) = content.find("line-rate=\"") {
                        let start = pos + 11;
                        if let Some(end) = content[start..].find('"') {
                            let rate_str = &content[start..start + end];
                            if let Ok(rate) = rate_str.parse::<f32>() {
                                if rate < (policy.min_test_coverage / 100.0) {
                                    add_result(
                                        &mut report,
                                        "QUAL-001",
                                        &policy.require_coverage.level,
                                        rate_str,
                                        &format!(
                                            "Coverage {}% is below required {}%",
                                            rate * 100.0,
                                            policy.min_test_coverage
                                        ),
                                        "Increase test coverage and regenerate report",
                                        source_name,
                                        source_id,
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                add_result(
                    &mut report,
                    "QUAL-002",
                    &policy.require_coverage.level,
                    "coverage.xml",
                    "Missing coverage report (coverage.xml)",
                    "Run tests with coverage and output Cobertura XML",
                    source_name,
                    source_id,
                );
            }
        }
    }

    // Lint check contract
    if policy.require_lint_pass.level != EnforcementLevel::Off && policy.require_lint_pass.enabled {
        if !is_excepted("quality.QUAL-002", exceptions, current_ago_path) {
            if !repo_path.join("lint.json").exists() && !repo_path.join("clippy.json").exists() {
                add_result(
                    &mut report,
                    "QUAL-002",
                    &policy.require_lint_pass.level,
                    "lint.json",
                    "Missing lint report (lint.json or clippy.json)",
                    "Run linter and export results to JSON",
                    source_name,
                    source_id,
                );
            }
        }
    }

    report.blocked = report
        .violations
        .iter()
        .any(|v| v.level == EnforcementLevel::Block);
    report
}

pub fn evaluate_runtime_policy(
    policy: &RuntimePolicy,
    repo_path: &Path,
    exceptions: &[PolicyException],
    current_ago_path: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) -> PolicyEvalReport {
    let mut report = PolicyEvalReport::default();

    let dockerfile = repo_path.join("Dockerfile");
    if policy.require_dockerfile.level != EnforcementLevel::Off && policy.require_dockerfile.enabled
    {
        if !is_excepted("runtime.require_dockerfile", exceptions, current_ago_path) {
            if !dockerfile.exists() {
                add_result(
                    &mut report,
                    "require_dockerfile",
                    &policy.require_dockerfile.level,
                    "Dockerfile check",
                    "Dockerfile is missing",
                    "Add a Dockerfile to the repository root",
                    source_name,
                    source_id,
                );
            }
        }
    }

    if dockerfile.exists() {
        if let Ok(content) = std::fs::read_to_string(&dockerfile) {
            let lines: Vec<_> = content.lines().collect();

            // RUN-001: Allowed Base Images
            if policy.allowed_base_images.level != EnforcementLevel::Off
                && !policy.allowed_base_images.items.is_empty()
            {
                if !is_excepted("runtime.RUN-001", exceptions, current_ago_path) {
                    for line in &lines {
                        if line.trim_start().to_uppercase().starts_with("FROM ") {
                            let base = line.trim_start()[5..]
                                .split_whitespace()
                                .next()
                                .unwrap_or_default();
                            if !policy
                                .allowed_base_images
                                .items
                                .iter()
                                .any(|item| base.to_lowercase().starts_with(&item.to_lowercase()))
                            {
                                add_result(
                                    &mut report,
                                    "RUN-001",
                                    &policy.allowed_base_images.level,
                                    base,
                                    &format!("Untrusted base image '{}'", base),
                                    "Use an approved base image (e.g. alpine, node:22-bookworm)",
                                    source_name,
                                    source_id,
                                );
                            }
                        }
                    }
                }
            }

            // RUN-002: Healthcheck check
            if policy.require_healthcheck.level != EnforcementLevel::Off
                && policy.require_healthcheck.enabled
            {
                if !is_excepted("runtime.RUN-002", exceptions, current_ago_path) {
                    if !content.contains("HEALTHCHECK") {
                        add_result(
                            &mut report,
                            "RUN-002",
                            &policy.require_healthcheck.level,
                            "Dockerfile",
                            "Missing HEALTHCHECK instruction",
                            "Add HEALTHCHECK to the Dockerfile",
                            source_name,
                            source_id,
                        );
                    }
                }
            }
        }
    }

    report.blocked = report
        .violations
        .iter()
        .any(|v| v.level == EnforcementLevel::Block);
    report
}

#[derive(Debug, Clone, Default)]
pub struct OperatorRoutineContext {
    pub action: String,
    pub has_active_scope: bool,
    pub repo_registered: bool,
    pub current_branch: Option<String>,
    pub repo_clean: Option<bool>,
    pub completed_steps: Vec<String>,
}

fn has_completed_step(completed: &[String], step: &str) -> bool {
    completed
        .iter()
        .any(|s| s.trim().eq_ignore_ascii_case(step))
}

pub fn evaluate_operator_routine_policy(
    policy: &OperatorRoutinePolicy,
    context: &OperatorRoutineContext,
    exceptions: &[PolicyException],
    current_ago_path: &str,
    source_name: &str,
    source_id: Option<Uuid>,
) -> PolicyEvalReport {
    let mut report = PolicyEvalReport::default();
    let action = context.action.trim().to_ascii_lowercase();
    let is_push = action == "push";

    if policy.require_active_scope.level != EnforcementLevel::Off
        && policy.require_active_scope.enabled
        && !is_excepted("operator_routine.ORT-001", exceptions, current_ago_path)
        && !context.has_active_scope
    {
        add_result(
            &mut report,
            "ORT-001",
            &policy.require_active_scope.level,
            "active_scope=false",
            "No active AGOrg scope is selected",
            "Select an active AGOrg before running governance actions",
            source_name,
            source_id,
        );
    }

    if policy.require_registered_repo.level != EnforcementLevel::Off
        && policy.require_registered_repo.enabled
        && !is_excepted("operator_routine.ORT-002", exceptions, current_ago_path)
        && !context.repo_registered
    {
        add_result(
            &mut report,
            "ORT-002",
            &policy.require_registered_repo.level,
            "repo_registered=false",
            "Current repository is not registered in active AGOrg scope",
            "Register the repository under Multi/AGOrg before running this action",
            source_name,
            source_id,
        );
    }

    if is_push
        && policy.require_clean_worktree_for_push.level != EnforcementLevel::Off
        && policy.require_clean_worktree_for_push.enabled
        && !is_excepted("operator_routine.ORT-003", exceptions, current_ago_path)
    {
        let clean = context.repo_clean.unwrap_or(false);
        if !clean {
            add_result(
                &mut report,
                "ORT-003",
                &policy.require_clean_worktree_for_push.level,
                "repo_clean=false",
                "Push requested from a dirty working tree",
                "Commit, stash, or discard local changes before push",
                source_name,
                source_id,
            );
        }
    }

    if is_push
        && policy.allowed_push_branches.level != EnforcementLevel::Off
        && !policy.allowed_push_branches.items.is_empty()
        && !is_excepted("operator_routine.ORT-004", exceptions, current_ago_path)
    {
        let branch = context
            .current_branch
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let allowed = policy
            .allowed_push_branches
            .items
            .iter()
            .any(|b| b.eq_ignore_ascii_case(branch.trim()));
        if !allowed {
            add_result(
                &mut report,
                "ORT-004",
                &policy.allowed_push_branches.level,
                &branch,
                &format!("Push from disallowed branch '{}'", branch),
                "Switch to an allowed branch or update operator_routine policy",
                source_name,
                source_id,
            );
        }
    }

    if is_push
        && policy.required_prepush_steps.level != EnforcementLevel::Off
        && !policy.required_prepush_steps.items.is_empty()
        && !is_excepted("operator_routine.ORT-005", exceptions, current_ago_path)
    {
        let missing: Vec<String> = policy
            .required_prepush_steps
            .items
            .iter()
            .filter(|s| !has_completed_step(&context.completed_steps, s))
            .cloned()
            .collect();
        if !missing.is_empty() {
            add_result(
                &mut report,
                "ORT-005",
                &policy.required_prepush_steps.level,
                &missing.join(","),
                &format!(
                    "Missing required routine steps before push: {}",
                    missing.join(", ")
                ),
                "Run the required pre-push routine steps first",
                source_name,
                source_id,
            );
        }
    }

    report.blocked = report
        .violations
        .iter()
        .any(|v| v.level == EnforcementLevel::Block);
    report
}

// -----------------------------------------------------------------------------
// FLEET SCAN
// -----------------------------------------------------------------------------

pub async fn fleet_governance_scan(
    store: &GovernanceStore,
    agorg_store: &AgorgStore,
    agorg_id: Uuid,
) -> miette::Result<GovernanceReconcileReport> {
    let mut agos = vec![];

    // Using a fast approach by fetching AGO paths under this agorg_id
    if let Ok(list) = agorg_store.get_agos(agorg_id).await {
        agos = list;
    }

    let mut statuses = Vec::new();
    let mut total_violations = 0;

    for ago in &agos {
        let repo_path = PathBuf::from(&ago.repo_path);

        let branch_trace = store
            .resolve_with_trace(agorg_id, &ago.repo_path, "branch")
            .await?;
        let branch_pol: BranchPolicy = get_policy_from_trace(store, &branch_trace)
            .await?
            .unwrap_or_default();
        let branch_ex = store.get_effective_exceptions(agorg_id, "branch").await?;
        let current_branch = get_current_branch(&repo_path).unwrap_or_else(|| "main".to_string());
        let branch_eval = evaluate_branch_policy(
            &branch_pol,
            "sync",
            &current_branch,
            &branch_ex,
            &ago.repo_path,
            &branch_trace.resolved_source,
            Some(branch_trace.resolved_agorg_id),
        );
        total_violations += branch_eval.violations.len();

        let dep_trace = store
            .resolve_with_trace(agorg_id, &ago.repo_path, "dependency")
            .await?;
        let dep_pol: DependencyPolicy = get_policy_from_trace(store, &dep_trace)
            .await?
            .unwrap_or_default();
        let dep_ex = store
            .get_effective_exceptions(agorg_id, "dependency")
            .await?;
        let dep_eval = evaluate_dependency_policy(
            &dep_pol,
            &repo_path,
            &dep_ex,
            &ago.repo_path,
            &dep_trace.resolved_source,
            Some(dep_trace.resolved_agorg_id),
        );
        total_violations += dep_eval.violations.len();

        let rel_trace = store
            .resolve_with_trace(agorg_id, &ago.repo_path, "release")
            .await?;
        let rel_pol: ReleasePolicy = get_policy_from_trace(store, &rel_trace)
            .await?
            .unwrap_or_default();
        let rel_ex = store.get_effective_exceptions(agorg_id, "release").await?;
        let rel_eval = evaluate_release_policy(
            &rel_pol,
            &repo_path,
            &rel_ex,
            &ago.repo_path,
            &rel_trace.resolved_source,
            Some(rel_trace.resolved_agorg_id),
        );
        total_violations += rel_eval.violations.len();

        let sec_trace = store
            .resolve_with_trace(agorg_id, &ago.repo_path, "security")
            .await?;
        let sec_pol: SecurityPolicy = get_policy_from_trace(store, &sec_trace)
            .await?
            .unwrap_or_default();
        let sec_ex = store.get_effective_exceptions(agorg_id, "security").await?;
        let sec_eval = evaluate_security_policy(
            &sec_pol,
            &repo_path,
            &sec_ex,
            &ago.repo_path,
            &sec_trace.resolved_source,
            Some(sec_trace.resolved_agorg_id),
        );
        total_violations += sec_eval.violations.len();

        let qual_trace = store
            .resolve_with_trace(agorg_id, &ago.repo_path, "quality")
            .await?;
        let qual_pol: QualityPolicy = get_policy_from_trace(store, &qual_trace)
            .await?
            .unwrap_or_default();
        let qual_ex = store.get_effective_exceptions(agorg_id, "quality").await?;
        let qual_eval = evaluate_quality_policy(
            &qual_pol,
            &repo_path,
            &qual_ex,
            &ago.repo_path,
            &qual_trace.resolved_source,
            Some(qual_trace.resolved_agorg_id),
        );
        total_violations += qual_eval.violations.len();

        let run_trace = store
            .resolve_with_trace(agorg_id, &ago.repo_path, "runtime")
            .await?;
        let run_pol: RuntimePolicy = get_policy_from_trace(store, &run_trace)
            .await?
            .unwrap_or_default();
        let run_ex = store.get_effective_exceptions(agorg_id, "runtime").await?;
        let run_eval = evaluate_runtime_policy(
            &run_pol,
            &repo_path,
            &run_ex,
            &ago.repo_path,
            &run_trace.resolved_source,
            Some(run_trace.resolved_agorg_id),
        );
        total_violations += run_eval.violations.len();

        let routine_trace = store
            .resolve_with_trace(agorg_id, &ago.repo_path, "operator_routine")
            .await?;
        let routine_pol: OperatorRoutinePolicy = get_policy_from_trace(store, &routine_trace)
            .await?
            .unwrap_or_default();
        let routine_ex = store
            .get_effective_exceptions(agorg_id, "operator_routine")
            .await?;
        let routine_ctx = OperatorRoutineContext {
            action: "push".to_string(),
            has_active_scope: true,
            repo_registered: true,
            current_branch: Some(current_branch.clone()),
            repo_clean: Some(repo_is_clean(&repo_path).unwrap_or(false)),
            completed_steps: vec![
                "policy".to_string(),
                "hook".to_string(),
                "drift".to_string(),
            ],
        };
        let routine_eval = evaluate_operator_routine_policy(
            &routine_pol,
            &routine_ctx,
            &routine_ex,
            &ago.repo_path,
            &routine_trace.resolved_source,
            Some(routine_trace.resolved_agorg_id),
        );
        total_violations += routine_eval.violations.len();

        // Include trace contexts
        // No conflict_trace field on PolicyEvalReport, we compute is_overridden directly
        let is_overridden = branch_trace.resolved_source == "ago_override"
            || dep_trace.resolved_source == "ago_override"
            || rel_trace.resolved_source == "ago_override"
            || sec_trace.resolved_source == "ago_override"
            || qual_trace.resolved_source == "ago_override"
            || run_trace.resolved_source == "ago_override"
            || routine_trace.resolved_source == "ago_override";

        let overall = if branch_eval.blocked
            || dep_eval.blocked
            || rel_eval.blocked
            || sec_eval.blocked
            || qual_eval.blocked
            || run_eval.blocked
            || routine_eval.blocked
        {
            "violation"
        } else if !branch_eval.warnings.is_empty()
            || !dep_eval.warnings.is_empty()
            || !rel_eval.warnings.is_empty()
            || !sec_eval.warnings.is_empty()
            || !qual_eval.warnings.is_empty()
            || !run_eval.warnings.is_empty()
            || !routine_eval.warnings.is_empty()
        {
            "warning"
        } else {
            "compliant"
        };

        statuses.push(AgoComplianceStatus {
            ago_path: ago.repo_path.clone(),
            ago_name: ago.name.clone(),
            overall_status: overall.to_string(),
            is_overridden,
            evaluations: [
                ("branch".to_string(), branch_eval),
                ("dependency".to_string(), dep_eval),
                ("release".to_string(), rel_eval),
                ("security".to_string(), sec_eval),
                ("quality".to_string(), qual_eval),
                ("runtime".to_string(), run_eval),
                ("operator_routine".to_string(), routine_eval),
            ]
            .into_iter()
            .collect(),
        });
    }

    let agorg_name = match agorg_store.get_agorg(agorg_id).await {
        Ok(Some(agorg)) => agorg.name,
        _ => "Unknown".to_string(),
    };

    let compliant_count = statuses
        .iter()
        .filter(|s| s.overall_status == "compliant")
        .count();
    let warning_count = statuses
        .iter()
        .filter(|s| s.overall_status == "warning")
        .count();

    Ok(GovernanceReconcileReport {
        agorg_id,
        agorg_name,
        timestamp: Utc::now(),
        total_agos: agos.len(),
        compliant_count,
        violation_count: total_violations,
        warning_count,
        ago_statuses: statuses,
    })
}

async fn get_policy_from_trace<T: serde::de::DeserializeOwned>(
    store: &GovernanceStore,
    trace: &PolicyConflictTrace,
) -> miette::Result<Option<T>> {
    let rec = store
        .get_policy_by_version(
            trace.resolved_agorg_id,
            Some(&trace.ago_path),
            &trace.policy_kind,
            trace.resolved_version,
        )
        .await?;
    let val = match rec {
        Some(r) => r.policy_json,
        None => {
            let rec_no_ago = store
                .get_policy_by_version(
                    trace.resolved_agorg_id,
                    None,
                    &trace.policy_kind,
                    trace.resolved_version,
                )
                .await?;
            if let Some(r2) = rec_no_ago {
                r2.policy_json
            } else {
                return Ok(None);
            }
        }
    };
    Ok(serde_json::from_value(val).ok())
}

fn get_current_branch(repo_path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        None
    } else {
        Some(branch)
    }
}

fn repo_is_clean(repo_path: &Path) -> Option<bool> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("status")
        .arg("--porcelain")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(output.stdout.is_empty())
}

/// Resolve the strongest applicable confirmation requirement for a branch operation.
/// Returns (ConfirmationType, optional required phrase).
pub fn required_confirmation(
    policy: &BranchPolicy,
    action: &str,
    branch: &str,
) -> (ConfirmationType, Option<String>) {
    // Prune always uses lifecycle confirmation
    if action == "prune" {
        return (
            policy.lifecycle.prune_confirmation_type.clone(),
            Some(policy.lifecycle.confirmation_phrase.clone()),
        );
    }

    // Check if branch matches any protected pattern
    let is_protected = policy.protected_branches.patterns.iter().any(|pat| {
        if pat.ends_with('*') {
            let prefix = &pat[..pat.len() - 1];
            branch.starts_with(prefix)
        } else {
            branch == pat
        }
    });

    if is_protected {
        return (
            policy.protected_branches.confirmation_type.clone(),
            policy.protected_branches.confirmation_phrase.clone(),
        );
    }

    // For destructive actions, use mutation control policy
    if action == "delete" || action == "force-push" {
        if policy.mutation_control.protected_branch_confirmation {
            return (
                policy
                    .mutation_control
                    .destructive_confirmation_type
                    .clone(),
                Some("CONFIRM".to_string()),
            );
        }
    }

    // Non-protected, non-prune: standard confirmation
    (ConfirmationType::Standard, None)
}

/// Command allowlist check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowlistCheckResult {
    pub allowed: bool,
    pub category: Option<String>,
    pub requires_confirmation: bool,
    pub blocked_reason: Option<String>,
}

/// Check if a command is allowed by the mutation control policy
pub fn check_command_allowlist(
    policy: &MutationControlPolicy,
    command: &str,
) -> AllowlistCheckResult {
    // Check blocked commands list
    for blocked in &policy.command_allowlist.blocked_commands {
        if command.to_lowercase().contains(&blocked.to_lowercase()) {
            return AllowlistCheckResult {
                allowed: false,
                category: None,
                requires_confirmation: false,
                blocked_reason: Some(format!("Command '{}' is explicitly blocked", command)),
            };
        }
    }

    // Check if command requires confirmation
    let requires_confirmation = policy
        .command_allowlist
        .confirmation_required
        .iter()
        .any(|c| command.to_lowercase().contains(&c.to_lowercase()));

    // Determine category - use command prefix matching to avoid false positives
    let command_lower = command.to_lowercase();
    let category = if command_lower.starts_with("list")
        || command_lower.starts_with("status")
        || command_lower.starts_with("query")
        || command_lower.starts_with("show")
        || command_lower.starts_with("get")
    {
        Some("read".to_string())
    } else if command_lower.starts_with("create") {
        Some("branch_create".to_string())
    } else if command_lower.starts_with("sync") || command_lower.starts_with("merge") {
        Some("branch_modify".to_string())
    } else if command_lower.starts_with("prune") || command_lower.starts_with("delete") {
        Some("branch_destroy".to_string())
    } else if command_lower.starts_with("policy") {
        Some("policy".to_string())
    } else if command_lower.starts_with("release") {
        Some("release".to_string())
    } else if command_lower.starts_with("service") || command_lower.starts_with("db") {
        Some("admin".to_string())
    } else {
        None
    };

    // Check if category is enabled
    if let Some(ref cat) = category {
        let enabled = policy
            .command_allowlist
            .enabled_categories
            .iter()
            .any(|c| c.as_str() == cat);

        if !enabled {
            return AllowlistCheckResult {
                allowed: false,
                category: Some(cat.clone()),
                requires_confirmation: false,
                blocked_reason: Some(format!("Category '{}' is not enabled", cat)),
            };
        }
    }

    AllowlistCheckResult {
        allowed: true,
        category,
        requires_confirmation,
        blocked_reason: None,
    }
}

/// Redact secrets from a string using the policy's redaction patterns
pub fn redact_secrets(policy: &MutationControlPolicy, input: &str) -> String {
    if !policy.secrets_safe_logging {
        return input.to_string();
    }

    let mut result = input.to_string();

    for pattern in &policy.redaction_patterns {
        match regex::Regex::new(pattern) {
            Ok(re) => {
                result = re.replace_all(&result, "[REDACTED]").to_string();
            }
            Err(e) => {
                // Log warning for invalid patterns to help with configuration debugging
                eprintln!("Warning: invalid redaction pattern '{}': {}", pattern, e);
            }
        }
    }

    result
}

// -----------------------------------------------------------------------------
// TESTS
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy_matches_hardcoded_valid() {
        let p = BranchPolicy::default();
        let e = vec![];

        let report = evaluate_branch_policy(
            &p,
            "create",
            "feat/wave-13",
            &e,
            "/path/to/repo",
            "Default",
            None,
        );
        assert!(!report.blocked);

        let report = evaluate_branch_policy(
            &p,
            "create",
            "fix/lock-drift-182",
            &e,
            "/path/to/repo",
            "Default",
            None,
        );
        assert!(!report.blocked);
    }

    #[test]
    fn test_default_policy_matches_hardcoded_invalid() {
        let p = BranchPolicy::default();
        let e = vec![];

        let report =
            evaluate_branch_policy(&p, "create", "main", &e, "/path/to/repo", "Default", None);
        assert!(report.blocked); // fails both naming and protected

        let report = evaluate_branch_policy(
            &p,
            "create",
            "feature/mixedCase",
            &e,
            "/path/to/repo",
            "Default",
            None,
        );
        assert!(report.blocked);

        let report = evaluate_branch_policy(
            &p,
            "create",
            "feat/bad__name",
            &e,
            "/path/to/repo",
            "Default",
            None,
        );
        assert!(report.blocked);
    }

    #[test]
    fn test_default_policy_matches_protected() {
        let p = BranchPolicy::default();
        let e = vec![];

        // "main"
        let report =
            evaluate_branch_policy(&p, "create", "main", &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);

        // "release/v1.0.0"
        let report = evaluate_branch_policy(
            &p,
            "create",
            "release/v1.0.0",
            &e,
            "/path/to/repo",
            "Default",
            None,
        );
        assert!(report.blocked);

        // Not protected
        let report =
            evaluate_branch_policy(&p, "create", "feat/x", &e, "/path/to/repo", "Default", None);
        assert!(!report.blocked);
    }

    #[test]
    fn test_dependency_policy_no_lockfile() {
        let p = DependencyPolicy::default();
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        let report =
            evaluate_dependency_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);

        std::fs::File::create(temp.path().join("Cargo.lock")).unwrap();
        let report =
            evaluate_dependency_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(!report.blocked);
    }

    #[test]
    fn test_release_policy_no_changelog() {
        let p = ReleasePolicy::default();
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        let report = evaluate_release_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);

        std::fs::File::create(temp.path().join("CHANGELOG.md")).unwrap();
        let report = evaluate_release_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(!report.blocked);
    }

    #[test]
    fn test_runtime_policy_no_dockerfile() {
        let mut p = RuntimePolicy::default();
        p.require_dockerfile.level = EnforcementLevel::Block;
        p.require_dockerfile.enabled = true;
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        let report = evaluate_runtime_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);

        std::fs::File::create(temp.path().join("Dockerfile")).unwrap();
        let report = evaluate_runtime_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(!report.blocked);
    }

    #[test]
    fn test_security_policy_naked_secrets() {
        let p = SecurityPolicy::default();
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        // Ensure the directory looks somewhat valid (e.g. minimal scanning environment)
        std::fs::File::create(temp.path().join("main.rs")).unwrap();
        std::fs::write(
            temp.path().join("main.rs"),
            "let key = \"AKIAABCDEFGHIJKLMNOP\";",
        )
        .unwrap();

        let report =
            evaluate_security_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule, "SEC-001");
    }

    #[test]
    fn test_dependency_policy_banned_package() {
        let mut p = DependencyPolicy::default();
        p.banned_packages.items = vec!["left-pad".to_string()];
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        // Setup package.json with banned package
        let pkg_content = r#"{"dependencies": {"left-pad": "1.3.0"}}"#;
        std::fs::write(temp.path().join("package.json"), pkg_content).unwrap();

        let report =
            evaluate_dependency_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);
        assert!(report.violations.iter().any(|v| v.rule == "DEP-001"));
    }

    #[test]
    fn test_dependency_policy_disallowed_license() {
        let mut p = DependencyPolicy::default();
        p.allowed_licenses.level = EnforcementLevel::Block;
        p.allowed_licenses.items = vec!["MIT".to_string()];
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        // Setup Cargo.toml with disallowed license
        let cargo_content = r#"[package]
name = "test"
version = "0.1.0"
license = "GPL-3.0"
"#;
        std::fs::write(temp.path().join("Cargo.toml"), cargo_content).unwrap();

        let report =
            evaluate_dependency_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(
            report.blocked,
            "Report should be blocked for disallowed license. Violations: {:?}",
            report.violations
        );
        assert!(report.violations.iter().any(|v| v.rule == "DEP-002"));
    }

    #[test]
    fn test_release_policy_forbidden_day() {
        let mut p = ReleasePolicy::default();
        p.forbidden_days.level = EnforcementLevel::Block;
        // Ensure today is in the forbidden list to test failure
        let today = Utc::now().weekday().to_string();
        p.forbidden_days.items = vec![today];
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        let report = evaluate_release_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);
        assert!(report.violations.iter().any(|v| v.rule == "REL-001"));
    }

    #[test]
    fn test_quality_policy_low_coverage() {
        let mut p = QualityPolicy::default();
        p.require_coverage.level = EnforcementLevel::Block;
        p.require_coverage.enabled = true;
        p.min_test_coverage = 80.0;
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        // Setup coverage.xml with low coverage
        let cov_content = r#"<?xml version="1.0" ?>
<coverage line-rate="0.50" version="1.0">
</coverage>"#;
        std::fs::write(temp.path().join("coverage.xml"), cov_content).unwrap();

        let report = evaluate_quality_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);
        assert!(report.violations.iter().any(|v| v.rule == "QUAL-001"));
    }

    #[test]
    fn test_runtime_policy_untrusted_image() {
        let mut p = RuntimePolicy::default();
        p.allowed_base_images.level = EnforcementLevel::Block;
        p.allowed_base_images.items = vec!["alpine".to_string()];
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        // Setup Dockerfile with untrusted image
        let docker_content = "FROM ubuntu:latest\n";
        std::fs::write(temp.path().join("Dockerfile"), docker_content).unwrap();

        let report = evaluate_runtime_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);
        assert!(report.violations.iter().any(|v| v.rule == "RUN-001"));
    }

    #[test]
    fn test_runtime_policy_missing_healthcheck() {
        let mut p = RuntimePolicy::default();
        p.require_healthcheck.level = EnforcementLevel::Block;
        p.require_healthcheck.enabled = true;
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        // Setup Dockerfile without HEALTHCHECK
        let docker_content = "FROM alpine:latest\n";
        std::fs::write(temp.path().join("Dockerfile"), docker_content).unwrap();

        let report = evaluate_runtime_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);
        assert!(report.violations.iter().any(|v| v.rule == "RUN-002"));
    }

    #[test]
    fn test_security_policy_violation() {
        let mut p = SecurityPolicy::default();
        p.max_cve_severity = "medium".to_string();
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        // Write a secret that scan_secrets will find with severity "high"
        // High (3) >= Medium (2) -> SEC-001
        std::fs::write(
            temp.path().join("main.rs"),
            "let k = \"AKIAABCDEFGHIJKLMNOP\";",
        )
        .unwrap();

        let report =
            evaluate_security_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(
            report.blocked,
            "Report should be blocked for high-severity secret. Violations: {:?}",
            report.violations
        );
        assert!(
            report.violations.iter().any(|v| v.rule == "SEC-001"),
            "Expected SEC-001 violation, got: {:?}",
            report.violations
        );
    }

    #[test]
    fn test_quality_policy_missing_reports() {
        let mut p = QualityPolicy::default();
        p.require_coverage.level = EnforcementLevel::Block;
        p.require_coverage.enabled = true;
        p.require_lint_pass.level = EnforcementLevel::Block;
        p.require_lint_pass.enabled = true;
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        // No files created
        let report = evaluate_quality_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(report.blocked);
        assert!(report.violations.iter().any(|v| v.rule == "QUAL-002"));
    }

    #[test]
    fn test_runtime_policy_malformed_dockerfile() {
        let p = RuntimePolicy::default();
        let e = vec![];
        let temp = tempfile::tempdir().unwrap();

        // Malformed content (not really malformed for my simple parser, but let's test empty)
        std::fs::write(temp.path().join("Dockerfile"), "").unwrap();

        let report = evaluate_runtime_policy(&p, temp.path(), &e, "/path/to/repo", "Default", None);
        assert!(!report.blocked); // Default is off
    }

    #[test]
    fn test_operator_routine_policy_blocks_unregistered_repo() {
        let policy = OperatorRoutinePolicy::default();
        let ex = vec![];
        let ctx = OperatorRoutineContext {
            action: "push".to_string(),
            has_active_scope: true,
            repo_registered: false,
            current_branch: Some("main".to_string()),
            repo_clean: Some(true),
            completed_steps: vec!["gate".to_string()],
        };
        let report =
            evaluate_operator_routine_policy(&policy, &ctx, &ex, "/path/repo", "Default", None);
        assert!(report.blocked);
        assert!(report.violations.iter().any(|v| v.rule == "ORT-002"));
    }

    #[test]
    fn test_operator_routine_policy_blocks_disallowed_branch_when_configured() {
        let mut policy = OperatorRoutinePolicy::default();
        policy.allowed_push_branches.level = EnforcementLevel::Block;
        policy.allowed_push_branches.items = vec!["main".to_string()];
        let ex = vec![];
        let ctx = OperatorRoutineContext {
            action: "push".to_string(),
            has_active_scope: true,
            repo_registered: true,
            current_branch: Some("feature/x".to_string()),
            repo_clean: Some(true),
            completed_steps: vec!["gate".to_string()],
        };
        let report =
            evaluate_operator_routine_policy(&policy, &ctx, &ex, "/path/repo", "Default", None);
        assert!(report.blocked);
        assert!(report.violations.iter().any(|v| v.rule == "ORT-004"));
    }

    #[test]
    fn test_operator_routine_policy_warns_missing_steps() {
        let mut policy = OperatorRoutinePolicy::default();
        policy.required_prepush_steps.level = EnforcementLevel::Warn;
        policy.required_prepush_steps.items = vec!["policy".to_string(), "gate".to_string()];
        let ex = vec![];
        let ctx = OperatorRoutineContext {
            action: "push".to_string(),
            has_active_scope: true,
            repo_registered: true,
            current_branch: Some("main".to_string()),
            repo_clean: Some(true),
            completed_steps: vec!["policy".to_string()],
        };
        let report =
            evaluate_operator_routine_policy(&policy, &ctx, &ex, "/path/repo", "Default", None);
        assert!(!report.blocked);
        assert!(report.warnings.iter().any(|v| v.rule == "ORT-005"));
    }

    #[test]
    fn test_required_confirmation_prune() {
        let policy = BranchPolicy::default();
        let (ct, phrase) = required_confirmation(&policy, "prune", "feat/old-branch");
        assert_eq!(ct, ConfirmationType::TypedPhrase);
        assert_eq!(phrase, Some("PRUNE".to_string()));
    }

    #[test]
    fn test_required_confirmation_protected_branch() {
        let policy = BranchPolicy::default();
        let (ct, phrase) = required_confirmation(&policy, "sync", "main");
        assert_eq!(ct, ConfirmationType::TypedPhrase);
        assert_eq!(phrase, Some("CONFIRM".to_string()));
    }

    #[test]
    fn test_required_confirmation_normal_branch() {
        let policy = BranchPolicy::default();
        let (ct, phrase) = required_confirmation(&policy, "create", "feat/new-thing");
        assert_eq!(ct, ConfirmationType::Standard);
        assert!(phrase.is_none());
    }
}
