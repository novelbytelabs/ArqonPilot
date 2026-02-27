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
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command as TokioCommand;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Clone)]
pub struct UiConfig {
    pub host: String,
    pub port: u16,
    pub bus: BusBridgeConfig,
    pub allow_mutations: bool,
    pub allowed_commands: Option<HashSet<String>>,
}

#[derive(Clone)]
struct UiState {
    bus: BusBridgeConfig,
    events: broadcast::Sender<Value>,
    allow_mutations: bool,
    allowed_commands: Option<HashSet<String>>,
    codex_contracts: Arc<Mutex<HashMap<String, CodexContractRecord>>>,
    codex_contracts_log: PathBuf,
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

pub async fn run_ui_server(cfg: UiConfig) -> Result<()> {
    let (event_tx, _) = broadcast::channel(512);
    spawn_bus_telemetry_listener(cfg.bus.clone(), event_tx.clone());
    let codex_contracts_log = codex_contracts_log_path();
    let contract_seed = load_persisted_codex_contracts(&codex_contracts_log).unwrap_or_default();
    let state = Arc::new(UiState {
        bus: cfg.bus,
        events: event_tx,
        allow_mutations: cfg.allow_mutations,
        allowed_commands: cfg.allowed_commands,
        codex_contracts: Arc::new(Mutex::new(contract_seed)),
        codex_contracts_log,
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/api/command", post(run_command))
        .route("/api/history", get(get_history))
        .route("/api/reports", get(get_reports))
        .route("/api/report", get(get_report_content))
        .route("/api/codex/contracts", get(get_codex_contracts))
        .route("/api/codex/contract", get(get_codex_contract))
        .route("/api/dependencies/run", post(run_dependency_action))
        .route("/api/dependencies/logs", get(get_dependency_logs))
        .route("/api/evidence/export", post(export_evidence_bundle))
        .route("/api/codex/action", post(run_codex_action))
        .route("/api/stream", get(stream_events))
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

async fn run_dependency_action(
    State(state): State<Arc<UiState>>,
    Json(req): Json<DependencyActionRequest>,
) -> Response {
    let action = req.action.trim();
    if action.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "action is required");
    }
    if action == "repair" && !state.allow_mutations {
        return error_response(
            StatusCode::FORBIDDEN,
            "repair action blocked in read-only UI mode",
        );
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
        if command.is_empty() || !command.starts_with("pilot.") {
            return error_response(
                StatusCode::BAD_REQUEST,
                "command must be namespaced as pilot.*",
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

        match send_command_once_with_retry(
            &state.bus,
            &execute_contract.command,
            execute_contract.payload_normalized.clone(),
            3,
        )
        .await
        {
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
        if verify_cmd.starts_with("pilot.") {
            if let Some(allowlist) = &state.allowed_commands {
                if !allowlist.contains(verify_cmd) {
                    return error_response(
                        StatusCode::FORBIDDEN,
                        &format!("verify command '{}' is not in ui allowlist", verify_cmd),
                    );
                }
            }
            match send_command_once_with_retry(
                &state.bus,
                verify_cmd,
                reconcile_contract.verify_payload.clone(),
                3,
            )
            .await
            {
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
        append_codex_contract_record, is_safe_cli_token, load_persisted_codex_contracts,
        CodexContractRecord,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    let stream = BroadcastStream::new(rx).filter_map(|item| async move {
        match item {
            Ok(value) => Some(Ok(Event::default()
                .event("pilot_event")
                .data(value.to_string()))),
            Err(_) => None,
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
        "pilot.branch.create"
        | "pilot.branch.sync"
        | "pilot.branch.prune"
        | "pilot.multi.prs.create" => !payload_truthy_bool(payload, "dry_run"),
        _ => is_mutating_command(command),
    }
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

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
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
      gap: 8px;
      font-size: 0.86rem;
      color: #b8c8ef;
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
      max-height: 340px;
      overflow: auto;
      font-size: 0.84rem;
      line-height: 1.4;
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
    .dep-ok { color: #b6f7cb; }
    .dep-fail { color: #ffb2b2; }
    .muted { color: var(--muted); margin-top: 7px; }
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
    <h2 class="muted">Oracle + Heal + Dependencies + Branch + Multi + Telemetry over ArqonBus (`pilot serve` required)</h2>
    <div class="bus-status-row">
      ArqonBus:
      <span id="bus-status-chip" class="bus-chip disconnected">DISCONNECTED</span>
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
    <div class="grid">
      <div class="card">
        <h3>System Status</h3>
        <div class="chip-row">
          <span id="dash-policy-chip" class="chip neutral">Policy: unknown</span>
          <span id="dash-hook-chip" class="chip neutral">Hook: unknown</span>
          <span id="dash-drift-chip" class="chip neutral">Drift: unknown</span>
          <span id="dash-bus-chip" class="chip neutral">Bus: unknown</span>
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
          <button class="btn secondary" onclick="dashBusStatus()">Bus Status</button>
          <button class="btn secondary" onclick="dashExportEvidence()">Export Evidence</button>
        </div>
        <div class="row">
          <input id="dash-push-branch" placeholder="main" value="main" />
          <input id="dash-push-remote" placeholder="origin" value="origin" />
          <button class="btn" onclick="dashRunPush()">Push Safe</button>
        </div>
        <pre id="dash-status-out">ready</pre>
      </div>

      <div class="card">
        <h3>Oracle + Heal Quick Ops</h3>
        <input id="dash-oracle-query" placeholder="where is branch sync implemented?" />
        <div class="row">
          <button class="btn secondary" onclick="oracleScan()">Oracle Scan</button>
          <button class="btn secondary" onclick="dashOracleQuery()">Oracle Query</button>
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
          <button class="btn secondary" onclick="dashHealPlan()">Heal Plan</button>
          <button class="btn" onclick="dashHealRun()">Heal Run</button>
        </div>
      </div>

      <div class="card">
        <h3>Branch + Multi Quick Ops</h3>
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
        <pre id="op-detail">[]</pre>
      </div>
    </div>

    <div class="card">
      <h3>Live Event Stream</h3>
      <div class="row">
        <button id="stream-toggle" class="btn secondary" onclick="toggleStream()">Pause Stream</button>
        <button class="btn secondary" onclick="clearLive()">Clear</button>
      </div>
      <pre id="live-stream">[]</pre>
    </div>
  </section>

  <section class="panel" id="oracle">
    <div class="grid">
      <div class="card">
        <h3>Oracle Scan / Query</h3>
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
        <pre id="oracle-report-content">No report selected.</pre>
      </div>
    </div>
  </section>

  <section class="panel" id="heal">
    <div class="grid">
      <div class="card">
        <h3>Heal Controls</h3>
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
        <pre id="dep-action-out">No dependency action run yet.</pre>
      </div>
      <div class="card">
        <h3>Recent Gate Logs</h3>
        <button class="btn secondary" onclick="depLoadLogs()">Refresh Logs</button>
        <pre id="dep-logs">[]</pre>
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
    <div class="grid">
      <div class="card">
        <h3>Register Repo</h3>
        <input id="repo-path" placeholder="/path/to/repo" />
        <input id="repo-name" placeholder="ArqonContinuum" />
        <input id="repo-group" placeholder="core" />
        <input id="repo-tags" placeholder="apply-pilot,wave7" />
        <button class="btn" onclick="multiRegister()">Register</button>
      </div>
      <div class="card">
        <h3>List / Status / Order / DAG / PR Plan</h3>
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
        <div class="helper">Runs branch creation in dependency stages. Default is dry-run preview.</div>
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

  <section class="panel" id="telemetry">
    <div class="grid">
      <div class="card">
        <h3>Telemetry Mirror</h3>
        <div class="row">
          <button class="btn secondary" onclick="syncTelemetryMirror()">Refresh Mirror</button>
        </div>
        <pre id="telemetry-mirror">[]</pre>
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
        <pre id="codex-out">No Codex action run yet.</pre>
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
        <pre id="codex-contracts-out">No contracts loaded yet.</pre>
      </div>
    </div>
  </section>

  <div class="status">
    <div class="card">
      <h3>Response</h3>
      <pre id="out">ready</pre>
    </div>
    <div class="card">
      <h3>Dependencies Action Output</h3>
      <pre id="dep-action-out-global">No dependency action run yet.</pre>
    </div>
  </div>
</div>
<script>
const out = document.getElementById('out');
const liveStream = document.getElementById('live-stream');
const busStatusChip = document.getElementById('bus-status-chip');
const opDetailMeta = document.getElementById('op-detail-meta');
const opDetailArtifact = document.getElementById('op-detail-artifact');
const opDetail = document.getElementById('op-detail');
const timelineEl = document.getElementById('timeline');
const failedOnlyToggle = document.getElementById('failed-only');
const timelineCommandFilter = document.getElementById('timeline-command-filter');
const timelineTextFilter = document.getElementById('timeline-text-filter');
const streamToggleBtn = document.getElementById('stream-toggle');
const oracleReportSelect = document.getElementById('oracle-report-select');
const oracleReportContent = document.getElementById('oracle-report-content');
const depActionOut = document.getElementById('dep-action-out');
const depActionOutGlobal = document.getElementById('dep-action-out-global');
const depLogs = document.getElementById('dep-logs');
const depPolicyStatus = document.getElementById('dep-policy-status');
const depHookStatus = document.getElementById('dep-hook-status');
const depDriftStatus = document.getElementById('dep-drift-status');
const codexOut = document.getElementById('codex-out');
const codexContractsOut = document.getElementById('codex-contracts-out');
const codexContractSelect = document.getElementById('codex-contract-select');
const telemetryMirror = document.getElementById('telemetry-mirror');
const dashStatusOut = document.getElementById('dash-status-out');
const dashPolicyChip = document.getElementById('dash-policy-chip');
const dashHookChip = document.getElementById('dash-hook-chip');
const dashDriftChip = document.getElementById('dash-drift-chip');
const dashBusChip = document.getElementById('dash-bus-chip');
const dashGateChip = document.getElementById('dash-gate-chip');
const dashPushChip = document.getElementById('dash-push-chip');
const multiDagChip = document.getElementById('multi-dag-chip');
const multiApplyChip = document.getElementById('multi-apply-chip');
const multiDagBtn = document.getElementById('multi-dag-btn');
const multiApplyDryBtn = document.getElementById('multi-apply-dry-btn');
const multiApplyExecBtn = document.getElementById('multi-apply-exec-btn');
const oracleChip = document.getElementById('oracle-chip');
const oracleScanBtn = document.getElementById('oracle-scan-btn');
const oracleQueryBtn = document.getElementById('oracle-query-btn');
const healChip = document.getElementById('heal-chip');
const healPlanBtn = document.getElementById('heal-plan-btn');
const healRunBtn = document.getElementById('heal-run-btn');
const BUS_HEALTH_KEY = 'pilot.bus.health.v1';
const timelineState = new Map();
let selectedOperationId = null;
let auditCache = [];
let streamPaused = false;
let streamHandle = null;
let latestCodexContractId = '';

for (const btn of document.querySelectorAll('.tab')) {
  btn.addEventListener('click', () => {
    for (const t of document.querySelectorAll('.tab')) t.classList.remove('active');
    for (const p of document.querySelectorAll('.panel')) p.classList.remove('active');
    btn.classList.add('active');
    document.getElementById(btn.dataset.tab).classList.add('active');
  });
}

function tags(v) { return v.split(',').map(s => s.trim()).filter(Boolean); }
function setButtonBusy(btn, busy, runningLabel) {
  if (!btn) return;
  if (!btn.dataset.defaultLabel) {
    btn.dataset.defaultLabel = btn.textContent || '';
  }
  btn.disabled = !!busy;
  if (busy && runningLabel) {
    btn.textContent = runningLabel;
  } else {
    btn.textContent = btn.dataset.defaultLabel;
  }
}

function setChipState(chip, label, state, suffix) {
  if (!chip) return;
  let level = 'neutral';
  if (state === 'running') level = 'warn';
  if (state === 'success') level = 'ok';
  if (state === 'failed') level = 'fail';
  chip.className = 'chip ' + level;
  const detail = suffix ? (': ' + suffix) : '';
  chip.textContent = label + detail;
}

async function run(command, payload, opts = {}) {
  const label = opts.label || command;
  const chip = opts.chip || null;
  const buttons = Array.isArray(opts.buttons) ? opts.buttons : [];
  payload.schema_version = 1;
  setChipState(chip, label, 'running', 'running');
  for (const b of buttons) setButtonBusy(b, true, opts.runningLabel || null);
  out.textContent = JSON.stringify({status: "running", command, payload}, null, 2);
  if (dashStatusOut) {
    dashStatusOut.textContent = out.textContent;
  }
  try {
    const ctl = new AbortController();
    const timeoutId = setTimeout(() => ctl.abort(), 25000);
    const res = await fetch('/api/command', {
      method: 'POST',
      headers: {'content-type':'application/json'},
      body: JSON.stringify({ command, payload }),
      signal: ctl.signal
    });
    clearTimeout(timeoutId);
    const data = await res.json();
    out.textContent = JSON.stringify(data, null, 2);
    if (dashStatusOut) {
      dashStatusOut.textContent = JSON.stringify(data, null, 2);
    }
    const ok = !!data.ok;
    setChipState(chip, label, ok ? 'success' : 'failed', ok ? 'success' : 'failed');
    appendLive({ source: 'ui_command', command, ok: !!data.ok, status: res.status });
    loadHistory();
    return data;
  } catch (err) {
    const msg = (err && err.name === 'AbortError')
      ? 'Request timed out. Check ArqonBus bridge health and try again.'
      : (err && err.message ? err.message : String(err));
    const payloadErr = { ok: false, error: msg, command };
    out.textContent = JSON.stringify(payloadErr, null, 2);
    if (dashStatusOut) dashStatusOut.textContent = out.textContent;
    setChipState(chip, label, 'failed', 'failed');
    appendLive({ source: 'ui_command', command, ok: false, error: msg });
    return payloadErr;
  } finally {
    for (const b of buttons) setButtonBusy(b, false, null);
  }
}

function appendLive(eventObj) {
  const current = liveStream.textContent.trim();
  let arr = [];
  if (current && current !== '[]') {
    try { arr = JSON.parse(current); } catch (_) { arr = []; }
  }
  arr.push(eventObj);
  if (arr.length > 120) arr = arr.slice(arr.length - 120);
  liveStream.textContent = JSON.stringify(arr, null, 2);
  if (telemetryMirror) {
    const tail = arr.slice(Math.max(0, arr.length - 20));
    telemetryMirror.textContent = JSON.stringify(tail, null, 2);
  }
  ingestTimeline(eventObj);
}

function clearLive() {
  liveStream.textContent = '[]';
  if (telemetryMirror) telemetryMirror.textContent = '[]';
}

function syncTelemetryMirror() {
  if (!telemetryMirror) return;
  const current = liveStream.textContent.trim();
  try {
    const arr = current ? JSON.parse(current) : [];
    const tail = Array.isArray(arr) ? arr.slice(Math.max(0, arr.length - 20)) : [];
    telemetryMirror.textContent = JSON.stringify(tail, null, 2);
  } catch (_) {
    telemetryMirror.textContent = current || '[]';
  }
}

function setBusStatus(connected, note) {
  busStatusChip.textContent = connected ? 'CONNECTED' : 'DISCONNECTED';
  busStatusChip.classList.toggle('connected', connected);
  busStatusChip.classList.toggle('disconnected', !connected);
  if (dashBusChip) {
    setChip(dashBusChip, 'Bus: ' + (connected ? 'RUNNING' : 'STOPPED'), connected ? 'ok' : 'fail');
  }
  try {
    localStorage.setItem(BUS_HEALTH_KEY, JSON.stringify({
      connected,
      note: note || '',
      at: new Date().toISOString()
    }));
  } catch (_) {}
  if (note) {
    opDetailMeta.textContent = note;
  }
}

function restoreBusStatus() {
  try {
    const raw = localStorage.getItem(BUS_HEALTH_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (typeof parsed.connected === 'boolean') {
      setBusStatus(parsed.connected, parsed.note || '');
    }
  } catch (_) {}
}

function filteredTimelineItems() {
  const cmdNeedle = String(timelineCommandFilter.value || '').trim().toLowerCase();
  const textNeedle = String(timelineTextFilter.value || '').trim().toLowerCase();
  return Array.from(timelineState.values())
    .sort((a, b) => String(b.updatedAt).localeCompare(String(a.updatedAt)))
    .filter((x) => !failedOnlyToggle.checked || x.phase === 'failed')
    .filter((x) => !cmdNeedle || String(x.command || '').toLowerCase().includes(cmdNeedle))
    .filter((x) => {
      if (!textNeedle) return true;
      const hay = [
        x.opId || '',
        x.command || '',
        ...(x.steps || []).map((s) => s.summary || '')
      ].join(' ').toLowerCase();
      return hay.includes(textNeedle);
    });
}

function exportTimeline() {
  const items = filteredTimelineItems();
  const payload = {
    exported_at: new Date().toISOString(),
    filters: {
      failed_only: !!failedOnlyToggle.checked,
      command_contains: timelineCommandFilter.value || '',
      text_contains: timelineTextFilter.value || ''
    },
    items
  };
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'pilot_timeline_export.json';
  a.click();
  URL.revokeObjectURL(url);
}

function extractTimelineRecord(evt) {
  if (!evt || typeof evt !== 'object') return null;

  if (typeof evt.eventType === 'string' && evt.eventType.startsWith('pilot.op.')) {
    const payload = evt.payload || {};
    const opId = payload.operation_id || payload.operationId;
    if (!opId) return null;
    return {
      opId,
      phase: evt.eventType.replace('pilot.op.', '') || 'progress',
      command: payload.command || 'unknown',
      summary: payload.summary || '',
      at: payload.timestamp || new Date().toISOString()
    };
  }

  if (evt.source === 'ui_command' && typeof evt.command === 'string') {
    const success = !!(evt.response && evt.response.success);
    return {
      opId: (evt.response && evt.response.reply_to) || ('ui-' + Date.now()),
      phase: success ? 'completed' : 'failed',
      command: evt.command,
      summary: evt.error || (evt.response && evt.response.data && evt.response.data.summary) || '',
      at: new Date().toISOString()
    };
  }

  return null;
}

function ingestTimeline(evt) {
  const rec = extractTimelineRecord(evt);
  if (!rec) return;

  const current = timelineState.get(rec.opId) || {
    opId: rec.opId,
    command: rec.command,
    phase: 'started',
    updatedAt: rec.at,
    steps: [],
    rawEvents: []
  };

  current.command = rec.command || current.command;
  current.phase = rec.phase || current.phase;
  current.updatedAt = rec.at || current.updatedAt;
  current.steps.push({
    phase: rec.phase,
    summary: rec.summary || '',
    at: rec.at || new Date().toISOString()
  });
  current.rawEvents.push(evt);
  if (current.steps.length > 10) current.steps = current.steps.slice(current.steps.length - 10);
  if (current.rawEvents.length > 20) current.rawEvents = current.rawEvents.slice(current.rawEvents.length - 20);

  timelineState.set(rec.opId, current);
  if (!selectedOperationId) selectedOperationId = rec.opId;
  renderTimeline();
  renderOperationDetail();
}

function renderTimeline() {
  timelineEl.innerHTML = '';
  const items = filteredTimelineItems().slice(0, 40);

  if (!items.length) {
    const empty = document.createElement('div');
    empty.className = 'tl-empty';
    empty.textContent = 'No operations yet';
    timelineEl.appendChild(empty);
    return;
  }

  for (const item of items) {
    const card = document.createElement('div');
    card.className = 'tl-card';
    if (item.opId === selectedOperationId) {
      card.classList.add('selected');
    }
    card.addEventListener('click', () => {
      selectedOperationId = item.opId;
      renderTimeline();
      renderOperationDetail();
    });

    const head = document.createElement('div');
    head.className = 'tl-head';

    const title = document.createElement('div');
    title.className = 'tl-title';
    title.textContent = item.command + ' (' + item.opId + ')';

    const badge = document.createElement('span');
    const phaseClass = ['started', 'progress', 'completed', 'failed'].includes(item.phase) ? item.phase : 'progress';
    badge.className = 'tl-badge ' + phaseClass;
    badge.textContent = String(item.phase).toUpperCase();

    head.appendChild(title);
    head.appendChild(badge);
    card.appendChild(head);

    const steps = document.createElement('ul');
    steps.className = 'tl-steps';
    for (const step of item.steps.slice().reverse()) {
      const li = document.createElement('li');
      const msg = step.summary ? ' - ' + step.summary : '';
      li.textContent = '[' + step.at + '] ' + step.phase + msg;
      steps.appendChild(li);
    }
    card.appendChild(steps);

    timelineEl.appendChild(card);
  }
}

function shortCommand(cmd) {
  if (!cmd) return '';
  return cmd.startsWith('pilot.') ? cmd.slice(6) : cmd;
}

function inferArtifactPath(item) {
  const cmd = shortCommand(item.command);
  for (let i = auditCache.length - 1; i >= 0; i--) {
    const ev = auditCache[i] || {};
    if (ev.command === cmd && ev.artifact_path) {
      return ev.artifact_path;
    }
  }
  return '';
}

function renderOperationDetail() {
  const item = selectedOperationId ? timelineState.get(selectedOperationId) : null;
  if (!item) {
    opDetailMeta.textContent = 'Select a timeline item';
    opDetailArtifact.textContent = '';
    opDetail.textContent = '[]';
    return;
  }
  opDetailMeta.textContent = item.command + ' | ' + item.opId + ' | phase=' + item.phase;
  const artifact = inferArtifactPath(item);
  opDetailArtifact.textContent = artifact ? ('Artifact: ' + artifact) : 'Artifact: (not resolved)';
  opDetail.textContent = JSON.stringify(item.rawEvents || [], null, 2);
}

failedOnlyToggle.addEventListener('change', renderTimeline);
timelineCommandFilter.addEventListener('input', renderTimeline);
timelineTextFilter.addEventListener('input', renderTimeline);

function branchCreate() {
  run('pilot.branch.create', {
    branch: document.getElementById('branch-name').value,
    base_branch: document.getElementById('branch-base').value,
    group: document.getElementById('branch-group').value || null,
    tags: tags(document.getElementById('branch-tags').value),
    dry_run: true
  });
}
function branchSync() {
  run('pilot.branch.sync', {
    branch: document.getElementById('sync-branch').value,
    base_branch: document.getElementById('sync-base').value,
    dry_run: true
  });
}
function branchPrune() {
  run('pilot.branch.prune', {
    base_branch: document.getElementById('sync-base').value,
    dry_run: true
  });
}
function branchStatus() { run('pilot.branch.status', { group: null, tags: [] }); }

function oracleScan() {
  run('pilot.oracle.scan', {}, {
    label: 'Oracle',
    chip: oracleChip,
    buttons: [oracleScanBtn, oracleQueryBtn],
    runningLabel: 'Running...'
  });
}

function oracleQuery() {
  run('pilot.oracle.query', {
    query: document.getElementById('oracle-query').value,
    cli: true
  }, {
    label: 'Oracle',
    chip: oracleChip,
    buttons: [oracleScanBtn, oracleQueryBtn],
    runningLabel: 'Running...'
  });
}

function dashOracleQuery() {
  run('pilot.oracle.query', {
    query: document.getElementById('dash-oracle-query').value,
    cli: true
  });
}

function healPayload(planOnly) {
  const maxAttemptsRaw = document.getElementById('heal-max-attempts').value;
  const maxFilesRaw = document.getElementById('heal-max-files').value;
  const maxAttempts = parseInt(maxAttemptsRaw || '2', 10);
  const maxFiles = parseInt(maxFilesRaw || '5', 10);
  return {
    log_file: document.getElementById('heal-log-file').value || 'test_output.json',
    target: document.getElementById('heal-target').value || null,
    max_attempts: Number.isFinite(maxAttempts) ? maxAttempts : 2,
    max_files: Number.isFinite(maxFiles) ? maxFiles : 5,
    verbose: !!document.getElementById('heal-verbose').checked,
    plan_only: !!planOnly
  };
}

function healPlan() {
  run('pilot.heal.run', healPayload(true), {
    label: 'Heal',
    chip: healChip,
    buttons: [healPlanBtn, healRunBtn],
    runningLabel: 'Running...'
  });
}

function healRun() {
  run('pilot.heal.run', healPayload(false), {
    label: 'Heal',
    chip: healChip,
    buttons: [healPlanBtn, healRunBtn],
    runningLabel: 'Running...'
  });
}

function dashHealPayload(planOnly) {
  const maxAttemptsRaw = document.getElementById('dash-heal-max-attempts').value;
  const maxFilesRaw = document.getElementById('dash-heal-max-files').value;
  const maxAttempts = parseInt(maxAttemptsRaw || '2', 10);
  const maxFiles = parseInt(maxFilesRaw || '5', 10);
  return {
    log_file: document.getElementById('dash-heal-log-file').value || 'test_output.json',
    target: document.getElementById('dash-heal-target').value || null,
    max_attempts: Number.isFinite(maxAttempts) ? maxAttempts : 2,
    max_files: Number.isFinite(maxFiles) ? maxFiles : 5,
    verbose: false,
    plan_only: !!planOnly
  };
}

function dashHealPlan() {
  run('pilot.heal.run', dashHealPayload(true));
}

function dashHealRun() {
  run('pilot.heal.run', dashHealPayload(false));
}

async function oracleLoadReports() {
  const res = await fetch('/api/reports?limit=200');
  const data = await res.json();
  const rows = (data && data.reports) ? data.reports : [];
  oracleReportSelect.innerHTML = '';
  if (!rows.length) {
    const opt = document.createElement('option');
    opt.value = '';
    opt.textContent = 'No report files found in ~/.pilot/reports';
    oracleReportSelect.appendChild(opt);
    oracleReportContent.textContent = 'No report files found.';
    return;
  }
  for (const row of rows) {
    const opt = document.createElement('option');
    opt.value = row.path;
    const kb = Math.max(1, Math.round((row.size_bytes || 0) / 1024));
    opt.textContent = row.path + ' (' + kb + ' KB)';
    oracleReportSelect.appendChild(opt);
  }
}

async function oracleViewReport() {
  const path = oracleReportSelect.value;
  if (!path) {
    oracleReportContent.textContent = 'No report selected.';
    return;
  }
  const res = await fetch('/api/report?path=' + encodeURIComponent(path) + '&max_bytes=524288');
  const data = await res.json();
  if (!data || !data.ok) {
    oracleReportContent.textContent = JSON.stringify(data, null, 2);
    return;
  }
  oracleReportContent.textContent = data.content || '';
}

async function depRun(action) {
  const isJsonAction = action === 'policy' || action === 'hook-policy';
  const req = { action, json: isJsonAction };
  if (action === 'push') {
    req.branch = document.getElementById('dash-push-branch').value || 'main';
    req.remote = document.getElementById('dash-push-remote').value || 'origin';
  }
  const res = await fetch('/api/dependencies/run', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const data = await res.json();
  if (isJsonAction) {
    try {
      const parsed = JSON.parse(data.stdout || '{}');
      if (action === 'policy') setDepStatus(depPolicyStatus, parsed);
      if (action === 'hook-policy') setDepStatus(depHookStatus, parsed);
      if (action === 'drift') setDepDriftStatus(parsed && parsed.ok ? 'PASS' : 'FAIL');
    } catch (_) {}
  }
  if (action === 'drift' && !isJsonAction) {
    setDepDriftStatus(data.ok ? 'PASS' : 'FAIL');
  }
  if (action.startsWith('bus-')) {
    const text = String(data.stdout || '') + '\n' + String(data.stderr || '');
    if (text.includes('RUNNING')) setBusStatus(true, 'bus shim reported RUNNING');
    if (text.includes('STOPPED')) setBusStatus(false, 'bus shim reported STOPPED');
  }
  depActionOut.textContent = JSON.stringify(data, null, 2);
  if (depActionOutGlobal) {
    depActionOutGlobal.textContent = JSON.stringify(data, null, 2);
  }
  updateDashChip(action, !!data.ok, data);
  depLoadLogs();
}

function setChip(el, label, level) {
  if (!el) return;
  el.textContent = label;
  el.className = 'chip ' + level;
}

function updateDashChip(action, ok, data) {
  const suffix = ok ? 'PASS' : 'FAIL';
  const level = ok ? 'ok' : 'fail';
  if (action === 'policy') setChip(dashPolicyChip, 'Policy: ' + suffix, level);
  if (action === 'hook-policy') setChip(dashHookChip, 'Hook: ' + suffix, level);
  if (action === 'drift') setChip(dashDriftChip, 'Drift: ' + suffix, level);
  if (action === 'bus-status' || action === 'bus-start' || action === 'bus-stop') {
    setChip(dashBusChip, 'Bus: ' + (ok ? 'RUNNING' : 'STOPPED'), ok ? 'ok' : 'fail');
  }
  if (action === 'gate') setChip(dashGateChip, 'Gate: ' + suffix, level);
  if (action === 'push') setChip(dashPushChip, 'Push: ' + suffix, level);
  if (!ok && data && data.error) {
    appendLive({ source: 'dashboard', action, error: data.error });
  }
}

function setDepStatus(el, parsed) {
  if (!parsed || typeof parsed !== 'object') {
    el.textContent = 'invalid response';
    el.className = 'dep-fail';
    return;
  }
  if (parsed.ok) {
    el.textContent = 'PASS';
    el.className = 'dep-ok';
    return;
  }
  const failed = Array.isArray(parsed.failed_checks) ? parsed.failed_checks.join(', ') : 'unknown';
  el.textContent = 'FAIL: ' + failed;
  el.className = 'dep-fail';
}

function setDepDriftStatus(text) {
  if (!depDriftStatus) return;
  const ok = text === 'PASS';
  depDriftStatus.textContent = text;
  depDriftStatus.className = ok ? 'dep-ok' : 'dep-fail';
}

async function depLoadLogs() {
  const res = await fetch('/api/dependencies/logs');
  const data = await res.json();
  depLogs.textContent = JSON.stringify(data, null, 2);
}

function multiRegister() {
  run('pilot.multi.register', {
    path: document.getElementById('repo-path').value,
    name: document.getElementById('repo-name').value || null,
    group: document.getElementById('repo-group').value || null,
    tags: tags(document.getElementById('repo-tags').value)
  });
}
function multiList() {
  run('pilot.multi.list', { group: document.getElementById('multi-group').value || null, tags: tags(document.getElementById('multi-tags').value) });
}
function multiStatus() {
  run('pilot.multi.status', { group: document.getElementById('multi-group').value || null, tags: tags(document.getElementById('multi-tags').value) });
}
function multiOrder() {
  run('pilot.multi.order', { group: document.getElementById('multi-group').value || null, tags: tags(document.getElementById('multi-tags').value) });
}
function multiDag() {
  run('pilot.multi.dag', {
    group: document.getElementById('multi-group').value || null,
    tags: tags(document.getElementById('multi-tags').value),
    dry_run: true
  }, {
    label: 'DAG',
    chip: multiDagChip,
    buttons: [multiDagBtn],
    runningLabel: 'DAG running...'
  });
}
function multiPrsCreate() {
  run('pilot.multi.prs.create', {
    group: document.getElementById('multi-group').value || null,
    tags: tags(document.getElementById('multi-tags').value),
    dry_run: true,
    head_branch: 'dev',
    base_branch: 'main'
  });
}

function multiApplyPayload(apply) {
  const stageSizeRaw = parseInt(document.getElementById('multi-apply-stage-size').value || '2', 10);
  const stageSize = Number.isFinite(stageSizeRaw) && stageSizeRaw > 0 ? stageSizeRaw : 2;
  return {
    branch: document.getElementById('multi-apply-branch').value || 'feat/pilot-wave13',
    base_branch: document.getElementById('multi-apply-base').value || 'dev',
    pr_base_branch: document.getElementById('multi-apply-pr-base').value || 'main',
    group: document.getElementById('multi-group').value || null,
    tags: tags(document.getElementById('multi-tags').value),
    stage_size: stageSize,
    continue_on_failure: !!document.getElementById('multi-apply-continue').checked,
    apply: !!apply
  };
}

function multiApplyDryRun() {
  const payload = multiApplyPayload(false);
  run('pilot.multi.apply', payload, {
    label: 'Staged Apply',
    chip: multiApplyChip,
    buttons: [multiApplyDryBtn, multiApplyExecBtn],
    runningLabel: 'Running...'
  });
}

function multiApplyExecute() {
  const payload = multiApplyPayload(true);
  run('pilot.multi.apply', payload, {
    label: 'Staged Apply',
    chip: multiApplyChip,
    buttons: [multiApplyDryBtn, multiApplyExecBtn],
    runningLabel: 'Running...'
  });
}

function dashBranchCreate() {
  run('pilot.branch.create', {
    branch: document.getElementById('dash-branch-name').value,
    base_branch: document.getElementById('dash-branch-base').value || 'main',
    group: document.getElementById('dash-branch-group').value || null,
    tags: tags(document.getElementById('dash-branch-tags').value),
    dry_run: true
  });
}

function dashRunPolicy() { depRun('policy'); }
function dashRunHookPolicy() { depRun('hook-policy'); }
function dashRunDrift() { depRun('drift'); }
function dashRunGate() { depRun('gate'); }
function dashRunRepair() { depRun('repair'); }
function dashStartBus() { depRun('bus-start'); }
function dashStopBus() { depRun('bus-stop'); }
function dashBusStatus() { depRun('bus-status'); }
function dashRunPush() { depRun('push'); }

async function dashExportEvidence() {
  const res = await fetch('/api/evidence/export', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({})
  });
  const data = await res.json();
  const text = JSON.stringify(data, null, 2);
  out.textContent = text;
  if (dashStatusOut) dashStatusOut.textContent = text;
  appendLive({ source: 'dashboard', action: 'evidence-export', ok: !!data.ok, path: data.path || '' });
}

function codexPayloadFromUi() {
  const raw = document.getElementById('codex-payload').value.trim();
  if (!raw) return {};
  return JSON.parse(raw);
}

async function codexRun(mode) {
  let payload;
  try {
    payload = codexPayloadFromUi();
  } catch (e) {
    const msg = 'Invalid JSON payload: ' + e.message;
    codexOut.textContent = msg;
    out.textContent = msg;
    return;
  }
  const req = {
    contract_id: document.getElementById('codex-contract-id').value.trim(),
    intent: document.getElementById('codex-intent').value.trim(),
    command: document.getElementById('codex-command').value.trim(),
    payload,
    mode,
    expected_effect: document.getElementById('codex-expected').value.trim(),
    rollback_strategy: document.getElementById('codex-rollback').value.trim(),
    verify_command: document.getElementById('codex-verify').value.trim(),
    reconcile_notes: document.getElementById('codex-reconcile-notes').value.trim()
  };
  const res = await fetch('/api/codex/action', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(req)
  });
  const data = await res.json();
  const text = JSON.stringify(data, null, 2);
  codexOut.textContent = text;
  out.textContent = text;
  if (dashStatusOut) dashStatusOut.textContent = text;
  if (data && data.contract && data.contract.contract_id) {
    latestCodexContractId = data.contract.contract_id;
    document.getElementById('codex-contract-id').value = latestCodexContractId;
  }
  appendLive({ source: 'codex_ui', mode, command: req.command, ok: !!data.ok });
  if (mode === 'execute' || mode === 'reconcile' || mode === 'approve') loadHistory();
  if (mode === 'execute' || mode === 'reconcile' || mode === 'approve' || mode === 'preview') codexLoadContracts();
}

function codexPreview() { codexRun('preview'); }
function codexApprove() {
  if (!document.getElementById('codex-contract-id').value.trim() && latestCodexContractId) {
    document.getElementById('codex-contract-id').value = latestCodexContractId;
  }
  codexRun('approve');
}
function codexExecute() { codexRun('execute'); }
function codexReconcile() {
  if (!document.getElementById('codex-contract-id').value.trim() && latestCodexContractId) {
    document.getElementById('codex-contract-id').value = latestCodexContractId;
  }
  codexRun('reconcile');
}

async function codexLoadContracts() {
  const status = document.getElementById('codex-contract-filter').value.trim();
  const qs = new URLSearchParams({ limit: '100' });
  if (status) qs.set('status', status);
  const res = await fetch('/api/codex/contracts?' + qs.toString());
  const data = await res.json();
  const items = (data && data.contracts) ? data.contracts : [];
  codexContractSelect.innerHTML = '';
  for (const c of items) {
    const opt = document.createElement('option');
    opt.value = c.contract_id;
    opt.textContent = `${c.contract_id} | ${c.status} | ${c.command}`;
    codexContractSelect.appendChild(opt);
  }
  if (items.length > 0) {
    latestCodexContractId = items[0].contract_id;
  }
  codexContractsOut.textContent = JSON.stringify(items, null, 2);
}

async function codexLoadSelectedContract() {
  const id = codexContractSelect.value || document.getElementById('codex-contract-id').value.trim();
  if (!id) {
    codexContractsOut.textContent = 'No contract selected.';
    return;
  }
  const res = await fetch('/api/codex/contract?contract_id=' + encodeURIComponent(id));
  const data = await res.json();
  if (data && data.contract) {
    const c = data.contract;
    document.getElementById('codex-contract-id').value = c.contract_id || '';
    document.getElementById('codex-intent').value = c.intent || '';
    document.getElementById('codex-command').value = c.command || '';
    document.getElementById('codex-payload').value = JSON.stringify(c.payload_original || {}, null, 2);
    document.getElementById('codex-expected').value = c.expected_effect || '';
    document.getElementById('codex-rollback').value = c.rollback_strategy || '';
    document.getElementById('codex-verify').value = c.verify_command || '';
    latestCodexContractId = c.contract_id || latestCodexContractId;
  }
  codexContractsOut.textContent = JSON.stringify(data, null, 2);
}

async function codexRetryFailedContract() {
  await codexLoadSelectedContract();
  await codexRun('approve');
  await codexRun('execute');
}

async function loadHistory() {
  const res = await fetch('/api/history');
  const data = await res.json();
  auditCache = (data && data.events) ? data.events : [];
  renderOperationDetail();
}

function attachStream() {
  streamHandle = new EventSource('/api/stream');
  streamHandle.onopen = () => {
    setBusStatus(true);
  };
  streamHandle.addEventListener('pilot_event', (evt) => {
    if (streamPaused) return;
    try {
      const parsed = JSON.parse(evt.data);
      if (parsed && parsed.source === 'bus_listener' && parsed.error) {
        setBusStatus(false, parsed.error);
      } else {
        setBusStatus(true);
      }
      appendLive(parsed);
    } catch (_) {
      appendLive({ raw: evt.data });
    }
  });
  streamHandle.onerror = () => {
    setBusStatus(false, 'stream disconnected, retrying...');
    appendLive({ source: 'ui', warning: 'stream disconnected, retrying...' });
  };
}

function toggleStream() {
  streamPaused = !streamPaused;
  streamToggleBtn.textContent = streamPaused ? 'Resume Stream' : 'Pause Stream';
  appendLive({ source: 'ui', info: streamPaused ? 'stream paused' : 'stream resumed' });
}

attachStream();
restoreBusStatus();
loadHistory();
oracleLoadReports();
depLoadLogs();
depRun('policy');
depRun('hook-policy');
depRun('drift');
depRun('bus-status');
codexLoadContracts();
setInterval(loadHistory, 30000);
</script>
</body>
</html>"#;
