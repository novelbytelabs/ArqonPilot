use crate::bus::{send_command_once, BusBridgeConfig};
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use miette::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct UiConfig {
    pub host: String,
    pub port: u16,
    pub bus: BusBridgeConfig,
}

#[derive(Clone)]
struct UiState {
    bus: BusBridgeConfig,
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

pub async fn run_ui_server(cfg: UiConfig) -> Result<()> {
    let state = Arc::new(UiState { bus: cfg.bus });

    let app = Router::new()
        .route("/", get(index))
        .route("/api/command", post(run_command))
        .route("/api/history", get(get_history))
        .with_state(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    println!("Pilot UI listening at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
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

    match send_command_once(&state.bus, &req.command, req.payload).await {
        Ok(response) => Json(UiCommandResponse { ok: true, response }).into_response(),
        Err(err) => error_response(StatusCode::BAD_GATEWAY, &err.to_string()),
    }
}

async fn get_history() -> Response {
    match read_recent_audit_events(200) {
        Ok(items) => Json(json!({"ok": true, "events": items})).into_response(),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    }
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

fn error_response(status: StatusCode, message: &str) -> Response {
    let mut response = Json(json!({"ok": false, "error": message})).into_response();
    *response.status_mut() = status;
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

const INDEX_HTML: &str = r#"<!doctype html>
<html lang=\"en\">
<head>
  <meta charset=\"UTF-8\" />
  <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\" />
  <title>Pilot Control Panel</title>
  <style>
    :root { color-scheme: dark; }
    body { font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif; margin: 0; background: #0c1220; color: #e6ebff; }
    .wrap { max-width: 1100px; margin: 0 auto; padding: 24px; }
    h1 { margin: 0 0 16px; }
    .tabs { display: flex; gap: 8px; margin-bottom: 16px; }
    button.tab { background: #1b2540; border: 1px solid #33466f; color: #dce5ff; padding: 10px 14px; border-radius: 8px; cursor: pointer; }
    button.tab.active { background: #3a4f8b; }
    .panel { display: none; background: #111a2c; border: 1px solid #2b3a62; border-radius: 12px; padding: 16px; }
    .panel.active { display: block; }
    .grid { display: grid; gap: 12px; grid-template-columns: repeat(2, minmax(0,1fr)); }
    .card { background: #151f34; border: 1px solid #2b3a62; border-radius: 10px; padding: 12px; }
    input, textarea, select { width: 100%; box-sizing: border-box; background: #0f1728; color: #e5ecff; border: 1px solid #32466f; border-radius: 8px; padding: 10px; }
    .row { display: flex; gap: 8px; }
    .btn { background: #506fd3; border: none; color: white; border-radius: 8px; padding: 10px 14px; cursor: pointer; }
    pre { background: #09101d; border: 1px solid #2a3f6d; border-radius: 8px; padding: 12px; max-height: 320px; overflow: auto; }
    .muted { color: #9cb0df; }
    @media (max-width: 900px) { .grid { grid-template-columns: 1fr; } }
  </style>
</head>
<body>
<div class=\"wrap\">
  <h1>Arqon Pilot Control Panel</h1>
  <p class=\"muted\">Branch + Multi + Telemetry over ArqonBus (`pilot serve` required)</p>

  <div class=\"tabs\">
    <button class=\"tab active\" data-tab=\"branch\">Branch</button>
    <button class=\"tab\" data-tab=\"multi\">Multi</button>
    <button class=\"tab\" data-tab=\"telemetry\">Telemetry</button>
  </div>

  <section class=\"panel active\" id=\"branch\">
    <div class=\"grid\">
      <div class=\"card\">
        <h3>Create Branch</h3>
        <input id=\"branch-name\" placeholder=\"feat/pilot-wave7\" />
        <input id=\"branch-base\" placeholder=\"main\" value=\"main\" />
        <input id=\"branch-group\" placeholder=\"core\" />
        <input id=\"branch-tags\" placeholder=\"apply-pilot,wave7\" />
        <button class=\"btn\" onclick=\"branchCreate()\">Run</button>
      </div>
      <div class=\"card\">
        <h3>Sync / Prune / Status</h3>
        <input id=\"sync-branch\" placeholder=\"dev\" value=\"dev\" />
        <input id=\"sync-base\" placeholder=\"main\" value=\"main\" />
        <div class=\"row\">
          <button class=\"btn\" onclick=\"branchSync()\">Sync</button>
          <button class=\"btn\" onclick=\"branchPrune()\">Prune</button>
          <button class=\"btn\" onclick=\"branchStatus()\">Status</button>
        </div>
      </div>
    </div>
  </section>

  <section class=\"panel\" id=\"multi\">
    <div class=\"grid\">
      <div class=\"card\">
        <h3>Register Repo</h3>
        <input id=\"repo-path\" placeholder=\"/path/to/repo\" />
        <input id=\"repo-name\" placeholder=\"ArqonContinuum\" />
        <input id=\"repo-group\" placeholder=\"core\" />
        <input id=\"repo-tags\" placeholder=\"apply-pilot,wave7\" />
        <button class=\"btn\" onclick=\"multiRegister()\">Register</button>
      </div>
      <div class=\"card\">
        <h3>List / Status / Order / PR Plan</h3>
        <input id=\"multi-group\" placeholder=\"core\" />
        <input id=\"multi-tags\" placeholder=\"apply-pilot,wave7\" />
        <div class=\"row\">
          <button class=\"btn\" onclick=\"multiList()\">List</button>
          <button class=\"btn\" onclick=\"multiStatus()\">Status</button>
          <button class=\"btn\" onclick=\"multiOrder()\">Order</button>
        </div>
        <button class=\"btn\" onclick=\"multiPrsCreate()\">PR Plan (Dry Run)</button>
      </div>
    </div>
  </section>

  <section class=\"panel\" id=\"telemetry\">
    <div class=\"card\">
      <h3>Recent Audit Events</h3>
      <button class=\"btn\" onclick=\"loadHistory()\">Refresh</button>
      <pre id=\"history\">[]</pre>
    </div>
  </section>

  <h3>Response</h3>
  <pre id=\"out\">ready</pre>
</div>
<script>
const out = document.getElementById('out');
const history = document.getElementById('history');

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
  history.textContent = JSON.stringify(data, null, 2);
}

loadHistory();
setInterval(loadHistory, 5000);
</script>
</body>
</html>"#;
