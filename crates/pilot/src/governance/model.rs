use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Command categories for allowlist-based mutation scope control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CommandCategory {
    /// Read-only operations (list, status, query)
    Read,
    /// Branch creation operations
    BranchCreate,
    /// Branch modification operations (sync, merge)
    BranchModify,
    /// Branch deletion/destructive operations (prune, delete)
    BranchDestroy,
    /// Policy operations (preview, apply)
    Policy,
    /// Release operations
    Release,
    /// Admin operations (services, db)
    Admin,
}

impl CommandCategory {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "read" => Some(CommandCategory::Read),
            "branch_create" | "create" => Some(CommandCategory::BranchCreate),
            "branch_modify" | "modify" | "sync" => Some(CommandCategory::BranchModify),
            "branch_destroy" | "destroy" | "prune" => Some(CommandCategory::BranchDestroy),
            "policy" => Some(CommandCategory::Policy),
            "release" => Some(CommandCategory::Release),
            "admin" => Some(CommandCategory::Admin),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            CommandCategory::Read => "read",
            CommandCategory::BranchCreate => "branch_create",
            CommandCategory::BranchModify => "branch_modify",
            CommandCategory::BranchDestroy => "branch_destroy",
            CommandCategory::Policy => "policy",
            CommandCategory::Release => "release",
            CommandCategory::Admin => "admin",
        }
    }
}

/// Command allowlist with mutation scope enforcement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandAllowlist {
    /// Enabled command categories
    pub enabled_categories: Vec<CommandCategory>,
    /// Explicitly blocked commands (by name)
    pub blocked_commands: Vec<String>,
    /// Commands requiring typed confirmation
    pub confirmation_required: Vec<String>,
    /// Default scope if not specified
    pub default_scope: CommandScope,
}

impl Default for CommandAllowlist {
    fn default() -> Self {
        Self {
            enabled_categories: vec![
                CommandCategory::Read,
                CommandCategory::BranchCreate,
                CommandCategory::BranchModify,
                CommandCategory::BranchDestroy,
                CommandCategory::Policy,
            ],
            blocked_commands: vec![],
            confirmation_required: vec![
                "branch prune".to_string(),
                "branch delete".to_string(),
                "multi apply".to_string(),
                "policy apply".to_string(),
            ],
            default_scope: CommandScope::Local,
        }
    }
}

/// Execution scope for mutation control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommandScope {
    /// Local only - no remote mutations
    Local,
    /// Dry-run mode - preview only
    DryRun,
    /// Full execution with remote mutations
    Full,
}

impl CommandScope {
    pub fn allows_mutation(&self) -> bool {
        matches!(self, CommandScope::Full)
    }
}

/// Graduated confirmation gate types for destructive branch operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationType {
    /// No extra confirmation required
    None,
    /// Simple checkbox / "Execute" button
    Standard,
    /// Must type an exact phrase to confirm
    TypedPhrase,
    /// Typed phrase + countdown timer before enable
    DoubleConfirm,
}

impl Default for ConfirmationType {
    fn default() -> Self {
        ConfirmationType::Standard
    }
}

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
    /// Mutation control policy for command allowlist and scope enforcement (FC-8)
    #[serde(default)]
    pub mutation_control: MutationControlPolicy,
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
    #[serde(default = "default_typed_phrase")]
    pub confirmation_type: ConfirmationType,
    #[serde(default)]
    pub confirmation_phrase: Option<String>,
}

