use crate::agorg::{self, AgorgStore};
use crate::bus::{send_command_once, BusBridgeConfig};
use axum::extract::{Query, State};
use axum::http::{HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::{SinkExt, StreamExt};
use miette::{IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command as TokioCommand;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

const FAVICON_ICO: &[u8] = include_bytes!("../assets/favicon.ico");
const PILOT_UI_JS: &str = include_str!("pilot_ui.js");

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
    agorg_reviews: Arc<Mutex<HashMap<String, AgorgReviewRecord>>>,
    agorg_reviews_log: PathBuf,
    agorg_store: AgorgStore,
}

#[derive(Debug, Deserialize)]
struct UiCommandRequest {
    command: String,
    payload: Value,
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
    review_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgReconcileRequest {
    agorg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgReviewsQuery {
    limit: Option<usize>,
    agorg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgorgReviewQuery {
    review_id: String,
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
}

#[derive(Debug, Deserialize)]
struct AgorgDashboardOverviewRequest {
    agorg: Option<String>,
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
struct AgorgReviewRecord {
    review_id: String,
    status: String,
    agorg_id: Option<String>,
    root: String,
    depth: usize,
    prune_missing: bool,
    candidates: Vec<agorg::DiscoverCandidate>,
    approved_paths: Vec<String>,
    imported_summary: Option<agorg::ImportDiscoverySummary>,
    created_at_unix: u64,
    updated_at_unix: u64,
}

pub async fn run_ui_server(cfg: UiConfig) -> Result<()> {
    let (event_tx, _) = broadcast::channel(512);
    spawn_bus_telemetry_listener(cfg.bus.clone(), event_tx.clone());
    let codex_contracts_log = codex_contracts_log_path();
    let contract_seed = load_persisted_codex_contracts(&codex_contracts_log).unwrap_or_default();
    let agorg_reviews_log = agorg_reviews_log_path();
    let agorg_review_seed = load_persisted_agorg_reviews(&agorg_reviews_log).unwrap_or_default();
    let agorg_store = AgorgStore::from_instance(cfg.instance_id.clone());
    if let Err(e) = agorg_store.initialize().await {
        eprintln!(
            "Warning: AGOrg store initialization failed ({}). AGOrg API may be unavailable until fixed.",
            e
        );
    }
    let state = Arc::new(UiState {
        instance_id: cfg.instance_id.clone(),
        bus: cfg.bus,
        events: event_tx,
        allow_mutations: cfg.allow_mutations,
        allowed_commands: cfg.allowed_commands,
        codex_contracts: Arc::new(Mutex::new(contract_seed)),
        codex_contracts_log,
        agorg_reviews: Arc::new(Mutex::new(agorg_review_seed)),
        agorg_reviews_log,
        agorg_store,
    });

    if state.allow_mutations {
        let store = state.agorg_store.clone();
        tokio::spawn(async move {
            let _ = store.ensure_managed_db().await;
        });
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/static/pilot_ui.js", get(static_pilot_ui_js))
        .route("/api/command", post(run_command))
        .route("/api/history", get(get_history))
        .route("/api/reports", get(get_reports))
        .route("/api/report", get(get_report_content))
        .route("/api/codex/contracts", get(get_codex_contracts))
        .route("/api/codex/contract", get(get_codex_contract))
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
        .route("/api/agorg/use", post(api_agorg_use))
        .route("/api/agorg/discover", post(api_agorg_discover))
        .route(
            "/api/agorg/import_selected",
            post(api_agorg_import_selected),
        )
        .route("/api/agorg/reviews", get(get_agorg_reviews))
        .route("/api/agorg/review", get(get_agorg_review))
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
        .route("/api/agorg/link", post(api_agorg_link))
        .route("/api/agorg/scan_master", post(api_agorg_scan_master))
        .route("/api/agorg/upgrade_ago", post(api_agorg_upgrade_ago))
        .route(
            "/api/agorg/edit_relationship",
            post(api_agorg_edit_relationship),
        )
        .route("/api/fs/pick-directory", post(api_fs_pick_directory))
        .route("/api/dependencies/run", post(run_dependency_action))
        .route("/api/dependencies/logs", get(get_dependency_logs))
        .route("/api/evidence/export", post(export_evidence_bundle))
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
    Response::builder()
        .header("Content-Type", "image/x-icon")
        .body(axum::body::Body::from(FAVICON_ICO))
        .unwrap()
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

    match send_command_once_with_retry(&state.bus, &req.command, req.payload, 3).await {
        Ok(response) => {
            let _ = state.events.send(json!({
                "source": "ui_command",
                "command": req.command,
                "response": response,
            }));
            Json(UiCommandResponse { ok: true, response }).into_response()
        }
        Err(err) => {
            let _ = state.events.send(json!({
                "source": "ui_command",
                "command": req.command,
                "error": err.to_string(),
            }));
            error_response(StatusCode::BAD_GATEWAY, &err.to_string())
        }
    }
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

async fn get_agorg_reviews(
    State(state): State<Arc<UiState>>,
    Query(q): Query<AgorgReviewsQuery>,
) -> Response {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let agorg_filter = q.agorg.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let reviews = state.agorg_reviews.lock().await;
    let mut items: Vec<AgorgReviewRecord> = reviews
        .values()
        .filter(|r| {
            if let Some(target) = agorg_filter {
                r.agorg_id.as_deref() == Some(target)
            } else {
                true
            }
        })
        .cloned()
        .collect();
    items.sort_by(|a, b| b.updated_at_unix.cmp(&a.updated_at_unix));
    items.truncate(limit);
    Json(json!({"ok": true, "reviews": items})).into_response()
}

async fn get_agorg_review(
    State(state): State<Arc<UiState>>,
    Query(q): Query<AgorgReviewQuery>,
) -> Response {
    let id = q.review_id.trim();
    if id.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "review_id is required");
    }
    let reviews = state.agorg_reviews.lock().await;
    let Some(review) = reviews.get(id) else {
        return error_response(StatusCode::NOT_FOUND, "review_id not found");
    };
    Json(json!({"ok": true, "review": review})).into_response()
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
    let review_id = new_agorg_review_id();
    let mut review_record = AgorgReviewRecord {
        review_id: review_id.clone(),
        status: if req.import_to.is_some() {
            "imported".to_string()
        } else {
            "previewed".to_string()
        },
        agorg_id: None,
        root: scan.root.clone(),
        depth: scan.depth,
        prune_missing: req.prune_missing.unwrap_or(false),
        candidates: scan.candidates.clone(),
        approved_paths: scan
            .candidates
            .iter()
            .filter(|c| c.kind == "ago")
            .map(|c| c.path.clone())
            .collect(),
        imported_summary: None,
        created_at_unix: now_unix(),
        updated_at_unix: now_unix(),
    };

    if let Some(target) = req.import_to.as_deref() {
        let id = match resolve_agorg_ref(&state.agorg_store, target.trim()).await {
            Ok(v) => v,
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        };
        review_record.agorg_id = Some(id.to_string());
        let prune_missing = req.prune_missing.unwrap_or(false);
        match state
            .agorg_store
            .import_discovery_with_options(id, &scan, prune_missing)
            .await
        {
            Ok(summary) => {
                review_record.imported_summary = Some(summary.clone());
                if let Err(err) =
                    upsert_agorg_review(&state, review_record.clone(), Some("imported")).await
                {
                    let _ = state.events.send(json!({
                        "source": "agorg_review",
                        "phase": "persist_warning",
                        "review_id": review_record.review_id,
                        "error": err.to_string()
                    }));
                }
                return Json(json!({
                    "ok": true,
                    "discovery": scan,
                    "import_summary": summary,
                    "review": review_record
                }))
                .into_response();
            }
            Err(err) => return error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        };
    }
    if let Err(err) = upsert_agorg_review(&state, review_record.clone(), Some("previewed")).await {
        let _ = state.events.send(json!({
            "source": "agorg_review",
            "phase": "persist_warning",
            "review_id": review_record.review_id,
            "error": err.to_string()
        }));
    }
    Json(json!({"ok": true, "discovery": scan, "review": review_record})).into_response()
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
            let approved_paths: Vec<String> = discovery
                .candidates
                .iter()
                .filter(|c| c.kind == "ago")
                .map(|c| c.path.clone())
                .collect();
            let review_record = AgorgReviewRecord {
                review_id: req.review_id.clone().unwrap_or_else(new_agorg_review_id),
                status: "imported".to_string(),
                agorg_id: Some(id.to_string()),
                root: discovery.root.clone(),
                depth: discovery.depth,
                prune_missing,
                candidates: discovery.candidates.clone(),
                approved_paths,
                imported_summary: Some(summary.clone()),
                created_at_unix: now_unix(),
                updated_at_unix: now_unix(),
            };
            if let Err(err) = upsert_agorg_review(&state, review_record.clone(), None).await {
                let _ = state.events.send(json!({
                    "source": "agorg_review",
                    "phase": "persist_warning",
                    "review_id": review_record.review_id,
                    "error": err.to_string()
                }));
            }
            Json(json!({"ok": true, "agorg_id": id, "import_summary": summary, "review": review_record}))
                .into_response()
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
        let out = json!({
            "ok": true,
            "dry_run": true,
            "issue_class": issue_class,
            "planned_prune_count": planned_paths.len(),
            "planned_prune_paths": planned_paths,
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
        if class_name != "topology" {
            return Err(format!(
                "issue_class '{}' is not currently auto-fixable (supported: topology)",
                class_name
            ));
        }
    }

    let selected_paths = filter_prune_paths_by_class(&report, issue_class.as_deref());
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

fn bus_shim_running(stdout: &str, stderr: &str) -> bool {
    let combined = format!("{stdout}\n{stderr}");
    combined.contains("RUNNING")
}

async fn run_dependency_action(
    State(state): State<Arc<UiState>>,
    Json(req): Json<DependencyActionRequest>,
) -> Response {
    let action = req.action.trim();
    if action.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "action is required");
    }
    if matches!(
        action,
        "repair"
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
    if action == "db-restart" {
        return match state.agorg_store.stop_managed_db().await {
            Ok(_) => match state.agorg_store.ensure_managed_db().await {
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
            },
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
        let bus = run_local_script(
            "PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh status",
        )
        .await;
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
        let bus_cmd = match action {
            "services-start" => "PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh start && PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh status",
            "services-stop" => "PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh stop && PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh status",
            _ => "PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh stop || true; PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh start && PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh status",
        };
        let bus_result = run_local_script(bus_cmd).await;
        let db_result = match action {
            "services-start" => state.agorg_store.ensure_managed_db().await,
            "services-stop" => state.agorg_store.stop_managed_db().await,
            _ => {
                let _ = state.agorg_store.stop_managed_db().await;
                state.agorg_store.ensure_managed_db().await
            }
        };
        return match (bus_result, db_result) {
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
                Json(body).into_response()
            }
            (Err(err), _) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
            (_, Err(err)) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
        };
    }

    let result = match (action, req.json) {
        ("policy", true) => run_local_script("./scripts/verify_toolchain_policy.sh --json").await,
        ("policy", false) => run_local_script("./scripts/verify_toolchain_policy.sh").await,
        ("hook-policy", true) => {
            run_local_script("./scripts/verify_git_hook_policy.sh --json").await
        }
        ("hook-policy", false) => run_local_script("./scripts/verify_git_hook_policy.sh").await,
        ("drift", true) => run_local_script("./scripts/drift_report.sh --json").await,
        ("drift", false) => run_local_script("./scripts/drift_report.sh").await,
        ("gate", _) => run_local_script("./scripts/prepush_gate.sh").await,
        ("repair", _) => run_local_script("./scripts/repair_lock_182.sh --no-gate").await,
        ("bus-start", _) => {
            run_local_script("PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh start && PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh status").await
        }
        ("bus-stop", _) => {
            run_local_script("PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh stop && PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh status").await
        }
        ("bus-restart", _) => {
            run_local_script("PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh stop || true; PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh start && PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh status").await
        }
        ("bus-status", _) => {
            run_local_script("PILOT_REPORT_DIR=/tmp/pilot-reports ./scripts/arqonbus_shim.sh status").await
        }
        ("push", _) => {
            let branch = req.branch.as_deref().unwrap_or("main");
            let remote = req.remote.as_deref().unwrap_or("origin");
            if !is_safe_cli_token(branch) || !is_safe_cli_token(remote) {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "branch/remote contains unsupported characters",
                );
            }
            run_local_script(&format!("./scripts/push_main.sh {branch} {remote}")).await
        }
        _ => return error_response(StatusCode::BAD_REQUEST, "unsupported action"),
    };

    match result {
        Ok((status, out, err)) => {
            let ok = status == 0;
            let body = json!({
                "ok": ok,
                "action": action,
                "exit_code": status,
                "stdout": out,
                "stderr": err
            });
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

    let policy_json = run_local_script("./scripts/verify_toolchain_policy.sh --json")
        .await
        .ok()
        .map(|(code, out, err)| json!({"exit_code": code, "stdout": out, "stderr": err}));
    let hook_json = run_local_script("./scripts/verify_git_hook_policy.sh --json")
        .await
        .ok()
        .map(|(code, out, err)| json!({"exit_code": code, "stdout": out, "stderr": err}));
    let drift_json = run_local_script("./scripts/drift_report.sh --json")
        .await
        .ok()
        .map(|(code, out, err)| json!({"exit_code": code, "stdout": out, "stderr": err}));

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let stamp = format!("{}", now);
    let root = reports_root();
    if let Err(err) = fs::create_dir_all(&root) {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    let file_name = format!("evidence_bundle_{}.json", stamp);
    let file_path = root.join(&file_name);
    let bundle = json!({
        "exported_at_unix": now,
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
            "toolchain_policy": policy_json,
            "hook_policy": hook_json,
            "drift_report": drift_json
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
            || !(command.starts_with("pilot.") || command.starts_with("api.agorg."))
        {
            return error_response(
                StatusCode::BAD_REQUEST,
                "command must be namespaced as pilot.* or api.agorg.*",
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

        let exec_result = if execute_contract.command.starts_with("api.agorg.") {
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
            let verify_result = if verify_cmd.starts_with("api.agorg.") {
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

fn new_agorg_review_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("agorg-review-{}", nanos)
}

fn codex_contracts_log_path() -> PathBuf {
    reports_root().join("codex_contracts.jsonl")
}

fn agorg_reviews_log_path() -> PathBuf {
    reports_root().join("agorg_reviews.jsonl")
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

fn append_agorg_review_record(path: &PathBuf, record: &AgorgReviewRecord) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(record)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    writeln!(file, "{}", line)?;
    Ok(())
}

fn load_persisted_agorg_reviews(
    path: &PathBuf,
) -> std::io::Result<HashMap<String, AgorgReviewRecord>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let file = OpenOptions::new().read(true).open(path)?;
    let reader = BufReader::new(file);
    let mut reviews = HashMap::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed: serde_json::Result<AgorgReviewRecord> = serde_json::from_str(trimmed);
        if let Ok(record) = parsed {
            let replace = reviews
                .get(&record.review_id)
                .map(|current: &AgorgReviewRecord| {
                    current.updated_at_unix <= record.updated_at_unix
                })
                .unwrap_or(true);
            if replace {
                reviews.insert(record.review_id.clone(), record);
            }
        }
    }
    Ok(reviews)
}

async fn upsert_agorg_review(
    state: &Arc<UiState>,
    mut record: AgorgReviewRecord,
    set_status: Option<&str>,
) -> std::io::Result<()> {
    let mut reviews = state.agorg_reviews.lock().await;
    if let Some(existing) = reviews.get(&record.review_id).cloned() {
        record.created_at_unix = existing.created_at_unix;
    }
    if let Some(status) = set_status {
        record.status = status.to_string();
    }
    record.updated_at_unix = now_unix();
    reviews.insert(record.review_id.clone(), record.clone());
    append_agorg_review_record(&state.agorg_reviews_log, &record)
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
        append_agorg_review_record, append_codex_contract_record, command_requires_cwd_scope,
        command_requires_multi_selector, command_requires_mutation, command_scope_required,
        dependency_action_requires_cwd_scope, dependency_action_scope_required,
        filter_prune_paths_by_class, is_safe_cli_token, load_persisted_agorg_reviews,
        load_persisted_codex_contracts, payload_has_multi_selector, with_event_agorg_scope,
        AgorgReviewRecord, CodexContractRecord,
    };
    use crate::agorg::{AgorgReconcileIssue, AgorgReconcileReport};
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::fs;
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
    fn test_scope_dependency_action_classification() {
        assert!(dependency_action_scope_required("policy"));
        assert!(dependency_action_scope_required("gate"));
        assert!(dependency_action_scope_required("push"));
        assert!(!dependency_action_scope_required("db-status"));
        assert!(!dependency_action_scope_required("services-start"));

        assert!(dependency_action_requires_cwd_scope("repair"));
        assert!(!dependency_action_requires_cwd_scope("db-start"));
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

    #[test]
    fn test_agorg_review_persistence_roundtrip() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("pilot_agorg_review_test_{}", nanos));
        fs::create_dir_all(&base).unwrap();
        let path = base.join("agorg_reviews.jsonl");

        let r1 = AgorgReviewRecord {
            review_id: "agorg-review-1".to_string(),
            status: "previewed".to_string(),
            agorg_id: Some("ag-1".to_string()),
            root: "/tmp/root".to_string(),
            depth: 4,
            prune_missing: true,
            candidates: vec![],
            approved_paths: vec!["/tmp/root/repo-a".to_string()],
            imported_summary: None,
            created_at_unix: 1,
            updated_at_unix: 1,
        };
        let mut r2 = r1.clone();
        r2.status = "imported".to_string();
        r2.updated_at_unix = 3;

        append_agorg_review_record(&path, &r1).unwrap();
        append_agorg_review_record(&path, &r2).unwrap();

        let loaded = load_persisted_agorg_reviews(&path).unwrap();
        let got = loaded.get("agorg-review-1").unwrap();
        assert_eq!(got.status, "imported");
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
                repo_name: "old".to_string(),
                repo_path: "/tmp/arqon/archive/old".to_string(),
                severity: "warn".to_string(),
                issue_class: "topology".to_string(),
                code: "archive_path".to_string(),
                message: "off-policy".to_string(),
            }],
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
}

async fn get_dependency_logs() -> Response {
    match read_recent_gate_logs(4, 20_000) {
        Ok(logs) => Json(json!({"ok": true, "logs": logs})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
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
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".pilot").join("audit.jsonl")
}

fn reports_root() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".pilot").join("reports")
}

fn agorg_policy_report_path(ts: &str) -> PathBuf {
    reports_root().join(format!("agorg_policy_report_{}.json", ts))
}

fn agorg_reconcile_action_report_path(ts: &str, mode: &str) -> PathBuf {
    reports_root().join(format!("agorg_reconcile_{}_{}.json", mode, ts))
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
    let path = agorg_reconcile_action_report_path(&now_stamp(), mode);
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

async fn run_local_script(cmd: &str) -> std::io::Result<(i32, String, String)> {
    let child = TokioCommand::new("bash")
        .arg("-lc")
        .arg(cmd)
        .output()
        .await?;
    let code = child.status.code().unwrap_or(-1);
    let out = String::from_utf8_lossy(&child.stdout).to_string();
    let err = String::from_utf8_lossy(&child.stderr).to_string();
    Ok((code, out, err))
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
            "data": {"token": token},
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
        "data": {"channel_id": bus.telemetry_channel},
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
        "policy" | "hook-policy" | "drift" | "gate" | "repair" | "push"
    )
}

fn dependency_action_requires_cwd_scope(action: &str) -> bool {
    matches!(
        action,
        "policy" | "hook-policy" | "drift" | "gate" | "repair" | "push"
    )
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
  <title>Pilot Control Panel</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #090f1c;
      --panel: #111a2d;
      --panel-2: #14213a;
      --border: #2e3f64;
      --text: #e7ecff;
      --muted: #9db0df;
      --primary: #6d7dff;
      --primary-hover: #8090ff;
      --accent: #30c7f4;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      color: var(--text);
      font-family: "Segoe UI", "Inter", ui-sans-serif, system-ui, sans-serif;
      background:
        radial-gradient(circle at 50% 0%, rgba(115, 97, 255, 0.25), transparent 52%),
        radial-gradient(circle at 90% 100%, rgba(37, 209, 246, 0.16), transparent 38%),
        var(--bg);
      min-height: 100vh;
    }
    .wrap { max-width: 1200px; margin: 0 auto; padding: 28px 20px 48px; }
    .hero {
      border: 1px solid var(--border);
      background: linear-gradient(160deg, rgba(20, 33, 58, 0.9), rgba(13, 21, 36, 0.95));
      border-radius: 14px;
      padding: 22px;
      margin-bottom: 18px;
      box-shadow: 0 16px 42px rgba(0, 0, 0, 0.28);
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
      border: 1px solid #3a578a;
      background: #152845;
      color: #dbe7ff;
      border-radius: 999px;
      padding: 4px 10px;
      cursor: pointer;
      user-select: none;
      transition: all 0.15s ease;
    }
    .status-right:hover {
      border-color: #5b7cc0;
      box-shadow: 0 0 0 2px rgba(90, 124, 200, 0.25);
    }
    .bus-chip {
      border-radius: 999px;
      padding: 4px 9px;
      font-weight: 700;
      border: 1px solid;
      font-size: 0.74rem;
      letter-spacing: 0.02em;
    }
    .bus-chip.connected {
      color: #b7f7ca;
      border-color: #2f965d;
      background: #113022;
    }
    .bus-chip.disconnected {
      color: #ffb9b9;
      border-color: #aa4c4c;
      background: #351919;
    }
    .agorg-chip.active {
      color: #b7f7ca;
      border-color: #2f965d;
      background: #113022;
    }
    .agorg-chip.none {
      color: #ffe6a6;
      border-color: #997a33;
      background: #2f2610;
    }
    h1 { margin: 0; font-size: 2rem; line-height: 1.1; letter-spacing: 0.01em; }
    h2 { margin: 0; font-size: 1rem; color: var(--muted); font-weight: 500; }
    h3 { margin: 0 0 10px; font-size: 1.05rem; }
    .tabs {
      display: flex;
      gap: 10px;
      margin: 0 0 16px;
      flex-wrap: wrap;
    }
    button.tab {
      background: #182744;
      border: 1px solid #355285;
      color: #dbe7ff;
      padding: 9px 16px;
      border-radius: 999px;
      cursor: pointer;
      font-weight: 600;
      transition: all 0.15s ease;
    }
    button.tab:hover { border-color: #4b72b6; }
    button.tab.active {
      background: linear-gradient(90deg, #4f63dc, #3e56cf);
      border-color: #5c74ef;
      box-shadow: 0 0 0 3px rgba(79, 99, 220, 0.18);
    }
    .panel {
      display: none;
      border: 1px solid var(--border);
      border-radius: 14px;
      padding: 14px;
      background: rgba(16, 26, 44, 0.92);
      backdrop-filter: blur(2px);
    }
    .panel.active { display: block; }
    .grid {
      display: grid;
      gap: 14px;
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .card {
      background: linear-gradient(155deg, rgba(21, 34, 57, 0.92), rgba(15, 24, 40, 0.98));
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 14px;
      display: flex;
      flex-direction: column;
      gap: 10px;
    }
    input, select, textarea {
      width: 100%;
      background: #0d1526;
      color: #ebf1ff;
      border: 1px solid #334f7d;
      border-radius: 8px;
      padding: 10px 11px;
      font-size: 0.95rem;
    }
    textarea {
      resize: vertical;
      min-height: 110px;
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    }
    input::placeholder, textarea::placeholder { color: #7f94c6; }
    input:focus, select:focus, textarea:focus {
      outline: none;
      border-color: var(--accent);
      box-shadow: 0 0 0 3px rgba(48, 199, 244, 0.18);
    }
    .row {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
    }
    .btn {
      background: linear-gradient(90deg, var(--primary), #5567e4);
      border: 1px solid #7484ff;
      color: #fff;
      border-radius: 9px;
      padding: 9px 13px;
      cursor: pointer;
      font-weight: 600;
      transition: all 0.15s ease;
    }
    .btn:hover { background: linear-gradient(90deg, var(--primary-hover), #6578f5); }
    .btn.secondary {
      background: #1a2844;
      border-color: #3a578a;
      color: #d9e6ff;
    }
    .field-label {
      font-size: 0.82rem;
      color: #b6c7ee;
      font-weight: 700;
      letter-spacing: 0.01em;
    }
    .helper {
      font-size: 0.82rem;
      color: #9cb0dc;
      line-height: 1.45;
      margin-top: -4px;
    }
    .sequence-strip {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-bottom: 12px;
      padding: 8px;
      border: 1px solid #2f436f;
      border-radius: 10px;
      background: rgba(10, 19, 33, 0.7);
    }
    .seq-step {
      border-radius: 999px;
      border: 1px solid #3a578a;
      background: #152845;
      color: #d5e4ff;
      font-size: 0.85rem;
      font-weight: 700;
      padding: 4px 10px;
      white-space: nowrap;
      cursor: pointer;
    }
    .step {
      border: 1px solid #2f4975;
      border-radius: 10px;
      padding: 10px;
      background: rgba(13, 22, 38, 0.6);
      display: flex;
      flex-direction: column;
      gap: 8px;
    }
    .step-title {
      font-size: 0.9rem;
      font-weight: 700;
      color: #dbe7ff;
    }
    .status {
      margin-top: 14px;
      display: grid;
      gap: 14px;
      grid-template-columns: 1fr 1fr;
    }
    pre {
      margin: 0;
      background: #080f1a;
      border: 1px solid #2d426c;
      border-radius: 10px;
      padding: 12px;
      max-height: 480px;
      overflow: auto;
      font-size: 0.84rem;
      line-height: 1.4;
      white-space: pre-wrap;
      word-break: break-all;
    }
    .timeline {
      display: flex;
      flex-direction: column;
      gap: 10px;
      max-height: 340px;
      overflow: auto;
      padding-right: 4px;
    }
    .tl-card {
      border: 1px solid #2f436f;
      border-radius: 10px;
      background: #0a1321;
      padding: 10px;
      cursor: pointer;
    }
    .tl-card.selected {
      border-color: #6a7dff;
      box-shadow: inset 0 0 0 1px rgba(109, 125, 255, 0.55);
    }
    .tl-head {
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 8px;
      margin-bottom: 8px;
    }
    .tl-title {
      font-size: 0.88rem;
      font-weight: 600;
      color: #dce8ff;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .tl-badge {
      border-radius: 999px;
      font-size: 0.72rem;
      font-weight: 700;
      padding: 3px 8px;
      border: 1px solid;
    }
    .tl-badge.started { color: #9dc7ff; border-color: #3f6db5; background: #152845; }
    .tl-badge.progress { color: #9de7ff; border-color: #2c7d9c; background: #113242; }
    .tl-badge.completed { color: #b6f7cb; border-color: #2d8a52; background: #102d1f; }
    .tl-badge.failed { color: #ffb2b2; border-color: #9e3f3f; background: #341616; }
    .tl-steps {
      margin: 0;
      padding-left: 16px;
      font-size: 0.8rem;
      color: #a8b9e3;
      display: flex;
      flex-direction: column;
      gap: 4px;
    }
    .tl-empty {
      color: #8ca0cf;
      font-size: 0.88rem;
      border: 1px dashed #34507e;
      border-radius: 8px;
      padding: 10px;
      text-align: center;
    }
    .dep-status-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
    }
    .dep-status-card {
      border: 1px solid #2f436f;
      border-radius: 10px;
      padding: 10px;
      background: #0a1321;
    }
    .dep-status-card h4 {
      margin: 0 0 8px;
      font-size: 0.9rem;
    }
    .chip-row {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-bottom: 8px;
    }
    .chip {
      border-radius: 999px;
      font-size: 0.72rem;
      font-weight: 700;
      padding: 4px 10px;
      border: 1px solid #3a578a;
      background: #152845;
      color: #d5e4ff;
    }
    .chip.ok { border-color: #2d8a52; background: #102d1f; color: #b6f7cb; }
    .chip.fail { border-color: #9e3f3f; background: #341616; color: #ffb2b2; }
    .chip.warn { border-color: #997a33; background: #2f2610; color: #ffe6a6; }
    .chip.neutral { border-color: #3a578a; background: #152845; color: #d5e4ff; }
    .pre-wrap { position: relative; }
    .pre-actions {
      position: absolute; top: 10px; right: 20px;
      display: flex; gap: 6px; opacity: 0.35; transition: opacity 0.2s; z-index: 20;
    }
    .pre-wrap:hover .pre-actions { opacity: 1; }
    .action-btn {
      background: rgba(30, 45, 80, 0.85); border: 1px solid #355285; color: #dbe7ff;
      border-radius: 6px; padding: 4px 9px; font-size: 0.72rem; font-weight: 700;
      cursor: pointer; backdrop-filter: blur(4px); transition: all 0.1s ease;
    }
    .action-btn:hover { background: #4f63dc; color: #fff; border-color: #5c74ef; }
    .action-btn:active { transform: scale(0.95); }
    .dep-ok { color: #b6f7cb; }
    .dep-fail { color: #ffb2b2; }
    .muted { color: var(--muted); margin-top: 7px; }
    .three-panel-layout { display: flex; flex-direction: column; gap: 24px; }
    .panel-left, .panel-center, .panel-right { width: 100%; }
    .panel-center { border-top: 1px solid #2f436f; border-bottom: 1px solid #2f436f; padding: 24px 0; }
    .tree-node { cursor: pointer; padding: 4px 8px; border-radius: 4px; transition: background 0.2s; }
    .tree-node:hover { background: rgba(109, 125, 255, 0.15); }
    .tree-node.selected { background: rgba(109, 125, 255, 0.3); border: 1px solid #6a7dff; }
    .tree-node.agorg { color: #9dc7ff; font-weight: 700; }
    .tree-node.ago { color: #b6f7cb; }
    .tree-node.none { color: #8ca0cf; font-style: italic; }
    .sub-tabs { display: flex; gap: 4px; margin-bottom: 12px; border-bottom: 1px solid #2d426c; padding-bottom: 4px; }
    .sub-tab { background: none; border: none; color: #8ca0cf; font-size: 0.8rem; font-weight: 700; cursor: pointer; padding: 4px 8px; border-bottom: 2px solid transparent; }
    .sub-tab.active { color: #6a7dff; border-bottom-color: #6a7dff; }
    .sub-panel { display: none; }
    .sub-panel.active { display: block; }
    .batch-list { font-family: monospace; min-height: 80px; padding: 8px; background: #1a2a47; color: #b6f7cb; border: 1px solid #2d426c; border-radius: 4px; }
    
    /* Hero Dropdown */
    .agorg-scope-container { position: relative; display: inline-block; }
    .agorg-dropdown { 
      position: relative;
    }
    .agorg-dropdown-menu {
      position: absolute;
      top: 100%;
      left: 16px;
      margin-top: 4px;
      background: #0f111a;
      border: 1px solid #4e6ba6;
      border-radius: 4px;
      min-width: 280px;
      max-height: 400px;
      overflow-y: auto;
      z-index: 1000;
      display: none;
      box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    }
    .agorg-dropdown.active .agorg-dropdown-menu {
      display: block;
    }
    .agorg-drop-item {
      padding: 8px 12px;
      cursor: pointer;
      border-bottom: 1px solid #202b38;
      display: flex;
      justify-content: space-between;
      align-items: center;
      color: #a8b9e3;
    }
    .agorg-drop-item:hover {
      background: #1a2235;
      color: #fff;
    }
    .agorg-drop-item .type {
      font-size: 0.7rem;
      padding: 2px 6px;
      background: #202b38;
      border-radius: 4px;
      color: #8b9bb4;
    }
    .agorg-drop-header {
      padding: 8px 12px;
      font-size: 0.75rem;
      text-transform: uppercase;
      color: #4e6ba6;
      font-weight: bold;
      background: #161b22;
      border-bottom: 1px solid #202b38;
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
    @media (max-width: 980px) {
      .grid, .status { grid-template-columns: 1fr; }
      h1 { font-size: 1.72rem; }
    }
  </style>
</head>
<body>
<div class="wrap">
  <div class="hero">
    <h1>Arqon Pilot Control Panel</h1>
    <h2 class="muted">The Operating System for Synthetic Life</h2>
    <div class="bus-status-row">
      <div class="status-left">
        ArqonBus:
        <span id="bus-status-chip" class="bus-chip disconnected">DISCONNECTED</span>
      </div>
      <div class="system-menu" style="display:flex; gap:8px; align-items:center;">
        <button class="menu-btn" onclick="run('pilot.engine.stop', {})" title="Stop System"><span class="icon">⏹</span></button>
        <button class="menu-btn" onclick="run('pilot.engine.restart', {})" title="Restart Engine"><span class="icon">↺</span></button>
      </div>
      
      <!-- Active AGOrg Scope dropdown -->
      <div style="position:relative; display:inline-block;" class="agorg-dropdown" id="agorg-hero-dropdown-container">
        <button class="btn secondary" id="agorg-open-btn" style="margin-left: 16px; min-width: 140px; border-color:#4e6ba6; color:#a8b9e3;" onclick="toggleAgorgDropdown(event)">
          AGOrg: <span id="agorg-status-chip" style="color:#fff; font-weight:bold;">Loading...</span> ▼
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
  </div>

  <section class="panel active" id="dashboard">
    <div class="sequence-strip">
      <span class="seq-step">Status -> Bus Health -> Oracle Query -> Heal Plan -> Heal Run</span>
      <span class="seq-step">Branch Preview -> Multi Status -> DAG -> Staged Apply</span>
      <span class="seq-step">Push Safe -> Timeline Verify</span>
    </div>
    <div class="grid">
      <div class="card">
        <h3>System Status</h3>
        <div class="chip-row">
          <span id="dash-policy-chip" class="chip neutral">Policy: unknown</span>
          <span id="dash-hook-chip" class="chip neutral">Hook: unknown</span>
          <span id="dash-drift-chip" class="chip neutral">Drift: unknown</span>
          <span id="dash-bus-chip" class="chip neutral">Bus: unknown</span>
          <span id="dash-db-chip" class="chip neutral">DB: unknown</span>
          <span id="dash-gate-chip" class="chip neutral">Gate: unknown</span>
          <span id="dash-push-chip" class="chip neutral">Push: unknown</span>
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

      <div class="card">
        <h3>AGOrg Overview</h3>
        <div class="helper">Dashboard control summary for active AGOrg scope: score, unresolved issues, and class distribution.</div>
        <div class="chip-row">
          <span id="dash-agorg-score-chip" class="chip neutral">Score: unknown</span>
          <span id="dash-agorg-issues-chip" class="chip neutral">Issues: unknown</span>
          <span id="dash-agorg-offpolicy-chip" class="chip neutral">Off-policy: unknown</span>
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
      </div>

      <div class="card">
        <h3>Oracle + Heal Quick Ops</h3>
        <div class="helper">Fast path for day-to-day work: ask Oracle for context, then run Heal in plan mode first before applying.</div>
        <div class="chip-row">
          <span id="dash-oracle-chip" class="chip neutral">Oracle: idle</span>
          <span id="dash-heal-chip" class="chip neutral">Heal: idle</span>
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
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-duplicates-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-duplicates-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-duplicates-out">No duplicate merge candidates yet.</pre>
      </div>
      <div class="pre-wrap">
        <div class="pre-actions">
          <button class="action-btn" onclick="copyToClipboard('dash-agorg-class-counts-out', this)">COPY</button>
          <button class="action-btn" onclick="clearElement('dash-agorg-class-counts-out')">CLEAR</button>
        </div>
        <pre id="dash-agorg-class-counts-out">No issue class counts yet.</pre>
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

    </div>

    <div class="status">
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
  </section>

  <section class="panel" id="oracle">
    <div class="sequence-strip">
      <span class="seq-step">Scan Index</span>
      <span class="seq-step">Run Query</span>
      <span class="seq-step">Open Report</span>
    </div>
    <div class="grid">
      <div class="card">
        <h3>Oracle Scan / Query</h3>
        <div class="helper">`Scan Index` refreshes your code graph/vector index. `Run Query` asks Oracle over that indexed state.</div>
        <div class="chip-row">
          <span id="oracle-chip" class="chip neutral">Oracle: idle</span>
        </div>
        <button id="oracle-scan-btn" class="btn" onclick="oracleScan()">Scan Index</button>
        <input id="oracle-query" placeholder="where is branch sync implemented?" />
        <button id="oracle-query-btn" class="btn secondary" onclick="oracleQuery()">Run Query</button>
      </div>
      <div class="card">
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
      </div>
    </div>
  </section>

  <section class="panel" id="heal">
    <div class="sequence-strip">
      <span class="seq-step">Plan Only</span>
      <span class="seq-step">Review Response/Timeline</span>
      <span class="seq-step">Run Heal</span>
    </div>
    <div class="grid">
      <div class="card">
        <h3>Heal Controls</h3>
        <div class="helper">Recommended sequence: `Plan Only` first, inspect response/timeline, then `Run Heal` only when the plan is acceptable.</div>
        <div class="chip-row">
          <span id="heal-chip" class="chip neutral">Heal: idle</span>
        </div>
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
        <div class="row">
          <button id="heal-plan-btn" class="btn secondary" onclick="healPlan()">Plan Only</button>
          <button id="heal-run-btn" class="btn" onclick="healRun()">Run Heal</button>
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
    <div class="grid">
      <div class="card">
        <h3>Checks and Recovery</h3>
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
          <button class="btn secondary" onclick="depRun('policy')">Policy Check</button>
          <button class="btn secondary" onclick="depRun('hook-policy')">Hook Policy</button>
          <button class="btn secondary" onclick="depRun('drift')">Drift Report</button>
        </div>
        <div class="row">
          <button class="btn secondary" onclick="depRun('gate')">Run Gate</button>
          <button class="btn" onclick="depRun('repair')">Repair Lock (No Gate)</button>
        </div>
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
      <div class="card">
        <h3>Create Branch</h3>
        <input id="branch-name" placeholder="feat/pilot-wave7" />
        <input id="branch-base" placeholder="main" value="main" />
        <input id="branch-group" placeholder="core" />
        <input id="branch-tags" placeholder="apply-pilot,wave7" />
        <button class="btn" onclick="branchCreate()">Run</button>
      </div>
      <div class="card">
        <h3>Sync / Prune / Status</h3>
        <input id="sync-branch" placeholder="dev" value="dev" />
        <input id="sync-base" placeholder="main" value="main" />
        <div class="row">
          <button class="btn" onclick="branchSync()">Sync</button>
          <button class="btn secondary" onclick="branchPrune()">Prune</button>
          <button class="btn secondary" onclick="branchStatus()">Status</button>
        </div>
      </div>
    </div>
  </section>

  <section class="panel" id="multi">
    <div class="sequence-strip">
      <span class="seq-step">Register</span>
      <span class="seq-step">List -> Status -> Order</span>
      <span class="seq-step">DAG -> PR Plan</span>
      <span class="seq-step">Staged Apply (Dry Run -> Execute)</span>
    </div>
    <div class="grid">
      <div class="card">
        <h3>Register Repo</h3>
        <div class="helper">Register each repository once, then target groups/tags for all multi-repo operations below.</div>
        <input id="repo-path" placeholder="/path/to/repo" />
        <input id="repo-name" placeholder="ArqonContinuum" />
        <input id="repo-group" placeholder="core" />
        <input id="repo-tags" placeholder="apply-pilot,wave7" />
        <button class="btn" onclick="multiRegister()">Register</button>
      </div>
      <div class="card">
        <h3>List / Status / Order / DAG / PR Plan</h3>
        <div class="helper">Run in this order when uncertain: `List` -> `Status` -> `Order` -> `DAG` -> `PR Plan`.</div>
        <div class="chip-row">
          <span id="multi-dag-chip" class="chip neutral">DAG: idle</span>
        </div>
        <input id="multi-group" placeholder="core" />
        <input id="multi-tags" placeholder="apply-pilot,wave7" />
        <div class="row">
          <button class="btn secondary" onclick="multiList()">List</button>
          <button class="btn secondary" onclick="multiStatus()">Status</button>
          <button class="btn secondary" onclick="multiOrder()">Order</button>
          <button id="multi-dag-btn" class="btn secondary" onclick="multiDag()">DAG</button>
        </div>
        <button class="btn" onclick="multiPrsCreate()">PR Plan (Dry Run)</button>
      </div>
      <div class="card">
        <h3>Staged Apply (Dependency-Aware)</h3>
        <div class="helper">Runs branch creation in dependency stages. Start with `Dry Run`; use `Execute` only after preview looks correct.</div>
        <div class="chip-row">
          <span id="multi-apply-chip" class="chip neutral">Staged Apply: idle</span>
        </div>
        <input id="multi-apply-branch" placeholder="feat/pilot-wave13" value="feat/pilot-wave13" />
        <input id="multi-apply-base" placeholder="dev" value="dev" />
        <input id="multi-apply-pr-base" placeholder="main" value="main" />
        <input id="multi-apply-stage-size" placeholder="2" value="2" />
        <label style="font-size:0.82rem;color:#a8b9e3;">
          <input id="multi-apply-continue" type="checkbox" style="width:auto;vertical-align:middle;margin-right:6px;" />
          Continue on failure
        </label>
        <div class="row">
          <button id="multi-apply-dry-btn" class="btn secondary" onclick="multiApplyDryRun()">Staged Apply (Dry Run)</button>
          <button id="multi-apply-exec-btn" class="btn" onclick="multiApplyExecute()">Staged Apply (Execute)</button>
        </div>
      </div>
    </div>
  </section>

  <section class="panel" id="agorg">
    <div class="three-panel-layout">
      <!-- Panel 1: Settings/CRUD -->
      <div class="panel-left" style="display:flex; flex-direction:row; gap:16px;">
        <div style="flex:1;">
          <div class="card">
            <h3>Active Scope</h3>
            <div class="helper">Manage global scope. Switch between known AGOrg contexts or input manually.</div>
            <div id="agorg-active-details" style="background:#0f111a; border-radius:4px; padding:10px; border:1px solid #202b38; margin-bottom:12px; font-size:0.85rem; word-break:break-all;">
              <em>Loading active scope...</em>
            </div>
            <label class="field-label" for="agorg-use-id">Manual Switch (UUID or Name)</label>
            <div class="row">
              <input id="agorg-use-id" placeholder="UUID or name" />
              <button class="btn secondary" onclick="agorgUse()">Switch</button>
            </div>
            <div class="row" style="margin-top:8px;">
              <button class="btn secondary" onclick="agorgUpdate()">Update</button>
              <button class="btn secondary" style="color:#ff6b6b; border-color:#ff6b6b;" onclick="agorgDelete()">Delete</button>
            </div>
            <h4 style="margin:14px 0 6px;">Scope Profile Preferences</h4>
            <label class="field-label" for="agorg-profile-name">Profile Name</label>
            <input id="agorg-profile-name" placeholder="primary" />
            <label class="field-label" for="agorg-pref-default-branch">Default Branch</label>
            <input id="agorg-pref-default-branch" placeholder="dev" />
            <label class="field-label" for="agorg-pref-release-branch">Release Branch</label>
            <input id="agorg-pref-release-branch" placeholder="main" />
            <label style="font-size:0.82rem;color:#a8b9e3; display:block; margin-top:6px;">
              <input id="agorg-pref-auto-prune" type="checkbox" style="width:auto;vertical-align:middle;margin-right:6px;" />
              Auto-prune stale AGO rows by default
            </label>
            <div class="row" style="margin-top:8px;">
              <button class="btn secondary" onclick="agorgLoadPreferences()">Load Prefs</button>
              <button class="btn secondary" onclick="agorgSavePreferences()">Save Prefs</button>
            </div>
          </div>
        </div>

        <div style="flex:1;">
          <div class="card" style="height:100%; display:flex; flex-direction:column;">
            <h3>Registry</h3>
            <div class="helper" style="margin-bottom:8px;">Click to switch scope instantly.</div>
            <div id="agorg-registry-list" class="agorg-registry-list" style="flex:1; overflow-y:auto; background:#0f111a; border:1px solid #202b38; border-radius:4px;">
              <div style="padding:10px; color:#4e6ba6; font-size:0.8rem;">Loading registry...</div>
            </div>
          </div>
        </div>
      </div>

      <div class="panel-left">

        <div class="card" style="margin-top: 16px;">
          <div class="sub-tabs">
            <button class="sub-tab active" onclick="activateSubPanel('agorg-import-panel', this)">Import Existing</button>
            <button class="sub-tab" onclick="activateSubPanel('agorg-create-panel', this)">Create New</button>
          </div>

          <!-- Sub-Panel: Import -->
          <div id="agorg-import-panel" class="sub-panel active">
            <h3>Import AGOrg</h3>
            <div class="helper">Onboard an existing Master Directory. All AGOs/AGOrgs must exist as siblings within this space.</div>
            <label class="field-label" for="agorg-master">AGOrg Master Directory</label>
            <div class="row">
              <input id="agorg-master" placeholder="/path/to/parent/dir" value="/home/irbsurfer/Projects/arqon" />
              <button class="btn secondary" onclick="browseAgorgMaster()">Browse…</button>
            </div>
            <label class="field-label" for="agorg-name">AGOrg Name</label>
            <input id="agorg-name" placeholder="Arqon" value="Arqon" />
            <label class="field-label" for="agorg-root">Parent AGOrg Root Path (Active)</label>
            <div class="row">
              <input id="agorg-root" placeholder="/path/to/org/repo" value="/home/irbsurfer/Projects/arqon" />
              <button class="btn secondary" onclick="browseAgorgRoot()">Browse…</button>
            </div>
            <label class="field-label" for="agorg-depth">Discovery Depth</label>
            <input id="agorg-depth" placeholder="scan depth" value="4" />
            <div class="row">
              <label style="font-size:0.82rem;color:#a8b9e3;">
                <input id="agorg-autoscan" type="checkbox" checked style="width:auto;vertical-align:middle;margin-right:6px;" />
                autoscan
              </label>
              <label style="font-size:0.82rem;color:#a8b9e3;">
                <input id="agorg-import" type="checkbox" checked style="width:auto;vertical-align:middle;margin-right:6px;" />
                import discovery
              </label>
              <label style="font-size:0.82rem;color:#a8b9e3;">
                <input id="agorg-prune" type="checkbox" checked style="width:auto;vertical-align:middle;margin-right:6px;" />
                prune stale AGO rows
              </label>
              <label style="font-size:0.82rem;color:#a8b9e3;">
                <input id="agorg-default" type="checkbox" checked style="width:auto;vertical-align:middle;margin-right:6px;" />
                set default scope
              </label>
            </div>
            </div>
            <div class="row">
              <button class="btn secondary" onclick="agorgDiscoverPreview()">Discover Preview</button>
              <button class="btn secondary" onclick="agorgImportApproved()">Import Approved</button>
              <button class="btn secondary" onclick="agorgReconcile()">Policy Report</button>
              <button class="btn secondary" onclick="agorgReconcileDryRun()">Reconcile Dry Run</button>
              <button class="btn secondary" onclick="agorgReconcileApply()">Reconcile Apply</button>
              <button class="btn" onclick="agorgCreateProject()">Import</button>
            </div>
            <div class="row">
              <button class="btn secondary" onclick="agorgLoadPolicyReports()">Refresh Policy Artifacts</button>
              <select id="agorg-policy-report-select"></select>
              <button class="btn secondary" onclick="agorgOpenPolicyReport()">Open</button>
            </div>
            <div class="helper" style="margin-top:8px;">`Discover Preview` lets you approve/reject before import. `Import` is one-shot create + autoscan + import.</div>
            <div class="pre-wrap">
              <div class="pre-actions">
                <button class="action-btn" onclick="copyToClipboard('agorg-duplicate-preview-out', this)">COPY</button>
                <button class="action-btn" onclick="clearElement('agorg-duplicate-preview-out')">CLEAR</button>
              </div>
              <pre id="agorg-duplicate-preview-out">No duplicate merge candidates yet.</pre>
            </div>
            <div class="pre-wrap">
              <div class="pre-actions">
                <button class="action-btn" onclick="copyToClipboard('agorg-class-counts-out', this)">COPY</button>
                <button class="action-btn" onclick="clearElement('agorg-class-counts-out')">CLEAR</button>
              </div>
              <pre id="agorg-class-counts-out">No issue class counts yet.</pre>
            </div>
            <div class="row">
              <select id="agorg-issue-class-filter">
                <option value="all">All Classes</option>
                <option value="policy_branch">policy_branch</option>
                <option value="policy_dependency">policy_dependency</option>
                <option value="metadata">metadata</option>
                <option value="topology">topology</option>
              </select>
              <button class="btn secondary" onclick="agorgApplyIssueClassFilter()">Apply Filter</button>
              <button class="btn secondary" onclick="agorgPrevIssue()">Prev</button>
              <button class="btn secondary" onclick="agorgNextIssue()">Next</button>
            </div>
            <div class="pre-wrap">
              <div class="pre-actions">
                <button class="action-btn" onclick="copyToClipboard('agorg-filtered-issues-out', this)">COPY</button>
                <button class="action-btn" onclick="clearElement('agorg-filtered-issues-out')">CLEAR</button>
              </div>
              <pre id="agorg-filtered-issues-out">No filtered issues yet.</pre>
            </div>
            <div class="pre-wrap">
              <div class="pre-actions">
                <button class="action-btn" onclick="copyToClipboard('agorg-issue-detail-out', this)">COPY</button>
                <button class="action-btn" onclick="clearElement('agorg-issue-detail-out')">CLEAR</button>
              </div>
              <pre id="agorg-issue-detail-out">No issue selected.</pre>
            </div>
          </div>

          <!-- Sub-Panel: Create -->
          <div id="agorg-create-panel" class="sub-panel">
            <h3>Initialize New AGOrg</h3>
            <div class="helper">Create a new Master Directory and optionally instantiate several AGOs at once.</div>
            <label class="field-label" for="agorg-create-dest">Destination Parent Folder</label>
            <div class="row">
              <input id="agorg-create-dest" placeholder="/home/irbsurfer/Projects/arqon" value="/home/irbsurfer/Projects/arqon" />
              <button class="btn secondary" onclick="browseAgorgCreateDest()">Browse…</button>
            </div>
            <label class="field-label" for="agorg-create-name">New Master Directory Name</label>
            <input id="agorg-create-name" placeholder="MyNewOrg" />
            
            <label class="field-label" for="agorg-create-siblings">Sibling AGOs to Create (one per line)</label>
            <textarea id="agorg-create-siblings" class="batch-list" placeholder="Core&#10;Pilot&#10;Sense"></textarea>
            
            <div class="row" style="margin-top:10px;">
              <label style="font-size:0.82rem;color:#a8b9e3;">
                <input id="agorg-create-git" type="checkbox" checked style="width:auto;vertical-align:middle;margin-right:6px;" />
                git init each
              </label>
            </div>
            
            <div class="row">
              <button class="btn" onclick="agorgBatchCreate()">Batch Create & Register</button>
            </div>
          </div>
        </div>
      </div>

      <!-- Panel 2: Interactive Hierarchy -->
      <div class="panel-center">
        <div class="card">
          <div class="row" style="justify-content: space-between; align-items: center; margin-bottom: 8px;">
            <h3 style="margin:0">Master Hierarchy</h3>
            <button class="btn secondary" onclick="agorgScanMaster()">Scan Master</button>
          </div>
          <div class="helper">Interactive view of all siblings in the Master Directory. Click to select; drag to link (TODO).</div>
          <div id="agorg-hierarchy-tree" class="timeline" style="max-height: 800px; padding: 10px; border: 1px solid #2d426c; border-radius: 10px; background: rgba(0,0,0,0.2);">
            <div class="tl-empty">No hierarchy loaded. Click "Scan Master" or "Import".</div>
          </div>
        </div>
      </div>

      <!-- Panel 3: Results Display -->
      <div class="panel-right">
        <div class="card">
          <h3>AGOrg Response</h3>
          <div class="pre-wrap">
            <div class="pre-actions">
              <button class="action-btn" onclick="copyToClipboard('agorg-out', this)">COPY</button>
              <button class="action-btn" onclick="clearElement('agorg-out')">CLEAR</button>
            </div>
            <pre id="agorg-out">ready</pre>
          </div>
        </div>
        <div class="card">
          <h3>Discovery Output</h3>
          <div class="pre-wrap">
            <div class="pre-actions">
              <button class="action-btn" onclick="copyToClipboard('agorg-discovery-out', this)">COPY</button>
              <button class="action-btn" onclick="clearElement('agorg-discovery-out')">CLEAR</button>
            </div>
            <pre id="agorg-discovery-out">[]</pre>
          </div>
        </div>
        <div class="card">
          <h3>Discovery Review (Approve / Reject)</h3>
          <div class="row">
            <button class="btn secondary" onclick="agorgSelectAllReview(true)">Approve All</button>
            <button class="btn secondary" onclick="agorgSelectAllReview(false)">Reject All</button>
            <button class="btn secondary" onclick="agorgLoadReviews()">Refresh Reviews</button>
            <button class="btn secondary" onclick="agorgLoadSelectedReview()">Load Review</button>
          </div>
          <label class="field-label" for="agorg-review-select">Saved Review Sessions</label>
          <select id="agorg-review-select"></select>
          <div class="helper">Only approved AGO candidates are imported by `Import Approved`.</div>
          <div id="agorg-discovery-review" class="timeline" style="max-height: 320px; overflow-y: auto; padding: 8px; border: 1px solid #2d426c; border-radius: 10px; background: rgba(0,0,0,0.2);">
            <div class="tl-empty">Run Discover Preview to populate candidates.</div>
          </div>
        </div>
      </div>
    </div>
  </section>

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

  <div class="status">
    <div class="card">
      <h3>Response</h3>
    <div class="pre-wrap">
      <div class="pre-actions">
        <button class="action-btn" onclick="copyToClipboard('out', this)">COPY</button>
        <button class="action-btn" onclick="clearElement('out')">CLEAR</button>
      </div>
      <pre id="out">ready</pre>
    </div>
    </div>
    <div class="card">
      <h3>Dependencies Action Output</h3>
    <div class="pre-wrap">
      <div class="pre-actions">
        <button class="action-btn" onclick="copyToClipboard('dep-action-out-global', this)">COPY</button>
        <button class="action-btn" onclick="clearElement('dep-action-out-global')">CLEAR</button>
      </div>
      <pre id="dep-action-out-global">No dependency action run yet.</pre>
    </div>
    </div>
  </div>
</div>
<script src="/static/pilot_ui.js"></script>
</body>
</html>"#;
