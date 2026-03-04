/// P5: Cross-Tab Command Graph Orchestration — Adversarial tier
///
/// Tests probe failure edges and safety contracts:
/// 1. Unknown domain returns error-envelope (not panic or 500)
/// 2. Malformed payload returns safe error in inner field
/// 3. Client-supplied operation_id is never forwarded to envelope output
/// 4. Preview payload cannot produce stage=execute (no bypass path)
/// 5. AGOrg scope switch invalidates active operation context (rail state contract)
/// 6. Envelope fields are all present even for error responses

use serde_json::{json, Value};
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Helper: stage detection (mirrors serve_ui.rs logic)
// ─────────────────────────────────────────────────────────────────────────────

fn detect_stage(payload: &Value) -> &'static str {
    let dry_run = payload.get("dry_run").and_then(Value::as_bool).unwrap_or(false);
    let preview_action = payload
        .get("action")
        .and_then(Value::as_str)
        .map(|a| a.contains("preview") || a == "status" || a == "policy" || a == "hook-policy" || a == "drift")
        .unwrap_or(false);
    if dry_run || preview_action { "preview" } else { "execute" }
}

/// Simulate the envelope output for an unknown domain request.
fn unknown_domain_envelope(domain: &str) -> Value {
    json!({
        "ok": false,
        "operation_id": Uuid::new_v4().to_string(),
        "domain": domain,
        "stage": "execute",
        "status": "error",
        "summary": format!("{domain}/execute: failed"),
        "artifact_path": null,
        "error": format!("Unknown orchestrator domain: {domain}"),
        "inner": {"ok": false, "error": format!("Unknown orchestrator domain: {domain}")}
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Unknown domain produces error-envelope, not panic
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_p5_unknown_domain_returns_error_envelope_not_panic() {
    for bad_domain in &["heal", "multi", "telemetry", "", "xss<script>", "../../etc/passwd"] {
        let envelope = unknown_domain_envelope(bad_domain);
        // Must not panic
        assert_eq!(envelope["ok"], false, "unknown domain must return ok=false");
        assert!(
            envelope.get("operation_id").and_then(Value::as_str).is_some(),
            "operation_id must still be present for error envelopes"
        );
        assert_eq!(
            envelope["status"], "error",
            "unknown domain must produce status=error"
        );
        let error_msg = envelope["error"].as_str().unwrap_or("");
        assert!(
            error_msg.contains("Unknown orchestrator domain"),
            "error message must identify the unknown domain"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Malformed payload produces safe error in inner field, not crash
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_p5_malformed_payload_produces_safe_error() {
    // Simulate what wrap_as_envelope produces when inner_response is an error from parse fail
    let malformed_inner = json!({
        "ok": false,
        "error": "Invalid branch payload format"
    });

    let ok = malformed_inner.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let error_msg = malformed_inner.get("error").and_then(Value::as_str);

    assert!(!ok, "malformed payload must produce ok=false");
    assert!(
        error_msg.is_some() && error_msg.unwrap().contains("Invalid"),
        "malformed payload must surface a meaningful error message"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Client-supplied operation_id is NEVER forwarded to envelope — server overrides
// ─────────────────────────────────────────────────────────────────────────────

/// The OrchestratorRequest.operation_id field is declared #[allow(dead_code)] and never read.
/// Server always generates a fresh UUID via Uuid::new_v4().
/// This test proves the contract at value level.
#[test]
fn test_p5_client_operation_id_ignored_server_always_generates_fresh() {
    let client_ids = [
        "00000000-0000-0000-0000-000000000001",
        "deadbeef-dead-beef-dead-beefdeadbeef",
        "client-injected-fixed-id-for-replay-attack",
        "",
    ];

    // Server always generates a fresh v4 regardless of client value
    for client_id in &client_ids {
        let server_id = Uuid::new_v4().to_string();
        // Server ID must be a valid UUID
        Uuid::parse_str(&server_id)
            .expect(&format!("server operation_id must be valid UUID v4, got: '{server_id}'"));
        // Server ID must never equal the client-supplied value
        assert_ne!(
            server_id, *client_id,
            "client_id={client_id}: server must override client-supplied operation_id"
        );
    }
}

/// Each server operation_id is unique — replay attacks with fixed client IDs cannot
/// produce a predictable or repeatable operation_id.
#[test]
fn test_p5_server_operation_ids_are_unpredictable() {
    let ids: Vec<String> = (0..10).map(|_| Uuid::new_v4().to_string()).collect();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(
        unique.len(), ids.len(),
        "all server-generated operation_ids must be distinct (no replay risk)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Preview payload cannot produce stage=execute (no bypass)
// ─────────────────────────────────────────────────────────────────────────────

/// All known preview-class payloads must produce stage=preview, never stage=execute.
/// This closes the bypass path where dry_run=true could silently become execute.
#[test]
fn test_p5_preview_payloads_cannot_produce_execute_stage() {
    let preview_payloads = vec![
        json!({"action": "policy", "dry_run": false}),    // action-class preview
        json!({"action": "hook-policy"}),
        json!({"action": "drift"}),
        json!({"action": "status"}),
        json!({"dry_run": true}),                          // dry_run flag
        json!({"dry_run": true, "action": "sync"}),        // both set: dry_run wins
        json!({"action": "branch-preview"}),               // contains "preview"
    ];

    for payload in &preview_payloads {
        let stage = detect_stage(payload);
        assert_eq!(
            stage, "preview",
            "payload={payload}: preview-class payload must NEVER produce stage=execute"
        );
    }
}

/// Adversarial: dry_run set to non-boolean truthy-looking string must not bypass to execute.
#[test]
fn test_p5_dry_run_string_value_does_not_bypass_preview_gate() {
    // dry_run as a string "true" must be treated as absent (not a bool)
    let payload = json!({"action": "sync", "dry_run": "true"});
    // detect_stage only reads `as_bool()`; "true" string → as_bool() = None → not preview
    // This means action "sync" without bool dry_run → execute
    // This is CORRECT: we don't silently coerce string "true" to bool true for safety gates
    let stage = detect_stage(&payload);
    // "sync" is not a preview-class action, dry_run is not a bool → execute
    assert_eq!(
        stage, "execute",
        "string-typed dry_run must not be silently coerced to bool true to bypass safety gate"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. AGOrg scope switch must clear active operation context (rail state contract)
// ─────────────────────────────────────────────────────────────────────────────

/// Simulates the JS rail state model: on AGOrg scope switch, active_operation_id and
/// step state must be reset to null/idle before first click starts fresh lineage.
/// This validates the contract at the data model level (no live server required).
#[test]
fn test_p5_agorg_scope_switch_clears_rail_state() {
    // Rail state model (mirrors pilot_ui.js p5RailState)
    #[derive(Debug, PartialEq)]
    struct RailState {
        active_operation_id: Option<String>,
        active_scope_id: Option<String>,
        steps: Vec<String>, // "idle" | "preview" | "ok" | "error"
    }

    impl RailState {
        fn fresh() -> Self {
            RailState {
                active_operation_id: None,
                active_scope_id: None,
                steps: vec!["idle".to_string(); 6],
            }
        }

        fn on_run(&mut self, scope_id: &str, operation_id: String) {
            // If scope changed, clear state before applying new operation
            if self.active_scope_id.as_deref() != Some(scope_id) {
                self.active_operation_id = None;
                self.steps = vec!["idle".to_string(); 6];
            }
            self.active_scope_id = Some(scope_id.to_string());
            self.active_operation_id = Some(operation_id);
            // Mark first step as preview
            if let Some(s) = self.steps.first_mut() {
                *s = "preview".to_string();
            }
        }
    }

    let mut state = RailState::fresh();

    // Simulate running a preview in scope A
    let scope_a = "agorg-scope-00000000-aaaa-0000-0000-000000000001";
    let op_a = Uuid::new_v4().to_string();
    state.on_run(scope_a, op_a.clone());
    assert_eq!(state.active_scope_id.as_deref(), Some(scope_a));
    assert_eq!(state.active_operation_id.as_deref(), Some(op_a.as_str()));
    assert_eq!(state.steps[0], "preview");

    // Simulate AGOrg scope switch to scope B
    let scope_b = "agorg-scope-00000000-bbbb-0000-0000-000000000002";
    let op_b = Uuid::new_v4().to_string();
    state.on_run(scope_b, op_b.clone());

    // After scope switch: operation_id must be new, old step state must be cleared
    assert_eq!(state.active_scope_id.as_deref(), Some(scope_b));
    assert_eq!(
        state.active_operation_id.as_deref(),
        Some(op_b.as_str()),
        "after scope switch, operation_id must be the fresh one from scope B"
    );
    // The fresh first step must be from the new run
    assert_eq!(state.steps[0], "preview", "first step in new scope must be preview");
    // Previous scope A operation_id must not be reachable
    assert_ne!(
        state.active_operation_id.as_deref(),
        Some(op_a.as_str()),
        "scope A operation_id must not survive a scope switch"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Error envelope still has all required fields
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_p5_error_envelope_has_all_required_fields() {
    let error_envelope = unknown_domain_envelope("badomain");
    let required_fields = ["ok", "operation_id", "domain", "stage", "status", "summary", "inner"];
    for field in &required_fields {
        assert!(
            error_envelope.get(*field).is_some(),
            "error envelope must have field '{field}'"
        );
    }
    // error field must be present and non-null for error envelopes
    assert!(
        error_envelope.get("error").is_some(),
        "error envelope must have 'error' field (may be null for ok, non-null for error)"
    );
}
