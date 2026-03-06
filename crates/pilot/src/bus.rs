use futures_util::{SinkExt, StreamExt};
use miette::{IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const CONTRACT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct BusBridgeConfig {
    pub ws_url: String,
    pub room: String,
    pub channel: String,
    pub telemetry_channel: String,
    pub jwt_env: String,
    pub once: bool,
}

#[derive(Debug, Clone, Serialize)]
struct OperationEvent {
    schema_version: u32,
    operation_id: String,
    event: String,
    command: String,
    success: Option<bool>,
    summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopePayload {
    schema_version: u32,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BranchCreatePayload {
    schema_version: u32,
    branch: String,
    #[serde(default)]
    base_branch: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    dry_run: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BranchSyncPayload {
    schema_version: u32,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    base_branch: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    dry_run: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BranchPrunePayload {
    schema_version: u32,
    #[serde(default)]
    base_branch: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    dry_run: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MultiRegisterPayload {
    schema_version: u32,
    path: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MultiPrsCreatePayload {
    schema_version: u32,
    #[serde(default)]
    head_branch: Option<String>,
    #[serde(default)]
    base_branch: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    dry_run: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MultiDagPayload {
    schema_version: u32,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    dry_run: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MultiApplyPayload {
    schema_version: u32,
    branch: String,
    #[serde(default)]
    base_branch: Option<String>,
    #[serde(default)]
    pr_base_branch: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    stage_size: Option<usize>,
    #[serde(default)]
    continue_on_failure: Option<bool>,
    #[serde(default)]
    pr_output: Option<String>,
    #[serde(default)]
    apply: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleScanPayload {
    schema_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleQueryPayload {
    schema_version: u32,
    query: String,
    #[serde(default)]
    cli: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct HealRunPayload {
    schema_version: u32,
    #[serde(default)]
    log_file: Option<String>,
    #[serde(default)]
    max_attempts: Option<u32>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    verbose: Option<bool>,
    #[serde(default)]
    plan_only: Option<bool>,
    #[serde(default)]
    max_files: Option<usize>,
}

pub async fn run_bridge(cfg: &BusBridgeConfig) -> Result<()> {
    let (ws_stream, _) = connect_async(&cfg.ws_url)
        .await
        .into_diagnostic()
        .map_err(|e| miette::miette!("Bus connect failed {}: {}", cfg.ws_url, e))?;
    let (mut writer, mut reader) = ws_stream.split();

    if let Ok(token) = env::var(&cfg.jwt_env) {
        let auth = json!({
            "type": "command",
            "command": "authenticate",
            "args": {"token": token},
            "room": cfg.room,
            "channel": cfg.channel,
        });
        writer
            .send(Message::Text(auth.to_string()))
            .await
            .into_diagnostic()?;
    }

    let join = json!({
        "type": "command",
        "command": "join_channel",
        "args": {"channel_id": cfg.channel},
        "room": cfg.room,
        "channel": cfg.channel,
    });
    writer
        .send(Message::Text(join.to_string()))
        .await
        .into_diagnostic()?;

    let ready = json!({
        "type": "telemetry",
        "payload": {
            "schema_version": CONTRACT_SCHEMA_VERSION,
            "event_type": "pilot.bridge.ready",
            "room": cfg.room,
            "channel": cfg.channel,
            "telemetry_channel": cfg.telemetry_channel
        },
        "room": cfg.room,
        "channel": cfg.telemetry_channel,
    });
    writer
        .send(Message::Text(ready.to_string()))
        .await
        .into_diagnostic()?;

    while let Some(msg) = reader.next().await {
        let msg = msg.into_diagnostic()?;
        let Message::Text(text) = msg else {
            continue;
        };

        let envelope: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if envelope.get("type").and_then(Value::as_str).unwrap_or("") != "command" {
            continue;
        }

        let Some(command) = extract_command_name(&envelope) else {
            continue;
        };
        if !command.starts_with("pilot.") {
            continue;
        }

        let request_id = envelope
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let payload = extract_payload(&envelope);
        let operation_id = make_operation_id();

        let started = OperationEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            operation_id: operation_id.clone(),
            event: "pilot.op.started".to_string(),
            command: command.clone(),
            success: None,
            summary: None,
        };
        emit_event(
            &mut writer,
            &cfg.room,
            &cfg.telemetry_channel,
            "pilot.op.started",
            &started,
        )
        .await?;

        let response = match run_pilot_subcommand(&command, payload) {
            Ok(report) => {
                let completed = OperationEvent {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    operation_id,
                    event: "pilot.op.completed".to_string(),
                    command: command.clone(),
                    success: Some(true),
                    summary: Some(
                        report
                            .get("summary")
                            .and_then(Value::as_str)
                            .unwrap_or("ok")
                            .to_string(),
                    ),
                };
                emit_event(
                    &mut writer,
                    &cfg.room,
                    &cfg.telemetry_channel,
                    "pilot.op.completed",
                    &completed,
                )
                .await?;
                json!({
                    "type": "response",
                    "request_id": request_id,
                    "status": "success",
                    "payload": report,
                    "room": cfg.room,
                    "channel": cfg.channel,
                })
            }
            Err(err) => {
                let failed = OperationEvent {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    operation_id,
                    event: "pilot.op.failed".to_string(),
                    command: command.clone(),
                    success: Some(false),
                    summary: Some(err.clone()),
                };
                emit_event(
                    &mut writer,
                    &cfg.room,
                    &cfg.telemetry_channel,
                    "pilot.op.failed",
                    &failed,
                )
                .await?;
                json!({
                    "type": "response",
                    "request_id": request_id,
                    "status": "error",
                    "error": err,
                    "room": cfg.room,
                    "channel": cfg.channel,
                })
            }
        };

        writer
            .send(Message::Text(response.to_string()))
            .await
            .into_diagnostic()?;

        if cfg.once {
            break;
        }
    }

    Ok(())
}

pub async fn send_command_once(
    cfg: &BusBridgeConfig,
    command: &str,
    payload: Value,
) -> Result<Value> {
    let (ws_stream, _) = connect_async(&cfg.ws_url).await.into_diagnostic()?;
    let (mut writer, mut reader) = ws_stream.split();

    if let Ok(token) = env::var(&cfg.jwt_env) {
        let auth = json!({
            "type": "command",
            "command": "authenticate",
            "args": {"token": token},
            "room": cfg.room,
            "channel": cfg.channel,
        });
        writer
            .send(Message::Text(auth.to_string()))
            .await
            .into_diagnostic()?;
    }

    let request_id = make_operation_id();
    let envelope = json!({
        "id": request_id,
        "type": "command",
        "command": command,
        "args": payload,
        "room": cfg.room,
        "channel": cfg.channel,
    });

    writer
        .send(Message::Text(envelope.to_string()))
        .await
        .into_diagnostic()?;

    let wait_for_response = async {
        while let Some(msg) = reader.next().await {
            let msg = msg.into_diagnostic()?;
            let Message::Text(text) = msg else {
                continue;
            };
            let value: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if is_matching_command_response(&value, command, &request_id) {
                return Ok(value);
            }
        }

        Err(miette::miette!(
            "No command response received for {}",
            command
        ))
    };

    match tokio::time::timeout(Duration::from_secs(20), wait_for_response).await {
        Ok(v) => v,
        Err(_) => Err(miette::miette!(
            "Timed out waiting for command response for {}",
            command
        )),
    }
}

fn is_matching_command_response(value: &Value, command: &str, request_id: &str) -> bool {
    let msg_type = value.get("type").and_then(Value::as_str).unwrap_or("");
    let id_matches = value.get("reply_to").and_then(Value::as_str) == Some(request_id)
        || value.get("request_id").and_then(Value::as_str) == Some(request_id)
        || value.get("id").and_then(Value::as_str) == Some(request_id);

    if !id_matches {
        return false;
    }

    // Newer ArqonBus responses may use `type=response` and omit `command`.
    if msg_type == "response" {
        return true;
    }

    if msg_type != "command_response" {
        return false;
    }

    match value.get("command").and_then(Value::as_str) {
        Some(cmd) => cmd == command,
        None => true,
    }
}

async fn emit_event(
    writer: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    room: &str,
    channel: &str,
    event_type: &str,
    payload: &impl Serialize,
) -> Result<()> {
    let payload_value = serde_json::to_value(payload).into_diagnostic()?;
    let msg = json!({
        "type": "telemetry",
        "payload": {
            "event_type": event_type,
            "data": payload_value,
        },
        "room": room,
        "channel": channel,
    });
    writer
        .send(Message::Text(msg.to_string()))
        .await
        .into_diagnostic()?;
    Ok(())
}

fn extract_command_name(v: &Value) -> Option<String> {
    if let Some(cmd) = v.get("command").and_then(Value::as_str) {
        return Some(cmd.to_string());
    }
    v.get("data")
        .and_then(|d| d.get("command"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn extract_payload(v: &Value) -> Value {
    if let Some(args) = v.get("args") {
        return args.clone();
    }
    if let Some(data) = v.get("data") {
        return data.clone();
    }
    if let Some(payload) = v.get("payload") {
        return payload.clone();
    }
    json!({})
}

fn run_pilot_subcommand(command: &str, payload: Value) -> std::result::Result<Value, String> {
    let args = map_bus_command_to_args(command, &payload)?;
    let exe = resolve_pilot_executable()?;

    let output = Command::new(exe)
        .args(args)
        .arg("--report-json")
        .output()
        .map_err(|e| format!("Failed to spawn pilot command: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_line = stdout
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .ok_or_else(|| "pilot report-json output missing".to_string())?;

    serde_json::from_str(json_line).map_err(|e| format!("Invalid report-json payload: {}", e))
}

pub fn run_pilot_subcommand_local(command: &str, payload: Value) -> Result<Value> {
    run_pilot_subcommand(command, payload).map_err(|e| miette::miette!(e))
}

fn resolve_pilot_executable() -> std::result::Result<PathBuf, String> {
    // Primary path: the currently running pilot binary.
    if let Ok(exe) = std::env::current_exe() {
        if exe.exists() {
            return Ok(exe);
        }
    }

    // Fallback for dev sessions where `cargo clean` invalidated the running path.
    if let Ok(cwd) = std::env::current_dir() {
        let debug_pilot = cwd.join("target").join("debug").join("pilot");
        if debug_pilot.exists() {
            return Ok(debug_pilot);
        }
    }

    // Final fallback: rely on PATH resolution.
    Ok(PathBuf::from("pilot"))
}

fn map_bus_command_to_args(
    command: &str,
    payload: &Value,
) -> std::result::Result<Vec<String>, String> {
    let mut args = Vec::new();

    match command {
        "pilot.branch.create" => {
            let req: BranchCreatePayload = parse_contract(command, payload)?;
            args.extend(["branch".to_string(), "create".to_string(), req.branch]);
            if let Some(base) = req.base_branch {
                args.extend(["--base-branch".to_string(), base]);
            }
            apply_scope(&mut args, req.group, req.tags);
            if req.dry_run.unwrap_or(true) {
                args.push("--dry-run".to_string());
            }
        }
        "pilot.branch.sync" => {
            let req: BranchSyncPayload = parse_contract(command, payload)?;
            args.extend(["branch".to_string(), "sync".to_string()]);
            if let Some(branch) = req.branch {
                args.extend(["--branch".to_string(), branch]);
            }
            if let Some(base) = req.base_branch {
                args.extend(["--base-branch".to_string(), base]);
            }
            apply_scope(&mut args, req.group, req.tags);
            if req.dry_run.unwrap_or(true) {
                args.push("--dry-run".to_string());
            }
        }
        "pilot.branch.status" => {
            let req: ScopePayload = parse_contract(command, payload)?;
            args.extend(["branch".to_string(), "status".to_string()]);
            apply_scope(&mut args, req.group, req.tags);
        }
        "pilot.branch.prune" => {
            let req: BranchPrunePayload = parse_contract(command, payload)?;
            args.extend(["branch".to_string(), "prune".to_string()]);
            if let Some(base) = req.base_branch {
                args.extend(["--base-branch".to_string(), base]);
            }
            apply_scope(&mut args, req.group, req.tags);
            if req.dry_run.unwrap_or(true) {
                args.push("--dry-run".to_string());
            }
        }
        "pilot.multi.register" => {
            let req: MultiRegisterPayload = parse_contract(command, payload)?;
            args.extend([
                "multi".to_string(),
                "register".to_string(),
                "--path".to_string(),
                req.path,
            ]);
            if let Some(name) = req.name {
                args.extend(["--name".to_string(), name]);
            }
            apply_scope(&mut args, req.group, req.tags);
        }
        "pilot.multi.list" => {
            let req: ScopePayload = parse_contract(command, payload)?;
            args.extend(["multi".to_string(), "list".to_string()]);
            apply_scope(&mut args, req.group, req.tags);
        }
        "pilot.multi.status" => {
            let req: ScopePayload = parse_contract(command, payload)?;
            args.extend(["multi".to_string(), "status".to_string()]);
            apply_scope(&mut args, req.group, req.tags);
        }
        "pilot.multi.order" => {
            let req: ScopePayload = parse_contract(command, payload)?;
            args.extend(["multi".to_string(), "order".to_string()]);
            apply_scope(&mut args, req.group, req.tags);
        }
        "pilot.multi.prs.create" => {
            let req: MultiPrsCreatePayload = parse_contract(command, payload)?;
            args.extend(["multi".to_string(), "prs".to_string(), "create".to_string()]);
            if let Some(head) = req.head_branch {
                args.extend(["--head-branch".to_string(), head]);
            }
            if let Some(base) = req.base_branch {
                args.extend(["--base-branch".to_string(), base]);
            }
            apply_scope(&mut args, req.group, req.tags);
            if let Some(output) = req.output {
                args.extend(["--output".to_string(), output]);
            }
            if req.dry_run.unwrap_or(true) {
                args.push("--dry-run".to_string());
            }
        }
        "pilot.multi.dag" => {
            let req: MultiDagPayload = parse_contract(command, payload)?;
            args.extend(["multi".to_string(), "dag".to_string()]);
            apply_scope(&mut args, req.group, req.tags);
            if let Some(output) = req.output {
                args.extend(["--output".to_string(), output]);
            }
            if req.dry_run.unwrap_or(false) {
                args.push("--dry-run".to_string());
            }
        }
        "pilot.multi.apply" => {
            let req: MultiApplyPayload = parse_contract(command, payload)?;
            args.extend([
                "multi".to_string(),
                "apply".to_string(),
                "--branch".to_string(),
                req.branch,
            ]);
            if let Some(base) = req.base_branch {
                args.extend(["--base-branch".to_string(), base]);
            }
            if let Some(base) = req.pr_base_branch {
                args.extend(["--pr-base-branch".to_string(), base]);
            }
            apply_scope(&mut args, req.group, req.tags);
            if let Some(stage_size) = req.stage_size {
                args.extend(["--stage-size".to_string(), stage_size.to_string()]);
            }
            if req.continue_on_failure.unwrap_or(false) {
                args.push("--continue-on-failure".to_string());
            }
            if let Some(out) = req.pr_output {
                args.extend(["--pr-output".to_string(), out]);
            }
            if req.apply.unwrap_or(false) {
                args.push("--apply".to_string());
            }
        }
        "pilot.oracle.scan" => {
            let _req: OracleScanPayload = parse_contract(command, payload)?;
            args.extend(["oracle".to_string(), "scan".to_string()]);
        }
        "pilot.oracle.query" => {
            let req: OracleQueryPayload = parse_contract(command, payload)?;
            args.extend([
                "oracle".to_string(),
                "query".to_string(),
                "--query".to_string(),
                req.query,
            ]);
            if req.cli.unwrap_or(true) {
                args.push("--cli".to_string());
            }
        }
        "pilot.heal.run" => {
            let req: HealRunPayload = parse_contract(command, payload)?;
            args.push("heal".to_string());
            if let Some(log) = req.log_file {
                args.extend(["--log-file".to_string(), log]);
            }
            if let Some(attempts) = req.max_attempts {
                args.extend(["--max-attempts".to_string(), attempts.to_string()]);
            }
            if let Some(target) = req.target {
                args.extend(["--target".to_string(), target]);
            }
            if req.verbose.unwrap_or(false) {
                args.push("--verbose".to_string());
            }
            if req.plan_only.unwrap_or(false) {
                args.push("--plan-only".to_string());
            }
            if let Some(max_files) = req.max_files {
                args.extend(["--max-files".to_string(), max_files.to_string()]);
            }
        }
        other => {
            return Err(format!("Unsupported pilot bus command: {}", other));
        }
    }

    Ok(args)
}

fn parse_contract<T>(command: &str, payload: &Value) -> std::result::Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let parsed: T = serde_json::from_value(payload.clone())
        .map_err(|e| format!("Invalid {} payload: {}", command, e))?;

    let schema = payload
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("Invalid {} payload: missing schema_version", command))?;
    if schema as u32 != CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "Invalid {} payload: schema_version={} (expected {})",
            command, schema, CONTRACT_SCHEMA_VERSION
        ));
    }

    Ok(parsed)
}

fn apply_scope(args: &mut Vec<String>, group: Option<String>, tags: Vec<String>) {
    if let Some(group) = group {
        args.extend(["--group".to_string(), group]);
    }
    for tag in tags {
        args.extend(["--tag".to_string(), tag]);
    }
}

fn make_operation_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("op-{}", now)
}

pub fn default_ws_url() -> String {
    env::var("ARQONBUS_WS_URL").unwrap_or_else(|_| "ws://127.0.0.1:9100".to_string())
}

pub fn default_jwt_env() -> String {
    "ARQONBUS_AUTH_JWT".to_string()
}

pub fn default_room() -> String {
    env::var("PILOT_BUS_ROOM").unwrap_or_else(|_| "pilot".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_legacy_command_response() {
        let v = json!({
            "type": "command_response",
            "command": "pilot.multi.register",
            "reply_to": "req-1"
        });
        assert!(is_matching_command_response(
            &v,
            "pilot.multi.register",
            "req-1"
        ));
    }

    #[test]
    fn test_matches_new_response_shape() {
        let v = json!({
            "type": "response",
            "request_id": "req-2",
            "status": "success",
            "payload": {"message":"ok"}
        });
        assert!(is_matching_command_response(
            &v,
            "pilot.multi.register",
            "req-2"
        ));
    }

    #[test]
    fn test_rejects_mismatched_request() {
        let v = json!({
            "type": "response",
            "request_id": "req-other",
            "status": "success"
        });
        assert!(!is_matching_command_response(
            &v,
            "pilot.multi.register",
            "req-3"
        ));
    }

    #[test]
    fn map_branch_create_defaults_to_dry_run() {
        let payload = json!({"schema_version": 1, "branch": "feat/test"});
        let args = map_bus_command_to_args("pilot.branch.create", &payload).unwrap();
        assert_eq!(args[0], "branch");
        assert!(args.contains(&"--dry-run".to_string()));
    }

    #[test]
    fn map_multi_status_with_filters() {
        let payload =
            json!({"schema_version": 1, "group": "core", "tags": ["apply-pilot", "wave7"]});
        let args = map_bus_command_to_args("pilot.multi.status", &payload).unwrap();
        assert_eq!(
            args,
            vec![
                "multi",
                "status",
                "--group",
                "core",
                "--tag",
                "apply-pilot",
                "--tag",
                "wave7"
            ]
        );
    }

    #[test]
    fn map_branch_prune_supported() {
        let payload = json!({"schema_version": 1, "base_branch": "dev", "dry_run": false});
        let args = map_bus_command_to_args("pilot.branch.prune", &payload).unwrap();
        assert!(args.starts_with(&["branch".to_string(), "prune".to_string()]));
        assert!(!args.contains(&"--dry-run".to_string()));
    }

    #[test]
    fn map_multi_prs_create_supported() {
        let payload = json!({
            "schema_version": 1,
            "group": "core",
            "head_branch": "feat/pilot-wave7",
            "base_branch": "dev",
            "dry_run": true
        });
        let args = map_bus_command_to_args("pilot.multi.prs.create", &payload).unwrap();
        assert_eq!(args[0], "multi");
        assert!(args.contains(&"prs".to_string()));
        assert!(args.contains(&"--dry-run".to_string()));
    }

    #[test]
    fn map_multi_dag_supported() {
        let payload = json!({
            "schema_version": 1,
            "group": "core",
            "tags": ["apply-pilot"],
            "dry_run": true
        });
        let args = map_bus_command_to_args("pilot.multi.dag", &payload).unwrap();
        assert_eq!(args[0], "multi");
        assert!(args.contains(&"dag".to_string()));
        assert!(args.contains(&"--dry-run".to_string()));
    }

    #[test]
    fn map_multi_apply_supported() {
        let payload = json!({
            "schema_version": 1,
            "branch": "feat/pilot-wave13",
            "base_branch": "dev",
            "pr_base_branch": "main",
            "group": "core",
            "stage_size": 2,
            "continue_on_failure": true,
            "apply": false
        });
        let args = map_bus_command_to_args("pilot.multi.apply", &payload).unwrap();
        assert_eq!(args[0], "multi");
        assert!(args.contains(&"apply".to_string()));
        assert!(args.contains(&"--branch".to_string()));
        assert!(args.contains(&"--stage-size".to_string()));
        assert!(args.contains(&"--continue-on-failure".to_string()));
        assert!(!args.contains(&"--apply".to_string()));
    }

    #[test]
    fn missing_schema_version_is_rejected() {
        let payload = json!({"branch": "feat/test"});
        let err = map_bus_command_to_args("pilot.branch.create", &payload).unwrap_err();
        assert!(err.contains("schema_version"));
    }

    #[test]
    fn wrong_schema_version_is_rejected() {
        let payload = json!({"schema_version": 2, "branch": "feat/test"});
        let err = map_bus_command_to_args("pilot.branch.create", &payload).unwrap_err();
        assert!(err.contains("expected 1"));
    }

    #[test]
    fn unsupported_command_is_rejected() {
        let payload = json!({"schema_version": 1});
        let err = map_bus_command_to_args("pilot.unknown.command", &payload).unwrap_err();
        assert!(err.contains("Unsupported"));
    }

    #[test]
    fn map_oracle_query_supported() {
        let payload = json!({
            "schema_version": 1,
            "query": "where does branch sync live?",
            "cli": true
        });
        let args = map_bus_command_to_args("pilot.oracle.query", &payload).unwrap();
        assert_eq!(args[0], "oracle");
        assert_eq!(args[1], "query");
        assert!(args.contains(&"--query".to_string()));
        assert!(args.contains(&"--cli".to_string()));
    }

    #[test]
    fn map_heal_run_supported() {
        let payload = json!({
            "schema_version": 1,
            "log_file": "test_output.json",
            "max_attempts": 2,
            "target": "crates/pilot/src/main.rs",
            "verbose": true,
            "plan_only": true,
            "max_files": 6
        });
        let args = map_bus_command_to_args("pilot.heal.run", &payload).unwrap();
        assert_eq!(args[0], "heal");
        assert!(args.contains(&"--log-file".to_string()));
        assert!(args.contains(&"--plan-only".to_string()));
        assert!(args.contains(&"--max-files".to_string()));
    }
}