fn default_typed_phrase() -> ConfirmationType {
    ConfirmationType::TypedPhrase
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LifecyclePolicy {
    pub auto_prune_merged: LevelEnabled,
    pub prune_requires_confirmation: bool,
    pub confirmation_phrase: String,
    #[serde(default = "default_typed_phrase")]
    pub prune_confirmation_type: ConfirmationType,
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

/// Mutation control policy for command allowlist and scope enforcement (FC-8)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MutationControlPolicy {
    /// Command allowlist configuration
    pub command_allowlist: CommandAllowlist,
    /// Require typed confirmation for protected branch operations
    pub protected_branch_confirmation: bool,
    /// Enable secrets-safe logging (redact patterns in evidence)
    pub secrets_safe_logging: bool,
    /// Patterns to redact from logs (regex)
    pub redaction_patterns: Vec<String>,
    /// Default confirmation type for destructive operations
    pub destructive_confirmation_type: ConfirmationType,
}

impl Default for MutationControlPolicy {
    fn default() -> Self {
        Self {
            command_allowlist: CommandAllowlist::default(),
            protected_branch_confirmation: true,
            secrets_safe_logging: true,
            redaction_patterns: vec![
                r"(?i)(api[_-]?key|secret[_-]?key|password|token|auth)[\s:=]+[\S]+".to_string(),
                r"(?i)ghp_[a-zA-Z0-9]{36}".to_string(),
                r"(?i)github_pat_[a-zA-Z0-9_]{22,}".to_string(),
                r"sk-[a-zA-Z0-9]{48}".to_string(),
            ],
            destructive_confirmation_type: ConfirmationType::TypedPhrase,
        }
    }
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
                confirmation_type: ConfirmationType::TypedPhrase,
                confirmation_phrase: Some("CONFIRM".to_string()),
            },
            lifecycle: LifecyclePolicy {
                auto_prune_merged: LevelEnabled {
                    level: EnforcementLevel::Off,
                    enabled: false,
                },
                prune_requires_confirmation: true,
                confirmation_phrase: "PRUNE".to_string(),
                prune_confirmation_type: ConfirmationType::TypedPhrase,
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
            mutation_control: MutationControlPolicy::default(),
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
    pub allowed_licenses: LevelList,
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
            allowed_licenses: LevelList {
                level: EnforcementLevel::Off,
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
    pub require_semver: LevelEnabled,
    pub version_strategy: String,
    pub allowed_channels: LevelList,
    pub forbidden_days: LevelList,
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
            require_semver: LevelEnabled {
                level: EnforcementLevel::Block,
                enabled: true,
            },
            version_strategy: "semver".to_string(),
            allowed_channels: LevelList {
                level: EnforcementLevel::Block,
                items: vec![
                    "alpha".to_string(),
                    "beta".to_string(),
                    "stable".to_string(),
                ],
            },
            forbidden_days: LevelList {
                level: EnforcementLevel::Warn,
                items: vec![
                    "Friday".to_string(),
                    "Saturday".to_string(),
                    "Sunday".to_string(),
                ],
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
    pub require_coverage: LevelEnabled,
    pub min_test_coverage: f32,
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
            require_coverage: LevelEnabled {
                level: EnforcementLevel::Off,
                enabled: false,
            },
            min_test_coverage: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RuntimePolicy {
    pub kind: String,
    pub version: i32,
    pub require_dockerfile: LevelEnabled,
    pub require_healthcheck: LevelEnabled,
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
            require_healthcheck: LevelEnabled {
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OperatorRoutinePolicy {
    pub kind: String,
    pub version: i32,
    pub require_active_scope: LevelEnabled,
    pub require_registered_repo: LevelEnabled,
    pub require_clean_worktree_for_push: LevelEnabled,
    pub allowed_push_branches: LevelList,
    pub required_prepush_steps: LevelList,
}

impl Default for OperatorRoutinePolicy {
    fn default() -> Self {
        Self {
            kind: "operator_routine".to_string(),
            version: 1,
            require_active_scope: LevelEnabled {
                level: EnforcementLevel::Block,
                enabled: true,
            },
            require_registered_repo: LevelEnabled {
                level: EnforcementLevel::Block,
                enabled: true,
            },
            require_clean_worktree_for_push: LevelEnabled {
                level: EnforcementLevel::Warn,
                enabled: true,
            },
            allowed_push_branches: LevelList {
                level: EnforcementLevel::Warn,
                items: vec!["main".to_string(), "dev".to_string()],
            },
            required_prepush_steps: LevelList {
                level: EnforcementLevel::Warn,
                items: vec!["gate".to_string()],
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
    pub policy_source_id: Option<Uuid>,
    pub policy_source_name: String,
    pub override_available: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GovernanceReconcileReport {
    pub agorg_id: Uuid,
    pub agorg_name: String,
    pub timestamp: DateTime<Utc>,
    pub total_agos: usize,
    pub compliant_count: usize,
    pub violation_count: usize,
    pub warning_count: usize,
    pub ago_statuses: Vec<AgoComplianceStatus>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgoComplianceStatus {
    pub ago_path: String,
    pub ago_name: String,
    pub overall_status: String,
    pub is_overridden: bool,
    pub evaluations: std::collections::HashMap<String, PolicyEvalReport>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolicyOverrideRecord {
    pub id: Uuid,
    pub agorg_id: Uuid,
    pub ago_path: String,
    pub policy_kind: String,
    pub reason: String,
    pub ticket_ref: Option<String>,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub parent_policy_version: i32,
    pub override_policy_version: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolicyConflictTrace {
    pub ago_path: String,
    pub policy_kind: String,
    pub resolved_source: String,
    pub resolved_agorg_id: Uuid,
    pub resolved_version: i32,
    pub chain: Vec<InheritanceStep>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InheritanceStep {
    pub agorg_id: Uuid,
    pub agorg_name: String,
    pub depth: i32,
    pub has_override: bool,
    pub has_fleet_policy: bool,
    pub is_winner: bool,
}
