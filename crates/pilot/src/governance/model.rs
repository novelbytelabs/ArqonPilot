use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EnforcementLevel {
    Off,
    Info,
    Warn,
    Block,
    #[serde(rename = "auto-fix")]
    AutoFix,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BranchPolicy {
    pub kind: String,
    pub version: i32,
    pub naming: NamingPolicy,
    pub protected_branches: ProtectedBranchesPolicy,
    pub lifecycle: LifecyclePolicy,
    pub sync: SyncPolicy,
    pub create: CreatePolicy,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NamingPolicy {
    pub level: EnforcementLevel,
    pub required_prefix: Vec<String>,
    pub separator: String,
    pub body_format: String,
    pub max_length: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProtectedBranchesPolicy {
    pub level: EnforcementLevel,
    pub patterns: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LifecyclePolicy {
    pub auto_prune_merged: LevelEnabled,
    pub prune_requires_confirmation: bool,
    pub confirmation_phrase: String,
    pub max_stale_days: LevelDays,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LevelEnabled {
    pub level: EnforcementLevel,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LevelDays {
    pub level: EnforcementLevel,
    pub days: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SyncPolicy {
    pub strategy: String,
    pub auto_fetch_before_sync: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CreatePolicy {
    pub require_preview: bool,
    pub base_branch_default: String,
}

impl Default for BranchPolicy {
    fn default() -> Self {
        Self {
            kind: "branch".to_string(),
            version: 1,
            naming: NamingPolicy {
                level: EnforcementLevel::Block,
                required_prefix: vec![
                    "feat".to_string(),
                    "fix".to_string(),
                    "docs".to_string(),
                    "test".to_string(),
                    "refactor".to_string(),
                    "chore".to_string(),
                    "perf".to_string(),
                ],
                separator: "/".to_string(),
                body_format: "kebab-case".to_string(),
                max_length: 80,
            },
            protected_branches: ProtectedBranchesPolicy {
                level: EnforcementLevel::Block,
                patterns: vec![
                    "main".to_string(),
                    "master".to_string(),
                    "dev".to_string(),
                    "release".to_string(),
                    "release/*".to_string(),
                ],
            },
            lifecycle: LifecyclePolicy {
                auto_prune_merged: LevelEnabled {
                    level: EnforcementLevel::Off,
                    enabled: false,
                },
                prune_requires_confirmation: true,
                confirmation_phrase: "PRUNE".to_string(),
                max_stale_days: LevelDays {
                    level: EnforcementLevel::Off,
                    days: 30,
                },
            },
            sync: SyncPolicy {
                strategy: "ff-only".to_string(),
                auto_fetch_before_sync: true,
            },
            create: CreatePolicy {
                require_preview: true,
                base_branch_default: "main".to_string(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LevelList {
    pub level: EnforcementLevel,
    pub items: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DependencyPolicy {
    pub kind: String,
    pub version: i32,
    pub allowed_registries: LevelList,
    pub banned_packages: LevelList,
    pub require_lockfile: LevelEnabled,
}

impl Default for DependencyPolicy {
    fn default() -> Self {
        Self {
            kind: "dependency".to_string(),
            version: 1,
            allowed_registries: LevelList {
                level: EnforcementLevel::Off,
                items: vec![],
            },
            banned_packages: LevelList {
                level: EnforcementLevel::Block,
                items: vec![],
            },
            require_lockfile: LevelEnabled {
                level: EnforcementLevel::Block,
                enabled: true,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReleasePolicy {
    pub kind: String,
    pub version: i32,
    pub require_changelog: LevelEnabled,
    pub version_strategy: String,
    pub allowed_channels: LevelList,
}

impl Default for ReleasePolicy {
    fn default() -> Self {
        Self {
            kind: "release".to_string(),
            version: 1,
            require_changelog: LevelEnabled {
                level: EnforcementLevel::Block,
                enabled: true,
            },
            version_strategy: "semver".to_string(),
            allowed_channels: LevelList {
                level: EnforcementLevel::Block,
                items: vec!["alpha".to_string(), "beta".to_string(), "stable".to_string()],
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SecurityPolicy {
    pub kind: String,
    pub version: i32,
    pub max_cve_severity: String, // e.g. "high", "medium", "low"
    pub block_naked_secrets: LevelEnabled,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            kind: "security".to_string(),
            version: 1,
            max_cve_severity: "medium".to_string(),
            block_naked_secrets: LevelEnabled {
                level: EnforcementLevel::Block,
                enabled: true,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct QualityPolicy {
    pub kind: String,
    pub version: i32,
    pub require_lint_pass: LevelEnabled,
    pub require_format_pass: LevelEnabled,
    pub min_test_coverage: u32,
}

impl Default for QualityPolicy {
    fn default() -> Self {
        Self {
            kind: "quality".to_string(),
            version: 1,
            require_lint_pass: LevelEnabled {
                level: EnforcementLevel::Warn,
                enabled: true,
            },
            require_format_pass: LevelEnabled {
                level: EnforcementLevel::Warn,
                enabled: true,
            },
            min_test_coverage: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RuntimePolicy {
    pub kind: String,
    pub version: i32,
    pub require_dockerfile: LevelEnabled,
    pub allowed_base_images: LevelList,
}

impl Default for RuntimePolicy {
    fn default() -> Self {
        Self {
            kind: "runtime".to_string(),
            version: 1,
            require_dockerfile: LevelEnabled {
                level: EnforcementLevel::Off,
                enabled: false,
            },
            allowed_base_images: LevelList {
                level: EnforcementLevel::Off,
                items: vec![],
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolicyEvalResult {
    pub rule: String,
    pub level: EnforcementLevel,
    pub input: String,
    pub violation: String,
    pub fix_suggestion: String,
    pub policy_source: String,
    pub override_available: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PolicyEvalReport {
    pub violations: Vec<PolicyEvalResult>,
    pub warnings: Vec<PolicyEvalResult>,
    pub infos: Vec<PolicyEvalResult>,
    pub auto_fixes: Vec<PolicyEvalResult>,
    pub blocked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolicyException {
    pub id: Uuid,
    pub agorg_id: Uuid,
    pub ago_path: Option<String>,
    pub policy_kind: String,
    pub rule_path: String,
    pub reason: String,
    pub ticket_ref: Option<String>,
    pub owner: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolicyDecisionRecord {
    pub decision_id: Uuid,
    pub agorg_id: Uuid,
    pub ago_path: String,
    pub policy_kind: String,
    pub action: String,
    pub result: String,
    pub decision_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgorgPolicyRecord {
    pub id: Uuid,
    pub agorg_id: Uuid,
    pub ago_path: Option<String>,
    pub policy_kind: String,
    pub version: i32,
    pub policy_json: Value,
    pub status: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}
