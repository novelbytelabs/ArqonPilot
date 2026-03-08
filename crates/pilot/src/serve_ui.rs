use crate::agorg::{self, AgorgStore};
use crate::bus::{run_pilot_subcommand_local, send_command_once, BusBridgeConfig};
use crate::governance::{eval::*, model::*, store::GovernanceStore};
use crate::service_supervisor::{supervised_start, RetryPolicy};
use crate::shim_runtime::bus_shim_command;
use axum::extract::{Query, State};
use axum::http::{HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use miette::{IntoDiagnostic, Result, WrapErr};
use pilot_branch as branch;
use pilot_multi as multi;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command as TokioCommand;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

const FAVICON_ICO: &[u8] = include_bytes!("../assets/favicon.ico");
const PILOT_UI_JS: &str = include_str!("pilot_ui.js");

// use sha2::{Digest, Sha256}; // Removed to avoid warnings if unused here (it's used in pilot-core/lib.rs)

#[derive(Clone)]
pub struct UiConfig {
    pub host: String,
    pub port: u16,
    pub instance_id: String,
    pub bus: BusBridgeConfig,
    pub allow_mutations: bool,
    pub allowed_commands: Option<HashSet<String>>,
}

#[derive(Clone)]
struct UiState {
    instance_id: String,
    bus: BusBridgeConfig,
    events: broadcast::Sender<Value>,
    allow_mutations: bool,
    allowed_commands: Option<HashSet<String>>,
    codex_contracts: Arc<Mutex<HashMap<String, CodexContractRecord>>>,
    codex_contracts_log: PathBuf,
    branch_previews: Arc<Mutex<HashMap<String, BranchPreviewRecord>>>,
    agorg_store: AgorgStore,
    server_start_time_unix: u64,
}

#[derive(Debug, Deserialize)]
struct UiCommandRequest {
    command: String,
    payload: Value,
}

#[derive(Debug, Deserialize)]
struct OrchestratorRequest {
    domain: String, // "branch", "dependency", "command"
    payload: Value,
    /// Client-supplied operation_id is intentionally ignored; server always generates a new UUID.
    #[allow(dead_code)]
    operation_id: Option<String>,
}

fn orchestrate_is_preview(payload: &Value) -> bool {
    payload
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || payload
            .get("action")
            .and_then(Value::as_str)
            .map(|a| {
                a.contains("preview")
                    || a == "status"
                    || a == "policy"
                    || a == "hook-policy"
                    || a == "drift"
            })
            .unwrap_or(false)
}

fn normalize_orchestrate_payload(payload: &mut Value, stage: &str) {
    if stage == "preview" {
        payload["dry_run"] = json!(true);
        if payload.get("apply").is_some() {
            payload["apply"] = json!(false);
        }
    }
}

fn command_request_from_orchestrate_payload(
    payload: Value,
) -> std::result::Result<UiCommandRequest, String> {
    if let Ok(req) = serde_json::from_value::<UiCommandRequest>(payload.clone()) {
        return Ok(req);
    }

    let action = payload
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| "Invalid command payload format".to_string())?;

    let mut cmd_payload = json!({});
    if let Some(group) = payload.get("group").and_then(Value::as_str) {
        cmd_payload["group"] = json!(group);
    }
    if let Some(tags) = payload.get("tags") {
        cmd_payload["tags"] = tags.clone();
    }
    if let Some(base_branch) = payload.get("base_branch").and_then(Value::as_str) {
        cmd_payload["base_branch"] = json!(base_branch);
    }
    if let Some(pr_base_branch) = payload.get("pr_base_branch").and_then(Value::as_str) {
        cmd_payload["pr_base_branch"] = json!(pr_base_branch);
    }
    if let Some(branch) = payload.get("branch").and_then(Value::as_str) {
        cmd_payload["branch"] = json!(branch);
    }
    if let Some(stage_size) = payload.get("stage_size").and_then(Value::as_u64) {
        cmd_payload["stage_size"] = json!(stage_size);
    }
    if let Some(continue_on_failure) = payload.get("continue_on_failure").and_then(Value::as_bool) {
        cmd_payload["continue_on_failure"] = json!(continue_on_failure);
    }

    let req = match action {
        "heal.plan" => {
            cmd_payload["plan_only"] = json!(true);
            UiCommandRequest {
                command: "pilot.heal.run".to_string(),
                payload: cmd_payload,
            }
        }
        "heal.run" => {
            cmd_payload["plan_only"] = json!(false);
            UiCommandRequest {
                command: "pilot.heal.run".to_string(),
                payload: cmd_payload,
            }
        }
        "multi.status" => UiCommandRequest {
            command: "pilot.multi.status".to_string(),
            payload: cmd_payload,
        },
        "dag.evaluate" | "multi.dag" => {
            cmd_payload["dry_run"] = json!(true);
            UiCommandRequest {
                command: "pilot.multi.dag".to_string(),
                payload: cmd_payload,
            }
        }
        "multi.apply" => {
            let dry_run = payload
                .get("dry_run")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            cmd_payload["apply"] = json!(!dry_run);
            UiCommandRequest {
                command: "pilot.multi.apply".to_string(),
                payload: cmd_payload,
            }
        }
        _ => {
            return Err(format!(
                "Invalid command payload format: unknown action '{}'",
                action
            ))
        }
    };
    Ok(req)
}

/// P5: Canonical cross-tab response envelope.
/// Injected at the orchestration API boundary — inner handler structs are NOT modified.
#[derive(Debug, Serialize)]
struct OrchEnvelope {
    ok: bool,
    operation_id: String,
    domain: String,
    stage: String,
    status: String,
    summary: String,
    artifact_path: Option<String>,
    error: Option<String>,
    inner: Value,
}

/// Wrap a domain handler's raw JSON response into a canonical OrchEnvelope.
/// Extracts ok/artifact_path/error from inner response; generates a fresh server-side UUID.
fn wrap_as_envelope(domain: &str, stage: &str, inner_response: Value) -> OrchEnvelope {
    let ok = inner_response
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let artifact_path = inner_response
        .get("artifact_path")
        .and_then(Value::as_str)
        .map(String::from);
    let error = if ok {
        None
    } else {
        inner_response
            .get("error")
            .or_else(|| inner_response.get("stderr"))
            .and_then(Value::as_str)
            .map(String::from)
    };
    let summary = if ok {
        format!("{domain}/{stage}: completed")
    } else {
        format!("{domain}/{stage}: failed")
    };
    let status = if stage == "preview" {
        "preview".to_string()
    } else if ok {
        "ok".to_string()
    } else {
        "error".to_string()
    };
    OrchEnvelope {
        ok,
        operation_id: Uuid::new_v4().to_string(),
        domain: domain.to_string(),
        stage: stage.to_string(),
        status,
        summary,
        artifact_path,
        error,
        inner: inner_response,
    }
}

#[derive(Debug, Serialize)]
struct UiCommandResponse {
    ok: bool,
    response: Value,
}

#[derive(Debug, Deserialize)]
struct ReportsQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct CodexContractsQuery {
    limit: Option<usize>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexContractQuery {
    contract_id: String,
}

#[derive(Debug, Deserialize)]
struct AgorgUseRequest {
    agorg: String,
}

#[derive(Debug, Deserialize)]
struct AgorgCreateRequest {
    name: String,
    root: String,
    master: Option<String>,
    parent: Option<String>,
    scan_depth: Option<usize>,
    default_scope: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AgorgCreateProjectRequest {
    name: String,
    root: String,
    master: Option<String>,
    parent: Option<String>,
    scan_depth: Option<usize>,
    autoscan: Option<bool>,
    import: Option<bool>,
    prune_missing: Option<bool>,
    default_scope: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AgorgUpdateRequest {
    id: Uuid,
    name: Option<String>,
    root: Option<String>,
    master: Option<String>,
    scan_depth: Option<usize>,
    default_scope: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AgorgDeleteRequest {
    id: Uuid,
}

#[derive(Debug, Deserialize)]
struct AgorgScanMasterRequest {
    path: String,
}

#[derive(Debug, Deserialize)]
struct AgorgUpgradeRequest {
    path: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct AgorgEditRelationshipRequest {
    path: String,
    parent: Option<String>,
    children: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgDiscoverRequest {
    root: String,
    depth: Option<usize>,
    import_to: Option<String>,
    prune_missing: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AgorgImportSelectedRequest {
    agorg: String,
    root: String,
    depth: Option<usize>,
    candidates: Vec<agorg::DiscoverCandidate>,
    prune_missing: Option<bool>,
    default_scope_path: Option<String>,
    agorg_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgReconcileRequest {
    agorg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgPolicyReportsQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct AgorgPolicyReportRequest {
    agorg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgReconcileApplyRequest {
    agorg: Option<String>,
    dry_run: Option<bool>,
    issue_class: Option<String>,
    dry_run_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgDashboardOverviewRequest {
    agorg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AcceptanceMatrixRequest {
    wave: Option<String>,
    profile: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgPreferencesQuery {
    agorg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgPreferencesRequest {
    agorg: Option<String>,
    #[serde(default)]
    preferences: Value,
    merge: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UiSessionUpdateRequest {
    #[serde(default)]
    session: Value,
}

#[derive(Debug, Deserialize)]
pub struct AgorgBatchCreateRequest {
    pub destination: String,
    pub name: String,
    pub siblings: Vec<String>,
    pub use_git: bool,
}

#[derive(Debug, Deserialize)]
struct AgorgTreeQuery {
    root: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgLinkRequest {
    parent: String,
    child: String,
}

#[derive(Debug, Deserialize)]
struct FsPickDirectoryRequest {
    start_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateDirRequest {
    path: String,
}

#[derive(Debug, Deserialize)]
struct ReportPathQuery {
    path: String,
    max_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DependencyActionRequest {
    action: String,
    #[serde(default)]
    json: bool,
    branch: Option<String>,
    remote: Option<String>,
    preflight_steps: Option<Vec<String>>,
    label: Option<String>,
    bundle_path: Option<String>,
    ci_timeout_sec: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct BranchMatrixRequest {
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    search: Option<String>,
    base_branch: Option<String>,
    target_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BranchMatrixRow {
    id: i64,
    repo: String,
    path: String,
    group: Option<String>,
    tags: Vec<String>,
    current_branch: String,
    clean: bool,
    ahead: Option<u32>,
    behind: Option<u32>,
    on_target: Option<bool>,
    protected: bool,
}

#[derive(Debug, Deserialize)]
struct BranchRunRequest {
    action: String,
    branch: Option<String>,
    base_branch: Option<String>,
    dry_run: Option<bool>,
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    selected_repo_ids: Vec<i64>,
    preview_token: Option<String>,
    confirm_phrase: Option<String>,
}

#[derive(Debug, Clone)]
struct BranchPreviewRecord {
    token: String,
    scope_id: Uuid,
    action: String,
    expected_execute_payload: Value,
    created_at_unix: u64,
    expires_at_unix: u64,
}

#[derive(Debug, Deserialize)]
struct EvidenceExportRequest {
    history_limit: Option<usize>,
    reports_limit: Option<usize>,
    gate_logs_limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct CodexActionRequest {
    intent: Option<String>,
    command: Option<String>,
    contract_id: Option<String>,
    #[serde(default)]
    payload: Value,
    mode: Option<String>,
    expected_effect: Option<String>,
    rollback_strategy: Option<String>,
    verify_command: Option<String>,
    reconcile_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexContractRecord {
    contract_id: String,
    status: String,
    intent: String,
    command: String,
    payload_original: Value,
    payload_normalized: Value,
    mutating_command: bool,
    expected_effect: Option<String>,
    rollback_strategy: Option<String>,
    verify_command: Option<String>,
    verify_payload: Value,
    execute_response: Option<Value>,
    verify_response: Option<Value>,
    last_error: Option<String>,
    reconcile_notes: Option<String>,
    created_at_unix: u64,
    updated_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UiRuntimeEntry {
    pid: u32,
    port: u16,
    instance_id: String,
    version: String,
    binary_fingerprint: String,
    started_at_unix: u64,
}

fn ui_runtime_registry_path() -> PathBuf {
    reports_root().join("ui_runtime_registry.json")
}

fn ui_binary_fingerprint() -> String {
    let exe = std::env::current_exe().ok();
    if let Some(path) = exe {
        if let Ok(meta) = fs::metadata(&path) {
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            return format!("{}|{}|{}", env!("CARGO_PKG_VERSION"), meta.len(), mtime);
        }
    }
    env!("CARGO_PKG_VERSION").to_string()
}

fn process_alive(pid: u32) -> bool {
    PathBuf::from(format!("/proc/{pid}")).exists()
}

fn enforce_ui_runtime_version_guard(port: u16, instance_id: &str) -> Result<()> {
    let path = ui_runtime_registry_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).into_diagnostic()?;
    }
    let mut entries: Vec<UiRuntimeEntry> = if path.exists() {
        let raw = fs::read_to_string(&path).into_diagnostic()?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        Vec::new()
    };
    entries.retain(|e| process_alive(e.pid));

    let current_fp = ui_binary_fingerprint();
    if let Some(conflict) = entries
        .iter()
        .find(|e| e.binary_fingerprint != current_fp)
        .cloned()
    {
        return Err(miette::miette!(
            "Refusing mixed Pilot UI versions: running pid={} port={} version={} fingerprint={} conflicts with current version={} fingerprint={}",
            conflict.pid,
            conflict.port,
            conflict.version,
            conflict.binary_fingerprint,
            env!("CARGO_PKG_VERSION"),
            current_fp
        ));
    }

    let pid = std::process::id();
    entries.retain(|e| e.pid != pid);
    entries.push(UiRuntimeEntry {
        pid,
        port,
        instance_id: instance_id.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        binary_fingerprint: current_fp,
        started_at_unix: now_unix(),
    });
    fs::write(
        &path,
        serde_json::to_string_pretty(&entries).into_diagnostic()?,
    )
    .into_diagnostic()?;
    Ok(())
}

pub async fn run_ui_server(cfg: UiConfig) -> Result<()> {
    enforce_ui_runtime_version_guard(cfg.port, &cfg.instance_id)?;
    let (event_tx, _) = broadcast::channel(512);
    spawn_bus_telemetry_listener(cfg.bus.clone(), event_tx.clone());
    let codex_contracts_log = codex_contracts_log_path();
    let contract_seed = load_persisted_codex_contracts(&codex_contracts_log).unwrap_or_default();
    let agorg_store = AgorgStore::from_instance(cfg.instance_id.clone());
    if let Err(e) = agorg_store.initialize().await {
        eprintln!(
            "Warning: AGOrg store initialization failed ({}). AGOrg API may be unavailable until fixed.",
            e
        );
    }
    let state = Arc::new(UiState {
        instance_id: cfg.instance_id.clone(),
        bus: cfg.bus.clone(),
        events: event_tx.clone(),
        allow_mutations: cfg.allow_mutations,
        allowed_commands: cfg.allowed_commands,
        codex_contracts: Arc::new(Mutex::new(contract_seed)),
        codex_contracts_log,
        branch_previews: Arc::new(Mutex::new(HashMap::new())),
        agorg_store,
        server_start_time_unix: now_unix(),
    });

    let policy = RetryPolicy::default();

    if state.allow_mutations {
        let store = state.agorg_store.clone();

        // 1. Supervised DB Startup (Blocking)
        println!("[Supervisor] Ensuring Managed DB is online...");
        supervised_start(
            "Managed Database",
            || {
                let store = store.clone();
                async move { store.ensure_managed_db().await.map_err(|e| e.to_string()) }
            },
            policy.clone(),
        )
        .await?;

        // 2. Supervised Bus Startup (Blocking)
        println!("[Supervisor] Ensuring ArqonBus is online...");
        supervised_start(
            "ArqonBus",
            || async {
                let (code, out, err) = run_local_script(&bus_shim_command("start"))
                    .await
                    .map_err(|e| e.to_string())?;
                if code != 0 {
                    return Err(format!("Shim exited with {}: {}", code, err));
                }
                if !bus_shim_running(&out, &err) {
                    return Err(
                        "Bus reported start success but is not actually running".to_string()
                    );
                }
                Ok(())
            },
            policy,
        )
        .await?;
    }

    // Now safe to spawn telemetry listener
    spawn_bus_telemetry_listener(cfg.bus.clone(), event_tx.clone());

    let app = Router::new()
        .route("/", get(index))
        .route("/static/pilot_ui.js", get(static_pilot_ui_js))
        .route("/api/command", post(run_command))
        .route("/api/history", get(get_history))
        .route("/api/reports", get(get_reports))
        .route("/api/report", get(get_report_content))
        .route("/api/codex/contracts", get(get_codex_contracts))
        .route("/api/codex/contract", get(get_codex_contract))
        .route("/api/health", get(api_health))
        .route("/api/agorg/list", get(api_agorg_list))
        .route("/api/agorg/active", get(api_agorg_active))
        .route("/api/agorg/scope_snapshot", get(api_agorg_scope_snapshot))
        .route("/api/agorg/preferences", get(api_agorg_preferences))
        .route("/api/agorg/preferences", post(api_agorg_set_preferences))
        .route("/api/agorg/batch-create", post(api_agorg_batch_create))
        .route("/api/agorg/create", post(api_agorg_create))
        .route("/api/agorg/create_project", post(api_agorg_create_project))
        .route("/api/agorg/update", post(api_agorg_update))
        .route("/api/agorg/delete", post(api_agorg_delete))
        .route("/api/agorg/reset", post(api_agorg_reset))
        .route("/api/agorg/use", post(api_agorg_use))
        .route("/api/agorg/discover", post(api_agorg_discover))
        .route(
            "/api/agorg/import_selected",
            post(api_agorg_import_selected),
        )
        .route("/api/agorg/policy_reports", get(get_agorg_policy_reports))
        .route("/api/agorg/policy_report", post(api_agorg_policy_report))
        .route(
            "/api/agorg/dashboard_overview",
            post(api_agorg_dashboard_overview),
        )
        .route("/api/agorg/reconcile", post(api_agorg_reconcile))
        .route(
            "/api/agorg/reconcile_apply",
            post(api_agorg_reconcile_apply),
        )
        .route("/api/agorg/tree", get(api_agorg_tree))
        .route("/api/agorg/repo_options", get(api_agorg_repo_options))
        .route("/api/agorg/link", post(api_agorg_link))
        .route("/api/agorg/scan_master", post(api_agorg_scan_master))
        .route("/api/agorg/upgrade_ago", post(api_agorg_upgrade_ago))
        .route(
            "/api/agorg/edit_relationship",
            post(api_agorg_edit_relationship),
        )
        .route("/api/fs/pick-directory", post(api_fs_pick_directory))
        .route("/api/fs/create-dir", post(api_fs_create_dir))
        .route("/api/dependencies/run", post(run_dependency_action))
        .route("/api/dependencies/logs", get(get_dependency_logs))
        .route("/api/branch/matrix", post(api_branch_matrix))
        .route("/api/multi/selectors", get(api_multi_selectors))
        .route("/api/multi/registry_stats", get(api_multi_registry_stats))
        .route("/api/multi/snapshot", get(api_multi_snapshot))
        .route(
            "/api/dashboard/routine/resolve",
            get(api_dashboard_routine_resolve),
        )
        .route("/api/dashboard/ci/catalog", get(api_dashboard_ci_catalog))
        .route("/api/branch/run", post(api_branch_run))
        .route(
            "/api/branch/conflict-radar",
            post(api_branch_conflict_radar),
        )
        .route("/api/branch/undo-journal", get(api_branch_undo_journal))
        .route("/api/branch/undo", post(api_branch_undo))
        .route("/api/branch/timeline", get(api_branch_timeline))
        .route("/api/orchestrate/timeline", get(api_orchestrate_timeline))
        .route("/api/orchestrate/run", post(api_orchestrate_run))
        .route(
            "/api/orchestrate/graph-status",
            get(api_orchestrate_graph_status),
        )
        .route(
            "/api/system/temporary_components",
            get(get_temporary_components),
        )
        .route(
            "/api/system/temporary_components/checklist",
            get(get_temporary_components_checklist),
        )
        .route(
            "/api/system/acceptance_matrix/run",
            post(run_acceptance_matrix),
        )
        .route(
            "/api/system/temporary_components/export",
            post(export_temporary_components_inventory),
        )
        .route("/api/evidence/export", post(export_evidence_bundle))
        .route("/api/evidence/verify", post(api_evidence_verify))
        .route("/api/settings/policy/:kind", get(api_settings_get_policy))
        .route(
            "/api/settings/policy/:kind/versions",
            get(api_settings_list_policy_versions),
        )
        .route(
            "/api/settings/policy/:kind/draft",
            post(api_settings_draft_policy),
        )
        .route(
            "/api/settings/policy/:kind/load_version",
            post(api_settings_load_policy_version),
        )
        .route(
            "/api/settings/policy/:kind/simulate",
            post(api_settings_simulate_policy),
        )
        .route(
            "/api/settings/policy/:kind/activate",
            post(api_settings_activate_policy),
        )
        .route(
            "/api/settings/policy/:kind/delete_version",
            post(api_settings_delete_policy_version),
        )
        .route(
            "/api/settings/exceptions/:kind",
            get(api_settings_get_exceptions),
        )
        .route(
            "/api/settings/exceptions/:kind",
            post(api_settings_add_exception),
        )
        .route(
            "/api/settings/exceptions/delete/:id",
            post(api_settings_delete_exception),
        )
        .route(
            "/api/settings/overrides/:kind",
            get(api_settings_get_overrides),
        )
        .route(
            "/api/settings/overrides/:kind",
            post(api_settings_create_override),
        )
        .route(
            "/api/settings/overrides/delete/:kind/:ago_encoded",
            post(api_settings_delete_override),
        )
        .route(
            "/api/settings/policy/resolve_trace",
            post(api_settings_resolve_trace),
        )
        .route(
            "/api/settings/governance_scan",
            post(api_settings_governance_scan),
        )
        .route(
            "/api/settings/compliance_scan",
            post(api_settings_compliance_scan),
        )
        .route("/api/settings/decisions", get(api_settings_decisions))
        .route(
            "/api/settings/policy/resolve",
            post(api_settings_policy_resolve),
        )
        .route("/api/codex/action", post(run_codex_action))
        .route("/api/ui/session", get(api_ui_session_get))
        .route("/api/ui/session", post(api_ui_session_set))
        .route("/api/stream", get(stream_events))
        .route("/favicon.ico", get(favicon))
        .with_state(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    println!("Pilot UI listening at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .into_diagnostic()?;
    axum::serve(listener, app).await.into_diagnostic()?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn static_pilot_ui_js() -> impl IntoResponse {
    Response::builder()
        .header("Content-Type", "application/javascript; charset=utf-8")
        .body(axum::body::Body::from(PILOT_UI_JS))
        .unwrap()
}

async fn favicon() -> impl IntoResponse {
    let blank_ico: Vec<u8> = vec![];
    (
        [(axum::http::header::CONTENT_TYPE, "image/x-icon")],
        blank_ico,
    )
}

fn classify_bus_health(
    bus_res: std::result::Result<(i32, String, String), String>,
) -> (bool, &'static str, String) {
    match bus_res {
        Ok((code, out, err)) => {
            if code == 0 && bus_shim_running(&out, &err) {
                (true, "RUNNING", "".to_string())
            } else if code != 0 {
                let note = if err.contains("not found") {
                    "ss utility not found. Install iproute2 or set SS_BIN.".to_string()
                } else {
                    format!(
                        "Status check failed (code {}): {}",
                        code,
                        err.lines().next().unwrap_or("Unknown error")
                    )
                };
                (false, "PROBE_FAILED", note)
            } else {
                (
                    false,
                    "STOPPED",
                    format!(
                        "Shim reports stopped. Err: {}",
                        err.lines().next().unwrap_or("")
                    ),
                )
            }
        }
        Err(e) => (false, "UNAVAILABLE", format!("Failed to run probe: {e}")),
    }
}

fn classify_db_health(
    db_res: std::result::Result<Option<crate::db_runtime::DbStatus>, String>,
) -> (bool, &'static str, String) {
    match db_res {
        Ok(Some(status)) => {
            if status.running {
                (true, "RUNNING", "".to_string())
            } else {
                let note = status
                    .error_note
                    .unwrap_or_else(|| "DB process stopped".to_string());
                (false, "STOPPED", note)
            }
        }
        Ok(None) => (true, "RUNNING", "Managed DB disabled".to_string()),
        Err(e) => (false, "UNAVAILABLE", format!("Check failed: {e}")),
    }
}

fn preflight_steps_from_action(
    action: &str,
    req_steps: Option<Vec<String>>,
) -> Vec<crate::preflight::model::PreflightStepType> {
    use crate::preflight::model::PreflightStepType;
    match action {
        "policy" => vec![PreflightStepType::Policy],
        "hook-policy" => vec![PreflightStepType::Hook],
        "drift" => vec![PreflightStepType::Drift],
        "gate" => vec![PreflightStepType::Gate],
        "prepush-gate" => vec![PreflightStepType::Gate],
        "push" => vec![PreflightStepType::Push],
        _ => {
            if let Some(rlist) = req_steps {
                let mut steps = Vec::new();
                for s in rlist {
                    match s.as_str() {
                        "policy" => steps.push(PreflightStepType::Policy),
                        "hook" => steps.push(PreflightStepType::Hook),
                        "drift" => steps.push(PreflightStepType::Drift),
                        "gate" => steps.push(PreflightStepType::Gate),
                        "push" => steps.push(PreflightStepType::Push),
                        _ => {}
                    }
                }
                if steps.is_empty() {
                    vec![
                        PreflightStepType::Policy,
                        PreflightStepType::Hook,
                        PreflightStepType::Drift,
                        PreflightStepType::Gate,
                    ]
                } else {
                    steps
                }
            } else {
                vec![
                    PreflightStepType::Policy,
                    PreflightStepType::Hook,
                    PreflightStepType::Drift,
                    PreflightStepType::Gate,
                ]
            }
        }
    }
}

async fn api_health(State(state): State<Arc<UiState>>) -> impl IntoResponse {
    let cmd = bus_shim_command("status");
    let bus_future = run_local_script(&cmd);
    let db_future = state.agorg_store.managed_db_status();

    let db_start = std::time::Instant::now();
    let (bus_res, db_res) = tokio::join!(bus_future, db_future);
    let latency_ms = db_start.elapsed().as_millis() as u64;

    let (bus_running, bus_state, bus_note) =
        classify_bus_health(bus_res.map_err(|e| e.to_string()));
    let (db_running, db_state, db_note) = classify_db_health(db_res.map_err(|e| e.to_string()));

    let ok = bus_running && db_running;
    let uptime_secs = now_unix().saturating_sub(state.server_start_time_unix);

    let body = json!({
        "ok": ok,
        "bus": {
            "running": bus_running,
            "state": bus_state,
            "note": bus_note,
            "latency_ms": latency_ms
        },
        "db": {
            "running": db_running,
            "state": db_state,
            "note": db_note,
            "latency_ms": latency_ms
        },
        "uptime_secs": uptime_secs
    });

    Json(body).into_response()
}

async fn run_command(
    State(state): State<Arc<UiState>>,
    Json(mut req): Json<UiCommandRequest>,
) -> Response {
    if !req.command.starts_with("pilot.") {
        return error_response(
            StatusCode::BAD_REQUEST,
            "command must be namespaced as pilot.*",
        );
    }

    if req.payload.get("schema_version").is_none() {
        req.payload["schema_version"] = json!(1);
    }

    if command_scope_required(&req.command) {
        let active_scope = match state.agorg_store.get_active_agorg().await {
            Ok(v) => v,
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        };
        let Some(scope) = active_scope else {
            return error_response(
                StatusCode::PRECONDITION_FAILED,
                "No active AGOrg scope selected. Set AGOrg scope before running this command.",
            );
        };
        req.payload["agorg_scope"] = json!({
            "id": scope.id.to_string(),
            "name": scope.name,
            "root_path": scope.root_path
        });

        if command_requires_cwd_scope(&req.command) {
            let cwd = match std::env::current_dir() {
                Ok(v) => canonicalize_path_lossy(&v),
                Err(err) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("Cannot resolve current working directory: {err}"),
                    )
                }
            };
            let scope_root = canonicalize_path_lossy(Path::new(&scope.root_path));
            if !path_is_within(&cwd, &scope_root) {
                return error_response(
                    StatusCode::FORBIDDEN,
                    &format!(
                        "Current repo path '{}' is outside active AGOrg scope '{}'",
                        cwd.display(),
                        scope_root.display()
                    ),
                );
            }
        }

        if command_requires_multi_selector(&req.command)
            && !payload_has_multi_selector(&req.payload)
        {
            return error_response(
                StatusCode::BAD_REQUEST,
                "Scope guard: multi-repo command requires explicit selector (group or tags).",
            );
        }

        if req.command == "pilot.multi.register" {
            let path_raw = match req.payload.get("path").and_then(Value::as_str) {
                Some(v) if !v.trim().is_empty() => v.trim(),
                _ => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        "Scope guard: pilot.multi.register requires a non-empty 'path'.",
                    )
                }
            };
            let input_path = PathBuf::from(path_raw);
            let normalized = if input_path.is_absolute() {
                canonicalize_path_lossy(&input_path)
            } else {
                match std::env::current_dir() {
                    Ok(cwd) => canonicalize_path_lossy(&cwd.join(input_path)),
                    Err(_) => canonicalize_path_lossy(&input_path),
                }
            };
            let roots = scope_roots(&scope);
            if !path_in_any_root(&normalized, &roots) {
                return error_response(
                    StatusCode::FORBIDDEN,
                    &format!(
                        "Scope guard: repo path '{}' is outside active AGOrg scope.",
                        normalized.display()
                    ),
                );
            }
        }
    }

    if let Some(allowlist) = &state.allowed_commands {
        if !allowlist.contains(&req.command) {
            return error_response(
                StatusCode::FORBIDDEN,
                &format!("command '{}' is not in ui allowlist", req.command),
            );
        }
    }

    if !state.allow_mutations {
        enforce_dry_run(&req.command, &mut req.payload);
        if command_requires_mutation(&req.command, &req.payload) {
            return error_response(
                StatusCode::FORBIDDEN,
                &format!("command '{}' blocked in read-only UI mode", req.command),
            );
        }
    }
    if let Some(err) = branch_policy_violation(&state, &req.command, &req.payload).await {
        return error_response(StatusCode::BAD_REQUEST, &err);
    }

    let command = req.command.clone();
    let payload = req.payload.clone();
    let local_payload = sanitize_payload_for_local_exec(payload.clone());
    if should_prefer_local_command(&command) {
        match run_pilot_subcommand_local(&command, local_payload.clone()) {
            Ok(local_response) => {
                let wrapped = json!({
                    "ok": true,
                    "execution_mode": "local_direct",
                    "response": local_response
                });
                let _ = state.events.send(json!({
                    "source": "ui_command",
                    "command": command,
                    "execution_mode": "local_direct",
                    "response": wrapped,
                }));
                return Json(UiCommandResponse {
                    ok: true,
                    response: wrapped,
                })
                .into_response();
            }
            Err(err) => {
                let msg = format!("Local direct execution failed: {}", err);
                let _ = state.events.send(json!({
                    "source": "ui_command",
                    "command": command,
                    "error": msg,
                }));
                return error_response(StatusCode::BAD_GATEWAY, &msg);
            }
        }
    }

    match send_command_with_bus_recovery(&state, &command, payload.clone()).await {
        Ok(response) => {
            let _ = state.events.send(json!({
                "source": "ui_command",
                "command": command,
                "response": response,
            }));
            Json(UiCommandResponse { ok: true, response }).into_response()
        }
        Err(err) => {
            let err_msg = err.to_string();
            if should_use_local_command_fallback(&err_msg) {
                match run_pilot_subcommand_local(&command, local_payload) {
                    Ok(local_response) => {
                        let wrapped = json!({
                            "ok": true,
                            "fallback_mode": "local_command",
                            "fallback_reason": err_msg,
                            "response": local_response
                        });
                        let _ = state.events.send(json!({
                            "source": "ui_command",
                            "command": command,
                            "fallback": "local_command",
                            "response": wrapped,
                        }));
                        return Json(UiCommandResponse {
                            ok: true,
                            response: wrapped,
                        })
                        .into_response();
                    }
                    Err(local_err) => {
                        let combined = format!(
                            "Bus path failed: {} | Local fallback failed: {}",
                            err_msg, local_err
                        );
                        let _ = state.events.send(json!({
                            "source": "ui_command",
                            "command": command,
                            "error": combined,
                        }));
                        return error_response(StatusCode::BAD_GATEWAY, &combined);
                    }
                }
            }
            let _ = state.events.send(json!({
                "source": "ui_command",
                "command": command,
                "error": err_msg,
            }));
            error_response(StatusCode::BAD_GATEWAY, &err_msg)
        }
    }
}

fn is_bus_recoverable_error(message: &str) -> bool {
    let msg = message.to_ascii_lowercase();
    msg.contains("connection refused")
        || msg.contains("timed out")
        || msg.contains("reset without closing handshake")
        || msg.contains("websocket protocol error")
}

fn should_use_local_command_fallback(message: &str) -> bool {
    let msg = message.to_ascii_lowercase();
    msg.contains("timed out")
        || msg.contains("connection refused")
        || msg.contains("auto-starting arqonbus shim")
        || msg.contains("bus connect failed")
}

fn should_prefer_local_command(command: &str) -> bool {
    command.starts_with("pilot.multi.")
}

fn sanitize_payload_for_local_exec(mut payload: Value) -> Value {
    if let Some(map) = payload.as_object_mut() {
        // UI-injected scope metadata is for server-side guards and not part of CLI schemas.
        map.remove("agorg_scope");
    }
    payload
}

async fn send_command_with_bus_recovery(
    state: &UiState,
    command: &str,
    payload: Value,
) -> Result<Value> {
    // Keep this fast so UI can surface deterministic fallback within client timeout.
    match send_command_once_with_retry(&state.bus, command, payload.clone(), 1).await {
        Ok(response) => Ok(response),
        Err(initial_err) => {
            let initial_msg = initial_err.to_string();
            if !is_bus_recoverable_error(&initial_msg) {
                return Err(initial_err);
            }

            let (code, out, err) = run_local_script(&bus_shim_command("start"))
                .await
                .into_diagnostic()
                .wrap_err("Failed to auto-start ArqonBus shim after bridge error")?;
            if code != 0 {
                let detail = if !err.trim().is_empty() { err } else { out };
                return Err(miette::miette!(
                    "Bus auto-start failed after bridge error: {}",
                    detail.trim()
                ));
            }

            tokio::time::sleep(Duration::from_millis(300)).await;
            send_command_once_with_retry(&state.bus, command, payload, 1)
                .await
                .wrap_err("Command failed after auto-starting ArqonBus shim")
        }
    }
}

async fn require_active_scope(state: &UiState) -> std::result::Result<agorg::Agorg, Response> {
    match state.agorg_store.get_active_agorg().await {
        Ok(Some(scope)) => Ok(scope),
        Ok(None) => Err(error_response(
            StatusCode::PRECONDITION_FAILED,
            "No active AGOrg scope selected. Set AGOrg scope before using Branch Control.",
        )),
        Err(err) => Err(error_response(StatusCode::BAD_REQUEST, &err.to_string())),
    }
}

fn branch_registry() -> std::result::Result<multi::MultiRegistry, Response> {
    multi::MultiRegistry::open(&multi::MultiRegistry::default_db_path())
        .map_err(|err| error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()))
}

fn parse_git_output(path: &Path, args: &[&str]) -> std::result::Result<String, ()> {
    Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .ok_or(())
}

fn branch_row_from_repo(
    repo: &multi::RepoEntry,
    base_branch: &str,
    target_branch: Option<&str>,
) -> BranchMatrixRow {
    let current_branch = parse_git_output(&repo.path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_else(|_| "unknown".to_string());
    let protected = current_branch == "main" || current_branch == "master";
    let clean = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repo.path)
        .output()
        .ok()
        .map(|o| o.status.success() && o.stdout.is_empty())
        .unwrap_or(false);

    let (ahead, behind) = Command::new("git")
        .args([
            "rev-list",
            "--left-right",
            "--count",
            &format!("HEAD...origin/{}", base_branch),
        ])
        .current_dir(&repo.path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let raw = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let mut parts = raw.split_whitespace();
            let left = parts.next()?.parse::<u32>().ok()?;
            let right = parts.next()?.parse::<u32>().ok()?;
            Some((Some(left), Some(right)))
        })
        .unwrap_or((None, None));

    let on_target = target_branch
        .and_then(|t| {
            if t.trim().is_empty() {
                None
            } else {
                Some(t.trim())
            }
        })
        .map(|t| current_branch == t);

    BranchMatrixRow {
        id: repo.id,
        repo: repo.name.clone(),
        path: repo.path.display().to_string(),
        group: repo.group_name.clone(),
        tags: repo.tags.clone(),
        current_branch,
        clean,
        ahead,
        behind,
        on_target,
        protected,
    }
}

fn scope_filter_rows(
    rows: Vec<BranchMatrixRow>,
    scope_roots: &[PathBuf],
    search: Option<&str>,
) -> Vec<BranchMatrixRow> {
    let needle = search
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase());
    rows.into_iter()
        .filter(|row| {
            let path = canonicalize_path_lossy(Path::new(&row.path));
            if !path_in_any_root(&path, scope_roots) {
                return false;
            }
            if let Some(n) = needle.as_ref() {
                let hay = format!(
                    "{} {}",
                    row.repo.to_ascii_lowercase(),
                    row.path.to_ascii_lowercase()
                );
                hay.contains(n)
            } else {
                true
            }
        })
        .collect()
}

fn scope_roots(scope: &agorg::Agorg) -> Vec<PathBuf> {
    let mut roots = vec![canonicalize_path_lossy(Path::new(&scope.root_path))];
    if let Some(master) = scope.master_path.as_deref() {
        let master_root = canonicalize_path_lossy(Path::new(master));
        if !roots.iter().any(|r| r == &master_root) {
            roots.push(master_root);
        }
    }
    roots
}

fn path_in_any_root(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| path_is_within(path, root))
}

#[derive(Debug, Deserialize, Default)]
struct MultiRegistryStatsQuery {
    group: Option<String>,
    tags: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct MultiSnapshotQuery {
    group: Option<String>,
    tags: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct DashboardRoutineResolveQuery {
    group: Option<String>,
    tags: Option<String>,
    branch: Option<String>,
    remote: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct DashboardCiCatalogQuery {
    branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct DashboardCiJobCatalogEntry {
    id: String,
    label: String,
    required_by_policy: bool,
    policy_reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct DashboardCiWorkflowCatalogEntry {
    key: String,
    workflow_name: String,
    workflow_path: String,
    trigger_events: Vec<String>,
    required_by_policy: bool,
    policy_reason: String,
    jobs: Vec<DashboardCiJobCatalogEntry>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct DashboardCiRequirementGap {
    kind: String,
    id: String,
    label: String,
    workflow_key: Option<String>,
    severity: String,
    remediation: String,
}

async fn api_multi_selectors(State(state): State<Arc<UiState>>) -> Response {
    let scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let registry = match branch_registry() {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let roots = scope_roots(&scope);
    let repos = match registry.list_repos(&multi::RepoFilter::default()) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };

    let mut groups: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut tags: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for repo in repos {
        let path = canonicalize_path_lossy(&repo.path);
        if !path_in_any_root(&path, &roots) {
            continue;
        }
        if let Some(group) = repo
            .group_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            groups.insert(group.to_string());
        }
        for tag in repo.tags {
            let t = tag.trim();
            if !t.is_empty() {
                tags.insert(t.to_string());
            }
        }
    }
    Json(json!({
        "ok": true,
        "groups": groups.into_iter().collect::<Vec<_>>(),
        "tags": tags.into_iter().collect::<Vec<_>>()
    }))
    .into_response()
}

async fn api_agorg_repo_options(State(state): State<Arc<UiState>>) -> Response {
    let scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let roots = scope_roots(&scope);

    let mut options: Vec<(String, String, String)> = Vec::new();
    let mut seen_paths: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    let tree = match state.agorg_store.tree(Some(scope.id)).await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };
    let mut agos = Vec::new();
    collect_agos_from_tree(&tree, &mut agos);
    for ago in agos {
        let path = canonicalize_path_lossy(Path::new(&ago.repo_path));
        if !path.exists() || !path_in_any_root(&path, &roots) {
            continue;
        }
        let key = path.display().to_string();
        if seen_paths.insert(key.clone()) {
            options.push((ago.name, key, "agorg_tree".to_string()));
        }
    }

    // Strict mode: only AGOs currently associated/imported into the active AGOrg
    // should appear in repo registry dropdown options.

    options.sort_by(|a, b| {
        a.0.to_lowercase()
            .cmp(&b.0.to_lowercase())
            .then(a.1.cmp(&b.1))
    });
    let items: Vec<Value> = options
        .into_iter()
        .map(|(name, path, source)| json!({ "name": name, "path": path, "source": source }))
        .collect();

    Json(json!({
        "ok": true,
        "agorg_id": scope.id.to_string(),
        "agorg_name": scope.name,
        "items": items
    }))
    .into_response()
}

async fn api_multi_registry_stats(
    State(state): State<Arc<UiState>>,
    Query(query): Query<MultiRegistryStatsQuery>,
) -> Response {
    let scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let registry = match branch_registry() {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let roots = scope_roots(&scope);
    let all = match registry.list_repos(&multi::RepoFilter::default()) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let in_scope_total = all
        .iter()
        .filter(|repo| {
            let path = canonicalize_path_lossy(&repo.path);
            path_in_any_root(&path, &roots)
        })
        .count();

    let tags: Vec<String> = query
        .tags
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();
    let group = query
        .group
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let filtered = if group.is_some() || !tags.is_empty() {
        let repos = match registry.list_repos(&multi::RepoFilter {
            group: group.map(ToString::to_string),
            tags: tags.clone(),
        }) {
            Ok(v) => v,
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        };
        repos
            .into_iter()
            .filter(|repo| {
                let path = canonicalize_path_lossy(&repo.path);
                path_in_any_root(&path, &roots)
            })
            .count()
    } else {
        in_scope_total
    };

    Json(json!({
        "ok": true,
        "total_registered": all.len(),
        "in_scope_total": in_scope_total,
        "filtered_count": filtered,
        "group": group,
        "tags": tags
    }))
    .into_response()
}

fn parse_csv_tags(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

async fn api_multi_snapshot(
    State(state): State<Arc<UiState>>,
    Query(query): Query<MultiSnapshotQuery>,
) -> Response {
    let scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let registry = match branch_registry() {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let roots = scope_roots(&scope);
    let filter = multi::RepoFilter {
        group: query.group.clone().filter(|s| !s.trim().is_empty()),
        tags: parse_csv_tags(query.tags.as_deref()),
    };

    let repos = match registry.list_repos(&filter) {
        Ok(v) => v
            .into_iter()
            .filter(|r| path_in_any_root(&canonicalize_path_lossy(&r.path), &roots))
            .collect::<Vec<_>>(),
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let statuses = match registry.status_repos(&filter) {
        Ok(v) => v
            .into_iter()
            .filter(|s| path_in_any_root(&canonicalize_path_lossy(&s.repo.path), &roots))
            .collect::<Vec<_>>(),
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let order = match registry.dependency_order(&filter) {
        Ok(v) => v
            .into_iter()
            .filter(|r| path_in_any_root(&canonicalize_path_lossy(&r.path), &roots))
            .map(|r| {
                json!({
                    "name": r.name,
                    "path": r.path.display().to_string(),
                    "group": r.group_name,
                    "tags": r.tags
                })
            })
            .collect::<Vec<_>>(),
        Err(err) => {
            return Json(json!({
                "ok": false,
                "error": format!("Order failed: {}", err),
                "repos": repos.iter().map(|r| json!({"name": r.name, "path": r.path.display().to_string(), "group": r.group_name, "tags": r.tags})).collect::<Vec<_>>(),
                "statuses": statuses.iter().map(|s| json!({
                    "name": s.repo.name,
                    "path": s.repo.path.display().to_string(),
                    "exists": s.path_exists,
                    "git_repo": s.is_git_repo,
                    "clean": s.git_clean,
                    "pilot_initialized": s.pilot_initialized,
                    "oracle_ready": s.oracle_ready
                })).collect::<Vec<_>>(),
            }))
            .into_response();
        }
    };
    let dag = match registry.dependency_dag_report(&filter) {
        Ok(v) => json!({
            "repos": v.repos.into_iter().filter(|r| path_in_any_root(&canonicalize_path_lossy(&r.path), &roots)).map(|r| r.name).collect::<Vec<_>>(),
            "edges": v.edges,
            "stages": v.stages
        }),
        Err(err) => json!({"error": err.to_string()}),
    };

    Json(json!({
        "ok": true,
        "filter": {
            "group": filter.group,
            "tags": filter.tags
        },
        "repos": repos.iter().map(|r| json!({
            "name": r.name,
            "path": r.path.display().to_string(),
            "group": r.group_name,
            "tags": r.tags
        })).collect::<Vec<_>>(),
        "statuses": statuses.iter().map(|s| json!({
            "name": s.repo.name,
            "path": s.repo.path.display().to_string(),
            "exists": s.path_exists,
            "git_repo": s.is_git_repo,
            "clean": s.git_clean,
            "pilot_initialized": s.pilot_initialized,
            "oracle_ready": s.oracle_ready
        })).collect::<Vec<_>>(),
        "order": order,
        "dag": dag
    }))
    .into_response()
}

fn dashboard_routine_guard_summary(report: &PolicyEvalReport) -> Value {
    let violations = report
        .violations
        .iter()
        .map(|item| {
            json!({
                "rule": item.rule,
                "level": item.level,
                "input": item.input,
                "violation": item.violation,
                "remediation": item.fix_suggestion,
                "source_name": item.policy_source_name,
            })
        })
        .collect::<Vec<_>>();
    let warnings = report
        .warnings
        .iter()
        .map(|item| {
            json!({
                "rule": item.rule,
                "level": item.level,
                "input": item.input,
                "violation": item.violation,
                "remediation": item.fix_suggestion,
                "source_name": item.policy_source_name,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "blocked": report.blocked,
        "violation_count": report.violations.len(),
        "warning_count": report.warnings.len(),
        "violations": violations,
        "warnings": warnings,
    })
}

async fn api_dashboard_routine_resolve(
    State(state): State<Arc<UiState>>,
    Query(query): Query<DashboardRoutineResolveQuery>,
) -> Response {
    let active_scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let registry = match branch_registry() {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let roots = scope_roots(&active_scope);
    let tags = parse_csv_tags(query.tags.as_deref());
    let filter = multi::RepoFilter {
        group: query.group.clone().filter(|s| !s.trim().is_empty()),
        tags: tags.clone(),
    };
    let branch = query
        .branch
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("main")
        .to_string();
    let remote = query
        .remote
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("origin")
        .to_string();

    let repos = match registry.list_repos(&filter) {
        Ok(v) => v
            .into_iter()
            .filter(|repo| path_in_any_root(&canonicalize_path_lossy(&repo.path), &roots))
            .collect::<Vec<_>>(),
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let statuses = match registry.status_repos(&filter) {
        Ok(v) => v
            .into_iter()
            .filter(|status| path_in_any_root(&canonicalize_path_lossy(&status.repo.path), &roots))
            .collect::<Vec<_>>(),
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let in_scope_total = match registry.list_repos(&multi::RepoFilter::default()) {
        Ok(v) => v
            .into_iter()
            .filter(|repo| path_in_any_root(&canonicalize_path_lossy(&repo.path), &roots))
            .count(),
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let clean_count = statuses
        .iter()
        .filter(|s| s.git_clean.unwrap_or(false))
        .count();
    let dirty_count = statuses.len().saturating_sub(clean_count);
    let initialized_count = statuses.iter().filter(|s| s.pilot_initialized).count();
    let oracle_ready_count = statuses.iter().filter(|s| s.oracle_ready).count();

    let cwd = match std::env::current_dir() {
        Ok(v) => canonicalize_path_lossy(&v),
        Err(err) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Cannot resolve current working directory: {err}"),
            )
        }
    };
    let cwd_for_lookup = cwd.display().to_string();
    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    let effective = gov_store
        .get_effective_policy_record(active_scope.id, cwd_for_lookup.as_str(), "operator_routine")
        .await
        .ok()
        .flatten();
    let (policy_json, policy_source, policy_version, policy_status) = match effective {
        Some((record, source_name)) => (
            record.policy_json,
            source_name,
            record.version,
            record.status,
        ),
        None => (
            serde_json::to_value(OperatorRoutinePolicy::default()).unwrap_or(json!({})),
            "Built-in default".to_string(),
            0,
            "fallback".to_string(),
        ),
    };
    let policy: OperatorRoutinePolicy = serde_json::from_value(policy_json.clone())
        .unwrap_or_else(|_| OperatorRoutinePolicy::default());
    let profile = policy.post_commit_profile.clone();
    let steps = profile
        .step_order
        .iter()
        .map(|step| match step.as_str() {
            "scope" => "resolve".to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>();
    let req = DependencyActionRequest {
        action: "push".to_string(),
        json: false,
        branch: Some(branch.clone()),
        remote: Some(remote.clone()),
        preflight_steps: Some(vec![
            "policy".to_string(),
            "hook".to_string(),
            "drift".to_string(),
            "gate".to_string(),
        ]),
        label: None,
        bundle_path: None,
        ci_timeout_sec: None,
    };
    let guard = evaluate_operator_routine_guard(&state, &active_scope, "push", &req)
        .await
        .unwrap_or_default();
    let mut plan_stages = vec!["resolve".to_string(), "plan".to_string()];
    plan_stages.extend(steps.clone());
    if !plan_stages.iter().any(|step| step == "reconcile") {
        plan_stages.push("reconcile".to_string());
    }

    Json(json!({
        "ok": true,
        "active_scope": {
            "id": active_scope.id,
            "name": active_scope.name,
            "root_path": active_scope.root_path,
        },
        "selector": {
            "group": filter.group,
            "tags": filter.tags,
        },
        "cohort": {
            "filtered_count": repos.len(),
            "in_scope_total": in_scope_total,
            "clean_count": clean_count,
            "dirty_count": dirty_count,
            "pilot_initialized_count": initialized_count,
            "oracle_ready_count": oracle_ready_count,
        },
        "repos": repos.iter().map(|repo| {
            json!({
                "name": repo.name,
                "path": repo.path.display().to_string(),
                "group": repo.group_name,
                "tags": repo.tags,
            })
        }).collect::<Vec<_>>(),
        "statuses": statuses.iter().map(|status| {
            json!({
                "name": status.repo.name,
                "path": status.repo.path.display().to_string(),
                "clean": status.git_clean,
                "pilot_initialized": status.pilot_initialized,
                "oracle_ready": status.oracle_ready,
                "git_repo": status.is_git_repo,
                "exists": status.path_exists,
            })
        }).collect::<Vec<_>>(),
        "resolved_policy": {
            "source": policy_source,
            "version": policy_version,
            "status": policy_status,
            "profile": profile.clone(),
            "policy_json": policy_json,
        },
        "plan": {
            "stages": plan_stages,
            "mutation_boundary": {
                "push_enabled": profile.include_push_step,
                "evidence_enabled": profile.export_evidence_step,
                "stop_on_fail": profile.stop_on_fail,
            },
            "push_target": {
                "branch": branch,
                "remote": remote,
            },
        },
        "guard_summary": dashboard_routine_guard_summary(&guard),
    }))
    .into_response()
}

fn workflow_policy_expectation(
    file_name: &str,
    workflow_name: &str,
    ci_enabled: bool,
) -> (bool, String) {
    if !ci_enabled {
        return (
            false,
            "Routine policy does not require the CI stage in the active post-commit profile."
                .to_string(),
        );
    }
    let lower_file = file_name.to_ascii_lowercase();
    let lower_name = workflow_name.to_ascii_lowercase();
    if lower_file == "ci.yml"
        || lower_file == "ci.yaml"
        || (lower_name.contains("pilot") && lower_name.contains("ci"))
    {
        return (
            true,
            "Core CI workflow required by the ArqonPilot frozen CI contract.".to_string(),
        );
    }
    if lower_file == "docs.yml" || lower_file == "docs.yaml" || lower_name.contains("docs") {
        return (
            true,
            "Docs workflow required to preserve Dashboard and MkDocs publication parity."
                .to_string(),
        );
    }
    if lower_file == "pypi.yml" || lower_file == "pypi.yaml" || lower_name.contains("pypi") {
        return (
            false,
            "PyPI publish workflow is release-scoped and observed as optional during routine CI."
                .to_string(),
        );
    }
    (
        false,
        "Workflow is discovered dynamically but not currently mandated by routine CI policy."
            .to_string(),
    )
}

fn workflow_job_policy_expectation(
    workflow_key: &str,
    job_id: &str,
    ci_enabled: bool,
) -> (bool, String) {
    if !ci_enabled {
        return (
            false,
            "Routine policy currently disables the CI stage.".to_string(),
        );
    }
    if workflow_key == "ci.yml" {
        match job_id {
            "rust" => {
                return (
                    true,
                    "Rust lane is required by the frozen 1.82.0 core validation contract."
                        .to_string(),
                )
            }
            "ui-smoke" => {
                return (
                    true,
                    "UI smoke lane is required to protect the operator surface contract."
                        .to_string(),
                )
            }
            "packaging-parity" => {
                return (
                    true,
                    "Packaging parity lane is required to enforce the scoped 1.88.0 exception."
                        .to_string(),
                )
            }
            _ => {}
        }
    }
    if workflow_key == "docs.yml" && job_id == "build" {
        return (
            true,
            "Docs build job is required to keep MkDocs artifacts valid and publishable."
                .to_string(),
        );
    }
    (
        false,
        "Job is discovered dynamically and remains informational unless CI policy expands."
            .to_string(),
    )
}

fn parse_workflow_catalog_entry(
    path: &Path,
    content: &str,
    ci_enabled: bool,
) -> DashboardCiWorkflowCatalogEntry {
    let workflow_key = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workflow")
        .to_string();
    let workflow_path = path.to_string_lossy().to_string();
    let mut workflow_name = workflow_key.clone();
    let mut trigger_events = Vec::<String>::new();
    let mut jobs: Vec<DashboardCiJobCatalogEntry> = Vec::new();
    let mut in_on = false;
    let mut in_jobs = false;
    let mut current_job_idx = None::<usize>;

    for raw in content.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = raw.chars().take_while(|c| *c == ' ').count();

        if indent == 0 && trimmed.starts_with("name:") {
            let value = trimmed.trim_start_matches("name:").trim();
            if !value.is_empty() {
                workflow_name = value.trim_matches('"').trim_matches('\'').to_string();
            }
            continue;
        }

        if indent == 0 && trimmed.starts_with("on:") {
            in_on = true;
            in_jobs = false;
            current_job_idx = None;
            let inline = trimmed.trim_start_matches("on:").trim();
            if inline.starts_with('[') && inline.ends_with(']') {
                let inner = inline.trim_matches(|c| c == '[' || c == ']');
                for item in inner.split(',') {
                    let event = item.trim().trim_matches('"').trim_matches('\'');
                    if !event.is_empty() && !trigger_events.iter().any(|v| v == event) {
                        trigger_events.push(event.to_string());
                    }
                }
            } else if !inline.is_empty() && inline != "{}" {
                let event = inline.trim_matches('"').trim_matches('\'');
                if !event.is_empty() && !trigger_events.iter().any(|v| v == event) {
                    trigger_events.push(event.to_string());
                }
            }
            continue;
        }

        if indent == 0 && trimmed.starts_with("jobs:") {
            in_jobs = true;
            in_on = false;
            current_job_idx = None;
            continue;
        }

        if indent == 0 {
            in_on = false;
            in_jobs = false;
            current_job_idx = None;
        }

        if in_on && indent == 2 && trimmed.ends_with(':') {
            let key = trimmed.trim_end_matches(':').trim();
            if !key.is_empty() && !trigger_events.iter().any(|v| v == key) {
                trigger_events.push(key.to_string());
            }
            continue;
        }

        if in_jobs {
            if indent == 2 && trimmed.ends_with(':') && !trimmed.starts_with('-') {
                let job_id = trimmed.trim_end_matches(':').trim();
                if !job_id.is_empty() {
                    let (required_by_policy, policy_reason) =
                        workflow_job_policy_expectation(&workflow_key, job_id, ci_enabled);
                    jobs.push(DashboardCiJobCatalogEntry {
                        id: job_id.to_string(),
                        label: job_id.to_string(),
                        required_by_policy,
                        policy_reason,
                    });
                    current_job_idx = jobs.len().checked_sub(1);
                }
                continue;
            }
            if indent >= 4 && trimmed.starts_with("name:") {
                if let Some(idx) = current_job_idx {
                    let value = trimmed.trim_start_matches("name:").trim();
                    if !value.is_empty() {
                        jobs[idx].label = value.trim_matches('"').trim_matches('\'').to_string();
                    }
                }
            }
        }
    }

    let (required_by_policy, policy_reason) =
        workflow_policy_expectation(&workflow_key, &workflow_name, ci_enabled);
    DashboardCiWorkflowCatalogEntry {
        key: workflow_key,
        workflow_name,
        workflow_path,
        trigger_events,
        required_by_policy,
        policy_reason,
        jobs,
    }
}

fn discover_dashboard_ci_catalog(
    workflows_dir: &Path,
    policy: &OperatorRoutinePolicy,
) -> std::result::Result<
    (
        Vec<DashboardCiWorkflowCatalogEntry>,
        Vec<DashboardCiRequirementGap>,
        bool,
        Vec<String>,
    ),
    String,
> {
    let ci_enabled = policy
        .post_commit_profile
        .step_order
        .iter()
        .any(|step| step.eq_ignore_ascii_case("ci"));
    let mut warnings = Vec::<String>::new();
    let mut workflow_paths = Vec::<PathBuf>::new();
    match fs::read_dir(workflows_dir) {
        Ok(entries) => {
            workflow_paths = entries
                .filter_map(|entry| entry.ok().map(|item| item.path()))
                .filter(|path| {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| matches!(ext, "yml" | "yaml"))
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();
        }
        Err(err) => {
            warnings.push(format!(
                "Cannot read workflow directory '{}': {err}",
                workflows_dir.display()
            ));
        }
    }
    workflow_paths.sort();

    let mut workflows = Vec::new();
    for path in workflow_paths {
        let content = match fs::read_to_string(&path) {
            Ok(v) => v,
            Err(err) => {
                warnings.push(format!(
                    "Cannot read workflow file '{}': {err}",
                    path.display()
                ));
                continue;
            }
        };
        workflows.push(parse_workflow_catalog_entry(&path, &content, ci_enabled));
    }

    let mut gaps = Vec::<DashboardCiRequirementGap>::new();
    if ci_enabled {
        let has_ci = workflows
            .iter()
            .any(|wf| wf.key == "ci.yml" || wf.key == "ci.yaml");
        if !has_ci {
            gaps.push(DashboardCiRequirementGap {
                kind: "workflow".to_string(),
                id: "ci.yml".to_string(),
                label: "Core CI workflow".to_string(),
                workflow_key: None,
                severity: "high".to_string(),
                remediation: "Restore .github/workflows/ci.yml to satisfy the frozen CI contract."
                    .to_string(),
            });
        }
        let has_docs = workflows
            .iter()
            .any(|wf| wf.key == "docs.yml" || wf.key == "docs.yaml");
        if !has_docs {
            gaps.push(DashboardCiRequirementGap {
                kind: "workflow".to_string(),
                id: "docs.yml".to_string(),
                label: "Docs workflow".to_string(),
                workflow_key: None,
                severity: "medium".to_string(),
                remediation:
                    "Restore .github/workflows/docs.yml to preserve MkDocs validation and deployment."
                        .to_string(),
            });
        }
        if let Some(ci_workflow) = workflows
            .iter()
            .find(|wf| wf.key == "ci.yml" || wf.key == "ci.yaml")
        {
            for required_job in ["rust", "ui-smoke", "packaging-parity"] {
                if !ci_workflow.jobs.iter().any(|job| job.id == required_job) {
                    gaps.push(DashboardCiRequirementGap {
                        kind: "job".to_string(),
                        id: required_job.to_string(),
                        label: format!("Required CI job '{required_job}'"),
                        workflow_key: Some(ci_workflow.key.clone()),
                        severity: "high".to_string(),
                        remediation: format!(
                            "Add job '{required_job}' back to {} to satisfy the frozen CI contract.",
                            ci_workflow.key
                        ),
                    });
                }
            }
        }
        if let Some(docs_workflow) = workflows
            .iter()
            .find(|wf| wf.key == "docs.yml" || wf.key == "docs.yaml")
        {
            if !docs_workflow.jobs.iter().any(|job| job.id == "build") {
                gaps.push(DashboardCiRequirementGap {
                    kind: "job".to_string(),
                    id: "build".to_string(),
                    label: "Required docs build job".to_string(),
                    workflow_key: Some(docs_workflow.key.clone()),
                    severity: "medium".to_string(),
                    remediation: format!(
                        "Restore the 'build' job in {} so MkDocs validation remains enforced.",
                        docs_workflow.key
                    ),
                });
            }
        }
    }

    Ok((workflows, gaps, ci_enabled, warnings))
}

async fn api_dashboard_ci_catalog(
    State(state): State<Arc<UiState>>,
    Query(query): Query<DashboardCiCatalogQuery>,
) -> Response {
    let active_scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let branch = query
        .branch
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("main")
        .to_string();
    let cwd = match std::env::current_dir() {
        Ok(v) => canonicalize_path_lossy(&v),
        Err(err) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("Cannot resolve current working directory: {err}"),
            )
        }
    };
    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    let cwd_for_lookup = cwd.display().to_string();
    let effective = gov_store
        .get_effective_policy_record(active_scope.id, cwd_for_lookup.as_str(), "operator_routine")
        .await
        .ok()
        .flatten();
    let (policy_json, policy_source, policy_version, policy_status) = match effective {
        Some((record, source_name)) => (
            record.policy_json,
            source_name,
            record.version,
            record.status,
        ),
        None => (
            serde_json::to_value(OperatorRoutinePolicy::default()).unwrap_or(json!({})),
            "Built-in default".to_string(),
            0,
            "fallback".to_string(),
        ),
    };
    let policy: OperatorRoutinePolicy =
        serde_json::from_value(policy_json).unwrap_or_else(|_| OperatorRoutinePolicy::default());
    let workflows_dir = cwd.join(".github").join("workflows");
    let (workflows, gaps, ci_enabled, warnings) =
        match discover_dashboard_ci_catalog(&workflows_dir, &policy) {
            Ok(v) => v,
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err),
        };

    Json(json!({
        "ok": true,
        "branch": branch,
        "workflows_dir": workflows_dir.display().to_string(),
        "policy_basis": {
            "source": policy_source,
            "version": policy_version,
            "status": policy_status,
            "ci_stage_enabled": ci_enabled,
            "step_order": policy.post_commit_profile.step_order,
            "frozen_contract": {
                "core_rust": "1.82.0",
                "packaging_rust": "1.88.0",
                "protobuf": "4.25.8",
                "protoc": "25.8"
            }
        },
        "summary": {
            "workflow_count": workflows.len(),
            "required_workflow_count": workflows.iter().filter(|wf| wf.required_by_policy).count(),
            "required_job_count": workflows.iter().flat_map(|wf| wf.jobs.iter()).filter(|job| job.required_by_policy).count(),
            "gap_count": gaps.len(),
            "warning_count": warnings.len()
        },
        "workflows": workflows,
        "missing": gaps,
        "warnings": warnings
    }))
    .into_response()
}

fn collect_agos_from_tree(nodes: &[agorg::AgorgTreeNode], out: &mut Vec<agorg::AgoRecord>) {
    for node in nodes {
        out.extend(node.agos.iter().cloned());
        collect_agos_from_tree(&node.child_agorgs, out);
    }
}

async fn bootstrap_branch_registry_from_scope(
    state: &UiState,
    scope: &agorg::Agorg,
) -> std::result::Result<usize, String> {
    let tree = state
        .agorg_store
        .tree(Some(scope.id))
        .await
        .map_err(|e| format!("Failed to read AGOrg tree: {e}"))?;
    let mut agos = Vec::new();
    collect_agos_from_tree(&tree, &mut agos);
    if agos.is_empty() {
        return Ok(0);
    }
    let registry = branch_registry().map_err(|_| "Failed to open branch registry".to_string())?;
    let roots = scope_roots(scope);
    let mut upserted = 0usize;
    for ago in agos {
        let path = canonicalize_path_lossy(Path::new(&ago.repo_path));
        if !path.exists() || !path_in_any_root(&path, &roots) {
            continue;
        }
        let tags: Vec<String> = ago
            .relationship_children
            .iter()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect();
        if registry
            .register_repo(&path, Some(&ago.name), None, &tags)
            .is_ok()
        {
            upserted += 1;
        }
    }
    Ok(upserted)
}

async fn discover_and_import_scope_agos(
    state: &UiState,
    scope: &agorg::Agorg,
) -> std::result::Result<usize, String> {
    let depth = usize::try_from(scope.scan_depth.max(1)).unwrap_or(4);
    let discovery = agorg::discover_hierarchy(Path::new(&scope.root_path), depth)
        .map_err(|e| format!("Failed AGOrg discovery: {e}"))?;
    let imported = state
        .agorg_store
        .import_discovery(scope.id, &discovery)
        .await
        .map_err(|e| format!("Failed AGOrg import: {e}"))?;
    Ok(imported.upserted)
}

fn sorted_unique_tags(tags: &[String]) -> Vec<String> {
    let mut v: Vec<String> = tags
        .iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    v.sort();
    v.dedup();
    v
}

fn sorted_unique_ids(ids: &[i64]) -> Vec<i64> {
    let mut v: Vec<i64> = ids.to_vec();
    v.sort();
    v.dedup();
    v
}

fn canonical_branch_payload(req: &BranchRunRequest, dry_run: bool) -> Value {
    json!({
        "action": req.action.trim().to_ascii_lowercase(),
        "branch": req.branch.as_deref().unwrap_or("").trim(),
        "base_branch": req.base_branch.as_deref().unwrap_or("main").trim(),
        "dry_run": dry_run,
        "group": req.group.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        "tags": sorted_unique_tags(&req.tags),
        "selected_repo_ids": sorted_unique_ids(&req.selected_repo_ids),
    })
}

fn prune_expired_branch_previews(previews: &mut HashMap<String, BranchPreviewRecord>, now: u64) {
    previews.retain(|_, rec| rec.expires_at_unix >= now);
}

async fn branch_policy_violation(
    state: &UiState,
    command: &str,
    payload: &Value,
) -> Option<String> {
    let policy = match state.agorg_store.get_active_agorg().await {
        Ok(Some(active)) => {
            let gov = GovernanceStore::new(state.agorg_store.dsn());
            match gov.get_policy(active.id, "branch").await.unwrap_or(None) {
                Some(r) => serde_json::from_value(r.policy_json)
                    .unwrap_or_else(|_| BranchPolicy::default()),
                None => BranchPolicy::default(),
            }
        }
        _ => BranchPolicy::default(),
    };

    // For simplicity in the generic command flow, we use empty exceptions as we can't easily resolve ago_path here.
    // Full exception precedence is applied in specific routes like `api_branch_run`.
    let exceptions = vec![];

    if command == "pilot.branch.create" && command_requires_mutation(command, payload) {
        let branch = payload
            .get("branch")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if branch.is_empty() {
            return Some("branch is required for create".to_string());
        }
        let report = evaluate_branch_policy(
            &policy,
            "create",
            &branch,
            &exceptions,
            "",
            "Active Scope",
            state
                .agorg_store
                .get_active_agorg()
                .await
                .ok()
                .flatten()
                .map(|a| a.id),
        );
        if report.blocked {
            let msgs = report
                .violations
                .iter()
                .map(|v| v.violation.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Some(format!("Governance Policy Violation: {}", msgs));
        }
    }
    if command == "pilot.multi.apply" && command_requires_mutation(command, payload) {
        let branch = payload
            .get("branch")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if branch.is_empty() {
            return Some("branch is required for multi apply".to_string());
        }
        let report = evaluate_branch_policy(
            &policy,
            "create",
            &branch,
            &exceptions,
            "",
            "Active Scope",
            state
                .agorg_store
                .get_active_agorg()
                .await
                .ok()
                .flatten()
                .map(|a| a.id),
        );
        if report.blocked {
            let msgs = report
                .violations
                .iter()
                .map(|v| v.violation.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Some(format!("Governance Policy Violation: {}", msgs));
        }
    }
    if command == "pilot.branch.prune" && command_requires_mutation(command, payload) {
        let phrase = payload
            .get("confirm_phrase")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_uppercase();
        let expected = if policy.lifecycle.prune_requires_confirmation {
            policy.lifecycle.confirmation_phrase.to_ascii_uppercase()
        } else {
            "PRUNE".to_string()
        };
        if policy.lifecycle.prune_requires_confirmation && phrase != expected {
            return Some(format!(
                "prune execute requires confirm_phrase={}",
                expected
            ));
        }
    }
    None
}

async fn api_branch_matrix(
    State(state): State<Arc<UiState>>,
    Json(req): Json<BranchMatrixRequest>,
) -> Response {
    let scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let filter = multi::RepoFilter {
        group: req.group.clone(),
        tags: req.tags.clone(),
    };
    let base_branch = req.base_branch.unwrap_or_else(|| "main".to_string());
    let roots = scope_roots(&scope);
    let mut rows: Vec<BranchMatrixRow> = {
        let registry = match branch_registry() {
            Ok(v) => v,
            Err(resp) => return resp,
        };
        let repos = match registry.list_repos(&filter) {
            Ok(v) => v,
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        };
        repos
            .iter()
            .map(|repo| branch_row_from_repo(repo, &base_branch, req.target_branch.as_deref()))
            .collect()
    };
    rows = scope_filter_rows(rows, &roots, req.search.as_deref());
    let mut bootstrapped = 0usize;
    let mut autodiscovered = 0usize;
    let mut matrix_source = "registry".to_string();
    if rows.is_empty() {
        match bootstrap_branch_registry_from_scope(&state, &scope).await {
            Ok(count) => {
                bootstrapped = count;
                if count > 0 {
                    matrix_source = "bootstrapped".to_string();
                    let registry = match branch_registry() {
                        Ok(v) => v,
                        Err(resp) => return resp,
                    };
                    let repos = match registry.list_repos(&filter) {
                        Ok(v) => v,
                        Err(err) => {
                            return error_response(StatusCode::BAD_REQUEST, &err.to_string());
                        }
                    };
                    rows = repos
                        .iter()
                        .map(|repo| {
                            branch_row_from_repo(repo, &base_branch, req.target_branch.as_deref())
                        })
                        .collect();
                    rows = scope_filter_rows(rows, &roots, req.search.as_deref());
                }
            }
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err),
        }
    }
    // If scope-backed AGOs were not present yet, auto-discover/import once and retry.
    if rows.is_empty() {
        match discover_and_import_scope_agos(&state, &scope).await {
            Ok(count) => {
                autodiscovered = count;
                if count > 0 {
                    matrix_source = "autodiscovered".to_string();
                    if let Err(err) = bootstrap_branch_registry_from_scope(&state, &scope).await {
                        return error_response(StatusCode::BAD_REQUEST, &err);
                    }
                    let registry = match branch_registry() {
                        Ok(v) => v,
                        Err(resp) => return resp,
                    };
                    let repos = match registry.list_repos(&filter) {
                        Ok(v) => v,
                        Err(err) => {
                            return error_response(StatusCode::BAD_REQUEST, &err.to_string());
                        }
                    };
                    rows = repos
                        .iter()
                        .map(|repo| {
                            branch_row_from_repo(repo, &base_branch, req.target_branch.as_deref())
                        })
                        .collect();
                    rows = scope_filter_rows(rows, &roots, req.search.as_deref());
                }
            }
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err),
        }
    }
    rows.sort_by(|a, b| a.repo.cmp(&b.repo));
    Json(json!({
        "ok": true,
        "base_branch": base_branch,
        "source": matrix_source,
        "bootstrapped": bootstrapped,
        "autodiscovered": autodiscovered,
        "count": rows.len(),
        "rows": rows
    }))
    .into_response()
}

fn resolve_branch_targets(
    registry: &multi::MultiRegistry,
    req: &BranchRunRequest,
    scope_roots: &[PathBuf],
) -> std::result::Result<Vec<multi::RepoEntry>, String> {
    let selected: HashSet<i64> = req.selected_repo_ids.iter().copied().collect();
    let list_filter = if selected.is_empty() {
        multi::RepoFilter {
            group: req.group.clone(),
            tags: req.tags.clone(),
        }
    } else {
        multi::RepoFilter::default()
    };
    let mut repos = registry
        .list_repos(&list_filter)
        .map_err(|e| format!("Failed to list repos: {e}"))?;
    if !selected.is_empty() {
        repos.retain(|r| selected.contains(&r.id));
    }
    repos.retain(|repo| {
        let path = canonicalize_path_lossy(&repo.path);
        path_in_any_root(&path, scope_roots)
    });
    if repos.is_empty() {
        return Err(
            "No repositories matched selected filters/selection within active AGOrg scope."
                .to_string(),
        );
    }
    Ok(repos)
}

async fn api_branch_run(
    State(state): State<Arc<UiState>>,
    Json(req): Json<BranchRunRequest>,
) -> Response {
    let scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let action = req.action.trim().to_ascii_lowercase();
    if !matches!(action.as_str(), "create" | "sync" | "prune" | "status") {
        return error_response(
            StatusCode::BAD_REQUEST,
            "action must be one of: create, sync, prune, status",
        );
    }
    let dry_run = req.dry_run.unwrap_or(true);
    let mutating = matches!(action.as_str(), "create" | "sync" | "prune") && !dry_run;
    if mutating && !state.allow_mutations {
        return error_response(
            StatusCode::FORBIDDEN,
            "branch action blocked in read-only UI mode; restart pilot serve with --ui-allow-mutations",
        );
    }
    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    let policy_record = gov_store
        .get_policy(scope.id, "branch")
        .await
        .unwrap_or(None);
    let policy = match policy_record {
        Some(r) => {
            serde_json::from_value(r.policy_json).unwrap_or_else(|_| BranchPolicy::default())
        }
        None => BranchPolicy::default(),
    };
    let exceptions = gov_store
        .get_exceptions(scope.id, "branch")
        .await
        .unwrap_or_default();

    if action == "create" && !dry_run {
        let branch_name = req.branch.as_deref().unwrap_or("").trim().to_string();
        if branch_name.is_empty() {
            return error_response(
                StatusCode::BAD_REQUEST,
                "branch is required for create action",
            );
        }

        let registry =
            multi::MultiRegistry::open(&multi::MultiRegistry::default_db_path()).unwrap();
        let roots = scope_roots(&scope);
        let repos = match resolve_branch_targets(&registry, &req, &roots) {
            Ok(v) => v,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, &e),
        };

        for repo in repos {
            let repo_path = canonicalize_path_lossy(&repo.path);
            let ago_path = repo_path
                .strip_prefix(Path::new(&scope.root_path))
                .unwrap_or(&repo_path)
                .display()
                .to_string();
            let report = evaluate_branch_policy(
                &policy,
                "create",
                &branch_name,
                &exceptions,
                &ago_path,
                "Override",
                Some(scope.id),
            );
            if report.blocked {
                let body = json!({
                    "error": format!("Governance Policy Violation in {}: Branch creation blocked", repo.name),
                    "policy_report": report
                });
                return (StatusCode::BAD_REQUEST, Json(body)).into_response();
            }
        }
    }

    if action == "prune" && !dry_run {
        let phrase = req
            .confirm_phrase
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_uppercase();

        let expected_phrase = if policy.lifecycle.prune_requires_confirmation {
            policy.lifecycle.confirmation_phrase.to_ascii_uppercase()
        } else {
            "PRUNE".to_string()
        };

        if policy.lifecycle.prune_requires_confirmation && phrase != expected_phrase {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("prune execute requires confirm_phrase={}", expected_phrase),
            );
        }

        // We also need a branch to check pruning against protected branches, but prune targets all merged branches
        // so we can't do a pre-flight name check easily without inspecting the repo.
        // `pilot-branch` does its own checks, but we'll leave the pre-flight minimal.
    }

    let now = now_unix();
    let mut issued_preview_token: Option<BranchPreviewRecord> = None;
    if matches!(action.as_str(), "create" | "sync" | "prune") {
        if dry_run {
            let token = format!("branch-preview-{}", Uuid::new_v4());
            let expected_execute_payload = canonical_branch_payload(&req, false);
            let record = BranchPreviewRecord {
                token: token.clone(),
                scope_id: scope.id,
                action: action.clone(),
                expected_execute_payload,
                created_at_unix: now,
                expires_at_unix: now + 900,
            };
            let mut previews = state.branch_previews.lock().await;
            prune_expired_branch_previews(&mut previews, now);
            previews.insert(token.clone(), record.clone());
            issued_preview_token = Some(record.clone());
            let _ = state.events.send(json!({
                "source": "branch_control",
                "action": action,
                "phase": "preview_token_issued",
                "token": token,
                "expires_at_unix": record.expires_at_unix
            }));
        } else {
            let token = req
                .preview_token
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_string();
            if token.is_empty() {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "preview_token is required for execute. Run preview first.",
                );
            }
            let expected_payload = canonical_branch_payload(&req, false);
            let mut previews = state.branch_previews.lock().await;
            prune_expired_branch_previews(&mut previews, now);
            let Some(record) = previews.get(&token).cloned() else {
                return error_response(
                    StatusCode::PRECONDITION_FAILED,
                    "preview_token not found or expired. Re-run preview.",
                );
            };
            if record.scope_id != scope.id {
                previews.remove(&token);
                return error_response(
                    StatusCode::PRECONDITION_FAILED,
                    "preview_token scope mismatch. Re-run preview in current AGOrg scope.",
                );
            }
            if record.action != action {
                previews.remove(&token);
                return error_response(
                    StatusCode::PRECONDITION_FAILED,
                    "preview_token action mismatch. Re-run preview.",
                );
            }
            if record.expected_execute_payload != expected_payload {
                previews.remove(&token);
                return error_response(
                    StatusCode::PRECONDITION_FAILED,
                    "preview payload mismatch. Inputs/selection changed; re-run preview.",
                );
            }
            previews.remove(&token);
        }
    }

    let registry = match branch_registry() {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let roots = scope_roots(&scope);
    let repos = match resolve_branch_targets(&registry, &req, &roots) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err),
    };
    let base_branch = req
        .base_branch
        .clone()
        .unwrap_or_else(|| "main".to_string());

    let response = match action.as_str() {
        "status" => {
            let statuses = branch::branch_status(&repos);
            json!({
                "ok": true,
                "action": action,
                "dry_run": true,
                "repo_count": repos.len(),
                "statuses": statuses
            })
        }
        "create" => {
            let branch_name = req.branch.clone().unwrap_or_default();
            if branch_name.trim().is_empty() {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "branch is required for create action",
                );
            }
            let outcomes = branch::create_branch(&repos, &branch_name, &base_branch, dry_run);
            let failures = outcomes.iter().filter(|x| !x.success).count();
            let mut payload = json!({
                "ok": failures == 0,
                "action": action,
                "branch": branch_name,
                "base_branch": base_branch,
                "dry_run": dry_run,
                "repo_count": repos.len(),
                "failures": failures,
                "outcomes": outcomes
            });
            if let Some(record) = issued_preview_token.as_ref() {
                payload["preview_token"] = json!(record.token);
                payload["preview_expires_at_unix"] = json!(record.expires_at_unix);
                payload["preview_created_at_unix"] = json!(record.created_at_unix);
                payload["expected_execute_payload"] = record.expected_execute_payload.clone();
            }
            payload
        }
        "sync" => {
            let branch_name = req.branch.clone().unwrap_or_default();
            if branch_name.trim().is_empty() {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "branch is required for sync action",
                );
            }
            let outcomes = branch::sync_branch(&repos, &branch_name, &base_branch, dry_run);
            let failures = outcomes.iter().filter(|x| !x.success).count();
            let mut payload = json!({
                "ok": failures == 0,
                "action": action,
                "branch": branch_name,
                "base_branch": base_branch,
                "dry_run": dry_run,
                "repo_count": repos.len(),
                "failures": failures,
                "outcomes": outcomes
            });
            if let Some(record) = issued_preview_token.as_ref() {
                payload["preview_token"] = json!(record.token);
                payload["preview_expires_at_unix"] = json!(record.expires_at_unix);
                payload["preview_created_at_unix"] = json!(record.created_at_unix);
                payload["expected_execute_payload"] = record.expected_execute_payload.clone();
            }
            payload
        }
        "prune" => match branch::prune_branches(&repos, &base_branch, dry_run) {
            Ok(outcomes) => {
                let failures = outcomes.iter().filter(|x| !x.success).count();
                let mut payload = json!({
                    "ok": failures == 0,
                    "action": action,
                    "base_branch": base_branch,
                    "dry_run": dry_run,
                    "repo_count": repos.len(),
                    "failures": failures,
                    "outcomes": outcomes
                });
                if let Some(record) = issued_preview_token.as_ref() {
                    payload["preview_token"] = json!(record.token);
                    payload["preview_expires_at_unix"] = json!(record.expires_at_unix);
                    payload["preview_created_at_unix"] = json!(record.created_at_unix);
                    payload["expected_execute_payload"] = record.expected_execute_payload.clone();
                }
                payload
            }
            Err(err) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    &format!("Failed to prune branches: {err}"),
                )
            }
        },
        _ => unreachable!(),
    };

    let _ = state.events.send(json!({
        "source": "branch_control",
        "action": action,
        "dry_run": dry_run,
        "repo_count": repos.len(),
        "ok": response.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
        "response": response
    }));

    // P4: Emit timeline event
    let branch_name = req.branch.clone().unwrap_or_default();
    let success = response
        .get("ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let failures = response
        .get("failures")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let timeline_event = pilot_core::AuditEvent {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: Utc::now().to_rfc3339(),
        scope_id: Some(scope.id.to_string()),
        domain: "branch".to_string(),
        action: action.clone(),
        dry_run,
        success,
        summary: format!("Branch action {} completed", action),
        repo_count: repos.len(),
        failures,
        artifact_path: None,
        repos: repos.iter().map(|r| r.name.clone()).collect(),
        content_hash: None,
        prev_hash: None,
        details: json!({"response_summary": {
            "ok": success,
            "failures": failures,
            "branch": branch_name,
            "base_branch": base_branch,
            "conflict_count": 0,
        }}),
    };
    let _ = pilot_core::append_audit_event(timeline_event);

    // P4: Include confirmation metadata in preview responses
    let mut response = response;
    if dry_run {
        let (confirmation_type, confirmation_phrase) =
            required_confirmation(&policy, &action, &branch_name);
        response["confirmation_required"] = json!({
            "type": confirmation_type,
            "phrase": confirmation_phrase,
        });
    }

    let decision_result = if response
        .get("ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        "allow"
    } else {
        "deny"
    };
    let decision_json = json!({
        "action": action,
        "dry_run": dry_run,
        "scope_id": scope.id,
        "repo_count": repos.len(),
        "response": response
    });
    let ago_scope_path = scope.root_path.clone();
    let _ = gov_store
        .record_decision(
            scope.id,
            ago_scope_path,
            "branch",
            &action,
            decision_result,
            &decision_json,
        )
        .await;

    Json(response).into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// P4 API Handlers
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ConflictRadarRequest {
    branch: String,
    base_branch: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

async fn api_branch_conflict_radar(
    State(state): State<Arc<UiState>>,
    Json(req): Json<ConflictRadarRequest>,
) -> Response {
    let scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let registry = match branch_registry() {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let roots = scope_roots(&scope);
    let filter = multi::RepoFilter {
        group: req.group.clone(),
        tags: req.tags.clone().unwrap_or_default(),
    };
    let mut repos = match registry.list_repos(&filter) {
        Ok(v) => v,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &e.to_string()),
    };
    repos.retain(|repo| {
        let path = canonicalize_path_lossy(&repo.path);
        path_in_any_root(&path, &roots)
    });
    let base_branch = req.base_branch.as_deref().unwrap_or("main");
    let results = branch::conflict_radar(&repos, &req.branch, base_branch);
    let has_any_conflicts = results.iter().any(|r| r.has_conflicts);
    let conflict_count = results.iter().filter(|r| r.has_conflicts).count();

    Json(json!({
        "ok": true,
        "branch": req.branch,
        "base_branch": base_branch,
        "has_conflicts": has_any_conflicts,
        "conflict_count": conflict_count,
        "repo_count": repos.len(),
        "results": results
    }))
    .into_response()
}

#[derive(Deserialize)]
struct UndoJournalQuery {
    limit: Option<usize>,
}

async fn api_branch_undo_journal(
    State(state): State<Arc<UiState>>,
    Query(q): Query<UndoJournalQuery>,
) -> Response {
    let scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let limit = q.limit.unwrap_or(50).min(500);
    let entries = branch::list_undo_journal(Some(&scope.id.to_string()), limit);

    Json(json!({
        "ok": true,
        "scope_id": scope.id,
        "count": entries.len(),
        "entries": entries
    }))
    .into_response()
}

#[derive(Deserialize)]
struct UndoRequest {
    entry_id: String,
    #[serde(default)]
    dry_run: Option<bool>,
}

async fn api_branch_undo(
    State(state): State<Arc<UiState>>,
    Json(req): Json<UndoRequest>,
) -> Response {
    let scope = match require_active_scope(&state).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    if !state.allow_mutations {
        return error_response(
            StatusCode::FORBIDDEN,
            "undo blocked in read-only UI mode; restart pilot serve with --ui-allow-mutations",
        );
    }
    let dry_run = req.dry_run.unwrap_or(true);
    let entries = branch::list_undo_journal(Some(&scope.id.to_string()), 500);
    let entry = match entries.iter().find(|e| e.id == req.entry_id) {
        Some(e) => e,
        None => {
            return error_response(
                StatusCode::NOT_FOUND,
                "Undo entry not found in current scope",
            )
        }
    };
    if entry.undone {
        return error_response(
            StatusCode::CONFLICT,
            "This operation has already been undone",
        );
    }

    let outcome = branch::execute_undo(entry, dry_run);

    if outcome.success && !dry_run {
        let _ = branch::mark_undone(&entry.id);

        // Emit timeline event for the undo
        let timeline_event = pilot_core::AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            scope_id: Some(scope.id.to_string()),
            domain: "branch".to_string(),
            action: "undo".to_string(),
            dry_run: false,
            success: outcome.success,
            summary: format!("Branch undo completed for {}", entry.branch_name),
            repo_count: 1,
            failures: if outcome.success { 0 } else { 1 },
            artifact_path: None,
            repos: vec![entry.repo.clone()],
            content_hash: None,
            prev_hash: None,
            details: json!({
                "undone_action": entry.action,
                "prior_ref": entry.prior_ref,
                "undo_entry_ids": vec![entry.id.clone()],
            }),
        };
        let _ = pilot_core::append_audit_event(timeline_event);
    }

    Json(json!({
        "ok": outcome.success,
        "dry_run": dry_run,
        "entry_id": entry.id,
        "outcome": outcome
    }))
    .into_response()
}

#[derive(Deserialize)]
struct TimelineQuery {
    limit: Option<usize>,
    offset: Option<usize>,
    domain: Option<String>,
    action: Option<String>,
}

/// P4: Branch-scoped timeline — returns audit events for domain="branch" in the
/// active AGOrg scope. Bounded offset and limit prevent unbounded reads (G-007).
/// The `details` field is included in full for drill-down; UI must truncate display.
async fn api_branch_timeline(
    State(state): State<Arc<UiState>>,
    Query(q): Query<TimelineQuery>,
) -> Response {
    let scope_id = match state.agorg_store.get_active_agorg().await {
        Ok(Some(active)) => active.id.to_string(),
        _ => String::new(),
    };
    // Bounds: offset must not be negative (query-param is usize, so inherently >= 0);
    // cap limit at 500 to prevent oversized payloads (G-007 guard).
    let limit = q.limit.unwrap_or(50).min(500);
    let offset = q.offset.unwrap_or(0).min(10_000);
    let scope_opt: Option<&str> = if scope_id.is_empty() {
        None
    } else {
        Some(&scope_id)
    };

    // Always filter to domain="branch"; action filter is optional.
    let events = pilot_core::query_audit_events(
        scope_opt,
        Some("branch"),
        q.action.as_deref(),
        limit,
        offset,
    );

    Json(json!({
        "ok": true,
        "scope_id": scope_id,
        "domain": "branch",
        "count": events.len(),
        "limit": limit,
        "offset": offset,
        "events": events
    }))
    .into_response()
}

async fn api_orchestrate_timeline(
    State(state): State<Arc<UiState>>,
    Query(q): Query<TimelineQuery>,
) -> Response {
    let scope_id = match state.agorg_store.get_active_agorg().await {
        Ok(Some(active)) => active.id.to_string(),
        _ => "".to_string(),
    };
    let limit = q.limit.unwrap_or(50).min(500);
    let offset = q.offset.unwrap_or(0);
    let scope_opt = if scope_id.is_empty() {
        None
    } else {
        Some(scope_id.as_str())
    };

    let events = pilot_core::query_audit_events(
        scope_opt,
        q.domain.as_deref(),
        q.action.as_deref(),
        limit,
        offset,
    );

    Json(json!({
        "ok": true,
        "scope_id": scope_id,
        "count": events.len(),
        "limit": limit,
        "offset": offset,
        "events": events
    }))
    .into_response()
}

async fn get_history() -> Response {
    match read_recent_audit_events(200) {
        Ok(items) => Json(json!({"ok": true, "events": items})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn get_reports(Query(q): Query<ReportsQuery>) -> Response {
    let limit = q.limit.unwrap_or(200).min(2000);
    match list_report_files(limit) {
        Ok(items) => Json(json!({"ok": true, "reports": items})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn get_report_content(Query(q): Query<ReportPathQuery>) -> Response {
    let max_bytes = q
        .max_bytes
        .unwrap_or(512 * 1024)
        .clamp(1024, 2 * 1024 * 1024);
    match read_report_file(&q.path, max_bytes) {
        Ok(content) => {
            Json(json!({"ok": true, "path": q.path, "content": content})).into_response()
        }
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn get_codex_contracts(
    State(state): State<Arc<UiState>>,
    Query(q): Query<CodexContractsQuery>,
) -> Response {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let status_filter = q.status.as_deref().map(str::trim).unwrap_or("");
    let status_filter = if status_filter.is_empty() {
        None
    } else {
        Some(status_filter.to_ascii_lowercase())
    };

    let contracts = state.codex_contracts.lock().await;
    let mut items: Vec<CodexContractRecord> = contracts
        .values()
        .filter(|c| {
            if let Some(s) = status_filter.as_ref() {
                c.status.eq_ignore_ascii_case(s)
            } else {
                true
            }
        })
        .cloned()
        .collect();
    items.sort_by(|a, b| b.updated_at_unix.cmp(&a.updated_at_unix));
    items.truncate(limit);
    Json(json!({"ok": true, "contracts": items})).into_response()
}

async fn get_codex_contract(
    State(state): State<Arc<UiState>>,
    Query(q): Query<CodexContractQuery>,
) -> Response {
    let id = q.contract_id.trim();
    if id.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "contract_id is required");
    }
    let contracts = state.codex_contracts.lock().await;
    let Some(contract) = contracts.get(id) else {
        return error_response(StatusCode::NOT_FOUND, "contract_id not found");
    };
    Json(json!({"ok": true, "contract": contract})).into_response()
}

async fn get_agorg_policy_reports(Query(q): Query<AgorgPolicyReportsQuery>) -> Response {
    let limit = q.limit.unwrap_or(40).clamp(1, 500);
    match list_agorg_policy_reports(limit) {
        Ok(items) => Json(json!({"ok": true, "reports": items})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn api_agorg_list(State(state): State<Arc<UiState>>) -> Response {
    match state.agorg_store.list_agorgs().await {
        Ok(items) => Json(json!({"ok": true, "agorgs": items})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn api_agorg_active(State(state): State<Arc<UiState>>) -> Response {
    match state.agorg_store.get_active_agorg().await {
        Ok(active) => Json(json!({"ok": true, "active": active})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn api_agorg_scope_snapshot(State(state): State<Arc<UiState>>) -> Response {
    let all = match state.agorg_store.list_agorgs().await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };
    let active = match state.agorg_store.get_active_agorg().await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };
    let recent_ids = match state.agorg_store.get_recent_scope_ids().await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };
    let all_map: HashMap<uuid::Uuid, agorg::Agorg> =
        all.iter().cloned().map(|ag| (ag.id, ag)).collect();
    let recent_scopes: Vec<agorg::Agorg> = recent_ids
        .iter()
        .filter_map(|id| all_map.get(id).cloned())
        .collect();
    let ui_session = match state
        .agorg_store
        .get_app_state_value("ui_session_state")
        .await
    {
        Ok(v) => v.unwrap_or_else(|| json!({})),
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };
    Json(json!({
        "ok": true,
        "instance_id": state.instance_id,
        "active": active,
        "agorgs": all,
        "recent_scopes": recent_scopes,
        "ui_session": ui_session
    }))
    .into_response()
}

async fn api_agorg_preferences(
    State(state): State<Arc<UiState>>,
    Query(q): Query<AgorgPreferencesQuery>,
) -> Response {
    let id = if let Some(value) = q.agorg.as_deref() {
        match resolve_agorg_ref(&state.agorg_store, value.trim()).await {
            Ok(v) => v,
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        }
    } else {
        match state.agorg_store.get_active_agorg().await {
            Ok(Some(ag)) => ag.id,
            Ok(None) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "No active AGOrg; set scope first or pass agorg",
                )
            }
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        }
    };
    let agorg_record = match state.agorg_store.get_agorg(id).await {
        Ok(Some(v)) => v,
        Ok(None) => return error_response(StatusCode::NOT_FOUND, "AGOrg not found"),
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };
    match state.agorg_store.get_agorg_settings(id).await {
        Ok(settings) => Json(json!({
            "ok": true,
            "agorg": agorg_record,
            "settings": settings.unwrap_or_else(|| json!({}))
        }))
        .into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn api_agorg_set_preferences(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgPreferencesRequest>,
) -> Response {
    let id = if let Some(value) = req.agorg.as_deref() {
        match resolve_agorg_ref(&state.agorg_store, value.trim()).await {
            Ok(v) => v,
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        }
    } else {
        match state.agorg_store.get_active_agorg().await {
            Ok(Some(ag)) => ag.id,
            Ok(None) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "No active AGOrg; set scope first or pass agorg",
                )
            }
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        }
    };
    if !req.preferences.is_object() {
        return error_response(StatusCode::BAD_REQUEST, "preferences must be a JSON object");
    }
    match state
        .agorg_store
        .update_agorg_settings(id, req.preferences, req.merge.unwrap_or(true))
        .await
    {
        Ok(settings) => {
            Json(json!({"ok": true, "agorg_id": id, "settings": settings})).into_response()
        }
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_ui_session_get(State(state): State<Arc<UiState>>) -> Response {
    match state
        .agorg_store
        .get_app_state_value("ui_session_state")
        .await
    {
        Ok(session) => Json(json!({
            "ok": true,
            "instance_id": state.instance_id,
            "session": session.unwrap_or_else(|| json!({}))
        }))
        .into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn api_ui_session_set(
    State(state): State<Arc<UiState>>,
    Json(req): Json<UiSessionUpdateRequest>,
) -> Response {
    if !req.session.is_object() {
        return error_response(StatusCode::BAD_REQUEST, "session must be a JSON object");
    }
    match state
        .agorg_store
        .set_app_state_value("ui_session_state", &req.session)
        .await
    {
        Ok(()) => Json(json!({"ok": true, "instance_id": state.instance_id})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn api_agorg_batch_create(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgBatchCreateRequest>,
) -> Response {
    if !state.allow_mutations {
        return error_response(StatusCode::FORBIDDEN, "Creation blocked in read-only mode");
    }
    match state
        .agorg_store
        .init_agorg_batch(
            Path::new(&req.destination),
            &req.name,
            &req.siblings,
            req.use_git,
        )
        .await
    {
        Ok(agorg) => Json(json!({ "ok": true, "agorg": agorg })).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn api_agorg_create(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgCreateRequest>,
) -> Response {
    let parent = match resolve_agorg_ref_optional(&state.agorg_store, req.parent.as_deref()).await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    match state
        .agorg_store
        .create_agorg(
            req.name.trim(),
            Path::new(req.root.trim()),
            req.master.as_deref().map(|s| s.trim()),
            parent,
            req.scan_depth.unwrap_or(4) as i32,
            req.default_scope.unwrap_or(false),
        )
        .await
    {
        Ok(ag) => Json(json!({"ok": true, "agorg": ag})).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_agorg_create_project(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgCreateProjectRequest>,
) -> Response {
    let parent = match resolve_agorg_ref_optional(&state.agorg_store, req.parent.as_deref()).await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    match state
        .agorg_store
        .create_project(
            req.name.trim(),
            Path::new(req.root.trim()),
            req.master.as_deref().map(|s| s.trim()),
            parent,
            req.scan_depth.unwrap_or(4),
            req.autoscan.unwrap_or(false),
            req.default_scope.unwrap_or(false),
        )
        .await
    {
        Ok((agorg, scan)) => {
            let mut import_summary: Option<agorg::ImportDiscoverySummary> = None;
            if req.import.unwrap_or(false) {
                if let Some(ref s) = scan {
                    let prune_missing = req.prune_missing.unwrap_or(false);
                    match state
                        .agorg_store
                        .import_discovery_with_options(agorg.id, s, prune_missing)
                        .await
                    {
                        Ok(summary) => import_summary = Some(summary),
                        Err(err) => {
                            return error_response(StatusCode::BAD_REQUEST, &err.to_string());
                        }
                    }
                }
            }
            Json(json!({"ok": true, "agorg": agorg, "discovery": scan, "import_summary": import_summary}))
                .into_response()
        }
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_agorg_use(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgUseRequest>,
) -> Response {
    let id = match resolve_agorg_ref(&state.agorg_store, req.agorg.trim()).await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    match state.agorg_store.set_active_agorg(id).await {
        Ok(_) => Json(json!({"ok": true, "active_agorg_id": id.to_string()})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn api_agorg_discover(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgDiscoverRequest>,
) -> Response {
    let depth = req.depth.unwrap_or(4);
    let scan = match agorg::discover_hierarchy(Path::new(req.root.trim()), depth) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Some(target) = req.import_to.as_deref() {
        let id = match resolve_agorg_ref(&state.agorg_store, target.trim()).await {
            Ok(v) => v,
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        };
        let prune_missing = req.prune_missing.unwrap_or(false);
        match state
            .agorg_store
            .import_discovery_with_options(id, &scan, prune_missing)
            .await
        {
            Ok(summary) => {
                return Json(json!({
                    "ok": true,
                    "discovery": scan,
                    "import_summary": summary
                }))
                .into_response();
            }
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        };
    }
    Json(json!({"ok": true, "discovery": scan})).into_response()
}

async fn api_agorg_import_selected(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgImportSelectedRequest>,
) -> Response {
    let id = match resolve_agorg_ref(&state.agorg_store, req.agorg.trim()).await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let discovery = agorg::DiscoverResult {
        root: req.root.trim().to_string(),
        depth: req.depth.unwrap_or(4),
        candidates: req.candidates,
    };
    let prune_missing = req.prune_missing.unwrap_or(false);
    match state
        .agorg_store
        .import_discovery_with_options(id, &discovery, prune_missing)
        .await
    {
        Ok(summary) => {
            if let Some(scope_path) = req.default_scope_path {
                let path_obj = std::path::Path::new(&scope_path);
                if let Ok(Some(ago)) = state.agorg_store.get_ago_by_path(id, path_obj).await {
                    let _ = state.agorg_store.set_active_agorg(ago.id).await;
                } else {
                    let _ = state.agorg_store.set_active_agorg(id).await;
                }
            } else if discovery.candidates.is_empty() {
                let _ = state.agorg_store.set_active_agorg(id).await;
            }
            // Create / update AGOrg pyproject.toml with children list
            if let Some(ref _agorg_name) = req.agorg_name {
                if let Ok(agorgs) = state.agorg_store.list_agorgs().await {
                    if let Some(agorg_rec) = agorgs.iter().find(|a| a.id == id) {
                        let agorg_dir = std::path::Path::new(&agorg_rec.root_path);
                        let children_names: Vec<String> = discovery
                            .candidates
                            .iter()
                            .filter(|c| c.kind == "ago" || c.kind == "folder")
                            .map(|c| c.name.clone())
                            .collect();
                        agorg::ensure_pyproject_relationships(
                            agorg_dir,
                            None, // parent = [] for AGOrg
                            &children_names,
                        );
                    }
                }
            }

            Json(json!({"ok": true, "agorg_id": id, "import_summary": summary})).into_response()
        }
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_agorg_reconcile(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgReconcileRequest>,
) -> Response {
    let id = match resolve_target_agorg_id(&state, req.agorg.as_deref()).await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err),
    };
    match state.agorg_store.reconcile_agorg(id).await {
        Ok(report) => Json(json!({"ok": true, "report": report})).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_agorg_policy_report(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgPolicyReportRequest>,
) -> Response {
    match agorg_policy_report_core(&state, req.agorg.as_deref()).await {
        Ok(v) => Json(v).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err),
    }
}

async fn api_agorg_dashboard_overview(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgDashboardOverviewRequest>,
) -> Response {
    let id = match resolve_target_agorg_id(&state, req.agorg.as_deref()).await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err),
    };
    let report = match state.agorg_store.reconcile_agorg(id).await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let score = agorg_conformance_score(
        report.total_agos,
        report.issue_count,
        report.off_policy_count,
    );
    Json(json!({
        "ok": true,
        "agorg_id": report.agorg_id,
        "agorg_name": report.agorg_name,
        "score": score,
        "unresolved_issues": report.issue_count,
        "off_policy": report.off_policy_count,
        "class_counts": report.class_counts,
        "report": report
    }))
    .into_response()
}

fn agorg_conformance_score(
    total_agos: usize,
    issue_count: usize,
    off_policy_count: usize,
) -> usize {
    let max_penalty = (total_agos as i64 * 20).max(1);
    let penalty = (issue_count as i64 * 5) + (off_policy_count as i64 * 10);
    ((max_penalty - penalty).max(0) * 100 / max_penalty) as usize
}

fn agorg_supported_apply_classes() -> &'static [&'static str] {
    &["topology"]
}

fn agorg_class_action_map(issue_class: Option<&str>) -> Value {
    let selected = issue_class.unwrap_or("all");
    let supported = agorg_supported_apply_classes();
    let auto_fixable = issue_class
        .map(|c| supported.iter().any(|v| *v == c))
        .unwrap_or(false);
    json!({
        "selected_issue_class": selected,
        "auto_fixable": auto_fixable,
        "dry_run_required_before_apply": true,
        "supported_apply_classes": supported,
        "policy_branch": "manual_review_required",
        "policy_dependency": "manual_review_required",
        "metadata": "manual_review_required",
        "topology": "auto_prune_supported"
    })
}

fn agorg_reconcile_dry_run_token(
    report: &agorg::AgorgReconcileReport,
    issue_class: Option<&str>,
    planned_paths: &[String],
) -> String {
    let class_name = issue_class.unwrap_or("all");
    format!(
        "{}:{}:{}:{}:{}",
        report.agorg_id,
        class_name,
        report.issue_count,
        report.off_policy_count,
        planned_paths.join("|")
    )
}

fn filter_prune_paths_by_class(
    report: &agorg::AgorgReconcileReport,
    issue_class: Option<&str>,
) -> Vec<String> {
    let Some(issue_class) = issue_class else {
        return report.prune_candidate_paths.clone();
    };
    let mut from_issues: HashSet<String> = report
        .issues
        .iter()
        .filter(|i| i.issue_class == issue_class)
        .map(|i| i.repo_path.clone())
        .collect();
    let mut paths: Vec<String> = report
        .prune_candidate_paths
        .iter()
        .filter(|p| from_issues.remove(*p))
        .cloned()
        .collect();
    paths.sort();
    paths
}

async fn agorg_policy_report_core(
    state: &Arc<UiState>,
    agorg: Option<&str>,
) -> std::result::Result<Value, String> {
    let id = resolve_target_agorg_id(state, agorg).await?;
    let report = state
        .agorg_store
        .reconcile_agorg(id)
        .await
        .map_err(|e| e.to_string())?;
    let path = persist_agorg_policy_report(&report).map_err(|e| e.to_string())?;
    let _ = state.events.send(json!({
        "source": "agorg_policy_report",
        "agorg_id": report.agorg_id.to_string(),
        "artifact_path": path,
        "issue_count": report.issue_count,
        "off_policy_count": report.off_policy_count
    }));
    Ok(agorg_policy_report_response(&report, &path))
}

async fn resolve_target_agorg_id(
    state: &Arc<UiState>,
    agorg: Option<&str>,
) -> std::result::Result<uuid::Uuid, String> {
    if let Some(value) = agorg {
        resolve_agorg_ref(&state.agorg_store, value.trim())
            .await
            .map_err(|e| e.to_string())
    } else {
        state
            .agorg_store
            .get_active_agorg()
            .await
            .map_err(|e| e.to_string())?
            .map(|ag| ag.id)
            .ok_or_else(|| "No active AGOrg; set scope first or pass agorg".to_string())
    }
}

async fn agorg_reconcile_apply_core(
    state: &Arc<UiState>,
    req: AgorgReconcileApplyRequest,
    enforce_mutation_guard: bool,
) -> std::result::Result<Value, String> {
    if enforce_mutation_guard && !state.allow_mutations && !req.dry_run.unwrap_or(true) {
        return Err("reconcile apply blocked in read-only UI mode".to_string());
    }
    let id = resolve_target_agorg_id(state, req.agorg.as_deref()).await?;
    let dry_run = req.dry_run.unwrap_or(true);
    let issue_class = req
        .issue_class
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let report = state
        .agorg_store
        .reconcile_agorg(id)
        .await
        .map_err(|e| e.to_string())?;

    if dry_run {
        let planned_paths = filter_prune_paths_by_class(&report, issue_class.as_deref());
        let dry_run_token =
            agorg_reconcile_dry_run_token(&report, issue_class.as_deref(), &planned_paths);
        let out = json!({
            "ok": true,
            "dry_run": true,
            "issue_class": issue_class,
            "dry_run_token": dry_run_token,
            "planned_prune_count": planned_paths.len(),
            "planned_prune_paths": planned_paths,
            "selected_action_mapping": agorg_class_action_map(issue_class.as_deref()),
            "report": report
        });
        let artifact_path =
            persist_agorg_reconcile_action_report("dryrun", &out).map_err(|e| e.to_string())?;
        let _ = state.events.send(json!({
            "source": "agorg_reconcile_apply",
            "agorg_id": report.agorg_id.to_string(),
            "dry_run": true,
            "issue_class": issue_class,
            "artifact_path": artifact_path
        }));
        let mut out_with_artifact = out;
        if let Some(map) = out_with_artifact.as_object_mut() {
            map.insert("artifact_path".to_string(), json!(artifact_path));
        }
        return Ok(out_with_artifact);
    }

    if let Some(class_name) = issue_class.as_deref() {
        if !agorg_supported_apply_classes().contains(&class_name) {
            return Err(format!(
                "issue_class '{}' is not currently auto-fixable (supported: topology)",
                class_name
            ));
        }
    }

    let selected_paths = filter_prune_paths_by_class(&report, issue_class.as_deref());
    let expected_token =
        agorg_reconcile_dry_run_token(&report, issue_class.as_deref(), &selected_paths);
    let provided_token = req
        .dry_run_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "apply requires dry_run_token from a matching dry-run result (dry-run-first policy)"
                .to_string()
        })?;
    if provided_token != expected_token {
        return Err(
            "dry_run_token mismatch; rerun dry-run for current AGOrg/class before apply"
                .to_string(),
        );
    }
    let pruned = state
        .agorg_store
        .prune_ago_paths(report.agorg_id, &selected_paths)
        .await
        .map_err(|e| e.to_string())?;
    let after = state
        .agorg_store
        .reconcile_agorg(report.agorg_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut out = agorg_reconcile_apply_success_response(pruned, &report, &after);
    let artifact_path =
        persist_agorg_reconcile_action_report("apply", &out).map_err(|e| e.to_string())?;
    let _ = state.events.send(json!({
        "source": "agorg_reconcile_apply",
        "agorg_id": report.agorg_id.to_string(),
        "dry_run": false,
        "issue_class": issue_class,
        "pruned": pruned,
        "remaining_off_policy": after.off_policy_count,
        "artifact_path": artifact_path
    }));
    if let Some(map) = out.as_object_mut() {
        map.insert("artifact_path".to_string(), json!(artifact_path));
        map.insert(
            "selected_action_mapping".to_string(),
            agorg_class_action_map(issue_class.as_deref()),
        );
        map.insert("issue_class".to_string(), json!(issue_class));
        map.insert("dry_run_token".to_string(), json!(expected_token));
    }
    Ok(out)
}

async fn api_agorg_reconcile_apply(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgReconcileApplyRequest>,
) -> Response {
    match agorg_reconcile_apply_core(&state, req, true).await {
        Ok(v) => Json(v).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err),
    }
}

fn agorg_policy_report_response(
    report: &agorg::AgorgReconcileReport,
    artifact_path: &str,
) -> Value {
    json!({
        "ok": true,
        "report": report,
        "artifact_path": artifact_path
    })
}

fn agorg_reconcile_apply_dry_run_response(report: &agorg::AgorgReconcileReport) -> Value {
    json!({
        "ok": true,
        "dry_run": true,
        "planned_prune_count": report.prune_candidate_paths.len(),
        "planned_prune_paths": report.prune_candidate_paths,
        "governance_issues": report.governance_issues,
        "conflict_traces": report.conflict_traces,
        "fleet_report": report.fleet_report,
        "report": report
    })
}

fn agorg_reconcile_apply_success_response(
    pruned: usize,
    before: &agorg::AgorgReconcileReport,
    after: &agorg::AgorgReconcileReport,
) -> Value {
    json!({
        "ok": true,
        "dry_run": false,
        "pruned": pruned,
        "governance_issues": after.governance_issues,
        "conflict_traces": after.conflict_traces,
        "fleet_report": after.fleet_report,
        "before": before,
        "after": after
    })
}

async fn api_agorg_tree(
    State(state): State<Arc<UiState>>,
    Query(q): Query<AgorgTreeQuery>,
) -> Response {
    let root_id = match q.root.as_deref() {
        Some(value) => match resolve_agorg_ref(&state.agorg_store, value.trim()).await {
            Ok(v) => Some(v),
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        },
        None => None,
    };
    match state.agorg_store.tree(root_id).await {
        Ok(tree) => Json(json!({"ok": true, "tree": tree})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn api_agorg_link(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgLinkRequest>,
) -> Response {
    let parent = match resolve_agorg_ref(&state.agorg_store, req.parent.trim()).await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let child = match resolve_agorg_ref(&state.agorg_store, req.child.trim()).await {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    match state.agorg_store.link_agorgs(parent, child).await {
        Ok(_) => {
            Json(json!({"ok": true, "parent": parent.to_string(), "child": child.to_string()}))
                .into_response()
        }
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_agorg_update(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgUpdateRequest>,
) -> Response {
    match state
        .agorg_store
        .update_agorg(
            req.id,
            req.name,
            req.root.map(PathBuf::from),
            req.master,
            req.scan_depth.map(|d| d as i32),
            req.default_scope,
        )
        .await
    {
        Ok(ag) => Json(json!({"ok": true, "agorg": ag})).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_agorg_delete(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgDeleteRequest>,
) -> Response {
    match state.agorg_store.delete_agorg(req.id).await {
        Ok(count) => Json(json!({"ok": true, "deleted_count": count})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

// -----------------------------------------------------------------------------
// OVERRIDE REGISTRY HANDLERS
// -----------------------------------------------------------------------------

async fn api_settings_get_overrides(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(kind): axum::extract::Path<String>,
) -> Response {
    let agorg_id = match state.agorg_store.get_active_agorg().await {
        Ok(Some(agorg)) => agorg.id,
        Ok(None) => return error_response(StatusCode::BAD_REQUEST, "No active AGOrg"),
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    match gov_store.list_overrides(agorg_id, &kind).await {
        Ok(overrides) => Json(json!({"ok": true, "overrides": overrides})).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

#[derive(serde::Deserialize)]
struct CreateOverrideReq {
    ago_path: String,
    reason: String,
    ticket_ref: Option<String>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    policy_json: serde_json::Value,
}

async fn api_settings_create_override(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(kind): axum::extract::Path<String>,
    Json(req): Json<CreateOverrideReq>,
) -> Response {
    if !state.allow_mutations {
        return error_response(StatusCode::FORBIDDEN, "mutations disabled");
    }
    let agorg_id = match state.agorg_store.get_active_agorg().await {
        Ok(Some(agorg)) => agorg.id,
        Ok(None) => return error_response(StatusCode::BAD_REQUEST, "No active AGOrg"),
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
    };

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    match gov_store
        .create_override_with_reason(
            agorg_id,
            &req.ago_path,
            &kind,
            &req.policy_json,
            "active",
            "pilot_ui",
            &req.reason,
            req.ticket_ref.as_deref(),
            req.expires_at,
        )
        .await
    {
        Ok(_) => Json(json!({"ok": true})).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn api_settings_delete_override(
    State(state): State<Arc<UiState>>,
    axum::extract::Path((kind, ago_encoded)): axum::extract::Path<(String, String)>,
) -> Response {
    if !state.allow_mutations {
        return error_response(StatusCode::FORBIDDEN, "mutations disabled");
    }
    let agorg_id = match state.agorg_store.get_active_agorg().await {
        Ok(Some(agorg)) => agorg.id,
        Ok(None) => return error_response(StatusCode::BAD_REQUEST, "No active AGOrg"),
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
    };

    let ago_path = ago_encoded
        .replace("%2F", "/")
        .replace("%2f", "/")
        .replace("%20", " ")
        .replace("%25", "%");

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    match gov_store.revoke_override(agorg_id, &ago_path, &kind).await {
        Ok(_) => Json(json!({"ok": true})).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

#[derive(serde::Deserialize)]
struct ResolveTraceReq {
    ago_path: String,
    policy_kind: String,
}

async fn api_settings_resolve_trace(
    State(state): State<Arc<UiState>>,
    Json(req): Json<ResolveTraceReq>,
) -> Response {
    let agorg_id = match state.agorg_store.get_active_agorg().await {
        Ok(Some(agorg)) => agorg.id,
        Ok(None) => return error_response(StatusCode::BAD_REQUEST, "No active AGOrg"),
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
    };

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    match gov_store
        .resolve_with_trace(agorg_id, &req.ago_path, &req.policy_kind)
        .await
    {
        Ok(trace) => Json(json!({"ok": true, "trace": trace})).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn api_settings_governance_scan(State(state): State<Arc<UiState>>) -> Response {
    let agorg_id = match state.agorg_store.get_active_agorg().await {
        Ok(Some(agorg)) => agorg.id,
        Ok(None) => return error_response(StatusCode::BAD_REQUEST, "No active AGOrg"),
        Err(_) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
    };

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    match crate::governance::eval::fleet_governance_scan(&gov_store, &state.agorg_store, agorg_id)
        .await
    {
        Ok(report) => Json(json!({"ok": true, "report": report})).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn api_agorg_reset(State(state): State<Arc<UiState>>) -> Response {
    if !state.allow_mutations {
        return error_response(StatusCode::FORBIDDEN, "reset blocked in read-only UI mode");
    }
    match state.agorg_store.reset_all().await {
        Ok((agorgs, agos)) => Json(json!({
            "ok": true,
            "deleted_agorgs": agorgs,
            "deleted_agos": agos
        }))
        .into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn api_agorg_scan_master(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AgorgScanMasterRequest>,
) -> Response {
    match agorg::scan_master_directory(Path::new(req.path.trim())) {
        Ok(mut candidates) => {
            // Enrich with "registered" status from DB
            if let Ok(registered) = state.agorg_store.list_agorgs().await {
                let paths: HashSet<_> = registered.iter().map(|a| a.root_path.clone()).collect();
                for c in &mut candidates {
                    c.is_registered = paths.contains(&c.path);
                }
            }
            Json(json!({"ok": true, "items": candidates})).into_response()
        }
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_agorg_upgrade_ago(
    State(_state): State<Arc<UiState>>,
    Json(req): Json<AgorgUpgradeRequest>,
) -> Response {
    match agorg::upgrade_ago(Path::new(req.path.trim()), &req.name) {
        Ok(_) => Json(json!({"ok": true})).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_agorg_edit_relationship(
    State(_state): State<Arc<UiState>>,
    Json(req): Json<AgorgEditRelationshipRequest>,
) -> Response {
    match agorg::edit_relationship(Path::new(req.path.trim()), req.parent, req.children) {
        Ok(_) => Json(json!({"ok": true})).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_fs_pick_directory(Json(req): Json<FsPickDirectoryRequest>) -> Response {
    match pick_directory(req.start_dir.as_deref()).await {
        Ok(path) => Json(json!({"ok": true, "path": path})).into_response(),
        Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    }
}

async fn api_fs_create_dir(Json(req): Json<CreateDirRequest>) -> Response {
    let path = req.path.trim();
    if path.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Path is required");
    }
    match tokio::fs::create_dir_all(path).await {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("Failed to create directory: {}", e),
        ),
    }
}

async fn pick_directory(start_dir: Option<&str>) -> Result<String> {
    let seed = start_dir.unwrap_or("/home");
    let mut zenity = TokioCommand::new("zenity");
    zenity
        .arg("--file-selection")
        .arg("--directory")
        .arg("--title=Select AGOrg Folder")
        .arg("--filename")
        .arg(seed);
    match zenity.output().await {
        Ok(output) => {
            if output.status.success() {
                let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if selected.is_empty() {
                    return Err(miette::miette!("No folder selected"));
                }
                return Ok(selected);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(miette::miette!("Failed to run zenity: {}", e)),
    }

    let mut kdialog = TokioCommand::new("kdialog");
    kdialog
        .arg("--getexistingdirectory")
        .arg(seed)
        .arg("Select AGOrg Folder");
    match kdialog.output().await {
        Ok(output) => {
            if output.status.success() {
                let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if selected.is_empty() {
                    return Err(miette::miette!("No folder selected"));
                }
                return Ok(selected);
            }
            Err(miette::miette!(
                "Folder selection canceled or failed (kdialog exit {})",
                output.status.code().unwrap_or(1)
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(miette::miette!(
            "No folder picker available. Install `zenity` or `kdialog`."
        )),
        Err(e) => Err(miette::miette!("Failed to run kdialog: {}", e)),
    }
}

async fn resolve_agorg_ref(store: &AgorgStore, input: &str) -> Result<uuid::Uuid> {
    if let Ok(id) = uuid::Uuid::parse_str(input) {
        if store.get_agorg(id).await?.is_some() {
            return Ok(id);
        }
        return Err(miette::miette!("AGOrg UUID {} not found", id));
    }
    let canonical_input = fs::canonicalize(input)
        .ok()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| input.to_string());
    let list = store.list_agorgs().await?;
    let mut found = list
        .into_iter()
        .filter(|a| {
            a.name.eq_ignore_ascii_case(input)
                || a.root_path == input
                || a.root_path == canonical_input
        })
        .collect::<Vec<_>>();
    if found.is_empty() {
        return Err(miette::miette!(
            "AGOrg '{}' not found (expected UUID, name, or root path)",
            input
        ));
    }
    if found.len() > 1 {
        return Err(miette::miette!(
            "AGOrg name '{}' is ambiguous; use UUID instead",
            input
        ));
    }
    Ok(found.remove(0).id)
}

async fn resolve_agorg_ref_optional(
    store: &AgorgStore,
    input: Option<&str>,
) -> Result<Option<uuid::Uuid>> {
    if let Some(v) = input {
        Ok(Some(resolve_agorg_ref(store, v).await?))
    } else {
        Ok(None)
    }
}

pub fn bus_shim_running(stdout: &str, stderr: &str) -> bool {
    let combined = format!("{stdout}\n{stderr}");
    combined.contains("RUNNING")
}

fn git_current_branch(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
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

fn git_repo_clean(cwd: &Path) -> Option<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .arg("status")
        .arg("--porcelain")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(output.stdout.is_empty())
}

async fn evaluate_operator_routine_guard(
    state: &Arc<UiState>,
    active_scope: &agorg::Agorg,
    action: &str,
    req: &DependencyActionRequest,
) -> std::result::Result<PolicyEvalReport, String> {
    let cwd = std::env::current_dir()
        .map_err(|e| format!("Cannot resolve current working directory: {e}"))?;
    let cwd_canon = canonicalize_path_lossy(&cwd);
    let roots = scope_roots(active_scope);

    let registry =
        branch_registry().map_err(|_| "Failed to open repository registry".to_string())?;
    let mut repos = registry
        .list_repos(&multi::RepoFilter::default())
        .map_err(|e| format!("Failed to list repos: {e}"))?;
    repos.retain(|repo| {
        let path = canonicalize_path_lossy(&repo.path);
        path_in_any_root(&path, &roots)
    });
    let repo_registered = repos
        .iter()
        .any(|r| canonicalize_path_lossy(&r.path) == cwd_canon);

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    let cwd_for_lookup = cwd_canon.display().to_string();
    let effective = gov_store
        .get_effective_policy_record(active_scope.id, cwd_for_lookup.as_str(), "operator_routine")
        .await
        .ok()
        .flatten()
        .map(|(p, _)| p.policy_json)
        .unwrap_or_else(|| {
            serde_json::to_value(OperatorRoutinePolicy::default()).unwrap_or(json!({}))
        });
    let policy: OperatorRoutinePolicy =
        serde_json::from_value(effective).unwrap_or_else(|_| OperatorRoutinePolicy::default());
    let exceptions = gov_store
        .get_effective_exceptions(active_scope.id, "operator_routine")
        .await
        .map_err(|e| e.to_string())?;

    let routine_context = OperatorRoutineContext {
        action: action.to_string(),
        has_active_scope: true,
        repo_registered,
        current_branch: git_current_branch(&cwd_canon),
        repo_clean: git_repo_clean(&cwd_canon),
        completed_steps: req.preflight_steps.clone().unwrap_or_default(),
    };
    Ok(evaluate_operator_routine_policy(
        &policy,
        &routine_context,
        &exceptions,
        cwd_for_lookup.as_str(),
        "UI",
        Some(active_scope.id),
    ))
}

async fn run_dependency_action(
    State(state): State<Arc<UiState>>,
    Json(req): Json<DependencyActionRequest>,
) -> Response {
    let action = req.action.trim();
    let mut active_scope_selected: Option<agorg::Agorg> = None;
    if action.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "action is required");
    }
    if matches!(
        action,
        "repair"
            | "cargo-fmt"
            | "ci-trigger"
            | "db-start"
            | "db-stop"
            | "db-restart"
            | "bus-start"
            | "bus-stop"
            | "bus-restart"
            | "services-start"
            | "services-stop"
            | "services-restart"
    ) && !state.allow_mutations
    {
        return error_response(StatusCode::FORBIDDEN, "action blocked in read-only UI mode");
    }
    if dependency_action_scope_required(action) {
        let active_scope = match state.agorg_store.get_active_agorg().await {
            Ok(v) => v,
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        };
        let Some(scope) = active_scope else {
            return error_response(
                StatusCode::PRECONDITION_FAILED,
                "No active AGOrg scope selected. Set AGOrg scope before running this action.",
            );
        };
        active_scope_selected = Some(scope.clone());
        if dependency_action_requires_cwd_scope(action) {
            let cwd = match std::env::current_dir() {
                Ok(v) => canonicalize_path_lossy(&v),
                Err(err) => {
                    return error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("Cannot resolve current working directory: {err}"),
                    )
                }
            };
            let scope_root = canonicalize_path_lossy(Path::new(&scope.root_path));
            if !path_is_within(&cwd, &scope_root) {
                return error_response(
                    StatusCode::FORBIDDEN,
                    &format!(
                        "Current repo path '{}' is outside active AGOrg scope '{}'",
                        cwd.display(),
                        scope_root.display()
                    ),
                );
            }
        }
    }

    if matches!(action, "prepush-gate" | "push") {
        let Some(scope) = active_scope_selected.as_ref() else {
            return error_response(
                StatusCode::PRECONDITION_FAILED,
                "No active AGOrg scope selected. Set AGOrg scope before running this action.",
            );
        };
        let guard = match evaluate_operator_routine_guard(&state, scope, action, &req).await {
            Ok(v) => v,
            Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err),
        };
        if guard.blocked {
            return Json(json!({
                "ok": false,
                "action": action,
                "exit_code": 1,
                "error": "operator_routine policy blocked action",
                "policy_report": guard
            }))
            .into_response();
        }
    }
    if action == "db-status" {
        return match state.agorg_store.managed_db_status().await {
            Ok(Some(status)) => {
                let ok = status.running;
                let body = json!({
                    "ok": ok,
                    "action": action,
                    "exit_code": 0,
                    "stdout": serde_json::to_string_pretty(&status).unwrap_or_default(),
                    "stderr": ""
                });
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": ok,
                    "exit_code": 0
                }));
                Json(body).into_response()
            }
            Ok(None) => {
                let body = json!({
                    "ok": true,
                    "action": action,
                    "exit_code": 0,
                    "stdout": "Managed DB disabled: PILOT_AGORG_DATABASE_URL override is set",
                    "stderr": ""
                });
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": true,
                    "exit_code": 0
                }));
                Json(body).into_response()
            }
            Err(err) => {
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": false,
                    "exit_code": 1,
                    "error": err.to_string()
                }));
                error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
        };
    }
    if action == "db-start" {
        return match state.agorg_store.ensure_managed_db().await {
            Ok(Some(status)) => {
                let ok = status.running;
                let body = json!({
                    "ok": ok,
                    "action": action,
                    "exit_code": 0,
                    "stdout": serde_json::to_string_pretty(&status).unwrap_or_default(),
                    "stderr": ""
                });
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": ok,
                    "exit_code": 0
                }));
                Json(body).into_response()
            }
            Ok(None) => {
                let body = json!({
                    "ok": true,
                    "action": action,
                    "exit_code": 0,
                    "stdout": "Managed DB disabled: PILOT_AGORG_DATABASE_URL override is set",
                    "stderr": ""
                });
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": true,
                    "exit_code": 0
                }));
                Json(body).into_response()
            }
            Err(err) => {
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": false,
                    "exit_code": 1,
                    "error": err.to_string()
                }));
                error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
        };
    }
    if action == "db-stop" {
        return match state.agorg_store.stop_managed_db().await {
            Ok(Some(status)) => {
                let ok = !status.running;
                let body = json!({
                    "ok": ok,
                    "action": action,
                    "exit_code": 0,
                    "stdout": serde_json::to_string_pretty(&status).unwrap_or_default(),
                    "stderr": ""
                });
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": ok,
                    "exit_code": 0
                }));
                Json(body).into_response()
            }
            Ok(None) => {
                let body = json!({
                    "ok": true,
                    "action": action,
                    "exit_code": 0,
                    "stdout": "Managed DB disabled: PILOT_AGORG_DATABASE_URL override is set",
                    "stderr": ""
                });
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": true,
                    "exit_code": 0
                }));
                Json(body).into_response()
            }
            Err(err) => {
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": false,
                    "exit_code": 1,
                    "error": err.to_string()
                }));
                error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
        };
    }

    let policy = RetryPolicy::default();

    if action == "db-restart" {
        let store = state.agorg_store.clone();
        let _ = store.stop_managed_db().await;
        return match supervised_start(
            "Managed Database (Restart)",
            || {
                let s = store.clone();
                async move { s.ensure_managed_db().await.map_err(|e| e.to_string()) }
            },
            policy.clone(),
        )
        .await
        {
            Ok(Some(status)) => {
                let ok = status.running;
                let body = json!({
                    "ok": ok,
                    "action": action,
                    "exit_code": if ok { 0 } else { 1 },
                    "stdout": serde_json::to_string_pretty(&status).unwrap_or_default(),
                    "stderr": ""
                });
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": ok,
                    "exit_code": if ok { 0 } else { 1 }
                }));
                Json(body).into_response()
            }
            Ok(None) => {
                let body = json!({
                    "ok": true,
                    "action": action,
                    "exit_code": 0,
                    "stdout": "Managed DB disabled: PILOT_AGORG_DATABASE_URL override is set",
                    "stderr": ""
                });
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": true,
                    "exit_code": 0
                }));
                Json(body).into_response()
            }
            Err(err) => {
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": false,
                    "exit_code": 1,
                    "error": err.to_string()
                }));
                error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
        };
    }
    if action == "services-status" {
        let bus = run_local_script(&bus_shim_command("status")).await;
        let db = state.agorg_store.managed_db_status().await;
        return match (bus, db) {
            (Ok((bus_code, bus_out, bus_err)), Ok(db_status_opt)) => {
                let bus_running = bus_code == 0 && bus_shim_running(&bus_out, &bus_err);
                let (db_running, db_stdout) = match db_status_opt {
                    Some(status) => (
                        status.running,
                        serde_json::to_string_pretty(&status).unwrap_or_default(),
                    ),
                    None => (
                        true,
                        "Managed DB disabled: PILOT_AGORG_DATABASE_URL override is set".to_string(),
                    ),
                };
                let ok = bus_running && db_running;
                let body = json!({
                    "ok": ok,
                    "action": action,
                    "exit_code": if ok { 0 } else { 1 },
                    "bus_running": bus_running,
                    "db_running": db_running,
                    "stdout": format!("Bus:\n{}\n{}\n\nDB:\n{}", bus_out, bus_err, db_stdout),
                    "stderr": ""
                });
                let _ = state.events.send(json!({
                    "source": "dependency_action",
                    "action": action,
                    "success": ok,
                    "exit_code": if ok { 0 } else { 1 },
                    "bus_running": bus_running,
                    "db_running": db_running
                }));
                Json(body).into_response()
            }
            (Err(err), _) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
            (_, Err(err)) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
        };
    }
    if matches!(
        action,
        "services-start" | "services-stop" | "services-restart"
    ) {
        let mut bus_running = false;
        let mut db_running = false;
        let mut db_stdout = String::new();

        let store = state.agorg_store.clone();

        let db_result = match action {
            "services-stop" => store
                .stop_managed_db()
                .await
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "services-start" => supervised_start(
                "Managed Database",
                || {
                    let s = store.clone();
                    async move { s.ensure_managed_db().await.map_err(|e| e.to_string()) }
                },
                policy.clone(),
            )
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
            _ => {
                // services-restart
                let _ = store.stop_managed_db().await;
                supervised_start(
                    "Managed Database (Restart)",
                    || {
                        let s = store.clone();
                        async move { s.ensure_managed_db().await.map_err(|e| e.to_string()) }
                    },
                    policy.clone(),
                )
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
            }
        };

        match db_result {
            Ok(_) => {
                if let Ok(Some(st)) = store.managed_db_status().await {
                    db_running = st.running;
                    db_stdout = serde_json::to_string_pretty(&st).unwrap_or_default();
                } else if let Ok(None) = store.managed_db_status().await {
                    db_running = true;
                    db_stdout =
                        "Managed DB disabled: PILOT_AGORG_DATABASE_URL override is set".to_string();
                }
            }
            Err(e) => {
                db_stdout = format!("DB Error: {}", e);
            }
        }

        let bus_cmd = match action {
            "services-stop" => bus_shim_command("stop"),
            "services-start" => bus_shim_command("start"),
            _ => bus_shim_command("restart"),
        };

        let (bus_out, bus_err) = if action == "services-stop" {
            let (code, out, err) = run_local_script(&bus_cmd).await.unwrap_or((
                1,
                "".into(),
                "Failed to run bus wrapper".into(),
            ));
            bus_running = code == 0 && bus_shim_running(&out, &err);
            (out, err)
        } else {
            let bus_res = supervised_start(
                "ArqonBus",
                || async {
                    let (code, out, err) = run_local_script(&bus_cmd)
                        .await
                        .map_err(|e| e.to_string())?;
                    if code != 0 {
                        return Err(format!("Shim exited {code}: {err}"));
                    }
                    if !bus_shim_running(&out, &err) {
                        return Err("Bus reported OK but is not running".to_string());
                    }
                    Ok((code, out, err))
                },
                policy.clone(),
            )
            .await;

            match bus_res {
                Ok((code, ref out, ref err)) => {
                    bus_running = code == 0 && bus_shim_running(out, err);
                    (out.to_string(), err.to_string())
                }
                Err(e) => (format!("Bus Error: {}", e), String::new()),
            }
        };

        let ok = match action {
            "services-stop" => !bus_running && !db_running,
            _ => bus_running && db_running,
        };
        let body = json!({
            "ok": ok,
            "action": action,
            "exit_code": if ok { 0 } else { 1 },
            "bus_running": bus_running,
            "db_running": db_running,
            "stdout": format!("Bus:\n{}\n{}\n\nDB:\n{}", bus_out, bus_err, db_stdout),
            "stderr": ""
        });
        let _ = state.events.send(json!({
            "source": "dependency_action",
            "action": action,
            "success": ok,
            "exit_code": if ok { 0 } else { 1 },
            "bus_running": bus_running,
            "db_running": db_running
        }));
        return Json(body).into_response();
    }

    let result = match (action, req.json) {
        ("preflight", _) | ("policy", _) | ("hook-policy", _) | ("drift", _) | ("gate", _) => {
            use crate::preflight::graph::run_preflight_graph;
            let steps = preflight_steps_from_action(action, req.preflight_steps.clone());
            let report = run_preflight_graph(
                Path::new("."),
                steps,
                req.branch.as_deref(),
                req.remote.as_deref(),
            )
            .await;
            match report {
                Ok(rep) => {
                    let ok = rep.is_pass();
                    let body = json!({
                        "ok": ok,
                        "action": "preflight",
                        "report": rep,
                    });
                    return Json(body).into_response();
                }
                Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
            }
        }
        ("repair", _) => run_local_script("./scripts/repair_lock_182.sh --no-gate").await,
        ("cargo-fmt", _) => run_local_script("cargo fmt").await,
        ("prepush-gate", _) => {
            run_local_script_streamed("./scripts/prepush_gate.sh", "prepush-gate", &state.events)
                .await
        }
        ("bus-start", _) => {
            let res = supervised_start(
                "ArqonBus (Start)",
                || async {
                    let (code, out, err) = run_local_script(&bus_shim_command("start"))
                        .await
                        .map_err(|e| e.to_string())?;
                    if code != 0 {
                        return Err(err);
                    }
                    if !bus_shim_running(&out, &err) {
                        return Err("Not running".into());
                    }
                    Ok((code, out, err))
                },
                policy.clone(),
            )
            .await;
            match res {
                Ok(v) => Ok(v),
                Err(e) => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )),
            }
        }
        ("bus-stop", _) => run_local_script(&bus_shim_command("stop")).await,
        ("bus-restart", _) => {
            let res = supervised_start(
                "ArqonBus (Restart)",
                || async {
                    let (code, out, err) = run_local_script(&bus_shim_command("restart"))
                        .await
                        .map_err(|e| e.to_string())?;
                    if code != 0 {
                        return Err(err);
                    }
                    if !bus_shim_running(&out, &err) {
                        return Err("Not running".into());
                    }
                    Ok((code, out, err))
                },
                policy.clone(),
            )
            .await;
            match res {
                Ok(v) => Ok(v),
                Err(e) => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )),
            }
        }
        ("bus-status", _) => run_local_script(&bus_shim_command("status")).await,
        ("push", _) => {
            let branch = req.branch.as_deref().unwrap_or("main");
            let remote = req.remote.as_deref().unwrap_or("origin");
            if !is_safe_cli_token(branch) || !is_safe_cli_token(remote) {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "branch/remote contains unsupported characters",
                );
            }
            run_local_script_streamed(
                &format!("./scripts/push_main.sh {branch} {remote}"),
                "push",
                &state.events,
            )
            .await
        }
        ("release-readiness", _) => {
            run_local_script_streamed(
                "./scripts/release_readiness_check.sh",
                "release-readiness",
                &state.events,
            )
            .await
        }
        ("release-compat-matrix", _) => {
            run_local_script_streamed(
                "./scripts/compat_matrix_smoke.sh",
                "release-compat-matrix",
                &state.events,
            )
            .await
        }
        ("release-migration-smoke", _) => {
            run_local_script_streamed(
                "./scripts/migration_smoke_test.sh",
                "release-migration-smoke",
                &state.events,
            )
            .await
        }
        ("release-collect-evidence", _) => {
            let label = req
                .label
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("release-ui");
            if !is_safe_cli_token(label) {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "label contains unsupported characters",
                );
            }
            run_local_script_streamed(
                &format!("./scripts/release_collect_evidence.sh --label {label}"),
                "release-collect-evidence",
                &state.events,
            )
            .await
        }
        ("release-verify-bundle", _) => {
            let Some(bundle_path) = req
                .bundle_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "bundle_path is required for release-verify-bundle",
                );
            };
            if !is_safe_cli_token(bundle_path) {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "bundle_path contains unsupported characters",
                );
            }
            let verify_script = format!("{}/verify_bundle.sh", bundle_path.trim_end_matches('/'));
            run_local_script_streamed(&verify_script, "release-verify-bundle", &state.events).await
        }
        ("ci-watch", _) => {
            let branch = req.branch.as_deref().unwrap_or("main");
            if !is_safe_cli_token(branch) {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "branch contains unsupported characters",
                );
            }
            let timeout = req.ci_timeout_sec.unwrap_or(1800).clamp(60, 7200);
            let cmd = format!(
                "./scripts/gh_actions_watch_latest.sh --branch {branch} --timeout-sec {timeout}"
            );
            run_local_script_streamed(&cmd, "ci-watch", &state.events).await
        }
        ("ci-trigger", _) => {
            let branch = req.branch.as_deref().unwrap_or("main");
            if !is_safe_cli_token(branch) {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "branch contains unsupported characters",
                );
            }
            let cmd = format!("./scripts/gh_actions_trigger_ci.sh --branch {branch}");
            run_local_script_streamed(&cmd, "ci-trigger", &state.events).await
        }
        ("ci-status", _) => {
            let branch = req.branch.as_deref().unwrap_or("main");
            if !is_safe_cli_token(branch) {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "branch contains unsupported characters",
                );
            }
            let cmd = format!("./scripts/gh_actions_status_latest.sh --branch {branch}");
            run_local_script_streamed(&cmd, "ci-status", &state.events).await
        }
        _ => return error_response(StatusCode::BAD_REQUEST, "unsupported action"),
    };

    match result {
        Ok((status, out, err)) => {
            let ok = status == 0;
            let mut body = json!({
                "ok": ok,
                "action": action,
                "exit_code": status,
                "stdout": out,
                "stderr": err
            });
            if action == "push" {
                if let Some(summary) = parse_push_main_summary(
                    body.get("stdout")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    body.get("stderr")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                ) {
                    body["summary"] = summary;
                }
            }
            if action == "release-collect-evidence" {
                if let Some(path) = parse_release_collect_evidence_path(
                    body.get("stdout")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    body.get("stderr")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                ) {
                    body["artifact_path"] = json!(path);
                }
            }
            if action == "ci-watch" {
                if let Some(summary) = parse_gh_watch_summary(
                    body.get("stdout")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    body.get("stderr")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                ) {
                    body["summary"] = summary;
                }
            }
            if action == "ci-status" {
                if let Some(summary) = parse_gh_status_summary(
                    body.get("stdout")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    body.get("stderr")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                ) {
                    body["summary"] = summary;
                }
            }
            if action == "prepush-gate" {
                if let Some(summary) = parse_prepush_summary(
                    body.get("stdout")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    body.get("stderr")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                ) {
                    body["summary"] = summary;
                }
            }
            let _ = state.events.send(json!({
                "source": "dependency_action",
                "action": action,
                "success": ok,
                "exit_code": status,
                "branch": req.branch,
                "remote": req.remote
            }));
            Json(body).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn export_evidence_bundle(
    State(state): State<Arc<UiState>>,
    Json(req): Json<EvidenceExportRequest>,
) -> Response {
    let history_limit = req.history_limit.unwrap_or(400).clamp(50, 5000);
    let reports_limit = req.reports_limit.unwrap_or(300).clamp(20, 4000);
    let gate_logs_limit = req.gate_logs_limit.unwrap_or(8).clamp(1, 50);

    let history = match read_recent_audit_events(history_limit) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };
    let reports = match list_report_files(reports_limit) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };
    let gate_logs = match read_recent_gate_logs(gate_logs_limit, 20_000) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };

    use crate::preflight::{graph::run_preflight_graph, model::PreflightStepType};
    let preflight_report = run_preflight_graph(
        Path::new("."),
        vec![
            PreflightStepType::Policy,
            PreflightStepType::Hook,
            PreflightStepType::Drift,
        ],
        None,
        None,
    )
    .await
    .unwrap_or_default();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let now_secs = now.as_secs();
    let now_nanos = now.subsec_nanos();
    let stamp = format!("{}_{}", now_secs, now_nanos);
    let root = reports_root();
    if let Err(err) = fs::create_dir_all(&root) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    // Use both seconds and nanoseconds to avoid timestamp collision within same second
    let file_name = format!("evidence_bundle_{}.json", stamp);
    let file_path = root.join(&file_name);

    let chain_info = pilot_core::verify_audit_chain();
    let mut artifacts = Vec::new();
    for row in &reports {
        if let Some(p) = row.get("path").and_then(|v| v.as_str()) {
            let abs_path = root.join(p);
            let sha = pilot_core::compute_file_hash(&abs_path)
                .unwrap_or_else(|_| "hash_failed".to_string());
            let size_bytes = std::fs::metadata(&abs_path).map(|m| m.len()).unwrap_or(0);
            artifacts.push(pilot_core::EvidenceArtifact {
                path: p.to_string(),
                sha256: sha,
                size_bytes,
            });
        }
    }
    artifacts.sort_by(|a, b| a.path.cmp(&b.path));

    let manifest = pilot_core::EvidenceBundleManifest {
        bundle_id: stamp.clone(),
        created_at: Utc::now().to_rfc3339(),
        scope_id: state
            .agorg_store
            .get_active_agorg()
            .await
            .ok()
            .flatten()
            .map(|a| a.id.to_string()),
        operator: std::env::var("USER").ok(),
        artifacts,
        chain_integrity: json!({
            "is_valid": chain_info.is_valid,
            "audited_events": chain_info.audited_events,
            "errors": chain_info.errors
        }),
    };

    let bundle_hash = manifest.compute_hash();

    let bundle = json!({
        "exported_at_unix": now,
        "bundle_hash": bundle_hash,
        "manifest": manifest,
        "bus": {
            "ws_url": state.bus.ws_url,
            "room": state.bus.room,
            "channel": state.bus.channel,
            "telemetry_channel": state.bus.telemetry_channel
        },
        "counts": {
            "history_events": history.len(),
            "report_files": reports.len(),
            "gate_logs": gate_logs.len()
        },
        "policy": {
            "preflight_report": preflight_report
        },
        "history": history,
        "reports": reports,
        "gate_logs": gate_logs
    });

    let bytes = match serde_json::to_vec_pretty(&bundle) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    };
    if let Err(err) = fs::write(&file_path, &bytes) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }

    let _ = state.events.send(json!({
        "source": "evidence_export",
        "file": file_name,
        "size_bytes": bytes.len(),
        "history_events": bundle["counts"]["history_events"],
        "report_files": bundle["counts"]["report_files"]
    }));

    Json(json!({
        "ok": true,
        "path": file_path.display().to_string(),
        "size_bytes": bytes.len(),
        "history_events": bundle["counts"]["history_events"],
        "report_files": bundle["counts"]["report_files"],
        "gate_logs": bundle["counts"]["gate_logs"]
    }))
    .into_response()
}

#[derive(Deserialize)]
struct EvidenceVerifyRequest {
    path: String,
}

async fn api_evidence_verify(Json(req): Json<EvidenceVerifyRequest>) -> Response {
    let path = std::path::PathBuf::from(&req.path);
    if !path.exists() {
        return Json(json!({
            "ok": false,
            "error": "Bundle file not found",
            "is_valid": false,
            "reason_code": "missing_file",
            "details": format!("Bundle file not found: {}", req.path),
            "offending_path": req.path
        }))
        .into_response();
    }

    let result = pilot_core::verify_evidence_bundle(&path);
    Json(json!({
        "ok": result.is_valid,
        "is_valid": result.is_valid,
        "reason_code": result.reason_code,
        "details": result.details,
        "offending_path": result.offending_path
    }))
    .into_response()
}

async fn run_codex_action(
    State(state): State<Arc<UiState>>,
    Json(mut req): Json<CodexActionRequest>,
) -> Response {
    let mode = req
        .mode
        .as_deref()
        .unwrap_or("preview")
        .trim()
        .to_ascii_lowercase();
    if !matches!(
        mode.as_str(),
        "preview" | "approve" | "execute" | "reconcile"
    ) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "mode must be preview, approve, execute, or reconcile",
        );
    }

    if mode == "preview" {
        let intent = req.intent.as_deref().unwrap_or("").trim().to_string();
        if intent.is_empty() {
            return error_response(StatusCode::BAD_REQUEST, "intent is required");
        }
        let command = req.command.as_deref().unwrap_or("").trim().to_string();
        if command.is_empty()
            || !(command.starts_with("pilot.")
                || command.starts_with("api.agorg.")
                || command.starts_with("pilot.dependency."))
        {
            return error_response(
                StatusCode::BAD_REQUEST,
                "command must be namespaced as pilot.*, pilot.dependency.*, or api.agorg.*",
            );
        }
        if let Some(allowlist) = &state.allowed_commands {
            if !allowlist.contains(&command) {
                return error_response(
                    StatusCode::FORBIDDEN,
                    &format!("command '{}' is not in ui allowlist", command),
                );
            }
        }

        if req.payload.get("schema_version").is_none() {
            req.payload["schema_version"] = json!(1);
        }
        let mut normalized_payload = req.payload.clone();
        enforce_dry_run(&command, &mut normalized_payload);
        let mutating_command = command_requires_mutation(&command, &normalized_payload);
        let now = now_unix();
        let contract_id = new_codex_contract_id();
        let contract = CodexContractRecord {
            contract_id: contract_id.clone(),
            status: "previewed".to_string(),
            intent,
            command: command.clone(),
            payload_original: req.payload.clone(),
            payload_normalized: normalized_payload,
            mutating_command,
            expected_effect: req.expected_effect.clone().filter(|s| !s.trim().is_empty()),
            rollback_strategy: req
                .rollback_strategy
                .clone()
                .filter(|s| !s.trim().is_empty()),
            verify_command: req.verify_command.clone().filter(|s| !s.trim().is_empty()),
            verify_payload: json!({"schema_version": 1}),
            execute_response: None,
            verify_response: None,
            last_error: None,
            reconcile_notes: None,
            created_at_unix: now,
            updated_at_unix: now,
        };

        {
            let mut contracts = state.codex_contracts.lock().await;
            contracts.insert(contract_id.clone(), contract.clone());
        }
        if let Err(err) = append_codex_contract_record(&state.codex_contracts_log, &contract) {
            let _ = state.events.send(json!({
                "source": "codex_action",
                "phase": "persist_warning",
                "contract_id": contract.contract_id,
                "error": err.to_string()
            }));
        }
        let _ = state.events.send(json!({
            "source": "codex_action",
            "phase": "preview",
            "contract_id": contract_id,
            "command": command
        }));
        return Json(json!({"ok": true, "contract": contract})).into_response();
    }

    let contract_id = req.contract_id.as_deref().unwrap_or("").trim().to_string();
    if contract_id.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "contract_id is required for approve/execute/reconcile",
        );
    }

    if mode == "approve" {
        let mut contracts = state.codex_contracts.lock().await;
        let Some(contract) = contracts.get_mut(&contract_id) else {
            return error_response(StatusCode::NOT_FOUND, "contract_id not found");
        };
        if contract.status != "previewed"
            && contract.status != "approved"
            && contract.status != "failed"
        {
            return error_response(
                StatusCode::CONFLICT,
                "contract must be in previewed/approved/failed state",
            );
        }
        if let Some(v) = req
            .expected_effect
            .as_ref()
            .filter(|s| !s.trim().is_empty())
        {
            contract.expected_effect = Some(v.trim().to_string());
        }
        if let Some(v) = req
            .rollback_strategy
            .as_ref()
            .filter(|s| !s.trim().is_empty())
        {
            contract.rollback_strategy = Some(v.trim().to_string());
        }
        if let Some(v) = req.verify_command.as_ref().filter(|s| !s.trim().is_empty()) {
            contract.verify_command = Some(v.trim().to_string());
        }
        contract.status = "approved".to_string();
        contract.updated_at_unix = now_unix();
        let response_contract = contract.clone();
        if let Err(err) =
            append_codex_contract_record(&state.codex_contracts_log, &response_contract)
        {
            let _ = state.events.send(json!({
                "source": "codex_action",
                "phase": "persist_warning",
                "contract_id": response_contract.contract_id,
                "error": err.to_string()
            }));
        }
        let _ = state.events.send(json!({
            "source": "codex_action",
            "phase": "approved",
            "contract_id": response_contract.contract_id,
            "command": response_contract.command
        }));
        return Json(json!({"ok": true, "contract": response_contract})).into_response();
    }

    if mode == "execute" {
        let execute_contract = {
            let contracts = state.codex_contracts.lock().await;
            let Some(contract) = contracts.get(&contract_id) else {
                return error_response(StatusCode::NOT_FOUND, "contract_id not found");
            };
            if contract.status != "approved" {
                return error_response(
                    StatusCode::CONFLICT,
                    "contract must be approved before execute",
                );
            }
            contract.clone()
        };

        if let Some(allowlist) = &state.allowed_commands {
            if !allowlist.contains(&execute_contract.command) {
                return error_response(
                    StatusCode::FORBIDDEN,
                    &format!(
                        "command '{}' is not in ui allowlist",
                        execute_contract.command
                    ),
                );
            }
        }
        if !state.allow_mutations && execute_contract.mutating_command {
            return error_response(
                StatusCode::FORBIDDEN,
                &format!(
                    "command '{}' blocked in read-only UI mode",
                    execute_contract.command
                ),
            );
        }

        let _ = state.events.send(json!({
            "source": "codex_action",
            "phase": "started",
            "contract_id": execute_contract.contract_id,
            "command": execute_contract.command,
            "intent": execute_contract.intent
        }));

        let exec_result = if execute_contract.command.starts_with("pilot.dependency.") {
            run_local_dependency_contract_command(
                &state,
                &execute_contract.command,
                execute_contract.payload_normalized.clone(),
            )
            .await
        } else if execute_contract.command.starts_with("api.agorg.") {
            run_local_agorg_contract_command(
                &state,
                &execute_contract.command,
                execute_contract.payload_normalized.clone(),
            )
            .await
        } else {
            send_command_once_with_retry(
                &state.bus,
                &execute_contract.command,
                execute_contract.payload_normalized.clone(),
                3,
            )
            .await
        };
        match exec_result {
            Ok(response) => {
                let updated = {
                    let mut contracts = state.codex_contracts.lock().await;
                    let Some(contract) = contracts.get_mut(&contract_id) else {
                        return error_response(StatusCode::NOT_FOUND, "contract_id not found");
                    };
                    contract.status = "executed".to_string();
                    contract.execute_response = Some(response.clone());
                    contract.last_error = None;
                    contract.updated_at_unix = now_unix();
                    contract.clone()
                };
                let _ = state.events.send(json!({
                    "source": "codex_action",
                    "phase": "completed",
                    "contract_id": updated.contract_id,
                    "command": updated.command,
                    "success": true
                }));
                if let Err(err) = append_codex_contract_record(&state.codex_contracts_log, &updated)
                {
                    let _ = state.events.send(json!({
                        "source": "codex_action",
                        "phase": "persist_warning",
                        "contract_id": updated.contract_id,
                        "error": err.to_string()
                    }));
                }
                return Json(json!({"ok": true, "contract": updated, "response": response}))
                    .into_response();
            }
            Err(err) => {
                let updated = {
                    let mut contracts = state.codex_contracts.lock().await;
                    if let Some(contract) = contracts.get_mut(&contract_id) {
                        contract.status = "failed".to_string();
                        contract.last_error = Some(err.to_string());
                        contract.updated_at_unix = now_unix();
                        Some(contract.clone())
                    } else {
                        None
                    }
                };
                let _ = state.events.send(json!({
                    "source": "codex_action",
                    "phase": "failed",
                    "contract_id": contract_id,
                    "error": err.to_string()
                }));
                if let Some(contract) = updated {
                    if let Err(persist_err) =
                        append_codex_contract_record(&state.codex_contracts_log, &contract)
                    {
                        let _ = state.events.send(json!({
                            "source": "codex_action",
                            "phase": "persist_warning",
                            "contract_id": contract.contract_id,
                            "error": persist_err.to_string()
                        }));
                    }
                    return Json(
                        json!({"ok": false, "contract": contract, "error": err.to_string()}),
                    )
                    .into_response();
                }
                return error_response(StatusCode::BAD_GATEWAY, &err.to_string());
            }
        }
    }

    // reconcile
    let reconcile_contract = {
        let contracts = state.codex_contracts.lock().await;
        let Some(contract) = contracts.get(&contract_id) else {
            return error_response(StatusCode::NOT_FOUND, "contract_id not found");
        };
        if contract.status != "executed" && contract.status != "failed" {
            return error_response(
                StatusCode::CONFLICT,
                "contract must be executed or failed before reconcile",
            );
        }
        contract.clone()
    };

    let mut verify_response: Option<Value> = None;
    if let Some(verify_cmd) = reconcile_contract.verify_command.as_ref() {
        if verify_cmd.starts_with("pilot.") || verify_cmd.starts_with("api.agorg.") {
            if let Some(allowlist) = &state.allowed_commands {
                if !allowlist.contains(verify_cmd) {
                    return error_response(
                        StatusCode::FORBIDDEN,
                        &format!("verify command '{}' is not in ui allowlist", verify_cmd),
                    );
                }
            }
            let verify_result = if verify_cmd.starts_with("pilot.dependency.") {
                run_local_dependency_contract_command(
                    &state,
                    verify_cmd,
                    reconcile_contract.verify_payload.clone(),
                )
                .await
            } else if verify_cmd.starts_with("api.agorg.") {
                run_local_agorg_contract_command(
                    &state,
                    verify_cmd,
                    reconcile_contract.verify_payload.clone(),
                )
                .await
            } else {
                send_command_once_with_retry(
                    &state.bus,
                    verify_cmd,
                    reconcile_contract.verify_payload.clone(),
                    3,
                )
                .await
            };
            match verify_result {
                Ok(v) => verify_response = Some(v),
                Err(err) => {
                    let _ = state.events.send(json!({
                        "source": "codex_action",
                        "phase": "reconcile_verify_failed",
                        "contract_id": contract_id,
                        "error": err.to_string()
                    }));
                }
            }
        }
    }

    let updated = {
        let mut contracts = state.codex_contracts.lock().await;
        let Some(contract) = contracts.get_mut(&contract_id) else {
            return error_response(StatusCode::NOT_FOUND, "contract_id not found");
        };
        contract.status = "reconciled".to_string();
        if let Some(v) = verify_response.clone() {
            contract.verify_response = Some(v);
        }
        if let Some(v) = req
            .reconcile_notes
            .as_ref()
            .filter(|s| !s.trim().is_empty())
        {
            contract.reconcile_notes = Some(v.trim().to_string());
        }
        contract.updated_at_unix = now_unix();
        contract.clone()
    };
    let _ = state.events.send(json!({
        "source": "codex_action",
        "phase": "reconciled",
        "contract_id": updated.contract_id,
        "command": updated.command
    }));
    if let Err(err) = append_codex_contract_record(&state.codex_contracts_log, &updated) {
        let _ = state.events.send(json!({
            "source": "codex_action",
            "phase": "persist_warning",
            "contract_id": updated.contract_id,
            "error": err.to_string()
        }));
    }
    Json(json!({
        "ok": true,
        "contract": updated,
        "verify_response": verify_response
    }))
    .into_response()
}

async fn run_local_agorg_contract_command(
    state: &Arc<UiState>,
    command: &str,
    payload: Value,
) -> Result<Value> {
    match command {
        "api.agorg.policy_report" => {
            let agorg = payload
                .get("agorg")
                .and_then(Value::as_str)
                .map(str::to_string);
            agorg_policy_report_core(state, agorg.as_deref())
                .await
                .map_err(|e| miette::miette!("{e}"))
        }
        "api.agorg.reconcile_apply" => {
            let req: AgorgReconcileApplyRequest = serde_json::from_value(payload)
                .map_err(|e| miette::miette!("invalid reconcile_apply payload: {e}"))?;
            agorg_reconcile_apply_core(state, req, true)
                .await
                .map_err(|e| miette::miette!("{e}"))
        }
        _ => Err(miette::miette!(
            "unsupported local AGOrg contract command '{}'",
            command
        )),
    }
}

async fn run_local_dependency_contract_command(
    state: &Arc<UiState>,
    command: &str,
    payload: Value,
) -> Result<Value> {
    let Some(action) = command.strip_prefix("pilot.dependency.") else {
        return Err(miette::miette!(
            "unsupported local dependency contract command '{}'",
            command
        ));
    };
    if action.trim().is_empty() {
        return Err(miette::miette!("dependency action suffix cannot be empty"));
    }
    let mut req_payload = if payload.is_object() {
        payload
    } else {
        json!({})
    };
    req_payload["action"] = json!(action);
    if req_payload.get("json").is_none() {
        req_payload["json"] = json!(true);
    }
    let req: DependencyActionRequest = serde_json::from_value(req_payload)
        .map_err(|e| miette::miette!("invalid dependency payload: {e}"))?;
    let resp = run_dependency_action(State(state.clone()), Json(req)).await;
    Ok(extract_json_body(resp).await)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn new_codex_contract_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("codex-{}", nanos)
}

fn codex_contracts_log_path() -> PathBuf {
    reports_root().join("codex_contracts.jsonl")
}

fn append_codex_contract_record(
    path: &PathBuf,
    record: &CodexContractRecord,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    writeln!(file, "{}", line)?;
    Ok(())
}

fn load_persisted_codex_contracts(
    path: &PathBuf,
) -> std::io::Result<HashMap<String, CodexContractRecord>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let file = OpenOptions::new().read(true).open(path)?;
    let reader = BufReader::new(file);
    let mut contracts = HashMap::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed: serde_json::Result<CodexContractRecord> = serde_json::from_str(trimmed);
        if let Ok(record) = parsed {
            let replace = contracts
                .get(&record.contract_id)
                .map(|current: &CodexContractRecord| {
                    current.updated_at_unix <= record.updated_at_unix
                })
                .unwrap_or(true);
            if replace {
                contracts.insert(record.contract_id.clone(), record);
            }
        }
    }
    Ok(contracts)
}

fn is_safe_cli_token(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/'))
}

#[cfg(test)]
mod tests {
    use super::{
        agorg_conformance_score, agorg_policy_report_response,
        agorg_reconcile_apply_dry_run_response, agorg_reconcile_apply_success_response,
        append_codex_contract_record, canonical_branch_payload, classify_bus_health,
        classify_db_health, command_request_from_orchestrate_payload, command_requires_cwd_scope,
        command_requires_multi_selector, command_requires_mutation, command_scope_required,
        dashboard_routine_guard_summary, default_policy_json_for_kind,
        dependency_action_requires_cwd_scope, dependency_action_scope_required,
        discover_dashboard_ci_catalog, filter_prune_paths_by_class, is_bus_recoverable_error,
        is_safe_cli_token, load_persisted_codex_contracts, normalize_orchestrate_payload,
        orchestrate_is_preview, parse_gh_status_summary, parse_gh_watch_summary,
        parse_json_from_mixed_output, parse_release_collect_evidence_path,
        parse_workflow_catalog_entry, payload_has_multi_selector, preflight_steps_from_action,
        prune_expired_branch_previews, resolve_branch_targets, sanitize_payload_for_local_exec,
        scope_filter_rows, should_prefer_local_command, should_use_local_command_fallback,
        sorted_unique_ids, sorted_unique_tags, with_event_agorg_scope, BranchMatrixRow,
        BranchPreviewRecord, BranchRunRequest, CodexContractRecord, INDEX_HTML,
    };
    use crate::agorg::{AgorgReconcileIssue, AgorgReconcileReport};
    use crate::db_runtime::DbStatus;
    use crate::governance::model::{
        EnforcementLevel, OperatorRoutinePolicy, PolicyEvalReport, PolicyEvalResult,
    };
    use pilot_multi as multi;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    #[test]
    fn test_safe_cli_token() {
        assert!(is_safe_cli_token("main"));
        assert!(is_safe_cli_token("origin"));
        assert!(is_safe_cli_token("feat/pilot-wave9"));
        assert!(is_safe_cli_token("release/v1.0.0"));
        assert!(!is_safe_cli_token(""));
        assert!(!is_safe_cli_token("main;rm -rf /"));
        assert!(!is_safe_cli_token("origin && whoami"));
        assert!(!is_safe_cli_token("bad token"));
    }

    #[test]
    fn test_scope_command_classification() {
        assert!(command_scope_required("pilot.multi.status"));
        assert!(command_scope_required("pilot.branch.create"));
        assert!(command_scope_required("pilot.oracle.scan"));
        assert!(!command_scope_required("pilot.know.record"));

        assert!(command_requires_cwd_scope("pilot.branch.status"));
        assert!(command_requires_cwd_scope("pilot.oracle.query"));
        assert!(!command_requires_cwd_scope("pilot.multi.status"));

        assert!(command_requires_multi_selector("pilot.multi.apply"));
        assert!(!command_requires_multi_selector("pilot.branch.create"));
    }

    #[test]
    fn test_orchestrate_preview_detection() {
        assert!(orchestrate_is_preview(
            &json!({"dry_run": true, "action": "sync"})
        ));
        assert!(orchestrate_is_preview(&json!({"action": "status"})));
        assert!(orchestrate_is_preview(&json!({"action": "hook-policy"})));
        assert!(!orchestrate_is_preview(
            &json!({"action": "sync", "dry_run": false})
        ));
    }

    #[test]
    fn test_orchestrate_payload_preview_normalization() {
        let mut payload = json!({"action":"multi.apply","apply":true,"dry_run":false});
        normalize_orchestrate_payload(&mut payload, "preview");
        assert_eq!(payload.get("dry_run").and_then(Value::as_bool), Some(true));
        assert_eq!(payload.get("apply").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn test_command_request_from_orchestrate_action_aliases() {
        let req = command_request_from_orchestrate_payload(json!({
            "action": "multi.status",
            "group": "core",
            "tags": ["apply-pilot"]
        }))
        .expect("multi.status alias should map");
        assert_eq!(req.command, "pilot.multi.status");
        assert_eq!(
            req.payload.get("group").and_then(Value::as_str),
            Some("core")
        );

        let dag_req = command_request_from_orchestrate_payload(json!({
            "action": "dag.evaluate",
            "group": "core",
            "tags": ["apply-pilot"]
        }))
        .expect("dag.evaluate alias should map");
        assert_eq!(dag_req.command, "pilot.multi.dag");
        assert_eq!(
            dag_req.payload.get("dry_run").and_then(Value::as_bool),
            Some(true)
        );

        let heal_req = command_request_from_orchestrate_payload(json!({
            "action": "heal.plan"
        }))
        .expect("heal.plan alias should map");
        assert_eq!(heal_req.command, "pilot.heal.run");
        assert_eq!(
            heal_req.payload.get("plan_only").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn test_scope_dependency_action_classification() {
        assert!(dependency_action_scope_required("policy"));
        assert!(dependency_action_scope_required("gate"));
        assert!(dependency_action_scope_required("prepush-gate"));
        assert!(dependency_action_scope_required("cargo-fmt"));
        assert!(dependency_action_scope_required("push"));
        assert!(dependency_action_scope_required("release-readiness"));
        assert!(dependency_action_scope_required("release-collect-evidence"));
        assert!(dependency_action_scope_required("release-verify-bundle"));
        assert!(dependency_action_scope_required("ci-trigger"));
        assert!(dependency_action_scope_required("ci-watch"));
        assert!(dependency_action_scope_required("ci-status"));
        assert!(!dependency_action_scope_required("db-status"));
        assert!(!dependency_action_scope_required("services-start"));

        assert!(!dependency_action_requires_cwd_scope("repair"));
        assert!(!dependency_action_requires_cwd_scope("cargo-fmt"));
        assert!(!dependency_action_requires_cwd_scope("db-start"));
    }

    #[test]
    fn test_parse_release_collect_evidence_path() {
        let stdout = r#"
Release evidence collected and manifest generated at:
  /home/tester/.pilot/release_evidence/release_alpha_20260306T010203Z
Summary:
  /home/tester/.pilot/release_evidence/release_alpha_20260306T010203Z/SUMMARY.md
"#;
        let parsed = parse_release_collect_evidence_path(stdout, "");
        assert_eq!(
            parsed.as_deref(),
            Some("/home/tester/.pilot/release_evidence/release_alpha_20260306T010203Z")
        );
    }

    #[test]
    fn test_parse_gh_watch_summary() {
        let stdout = r#"
========== gh_watch summary ==========
result:                FAIL
repo:                  novelbytelabs/ArqonPilot
branch:                main
run_id:                123456789
workflow:              ci.yml
status:                completed
conclusion:            failure
failed_jobs:           2
failed_job_names:      rust, ui-smoke
run_url:               https://github.com/x/y/actions/runs/123
likely_cause:          job_failures
======================================
"#;
        let parsed = parse_gh_watch_summary(stdout, "").expect("summary should parse");
        assert_eq!(parsed.get("result").and_then(Value::as_str), Some("FAIL"));
        assert_eq!(parsed.get("failed_jobs").and_then(Value::as_str), Some("2"));
    }

    #[test]
    fn test_parse_gh_status_summary() {
        let stdout = r#"
========== gh_status summary ==========
repo:                  novelbytelabs/ArqonPilot
branch:                main
ci_run_id:             111
docs_run_id:           222
overall_state:         running
overall_conclusion:    unknown
docs_state:            pass
rust_state:            running
ui_smoke_state:        pass
packaging_parity_state: fail
run_url:               https://github.com/novelbytelabs/ArqonPilot/actions/runs/111
======================================
"#;
        let parsed = parse_gh_status_summary(stdout, "").expect("summary should parse");
        assert_eq!(
            parsed.get("repo").and_then(Value::as_str),
            Some("novelbytelabs/ArqonPilot")
        );
        assert_eq!(
            parsed.get("overall_state").and_then(Value::as_str),
            Some("running")
        );
        assert_eq!(
            parsed.get("docs_state").and_then(Value::as_str),
            Some("pass")
        );
        assert_eq!(
            parsed.get("rust_state").and_then(Value::as_str),
            Some("running")
        );
        assert_eq!(
            parsed.get("packaging_parity_state").and_then(Value::as_str),
            Some("fail")
        );
    }

    #[test]
    fn test_payload_multi_selector() {
        assert!(!payload_has_multi_selector(&json!({})));
        assert!(!payload_has_multi_selector(&json!({"group": ""})));
        assert!(!payload_has_multi_selector(&json!({"tags": []})));
        assert!(payload_has_multi_selector(&json!({"group": "core"})));
        assert!(payload_has_multi_selector(
            &json!({"tags": ["apply-pilot"]})
        ));
    }

    #[test]
    fn test_command_requires_mutation() {
        assert!(!command_requires_mutation(
            "api.agorg.reconcile_apply",
            &json!({"dry_run": true})
        ));
        assert!(command_requires_mutation(
            "api.agorg.reconcile_apply",
            &json!({"dry_run": false})
        ));
        assert!(!command_requires_mutation(
            "pilot.multi.apply",
            &json!({"apply": false})
        ));
        assert!(command_requires_mutation(
            "pilot.multi.apply",
            &json!({"apply": true})
        ));
        assert!(!command_requires_mutation(
            "pilot.heal.run",
            &json!({"plan_only": true})
        ));
    }

    #[test]
    fn test_event_agorg_scope_annotation() {
        let plain = json!({"source": "bus_listener"});
        let tagged = with_event_agorg_scope(plain, None);
        assert_eq!(tagged.get("agorg_scope"), Some(&Value::Null));

        let existing = json!({"source": "ui_command", "agorg_scope": {"id":"x"}});
        let preserved = with_event_agorg_scope(existing.clone(), None);
        assert_eq!(preserved, existing);
    }

    #[test]
    fn test_classify_bus_health_running() {
        let (running, state, note) = classify_bus_health(Ok((
            0,
            "[shim] RUNNING pid=123 host=127.0.0.1 port=9100".to_string(),
            "".to_string(),
        )));
        assert!(running);
        assert_eq!(state, "RUNNING");
        assert!(note.is_empty());
    }

    #[test]
    fn test_classify_bus_health_probe_failed() {
        let (running, state, note) =
            classify_bus_health(Ok((1, "".to_string(), "ss: command not found".to_string())));
        assert!(!running);
        assert_eq!(state, "PROBE_FAILED");
        assert!(note.contains("iproute2") || note.contains("SS_BIN"));
    }

    #[test]
    fn test_classify_bus_health_unavailable() {
        let (running, state, note) = classify_bus_health(Err("spawn failed: denied".to_string()));
        assert!(!running);
        assert_eq!(state, "UNAVAILABLE");
        assert!(note.contains("spawn failed"));
    }

    #[test]
    fn test_is_bus_recoverable_error() {
        assert!(is_bus_recoverable_error(
            "Bus connect failed ws://127.0.0.1:9100: IO error: Connection refused (os error 111)"
        ));
        assert!(is_bus_recoverable_error(
            "WebSocket protocol error: Connection reset without closing handshake"
        ));
        assert!(is_bus_recoverable_error("request timed out"));
        assert!(!is_bus_recoverable_error("command not in ui allowlist"));
    }

    #[test]
    fn test_should_use_local_command_fallback() {
        assert!(should_use_local_command_fallback(
            "Timed out waiting for command response for pilot.multi.register"
        ));
        assert!(should_use_local_command_fallback(
            "Bus connect failed ws://127.0.0.1:9100"
        ));
        assert!(!should_use_local_command_fallback(
            "command not in ui allowlist"
        ));
    }

    #[test]
    fn test_should_prefer_local_command() {
        assert!(should_prefer_local_command("pilot.multi.register"));
        assert!(should_prefer_local_command("pilot.multi.status"));
        assert!(!should_prefer_local_command("pilot.oracle.scan"));
        assert!(!should_prefer_local_command("pilot.heal.run"));
    }

    #[test]
    fn test_sanitize_payload_for_local_exec_removes_agorg_scope() {
        let input = json!({
            "schema_version": 1,
            "path": "/tmp/repo",
            "name": "Repo",
            "group": "core",
            "tags": ["apply-pilot"],
            "agorg_scope": {
                "id": "abc",
                "name": "Arqon"
            }
        });
        let out = sanitize_payload_for_local_exec(input);
        assert!(out.get("agorg_scope").is_none());
        assert_eq!(out.get("path").and_then(Value::as_str), Some("/tmp/repo"));
        assert_eq!(out.get("name").and_then(Value::as_str), Some("Repo"));
    }

    #[test]
    fn test_default_policy_json_for_operator_routine_kind() {
        let v = default_policy_json_for_kind("operator_routine").unwrap();
        let obj = v
            .as_object()
            .expect("operator_routine default policy should be a JSON object");
        assert!(!obj.is_empty());
        assert!(default_policy_json_for_kind("not-a-kind").is_err());
    }

    #[test]
    fn test_dashboard_routine_deck_markup_is_present() {
        assert!(INDEX_HTML.contains("dash-routine-stage-resolve-tab"));
        assert!(INDEX_HTML.contains("dash-routine-stage-panel"));
        assert!(INDEX_HTML.contains("dash-routine-policy-modal"));
        assert!(INDEX_HTML.contains("dash-routine-branch"));
        assert!(INDEX_HTML.contains("dash-routine-ci-observatory-title"));
        assert!(INDEX_HTML.contains("dash-routine-ci-policy-summary"));
    }

    #[test]
    fn test_dashboard_routine_guard_summary_counts() {
        let mut report = PolicyEvalReport::default();
        report.blocked = true;
        report.violations.push(PolicyEvalResult {
            rule: "operator_routine.ORT-001".to_string(),
            level: EnforcementLevel::Block,
            input: "push".to_string(),
            violation: "Active scope missing".to_string(),
            fix_suggestion: "Select an AGOrg".to_string(),
            policy_source: "ui".to_string(),
            policy_source_id: Some(Uuid::nil()),
            policy_source_name: "UI".to_string(),
            override_available: false,
        });
        report.warnings.push(PolicyEvalResult {
            rule: "operator_routine.ORT-005".to_string(),
            level: EnforcementLevel::Warn,
            input: "main".to_string(),
            violation: "Branch warning".to_string(),
            fix_suggestion: "Use an allowed branch".to_string(),
            policy_source: "ui".to_string(),
            policy_source_id: Some(Uuid::nil()),
            policy_source_name: "UI".to_string(),
            override_available: false,
        });
        let summary = dashboard_routine_guard_summary(&report);
        assert_eq!(summary.get("blocked").and_then(Value::as_bool), Some(true));
        assert_eq!(
            summary.get("violation_count").and_then(Value::as_u64),
            Some(1)
        );
        assert_eq!(
            summary.get("warning_count").and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn test_parse_workflow_catalog_entry_extracts_name_triggers_and_jobs() {
        let entry = parse_workflow_catalog_entry(
            PathBuf::from(".github/workflows/ci.yml").as_path(),
            r#"
name: ArqonPilot CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  rust:
    name: Rust
    runs-on: ubuntu-latest
  ui-smoke:
    runs-on: ubuntu-latest
"#,
            true,
        );
        assert_eq!(entry.key, "ci.yml");
        assert_eq!(entry.workflow_name, "ArqonPilot CI");
        assert!(entry.trigger_events.iter().any(|event| event == "push"));
        assert!(entry
            .trigger_events
            .iter()
            .any(|event| event == "pull_request"));
        assert_eq!(entry.jobs.len(), 2);
        assert_eq!(entry.jobs[0].id, "rust");
        assert_eq!(entry.jobs[0].label, "Rust");
        assert!(entry.jobs[0].required_by_policy);
        assert_eq!(entry.jobs[1].id, "ui-smoke");
    }

    #[test]
    fn test_discover_dashboard_ci_catalog_reports_missing_required_jobs() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pilot-ci-catalog-{unique}"));
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).expect("workflow dir should exist");
        fs::write(
            workflows_dir.join("ci.yml"),
            r#"
name: ArqonPilot CI
on:
  push:
    branches: [main]
jobs:
  rust:
    runs-on: ubuntu-latest
"#,
        )
        .expect("ci.yml should be written");
        fs::write(
            workflows_dir.join("docs.yml"),
            r#"
name: Docs (MkDocs)
on:
  push:
    branches: [main]
jobs:
  build:
    runs-on: ubuntu-latest
"#,
        )
        .expect("docs.yml should be written");

        let policy = OperatorRoutinePolicy::default();
        let (workflows, gaps, ci_enabled, warnings) =
            discover_dashboard_ci_catalog(&workflows_dir, &policy).expect("catalog should parse");

        assert!(ci_enabled);
        assert_eq!(workflows.len(), 2);
        assert!(warnings.is_empty());
        assert!(gaps.iter().any(|gap| gap.id == "ui-smoke"));
        assert!(gaps.iter().any(|gap| gap.id == "packaging-parity"));
        assert!(!gaps
            .iter()
            .any(|gap| gap.id == "build" && gap.kind == "job"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_discover_dashboard_ci_catalog_missing_directory_yields_warnings_and_gaps() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let missing_dir = std::env::temp_dir().join(format!("pilot-ci-catalog-missing-{unique}"));
        let policy = OperatorRoutinePolicy::default();
        let (workflows, gaps, ci_enabled, warnings) =
            discover_dashboard_ci_catalog(&missing_dir, &policy).expect("catalog should degrade");

        assert!(ci_enabled);
        assert!(workflows.is_empty());
        assert!(warnings
            .iter()
            .any(|warn| warn.contains("Cannot read workflow directory")));
        assert!(gaps.iter().any(|gap| gap.id == "ci.yml"));
        assert!(gaps.iter().any(|gap| gap.id == "docs.yml"));
    }

    #[test]
    fn test_classify_db_health_states() {
        let stopped = DbStatus {
            initialized: true,
            running: false,
            error_note: Some("permission denied".to_string()),
            mode: "tcp".to_string(),
            endpoint: "127.0.0.1:9132".to_string(),
            dsn: "host=127.0.0.1 port=9132 user=x dbname=pilot_local".to_string(),
            data_dir: "/tmp/db".to_string(),
            log_file: "/tmp/postgres.log".to_string(),
        };
        let (running1, state1, note1) = classify_db_health(Ok(Some(stopped)));
        assert!(!running1);
        assert_eq!(state1, "STOPPED");
        assert!(note1.contains("permission denied"));

        let (running2, state2, note2) = classify_db_health(Ok(None));
        assert!(running2);
        assert_eq!(state2, "RUNNING");
        assert!(note2.contains("disabled"));

        let (running3, state3, note3) = classify_db_health(Err("status read failed".to_string()));
        assert!(!running3);
        assert_eq!(state3, "UNAVAILABLE");
        assert!(note3.contains("status read failed"));
    }

    #[test]
    fn test_preflight_steps_from_legacy_actions() {
        let policy = preflight_steps_from_action("policy", None);
        assert_eq!(policy.len(), 1);
        let gate = preflight_steps_from_action("gate", None);
        assert_eq!(gate.len(), 1);
        let hook = preflight_steps_from_action("hook-policy", None);
        assert_eq!(hook.len(), 1);
    }

    #[test]
    fn test_preflight_steps_from_preflight_payload() {
        let steps = preflight_steps_from_action(
            "preflight",
            Some(vec![
                "policy".to_string(),
                "drift".to_string(),
                "unknown".to_string(),
            ]),
        );
        assert_eq!(steps.len(), 2);

        let fallback = preflight_steps_from_action("preflight", Some(vec!["invalid".to_string()]));
        assert_eq!(fallback.len(), 4);
    }

    #[test]
    fn test_codex_contract_persistence_roundtrip() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("pilot_codex_test_{}", nanos));
        fs::create_dir_all(&base).unwrap();
        let path = base.join("codex_contracts.jsonl");

        let c1 = CodexContractRecord {
            contract_id: "codex-1".to_string(),
            status: "previewed".to_string(),
            intent: "intent".to_string(),
            command: "pilot.multi.status".to_string(),
            payload_original: json!({"schema_version":1}),
            payload_normalized: json!({"schema_version":1}),
            mutating_command: false,
            expected_effect: None,
            rollback_strategy: None,
            verify_command: None,
            verify_payload: json!({"schema_version":1}),
            execute_response: None,
            verify_response: None,
            last_error: None,
            reconcile_notes: None,
            created_at_unix: 1,
            updated_at_unix: 1,
        };
        let mut c2 = c1.clone();
        c2.status = "executed".to_string();
        c2.updated_at_unix = 3;
        c2.execute_response = Some(json!({"ok":true}));

        append_codex_contract_record(&path, &c1).unwrap();
        append_codex_contract_record(&path, &c2).unwrap();

        let loaded: HashMap<String, CodexContractRecord> =
            load_persisted_codex_contracts(&path).unwrap();
        let got = loaded.get("codex-1").unwrap();
        assert_eq!(got.status, "executed");
        assert_eq!(got.updated_at_unix, 3);

        let _ = fs::remove_dir_all(base);
    }

    fn sample_reconcile_report() -> AgorgReconcileReport {
        AgorgReconcileReport {
            agorg_id: Uuid::nil(),
            agorg_name: "Arqon".to_string(),
            root_path: "/tmp/arqon".to_string(),
            total_agos: 3,
            issue_count: 1,
            off_policy_count: 1,
            class_counts: std::collections::BTreeMap::from([("topology".to_string(), 1usize)]),
            prune_candidate_paths: vec!["/tmp/arqon/archive/old".to_string()],
            duplicate_resolutions: vec![],
            issues: vec![AgorgReconcileIssue {
                issue_id: "topology:archive_path:/tmp/arqon/archive/old".to_string(),
                repo_name: "old".to_string(),
                repo_path: "/tmp/arqon/archive/old".to_string(),
                severity: "warn".to_string(),
                issue_class: "topology".to_string(),
                code: "archive_path".to_string(),
                message: "off-policy".to_string(),
            }],
            governance_issues: vec![],
            conflict_traces: vec![],
            fleet_report: None,
        }
    }

    #[test]
    fn test_filter_prune_paths_by_class() {
        let report = sample_reconcile_report();
        let all = filter_prune_paths_by_class(&report, None);
        assert_eq!(all, vec!["/tmp/arqon/archive/old"]);
        let topo = filter_prune_paths_by_class(&report, Some("topology"));
        assert_eq!(topo, vec!["/tmp/arqon/archive/old"]);
        let dep = filter_prune_paths_by_class(&report, Some("policy_dependency"));
        assert!(dep.is_empty());
    }

    #[test]
    fn test_agorg_conformance_score_bounds() {
        assert_eq!(agorg_conformance_score(3, 0, 0), 100);
        assert_eq!(agorg_conformance_score(3, 1, 1), 75);
        assert_eq!(agorg_conformance_score(3, 100, 100), 0);
        assert_eq!(agorg_conformance_score(0, 0, 0), 100);
    }

    #[test]
    fn test_agorg_reconcile_api_policy_report_contract() {
        let report = sample_reconcile_report();
        let out = agorg_policy_report_response(&report, "/tmp/report.json");
        assert_eq!(out["ok"], true);
        assert_eq!(out["artifact_path"], "/tmp/report.json");
        assert_eq!(out["report"]["agorg_name"], "Arqon");
        assert_eq!(out["report"]["off_policy_count"], 1);
        assert_eq!(out["report"]["class_counts"]["topology"], 1);
    }

    #[test]
    fn test_agorg_reconcile_api_dry_run_contract() {
        let report = sample_reconcile_report();
        let out = agorg_reconcile_apply_dry_run_response(&report);
        assert_eq!(out["ok"], true);
        assert_eq!(out["dry_run"], true);
        assert_eq!(out["planned_prune_count"], 1);
        assert_eq!(out["planned_prune_paths"][0], "/tmp/arqon/archive/old");
    }

    #[test]
    fn test_agorg_reconcile_api_apply_contract() {
        let before = sample_reconcile_report();
        let mut after = sample_reconcile_report();
        after.off_policy_count = 0;
        after.issue_count = 0;
        after.prune_candidate_paths = Vec::new();
        after.issues = Vec::new();
        let out = agorg_reconcile_apply_success_response(1, &before, &after);
        assert_eq!(out["ok"], true);
        assert_eq!(out["dry_run"], false);
        assert_eq!(out["pruned"], 1);
        assert_eq!(out["before"]["off_policy_count"], 1);
        assert_eq!(out["after"]["off_policy_count"], 0);
    }

    #[test]
    fn test_persist_agorg_reconcile_writes_governance_sidecar_when_fleet_report_present() {
        let temp_root = std::env::temp_dir()
            .join("pilot_test_reports")
            .join(Uuid::new_v4().to_string());
        fs::create_dir_all(&temp_root).expect("create temp report root");
        let prior_pilot_home = std::env::var("PILOT_HOME").ok();
        std::env::set_var("PILOT_HOME", &temp_root);
        let reports_root = super::reports_root();
        fs::create_dir_all(&reports_root).expect("create reports root");
        let probe = reports_root.join("probe_write.json");
        fs::write(&probe, b"{}").expect("probe write failed");
        let _ = fs::remove_file(&probe);

        let payload = json!({
            "ok": true,
            "dry_run": true,
            "governance_issues": [{
                "ago_path": "/tmp/repo-a",
                "policy_kind": "security",
                "issue_type": "policy_violation",
                "severity": "error",
                "message": "test",
                "remediation": "fix"
            }],
            "conflict_traces": [],
            "fleet_report": {
                "agorg_id": Uuid::nil().to_string(),
                "agorg_name": "Arqon",
                "scan_time": "2026-01-01T00:00:00Z",
                "ago_statuses": []
            }
        });
        let main_path = super::persist_agorg_reconcile_action_report("dry_run", &payload)
            .expect("persist failed");
        let main_pb = PathBuf::from(&main_path);
        assert!(main_pb.exists(), "main reconcile artifact should exist");

        let file_name = main_pb
            .file_name()
            .and_then(|n| n.to_str())
            .expect("main artifact file name missing");
        let ts = file_name
            .strip_prefix("agorg_reconcile_dry_run_")
            .and_then(|n| n.strip_suffix(".json"))
            .expect("timestamp parse failed");
        let sidecar =
            super::reports_root().join(format!("governance_reconcile_dry_run_{}.json", ts));
        assert!(
            sidecar.exists(),
            "governance sidecar should exist when fleet_report is present"
        );

        let _ = fs::remove_file(main_pb);
        let _ = fs::remove_file(sidecar);
        match prior_pilot_home {
            Some(v) => std::env::set_var("PILOT_HOME", v),
            None => std::env::remove_var("PILOT_HOME"),
        }
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_parse_json_from_mixed_output_plain_json() {
        let raw = r#"{"ok":true,"checks":[]}"#;
        let parsed = parse_json_from_mixed_output(raw).unwrap();
        assert_eq!(parsed["ok"], true);
    }

    #[test]
    fn test_parse_json_from_mixed_output_with_prefix_line() {
        let raw = "cuquantum\n{\n  \"ok\": true,\n  \"checks\": []\n}\n";
        let parsed = parse_json_from_mixed_output(raw).unwrap();
        assert_eq!(parsed["ok"], true);
        assert!(parsed["checks"].as_array().is_some());
    }

    #[test]
    fn test_scope_filter_rows_respects_scope_and_search() {
        let rows = vec![
            BranchMatrixRow {
                id: 1,
                repo: "ArqonCore".to_string(),
                path: "/tmp/agorg/ArqonCore".to_string(),
                group: Some("core".to_string()),
                tags: vec!["apply-pilot".to_string()],
                current_branch: "dev".to_string(),
                clean: true,
                ahead: Some(0),
                behind: Some(0),
                on_target: Some(true),
                protected: true,
            },
            BranchMatrixRow {
                id: 2,
                repo: "ExternalRepo".to_string(),
                path: "/tmp/outside/ExternalRepo".to_string(),
                group: Some("misc".to_string()),
                tags: vec![],
                current_branch: "main".to_string(),
                clean: true,
                ahead: Some(0),
                behind: Some(0),
                on_target: Some(false),
                protected: true,
            },
        ];
        let roots = vec![PathBuf::from("/tmp/agorg")];
        let filtered = scope_filter_rows(rows.clone(), &roots, None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].repo, "ArqonCore");

        let searched = scope_filter_rows(rows, &roots, Some("core"));
        assert_eq!(searched.len(), 1);
        assert_eq!(searched[0].repo, "ArqonCore");
    }

    #[test]
    fn test_resolve_branch_targets_honors_selected_ids_and_scope() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("pilot_branch_targets_test_{}", nanos));
        let scope_root = base.join("scope");
        let outside_root = base.join("outside");
        fs::create_dir_all(&scope_root).unwrap();
        fs::create_dir_all(&outside_root).unwrap();
        let repo_a = scope_root.join("A");
        let repo_b = scope_root.join("B");
        let repo_c = outside_root.join("C");
        fs::create_dir_all(&repo_a).unwrap();
        fs::create_dir_all(&repo_b).unwrap();
        fs::create_dir_all(&repo_c).unwrap();

        let db_path = base.join("workspace.db");
        let registry = multi::MultiRegistry::open(&db_path).unwrap();
        let a = registry
            .register_repo(
                &repo_a,
                Some("A"),
                Some("core"),
                &vec!["apply-pilot".to_string()],
            )
            .unwrap();
        let _b = registry
            .register_repo(
                &repo_b,
                Some("B"),
                Some("core"),
                &vec!["apply-pilot".to_string()],
            )
            .unwrap();
        let _c = registry
            .register_repo(
                &repo_c,
                Some("C"),
                Some("core"),
                &vec!["apply-pilot".to_string()],
            )
            .unwrap();

        let req = BranchRunRequest {
            action: "status".to_string(),
            branch: None,
            base_branch: Some("main".to_string()),
            dry_run: Some(true),
            group: Some("core".to_string()),
            tags: vec!["apply-pilot".to_string()],
            selected_repo_ids: vec![a.id],
            preview_token: None,
            confirm_phrase: None,
        };
        let roots = vec![PathBuf::from(&scope_root)];
        let targets = resolve_branch_targets(&registry, &req, &roots).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "A");

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn test_resolve_branch_targets_selected_ids_ignore_group_filter() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("pilot_branch_targets_group_test_{}", nanos));
        let scope_root = base.join("scope");
        fs::create_dir_all(&scope_root).unwrap();
        let repo_a = scope_root.join("A");
        fs::create_dir_all(&repo_a).unwrap();

        let db_path = base.join("workspace.db");
        let registry = multi::MultiRegistry::open(&db_path).unwrap();
        let a = registry
            .register_repo(&repo_a, Some("A"), Some("agorg"), &vec![])
            .unwrap();

        let req = BranchRunRequest {
            action: "create".to_string(),
            branch: Some("feat/test".to_string()),
            base_branch: Some("main".to_string()),
            dry_run: Some(false),
            group: Some("core".to_string()),
            tags: vec!["apply-pilot".to_string()],
            selected_repo_ids: vec![a.id],
            preview_token: None,
            confirm_phrase: None,
        };
        let roots = vec![PathBuf::from(&scope_root)];
        let targets = resolve_branch_targets(&registry, &req, &roots).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "A");

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn test_sorted_helpers() {
        let tags = sorted_unique_tags(&[
            "beta".to_string(),
            " alpha ".to_string(),
            "beta".to_string(),
            "".to_string(),
        ]);
        assert_eq!(tags, vec!["alpha".to_string(), "beta".to_string()]);
        let ids = sorted_unique_ids(&[4, 2, 4, 3, 2]);
        assert_eq!(ids, vec![2, 3, 4]);
    }

    #[test]
    fn test_canonical_branch_payload_normalizes_fields() {
        let req = BranchRunRequest {
            action: "Create".to_string(),
            branch: Some(" feat/x ".to_string()),
            base_branch: Some(" dev ".to_string()),
            dry_run: Some(true),
            group: Some(" core ".to_string()),
            tags: vec!["b".to_string(), "a".to_string(), "b".to_string()],
            selected_repo_ids: vec![3, 1, 3],
            preview_token: Some("tok".to_string()),
            confirm_phrase: None,
        };
        let v = canonical_branch_payload(&req, false);
        assert_eq!(v["action"], "create");
        assert_eq!(v["branch"], "feat/x");
        assert_eq!(v["base_branch"], "dev");
        assert_eq!(v["dry_run"], false);
        assert_eq!(v["group"], "core");
        assert_eq!(v["tags"], json!(["a", "b"]));
        assert_eq!(v["selected_repo_ids"], json!([1, 3]));
        assert!(v.get("preview_token").is_none());
    }

    #[test]
    fn test_prune_expired_branch_previews() {
        let mut map = HashMap::new();
        map.insert(
            "old".to_string(),
            BranchPreviewRecord {
                token: "old".to_string(),
                scope_id: Uuid::nil(),
                action: "create".to_string(),
                expected_execute_payload: json!({}),
                created_at_unix: 10,
                expires_at_unix: 20,
            },
        );
        map.insert(
            "new".to_string(),
            BranchPreviewRecord {
                token: "new".to_string(),
                scope_id: Uuid::nil(),
                action: "create".to_string(),
                expected_execute_payload: json!({}),
                created_at_unix: 30,
                expires_at_unix: 100,
            },
        );
        prune_expired_branch_previews(&mut map, 50);
        assert!(!map.contains_key("old"));
        assert!(map.contains_key("new"));
    }

    // Legacy branch policy tests were removed because they are now unit tested within eval.rs

    // test_branch_policy_violation was removed because it is now unit tested within eval.rs
}

async fn get_dependency_logs() -> Response {
    match read_recent_gate_logs(4, 20_000) {
        Ok(logs) => Json(json!({"ok": true, "logs": logs})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
}

async fn get_temporary_components() -> Response {
    let payload = match build_temporary_components_payload().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to build temporary component inventory: {}", err),
            )
        }
    };
    Json(payload).into_response()
}

async fn get_temporary_components_checklist() -> Response {
    let inventory = match build_temporary_components_payload().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to build temporary component inventory: {}", err),
            )
        }
    };
    let components = inventory
        .get("components")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let shim_component = components
        .iter()
        .find(|c| c.get("id").and_then(Value::as_str) == Some("arqonbus_compat_shim"));
    let hierarchy_component = components
        .iter()
        .find(|c| c.get("id").and_then(Value::as_str) == Some("hierarchy_drag_link_editor"));
    let shim_status = shim_component
        .and_then(|c| c.get("status").and_then(Value::as_str))
        .unwrap_or("unknown");
    let hierarchy_status = hierarchy_component
        .and_then(|c| c.get("status").and_then(Value::as_str))
        .unwrap_or("unknown");
    let shim_exit_code_present = shim_component
        .and_then(|c| c.get("details"))
        .and_then(|d| d.get("exit_code"))
        .and_then(Value::as_i64)
        .is_some();
    let hierarchy_api_present = hierarchy_component
        .and_then(|c| c.get("details"))
        .and_then(|d| d.get("api"))
        .and_then(Value::as_str)
        .map(|v| v == "/api/agorg/edit_relationship")
        .unwrap_or(false);
    let exit_criteria_present = components.iter().all(|c| {
        c.get("exit_criteria")
            .and_then(Value::as_str)
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    });
    let component_ids_present = components
        .iter()
        .any(|c| c.get("id").and_then(Value::as_str) == Some("arqonbus_compat_shim"))
        && components
            .iter()
            .any(|c| c.get("id").and_then(Value::as_str) == Some("hierarchy_drag_link_editor"));

    let checks = vec![
        json!({
            "id": "inventory_api_available",
            "label": "Inventory API is available",
            "required": true,
            "pass": true,
            "details": "GET /api/system/temporary_components responded."
        }),
        json!({
            "id": "shim_status_detected",
            "label": "ArqonBus shim status is detectable",
            "required": true,
            "pass": shim_status != "unknown",
            "details": format!("shim_status={}", shim_status)
        }),
        json!({
            "id": "shim_detail_contract",
            "label": "ArqonBus shim detail contract is complete",
            "required": true,
            "pass": shim_exit_code_present,
            "details": "shim details include exit_code for deterministic triage."
        }),
        json!({
            "id": "hierarchy_editor_contract",
            "label": "Hierarchy editor fallback contract is explicit",
            "required": true,
            "pass": hierarchy_status == "manual-editor-active" && hierarchy_api_present,
            "details": format!(
                "hierarchy_status={}, api_present={}",
                hierarchy_status, hierarchy_api_present
            )
        }),
        json!({
            "id": "component_exit_criteria_present",
            "label": "Temporary components define exit criteria",
            "required": true,
            "pass": exit_criteria_present,
            "details": format!("all_components_have_exit_criteria={}", exit_criteria_present)
        }),
        json!({
            "id": "required_component_ids_present",
            "label": "Required temporary component IDs are present",
            "required": true,
            "pass": component_ids_present,
            "details": "arqonbus_compat_shim + hierarchy_drag_link_editor found."
        }),
    ];
    let overall_pass = checks.iter().all(|c| {
        !c.get("required").and_then(Value::as_bool).unwrap_or(false)
            || c.get("pass").and_then(Value::as_bool).unwrap_or(false)
    });
    Json(json!({
        "ok": true,
        "overall_pass": overall_pass,
        "checks": checks,
        "inventory": inventory
    }))
    .into_response()
}

async fn run_acceptance_matrix(
    State(state): State<Arc<UiState>>,
    Json(req): Json<AcceptanceMatrixRequest>,
) -> Response {
    let wave = req.wave.unwrap_or_else(|| "I".to_string());
    let profile = req.profile.unwrap_or_else(|| "quick".to_string());
    let cmd = format!(
        "./scripts/wave_acceptance_matrix.sh --wave {} --profile {}",
        wave, profile
    );
    let (exit_code, stdout, stderr) = match run_local_script(&cmd).await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("acceptance matrix execution failed: {}", err),
            )
        }
    };
    let parsed: Value = match parse_json_from_mixed_output(&stdout) {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!(
                    "acceptance matrix output was not valid JSON: {}; stderr={}",
                    err, stderr
                ),
            )
        }
    };
    let path = match persist_acceptance_matrix_report(&parsed, &wave, &profile) {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to persist acceptance matrix artifact: {}", err),
            )
        }
    };
    let overall_ok = parsed.get("ok").and_then(Value::as_bool).unwrap_or(false) && exit_code == 0;
    let _ = state.events.send(json!({
        "source": "acceptance_matrix",
        "action": "run",
        "wave": wave,
        "profile": profile,
        "ok": overall_ok,
        "artifact_path": path
    }));
    Json(json!({
        "ok": overall_ok,
        "wave": wave,
        "profile": profile,
        "exit_code": exit_code,
        "artifact_path": path,
        "result": parsed
    }))
    .into_response()
}

async fn export_temporary_components_inventory(State(state): State<Arc<UiState>>) -> Response {
    let payload = match build_temporary_components_payload().await {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to build temporary component inventory: {}", err),
            )
        }
    };
    let path = match persist_temporary_components_inventory_report(&payload) {
        Ok(v) => v,
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to persist temporary component inventory: {}", err),
            )
        }
    };
    let _ = state.events.send(json!({
        "source": "temporary_components",
        "action": "export",
        "ok": true,
        "artifact_path": path
    }));
    Json(json!({
        "ok": true,
        "path": path,
        "inventory": payload
    }))
    .into_response()
}

async fn build_temporary_components_payload() -> std::io::Result<Value> {
    let (shim_code, shim_stdout, shim_stderr) =
        match run_local_script(&bus_shim_command("status")).await {
            Ok(v) => v,
            Err(err) => (-1, String::new(), err.to_string()),
        };
    let shim_running = shim_code == 0 && bus_shim_running(&shim_stdout, &shim_stderr);
    let components = vec![
        json!({
            "id": "arqonbus_compat_shim",
            "name": "ArqonBus compatibility shim",
            "type": "shim",
            "required": true,
            "status": if shim_running { "running" } else { "stopped" },
            "detection_command": bus_shim_command("status"),
            "details": {
                "exit_code": shim_code,
                "stdout": shim_stdout,
                "stderr": shim_stderr,
            },
            "exit_criteria": "Native ArqonBus runtime integration available in frozen fleet checkout."
        }),
        json!({
            "id": "hierarchy_drag_link_editor",
            "name": "Hierarchy drag/link editor",
            "type": "ux_gap",
            "required": false,
            "status": "manual-editor-active",
            "detection_command": "UI AGOrg panel + /api/agorg/edit_relationship",
            "details": {
                "current_path": "Use explicit parent + relationship editor actions; drag-and-drop linking is intentionally disabled.",
                "api": "/api/agorg/edit_relationship"
            },
            "exit_criteria": "Audited drag-link UX lands with deterministic mutation preview + apply."
        }),
    ];
    Ok(json!({
        "ok": true,
        "count": components.len(),
        "components": components,
    }))
}

async fn stream_events(
    State(state): State<Arc<UiState>>,
) -> Sse<impl tokio_stream::Stream<Item = std::result::Result<Event, Infallible>>> {
    let rx = state.events.subscribe();
    let state_for_stream = state.clone();
    let stream = BroadcastStream::new(rx).filter_map(move |item| {
        let state = state_for_stream.clone();
        async move {
            match item {
                Ok(value) => {
                    let active_scope = state.agorg_store.get_active_agorg().await.ok().flatten();
                    let tagged = with_event_agorg_scope(value, active_scope.as_ref());
                    Some(Ok(Event::default()
                        .event("pilot_event")
                        .data(tagged.to_string())))
                }
                Err(_) => None,
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

fn read_recent_audit_events(limit: usize) -> std::io::Result<Vec<Value>> {
    let path = audit_path();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in content.lines().rev().take(limit) {
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            out.push(v);
        }
    }
    out.reverse();
    Ok(out)
}

fn audit_path() -> PathBuf {
    pilot_data_root().join("audit.jsonl")
}

fn reports_root() -> PathBuf {
    let preferred = pilot_data_root().join("reports");
    if fs::create_dir_all(&preferred).is_ok() {
        return preferred;
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let fallback = cwd.join(".pilot").join("reports");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

fn pilot_data_root() -> PathBuf {
    if let Ok(root) = std::env::var("PILOT_HOME") {
        let pb = PathBuf::from(root);
        let _ = fs::create_dir_all(&pb);
        return pb;
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let preferred = PathBuf::from(home).join(".pilot");
    if fs::create_dir_all(&preferred).is_ok() {
        return preferred;
    }

    // Fallback for constrained/sandboxed contexts where HOME is not writable.
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let fallback = cwd.join(".pilot");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

fn agorg_policy_report_path(ts: &str) -> PathBuf {
    reports_root().join(format!("agorg_policy_report_{}.json", ts))
}

fn agorg_reconcile_action_report_path(ts: &str, mode: &str) -> PathBuf {
    reports_root().join(format!("agorg_reconcile_{}_{}.json", mode, ts))
}

/// Governance-specific sidecar artifact: inheritance chain, override registry, per-AGO compliance.
/// Written alongside the main reconcile artifact for every dry-run and apply call.
fn governance_reconcile_artifact_path(ts: &str, mode: &str) -> PathBuf {
    reports_root().join(format!("governance_reconcile_{}_{}.json", mode, ts))
}

fn temporary_components_inventory_report_path(ts: &str) -> PathBuf {
    reports_root().join(format!("temporary_components_inventory_{}.json", ts))
}

fn acceptance_matrix_report_path(ts: &str, wave: &str, profile: &str) -> PathBuf {
    reports_root().join(format!(
        "acceptance_matrix_wave_{}_{}_{}.json",
        wave.to_lowercase(),
        profile.to_lowercase(),
        ts
    ))
}

fn now_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    secs.to_string()
}

fn persist_agorg_policy_report(report: &agorg::AgorgReconcileReport) -> std::io::Result<String> {
    let path = agorg_policy_report_path(&now_stamp());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    fs::write(&path, payload)?;
    Ok(path.display().to_string())
}

fn persist_agorg_reconcile_action_report(mode: &str, payload: &Value) -> std::io::Result<String> {
    let ts = now_stamp();
    let path = agorg_reconcile_action_report_path(&ts, mode);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(payload)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    fs::write(&path, body)?;

    // Write sidecar governance artifact if the payload includes a fleet_report.
    // This surfaces inheritance chains and per-AGO compliance independently of the main report.
    // G-043: write errors here are non-fatal — reconcile must not fail on artifact write.
    if let Some(fleet_report) = payload.get("fleet_report") {
        let gov_path = governance_reconcile_artifact_path(&ts, mode);
        let governance_artifact = serde_json::json!({
            "mode": mode,
            "timestamp": ts,
            "fleet_report": fleet_report,
            "governance_issues": payload.get("governance_issues"),
            "conflict_traces": payload.get("conflict_traces")
        });
        match serde_json::to_string_pretty(&governance_artifact) {
            Ok(gov_body) => {
                if let Err(e) = fs::write(&gov_path, gov_body) {
                    eprintln!(
                        "Warning [G-043]: governance artifact write failed at {}: {}",
                        gov_path.display(),
                        e
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning [G-043]: governance artifact serialization failed: {}",
                    e
                );
            }
        }
    }

    Ok(path.display().to_string())
}

fn persist_temporary_components_inventory_report(payload: &Value) -> std::io::Result<String> {
    let path = temporary_components_inventory_report_path(&now_stamp());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(payload)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    fs::write(&path, body)?;
    Ok(path.display().to_string())
}

fn parse_json_from_mixed_output(stdout: &str) -> Result<Value, serde_json::Error> {
    if let Ok(v) = serde_json::from_str::<Value>(stdout) {
        return Ok(v);
    }
    if let Some(start) = stdout.find('{') {
        let tail = &stdout[start..];
        if let Ok(v) = serde_json::from_str::<Value>(tail) {
            return Ok(v);
        }
        if let Some(end) = tail.rfind('}') {
            let candidate = &tail[..=end];
            if let Ok(v) = serde_json::from_str::<Value>(candidate) {
                return Ok(v);
            }
        }
    }
    serde_json::from_str(stdout)
}

fn persist_acceptance_matrix_report(
    payload: &Value,
    wave: &str,
    profile: &str,
) -> std::io::Result<String> {
    let path = acceptance_matrix_report_path(&now_stamp(), wave, profile);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(payload)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    fs::write(&path, body)?;
    Ok(path.display().to_string())
}

fn list_agorg_policy_reports(limit: usize) -> std::io::Result<Vec<Value>> {
    let root = reports_root();
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = fs::read_dir(&root)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("agorg_policy_report_") && n.ends_with(".json"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    files.reverse();
    files.truncate(limit);
    Ok(files
        .iter()
        .map(|path| {
            let rel = path
                .strip_prefix(&root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            json!({ "path": rel, "name": name })
        })
        .collect())
}

fn list_report_files(limit: usize) -> std::io::Result<Vec<Value>> {
    let root = reports_root();
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut rows: Vec<(std::time::SystemTime, Value)> = Vec::new();
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let md = entry.metadata()?;
        let modified = md.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        rows.push((
            modified,
            json!({
                "path": rel,
                "size_bytes": md.len(),
                "modified_unix": modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }),
        ));
    }

    rows.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(rows.into_iter().take(limit).map(|(_, v)| v).collect())
}

fn read_report_file(path: &str, max_bytes: usize) -> std::io::Result<String> {
    let root = reports_root();
    if !root.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "reports directory does not exist",
        ));
    }

    let root_canon = root.canonicalize()?;
    let requested = root.join(path);
    let requested_canon = requested.canonicalize()?;
    if !requested_canon.starts_with(&root_canon) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "report path must stay within ~/.pilot/reports",
        ));
    }

    let bytes = std::fs::read(requested_canon)?;
    let clipped = if bytes.len() > max_bytes {
        bytes[..max_bytes].to_vec()
    } else {
        bytes
    };
    Ok(String::from_utf8_lossy(&clipped).to_string())
}

pub async fn run_local_script(cmd: &str) -> std::io::Result<(i32, String, String)> {
    // Keep profile isolation, but preserve a practical PATH for system tools
    // used by helper scripts (e.g., `ss` in /usr/sbin, `awk` in /usr/bin).
    let mut path = std::env::var("PATH").unwrap_or_default();
    if path.is_empty() {
        path = "/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin:/sbin".to_string();
    }
    for extra in [
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/local/sbin",
        "/usr/sbin",
        "/sbin",
    ] {
        let needle = format!("{extra}:");
        if !path.starts_with(extra) && !path.contains(&needle) && !path.ends_with(extra) {
            path.push(':');
            path.push_str(extra);
        }
    }
    let child = TokioCommand::new("bash")
        .arg("--noprofile")
        .arg("--norc")
        .arg("-lc")
        .arg(cmd)
        .env("PATH", path)
        .output()
        .await?;
    let code = child.status.code().unwrap_or(-1);
    let out = String::from_utf8_lossy(&child.stdout).to_string();
    let err = String::from_utf8_lossy(&child.stderr).to_string();
    Ok((code, out, err))
}

async fn read_stream_lines<R>(
    reader: R,
    action: &str,
    stream: &str,
    events: &broadcast::Sender<Value>,
) -> String
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = tokio::io::BufReader::new(reader).lines();
    let mut buf = String::new();
    while let Ok(Some(line)) = lines.next_line().await {
        buf.push_str(&line);
        buf.push('\n');
        let _ = events.send(json!({
            "source": "dependency_action_progress",
            "action": action,
            "stream": stream,
            "line": line
        }));
    }
    buf
}

async fn run_local_script_streamed(
    cmd: &str,
    action: &str,
    events: &broadcast::Sender<Value>,
) -> std::io::Result<(i32, String, String)> {
    let mut path = std::env::var("PATH").unwrap_or_default();
    if path.is_empty() {
        path = "/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin:/sbin".to_string();
    }
    for extra in [
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/local/sbin",
        "/usr/sbin",
        "/sbin",
    ] {
        let needle = format!("{extra}:");
        if !path.starts_with(extra) && !path.contains(&needle) && !path.ends_with(extra) {
            path.push(':');
            path.push_str(extra);
        }
    }

    let mut child = TokioCommand::new("bash")
        .arg("--noprofile")
        .arg("--norc")
        .arg("-lc")
        .arg(cmd)
        .env("PATH", path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "failed to capture stdout")
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "failed to capture stderr")
    })?;

    let action_out = action.to_string();
    let action_err = action.to_string();
    let events_out = events.clone();
    let events_err = events.clone();
    let out_task =
        tokio::spawn(
            async move { read_stream_lines(stdout, &action_out, "stdout", &events_out).await },
        );
    let err_task =
        tokio::spawn(
            async move { read_stream_lines(stderr, &action_err, "stderr", &events_err).await },
        );

    let status = child.wait().await?;
    let out = out_task.await.unwrap_or_default();
    let err = err_task.await.unwrap_or_default();
    let code = status.code().unwrap_or(-1);
    Ok((code, out, err))
}

fn parse_push_main_summary(stdout: &str, stderr: &str) -> Option<Value> {
    let combined = format!("{stdout}\n{stderr}");
    let mut in_block = false;
    let mut summary = serde_json::Map::new();
    for raw in combined.lines() {
        let line = raw.trim();
        if line == "========== push_main summary ==========" {
            in_block = true;
            continue;
        }
        if line == "=======================================" {
            break;
        }
        if !in_block {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            summary.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
        }
    }
    if summary.is_empty() {
        None
    } else {
        Some(Value::Object(summary))
    }
}

fn parse_release_collect_evidence_path(stdout: &str, stderr: &str) -> Option<String> {
    let combined = format!("{stdout}\n{stderr}");
    let mut next_path = false;
    for raw in combined.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("/home/") && line.contains("/.pilot/release_evidence/") {
            return Some(line.to_string());
        }
        if line.starts_with("Release evidence collected and manifest generated at:") {
            next_path = true;
            continue;
        }
        if next_path && line.starts_with('/') {
            return Some(line.to_string());
        }
    }
    None
}

fn parse_gh_watch_summary(stdout: &str, stderr: &str) -> Option<Value> {
    let combined = format!("{stdout}\n{stderr}");
    let mut in_block = false;
    let mut summary = serde_json::Map::new();
    for raw in combined.lines() {
        let line = raw.trim();
        if line == "========== gh_watch summary ==========" {
            in_block = true;
            continue;
        }
        if line == "======================================" {
            break;
        }
        if !in_block {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            summary.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
        }
    }
    if summary.is_empty() {
        None
    } else {
        Some(Value::Object(summary))
    }
}

fn parse_gh_status_summary(stdout: &str, stderr: &str) -> Option<Value> {
    let combined = format!("{stdout}\n{stderr}");
    let mut in_block = false;
    let mut summary = serde_json::Map::new();
    for raw in combined.lines() {
        let line = raw.trim();
        if line == "========== gh_status summary ==========" {
            in_block = true;
            continue;
        }
        if line == "======================================" {
            break;
        }
        if !in_block {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            summary.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
        }
    }
    if summary.is_empty() {
        None
    } else {
        Some(Value::Object(summary))
    }
}

fn parse_prepush_summary(stdout: &str, stderr: &str) -> Option<Value> {
    let combined = format!("{stdout}\n{stderr}");
    let mut status = None::<String>;
    let mut log_file = None::<String>;
    for raw in combined.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("[pre-push] status:") {
            status = Some(rest.trim().to_string());
        }
        if let Some(rest) = line.strip_prefix("[pre-push] log file:") {
            log_file = Some(rest.trim().to_string());
        }
    }
    if status.is_none() && log_file.is_none() {
        return None;
    }
    Some(json!({
        "status": status.unwrap_or_else(|| "unknown".to_string()),
        "log_file": log_file
    }))
}

fn read_recent_gate_logs(limit: usize, tail_bytes: usize) -> std::io::Result<Vec<Value>> {
    let mut roots = Vec::new();
    roots.push(reports_root());
    roots.push(PathBuf::from("/tmp/pilot-reports"));

    let mut files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if !name.starts_with("prepush_gate_") || !name.ends_with(".log") {
                continue;
            }
            let md = entry.metadata()?;
            files.push((
                md.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                path,
            ));
        }
    }
    files.sort_by(|a, b| b.0.cmp(&a.0));

    let mut out = Vec::new();
    for (_, path) in files.into_iter().take(limit) {
        let content = std::fs::read(&path)?;
        let start = content.len().saturating_sub(tail_bytes);
        let tail = String::from_utf8_lossy(&content[start..]).to_string();
        out.push(json!({
            "path": path.display().to_string(),
            "tail": tail
        }));
    }
    Ok(out)
}

fn error_response(status: StatusCode, message: &str) -> Response {
    let mut response = Json(json!({"ok": false, "error": message})).into_response();
    *response.status_mut() = status;
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

fn spawn_bus_telemetry_listener(bus: BusBridgeConfig, tx: broadcast::Sender<Value>) {
    tokio::spawn(async move {
        let mut last_error = String::new();
        let mut last_emit = std::time::Instant::now() - Duration::from_secs(60);
        loop {
            if let Err(err) = consume_bus_telemetry(&bus, &tx).await {
                let msg = err.to_string();
                let should_emit =
                    msg != last_error || last_emit.elapsed() >= Duration::from_secs(30);
                if should_emit {
                    let _ = tx.send(json!({
                        "source": "bus_listener",
                        "error": msg,
                        "hint": "Verify ArqonBus is running and reachable at configured --ws-url"
                    }));
                    last_error = err.to_string();
                    last_emit = std::time::Instant::now();
                }

                // Attempt auto-start if mutations allowed and it's been down for a few loops
                if let Ok(_) = std::env::var("PILOT_AUTO_START_BUS") {
                    // We would ideally call into a bus management system here
                    // For now, we spawn the 'pilot bus start' command as a repair attempt
                    let _ = tokio::process::Command::new("pilot")
                        .args(["bus", "start"])
                        .spawn();
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

async fn consume_bus_telemetry(bus: &BusBridgeConfig, tx: &broadcast::Sender<Value>) -> Result<()> {
    let (ws_stream, _) = connect_async(&bus.ws_url).await.into_diagnostic()?;
    let (mut writer, mut reader) = ws_stream.split();

    if let Ok(token) = std::env::var(&bus.jwt_env) {
        let auth = json!({
            "type": "command",
            "command": "authenticate",
            "args": {"token": token},
            "room": bus.room,
            "channel": bus.telemetry_channel,
        });
        writer
            .send(Message::Text(auth.to_string()))
            .await
            .into_diagnostic()?;
    }

    let join = json!({
        "type": "command",
        "command": "join_channel",
        "args": {"channel_id": bus.telemetry_channel},
        "room": bus.room,
        "channel": bus.telemetry_channel,
    });
    writer
        .send(Message::Text(join.to_string()))
        .await
        .into_diagnostic()?;

    while let Some(msg) = reader.next().await {
        let msg = msg.into_diagnostic()?;
        let Message::Text(text) = msg else {
            continue;
        };
        let parsed: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let is_telemetry = parsed
            .get("channel")
            .and_then(Value::as_str)
            .map(|c| c == bus.telemetry_channel)
            .unwrap_or(false)
            || parsed
                .get("eventType")
                .and_then(Value::as_str)
                .map(|e| e.starts_with("pilot."))
                .unwrap_or(false)
            || parsed
                .get("payload")
                .and_then(|p| p.get("event_type"))
                .and_then(Value::as_str)
                .map(|e| e.starts_with("pilot."))
                .unwrap_or(false);

        if is_telemetry {
            let _ = tx.send(parsed);
        }
    }

    Ok(())
}

async fn send_command_once_with_retry(
    bus: &BusBridgeConfig,
    command: &str,
    payload: Value,
    max_attempts: u32,
) -> Result<Value> {
    let attempts = max_attempts.max(1);
    for attempt in 1..=attempts {
        match send_command_once(bus, command, payload.clone()).await {
            Ok(v) => return Ok(v),
            Err(err) if attempt < attempts => {
                let backoff_ms = 200u64 * (1 << (attempt - 1));
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                let _ = err;
            }
            Err(err) => return Err(err),
        }
    }

    Err(miette::miette!(
        "unreachable retry state for command {}",
        command
    ))
}

fn is_mutating_command(command: &str) -> bool {
    matches!(
        command,
        "pilot.branch.create"
            | "pilot.branch.sync"
            | "pilot.branch.prune"
            | "pilot.multi.register"
            | "pilot.multi.apply"
            | "pilot.multi.prs.create"
            | "pilot.heal.run"
    )
}

fn payload_truthy_bool(payload: &Value, key: &str) -> bool {
    payload.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn command_requires_mutation(command: &str, payload: &Value) -> bool {
    match command {
        "pilot.multi.apply" => payload_truthy_bool(payload, "apply"),
        "pilot.heal.run" => !payload_truthy_bool(payload, "plan_only"),
        "api.agorg.reconcile_apply" => !payload_truthy_bool(payload, "dry_run"),
        "pilot.branch.create"
        | "pilot.branch.sync"
        | "pilot.branch.prune"
        | "pilot.multi.prs.create" => !payload_truthy_bool(payload, "dry_run"),
        _ => is_mutating_command(command),
    }
}

fn command_scope_required(command: &str) -> bool {
    command.starts_with("pilot.branch.")
        || command.starts_with("pilot.multi.")
        || command.starts_with("pilot.oracle.")
        || command.starts_with("pilot.heal.")
        || command.starts_with("pilot.navigate.")
}

fn command_requires_cwd_scope(command: &str) -> bool {
    command.starts_with("pilot.branch.")
        || command.starts_with("pilot.oracle.")
        || command.starts_with("pilot.heal.")
        || command.starts_with("pilot.navigate.")
}

fn dependency_action_scope_required(action: &str) -> bool {
    matches!(
        action,
        "policy"
            | "hook-policy"
            | "drift"
            | "gate"
            | "prepush-gate"
            | "repair"
            | "cargo-fmt"
            | "push"
            | "release-readiness"
            | "release-compat-matrix"
            | "release-migration-smoke"
            | "release-collect-evidence"
            | "release-verify-bundle"
            | "ci-trigger"
            | "ci-watch"
            | "ci-status"
    )
}

fn dependency_action_requires_cwd_scope(action: &str) -> bool {
    let _ = action;
    // Dependency actions are control-plane operations and must be runnable
    // regardless of where the Arqon Pilot process was started from.
    // AGOrg selection is still required, but cwd path coupling is disabled.
    false
}

fn command_requires_multi_selector(command: &str) -> bool {
    command.starts_with("pilot.multi.")
}

fn payload_has_multi_selector(payload: &Value) -> bool {
    let group_ok = payload
        .get("group")
        .and_then(|v| v.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let tags_ok = payload
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str())
                .any(|s| !s.trim().is_empty())
        })
        .unwrap_or(false);
    group_ok || tags_ok
}

fn canonicalize_path_lossy(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

fn enforce_dry_run(command: &str, payload: &mut Value) {
    if matches!(
        command,
        "pilot.branch.create"
            | "pilot.branch.sync"
            | "pilot.branch.prune"
            | "pilot.multi.prs.create"
    ) {
        payload["dry_run"] = json!(true);
    }
    if command == "pilot.multi.apply" {
        payload["apply"] = json!(false);
    }
    if command == "pilot.heal.run" {
        payload["plan_only"] = json!(true);
    }
}

fn with_event_agorg_scope(value: Value, active_scope: Option<&agorg::Agorg>) -> Value {
    let mut value = value;
    if let Value::Object(ref mut map) = value {
        if !map.contains_key("agorg_scope") {
            let scope_value = active_scope
                .map(|scope| {
                    json!({
                        "id": scope.id.to_string(),
                        "name": scope.name,
                        "root_path": scope.root_path
                    })
                })
                .unwrap_or(Value::Null);
            map.insert("agorg_scope".to_string(), scope_value);
        }
    }
    value
}

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <link rel="icon" type="image/x-icon" href="/favicon.ico" />
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@300;400;500;700&display=swap" rel="stylesheet" />
  <title>Pilot Control Panel</title>
  <style>
    :root {
      color-scheme: dark;
      --bg-deep: #06080B;
      --bg-mid: #0B0F14;
      --bg-light: #161C24;
      --border: rgba(255, 255, 255, 0.06);
      --border-hover: rgba(255, 255, 255, 0.15);
      --text: #e2e8f0;
      --muted: #94A3B8;
      --dim: #475569;
      --primary: #00F5FF;
      --primary-dim: rgba(0, 245, 255, 0.15);
      --accent: #FFD700;
      --rose: #FF2E2E;
      --glass-bg: linear-gradient(180deg, rgba(255, 255, 255, 0.03) 0%, rgba(255, 255, 255, 0.01) 100%);
      --glass-border: rgba(255, 255, 255, 0.05);
      
      --gov-inherited: #7C3AED;
      --gov-override: #F59E0B;
      --gov-conflict: #EF4444;
    }
    * { box-sizing: border-box; margin: 0; }

    ::-webkit-scrollbar { width: 4px; height: 4px; }
    ::-webkit-scrollbar-track { background: transparent; }
    ::-webkit-scrollbar-thumb { background: var(--primary-dim); border-radius: 999px; }
    ::-webkit-scrollbar-thumb:hover { background: rgba(0, 245, 255, 0.4); }

    body {
      margin: 0;
      color: var(--text);
      font-family: 'Inter', system-ui, -apple-system, sans-serif;
      background: var(--bg-deep);
      min-height: 100vh;
      overflow-x: hidden;
      -webkit-font-smoothing: antialiased;
      -moz-osx-font-smoothing: grayscale;
    }
    ::selection { background: rgba(0, 245, 255, 0.25); color: #fff; }

    /* ═══════════ Animated Orb Background ═══════════ */
    .bg-orbs { position: fixed; inset: 0; pointer-events: none; z-index: 0; overflow: hidden; }
    .bg-orbs::before, .bg-orbs::after {
      content: '';
      position: absolute;
      border-radius: 50%;
      filter: blur(120px);
      animation: orbFloat 8s ease-in-out infinite;
    }
    .bg-orbs::before {
      width: 600px; height: 600px;
      top: -15%; left: 15%;
      background: rgba(99, 60, 255, 0.12);
    }
    .bg-orbs::after {
      width: 500px; height: 500px;
      bottom: -10%; right: 10%;
      background: rgba(0, 245, 255, 0.06);
      animation-delay: 4s;
    }
    .orb-accent {
      position: absolute;
      border-radius: 50%;
      filter: blur(100px);
      animation: orbFloat 10s ease-in-out infinite;
      animation-delay: 2s;
      width: 400px; height: 400px;
      top: 40%; right: 30%;
      background: rgba(139, 92, 246, 0.08);
    }
    @keyframes orbFloat {
      0%, 100% { transform: translate(0, 0) scale(1); opacity: 1; }
      50% { transform: translate(30px, -20px) scale(1.1); opacity: 0.7; }
    }

    /* ═══════════ Layout ═══════════ */
    .wrap {
      position: relative;
      z-index: 1;
      width: 100%;
      max-width: 100%;
      padding: 0;
    }

    /* ═══════════ Governance Tables ═══════════ */
    .gov-table { width: 100%; border-collapse: collapse; font-size: 0.85rem; margin-top: 12px; }
    .gov-table th { text-align: left; padding: 10px; border-bottom: 2px solid var(--border); color: var(--muted); text-transform: uppercase; font-size: 0.72rem; letter-spacing: 0.05em; }
    .gov-table td { padding: 10px; border-bottom: 1px solid var(--border); vertical-align: middle; }
    .gov-table tr:hover { background: rgba(255, 255, 255, 0.02); }
    
    .source-pill { display: inline-flex; align-items: center; padding: 2px 8px; border-radius: 4px; font-size: 0.75rem; font-weight: 500; }
    .source-pill.agorg { background: rgba(124, 58, 237, 0.15); color: #C4B5FD; border: 1px solid rgba(124, 58, 237, 0.3); }
    .source-pill.ago { background: rgba(245, 158, 11, 0.15); color: #FCD34D; border: 1px solid rgba(245, 158, 11, 0.3); }
    .source-pill.default { background: rgba(148, 163, 184, 0.1); color: var(--muted); border: 1px solid var(--border); }
    
    .override-tag { margin-left: 6px; color: var(--gov-override); font-size: 0.68rem; font-weight: 700; text-transform: uppercase; }
    .conflict-warning { color: var(--gov-conflict); font-weight: 600; font-size: 0.75rem; display: flex; align-items: center; gap: 4px; }
    .inheritance-trace { font-size: 0.78rem; color: var(--muted); border-left: 2px solid var(--gov-inherited); padding-left: 10px; margin: 8px 0; }

    /* ═══════════ Top Nav / Hero ═══════════ */
    .hero {
      position: sticky;
      top: 0;
      z-index: 50;
      border-bottom: 1px solid var(--border);
      background: rgba(6, 8, 11, 0.85);
      backdrop-filter: blur(16px) saturate(180%);
      padding: 16px 28px;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }
    .bus-status-row {
      margin-top: 10px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      flex-wrap: wrap;
      font-size: 0.86rem;
      color: #b8c8ef;
    }
    .status-left, .status-right {
      display: inline-flex;
      align-items: center;
      gap: 8px;
    }
    .status-right {
      border: 1px solid var(--border);
      background: rgba(255, 255, 255, 0.04);
      color: var(--muted);
      border-radius: 6px;
      padding: 4px 10px;
      cursor: pointer;
      transition: all 0.25s;
      font-size: 0.8rem;
    }
    .status-right:hover {
      border-color: var(--primary);
      color: var(--primary);
      box-shadow: 0 0 12px rgba(0, 245, 255, 0.15);
    }
    .bus-chip {
      border-radius: 4px;
      padding: 3px 8px;
      font-weight: 600;
      border: 1px solid;
      font-size: 0.7rem;
      font-family: 'JetBrains Mono', monospace;
      letter-spacing: 0.04em;
    }
    .bus-chip.connected {
      color: var(--primary);
      border-color: rgba(0, 245, 255, 0.25);
      background: rgba(0, 245, 255, 0.06);
      box-shadow: 0 0 8px rgba(0, 245, 255, 0.1);
    }
    .bus-chip.disconnected {
      color: var(--rose);
      border-color: rgba(255, 46, 46, 0.25);
      background: rgba(255, 46, 46, 0.06);
    }
    .agorg-chip.active {
      color: var(--primary);
      border-color: rgba(0, 245, 255, 0.25);
      background: rgba(0, 245, 255, 0.06);
    }
    .agorg-chip.none {
      color: var(--accent);
      border-color: rgba(255, 215, 0, 0.25);
      background: rgba(255, 215, 0, 0.06);
    }
    h1 { font-size: 1.1rem; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: #fff; }
    h1 .accent { color: var(--primary); }
    h2 { font-size: 0.7rem; color: var(--dim); font-weight: 400; font-family: 'JetBrains Mono', monospace; letter-spacing: 0.15em; text-transform: uppercase; }
    h3 { margin: 0 0 14px; font-size: 0.75rem; font-weight: 600; font-family: 'JetBrains Mono', monospace; text-transform: uppercase; letter-spacing: 0.1em; color: var(--muted); border-bottom: 1px solid var(--border); padding-bottom: 10px; }

    /* ═══════════ Tab Bar ═══════════ */
    .tabs {
      display: flex;
      gap: 0;
      border-bottom: 1px solid var(--border);
      padding: 0 28px;
      background: rgba(6, 8, 11, 0.6);
      backdrop-filter: blur(8px);
      overflow-x: auto;
      position: sticky;
      top: 56px;
      z-index: 40;
    }
    button.tab {
      background: transparent;
      border: none;
      border-bottom: 2px solid transparent;
      color: var(--dim);
      padding: 12px 18px;
      cursor: pointer;
      font-weight: 500;
      font-size: 0.8rem;
      transition: all 0.2s;
      white-space: nowrap;
    }
    button.tab:hover { color: var(--text); }
    button.tab.active {
      color: var(--primary);
      border-bottom-color: var(--primary);
      text-shadow: 0 0 10px rgba(0, 245, 255, 0.3);
    }

    /* ═══════════ Panels ═══════════ */
    .panel {
      display: none;
      padding: 28px;
      animation: fadeIn 0.25s ease-out;
    }
    @keyframes fadeIn { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: translateY(0); } }
    .panel.active { display: block; }
    /* ═══════════ Grid / Cards ═══════════ */
    .grid {
      display: grid;
      gap: 24px;
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .card {
      background: var(--glass-bg);
      backdrop-filter: blur(8px);
      border: 1px solid var(--glass-border);
      border-radius: 12px;
      padding: 20px;
      display: flex;
      flex-direction: column;
      gap: 14px;
      position: relative;
      overflow: hidden;
      transition: border-color 0.3s;
    }
    .card:hover { border-color: var(--border-hover); }
    .card::before {
      content: '';
      position: absolute;
      inset: 0;
      background: radial-gradient(circle at 50% -20%, rgba(0, 245, 255, 0.03), transparent 70%);
      pointer-events: none;
    }

    /* ═══════════ Section Box (Import panel groups) ═══════════ */
    .section-box {
      border: 1px solid var(--border);
      border-radius: 10px;
      padding: 18px;
      background: rgba(255, 255, 255, 0.015);
      display: flex;
      flex-direction: column;
      gap: 12px;
      position: relative;
    }
    .section-box h4 {
      margin: 0;
      font-size: 0.7rem;
      font-family: 'JetBrains Mono', monospace;
      text-transform: uppercase;
      letter-spacing: 0.12em;
      color: var(--primary);
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .section-box h4::before {
      content: '';
      width: 3px;
      height: 14px;
      background: var(--primary);
      border-radius: 2px;
      box-shadow: 0 0 6px rgba(0,245,255,0.4);
    }
    /* ═══════════ Inputs ═══════════ */
    input, select, textarea {
      width: 100%;
      background: rgba(0, 0, 0, 0.3);
      color: var(--text);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 10px 14px;
      font-size: 0.82rem;
      font-family: 'JetBrains Mono', monospace;
      transition: all 0.25s;
    }
    textarea {
      resize: vertical;
      min-height: 110px;
    }
    input::placeholder, textarea::placeholder { color: var(--dim); }
    input:focus, select:focus, textarea:focus, button:focus-visible, .action-chip:focus-visible {
      outline: 2px solid var(--accent);
      outline-offset: 2px;
      border-color: var(--accent);
    }
    
    .action-chip {
        cursor: pointer;
        font-family: monospace;
        padding: 4px 8px;
        margin-right: 4px;
        border-radius: 4px;
        color: var(--text);
        background: rgba(255,255,255,0.05);
        border: 1px solid var(--border);
    }
    .action-chip:hover {
        background: rgba(255,255,255,0.1);
        border-color: var(--text-muted);
    }
    select option {
      background: #0d1117; 
      color: #e6edf3;
    }

    /* ═══════════ Row / Buttons ═══════════ */
    .row {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      align-items: center;
    }
    .btn {
      background: linear-gradient(135deg, rgba(0, 245, 255, 0.15), rgba(99, 60, 255, 0.1));
      border: 1px solid rgba(0, 245, 255, 0.3);
      color: var(--primary);
      border-radius: 8px;
      padding: 9px 16px;
      cursor: pointer;
      font-weight: 600;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      transition: all 0.25s ease;
      position: relative;
      overflow: hidden;
    }
    .btn::before {
      content: '';
      position: absolute;
      inset: 0;
      background: linear-gradient(135deg, rgba(0,245,255,0.1), transparent);
      opacity: 0;
      transition: opacity 0.25s;
    }
    .btn:hover::before { opacity: 1; }
    .btn:hover { border-color: var(--primary); color: #fff; box-shadow: 0 0 12px rgba(0, 245, 255, 0.2); transform: translateY(-1px); }
    .btn:active { transform: translateY(0); }
    .btn.secondary { background: rgba(255, 255, 255, 0.04); border-color: var(--border); color: var(--muted); }
    .btn.secondary:hover { background: rgba(255, 255, 255, 0.08); border-color: var(--text-muted); color: var(--text); }
    
    /* ═══════════ Modals ═══════════ */
    .modal-overlay {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.8);
      backdrop-filter: blur(8px);
      z-index: 1000;
      display: none;
      align-items: center;
      justify-content: center;
      padding: 20px;
      animation: fadeIn 0.2s ease-out;
    }
    .modal-overlay.active { display: flex; }
    .modal-box {
      background: var(--bg-mid);
      border: 1px solid var(--border);
      border-radius: 16px;
      width: 100%;
      max-width: 550px;
      padding: 30px;
      box-shadow: 0 20px 50px rgba(0,0,0,0.5), 0 0 0 1px var(--glass-border);
      display: flex;
      flex-direction: column;
      gap: 20px;
      animation: modalSlideUp 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    }
    @keyframes modalSlideUp { from { opacity: 0; transform: translateY(20px) scale(0.98); } to { opacity: 1; transform: translateY(0) scale(1); } }
    @keyframes pulse-blue {
      0% { box-shadow: 0 0 5px rgba(0, 209, 255, 0.4); }
      50% { box-shadow: 0 0 15px rgba(0, 209, 255, 0.8); }
      100% { box-shadow: 0 0 5px rgba(0, 209, 255, 0.4); }
    }
    .agorg-reg-item.active-node {
      background: rgba(106, 125, 255, 0.15) !important;
      border-left: 3px solid #6a7dff !important;
    }

    /* ═══════════ Form Layout ═══════════ */
    .form-row {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-bottom: 2px;
    }
    .form-label {
      width: 60px;
      min-width: 60px;
      font-size: 0.72rem;
      font-family: 'JetBrains Mono', monospace;
      color: var(--muted);
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }
    .form-content {
      flex: 1;
      display: flex;
      gap: 8px;
    }
    .term-out {
      margin-top: 12px;
      background: rgba(0, 0, 0, 0.4);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 12px;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.75rem;
      color: #b8c8ef;
      white-space: pre-wrap;
      word-break: break-all;
      min-height: 100px;
      max-height: 300px;
      overflow-y: auto;
    }
    .agorg-reg-item.active-node .agorg-icon {
      filter: drop-shadow(0 0 5px #6a7dff);
    }
    .agorg-reg-item.active-node .agorg-name {
      color: #fff;
    }
    .agorg-reg-item.active-node .agorg-badge {
      background: #6a7dff !important;
      color: #001a33 !important;
    }
    .modal-box h3 { border: none; margin: 0; padding: 0; color: #fff; font-size: 1.1rem; }
    .modal-footer { margin-top: 10px; display: flex; justify-content: flex-end; gap: 12px; }
    .btn:hover {
      border-color: var(--primary);
      box-shadow: 0 0 20px rgba(0, 245, 255, 0.15), inset 0 0 20px rgba(0, 245, 255, 0.05);
      transform: translateY(-1px);
    }
    .btn:active { transform: translateY(0); }
    .btn.secondary {
      background: rgba(255, 255, 255, 0.03);
      border-color: var(--border);
      color: var(--muted);
    }
    .btn.secondary::before { display: none; }
    .btn.secondary:hover {
      border-color: var(--border-hover);
      color: var(--text);
      box-shadow: none;
      transform: none;
    }
    .btn.btn-jumbo {
      font-size: 1.25rem;
      padding: 18px 32px;
      font-weight: 700;
      letter-spacing: 0.08em;
    }
    
    /* Pulsating Background Glow for specific buttons */
    .btn-glow-wrap {
      position: relative;
      display: inline-flex;
    }
    .btn-glow-wrap::before {
      content: '';
      position: absolute;
      inset: -2px;
      background: rgba(0, 245, 255, 0.6);
      border-radius: 10px;
      filter: blur(8px);
      z-index: 0;
      animation: neonPulseBg 2s ease-in-out infinite alternate;
      pointer-events: none;
    }
    .btn-glow-wrap .btn {
      position: relative;
      z-index: 1;
      background: #06080b; /* make it opaque so glow is strictly behind */
    }
    @keyframes neonPulseBg {
      from { opacity: 0.3; filter: blur(6px); }
      to { opacity: 0.8; filter: blur(12px); transform: scale(1.02); }
    }

    /* ═══════════ Labels / Helpers ═══════════ */
    .field-label {
      font-size: 0.68rem;
      color: var(--dim);
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.1em;
      font-family: 'JetBrains Mono', monospace;
    }
    .helper {
      font-size: 0.78rem;
      color: var(--dim);
      line-height: 1.5;
    }
    .sequence-strip {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
      margin-bottom: 24px;
      padding: 10px 14px;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: rgba(0, 0, 0, 0.2);
    }
    .seq-step {
      border-radius: 4px;
      border: 1px solid var(--border);
      background: rgba(255, 255, 255, 0.03);
      color: var(--muted);
      font-size: 0.7rem;
      font-family: 'JetBrains Mono', monospace;
      padding: 4px 10px;
      white-space: nowrap;
    }
    .seq-step-btn {
      cursor: pointer;
      transition: all 0.2s ease;
    }
    .seq-step-btn:hover,
    .seq-step-btn:focus-visible {
      outline: 2px solid rgba(0, 245, 255, 0.8);
      outline-offset: 2px;
      border-color: rgba(0, 245, 255, 0.45);
      background: rgba(0, 245, 255, 0.08);
      color: var(--text);
      box-shadow: 0 0 0 2px rgba(0, 245, 255, 0.12);
    }

    /* ═══════════ Pre / Code ═══════════ */
    pre {
      margin: 0;
      background: rgba(0, 0, 0, 0.4);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 14px;
      max-height: 420px;
      overflow: auto;
      font-size: 0.78rem;
      line-height: 1.6;
      font-family: 'JetBrains Mono', monospace;
      color: var(--muted);
      white-space: pre-wrap;
      word-break: break-all;
    }

    /* ═══════════ Timeline ═══════════ */
    .timeline {
      display: flex;
      flex-direction: column;
      gap: 6px;
      padding-right: 4px;
    }
    .tl-card {
      border: 1px solid var(--border);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.02);
      padding: 12px;
      cursor: pointer;
      transition: all 0.25s;
      border-left: 2px solid transparent;
    }
    .tl-card:hover { border-left-color: rgba(0, 245, 255, 0.3); background: rgba(0, 245, 255, 0.02); }
    .tl-card.selected {
      border-left-color: var(--primary);
      background: rgba(0, 245, 255, 0.04);
    }
    .tl-head {
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 8px;
      margin-bottom: 4px;
    }
    .tl-title {
      font-size: 0.78rem;
      font-family: 'JetBrains Mono', monospace;
      font-weight: 500;
      color: var(--text);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .tl-badge {
      border-radius: 4px;
      font-size: 0.65rem;
      font-family: 'JetBrains Mono', monospace;
      font-weight: 600;
      padding: 2px 6px;
      border: 1px solid;
      flex-shrink: 0;
    }
    .tl-badge.started { color: var(--muted); border-color: var(--border); background: transparent; }
    .tl-badge.progress { color: var(--accent); border-color: rgba(255, 215, 0, 0.25); background: rgba(255, 215, 0, 0.06); }
    .tl-badge.completed { color: var(--primary); border-color: rgba(0, 245, 255, 0.25); background: rgba(0, 245, 255, 0.06); }
    .tl-badge.failed { color: var(--rose); border-color: rgba(255, 46, 46, 0.25); background: rgba(255, 46, 46, 0.06); }
    .tl-steps {
      margin: 0;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.7rem;
      color: var(--dim);
      display: flex;
      flex-direction: column;
      gap: 2px;
    }
    .tl-empty {
      color: var(--muted);
      font-size: 0.85rem;
      font-family: "JetBrains Mono", monospace;
      border: 1px dashed var(--border);
      border-radius: 6px;
      padding: 14px;
      text-align: center;
    }
    .dep-status-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
    .dep-status-card { border: 1px solid var(--border); border-radius: 6px; padding: 12px; background: var(--bg-mid); }
    .dep-status-card h4 { margin: 0 0 8px; font-size: 0.85rem; text-transform: uppercase; color: var(--muted); }
    .chip-row { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 12px; }
    .chip {
      border-radius: 4px;
      font-size: 0.68rem;
      font-family: 'JetBrains Mono', monospace;
      padding: 3px 8px;
      border: 1px solid;
    }
    .chip.ok { border-color: rgba(0, 245, 255, 0.2); background: rgba(0, 245, 255, 0.04); color: var(--primary); }
    .chip.fail { border-color: rgba(255, 46, 46, 0.2); background: rgba(255, 46, 46, 0.04); color: var(--rose); }
    .chip.warn { border-color: rgba(255, 215, 0, 0.2); background: rgba(255, 215, 0, 0.04); color: var(--accent); }
    .chip.neutral { border-color: var(--border); background: rgba(255,255,255,0.02); color: var(--muted); }
    .chip.running { border-color: rgba(255, 215, 0, 0.26); background: rgba(255, 215, 0, 0.06); color: var(--accent); }
    .routine-jobs-row {
      margin-top: -4px;
      margin-bottom: 0;
      border: 1px solid rgba(0, 245, 255, 0.16);
      border-radius: 8px;
      padding: 8px;
      background: linear-gradient(135deg, rgba(0,245,255,0.05), rgba(95,111,255,0.06));
      box-shadow: inset 0 0 0 1px rgba(255,255,255,0.03);
    }
    .routine-jobs-title {
      font-size: 0.66rem;
      font-family: 'JetBrains Mono', monospace;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--dim);
      margin-bottom: 6px;
    }
    .routine-shell {
      display: grid;
      grid-template-columns: minmax(0, 1.7fr) minmax(300px, 0.9fr);
      gap: 18px;
      align-items: start;
    }
    .routine-main-column,
    .routine-side-column {
      display: flex;
      flex-direction: column;
      gap: 14px;
      min-width: 0;
    }
    .routine-meta-strip,
    .routine-setup-strip {
      border: 1px solid rgba(0, 245, 255, 0.16);
      border-radius: 10px;
      padding: 12px;
      background: linear-gradient(135deg, rgba(0,245,255,0.04), rgba(95,111,255,0.06));
      box-shadow: inset 0 0 0 1px rgba(255,255,255,0.03);
    }
    .routine-meta-grid,
    .routine-setup-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px 12px;
    }
    .routine-control {
      display: flex;
      flex-direction: column;
      gap: 6px;
      min-width: 0;
    }
    .routine-control label {
      font-size: 0.68rem;
      color: var(--dim);
      font-family: 'JetBrains Mono', monospace;
      letter-spacing: 0.08em;
      text-transform: uppercase;
    }
    .routine-toggle-row {
      display: flex;
      flex-wrap: wrap;
      gap: 12px;
      align-items: center;
    }
    .routine-inline-actions {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      align-items: center;
    }
    .routine-toggle {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      font-size: 0.78rem;
      color: #a8b9e3;
    }
    .routine-toggle input {
      width: auto;
      margin: 0;
    }
    .routine-command-deck {
      border: 1px solid rgba(95,111,255,0.18);
      border-radius: 12px;
      padding: 14px;
      background:
        radial-gradient(circle at top left, rgba(0,245,255,0.08), transparent 42%),
        linear-gradient(145deg, rgba(5, 11, 18, 0.9), rgba(10, 18, 30, 0.82));
      position: relative;
      overflow: hidden;
    }
    .routine-command-deck::before {
      content: '';
      position: absolute;
      inset: 0;
      background:
        linear-gradient(90deg, transparent 0, rgba(255,255,255,0.03) 50%, transparent 100%);
      opacity: 0.4;
      pointer-events: none;
    }
    .routine-command-header {
      display: flex;
      justify-content: space-between;
      gap: 12px;
      align-items: start;
      margin-bottom: 12px;
    }
    .routine-command-header h4,
    .routine-stage-card h4,
    .routine-ci-card h4,
    .routine-ledger-card h4,
    .routine-transcript-card h4 {
      margin: 0;
      font-size: 0.72rem;
      font-family: 'JetBrains Mono', monospace;
      text-transform: uppercase;
      letter-spacing: 0.12em;
      color: var(--primary);
    }
    .routine-command-spine {
      display: grid;
      grid-template-columns: repeat(8, minmax(0, 1fr));
      gap: 10px;
      align-items: stretch;
    }
    .routine-stage-tab {
      position: relative;
      border: 1px solid rgba(120, 141, 255, 0.24);
      background: rgba(255,255,255,0.03);
      color: var(--muted);
      border-radius: 12px;
      padding: 10px 8px 11px;
      text-align: left;
      cursor: pointer;
      min-height: 76px;
      transition: border-color 0.2s, transform 0.2s, box-shadow 0.2s, background 0.2s;
    }
    .routine-stage-tab:hover {
      border-color: rgba(0,245,255,0.42);
      transform: translateY(-1px);
      box-shadow: 0 10px 20px rgba(0,0,0,0.16);
    }
    .routine-stage-tab[aria-selected="true"] {
      border-color: rgba(0,245,255,0.48);
      background: linear-gradient(135deg, rgba(0,245,255,0.12), rgba(95,111,255,0.12));
      color: var(--text);
      box-shadow: 0 0 0 1px rgba(0,245,255,0.16), 0 14px 26px rgba(0,0,0,0.2);
    }
    .routine-stage-tab::after {
      content: '';
      position: absolute;
      left: calc(100% + 5px);
      top: 50%;
      width: 10px;
      height: 1px;
      background: linear-gradient(90deg, rgba(0,245,255,0.3), rgba(95,111,255,0.2));
      pointer-events: none;
    }
    .routine-stage-tab:last-child::after {
      display: none;
    }
    .routine-stage-tab .routine-stage-kicker {
      display: block;
      font-size: 0.62rem;
      font-family: 'JetBrains Mono', monospace;
      text-transform: uppercase;
      letter-spacing: 0.12em;
      color: var(--dim);
      margin-bottom: 8px;
    }
    .routine-stage-tab .routine-stage-name {
      display: block;
      font-size: 0.8rem;
      font-weight: 600;
      color: inherit;
      margin-bottom: 10px;
    }
    .routine-stage-tab .routine-stage-state {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      font-size: 0.68rem;
      font-family: 'JetBrains Mono', monospace;
      color: inherit;
    }
    .routine-stage-tab .routine-stage-state::before {
      content: '';
      width: 7px;
      height: 7px;
      border-radius: 999px;
      background: currentColor;
      opacity: 0.7;
      box-shadow: 0 0 8px currentColor;
    }
    .routine-stage-tab[data-level="ok"] .routine-stage-state { color: var(--primary); }
    .routine-stage-tab[data-level="warn"] .routine-stage-state { color: var(--accent); }
    .routine-stage-tab[data-level="fail"] .routine-stage-state { color: var(--rose); }
    .routine-stage-tab[data-level="neutral"] .routine-stage-state { color: var(--muted); }
    .routine-section-grid {
      display: grid;
      grid-template-columns: minmax(0, 1fr);
      gap: 14px;
      align-items: start;
    }
    .routine-stage-card,
    .routine-ci-card,
    .routine-ledger-card,
    .routine-transcript-card {
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 14px;
      background: rgba(255,255,255,0.02);
      box-shadow: inset 0 0 0 1px rgba(255,255,255,0.02);
      min-width: 0;
    }
    .routine-stage-summary {
      font-size: 0.86rem;
      color: var(--text);
      line-height: 1.55;
    }
    .routine-metric-grid {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 10px;
      margin-top: 12px;
    }
    .routine-metric {
      border: 1px solid rgba(255,255,255,0.08);
      border-radius: 10px;
      padding: 10px;
      background: rgba(0,0,0,0.18);
    }
    .routine-metric .metric-label {
      display: block;
      font-size: 0.64rem;
      letter-spacing: 0.1em;
      text-transform: uppercase;
      color: var(--dim);
      font-family: 'JetBrains Mono', monospace;
      margin-bottom: 6px;
    }
    .routine-metric .metric-value {
      display: block;
      font-size: 0.88rem;
      color: var(--text);
      word-break: break-word;
    }
    .routine-subgrid {
      display: grid;
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      gap: 12px;
      margin-top: 12px;
    }
    .routine-detail-box {
      border: 1px solid rgba(255,255,255,0.08);
      border-radius: 10px;
      padding: 10px;
      background: rgba(0,0,0,0.18);
      min-width: 0;
    }
    .routine-detail-box h5 {
      margin: 0 0 8px;
      font-size: 0.68rem;
      font-family: 'JetBrains Mono', monospace;
      text-transform: uppercase;
      letter-spacing: 0.1em;
      color: var(--muted);
    }
    .routine-detail-box ul {
      margin: 0;
      padding-left: 18px;
      color: #b7c6ec;
      font-size: 0.78rem;
      line-height: 1.5;
    }
    .routine-detail-box li + li {
      margin-top: 5px;
    }
    .routine-stage-notes {
      margin-top: 12px;
      border: 1px solid rgba(95,111,255,0.18);
      border-radius: 10px;
      padding: 10px;
      background: rgba(8, 12, 20, 0.76);
      font-size: 0.75rem;
      font-family: 'JetBrains Mono', monospace;
      color: #a8b9e3;
      min-height: 120px;
      white-space: pre-wrap;
      word-break: break-word;
    }
    .routine-dag-view {
      margin-top: 12px;
      border: 1px solid rgba(0,245,255,0.16);
      border-radius: 12px;
      padding: 12px;
      background:
        radial-gradient(circle at top left, rgba(0,245,255,0.08), transparent 38%),
        radial-gradient(circle at bottom right, rgba(95,111,255,0.10), transparent 42%),
        rgba(4, 10, 18, 0.88);
      box-shadow: inset 0 0 0 1px rgba(255,255,255,0.02);
      display: none;
    }
    .routine-dag-view.active {
      display: block;
    }
    .routine-dag-header {
      display: flex;
      justify-content: space-between;
      gap: 8px;
      align-items: center;
      margin-bottom: 10px;
    }
    .routine-dag-lanes {
      display: grid;
      gap: 10px;
    }
    .routine-dag-lane {
      border: 1px solid rgba(255,255,255,0.08);
      border-radius: 12px;
      padding: 10px;
      background: linear-gradient(135deg, rgba(0,0,0,0.22), rgba(8, 16, 28, 0.45));
      position: relative;
      overflow: hidden;
    }
    .routine-dag-lane::before {
      content: '';
      position: absolute;
      left: 0;
      top: 0;
      bottom: 0;
      width: 2px;
      background: linear-gradient(180deg, rgba(0,245,255,0.9), rgba(95,111,255,0.5));
      box-shadow: 0 0 14px rgba(0,245,255,0.32);
    }
    .routine-dag-lane-title {
      font-size: 0.66rem;
      font-family: 'JetBrains Mono', monospace;
      letter-spacing: 0.12em;
      text-transform: uppercase;
      color: var(--dim);
      margin-bottom: 8px;
      padding-left: 8px;
    }
    .routine-dag-nodes {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      padding-left: 8px;
    }
    .routine-dag-node {
      min-width: 132px;
      border: 1px solid rgba(0,245,255,0.18);
      border-radius: 10px;
      padding: 9px 10px;
      background: rgba(255,255,255,0.03);
      box-shadow: 0 0 0 1px rgba(255,255,255,0.02), 0 10px 18px rgba(0,0,0,0.18);
    }
    .routine-dag-node-name {
      font-size: 0.78rem;
      color: var(--text);
      margin-bottom: 4px;
      font-weight: 600;
    }
    .routine-dag-node-meta {
      font-size: 0.65rem;
      color: #a8b9e3;
      font-family: 'JetBrains Mono', monospace;
      letter-spacing: 0.04em;
    }
    .routine-dag-empty {
      border: 1px dashed rgba(255,255,255,0.14);
      border-radius: 10px;
      padding: 12px;
      color: var(--dim);
      font-size: 0.78rem;
      text-align: center;
    }
    .routine-ci-grid {
      display: grid;
      gap: 12px;
    }
    .routine-ci-dynamic-list {
      display: flex;
      flex-direction: column;
      gap: 8px;
      max-height: 240px;
      overflow: auto;
    }
    .routine-ci-item {
      border: 1px solid rgba(255,255,255,0.08);
      border-radius: 10px;
      padding: 10px;
      background: rgba(0,0,0,0.2);
      width: 100%;
      text-align: left;
      color: inherit;
      font: inherit;
    }
    button.routine-ci-item {
      cursor: pointer;
    }
    button.routine-ci-item:hover,
    button.routine-ci-item:focus-visible,
    .routine-ci-item.selected {
      border-color: rgba(0,245,255,0.35);
      background: linear-gradient(135deg, rgba(0,245,255,0.08), rgba(95,111,255,0.08));
      outline: none;
      box-shadow: 0 0 0 1px rgba(0,245,255,0.14);
    }
    .routine-ci-missing {
      border-style: dashed;
      border-color: rgba(255,215,0,0.22);
      background: rgba(255,215,0,0.04);
    }
    .routine-ci-item-header {
      display: flex;
      justify-content: space-between;
      gap: 8px;
      align-items: center;
      margin-bottom: 6px;
      font-size: 0.75rem;
    }
    .routine-ci-item-label {
      color: var(--text);
      font-weight: 600;
    }
    .routine-ci-item-meta {
      color: var(--dim);
      font-size: 0.68rem;
      font-family: 'JetBrains Mono', monospace;
    }
    .routine-ci-item-copy {
      color: #a8b9e3;
      font-size: 0.72rem;
      line-height: 1.5;
    }
    .routine-transcript-card pre,
    #dash-routine-policy-detail {
      max-height: none;
    }
    .routine-policy-actions {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      align-items: center;
    }
    .routine-modal-box {
      max-width: 880px;
    }
    .routine-modal-header {
      display: flex;
      justify-content: space-between;
      gap: 12px;
      align-items: start;
    }
    .routine-modal-grid {
      display: grid;
      grid-template-columns: minmax(0, 1.2fr) minmax(280px, 0.8fr);
      gap: 14px;
      min-width: 0;
    }
    .routine-modal-actions {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      align-items: center;
    }
    .routine-modal-side {
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .routine-modal-status {
      border: 1px solid var(--border);
      border-radius: 10px;
      padding: 10px;
      background: rgba(0,0,0,0.18);
      min-height: 120px;
      font-size: 0.75rem;
      font-family: 'JetBrains Mono', monospace;
      color: #b7c6ec;
      white-space: pre-wrap;
      word-break: break-word;
    }
    .sr-only {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }
    @media (max-width: 1180px) {
      .routine-shell,
      .routine-section-grid,
      .routine-modal-grid {
        grid-template-columns: 1fr;
      }
      .routine-command-spine {
        grid-template-columns: repeat(4, minmax(0, 1fr));
      }
    }
    @media (max-width: 860px) {
      .routine-meta-grid,
      .routine-setup-grid,
      .routine-metric-grid,
      .routine-subgrid {
        grid-template-columns: 1fr;
      }
      .routine-command-spine {
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }
      .routine-stage-tab::after {
        display: none;
      }
    }
    .tl-item {
      border: 1px solid var(--border);
      border-radius: 9px;
      padding: 10px 12px;
      background: rgba(255,255,255,0.02);
    }
    .tl-item.running { border-left: 2px solid rgba(255, 215, 0, 0.6); }
    .tl-item.completed { border-left: 2px solid rgba(0, 245, 255, 0.66); }
    .tl-item.failed { border-left: 2px solid rgba(255, 46, 46, 0.7); }
    .tl-item.stage-scope { background: linear-gradient(90deg, rgba(0,245,255,0.06), rgba(0,0,0,0.02)); }
    .tl-item.stage-multi { background: linear-gradient(90deg, rgba(95,111,255,0.08), rgba(0,0,0,0.02)); }
    .tl-item.stage-gates { background: linear-gradient(90deg, rgba(255, 56, 196, 0.14), rgba(0,0,0,0.02)); }
    .tl-item.stage-push { background: linear-gradient(90deg, rgba(0,245,255,0.09), rgba(0,0,0,0.02)); }
    .tl-item.stage-ci { background: linear-gradient(90deg, rgba(168,185,227,0.12), rgba(0,0,0,0.02)); }
    .tl-item.stage-evidence { background: linear-gradient(90deg, rgba(120,255,208,0.08), rgba(0,0,0,0.02)); }
    .branch-matrix-table {
      width: 100%;
      border-collapse: collapse;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.72rem;
      color: var(--muted);
      background: rgba(0, 0, 0, 0.25);
    }
    .branch-matrix-table th,
    .branch-matrix-table td {
      border-bottom: 1px solid var(--border);
      padding: 8px 10px;
      text-align: left;
      white-space: nowrap;
    }
    .branch-matrix-table th {
      position: sticky;
      top: 0;
      z-index: 2;
      background: rgba(10, 14, 20, 0.95);
      color: var(--text);
      font-size: 0.68rem;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }
    .branch-matrix-table tbody tr:hover {
      background: rgba(255, 255, 255, 0.03);
    }
    .branch-matrix-table tbody tr.selected {
      background: rgba(0, 245, 255, 0.08);
    }
    .modal-backdrop {
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.65);
      display: none;
      align-items: center;
      justify-content: center;
      z-index: 9999;
    }
    .modal-backdrop.open {
      display: flex;
    }
    .modal-card {
      width: min(520px, 92vw);
      background: #0a0f16;
      border: 1px solid var(--border-hover);
      border-radius: 10px;
      padding: 18px;
      display: flex;
      flex-direction: column;
      gap: 10px;
      box-shadow: 0 18px 60px rgba(0,0,0,0.55);
    }
    .modal-title {
      font-size: 0.86rem;
      letter-spacing: 0.08em;
      text-transform: uppercase;
      color: var(--rose);
      font-family: 'JetBrains Mono', monospace;
      margin: 0;
    }
    .multi-dag-visual-wrap {
      margin-top: 12px;
      border: 1px solid var(--border);
      border-radius: 10px;
      background: radial-gradient(circle at 15% 10%, rgba(0,245,255,0.09), transparent 50%),
                  radial-gradient(circle at 90% 90%, rgba(106,125,255,0.09), transparent 55%),
                  rgba(0, 0, 0, 0.28);
      padding: 10px;
      min-height: 180px;
    }
    .multi-dag-visual-empty {
      color: var(--dim);
      font-size: 0.78rem;
      font-family: 'JetBrains Mono', monospace;
      text-align: center;
      padding: 22px 8px;
    }
    .multi-dag-visual-scroll {
      width: 100%;
      overflow: auto;
      border-radius: 8px;
    }
    #multi-dag-visual {
      display: block;
      min-width: 100%;
      height: auto;
    }
    .branch-log-entry {
      border: 1px solid var(--border);
      border-left: 3px solid transparent;
      border-radius: 8px;
      padding: 10px;
      background: rgba(255,255,255,0.02);
      display: flex;
      flex-direction: column;
      gap: 8px;
    }
    .branch-log-entry.ok { border-left-color: var(--primary); }
    .branch-log-entry.fail { border-left-color: var(--rose); }
    .branch-log-head {
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 10px;
      flex-wrap: wrap;
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.72rem;
      color: var(--muted);
    }
    .branch-log-actions {
      display: flex;
      gap: 6px;
      flex-wrap: wrap;
    }
    .branch-log-body {
      font-family: 'JetBrains Mono', monospace;
      font-size: 0.75rem;
      color: var(--text);
      line-height: 1.45;
      white-space: pre-wrap;
      word-break: break-word;
    }
    .pre-wrap { position: relative; }
    .pre-actions { position: absolute; top: 6px; right: 10px; display: flex; gap: 4px; opacity: 0; transition: opacity 0.2s; z-index: 20; }
    .pre-wrap:hover .pre-actions { opacity: 1; }
    .action-btn {
      background: rgba(0,0,0,0.6); border: 1px solid var(--border); color: var(--dim);
      border-radius: 4px; padding: 3px 7px; font-size: 0.65rem; font-family: 'JetBrains Mono', monospace;
      cursor: pointer; transition: all 0.2s; backdrop-filter: blur(4px);
    }
    .action-btn:hover { background: rgba(0, 245, 255, 0.1); color: var(--primary); border-color: rgba(0,245,255,0.3); }
    .dep-ok { color: var(--primary); }
    .dep-fail { color: var(--rose); }
    .muted { color: var(--muted); margin-top: 4px; }
    .three-panel-layout { display: flex; flex-direction: column; gap: 24px; }
    .panel-left, .panel-center, .panel-right { width: 100%; }
    .panel-center { border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); padding: 24px 0; }
    .tree-node { cursor: pointer; padding: 5px 10px; border-radius: 6px; font-size: 0.78rem; font-family: 'JetBrains Mono', monospace; transition: all 0.2s; border-left: 2px solid transparent; }
    .tree-node:hover { background: rgba(255,255,255,0.04); border-left-color: rgba(255,255,255,0.1); }
    .tree-node.selected { background: rgba(0, 245, 255, 0.06); border-left-color: var(--primary); }
    .tree-node.agorg { color: #818cf8; font-weight: 600; }
    .tree-node.ago { color: var(--primary); }
    .tree-node.none { color: var(--dim); font-style: italic; }
    .sub-tabs { display: flex; gap: 0; margin-bottom: 18px; border-bottom: 1px solid var(--border); }
    .sub-tab { background: none; border: none; color: var(--dim); font-size: 0.78rem; font-weight: 500; cursor: pointer; padding: 10px 16px; border-bottom: 2px solid transparent; transition: all 0.2s; }
    .sub-tab:hover { color: var(--text); }
    .sub-tab.active { color: var(--primary); border-bottom-color: var(--primary); }
    .sub-panel { display: none; animation: fadeIn 0.25s ease-out; }
    .sub-panel.active { display: block; }
    .batch-list { font-family: 'JetBrains Mono', monospace; min-height: 80px; padding: 10px; background: rgba(0,0,0,0.3); color: var(--primary); border: 1px solid var(--border); border-radius: 8px; }
    .check-label {
      font-size: 0.78rem;
      color: var(--muted);
      display: inline-flex;
      align-items: center;
      gap: 6px;
      cursor: pointer;
      padding: 4px 0;
      transition: color 0.2s;
    }
    .check-label:hover { color: var(--text); }
    .check-label input[type="checkbox"] { width: 14px; height: 14px; accent-color: var(--primary); }
    .status {
      margin-top: 24px;
      display: grid;
      gap: 24px;
      grid-template-columns: 1fr 1fr;
    }
    .step {
      border: 1px solid var(--border);
      border-radius: 10px;
      padding: 14px;
      background: rgba(255, 255, 255, 0.015);
      display: flex;
      flex-direction: column;
      gap: 10px;
    }
    .step-title {
      font-size: 0.75rem;
      font-weight: 600;
      font-family: 'JetBrains Mono', monospace;
      color: var(--text);
      text-transform: uppercase;
      letter-spacing: 0.06em;
    }
    
    /* Hero Dropdown */
    .agorg-scope-container { position: relative; display: inline-block; }
    .agorg-dropdown { 
      position: relative;
    }
    .agorg-dropdown-menu {
      position: absolute;
      top: 100%;
      left: 16px;
      margin-top: 8px;
      background: var(--bg-mid);
      border: 1px solid var(--border);
      border-radius: 6px;
      min-width: 280px;
      max-height: 400px;
      overflow-y: auto;
      z-index: 1000;
      display: none;
      box-shadow: 0 8px 30px rgba(0,0,0,0.6);
      backdrop-filter: blur(8px);
    }
    .agorg-dropdown.active .agorg-dropdown-menu { display: block; }
    .agorg-drop-item {
      padding: 10px 14px;
      cursor: pointer;
      border-bottom: 1px solid var(--border);
      display: flex;
      justify-content: space-between;
      align-items: center;
      color: var(--text);
      transition: background 0.2s;
    }
    .agorg-drop-item:hover { background: rgba(0, 245, 255, 0.05); color: #00F5FF; border-left: 2px solid var(--primary); }
    .agorg-drop-item .type {
      font-size: 0.7rem;
      font-family: "JetBrains Mono", monospace;
      padding: 2px 6px;
      background: var(--bg-light);
      border-radius: 4px;
      color: var(--muted);
    }
    .agorg-drop-header {
      padding: 8px 14px;
      font-size: 0.75rem;
      font-family: "JetBrains Mono", monospace;
      text-transform: uppercase;
      color: var(--muted);
      font-weight: 600;
      background: var(--bg-deep);
      border-bottom: 1px solid var(--border);
      position: sticky;
      top: 0;
    }

    /* Registry list styles */
    .agorg-registry-list {
      display: flex;
      flex-direction: column;
    }
    .agorg-reg-item {
      padding: 8px 10px;
      border-bottom: 1px solid #202b38;
      cursor: pointer;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
    .agorg-reg-item:hover, .agorg-reg-item.active {
      background: #1a2235;
    }
    .agorg-reg-type {
      font-size: 0.7rem;
      padding: 2px 6px;
      background: #202b38;
      border-radius: 4px;
      color: #8b9bb4;
    }
    /* ═══════════ Responsive ═══════════ */
    @media (min-width: 1200px) {
      .three-panel-layout { flex-direction: row; }
      .panel-left { flex: 1; }
      .panel-center { flex: 1; }
      .panel-right { flex: 1; }
    }
    @media (max-width: 980px) {
      .grid, .status { grid-template-columns: 1fr; }
      .hero { flex-direction: column; gap: 12px; align-items: flex-start; }
      .tabs { padding: 0 16px; }
      .panel { padding: 16px; }
    }
    @media (max-width: 600px) {
      .tabs { gap: 0; }
      button.tab { padding: 10px 12px; font-size: 0.72rem; }
      .hero { padding: 12px 16px; }
    }
  </style>
</head>
<body>
<div class="bg-orbs"><div class="orb-accent"></div></div>
<div class="wrap">
  <div class="hero">
    <div>
      <h1>Arqon <span class="accent">Pilot</span></h1>
      <h2>Orchestrating Autonomous Evolution</h2>
    </div>
    <div class="bus-status-row">
      <div class="status-left">
        ArqonBus:
        <span id="bus-status-chip" class="bus-chip disconnected" tabindex="0" role="status">DISCONNECTED</span>
      </div>
      <div class="system-menu" style="display:flex; gap:8px; align-items:center;">
        <button class="menu-btn" onclick="run('pilot.engine.stop', {})" title="Stop System"><span class="icon">⏹</span></button>
        <button class="menu-btn" onclick="run('pilot.engine.restart', {})" title="Restart Engine"><span class="icon">↺</span></button>
      </div>
      
      <!-- Active AGOrg Scope dropdown -->
      <div style="position:relative; display:inline-block;" class="agorg-dropdown" id="agorg-hero-dropdown-container">
        <button class="btn secondary" id="agorg-open-btn" style="margin-left: 16px; min-width: 140px; color:var(--text);" onclick="toggleAgorgDropdown(event)">
          AGOrg: <span id="agorg-status-chip" style="color:#00F5FF; text-shadow: 0 0 5px rgba(0,245,255,0.4);" tabindex="0" role="status">Loading...</span> ▼
        </button>
        <div class="agorg-dropdown-menu" id="agorg-hero-dropdown" onclick="event.stopPropagation()">
          <div class="agorg-drop-header">Loading registered repositories...</div>
        </div>
      </div>
    </div>
  </div>

  <div class="tabs">
    <button class="tab active" data-tab="dashboard">Dashboard</button>
    <button class="tab" data-tab="oracle">Oracle</button>
    <button class="tab" data-tab="heal">Heal</button>
    <button class="tab" data-tab="dependencies">Dependencies</button>
    <button class="tab" data-tab="branch">Branch</button>
    <button class="tab" data-tab="multi">Multi</button>
    <button class="tab" data-tab="telemetry">Telemetry</button>
    <button class="tab" data-tab="codex">Codex</button>
    <button class="tab" data-tab="settings">Settings</button>
  </div>

  <section class="panel active" id="dashboard">
    <div class="grid">
      <div class="card" style="grid-column: 1 / -1;">
      <h3>Command Graph Orchestration (P5)</h3>
      <div class="helper">Unified cross-tab sequence. Preview operations never mutate. Execution emits lineage.</div>
      <div class="chip-row" id="p5-rail-strip">
        <button id="p5-chip-status" class="chip neutral action-chip" onclick="p5OrchestrateStep('dependency', 'status', true)" aria-label="Preview Database and Dependency Status" title="Preview Database and Dependency Status">Status</button>
        <button id="p5-chip-bus" class="chip neutral action-chip" onclick="p5OrchestrateStep('dependency', 'bus-status', true)" aria-label="Preview Bus Health" title="Preview Bus Health">Bus Health</button>
        <button id="p5-chip-heal-plan" class="chip neutral action-chip" onclick="p5OrchestrateStep('command', 'heal.plan', true)" aria-label="Preview Heal Plan" title="Preview Heal Plan">Heal Plan</button>
        <button id="p5-chip-heal-run" class="chip neutral action-chip" onclick="p5OrchestrateStep('command', 'heal.run', false)" aria-label="Execute Heal Run" title="Execute Heal Run (Mutates)">Heal Run</button>
        <button id="p5-chip-push" class="chip neutral action-chip" onclick="p5OrchestrateStep('dependency', 'push', false)" aria-label="Execute Push Safe" title="Execute Push Safe (Mutates)">Push Safe</button>
        <button id="p5-chip-branch" class="chip neutral action-chip" onclick="p5OrchestrateStep('branch', 'status', true)" aria-label="Preview Branch Status" title="Preview Branch Status">Branch Preview</button>
        <button id="p5-chip-multi" class="chip neutral action-chip" onclick="p5OrchestrateStep('command', 'multi.status', true)" aria-label="Preview Multi Repo Status" title="Preview Multi Repo Status">Multi</button>
        <button id="p5-chip-dag" class="chip neutral action-chip" onclick="p5OrchestrateStep('command', 'dag.evaluate', true)" aria-label="Preview DAG Evaluation" title="Preview DAG Evaluation">DAG</button>
        <button id="p5-chip-apply" class="chip neutral action-chip" onclick="p5OrchestrateStep('command', 'multi.apply', false)" aria-label="Execute Staged Apply" title="Execute Staged Apply (Mutates)">Staged Apply</button>
      </div>
    </div>
    <div class="card" style="grid-column: 1 / -1;">
      <h3>Post-Commit Routine (Pilot for Pilot)</h3>
      <div id="dash-routine-live-status" class="sr-only" role="status" aria-live="polite"></div>
      <div id="dash-routine-live-alert" class="sr-only" role="alert" aria-live="assertive"></div>
      <div class="routine-shell">
        <div class="routine-main-column">
          <div class="routine-meta-strip">
            <div class="routine-inline-actions" style="justify-content: space-between; margin-bottom: 12px;">
              <div class="chip-row" style="margin-bottom:0;">
                <span id="dash-routine-profile-source-chip" class="chip neutral" tabindex="0" role="status">Profile: loading</span>
                <span id="dash-routine-profile-steps-chip" class="chip neutral" tabindex="0" role="status">Steps: -</span>
                <span id="dash-routine-mode-chip" class="chip neutral" tabindex="0" role="status">Mode: safe</span>
                <span id="dash-routine-last-result-chip" class="chip neutral" tabindex="0" role="status">Last Result: idle</span>
              </div>
              <div class="routine-inline-actions">
                <button class="action-btn" onclick="dashLoadRoutine()" style="padding: 2px 8px; background: rgba(255,255,255,0.05); border: 1px solid var(--border); border-radius: 4px; font-size: 0.7rem; color: var(--dim); cursor: pointer;" onmouseover="this.style.color='var(--text)'" onmouseout="this.style.color='var(--dim)'">Load Routine</button>
                <button class="action-btn" onclick="dashToggleRoutinePolicyView()" style="padding: 2px 8px; background: rgba(255,255,255,0.05); border: 1px solid var(--border); border-radius: 4px; font-size: 0.7rem; color: var(--dim); cursor: pointer;" onmouseover="this.style.color='var(--text)'" onmouseout="this.style.color='var(--dim)'">View Policy</button>
                <button class="action-btn" onclick="routinePolicyModalOpen()" style="padding: 2px 8px; background: rgba(255,255,255,0.05); border: 1px solid var(--border); border-radius: 4px; font-size: 0.7rem; color: var(--dim); cursor: pointer;" onmouseover="this.style.color='var(--text)'" onmouseout="this.style.color='var(--dim)'">Quick Edit Policy</button>
              </div>
            </div>
            <div class="routine-meta-grid">
              <div class="routine-control">
                <label for="dash-routine-scope-summary">Resolved Scope</label>
                <input id="dash-routine-scope-summary" value="Awaiting resolve stage" readonly />
              </div>
              <div class="routine-control">
                <label for="dash-routine-plan-summary">Execution Plan</label>
                <input id="dash-routine-plan-summary" value="Resolve to compute plan" readonly />
              </div>
            </div>
          </div>
          <div id="dash-routine-policy-view" style="display: none; max-height: 300px; overflow: auto; background: rgba(0,0,0,0.2); border: 1px solid var(--border); border-radius: 6px; padding: 10px;">
            <pre id="dash-routine-policy-detail" style="font-size: 0.75rem; margin: 0; color: #a8b9e3;"></pre>
          </div>
          <div class="routine-command-deck">
            <div class="routine-command-header">
              <div>
                <h4>Constitutional Development Deck</h4>
                <div class="helper">Resolve scope, visualize the plan, execute the governed path, and reconcile the resulting state without leaving Dashboard.</div>
              </div>
              <span class="chip neutral" id="dash-routine-stage-status-chip" role="status" aria-live="polite">Workspace: Resolve</span>
            </div>
            <div id="dash-routine-stage-tabs" class="routine-command-spine" role="tablist" aria-label="Routine stages">
              <button id="dash-routine-stage-resolve-tab" class="routine-stage-tab" type="button" role="tab" aria-selected="true" aria-controls="dash-routine-stage-panel" data-stage="resolve" data-level="neutral" onclick="routineSelectStage('resolve')">
                <span class="routine-stage-kicker">Phase 01</span>
                <span class="routine-stage-name">Resolve</span>
                <span class="routine-stage-state" id="dash-routine-stage-resolve-state">Idle</span>
              </button>
              <button id="dash-routine-stage-plan-tab" class="routine-stage-tab" type="button" role="tab" aria-selected="false" aria-controls="dash-routine-stage-panel" data-stage="plan" data-level="neutral" onclick="routineSelectStage('plan')">
                <span class="routine-stage-kicker">Phase 02</span>
                <span class="routine-stage-name">Plan</span>
                <span class="routine-stage-state" id="dash-routine-stage-plan-state">Idle</span>
              </button>
              <button id="dash-routine-stage-multi-tab" class="routine-stage-tab" type="button" role="tab" aria-selected="false" aria-controls="dash-routine-stage-panel" data-stage="multi" data-level="neutral" onclick="routineSelectStage('multi')">
                <span class="routine-stage-kicker">Phase 03</span>
                <span class="routine-stage-name">Multi</span>
                <span class="routine-stage-state" id="dash-routine-stage-multi-state">Idle</span>
              </button>
              <button id="dash-routine-stage-gates-tab" class="routine-stage-tab" type="button" role="tab" aria-selected="false" aria-controls="dash-routine-stage-panel" data-stage="gates" data-level="neutral" onclick="routineSelectStage('gates')">
                <span class="routine-stage-kicker">Phase 04</span>
                <span class="routine-stage-name">Gates</span>
                <span class="routine-stage-state" id="dash-routine-stage-gates-state">Idle</span>
              </button>
              <button id="dash-routine-stage-push-tab" class="routine-stage-tab" type="button" role="tab" aria-selected="false" aria-controls="dash-routine-stage-panel" data-stage="push" data-level="neutral" onclick="routineSelectStage('push')">
                <span class="routine-stage-kicker">Phase 05</span>
                <span class="routine-stage-name">Push</span>
                <span class="routine-stage-state" id="dash-routine-stage-push-state">Idle</span>
              </button>
              <button id="dash-routine-stage-ci-tab" class="routine-stage-tab" type="button" role="tab" aria-selected="false" aria-controls="dash-routine-stage-panel" data-stage="ci" data-level="neutral" onclick="routineSelectStage('ci')">
                <span class="routine-stage-kicker">Phase 06</span>
                <span class="routine-stage-name">CI</span>
                <span class="routine-stage-state" id="dash-routine-stage-ci-state">Idle</span>
              </button>
              <button id="dash-routine-stage-evidence-tab" class="routine-stage-tab" type="button" role="tab" aria-selected="false" aria-controls="dash-routine-stage-panel" data-stage="evidence" data-level="neutral" onclick="routineSelectStage('evidence')">
                <span class="routine-stage-kicker">Phase 07</span>
                <span class="routine-stage-name">Evidence</span>
                <span class="routine-stage-state" id="dash-routine-stage-evidence-state">Idle</span>
              </button>
              <button id="dash-routine-stage-reconcile-tab" class="routine-stage-tab" type="button" role="tab" aria-selected="false" aria-controls="dash-routine-stage-panel" data-stage="reconcile" data-level="neutral" onclick="routineSelectStage('reconcile')">
                <span class="routine-stage-kicker">Phase 08</span>
                <span class="routine-stage-name">Reconcile</span>
                <span class="routine-stage-state" id="dash-routine-stage-reconcile-state">Idle</span>
              </button>
            </div>
          </div>
          <div class="routine-setup-strip">
            <div class="routine-setup-grid">
              <div class="routine-control">
                <label for="dash-routine-group">Cohort Group</label>
                <input id="dash-routine-group" placeholder="group (e.g. core)" value="core" />
              </div>
              <div class="routine-control">
                <label for="dash-routine-tags">Cohort Tags</label>
                <input id="dash-routine-tags" placeholder="tags (comma-separated, e.g. pilot)" value="pilot" />
              </div>
              <div class="routine-control">
                <label for="dash-routine-branch">Push Branch</label>
                <input id="dash-routine-branch" placeholder="main" value="main" />
              </div>
              <div class="routine-control">
                <label for="dash-routine-remote">Push Remote</label>
                <input id="dash-routine-remote" placeholder="origin" value="origin" />
              </div>
            </div>
            <div class="routine-toggle-row" style="margin-top: 12px;">
              <label class="routine-toggle">
                <input id="dash-routine-allow-push" type="checkbox" checked />
                Allow push step
              </label>
              <label class="routine-toggle">
                <input id="dash-routine-export-evidence" type="checkbox" />
                Export evidence
              </label>
              <label class="routine-toggle">
                <input id="dash-routine-auto-heal" type="checkbox" checked />
                Auto-heal known-safe failures
              </label>
              <label class="routine-toggle">
                <input id="dash-routine-auto-codex" type="checkbox" checked />
                Auto-run Codex remediation
              </label>
              <button id="dash-routine-run-btn" class="btn" onclick="dashRunPostCommitRoutine()">Run Post-Commit Routine</button>
            </div>
          </div>
          <div class="routine-section-grid">
            <section class="routine-ci-card" aria-labelledby="dash-routine-ci-observatory-title">
              <div class="routine-inline-actions" style="justify-content: space-between; margin-bottom: 10px;">
                <div>
                  <h4 id="dash-routine-ci-observatory-title">Continuous Integration Observatory</h4>
                  <div class="helper">Live GitHub Actions posture for the current branch and routine context.</div>
                </div>
                <button class="action-btn" onclick="dashRefreshCiStatus()" style="padding: 2px 8px; background: rgba(255,255,255,0.05); border: 1px solid var(--border); border-radius: 4px; font-size: 0.7rem; color: var(--dim); cursor: pointer;" onmouseover="this.style.color='var(--text)'" onmouseout="this.style.color='var(--dim)'">Refresh CI</button>
              </div>
              <div class="routine-ci-grid">
                <div id="dash-routine-ci-dynamic-list" class="routine-ci-dynamic-list" role="log" aria-live="polite" aria-label="CI workflow summary">
                  <div class="routine-ci-item">
                    <div class="routine-ci-item-header">
                      <span class="routine-ci-item-label">No workflow data loaded</span>
                      <span class="chip neutral">Idle</span>
                    </div>
                    <div class="routine-ci-item-copy">Refresh CI to populate live GitHub Actions state for the selected branch.</div>
                  </div>
                </div>
                <div class="routine-detail-box">
                  <h5>CI Notes</h5>
                  <div id="dash-routine-ci-policy-summary" class="helper" style="margin-bottom: 8px;">Policy coverage will appear after workflow discovery.</div>
                  <div id="dash-routine-ci-notes" class="routine-modal-status" style="min-height: 96px;">CI observatory ready.</div>
                </div>
              </div>
            </section>
            <section id="dash-routine-stage-panel" class="routine-stage-card" role="tabpanel" aria-labelledby="dash-routine-stage-resolve-tab" tabindex="0">
              <div class="routine-inline-actions" style="justify-content: space-between; margin-bottom: 10px;">
                <div>
                  <h4 id="dash-routine-workspace-title">Resolve</h4>
                  <div id="dash-routine-workspace-summary" class="routine-stage-summary">Active scope, cohort, and execution permissions will appear here.</div>
                </div>
                <div id="dash-routine-workspace-chip-row" class="chip-row" style="margin-bottom:0;"></div>
              </div>
              <div id="dash-routine-workspace-metrics" class="routine-metric-grid"></div>
              <div class="routine-subgrid">
                <div class="routine-detail-box">
                  <h5>Details</h5>
                  <ul id="dash-routine-workspace-details">
                    <li>Run Resolve to materialize scope and cohort data.</li>
                  </ul>
                </div>
                <div class="routine-detail-box">
                  <h5>Artifacts And Actions</h5>
                  <ul id="dash-routine-workspace-artifacts">
                    <li>No artifacts yet.</li>
                  </ul>
                </div>
              </div>
              <div id="dash-routine-workspace-notes" class="routine-stage-notes" role="status" aria-live="polite">Routine workspace ready.</div>
              <div id="dash-routine-dag-view" class="routine-dag-view" aria-live="polite">
                <div class="routine-dag-header">
                  <div>
                    <h4 style="margin:0;">Dependency DAG</h4>
                    <div class="helper">Stage-banded cohort topology for the current Multi preview.</div>
                  </div>
                  <span id="dash-routine-dag-summary-chip" class="chip neutral">DAG: pending</span>
                </div>
                <div id="dash-routine-dag-lanes" class="routine-dag-lanes">
                  <div class="routine-dag-empty">Run Multi to materialize dependency topology.</div>
                </div>
              </div>
            </section>
          </div>
        </div>
        <div class="routine-side-column">
          <section class="routine-ledger-card">
            <div class="routine-inline-actions" style="justify-content: space-between; margin-bottom: 10px;">
              <div>
                <h4>Run Ledger</h4>
                <div class="helper">Durations, failures, and remediation stay visible while the workspace pivots across stages.</div>
              </div>
              <button class="action-btn" onclick="dashRefreshCdStatus()" style="padding: 2px 8px; background: rgba(255,255,255,0.05); border: 1px solid var(--border); border-radius: 4px; font-size: 0.7rem; color: var(--dim); cursor: pointer;" onmouseover="this.style.color='var(--text)'" onmouseout="this.style.color='var(--dim)'">Refresh CD</button>
            </div>
            <div id="dash-routine-timeline" class="timeline" role="log" aria-live="polite">
              <div class="tl-empty">No routine run yet.</div>
            </div>
            <div id="dash-routine-actions" class="row"></div>
          </section>
          <section class="routine-transcript-card">
            <div class="routine-inline-actions" style="justify-content: space-between; margin-bottom: 10px;">
              <div>
                <h4>Transcript</h4>
                <div class="helper">Operator-readable run summary with lineage pointers and next actions.</div>
              </div>
              <div class="pre-actions">
                <button class="action-btn" onclick="copyToClipboard('dash-routine-out', this)">COPY</button>
                <button class="action-btn" onclick="clearElement('dash-routine-out')">CLEAR</button>
              </div>
            </div>
            <pre id="dash-routine-out" role="status" aria-live="polite">Routine ready.</pre>
          </section>
        </div>
      </div>
    </div>
    <div id="dash-routine-policy-modal" class="modal-overlay" role="dialog" aria-modal="true" aria-labelledby="dash-routine-policy-modal-title" aria-hidden="true">
      <div class="modal-box routine-modal-box">
        <div class="routine-modal-header">
          <div>
            <h3 id="dash-routine-policy-modal-title">Operator Routine Policy Quick Edit</h3>
            <div class="helper">Draft, simulate, and activate `operator_routine` policy from the dashboard context without leaving the routine deck.</div>
          </div>
          <button class="btn secondary" type="button" onclick="routinePolicyModalClose()">Close</button>
        </div>
        <div class="routine-modal-grid">
          <div class="routine-control">
            <label for="dash-routine-policy-editor">Policy JSON</label>
            <textarea id="dash-routine-policy-editor" spellcheck="false" aria-describedby="dash-routine-policy-modal-title"></textarea>
          </div>
          <div class="routine-modal-side">
            <div class="routine-detail-box">
              <h5>Context</h5>
              <ul id="dash-routine-policy-context">
                <li>Kind: operator_routine</li>
                <li>Target: current dashboard scope</li>
                <li>Simulation required before activation</li>
              </ul>
            </div>
            <div class="routine-detail-box">
              <h5>Status</h5>
              <div id="dash-routine-policy-modal-status" class="routine-modal-status" role="status" aria-live="polite">Modal ready.</div>
            </div>
          </div>
        </div>
        <div class="routine-modal-actions">
          <button class="btn secondary" type="button" onclick="routinePolicyModalLoad()">Load</button>
          <button class="btn secondary" type="button" onclick="routinePolicyModalSimulate()">Simulate</button>
          <button class="btn" type="button" onclick="routinePolicyModalActivate()">Activate</button>
        </div>
      </div>
    </div>
    <div class="card" style="grid-column: 1 / -1;">
      <h3>Release Routine (Phase D)</h3>
      <div class="helper">Release mode runs readiness, compatibility, migration smoke, gate/push, bundle verify, and signed evidence export.</div>
      <div class="chip-row">
        <span id="dash-release-readiness-chip" class="chip neutral" tabindex="0" role="status">Readiness: idle</span>
        <span id="dash-release-compat-chip" class="chip neutral" tabindex="0" role="status">Compat: idle</span>
        <span id="dash-release-migration-chip" class="chip neutral" tabindex="0" role="status">Migration: idle</span>
        <span id="dash-release-push-chip" class="chip neutral" tabindex="0" role="status">Publish: idle</span>
        <span id="dash-release-bundle-chip" class="chip neutral" tabindex="0" role="status">Bundle: idle</span>
        <span id="dash-release-verify-chip" class="chip neutral" tabindex="0" role="status">Verify: idle</span>
        <span id="dash-release-score-chip" class="chip neutral" tabindex="0" role="status">Score: -</span>
      </div>
      <div class="row">
        <input id="dash-release-label" placeholder="release label (e.g. 0.2.0a1)" value="alpha-local" />
        <input id="dash-release-bundle-path" placeholder="release bundle path (auto-filled after collect)" />
      </div>
      <div class="row">
        <label style="font-size:0.82rem;color:#a8b9e3;">
          <input id="dash-release-allow-push" type="checkbox" style="width:auto;vertical-align:middle;margin-right:6px;" />
          allow publish push step
        </label>
        <button id="dash-release-run-btn" class="btn" onclick="dashRunReleaseRoutine()">Run Release Routine</button>
        <button class="btn secondary" onclick="dashReleaseRunStep('release-readiness')">Readiness</button>
        <button class="btn secondary" onclick="dashReleaseRunStep('release-compat-matrix')">Compat</button>
        <button class="btn secondary" onclick="dashReleaseRunStep('release-migration-smoke')">Migration</button>
        <button class="btn secondary" onclick="dashReleaseRunStep('release-collect-evidence')">Collect Evidence</button>
        <button class="btn secondary" onclick="dashReleaseRunStep('release-verify-bundle')">Verify Bundle</button>
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-release-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-release-out')">CLEAR</button>
        </div>
        <pre id="dash-release-out" role="status" aria-live="polite">Release routine ready.</pre>
      </div>
    </div>
    <div class="card" style="grid-column: 1 / -1;">
        <h3>Unified Operations Timeline</h3>
        <div class="helper">Stitched chronological log across all domains (Branch, Dependencies, Commands).</div>
        <div class="row">
          <select id="dash-timeline-domain" onchange="unifiedTimelineLoad()">
            <option value="">All Domains</option>
            <option value="branch">Branch</option>
            <option value="dependency">Dependency</option>
            <option value="heal">Heal</option>
            <option value="command">Command</option>
          </select>
          <button class="btn secondary" onclick="unifiedTimelineLoad()">Refresh Timeline</button>
        </div>
        <div class="pre-wrap">
          <div class="pre-actions">
            <button class="action-btn" onclick="clearElement('dash-timeline-out')">CLEAR</button>
          </div>
          <pre id="dash-timeline-out">Loading timeline...</pre>
        </div>
      </div>
      <div class="card" style="grid-column: 1 / -1;">
        <h3>Evidence Integrity Verification</h3>
        <div class="helper">Validate the cryptographic integrity of the unified audit chain, exported bundles, or individual artifact sidecars.</div>
        <div class="row">
          <input type="text" id="dash-verify-path" style="flex:1" placeholder="Path to bundle or artifact (leave empty to verify local audit chain)" />
          <button class="btn" onclick="dashVerifyEvidence()">Verify Integrity</button>
        </div>
        <div class="pre-wrap">
          <div class="pre-actions">
            <button class="action-btn" onclick="clearElement('dash-verify-out')">CLEAR</button>
          </div>
          <pre id="dash-verify-out">Ready to verify</pre>
        </div>
      </div>
      <div class="card">
        <h3>System Status</h3>
        <div class="chip-row">
          <span id="dash-policy-chip" class="chip neutral" tabindex="0" role="status">Policy: unknown</span>
          <span id="dash-hook-chip" class="chip neutral" tabindex="0" role="status">Hook: unknown</span>
          <span id="dash-drift-chip" class="chip neutral" tabindex="0" role="status">Drift: unknown</span>
          <span id="dash-bus-chip" class="chip neutral" tabindex="0" role="status">Bus: unknown</span>
          <span id="dash-db-chip" class="chip neutral" tabindex="0" role="status">DB: unknown</span>
          <span id="dash-gate-chip" class="chip neutral" tabindex="0" role="status">Gate: unknown</span>
          <span id="dash-push-chip" class="chip neutral" tabindex="0" role="status">Push: unknown</span>
        </div>
        <div class="row">
          <button class="btn secondary" onclick="dashRunPolicy()">Policy</button>
          <button class="btn secondary" onclick="dashRunHookPolicy()">Hook Policy</button>
          <button class="btn secondary" onclick="dashRunDrift()">Drift</button>
          <button class="btn secondary" onclick="dashRunGate()">Gate</button>
          <button class="btn" onclick="dashRunRepair()">Repair</button>
          <button class="btn secondary" onclick="dashStartBus()">Start Bus</button>
          <button class="btn secondary" onclick="dashStopBus()">Stop Bus</button>
          <button class="btn secondary" onclick="dashRestartBus()">Restart Bus</button>
          <button class="btn secondary" onclick="dashBusStatus()">Bus Status</button>
          <button class="btn secondary" onclick="dashDbStatus()">DB Status</button>
          <button class="btn secondary" onclick="dashDbStart()">DB Start</button>
          <button class="btn secondary" onclick="dashDbStop()">DB Stop</button>
          <button class="btn secondary" onclick="dashDbRestart()">DB Restart</button>
          <button class="btn secondary" onclick="dashServicesStatus()">Services Status</button>
          <button class="btn secondary" onclick="dashServicesStart()">Start Services</button>
          <button class="btn secondary" onclick="dashServicesStop()">Stop Services</button>
          <button class="btn secondary" onclick="dashServicesRestart()">Restart Services</button>
          <button class="btn secondary" onclick="dashExportEvidence()">Export Evidence</button>
        </div>
        <div class="row">
          <input id="dash-push-branch" placeholder="main" value="main" />
          <input id="dash-push-remote" placeholder="origin" value="origin" />
          <button class="btn" onclick="dashRunPush()">Push Safe</button>
        </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-status-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-status-out')">CLEAR</button>
        </div>
        <pre id="dash-status-out">ready</pre>
      </div>
    </div>
    <div class="card">
        <h3>Temporary Components Inventory</h3>
        <div class="helper">Wave H transparency surface. Lists unavoidable shims/bridges and their runtime state.</div>
        <div class="row">
          <button class="btn secondary" onclick="dashRefreshTemporaryComponents()">Refresh Inventory</button>
          <button class="btn secondary" onclick="dashRunTemporaryChecklist()">Run Checklist</button>
          <button class="btn secondary" onclick="dashExportTemporaryComponents()">Export Inventory Artifact</button>
        </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-temp-components-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-temp-components-out')">CLEAR</button>
        </div>
        <pre id="dash-temp-components-out">No temporary component inventory loaded yet.</pre>
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-temp-checklist-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-temp-checklist-out')">CLEAR</button>
        </div>
        <pre id="dash-temp-checklist-out">No temporary component checklist run yet.</pre>
      </div>
      </div>

      <div class="card">
        <h3>Wave Acceptance Matrix</h3>
        <div class="helper">Wave I deterministic closure checks. Runs the acceptance matrix script and persists an artifact.</div>
        <div class="row">
          <input id="dash-accept-wave" placeholder="I" value="I" />
          <select id="dash-accept-profile">
            <option value="quick">quick</option>
            <option value="full">full</option>
          </select>
          <button class="btn secondary" onclick="dashRunAcceptanceMatrix()">Run Matrix</button>
        </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="openDashAcceptanceArtifact()">OPEN ARTIFACT</button>
          <button class="action-btn" onclick="copyToClipboard('dash-acceptance-matrix-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-acceptance-matrix-out')">CLEAR</button>
        </div>
        <pre id="dash-acceptance-matrix-out">No acceptance matrix run yet.</pre>
      </div>
      </div>

      <div class="card">
        <h3>AGOrg Overview</h3>
        <div class="helper">Dashboard control summary for active AGOrg scope: score, unresolved issues, and class distribution.</div>
        <div class="chip-row">
          <span id="dash-agorg-score-chip" class="chip neutral" tabindex="0" role="status">Score: unknown</span>
          <span id="dash-agorg-issues-chip" class="chip neutral" tabindex="0" role="status">Issues: unknown</span>
          <span id="dash-agorg-offpolicy-chip" class="chip neutral" tabindex="0" role="status">Off-policy: unknown</span>
        </div>
        <div class="row">
          <button class="btn secondary" onclick="dashAgorgOverviewRefresh()">Refresh Overview</button>
        </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-overview-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-overview-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-overview-out">No AGOrg overview yet.</pre>
      </div>
    </div>

    <div class="card">
        <h3>Oracle + Heal Quick Ops</h3>
        <div class="helper">Fast path for day-to-day work: ask Oracle for context, then run Heal in plan mode first before applying.</div>
        <div class="chip-row">
          <span id="dash-oracle-chip" class="chip neutral" tabindex="0" role="status">Oracle: idle</span>
          <span id="dash-heal-chip" class="chip neutral" tabindex="0" role="status">Heal: idle</span>
        </div>
        <input id="dash-oracle-query" placeholder="where is branch sync implemented?" />
        <div class="row">
          <button id="dash-oracle-scan-btn" class="btn secondary" onclick="dashOracleScan()">Oracle Scan</button>
          <button id="dash-oracle-query-btn" class="btn secondary" onclick="dashOracleQuery()">Oracle Query</button>
        </div>
        <div class="row">
          <input id="dash-heal-log-file" placeholder="test_output.json" value="test_output.json" />
          <input id="dash-heal-target" placeholder="optional target" />
        </div>
        <div class="row">
          <input id="dash-heal-max-attempts" placeholder="2" value="2" />
          <input id="dash-heal-max-files" placeholder="5" value="5" />
        </div>
        <div class="row">
          <button id="dash-heal-plan-btn" class="btn secondary" onclick="dashHealPlan()">Heal Plan</button>
          <button id="dash-heal-run-btn" class="btn" onclick="dashHealRun()">Heal Run</button>
        </div>
      </div>

      <div class="card">
        <h3>Branch + Multi Quick Ops</h3>
        <div class="helper">Use this block to preview branch creation/status across your registered cohort before any real apply step.</div>
        <div class="row">
          <input id="dash-branch-name" placeholder="feat/pilot-wave9" />
          <input id="dash-branch-base" placeholder="main" value="main" />
        </div>
        <div class="row">
          <input id="dash-branch-group" placeholder="core" />
          <input id="dash-branch-tags" placeholder="apply-pilot,wave9" />
        </div>
        <div class="row">
          <button class="btn secondary" onclick="dashBranchCreate()">Branch Create</button>
          <button class="btn secondary" onclick="branchStatus()">Branch Status</button>
          <button class="btn secondary" onclick="multiStatus()">Multi Status</button>
          <button class="btn secondary" onclick="multiOrder()">Multi Order</button>
        </div>
      </div>

      <div class="card">
        <h3>AGOrg Policy Reconcile</h3>
        <div class="helper">Generate policy artifact, preview prune impact, then apply reconciliation when ready.</div>
        <div class="row">
          <select id="dash-agorg-reconcile-class">
            <option value="">all classes</option>
            <option value="topology">topology (auto-fix)</option>
            <option value="policy_dependency">policy_dependency (manual)</option>
            <option value="policy_branch">policy_branch (manual)</option>
            <option value="metadata">metadata (manual)</option>
          </select>
        </div>
        <div class="row">
          <button class="btn secondary" onclick="dashAgorgPolicyReport()">Policy Report</button>
          <button class="btn secondary" onclick="dashAgorgReconcileDryRun()">Reconcile Dry Run</button>
          <button class="btn" onclick="dashAgorgReconcileApply()">Reconcile Apply</button>
        </div>
        <div class="row">
          <button class="btn secondary" onclick="dashAgorgPolicyReports()">Refresh Artifacts</button>
          <select id="dash-agorg-report-select"></select>
          <button class="btn secondary" onclick="dashAgorgPolicyOpen()">Open</button>
        </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-policy-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-policy-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-policy-out">No AGOrg policy action run yet.</pre>
      </div>

      <div class="card">
        <h3>AGOrg Action Contract</h3>
        <div class="helper">Guided preview -> approve -> execute -> reconcile flow from Dashboard for AGOrg policy actions.</div>
        <input id="dash-agorg-contract-intent" placeholder="Reconcile topology drift in active AGOrg" />
        <div class="row">
          <select id="dash-agorg-contract-command">
            <option value="api.agorg.reconcile_apply">api.agorg.reconcile_apply</option>
            <option value="api.agorg.policy_report">api.agorg.policy_report</option>
          </select>
          <select id="dash-agorg-contract-class">
            <option value="">all classes</option>
            <option value="topology">topology</option>
            <option value="policy_dependency">policy_dependency</option>
            <option value="policy_branch">policy_branch</option>
            <option value="metadata">metadata</option>
          </select>
          <label style="font-size:0.82rem;color:#a8b9e3;">
            <input id="dash-agorg-contract-dry-run" type="checkbox" checked style="width:auto;vertical-align:middle;margin-right:6px;" />
            dry-run
          </label>
        </div>
        <div class="row">
          <input id="dash-agorg-contract-id" placeholder="auto-filled after preview" />
          <button class="btn secondary" onclick="dashAgorgContractPreview()">Preview</button>
          <button class="btn secondary" onclick="dashAgorgContractApprove()">Approve</button>
          <button class="btn" onclick="dashAgorgContractExecute()">Execute</button>
          <button class="btn secondary" onclick="dashAgorgContractReconcile()">Reconcile</button>
        </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="dashAgorgContractOpenArtifact()">OPEN ARTIFACT</button>
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-contract-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-contract-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-contract-out">No AGOrg action contract run yet.</pre>
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-duplicates-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-duplicates-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-duplicates-out">No duplicate merge candidates yet.</pre>
      </div>
      <div class="row">
        <select id="dash-agorg-dup-kind-filter">
          <option value="all">All Duplicate Kinds</option>
          <option value="canonical_path">canonical_path</option>
          <option value="name">name</option>
        </select>
        <button class="btn secondary" onclick="dashAgorgApplyDuplicateFilter()">Apply Filter</button>
        <button class="btn secondary" onclick="dashAgorgPrevDuplicate()">Prev</button>
        <button class="btn secondary" onclick="dashAgorgNextDuplicate()">Next</button>
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-filtered-duplicates-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-filtered-duplicates-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-filtered-duplicates-out">No duplicate candidates for current filter.</pre>
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-duplicate-detail-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-duplicate-detail-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-duplicate-detail-out">No duplicate candidate selected.</pre>
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-class-counts-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-class-counts-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-class-counts-out">No issue class counts yet.</pre>
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-parity-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-parity-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-parity-out">No report/dry-run/apply parity summary yet.</pre>
      </div>
      <div class="row">
        <select id="dash-agorg-issue-class-filter">
          <option value="all">All Classes</option>
          <option value="policy_branch">policy_branch</option>
          <option value="policy_dependency">policy_dependency</option>
          <option value="metadata">metadata</option>
          <option value="topology">topology</option>
        </select>
        <button class="btn secondary" onclick="dashAgorgApplyIssueClassFilter()">Apply Filter</button>
        <button class="btn secondary" onclick="dashAgorgPrevIssue()">Prev</button>
        <button class="btn secondary" onclick="dashAgorgNextIssue()">Next</button>
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-filtered-issues-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-filtered-issues-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-filtered-issues-out">No filtered issues yet.</pre>
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-issue-detail-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-issue-detail-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-issue-detail-out">No issue selected.</pre>
      </div>
    </div>

    <div class="card">
        <h3>Operations Timeline</h3>
        <div class="row">
          <label style="font-size:0.82rem;color:#a8b9e3;">
            <input id="failed-only" type="checkbox" style="width:auto;vertical-align:middle;margin-right:6px;" />
            failed only
          </label>
          <button class="btn secondary" onclick="exportTimeline()">Export Filtered JSON</button>
        </div>
        <input id="timeline-command-filter" placeholder="filter command (e.g. pilot.branch)" />
        <input id="timeline-text-filter" placeholder="filter op id or summary text" />
        <div id="timeline" class="timeline">
          <div class="tl-empty">No operations yet</div>
        </div>
      </div>
      <div class="card">
        <h3>Operation Detail</h3>
        <div id="op-detail-meta" class="muted">Select a timeline item</div>
        <div id="op-detail-artifact" class="muted"></div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="openSelectedTimelineArtifact()">OPEN ARTIFACT</button>
          <button class="action-btn" onclick="copyToClipboard('op-detail', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('op-detail')">CLEAR</button>
        </div>
        <pre id="op-detail">[]</pre>
      </div>
      </div>
    </div>

    <div class="card">
      <h3>Live Event Stream</h3>
      <div class="row">
        <button id="stream-toggle" class="btn secondary" onclick="toggleStream()">Pause Stream</button>
        <button class="btn secondary" onclick="clearLive()">Clear</button>
      </div>
    <div class="pre-wrap">
      <div class="pre-actions">
        <button class="action-btn" onclick="copyToClipboard('live-stream', this)">COPY</button>
        <button class="action-btn" onclick="clearElement('live-stream')">CLEAR</button>
      </div>
      <pre id="live-stream">[]</pre>
    </div>
    </div>
    </div>
    </div>
  </section>

  <section class="panel" id="oracle">
    <div id="oracle-empty-state" class="empty-state-notice" aria-live="polite"></div>
    <div class="sequence-strip">
      <span class="seq-step">Scan Index</span>
      <span class="seq-step">Run Query</span>
      <span class="seq-step">Open Report</span>
    </div>
    <div class="grid">
      <div class="card">
        <h3>Oracle Scan / Query</h3>
        <div class="helper" id="oracle-scan-helper">`Scan Index` refreshes your code graph/vector index. `Run Query` asks Oracle over that indexed state.</div>
        <div class="chip-row">
          <span id="oracle-chip" class="chip neutral" tabindex="0" role="status">Oracle: idle</span>
        </div>
        <button id="oracle-scan-btn" class="btn" onclick="oracleScan()" aria-describedby="oracle-scan-helper">Scan Index</button>
        <details class="subtle-block" style="margin-top:12px;">
          <summary style="cursor:pointer; color:var(--text-muted); font-size:0.86rem; margin-bottom:8px;">Advanced: Query</summary>
          <input id="oracle-query" placeholder="where is branch sync implemented?" />
          <button id="oracle-query-btn" class="btn secondary" onclick="oracleQuery()">Run Query</button>
        </details>
      </div>
      <div class="card">
        <details class="subtle-block">
          <summary style="cursor:pointer; color:var(--text-muted); font-size:0.86rem; margin-bottom:8px;">Advanced: Oracle Reports</summary>
          <h3>Oracle Reports</h3>
          <div class="row">
            <button class="btn secondary" onclick="oracleLoadReports()">Refresh</button>
            <button class="btn secondary" onclick="oracleViewReport()">View</button>
          </div>
          <select id="oracle-report-select"></select>
          <div class="pre-wrap">
            <div class="pre-actions">
              <button class="action-btn" onclick="copyToClipboard('oracle-report-content', this)">COPY</button>
              <button class="action-btn" onclick="clearElement('oracle-report-content')">CLEAR</button>
            </div>
            <pre id="oracle-report-content">No report selected.</pre>
          </div>
        </details>
      </div>
    </div>
  </section>

  <section class="panel" id="heal">
    <div id="heal-empty-state" class="empty-state-notice" aria-live="polite"></div>
    <div class="sequence-strip">
      <span class="seq-step">Plan Only</span>
      <span class="seq-step">Review Response/Timeline</span>
      <span class="seq-step">Run Heal</span>
    </div>
    <div class="grid">
      <div class="card">
        <h3>Heal Controls</h3>
        <div class="helper" id="heal-plan-helper">Recommended sequence: `Plan Only` first, inspect response/timeline, then `Run Heal` only when the plan is acceptable.</div>
        <div class="chip-row">
          <span id="heal-chip" class="chip neutral" tabindex="0" role="status">Heal: idle</span>
        </div>
        <details class="subtle-block" style="margin-top:12px; margin-bottom:12px;">
          <summary style="cursor:pointer; color:var(--text-muted); font-size:0.86rem; margin-bottom:8px;">Advanced Options</summary>
          <input id="heal-log-file" placeholder="test_output.json" value="test_output.json" />
          <input id="heal-target" placeholder="optional target file/crate" />
          <div class="row">
            <input id="heal-max-attempts" placeholder="max attempts" value="2" />
            <input id="heal-max-files" placeholder="max files (plan mode)" value="5" />
          </div>
          <label style="font-size:0.82rem;color:#a8b9e3;">
            <input id="heal-verbose" type="checkbox" style="width:auto;vertical-align:middle;margin-right:6px;" />
            verbose output
          </label>
        </details>
        <div class="row">
          <button id="heal-plan-btn" class="btn secondary" onclick="healPlan()" aria-describedby="heal-plan-helper">Plan Only</button>
          <button id="heal-run-btn" class="btn" onclick="healRun()" aria-describedby="heal-plan-helper">Run Heal</button>
        </div>
      </div>
      <div class="card">
        <h3>Heal Notes</h3>
        <pre>Read-only UI mode forces plan-only behavior.
To allow applying fixes from UI/API:
pilot serve ... --ui-allow-mutations

Recommended flow:
1) Plan Only
2) Review output + timeline
3) Run Heal (when safe)</pre>
      </div>
    </div>
  </section>

  <section class="panel" id="dependencies">
    <div id="dep-empty-state" class="empty-state-notice" aria-live="polite"></div>
    <div class="grid">
      <div class="card">
        <h3>Checks and Recovery</h3>
        <div class="helper" id="dep-policy-helper" style="margin-bottom:12px;">Run Policy Check to verify dependencies and lockfiles across the active AGOrg against policy rules.</div>
        <div id="dep-status-grid" class="dep-status-grid">
          <div class="dep-status-card">
            <h4>Policy</h4>
            <div id="dep-policy-status" class="muted">unknown</div>
          </div>
          <div class="dep-status-card">
            <h4>Hook Policy</h4>
            <div id="dep-hook-status" class="muted">unknown</div>
          </div>
          <div class="dep-status-card">
            <h4>Drift</h4>
            <div id="dep-drift-status" class="muted">unknown</div>
          </div>
        </div>
        <div class="row">
          <button class="btn secondary" onclick="depRun('policy')" aria-describedby="dep-policy-helper">Policy Check</button>
        </div>
        <details class="subtle-block" style="margin-top:12px; margin-bottom:12px;">
          <summary style="cursor:pointer; color:var(--text-muted); font-size:0.86rem; margin-bottom:8px;">Advanced Commands</summary>
          <div class="row">
            <button class="btn secondary" onclick="depRun('hook-policy')">Hook Policy</button>
            <button class="btn secondary" onclick="depRun('drift')">Drift Report</button>
          </div>
          <div class="row" style="margin-top:8px;">
            <button class="btn secondary" onclick="depRun('gate')">Run Gate</button>
            <button class="btn" onclick="depRun('repair')">Repair Lock (No Gate)</button>
          </div>
        </details>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dep-action-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dep-action-out')">CLEAR</button>
        </div>
        <pre id="dep-action-out">No dependency action run yet.</pre>
      </div>
      </div>
      <div class="card">
        <h3>Recent Gate Logs</h3>
        <button class="btn secondary" onclick="depLoadLogs()">Refresh Logs</button>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dep-logs', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dep-logs')">CLEAR</button>
        </div>
        <pre id="dep-logs">[]</pre>
      </div>
      </div>
    </div>
  </section>

  <section class="panel" id="branch">
    <div class="grid">
      <div class="card" style="grid-column: 1 / -1;">
        <h3>Fleet Branch Matrix</h3>
        <div class="helper">Use search + base branch for daily flow. Open Advanced Filters only when you need cohort scoping.</div>
        <div class="row">
          <input id="branch-matrix-search" placeholder="search repo/path" />
          <input id="branch-matrix-base" placeholder="compare against branch" value="main" />
        </div>
        <details id="branch-matrix-advanced" class="subtle-block">
          <summary style="cursor:pointer; color:var(--text-muted); font-size:0.86rem;">Advanced Filters (group/tags)</summary>
          <div class="row" style="margin-top:8px;">
            <input id="branch-matrix-group" placeholder="group (optional)" />
            <input id="branch-matrix-tags" placeholder="tags (comma-separated)" />
          </div>
        </details>
        <div class="row">
          <button id="branch-matrix-refresh-btn" class="btn secondary" onclick="branchLoadMatrix()">Refresh Matrix</button>
          <button class="btn secondary" onclick="branchSelectVisible()">Select Visible</button>
          <button class="btn secondary" onclick="branchClearSelection()">Clear Selection</button>
        </div>
        <div class="chip-row">
          <span id="branch-matrix-source-chip" class="chip neutral" tabindex="0" role="status">Matrix Source: unknown</span>
        </div>
        <div id="branch-matrix-summary" class="helper">No matrix loaded yet.</div>
      <div class="pre-wrap">
        <div style="max-height: 320px; overflow: auto; border: 1px solid var(--border); border-radius: 8px;">
          <table class="branch-matrix-table">
            <thead>
              <tr>
                <th>Sel</th>
                <th>Repo</th>
                <th>Group</th>
                <th>Tags</th>
                <th>Branch</th>
                <th>Protected</th>
                <th>Clean</th>
                <th>Ahead</th>
                <th>Behind</th>
                <th>On Target</th>
              </tr>
            </thead>
            <tbody id="branch-matrix-body">
              <tr><td colspan="10" class="muted">No data loaded.</td></tr>
            </tbody>
          </table>
        </div>
      </div>
      </div>
      <div class="card">
        <h3>Create Branch</h3>
        <div class="helper">Use Preview first, then Execute when response looks correct.</div>
        <div id="branch-preview-state" class="helper">No active preview token.</div>
        <div class="chip-row">
          <span id="branch-create-chip" class="chip neutral" tabindex="0" role="status">Create: idle</span>
        </div>
        <div class="helper">Branch name</div>
        <input id="branch-name" placeholder="feat/pilot-wave7" />
        <div class="helper">Base branch</div>
        <input id="branch-base" placeholder="main" value="main" />
        <div class="row">
          <button id="branch-create-preview-btn" class="btn secondary" onclick="branchCreatePreview()">Preview</button>
          <button id="branch-create-exec-btn" class="btn" onclick="branchCreateExecute()">Execute</button>
        </div>
      </div>
      <div class="card">
        <h3>Sync / Prune / Status</h3>
        <div class="chip-row">
          <span id="branch-sync-chip" class="chip neutral" tabindex="0" role="status">Sync: idle</span>
          <span id="branch-prune-chip" class="chip neutral" tabindex="0" role="status">Prune: idle</span>
          <span id="branch-status-chip" class="chip neutral" tabindex="0" role="status">Status: idle</span>
        </div>
        <div class="helper">Target branch to sync</div>
        <input id="sync-branch" placeholder="dev" value="dev" />
        <div class="helper">Base branch for sync/prune</div>
        <input id="sync-base" placeholder="main" value="main" />
        <div class="row">
          <button id="branch-sync-preview-btn" class="btn secondary" onclick="branchSyncPreview()">Sync Preview</button>
          <button id="branch-sync-exec-btn" class="btn" onclick="branchSyncExecute()">Sync Execute</button>
        </div>
        <div class="row">
          <button id="branch-prune-preview-btn" class="btn secondary" onclick="branchPrunePreview()">Prune Preview</button>
          <button id="branch-prune-exec-btn" class="btn" onclick="branchPruneExecute()">Prune Execute</button>
          <button id="branch-status-btn" class="btn secondary" onclick="branchStatus()">Status</button>
        </div>
      </div>

      <div class="card">
        <h3>Undo Journal</h3>
        <div class="helper">Recent destructive branch operations that can be reverted.</div>
        <div class="chip-row">
          <span id="branch-undo-chip" class="chip neutral" tabindex="0" role="status">Undo: idle</span>
        </div>
        <div class="pre-wrap" style="margin-top: 10px;">
          <div style="max-height: 250px; overflow: auto; border: 1px solid var(--border); border-radius: 8px;">
            <table class="branch-matrix-table">
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Action</th>
                  <th>Repo</th>
                  <th>Branch</th>
                  <th>Prior Ref</th>
                  <th>Status</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody id="branch-undo-body">
                <tr><td colspan="7" class="muted">Loading undo journal...</td></tr>
              </tbody>
            </table>
          </div>
        </div>
      <div class="card">
        <h3>Conflict Radar</h3>
        <div class="helper">Identify potential merge conflicts across all repositories before executing sync or merge.</div>
        <div class="chip-row">
          <span id="branch-radar-chip" class="chip neutral" tabindex="0" role="status">Radar: idle</span>
        </div>
        <div class="row">
          <div style="flex:1;min-width:180px;">
            <div class="helper">Radar branch</div>
            <input id="branch-radar-input" placeholder="feat/pilot-wave13" value="feat/pilot-wave13" />
          </div>
          <div style="flex:1;min-width:140px;">
            <div class="helper">Radar base</div>
            <input id="branch-radar-base" placeholder="dev" value="dev" />
          </div>
        </div>
        <div class="row">
          <button id="branch-radar-btn" class="btn secondary" onclick="branchConflictRadarRun()">Run Conflict Radar</button>
        </div>
        <div id="branch-radar-results" style="margin-top: 10px; max-height: 300px; overflow: auto; border: 1px solid var(--border); border-radius: 8px; padding: 10px;">
          <div class="muted">Enter branch and run radar to detect conflicts.</div>
        </div>
      </div>

      <div class="card">
        <h3>Branch Timeline</h3>
        <div class="helper">Chronological history of branch mutations.</div>
        <div class="row">
          <button id="branch-timeline-refresh-btn" class="btn secondary" onclick="branchTimelineLoad()">Refresh Timeline</button>
        </div>
        <div id="branch-timeline-list" style="margin-top: 10px; max-height: 400px; overflow: auto; border: 1px solid var(--border); border-radius: 8px; padding: 10px;">
          <div class="muted">No timeline events loaded.</div>
        </div>
      </div>
      <div class="card">
        <h3>Dependency DAG + Staged Apply (Primary Branch Flow)</h3>
        <div class="helper">Run DAG preview, then staged apply preview, then staged apply execute when approved.</div>
        <div class="chip-row">
          <span id="branch-dag-chip" class="chip neutral" tabindex="0" role="status">DAG: idle</span>
          <span id="branch-apply-chip" class="chip neutral" tabindex="0" role="status">Staged Apply: idle</span>
        </div>
        <div class="row">
          <div style="flex:1;min-width:180px;">
            <div class="helper">Apply branch</div>
            <input id="branch-apply-branch" placeholder="feat/pilot-wave13" value="feat/pilot-wave13" />
          </div>
          <div style="flex:1;min-width:140px;">
            <div class="helper">Apply base</div>
            <input id="branch-apply-base" placeholder="dev" value="dev" />
          </div>
          <div style="flex:1;min-width:140px;">
            <div class="helper">PR base</div>
            <input id="branch-apply-pr-base" placeholder="main" value="main" />
          </div>
        </div>
        <div class="row">
          <div style="max-width:180px;">
            <div class="helper">Stage size</div>
            <input id="branch-apply-stage-size" placeholder="2" value="2" />
          </div>
          <label style="font-size:0.82rem;color:#a8b9e3;">
            <input id="branch-apply-continue" type="checkbox" style="width:auto;vertical-align:middle;margin-right:6px;" />
            Continue on failure
          </label>
        </div>
        <div class="row">
          <button id="branch-dag-btn" class="btn secondary" onclick="branchDagPreview()">DAG Preview</button>
          <button id="branch-apply-preview-btn" class="btn secondary" onclick="branchApplyPreview()">Staged Apply Preview</button>
          <button id="branch-apply-exec-btn" class="btn" onclick="branchApplyExecute()">Staged Apply Execute</button>
        </div>
      </div>
      <div class="card">
        <h3>Branch Action Output</h3>
        <div class="row">
          <label class="field-label" for="branch-log-limit">Max logs</label>
          <input id="branch-log-limit" type="number" min="1" max="100" value="50" style="max-width:110px;" />
          <button class="btn secondary" onclick="branchClearHtmlLog()">Clear Logs</button>
        </div>
        <div id="branch-log-summary" class="helper">No branch activity entries yet.</div>
        <div id="branch-log-list" style="display:flex;flex-direction:column;gap:10px;max-height:420px;overflow:auto;border:1px solid var(--border);border-radius:8px;padding:10px;background:rgba(0,0,0,0.25);">
          <div class="muted">No branch action run yet.</div>
        </div>
      </div>
    </div>
    <div id="branch-prune-modal" class="modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="branch-prune-modal-title">
      <div class="modal-card">
        <h4 id="branch-prune-modal-title" class="modal-title">Confirm Destructive Prune</h4>
        <div class="helper">Type <code>PRUNE</code> to confirm execute. This deletes merged branches.</div>
        <input id="branch-prune-confirm-input" placeholder="Type PRUNE to confirm" />
        <div class="row">
          <button class="btn secondary" onclick="branchCancelPruneConfirm()">Cancel</button>
          <button class="btn" onclick="branchConfirmPruneExecute()">Confirm Prune Execute</button>
        </div>
      </div>
    </div>
  </section>

  <section class="panel" id="multi">
    <div id="multi-empty-state" class="empty-state-notice" aria-live="polite"></div>
    <div class="sequence-strip">
      <button class="seq-step seq-step-btn" onclick="multiMacroFleetFlow()">List > Status > Order > DAG > PR Plan</button>
      <span class="seq-step">Staged Apply (Dry Run -> Execute)</span>
    </div>
    <div class="pre-wrap" style="margin-bottom:12px;">
      <div class="pre-actions">
        <button id="multi-macro-toggle-btn" class="action-btn" onclick="toggleMultiMacroLog()">🔽 Expand Macro Telemetry</button>
        <button class="action-btn" onclick="copyToClipboard('multi-macro-log-out', this)">📋 Copy</button>
        <button class="action-btn" onclick="clearElement('multi-macro-log-out')">🧹 Clear</button>
      </div>
      <pre id="multi-macro-log-out" class="term-out" style="display:none;" role="status" aria-live="polite" tabindex="-1">Macro telemetry ready.</pre>
    </div>
    <div class="grid">
      <div class="card">
        <h3>List / Status / Order / DAG / PR Plan</h3>
        <div class="helper" id="multi-list-helper"></div>
        <div class="chip-row">
          <span id="multi-registry-chip" class="chip neutral" tabindex="0" role="status">Registry: --</span>
          <span id="multi-dag-chip" class="chip neutral" tabindex="0" role="status">DAG: idle</span>
        </div>

        <div class="form-row">
          <div class="form-label">Group</div>
          <div class="form-content">
            <input id="multi-group" list="multi-group-options" placeholder="core" />
            <datalist id="multi-group-options"></datalist>
          </div>
        </div>

        <div class="form-row">
          <div class="form-label">Tags</div>
          <div class="form-content">
            <input id="multi-tags" list="multi-tag-options" placeholder="apply-pilot,wave7" />
            <datalist id="multi-tag-options"></datalist>
          </div>
        </div>

        <div class="row">
          <button id="multi-list-btn" class="btn secondary" onclick="multiList()" aria-describedby="multi-list-helper">List</button>
          <button id="multi-status-btn" class="btn secondary" onclick="multiStatus()">Status</button>
          <button id="multi-order-btn" class="btn secondary" onclick="multiOrder()">Order</button>
          <button id="multi-dag-btn" class="btn secondary" onclick="multiDag()">DAG</button>
        </div>
        <button id="multi-pr-plan-btn" class="btn" onclick="multiPrsCreate()">PR Plan (Dry Run)</button>
        <div class="pre-wrap" style="margin-top:12px;">
          <div class="pre-actions">
            <button id="multi-output-html" class="action-btn" onclick="multiSetOutputMode('html')">HTML</button>
            <button id="multi-output-json" class="action-btn" onclick="multiSetOutputMode('json')">JSON</button>
            <button class="action-btn" onclick="copyToClipboard('multi-action-out', this)">COPY</button>
            <button class="action-btn" onclick="clearElement('multi-action-out')">CLEAR</button>
          </div>
          <pre id="multi-action-out" class="term-out" tabindex="-1">ready</pre>
        </div>
        <div class="multi-dag-visual-wrap" aria-live="polite">
          <div class="helper" style="margin-bottom:8px;">Dependency map (stages and edges)</div>
          <div id="multi-dag-visual-empty" class="multi-dag-visual-empty">Run `DAG` or a Multi macro to render the dependency map.</div>
          <div id="multi-dag-visual-scroll" class="multi-dag-visual-scroll" style="display:none;">
            <svg id="multi-dag-visual" role="img" aria-label="Multi dependency DAG visual"></svg>
          </div>
        </div>
      </div>
      <div class="card">
        <h3>Staged Apply (Dependency-Aware)</h3>
        <div class="helper">Runs branch creation in dependency stages. Start with `Dry Run`; use `Execute` only after preview looks correct.</div>
        <div class="chip-row">
          <span id="multi-apply-chip" class="chip neutral" tabindex="0" role="status">Staged Apply: idle</span>
        </div>

        <div class="form-row">
          <div class="form-label">Branch</div>
          <div class="form-content">
            <input id="multi-apply-branch" placeholder="feat/pilot-wave13" value="feat/pilot-wave13" />
          </div>
        </div>

        <div class="form-row">
          <div class="form-label">Base</div>
          <div class="form-content">
            <input id="multi-apply-base" placeholder="dev" value="dev" />
          </div>
        </div>

        <div class="form-row">
          <div class="form-label">PR Base</div>
          <div class="form-content">
            <input id="multi-apply-pr-base" placeholder="main" value="main" />
          </div>
        </div>

        <div class="form-row">
          <div class="form-label">Size</div>
          <div class="form-content" style="display:flex; align-items:center; gap:12px;">
            <input id="multi-apply-stage-size" placeholder="2" value="2" style="width: 80px;" />
            <label style="font-size:0.82rem;color:#a8b9e3; display:flex; align-items:center; gap:6px; cursor:pointer; white-space:nowrap;">
              <input id="multi-apply-continue" type="checkbox" style="width:auto;margin:0;" />
              Continue on failure
            </label>
          </div>
        </div>

        <div class="row">
          <button class="btn secondary" onclick="multiApplyDryRun()">Staged Apply (Dry Run)</button>
          <button class="btn" onclick="multiApplyExecute()">Staged Apply (Execute)</button>
        </div>
      </div>
    </div>
  </section>

  <section class="panel" id="agorg">
    <div class="sequence-strip">
      <button class="seq-step seq-step-btn" onclick="agorgRefreshActive()" title="Sync active scope and registry list.">Quick Sync</button>
      <button class="seq-step seq-step-btn" onclick="agorgMacroImportDiscover()" title="Switch to Onboarding tab and pick a directory.">Import > Discover</button>
      <button class="seq-step seq-step-btn" onclick="agorgMacroCreateNew()" title="Switch to Onboarding tab and focus creation options.">Create New</button>
      <button class="seq-step seq-step-btn" onclick="agorgReconcile()" title="Run policy reconciliation report.">Policy Report > Reconcile</button>
    </div>

    <!-- Row 1: Active Scope + Registry (50/50) -->
    <div class="grid" style="grid-template-columns: 1fr;">
      <div class="card" style="display:flex; flex-direction:column;">
        <h3>Registry</h3>
        
        <!-- Active Scope Metadata Display (Embedded in Registry) -->
        <div class="helper">Current Active Scope:</div>
        <div id="agorg-active-details" style="background:rgba(0,0,0,0.3); border-radius:8px; padding:12px; border:1px solid var(--border); font-size:0.8rem; font-family:'JetBrains Mono',monospace; word-break:break-all; margin-bottom:12px;">
          <em style="color:var(--dim);">Loading active scope...</em>
        </div>
        <div class="row" style="margin-bottom:20px; flex-wrap:wrap;">
          <button class="btn" onclick="agorgOpenEditModal()">Edit</button>
          <button class="btn secondary" onclick="agorgRefreshActive()">Refresh</button>
          <button class="btn secondary" style="border-color:rgba(255,255,255,0.15);" onclick="agorgRemoveSelected()" title="Remove this AGOrg from the Registry.">Remove</button>
          <button class="btn" style="margin-left:auto; background:rgba(255,46,46,0.1); border-color:rgba(255,46,46,0.3); color:#ff6b6b; font-size:0.75rem;" onclick="agorgResetDb()" title="Wipe all AGOrg and AGO records from the database. This is for testing only.">⚠️ Reset Database</button>
        </div>

        <div class="helper">Click to switch scope instantly. AGOs are nested under their parent AGOrgs.</div>
        <div id="agorg-registry-list" class="agorg-registry-list" style="flex:1; overflow-y:auto; background:rgba(0,0,0,0.3); border:1px solid var(--border); border-radius:8px; min-height:300px;">
          <div style="padding:14px; color:var(--dim); font-size:0.78rem; font-family:'JetBrains Mono',monospace;">Loading registry...</div>
        </div>
      </div>
    </div>

    <!-- Row 2: Import / Create New (full width) -->
    <div class="card" style="margin-top:24px;">
      <h3>AGOrg Management</h3>
      <div class="sub-tabs" style="margin-top:10px;">
        <button class="sub-tab active" onclick="activateSubPanel('agorg-onboarding-panel', this)">ONBOARDING</button>
        <button class="sub-tab" onclick="activateSubPanel('agorg-repo-registry-panel', this)">AGO REGISTRY</button>
        <button class="sub-tab" onclick="activateSubPanel('agorg-governance-panel', this)">GOVERNANCE</button>
      </div>

      <!-- Sub-Panel: Onboarding -->
      <div id="agorg-onboarding-panel" class="sub-panel active">
        <div class="helper">Onboard a collective by either discovering an existing Master Directory or creating a new one from scratch.</div>

        <div class="grid" style="margin-top:16px; grid-template-columns: 1fr;">
          <!-- Section 1: Master Directory -->
          <div class="section-box">
            <h4>MASTER DIRECTORY (ORG)</h4>
            <div class="row">
              <input id="agorg-master" placeholder="/path/to/master/dir (existing or new)" value="" onchange="agorgDiscoverPreview()" />
              <button class="btn secondary" onclick="browseAgorgMaster()">Browse…</button>
              <button class="btn secondary" onclick="agorgDiscoverPreview()">Scan / Discover</button>
            </div>
          </div>
        </div>

        <!-- Section: Discovery Review -->
        <div class="section-box" style="margin-top:16px;">
          <h4>BRANCH & LEAF DIRECTORIES</h4>
          <div class="row" style="justify-content: space-between;">
            <div class="row">
              <button class="btn secondary" onclick="agorgSelectAllReview(true)">Select All</button>
              <button class="btn secondary" onclick="agorgSelectAllReview(false)">Deselect All</button>
            </div>
            <div class="row">
               <div class="btn-glow-wrap" style="margin-left:auto;">
                 <button class="btn btn-jumbo" onclick="agorgImportApproved()">Import Approved</button>
               </div>
            </div>
          </div>
          <div id="agorg-discovery-review" class="timeline" style="padding: 10px; border: 1px solid var(--border); border-radius: 8px; background: rgba(0,0,0,0.2);">
            <div class="tl-empty">Enter a path and click Scan / Discover to review project candidates.</div>
          </div>
        </div>

        <!-- Section: Creation Options (Collapsible) -->
        <details class="subtle-block" id="agorg-creation-details" style="margin-top:20px;">
          <summary style="cursor:pointer; color:var(--text-muted); font-size:0.86rem; margin-bottom:8px; padding: 8px; background: rgba(0,0,0,0.1); border-radius: 4px;">Advanced: Creation Options (Batch Instantiate)</summary>
          <div class="section-box" style="margin-top:0;">
            <div class="helper" style="margin-bottom:12px;">If the Master Directory does not exist, use these options to bootstrap it with sibling AGOs.</div>
            
            <label class="field-label" for="agorg-create-siblings">Sibling AGOs (one per line)</label>
            <textarea id="agorg-create-siblings" class="batch-list" placeholder="Core&#10;Pilot&#10;Sense" style="min-height:80px;"></textarea>
            
            <label class="check-label" style="margin-top:10px;">
              <input id="agorg-create-git" type="checkbox" checked />
              git init each project
            </label>

            <div class="row" style="margin-top:16px;">
              <button class="btn" style="background:rgba(0, 245, 255, 0.1); border-color:rgba(0, 245, 255, 0.3);" onclick="agorgBatchCreate()">Batch Create & Register</button>
            </div>
          </div>
        </details>

      </div>

      <!-- Sub-Panel: Governance -->
      <div id="agorg-governance-panel" class="sub-panel">
        <div class="helper">Audit and reconcile policy drift across the collective. Use `Policy Report` to scan and `Reconcile Apply` to resolve auto-fixable issues.</div>
        <div class="section-box" style="margin-top:16px;">
          <div class="row">
            <select id="agorg-reconcile-class">
              <option value="">all classes</option>
              <option value="topology">topology (auto-fix)</option>
              <option value="policy_dependency">policy_dependency (manual)</option>
              <option value="policy_branch">policy_branch (manual)</option>
              <option value="metadata">metadata (manual)</option>
            </select>
            <button class="btn secondary" onclick="agorgReconcile()">Policy Report</button>
            <button class="btn secondary" onclick="agorgReconcileApply()">Reconcile Apply</button>
          </div>
          <div class="row">
            <button class="btn secondary" onclick="agorgLoadPolicyReports()">Refresh Policy Artifacts</button>
            <select id="agorg-policy-report-select"></select>
            <button class="btn secondary" onclick="agorgOpenPolicyReport()">Open</button>
          </div>
        </div>
      </div>

      <!-- Sub-Panel: AGO Registry -->
      <div id="agorg-repo-registry-panel" class="sub-panel">
        <div class="helper">Register AGOs under the active AGOrg, then use Group/Tags in Multi for orchestration.</div>
        <div class="section-box" style="margin-top:16px;">
          <h4>REGISTER AGO</h4>
          <div class="form-row">
            <div class="form-label">Path</div>
            <div class="form-content">
              <input id="repo-path" placeholder="/path/to/repo" />
              <button class="btn secondary" style="padding: 9px 12px;" onclick="browseRepoPath()">Browse</button>
            </div>
          </div>

          <div class="form-row">
            <div class="form-label">Name</div>
            <div class="form-content">
              <select id="repo-name" onchange="repoSelectAgo()">
                <option value="">Select AGO from active AGOrg…</option>
              </select>
            </div>
          </div>

          <div class="form-row">
            <div class="form-label">Group</div>
            <div class="form-content">
              <input id="repo-group" placeholder="core" />
            </div>
          </div>

          <div class="form-row">
            <div class="form-label">Tags</div>
            <div class="form-content">
              <input id="repo-tags" placeholder="apply-pilot,wave7" />
            </div>
          </div>

          <button class="btn" onclick="multiRegister()">Register</button>
          <div id="multi-register-actions" class="row" style="margin-top: 12px; display: none; justify-content: flex-end;">
            <button class="btn secondary" style="padding: 4px 8px; font-size: 0.65rem;" onclick="copyMultiRegister()">Copy</button>
            <button class="btn secondary" style="padding: 4px 8px; font-size: 0.65rem;" onclick="clearMultiRegister()">Clear</button>
          </div>
          <pre id="multi-register-out" class="term-out" style="display:none;"></pre>
        </div>
      </div>

    </div>

    <!-- Row 4: Activity Log -->
    <div class="card" style="margin-top:24px;">
      <h3>Activity Log</h3>
      <div class="pre-wrap" style="background: rgba(0,0,0,0.2);">
        <div class="pre-actions">
          <button class="action-btn" onclick="clearElement('agorg-activity-log')">CLEAR LOG</button>
        </div>
        <div id="agorg-activity-log" style="max-height: 400px; overflow-y: auto; padding: 10px; display: flex; flex-direction: column; gap: 8px;">
          <div style="color: var(--text-muted); font-style: italic; font-size: 0.9em; text-align: center;">Activity log started.</div>
        </div>
      </div>
    </div>

  </section>

  <!-- Modals -->
  <div id="agorg-edit-modal" class="modal-overlay">
    <div class="modal-box">
      <h3>Edit AGOrg Settings</h3>
      <div class="helper" style="margin-bottom:12px;">Update the metadata for the active scope. ID is immutable.</div>
      
      <div class="section-box">
        <label class="field-label">AGOrg ID (Read-only)</label>
        <div id="agorg-edit-id" style="padding:10px; background:rgba(0,0,0,0.2); border-radius:8px; font-family:monospace; color:var(--dim); font-size:0.85rem;"></div>
        
        <label class="field-label" for="agorg-edit-name">Display Name</label>
        <input id="agorg-edit-name" placeholder="My Projects" />
        
        <label class="field-label" for="agorg-edit-root">Root Directory</label>
        <div class="row">
          <input id="agorg-edit-root" placeholder="/path/to/root" />
          <button class="btn secondary" onclick="browseAgorgEditRoot()">Browse…</button>
        </div>
        
        <label class="field-label" for="agorg-edit-master">Master Directory (Optional)</label>
        <div class="row">
          <input id="agorg-edit-master" placeholder="/path/to/master" />
          <button class="btn secondary" onclick="browseAgorgEditMaster()">Browse…</button>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn secondary" onclick="agorgCloseEditModal()">Cancel</button>
        <button class="btn" onclick="agorgSaveEditModal()">Save Changes</button>
      </div>
    </div>
  </div>

  <div id="multi-scope-modal" class="modal-overlay">
    <div class="modal-box">
      <h3>Scope Required</h3>
      <div class="helper" style="margin-bottom:12px;">
        Set at least one selector before running this macro:
        <b>Group</b> or <b>Tags</b>.
      </div>
      <div class="section-box">
        <label class="field-label" for="multi-scope-modal-group">Group</label>
        <input id="multi-scope-modal-group" placeholder="core" autocomplete="off" />
        <label class="field-label" for="multi-scope-modal-tags">Tags (comma-separated)</label>
        <input id="multi-scope-modal-tags" placeholder="pilot,wave7" autocomplete="off" />
        <div class="helper" style="margin-top:8px;">Enter either Group, Tags, or both. Click Apply to update selectors.</div>
      </div>
      <div class="modal-footer">
        <button class="btn" onclick="applyMultiScopeModal()">Apply</button>
        <button class="btn secondary" onclick="closeMultiScopeModal()">Close</button>
      </div>
    </div>
  </div>

  <section class="panel" id="telemetry">
    <div class="grid">
      <div class="card">
        <h3>Telemetry Mirror</h3>
        <div class="row">
          <button class="btn secondary" onclick="syncTelemetryMirror()">Refresh Mirror</button>
        </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('telemetry-mirror', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('telemetry-mirror')">CLEAR</button>
        </div>
        <pre id="telemetry-mirror">[]</pre>
      </div>
      </div>
      <div class="card">
        <h3>Telemetry Mode</h3>
        <pre id="telemetry-mode">Dashboard stream is active. Use this tab for mirrored view and quick inspection.</pre>
      </div>
    </div>
  </section>

  <section class="panel" id="codex">
    <div class="grid">
      <div class="card">
        <h3>Codex Guided Flow</h3>
        <div class="helper">Use this sequence every time: <b>Preview</b> (safe) -> <b>Approve</b> -> <b>Execute</b> -> <b>Reconcile</b>.</div>

        <label class="field-label" for="codex-contract-id">Current Contract ID</label>
        <input id="codex-contract-id" placeholder="auto-filled after preview" />
        <div class="helper">Leave blank for a new contract. Use a Contract ID to resume/replay existing work.</div>

        <div class="step">
          <div class="step-title">Step 1. Define intent and preview (no execution)</div>
          <label class="field-label" for="codex-intent">Intent (required)</label>
          <input id="codex-intent" placeholder="what you want to achieve" value="Check multi-repo status before branch action" />
          <label class="field-label" for="codex-command">Pilot Command (required)</label>
          <input id="codex-command" placeholder="pilot.command" value="pilot.multi.status" />
          <label class="field-label" for="codex-payload">Payload JSON</label>
          <textarea id="codex-payload" rows="8" placeholder='{"group":"core","tags":["apply-pilot"]}'>{ "group": "core", "tags": ["apply-pilot"] }</textarea>
          <button class="btn secondary" onclick="codexPreview()">1) Preview Contract (Safe)</button>
        </div>

        <div class="step">
          <div class="step-title">Step 2. Approve execution contract</div>
          <label class="field-label" for="codex-expected">Expected Effect</label>
          <input id="codex-expected" placeholder="what success looks like" value="Status summary is returned for the core cohort" />
          <label class="field-label" for="codex-rollback">Rollback Strategy</label>
          <input id="codex-rollback" placeholder="how to safely undo" value="No repo mutation expected; rerun in preview mode if uncertain" />
          <label class="field-label" for="codex-verify">Verify Command (optional)</label>
          <input id="codex-verify" placeholder="pilot.command for post-run verification" value="pilot.multi.status" />
          <button class="btn secondary" onclick="codexApprove()">2) Approve Contract</button>
        </div>

        <div class="step">
          <div class="step-title">Step 3. Execute and reconcile</div>
          <label class="field-label" for="codex-reconcile-notes">Reconcile Notes</label>
          <input id="codex-reconcile-notes" placeholder="what was verified and closed" value="Outcome reviewed in timeline and response payload." />
          <div class="row">
            <button class="btn" onclick="codexExecute()">3) Execute Approved Contract</button>
            <button class="btn secondary" onclick="codexReconcile()">4) Reconcile and Close</button>
          </div>
        </div>
      </div>
      <div class="card">
        <h3>Codex Response</h3>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('codex-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('codex-out')">CLEAR</button>
        </div>
        <pre id="codex-out">No Codex action run yet.</pre>
      </div>
        <h3>Contracts (Resume / Replay)</h3>
        <div class="helper">Choose a past contract to reload state, then continue with approve/execute/reconcile.</div>
        <label class="field-label" for="codex-contract-filter">Filter by Status (optional)</label>
        <input id="codex-contract-filter" placeholder="status filter (optional): failed|approved|reconciled" />
        <div class="row">
          <button class="btn secondary" onclick="codexLoadContracts()">Refresh Contracts</button>
          <button class="btn secondary" onclick="codexLoadSelectedContract()">Load Selected Contract</button>
          <button class="btn secondary" onclick="codexRetryFailedContract()">Retry Failed (Approve + Execute)</button>
        </div>
        <label class="field-label" for="codex-contract-select">Available Contracts</label>
        <select id="codex-contract-select"></select>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('codex-contracts-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('codex-contracts-out')">CLEAR</button>
        </div>
        <pre id="codex-contracts-out">No contracts loaded yet.</pre>
      </div>
      </div>
    </div>
  </section>

  <section class="panel" id="settings">
    <div class="grid">
      <div class="card">
        <h3>Governance Configuration</h3>
        <p class="helper">Use this section to view inherited policies, manage overrides, and establish exceptions.</p>
        
        <label class="field-label" for="settings-policy-kind">Select Policy Type</label>
        <select id="settings-policy-kind" onchange="settingsReloadPolicyControls()">
          <option value="branch" selected>Branch Rules</option>
          <option value="dependency">Dependency & Lockfiles</option>
          <option value="release">Release Structure</option>
          <option value="security">Security & Secrets</option>
          <option value="quality">Code Quality</option>
          <option value="runtime">Runtime Env</option>
          <option value="operator_routine">Operator Routine</option>
        </select>

        <label class="field-label" for="settings-policy-target">Target AGO (auto-populated from active AGOrg scope)</label>
        <select id="settings-policy-target" onchange="settingsReloadPolicyControls()">
          <option value="">AGOrg level (no AGO override)</option>
        </select>

        <div class="row">
           <button class="btn secondary" onclick="settingsLoadPolicy()">Refresh Active Policy</button>
           <button class="btn" onclick="settingsDraftPolicy()">Save Draft</button>
           <button class="btn secondary" onclick="settingsSimulatePolicy()">Simulate Draft</button>
           <button class="btn action-btn" onclick="settingsActivatePolicy()" style="color:var(--rose);border-color:var(--rose)">Activate Policy</button>
        </div>
        <label class="field-label" for="settings-status-out">Settings Status</label>
        <div id="settings-status-panel" class="pre-wrap">
          <div class="pre-actions">
            <button class="action-btn" onclick="copyToClipboard('settings-status-out', this)">COPY</button>
            <button class="action-btn" onclick="clearElement('settings-status-out')">CLEAR</button>
          </div>
          <pre id="settings-status-out">ready</pre>
        </div>

        <label class="field-label" for="settings-policy-editor">Policy JSON (Draft / Active)</label>
        <textarea id="settings-policy-editor" placeholder="JSON policy definition" style="min-height:200px;"></textarea>
        
        <hr style="border-color:var(--border);margin:16px 0;" />
        <h4>Policy Versions (Precision CRUD)</h4>
        <p class="helper">Create/update with <code>Save Draft</code>. Read exact versions. Delete a specific version only after typing <code>DELETE</code>.</p>
        <div class="row">
          <button class="btn secondary" onclick="settingsLoadPolicyVersions()">Refresh Versions</button>
          <button class="btn secondary" onclick="settingsLoadSelectedPolicyVersion()">Load Selected Version</button>
        </div>
        <select id="settings-policy-versions" size="6" style="height:auto; min-height:120px;"></select>
        <div class="grid" style="grid-template-columns: 1fr auto; gap: 8px; margin-top: 10px;">
          <input id="settings-policy-delete-confirm" placeholder="Type DELETE to enable deletion" />
          <button class="btn action-btn" onclick="settingsDeleteSelectedPolicyVersion()" style="color:var(--rose);border-color:var(--rose)">Delete Selected Version</button>
        </div>
      </div>

      <div class="card">
         <h3>Active Exceptions</h3>
         <p class="helper">Bypass specific rules temporarily. Required: Owner, Reason, Expiration, and Ticketing Ref.</p>
         
         <div class="row">
           <button class="btn secondary" onclick="settingsLoadExceptions()">Refresh Exceptions</button>
           <button class="btn secondary" onclick="settingsDeleteException()">Revoke Selected</button>
         </div>
         
         <select id="settings-exceptions-list" size="4" style="height:auto; min-height:80px;"></select>

         <hr style="border-color:var(--border);margin:16px 0;" />

         <h4>Add New Exception</h4>
         <div class="grid" style="grid-template-columns: 1fr 1fr; gap: 12px; margin-bottom: 0;">
           <div>
             <label class="field-label">Rule Path (e.g. naming.required_prefix)</label>
             <input id="settings-exc-rule" type="text" placeholder="Rule to bypass" />
           </div>
           <div>
             <label class="field-label">Ticket Ref</label>
             <input id="settings-exc-ticket" type="text" placeholder="JIRA-123" />
           </div>
           <div>
             <label class="field-label">Owner</label>
             <input id="settings-exc-owner" type="text" placeholder="Email or LDAP" />
           </div>
           <div>
             <label class="field-label">Expires At</label>
             <input id="settings-exc-expires" type="date" />
           </div>
         </div>
         <label class="field-label">Reason</label>
         <input id="settings-exc-reason" type="text" placeholder="Why is this bypass needed?" />

         <div class="row" style="margin-top:8px;">
           <button class="btn" onclick="settingsAddException()">Add Exception</button>
         </div>

      </div>

      <div class="card">
         <h3>Compliance & Auditing</h3>
         <p class="helper">Run on-demand branch compliance scans, resolve local policies for current context, and explore decision logs.</p>
         
         <div class="row">
           <button class="btn secondary" onclick="settingsComplianceScan()">Run Compliance Scan</button>
           <button class="btn secondary" onclick="settingsResolvePolicy()">Resolve Local Policy</button>
           <button class="btn secondary" onclick="settingsExploreDecisions()">Explore Decisions</button>
         </div>
      </div>

      <div class="card">
        <h3>Override Registry</h3>
        <p class="helper">Manage AGOrg-level policy overrides across the fleet. Overrides supersede baseline and default policies.</p>
        <div class="row">
          <select id="settings-override-kind" onchange="settingsLoadOverrides()">
            <option value="branch" selected>Branch Rules</option>
            <option value="dependency">Dependency & Lockfiles</option>
            <option value="release">Release Structure</option>
            <option value="security">Security & Secrets</option>
            <option value="quality">Code Quality</option>
            <option value="runtime">Runtime Env</option>
            <option value="operator_routine">Operator Routine</option>
          </select>
          <button class="btn secondary" onclick="settingsLoadOverrides()">Refresh Overrides</button>
        </div>

        <div style="overflow-x:auto; margin-bottom: 16px;">
          <table class="branch-matrix-table" id="settings-overrides-table">
            <thead>
              <tr>
                <th>AGO Target</th>
                <th>Owner</th>
                <th>Reason</th>
                <th>Expires</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              <tr><td colspan="5" style="text-align:center; color:var(--muted)">No overrides loaded</td></tr>
            </tbody>
          </table>
        </div>

        <h4>Register New Override</h4>
        <div class="grid" style="grid-template-columns: 1fr 1fr; gap: 12px; margin-bottom: 0;">
          <div>
            <label class="field-label">Target AGO (Path)</label>
            <input id="settings-override-target" type="text" placeholder="e.g. core/engine" />
          </div>
          <div>
            <label class="field-label">Ticket Ref (Optional)</label>
            <input id="settings-override-ticket" type="text" placeholder="JIRA-123" />
          </div>
          <div>
            <label class="field-label">Expires At (Optional)</label>
            <input id="settings-override-expires" type="date" />
          </div>
        </div>
        <label class="field-label">Reason</label>
        <input id="settings-override-reason" type="text" placeholder="Why is this policy overridden?" />
        
        <label class="field-label">Override Policy JSON</label>
        <textarea id="settings-override-json" placeholder='{"level": "Warn"}' style="min-height:100px;"></textarea>

        <div class="row" style="margin-top:8px;">
          <button class="btn" onclick="settingsCreateOverride()">Create Override</button>
          <button class="btn secondary" onclick="settingsResolveTrace()">Resolve Policy Trace</button>
        </div>
      </div>

      <div class="card">
        <h3>Fleet-Wide Governance Health</h3>
        <p class="helper">Run a holistic fleet scan to verify compliance across all policy families.</p>
        <div class="row">
          <button class="btn secondary" onclick="settingsGovernanceScan()">Run Fleet Scan</button>
          <button class="btn secondary" onclick="settingsExportGovernanceReport()">Export Report</button>
        </div>
        
        <div class="chip-row">
            <span id="gov-scan-total-chip" class="chip neutral" tabindex="0" role="status">Scanned: unknown</span>
            <span id="gov-scan-violations-chip" class="chip neutral" tabindex="0" role="status">Violations: unknown</span>
        </div>

        <div style="overflow-x:auto;">
          <table class="branch-matrix-table" id="settings-gov-scan-table">
            <thead>
              <tr>
                <th>AGO Target</th>
                <th>Overall</th>
                <th>Branch</th>
                <th>Dep.</th>
                <th>Release</th>
                <th>Security</th>
                <th>Quality</th>
                <th>Runtime</th>
                <th>Overrides</th>
              </tr>
            </thead>
            <tbody>
              <tr><td colspan="9" style="text-align:center; color:var(--muted)">Run scan to view compliance matrix</td></tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </section>

</div>
<script src="/static/pilot_ui.js"></script>
</body>
</html>"#;

// ============================================================================
// GOVERNANCE ENDPOINTS
// ============================================================================

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// Helper for generating deterministic policy hashes
fn compute_policy_hash(val: &Value) -> String {
    let mut hasher = DefaultHasher::new();
    let s = serde_json::to_string(val).unwrap_or_default();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Debug, Deserialize)]
struct PolicyPathParams {
    kind: String,
}

#[derive(Debug, Deserialize)]
struct DraftPolicyRequest {
    ago_path: Option<String>,
    policy_json: Value,
}

#[derive(Debug, Serialize)]
struct PolicyResponse {
    id: String,
    agorg_id: String,
    ago_path: Option<String>,
    version: i32,
    status: String,
    policy_json: Value,
    hash: String,
}

#[derive(Debug, Deserialize)]
struct SimulatePolicyRequest {
    ago_path: Option<String>,
    policy_json: Value,
}

#[derive(Debug, Deserialize)]
struct ActivatePolicyRequest {
    ago_path: Option<String>,
    simulation_evidence_id: String,
}

#[derive(Debug, Deserialize)]
struct PolicyVersionsQuery {
    ago_path: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct LoadPolicyVersionRequest {
    ago_path: Option<String>,
    version: i32,
}

#[derive(Debug, Deserialize)]
struct DeletePolicyVersionRequest {
    ago_path: Option<String>,
    version: i32,
    confirm: String,
}

fn normalize_scope_path(opt: Option<&str>) -> String {
    opt.unwrap_or("__agorg__").trim().to_string()
}

fn default_policy_json_for_kind(kind: &str) -> std::result::Result<Value, String> {
    let value = match kind {
        "branch" => serde_json::to_value(BranchPolicy::default()),
        "dependency" => serde_json::to_value(DependencyPolicy::default()),
        "release" => serde_json::to_value(ReleasePolicy::default()),
        "security" => serde_json::to_value(SecurityPolicy::default()),
        "quality" => serde_json::to_value(QualityPolicy::default()),
        "runtime" => serde_json::to_value(RuntimePolicy::default()),
        "operator_routine" => serde_json::to_value(OperatorRoutinePolicy::default()),
        _ => return Err(format!("Unsupported policy kind '{}'", kind)),
    }
    .map_err(|e| e.to_string())?;
    Ok(value)
}

async fn api_settings_get_policy(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(params): axum::extract::Path<PolicyPathParams>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let ago_path = query
        .get("ago_path")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    if let Some(path) = ago_path {
        match gov_store
            .get_effective_policy_record(active_scope.id, path, &params.kind)
            .await
        {
            Ok(Some((record, source_name))) => {
                let hash = compute_policy_hash(&record.policy_json);
                return Json(json!({
                    "id": record.id.to_string(),
                    "agorg_id": record.agorg_id.to_string(),
                    "ago_path": record.ago_path,
                    "version": record.version,
                    "status": record.status,
                    "policy_json": record.policy_json,
                    "hash": hash,
                    "source": source_name,
                    "is_override": record.ago_path.is_some()
                }))
                .into_response();
            }
            Ok(None) => match default_policy_json_for_kind(&params.kind) {
                Ok(policy_json) => {
                    let hash = compute_policy_hash(&policy_json);
                    return Json(json!({
                        "id": format!("default:{}", params.kind),
                        "agorg_id": active_scope.id.to_string(),
                        "ago_path": path,
                        "version": 0,
                        "status": "default",
                        "policy_json": policy_json,
                        "hash": hash,
                        "source": "Default",
                        "is_override": false
                    }))
                    .into_response();
                }
                Err(err) => return error_response(StatusCode::BAD_REQUEST, &err),
            },
            Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        }
    }

    match gov_store.get_policy(active_scope.id, &params.kind).await {
        Ok(Some(record)) => {
            let hash = compute_policy_hash(&record.policy_json);
            Json(json!({
                "id": record.id.to_string(),
                "agorg_id": record.agorg_id.to_string(),
                "ago_path": record.ago_path,
                "version": record.version,
                "status": record.status,
                "policy_json": record.policy_json,
                "hash": hash,
                "source": "AGOrg",
                "is_override": false
            }))
            .into_response()
        }
        Ok(None) => match default_policy_json_for_kind(&params.kind) {
            Ok(policy_json) => {
                let hash = compute_policy_hash(&policy_json);
                Json(json!({
                    "id": format!("default:{}", params.kind),
                    "agorg_id": active_scope.id.to_string(),
                    "ago_path": Value::Null,
                    "version": 0,
                    "status": "default",
                    "policy_json": policy_json,
                    "hash": hash,
                    "source": "Default",
                    "is_override": false
                }))
                .into_response()
            }
            Err(err) => error_response(StatusCode::BAD_REQUEST, &err),
        },
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn api_settings_list_policy_versions(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(params): axum::extract::Path<PolicyPathParams>,
    Query(query): Query<PolicyVersionsQuery>,
) -> Response {
    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    let limit = query.limit.unwrap_or(25).clamp(1, 100);
    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    match gov_store
        .list_policy_versions(
            active_scope.id,
            query.ago_path.as_deref(),
            &params.kind,
            limit,
        )
        .await
    {
        Ok(rows) => Json(json!({
            "ok": true,
            "kind": params.kind,
            "ago_path": query.ago_path,
            "count": rows.len(),
            "items": rows.iter().map(|r| json!({
                "id": r.id.to_string(),
                "version": r.version,
                "status": r.status,
                "updated_at": r.updated_at,
                "updated_by": r.updated_by
            })).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn api_settings_load_policy_version(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(params): axum::extract::Path<PolicyPathParams>,
    Json(req): Json<LoadPolicyVersionRequest>,
) -> Response {
    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    match gov_store
        .get_policy_by_version(
            active_scope.id,
            req.ago_path.as_deref(),
            &params.kind,
            req.version,
        )
        .await
    {
        Ok(Some(record)) => {
            let hash = compute_policy_hash(&record.policy_json);
            Json(json!({
                "id": record.id.to_string(),
                "agorg_id": record.agorg_id.to_string(),
                "ago_path": record.ago_path,
                "version": record.version,
                "status": record.status,
                "policy_json": record.policy_json,
                "hash": hash,
                "source": if req.ago_path.is_some() { "AGO Override" } else { "AGOrg" },
                "is_override": req.ago_path.is_some()
            }))
            .into_response()
        }
        Ok(None) => error_response(StatusCode::NOT_FOUND, "Policy version not found"),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn api_settings_delete_policy_version(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(params): axum::extract::Path<PolicyPathParams>,
    Json(req): Json<DeletePolicyVersionRequest>,
) -> Response {
    if !state.allow_mutations {
        return error_response(StatusCode::FORBIDDEN, "Mutations are disabled");
    }
    if req.confirm.trim() != "DELETE" {
        return error_response(
            StatusCode::PRECONDITION_FAILED,
            "Deletion blocked: type DELETE to confirm",
        );
    }

    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    let deleted = match gov_store
        .delete_policy_version(
            active_scope.id,
            req.ago_path.as_deref(),
            &params.kind,
            req.version,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    if deleted == 0 {
        return error_response(StatusCode::NOT_FOUND, "Policy version not found");
    }
    let _ = state.events.send(json!({
        "source": "governance",
        "action": "delete_policy_version",
        "policy_kind": params.kind,
        "ago_path": req.ago_path,
        "version": req.version
    }));
    Json(json!({
        "ok": true,
        "kind": params.kind,
        "ago_path": req.ago_path,
        "version": req.version,
        "deleted": deleted
    }))
    .into_response()
}

async fn api_settings_draft_policy(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(params): axum::extract::Path<PolicyPathParams>,
    Json(req): Json<DraftPolicyRequest>,
) -> Response {
    if !state.allow_mutations {
        return error_response(StatusCode::FORBIDDEN, "Mutations are disabled");
    }

    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());

    let saved = match gov_store
        .save_policy(
            active_scope.id,
            req.ago_path.as_deref(),
            &params.kind,
            &req.policy_json,
            "draft",
            "pilot_ui",
        )
        .await
    {
        Ok(s) => s,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let hash = compute_policy_hash(&saved.policy_json);
    let resp = PolicyResponse {
        id: saved.id.to_string(),
        agorg_id: saved.agorg_id.to_string(),
        ago_path: saved.ago_path,
        version: saved.version,
        status: saved.status,
        policy_json: saved.policy_json,
        hash,
    };

    let _ = state.events.send(json!({
        "source": "governance",
        "action": "draft_policy",
        "policy_kind": params.kind,
        "ago_path": req.ago_path,
        "version": resp.version
    }));

    Json(resp).into_response()
}

async fn api_settings_simulate_policy(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(params): axum::extract::Path<PolicyPathParams>,
    Json(req): Json<SimulatePolicyRequest>,
) -> Response {
    if !matches!(
        params.kind.as_str(),
        "branch"
            | "dependency"
            | "release"
            | "security"
            | "quality"
            | "runtime"
            | "operator_routine"
    ) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Simulation supports branch, dependency, release, security, quality, runtime, operator_routine policies",
        );
    }
    let policy_parse_ok = match params.kind.as_str() {
        "branch" => serde_json::from_value::<BranchPolicy>(req.policy_json.clone()).is_ok(),
        "dependency" => serde_json::from_value::<DependencyPolicy>(req.policy_json.clone()).is_ok(),
        "release" => serde_json::from_value::<ReleasePolicy>(req.policy_json.clone()).is_ok(),
        "security" => serde_json::from_value::<SecurityPolicy>(req.policy_json.clone()).is_ok(),
        "quality" => serde_json::from_value::<QualityPolicy>(req.policy_json.clone()).is_ok(),
        "runtime" => serde_json::from_value::<RuntimePolicy>(req.policy_json.clone()).is_ok(),
        "operator_routine" => {
            serde_json::from_value::<OperatorRoutinePolicy>(req.policy_json.clone()).is_ok()
        }
        _ => false,
    };
    if !policy_parse_ok {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Invalid policy JSON for selected policy kind",
        );
    }

    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    let roots = scope_roots(&active_scope);
    let registry = match branch_registry() {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let mut repos = match registry.list_repos(&multi::RepoFilter::default()) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    repos.retain(|repo| {
        let path = canonicalize_path_lossy(&repo.path);
        path_in_any_root(&path, &roots)
    });
    if let Some(target) = req
        .ago_path
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        repos.retain(|repo| {
            repo.name.eq_ignore_ascii_case(target)
                || repo.path.display().to_string().contains(target)
        });
    }

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    let exceptions = match gov_store
        .get_exceptions(active_scope.id, &params.kind)
        .await
    {
        Ok(v) => v,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let statuses = branch::branch_status(&repos);
    let mut violations = 0usize;
    let mut warnings = 0usize;
    let mut blocked = 0usize;
    let evaluations: Vec<Value> = statuses
        .iter()
        .map(|st| {
            let report = match params.kind.as_str() {
                "branch" => match serde_json::from_value::<BranchPolicy>(req.policy_json.clone()) {
                    Ok(policy) => evaluate_branch_policy(
                        &policy,
                        "create",
                        &st.current_branch,
                        &exceptions,
                        &st.path,
                        "AGOrg",
                        Some(active_scope.id),
                    ),
                    Err(_) => PolicyEvalReport::default(),
                },
                "dependency" => {
                    match serde_json::from_value::<DependencyPolicy>(req.policy_json.clone()) {
                        Ok(policy) => evaluate_dependency_policy(
                            &policy,
                            Path::new(&st.path),
                            &exceptions,
                            &st.path,
                            "AGOrg",
                            Some(active_scope.id),
                        ),
                        Err(_) => PolicyEvalReport::default(),
                    }
                }
                "release" => match serde_json::from_value::<ReleasePolicy>(req.policy_json.clone())
                {
                    Ok(policy) => evaluate_release_policy(
                        &policy,
                        Path::new(&st.path),
                        &exceptions,
                        &st.path,
                        "AGOrg",
                        Some(active_scope.id),
                    ),
                    Err(_) => PolicyEvalReport::default(),
                },
                "security" => {
                    match serde_json::from_value::<SecurityPolicy>(req.policy_json.clone()) {
                        Ok(policy) => evaluate_security_policy(
                            &policy,
                            Path::new(&st.path),
                            &exceptions,
                            &st.path,
                            "AGOrg",
                            Some(active_scope.id),
                        ),
                        Err(_) => PolicyEvalReport::default(),
                    }
                }
                "quality" => match serde_json::from_value::<QualityPolicy>(req.policy_json.clone())
                {
                    Ok(policy) => evaluate_quality_policy(
                        &policy,
                        Path::new(&st.path),
                        &exceptions,
                        &st.path,
                        "AGOrg",
                        Some(active_scope.id),
                    ),
                    Err(_) => PolicyEvalReport::default(),
                },
                "runtime" => match serde_json::from_value::<RuntimePolicy>(req.policy_json.clone())
                {
                    Ok(policy) => evaluate_runtime_policy(
                        &policy,
                        Path::new(&st.path),
                        &exceptions,
                        &st.path,
                        "AGOrg",
                        Some(active_scope.id),
                    ),
                    Err(_) => PolicyEvalReport::default(),
                },
                "operator_routine" => {
                    match serde_json::from_value::<OperatorRoutinePolicy>(req.policy_json.clone()) {
                        Ok(policy) => {
                            let context = OperatorRoutineContext {
                                action: "push".to_string(),
                                has_active_scope: true,
                                repo_registered: true,
                                current_branch: Some(st.current_branch.clone()),
                                repo_clean: git_repo_clean(Path::new(&st.path)),
                                completed_steps: vec![],
                            };
                            evaluate_operator_routine_policy(
                                &policy,
                                &context,
                                &exceptions,
                                &st.path,
                                "AGOrg",
                                Some(active_scope.id),
                            )
                        }
                        Err(_) => PolicyEvalReport::default(),
                    }
                }
                _ => PolicyEvalReport::default(),
            };
            violations += report.violations.len();
            warnings += report.warnings.len();
            if report.blocked {
                blocked += 1;
            }
            json!({
                "repo": st.repo,
                "path": st.path,
                "branch": st.current_branch,
                "blocked": report.blocked,
                "violations": report.violations.len(),
                "warnings": report.warnings.len()
            })
        })
        .collect();

    let simulation_evidence_id = Uuid::new_v4().to_string();
    let summary = json!({
        "ok": true,
        "status": if blocked > 0 { "blocked" } else { "pass" },
        "evidence_id": simulation_evidence_id,
        "policy_kind": params.kind,
        "scope_target": req.ago_path,
        "evaluated_branches": evaluations.len(),
        "violations": violations,
        "warnings": warnings,
        "blocked": blocked,
        "evaluations": evaluations
    });

    let _ = state.events.send(json!({
        "source": "governance",
        "action": "simulate_policy",
        "policy_kind": params.kind,
        "evidence_id": simulation_evidence_id,
        "evaluated_branches": summary.get("evaluated_branches").and_then(Value::as_u64).unwrap_or(0),
        "violations": violations,
        "warnings": warnings,
        "blocked": blocked
    }));

    Json(summary).into_response()
}

async fn api_settings_activate_policy(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(params): axum::extract::Path<PolicyPathParams>,
    Json(req): Json<ActivatePolicyRequest>,
) -> Response {
    if !state.allow_mutations {
        return error_response(StatusCode::FORBIDDEN, "Mutations are disabled");
    }

    if req.simulation_evidence_id.is_empty() {
        return error_response(
            StatusCode::PRECONDITION_FAILED,
            "Activation blocked: valid simulation evidence required",
        );
    }

    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    let scope_key = normalize_scope_path(req.ago_path.as_deref());
    let idem_key = format!(
        "settings.activate:{}:{}:{}:{}",
        active_scope.id, params.kind, scope_key, req.simulation_evidence_id
    );
    if let Ok(Some(existing)) = gov_store.get_idempotency_response(&idem_key).await {
        return Json(existing).into_response();
    }

    let current = if let Some(ref path) = req.ago_path {
        gov_store
            .get_ago_policy_override(active_scope.id, path, &params.kind)
            .await
    } else {
        gov_store.get_policy(active_scope.id, &params.kind).await
    };

    let current = match current {
        Ok(Some(c)) => c,
        Ok(None) => {
            return error_response(StatusCode::NOT_FOUND, "No draft policy found to activate")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let saved = match gov_store
        .save_policy(
            active_scope.id,
            req.ago_path.as_deref(),
            &params.kind,
            &current.policy_json,
            "active",
            "pilot_ui",
        )
        .await
    {
        Ok(s) => s,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let hash = compute_policy_hash(&saved.policy_json);
    let resp = PolicyResponse {
        id: saved.id.to_string(),
        agorg_id: saved.agorg_id.to_string(),
        ago_path: saved.ago_path,
        version: saved.version,
        status: saved.status,
        policy_json: saved.policy_json,
        hash,
    };

    let _ = state.events.send(json!({
        "source": "governance",
        "action": "activate_policy",
        "policy_kind": params.kind,
        "version": resp.version,
        "evidence_id": req.simulation_evidence_id
    }));

    let response_json = serde_json::to_value(&resp)
        .unwrap_or_else(|_| json!({"ok": false, "error": "serialize_failed"}));
    let _ = gov_store
        .save_idempotency_response(&idem_key, &response_json)
        .await;
    Json(response_json).into_response()
}

#[derive(Debug, Deserialize)]
struct GetExceptionsQuery {
    ago_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddExceptionRequest {
    ago_path: Option<String>,
    rule_path: String,
    reason: String,
    ticket_ref: Option<String>,
    owner: String,
    expires_at_unix: i64,
}

#[derive(Debug, Deserialize)]
struct ExceptionIdParams {
    id: String,
}

async fn api_settings_get_exceptions(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(params): axum::extract::Path<PolicyPathParams>,
    Query(query): Query<GetExceptionsQuery>,
) -> Response {
    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    match gov_store
        .get_exceptions(active_scope.id, &params.kind)
        .await
    {
        Ok(all_exceptions) => {
            let filtered: Vec<_> = if let Some(path) = query.ago_path {
                all_exceptions
                    .into_iter()
                    .filter(|e| e.ago_path.as_deref() == Some(path.as_str()))
                    .collect()
            } else {
                all_exceptions
            };
            Json(filtered).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn api_settings_add_exception(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(params): axum::extract::Path<PolicyPathParams>,
    Json(req): Json<AddExceptionRequest>,
) -> Response {
    if !state.allow_mutations {
        return error_response(StatusCode::FORBIDDEN, "Mutations are disabled");
    }

    if req.owner.trim().is_empty() || req.reason.trim().is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "owner and reason are required");
    }

    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let expires_at =
        chrono::DateTime::from_timestamp(req.expires_at_unix, 0).unwrap_or_else(chrono::Utc::now);

    let exception = PolicyException {
        id: Uuid::new_v4(),
        agorg_id: active_scope.id,
        ago_path: req.ago_path,
        policy_kind: params.kind.clone(),
        rule_path: req.rule_path,
        reason: req.reason,
        ticket_ref: req.ticket_ref,
        owner: req.owner,
        expires_at,
        created_at: chrono::Utc::now(),
    };

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    match gov_store.add_exception(exception.clone()).await {
        Ok(_) => {
            let _ = state.events.send(json!({
                "source": "governance",
                "action": "add_exception",
                "policy_kind": params.kind,
                "exception_id": exception.id.to_string()
            }));
            Json(exception).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn api_settings_delete_exception(
    State(state): State<Arc<UiState>>,
    axum::extract::Path(params): axum::extract::Path<ExceptionIdParams>,
) -> Response {
    if !state.allow_mutations {
        return error_response(StatusCode::FORBIDDEN, "Mutations are disabled");
    }

    let exception_id = match Uuid::parse_str(&params.id) {
        Ok(id) => id,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid exception ID format"),
    };

    let _active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    match gov_store.delete_exception(exception_id).await {
        Ok(_) => {
            let _ = state.events.send(json!({
                "source": "governance",
                "action": "delete_exception",
                "exception_id": exception_id.to_string()
            }));
            Json(json!({"ok": true})).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

#[derive(Debug, Deserialize)]
struct ComplianceScanRequest {
    ago_path: Option<String>,
    kind: String,
}

async fn api_settings_compliance_scan(
    State(state): State<Arc<UiState>>,
    Json(req): Json<ComplianceScanRequest>,
) -> Response {
    if req.kind != "branch"
        && req.kind != "dependency"
        && req.kind != "release"
        && req.kind != "security"
        && req.kind != "quality"
        && req.kind != "runtime"
        && req.kind != "operator_routine"
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Compliance scan supports branch, dependency, release, security, quality, runtime, operator_routine policies",
        );
    }

    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    let roots = scope_roots(&active_scope);
    let registry = match branch_registry() {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let mut repos = match registry.list_repos(&multi::RepoFilter::default()) {
        Ok(v) => v,
        Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    repos.retain(|repo| {
        let path = canonicalize_path_lossy(&repo.path);
        path_in_any_root(&path, &roots)
    });
    if let Some(target) = req
        .ago_path
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        repos.retain(|repo| {
            repo.name.eq_ignore_ascii_case(target)
                || repo.path.display().to_string().contains(target)
        });
    }
    let statuses = branch::branch_status(&repos);
    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    let exceptions = match gov_store
        .get_effective_exceptions(active_scope.id, &req.kind)
        .await
    {
        Ok(v) => v,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    let default_policy_json = match req.kind.as_str() {
        "branch" => serde_json::to_value(BranchPolicy::default()),
        "dependency" => serde_json::to_value(DependencyPolicy::default()),
        "release" => serde_json::to_value(ReleasePolicy::default()),
        "security" => serde_json::to_value(SecurityPolicy::default()),
        "quality" => serde_json::to_value(QualityPolicy::default()),
        "runtime" => serde_json::to_value(RuntimePolicy::default()),
        "operator_routine" => serde_json::to_value(OperatorRoutinePolicy::default()),
        _ => serde_json::to_value(BranchPolicy::default()),
    }
    .unwrap_or(json!({}));

    let mut issues = 0usize;
    let mut off_policy = 0usize;
    let mut details: Vec<Value> = Vec::with_capacity(statuses.len());
    for st in &statuses {
        let repo_path_str = canonicalize_path_lossy(Path::new(&st.path))
            .display()
            .to_string();

        let (policy_record, source_name) = match gov_store
            .get_effective_policy_record(active_scope.id, repo_path_str.as_str(), &req.kind)
            .await
        {
            Ok(Some((p, name))) => (p, name),
            _ => (
                AgorgPolicyRecord {
                    id: Uuid::nil(),
                    agorg_id: Uuid::nil(),
                    ago_path: None,
                    policy_kind: req.kind.clone(),
                    version: 0,
                    policy_json: default_policy_json.clone(),
                    status: "default".to_string(),
                    updated_at: Utc::now(),
                    updated_by: "system".to_string(),
                },
                "Default".to_string(),
            ),
        };

        let policy_json = policy_record.policy_json.clone();
        let source_id = if policy_record.version > 0 {
            Some(policy_record.agorg_id)
        } else {
            None
        };

        let path = Path::new(&st.path);
        let eval = match req.kind.as_str() {
            "branch" => {
                let policy: BranchPolicy = serde_json::from_value(policy_json).unwrap_or_default();
                evaluate_branch_policy(
                    &policy,
                    "create",
                    &st.current_branch,
                    &exceptions,
                    &repo_path_str,
                    &source_name,
                    source_id,
                )
            }
            "dependency" => {
                let policy: DependencyPolicy =
                    serde_json::from_value(policy_json).unwrap_or_default();
                evaluate_dependency_policy(
                    &policy,
                    path,
                    &exceptions,
                    &repo_path_str,
                    &source_name,
                    source_id,
                )
            }
            "release" => {
                let policy: ReleasePolicy = serde_json::from_value(policy_json).unwrap_or_default();
                evaluate_release_policy(
                    &policy,
                    path,
                    &exceptions,
                    &repo_path_str,
                    &source_name,
                    source_id,
                )
            }
            "security" => {
                let policy: SecurityPolicy =
                    serde_json::from_value(policy_json).unwrap_or_default();
                evaluate_security_policy(
                    &policy,
                    path,
                    &exceptions,
                    &repo_path_str,
                    &source_name,
                    source_id,
                )
            }
            "quality" => {
                let policy: QualityPolicy = serde_json::from_value(policy_json).unwrap_or_default();
                evaluate_quality_policy(
                    &policy,
                    path,
                    &exceptions,
                    &repo_path_str,
                    &source_name,
                    source_id,
                )
            }
            "runtime" => {
                let policy: RuntimePolicy = serde_json::from_value(policy_json).unwrap_or_default();
                evaluate_runtime_policy(
                    &policy,
                    path,
                    &exceptions,
                    &repo_path_str,
                    &source_name,
                    source_id,
                )
            }
            "operator_routine" => {
                let policy: OperatorRoutinePolicy =
                    serde_json::from_value(policy_json).unwrap_or_default();
                let context = OperatorRoutineContext {
                    action: "push".to_string(),
                    has_active_scope: true,
                    repo_registered: true,
                    current_branch: Some(st.current_branch.clone()),
                    repo_clean: git_repo_clean(path),
                    completed_steps: vec![],
                };
                evaluate_operator_routine_policy(
                    &policy,
                    &context,
                    &exceptions,
                    &repo_path_str,
                    &source_name,
                    source_id,
                )
            }
            _ => PolicyEvalReport::default(),
        };

        let issue_count = eval.violations.len() + eval.warnings.len();
        issues += issue_count;
        if issue_count > 0 {
            off_policy += 1;
        }
        details.push(json!({
            "repo": st.repo,
            "path": st.path,
            "branch": st.current_branch,
            "policy_source": source_name,
            "policy_source_id": source_id,
            "is_override": policy_record.ago_path.is_some(),
            "blocked": eval.blocked,
            "violations": eval.violations.len(),
            "warnings": eval.warnings.len()
        }));
    }
    let payload = json!({
        "ok": true,
        "status": if off_policy > 0 { "issues_found" } else { "pass" },
        "kind": req.kind,
        "scope_target": req.ago_path,
        "scanned": details.len(),
        "issues": issues,
        "off_policy": off_policy,
        "details": details
    });
    Json(payload).into_response()
}

#[derive(Debug, Deserialize)]
struct DecisionsQuery {
    limit: Option<usize>,
    kind: Option<String>,
}

async fn api_settings_decisions(
    State(state): State<Arc<UiState>>,
    Query(q): Query<DecisionsQuery>,
) -> Response {
    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let gov_store = GovernanceStore::new(state.agorg_store.dsn());
    let kind = q.kind.unwrap_or_else(|| "branch".to_string());
    match gov_store
        .get_decisions(active_scope.id, &kind, q.limit.unwrap_or(100))
        .await
    {
        Ok(decisions) => Json(json!({"decisions": decisions, "kind": kind})).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

#[derive(Debug, Deserialize)]
struct PolicyResolveRequest {
    repo_path: String,
    kind: String,
}

async fn api_settings_policy_resolve(
    State(state): State<Arc<UiState>>,
    Json(req): Json<PolicyResolveRequest>,
) -> Response {
    let active_scope = match state.agorg_store.get_active_agorg().await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return error_response(StatusCode::PRECONDITION_FAILED, "No active AGOrg scope")
        }
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let canonical_repo = canonicalize_path_lossy(Path::new(&req.repo_path))
        .display()
        .to_string();
    let gov_store = GovernanceStore::new(state.agorg_store.dsn());

    let (policy_record, source_name) = match gov_store
        .get_effective_policy_record(active_scope.id, canonical_repo.as_str(), &req.kind)
        .await
    {
        Ok(Some((p, name))) => (p, name),
        _ => (
            AgorgPolicyRecord {
                id: Uuid::nil(),
                agorg_id: Uuid::nil(),
                ago_path: None,
                policy_kind: req.kind.clone(),
                version: 0,
                policy_json: match req.kind.as_str() {
                    "branch" => serde_json::to_value(BranchPolicy::default()),
                    "dependency" => serde_json::to_value(DependencyPolicy::default()),
                    "release" => serde_json::to_value(ReleasePolicy::default()),
                    "security" => serde_json::to_value(SecurityPolicy::default()),
                    "quality" => serde_json::to_value(QualityPolicy::default()),
                    "runtime" => serde_json::to_value(RuntimePolicy::default()),
                    "operator_routine" => serde_json::to_value(OperatorRoutinePolicy::default()),
                    _ => serde_json::to_value(BranchPolicy::default()),
                }
                .unwrap_or(json!({})),
                status: "default".to_string(),
                updated_at: Utc::now(),
                updated_by: "system".to_string(),
            },
            "Default".to_string(),
        ),
    };

    Json(json!({
        "ok": true,
        "repo_path": canonical_repo,
        "kind": req.kind,
        "source": source_name,
        "source_id": if policy_record.version > 0 { Some(policy_record.agorg_id) } else { None },
        "is_override": policy_record.ago_path.is_some(),
        "version": policy_record.version,
        "status": policy_record.status,
        "resolved_policy": policy_record.policy_json
    }))
    .into_response()
}

async fn api_orchestrate_run(
    State(state): State<Arc<UiState>>,
    Json(req): Json<OrchestratorRequest>,
) -> Response {
    // Determine stage from payload: dry_run=true or action=preview-class → "preview"
    let is_preview = orchestrate_is_preview(&req.payload);
    let stage = if is_preview { "preview" } else { "execute" };
    let mut payload = req.payload.clone();
    normalize_orchestrate_payload(&mut payload, stage);

    let inner_response: Value = match req.domain.as_str() {
        "branch" => {
            if let Ok(branch_req) = serde_json::from_value::<BranchRunRequest>(payload) {
                let resp = api_branch_run(State(state), Json(branch_req)).await;
                extract_json_body(resp).await
            } else {
                json!({"ok": false, "error": "Invalid branch payload format"})
            }
        }
        "dependency" => {
            if let Ok(dep_req) = serde_json::from_value::<DependencyActionRequest>(payload) {
                let resp = run_dependency_action(State(state), Json(dep_req)).await;
                extract_json_body(resp).await
            } else {
                json!({"ok": false, "error": "Invalid dependency payload format"})
            }
        }
        "command" => match command_request_from_orchestrate_payload(payload) {
            Ok(cmd_req) => {
                let resp = run_command(State(state), Json(cmd_req)).await;
                extract_json_body(resp).await
            }
            Err(err) => {
                json!({"ok": false, "error": err})
            }
        },
        other => json!({
            "ok": false,
            "error": format!("Unknown orchestrator domain: {other}")
        }),
    };

    let envelope = wrap_as_envelope(&req.domain, stage, inner_response);
    (
        StatusCode::OK,
        Json(serde_json::to_value(&envelope).unwrap_or_default()),
    )
        .into_response()
}

/// P5: Return current orchestration graph step-status (stateless — always returns schema info).
async fn api_orchestrate_graph_status() -> Response {
    Json(json!({
        "ok": true,
        "schema_version": "p5.1",
        "envelope_fields": ["ok", "operation_id", "domain", "stage", "status", "summary", "artifact_path", "error", "inner"],
        "domains": ["branch", "dependency", "command"],
        "stages": ["preview", "execute"],
        "status_values": ["preview", "ok", "error"]
    }))
    .into_response()
}

/// Extract the JSON body from an axum Response for envelope wrapping.
async fn extract_json_body(resp: Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 10 * 1024 * 1024)
        .await
        .unwrap_or_default();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"ok": false, "error": "non-JSON body"}))
}
