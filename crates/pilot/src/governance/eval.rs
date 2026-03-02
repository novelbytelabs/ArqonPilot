use super::model::*;
use chrono::Utc;

pub fn evaluate_branch_policy(
    policy: &BranchPolicy,
    action: &str, // "create", "sync", "prune"
    branch_name: &str,
    exceptions: &[PolicyException],
) -> PolicyEvalReport {
    let mut report = PolicyEvalReport::default();

    // naming policy
    if action == "create" || action == "sync" {
        eval_naming(&policy.naming, branch_name, exceptions, &mut report);
        eval_protected(
            &policy.protected_branches,
            branch_name,
            exceptions,
            &mut report,
        );
    }

    if action == "prune" {
        eval_protected_prune(
            &policy.protected_branches,
            branch_name,
            exceptions,
            &mut report,
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
) {
    if policy.level == EnforcementLevel::Off {
        return;
    }

    if is_excepted("naming", exceptions) {
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
        );
    }
}

fn eval_protected(
    policy: &ProtectedBranchesPolicy,
    branch_name: &str,
    exceptions: &[PolicyException],
    report: &mut PolicyEvalReport,
) {
    if policy.level == EnforcementLevel::Off {
        return;
    }

    if is_excepted("protected_branches", exceptions) {
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
) {
    if policy.level == EnforcementLevel::Off {
        return;
    }
    if is_excepted("protected_branches", exceptions) {
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

fn is_excepted(rule_prefix: &str, exceptions: &[PolicyException]) -> bool {
    let now = Utc::now();
    exceptions
        .iter()
        .any(|e| e.rule_path.starts_with(rule_prefix) && e.expires_at > now)
}

fn add_result(
    report: &mut PolicyEvalReport,
    rule: &str,
    level: &EnforcementLevel,
    input: &str,
    violation: &str,
    fix: &str,
) {
    let res = PolicyEvalResult {
        rule: rule.to_string(),
        level: level.clone(),
        input: input.to_string(),
        violation: violation.to_string(),
        fix_suggestion: fix.to_string(),
        policy_source: "Evaluated".to_string(), // Injected higher up
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
// TESTS
// -----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy_matches_hardcoded_valid() {
        let p = BranchPolicy::default();
        let e = vec![];

        let report = evaluate_branch_policy(&p, "create", "feat/wave-13", &e);
        assert!(!report.blocked);

        let report = evaluate_branch_policy(&p, "create", "fix/lock-drift-182", &e);
        assert!(!report.blocked);
    }

    #[test]
    fn test_default_policy_matches_hardcoded_invalid() {
        let p = BranchPolicy::default();
        let e = vec![];

        let report = evaluate_branch_policy(&p, "create", "main", &e);
        assert!(report.blocked); // fails both naming and protected

        let report = evaluate_branch_policy(&p, "create", "feature/mixedCase", &e);
        assert!(report.blocked);

        let report = evaluate_branch_policy(&p, "create", "feat/bad__name", &e);
        assert!(report.blocked);
    }

    #[test]
    fn test_default_policy_matches_protected() {
        let p = BranchPolicy::default();
        let e = vec![];

        // "main"
        let report = evaluate_branch_policy(&p, "create", "main", &e);
        assert!(report.blocked);

        // "release/v1.0.0"
        let report = evaluate_branch_policy(&p, "create", "release/v1.0.0", &e);
        assert!(report.blocked);

        // Not protected
        // wait, feat/x should pass protected checks but naming checks might apply.
        // It passes naming checks, so it should not be blocked.
        let report = evaluate_branch_policy(&p, "create", "feat/x", &e);
        assert!(!report.blocked);
    }
}
