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
use std::collections::HashSet;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::sync::broadcast;
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
struct ReportPathQuery {
    path: String,
    max_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DependencyActionRequest {
    action: String,
    #[serde(default)]
    json: bool,
}

pub async fn run_ui_server(cfg: UiConfig) -> Result<()> {
    let (event_tx, _) = broadcast::channel(512);
    spawn_bus_telemetry_listener(cfg.bus.clone(), event_tx.clone());
    let state = Arc::new(UiState {
        bus: cfg.bus,
        events: event_tx,
        allow_mutations: cfg.allow_mutations,
        allowed_commands: cfg.allowed_commands,
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/api/command", post(run_command))
        .route("/api/history", get(get_history))
        .route("/api/reports", get(get_reports))
        .route("/api/report", get(get_report_content))
        .route("/api/dependencies/run", post(run_dependency_action))
        .route("/api/dependencies/logs", get(get_dependency_logs))
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
        if is_mutating_command(&req.command) {
            return error_response(
                StatusCode::FORBIDDEN,
                &format!("command '{}' blocked in read-only UI mode", req.command),
            );
        }
        enforce_dry_run(&req.command, &mut req.payload);
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
    let cmd = match (action, req.json) {
        ("policy", true) => "./scripts/verify_toolchain_policy.sh --json",
        ("policy", false) => "./scripts/verify_toolchain_policy.sh",
        ("hook-policy", true) => "./scripts/verify_git_hook_policy.sh --json",
        ("hook-policy", false) => "./scripts/verify_git_hook_policy.sh",
        ("gate", _) => "./scripts/prepush_gate.sh",
        ("repair", _) => "./scripts/repair_lock_182.sh --no-gate",
        _ => return error_response(StatusCode::BAD_REQUEST, "unsupported action"),
    };

    match run_local_script(cmd).await {
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
                "exit_code": status
            }));
            Json(body).into_response()
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
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
            | "pilot.multi.prs.create"
            | "pilot.heal.run"
    )
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
    input, select {
      width: 100%;
      background: #0d1526;
      color: #ebf1ff;
      border: 1px solid #334f7d;
      border-radius: 8px;
      padding: 10px 11px;
      font-size: 0.95rem;
    }
    input::placeholder { color: #7f94c6; }
    input:focus, select:focus {
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
    <button class="tab" data-tab="oracle">Oracle</button>
    <button class="tab" data-tab="heal">Heal</button>
    <button class="tab" data-tab="dependencies">Dependencies</button>
    <button class="tab active" data-tab="branch">Branch</button>
    <button class="tab" data-tab="multi">Multi</button>
    <button class="tab" data-tab="telemetry">Telemetry</button>
  </div>

  <section class="panel" id="oracle">
    <div class="grid">
      <div class="card">
        <h3>Oracle Scan / Query</h3>
        <button class="btn" onclick="oracleScan()">Scan Index</button>
        <input id="oracle-query" placeholder="where is branch sync implemented?" />
        <button class="btn secondary" onclick="oracleQuery()">Run Query</button>
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
          <button class="btn secondary" onclick="healPlan()">Plan Only</button>
          <button class="btn" onclick="healRun()">Run Heal</button>
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
        </div>
        <div class="row">
          <button class="btn secondary" onclick="depRun('policy')">Policy Check</button>
          <button class="btn secondary" onclick="depRun('hook-policy')">Hook Policy</button>
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

  <section class="panel active" id="branch">
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
        <h3>List / Status / Order / PR Plan</h3>
        <input id="multi-group" placeholder="core" />
        <input id="multi-tags" placeholder="apply-pilot,wave7" />
        <div class="row">
          <button class="btn secondary" onclick="multiList()">List</button>
          <button class="btn secondary" onclick="multiStatus()">Status</button>
          <button class="btn secondary" onclick="multiOrder()">Order</button>
        </div>
        <button class="btn" onclick="multiPrsCreate()">PR Plan (Dry Run)</button>
      </div>
    </div>
  </section>

  <section class="panel" id="telemetry">
    <div class="grid">
      <div class="card">
        <h3>Live Event Stream</h3>
        <div class="row">
          <button id="stream-toggle" class="btn secondary" onclick="toggleStream()">Pause Stream</button>
          <button class="btn secondary" onclick="clearLive()">Clear</button>
        </div>
        <pre id="live-stream">[]</pre>
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
    </div>
  </section>

  <div class="status">
    <div class="card">
      <h3>Response</h3>
      <pre id="out">ready</pre>
    </div>
    <div class="card">
      <h3>Operation Detail</h3>
      <div id="op-detail-meta" class="muted">Select a timeline item</div>
      <div id="op-detail-artifact" class="muted"></div>
      <pre id="op-detail">[]</pre>
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
const depLogs = document.getElementById('dep-logs');
const depPolicyStatus = document.getElementById('dep-policy-status');
const depHookStatus = document.getElementById('dep-hook-status');
const timelineState = new Map();
let selectedOperationId = null;
let auditCache = [];
let streamPaused = false;
let streamHandle = null;

for (const btn of document.querySelectorAll('.tab')) {
  btn.addEventListener('click', () => {
    for (const t of document.querySelectorAll('.tab')) t.classList.remove('active');
    for (const p of document.querySelectorAll('.panel')) p.classList.remove('active');
    btn.classList.add('active');
    document.getElementById(btn.dataset.tab).classList.add('active');
  });
}

function tags(v) { return v.split(',').map(s => s.trim()).filter(Boolean); }
async function run(command, payload) {
  payload.schema_version = 1;
  const res = await fetch('/api/command', {
    method: 'POST', headers: {'content-type':'application/json'},
    body: JSON.stringify({ command, payload })
  });
  const data = await res.json();
  out.textContent = JSON.stringify(data, null, 2);
  loadHistory();
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
  ingestTimeline(eventObj);
}

function clearLive() {
  liveStream.textContent = '[]';
}

function setBusStatus(connected, note) {
  busStatusChip.textContent = connected ? 'CONNECTED' : 'DISCONNECTED';
  busStatusChip.classList.toggle('connected', connected);
  busStatusChip.classList.toggle('disconnected', !connected);
  if (note) {
    opDetailMeta.textContent = note;
  }
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
  run('pilot.oracle.scan', {});
}

function oracleQuery() {
  run('pilot.oracle.query', {
    query: document.getElementById('oracle-query').value,
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
  run('pilot.heal.run', healPayload(true));
}

function healRun() {
  run('pilot.heal.run', healPayload(false));
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
  const res = await fetch('/api/dependencies/run', {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify({ action, json: isJsonAction })
  });
  const data = await res.json();
  if (isJsonAction) {
    try {
      const parsed = JSON.parse(data.stdout || '{}');
      if (action === 'policy') setDepStatus(depPolicyStatus, parsed);
      if (action === 'hook-policy') setDepStatus(depHookStatus, parsed);
    } catch (_) {}
  }
  depActionOut.textContent = JSON.stringify(data, null, 2);
  depLoadLogs();
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
function multiPrsCreate() {
  run('pilot.multi.prs.create', {
    group: document.getElementById('multi-group').value || null,
    tags: tags(document.getElementById('multi-tags').value),
    dry_run: true,
    head_branch: 'dev',
    base_branch: 'main'
  });
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
loadHistory();
oracleLoadReports();
depLoadLogs();
setInterval(loadHistory, 30000);
</script>
</body>
</html>"#;
