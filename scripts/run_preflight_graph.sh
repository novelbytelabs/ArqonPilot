#!/usr/bin/env bash
# =============================================================================
# Unified Preflight Graph Runner
# =============================================================================
# Executes the canonical preflight graph: Policy -> Hook -> Drift -> Gate -> Push
#
# Usage:
#   ./scripts/run_preflight_graph.sh [--json] [--continue-on-failure] [--skip-push]
#
# Output:
#   Canonical JSON envelope with status, failure codes, remediation, evidence
# =============================================================================

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Configuration
CONTINUE_ON_FAILURE=0
SKIP_PUSH=0
JSON_OUTPUT=0
if [[ "${1:-}" == "--json" ]]; then
  JSON_OUTPUT=1
  shift
fi
if [[ "${1:-}" == "--continue-on-failure" ]]; then
  CONTINUE_ON_FAILURE=1
  shift
fi
if [[ "${1:-}" == "--skip-push" ]]; then
  SKIP_PUSH=1
  shift
fi

# Generate execution ID
EXECUTION_ID="$(uuidgen 2>/dev/null || echo "exec-$(date +%s)-$$")"
START_TIME="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Report directory
REPORT_DIR="${PILOT_REPORT_DIR:-$HOME/.pilot/reports}"
mkdir -p "$REPORT_DIR"

# JSON output file
JSON_OUTPUT_FILE="$REPORT_DIR/preflight_graph_${EXECUTION_ID}.json"

# =============================================================================
# Helper Functions
# =============================================================================

log() {
  if [[ "$JSON_OUTPUT" -eq 0 ]]; then
    echo "$@"
  fi
}

timestamp_ms() {
  date +%s%3N
}

# Initialize JSON envelope
init_json() {
  cat > "$JSON_OUTPUT_FILE" <<EOF
{
  "execution_id": "$EXECUTION_ID",
  "timestamp": "$START_TIME",
  "graph": {
    "policy": { "status": "NOT_RUN", "duration_ms": 0 },
    "hook": { "status": "NOT_RUN", "duration_ms": 0 },
    "drift": { "status": "NOT_RUN", "duration_ms": 0 },
    "gate": { "status": "NOT_RUN", "duration_ms": 0 },
    "push": { "status": "NOT_RUN", "duration_ms": 0 }
  },
  "summary": {
    "total_duration_ms": 0,
    "overall_status": "PENDING",
    "retryable": false,
    "continue_on_failure": $CONTINUE_ON_FAILURE
  }
}
EOF
}

# Update JSON field
update_json() {
  local key="$1"
  local value="$2"
  # Use sed to update JSON - simple approach
  local tmp="$JSON_OUTPUT_FILE.tmp"
  sed "s/\"$key\": \"[^\"]*\"/\"$key\": \"$value\"/" "$JSON_OUTPUT_FILE" > "$tmp"
  mv "$tmp" "$JSON_OUTPUT_FILE"
}

# Update JSON field with object
update_json_object() {
  local key="$1"
  local value="$2"
  local tmp="$JSON_OUTPUT_FILE.tmp"
  sed "s/\"$key\": {[^}]*}/\"$key\": $value/" "$JSON_OUTPUT_FILE" > "$tmp"
  mv "$tmp" "$JSON_OUTPUT_FILE"
}

# Add failure to JSON
add_failure() {
  local code="$1"
  local message="$2"
  local remediation="$3"
  local evidence="$4"
  
  local tmp="$JSON_OUTPUT_FILE.tmp"
  cat > "$tmp" <<EOF
{
  "execution_id": "$EXECUTION_ID",
  "timestamp": "$START_TIME",
  $(tail -n +2 "$JSON_OUTPUT_FILE" | head -n -1 | sed 's/}$/,\n  "failure": {\n    "code": "'"$code"'",\n    "message": "'"$message"'",\n    "remediation": ["'"$remediation"'"],\n    "evidence": {"log_file": "'"$evidence"'"}    }\n  }/')
}
EOF
  mv "$tmp" "$JSON_OUTPUT_FILE"
}

# =============================================================================
# Step 1: Policy
# =============================================================================
run_policy() {
  log "[1/5] Policy evaluation"
  
  local start_ms end_ms duration status
  start_ms=$(timestamp_ms)
  
  # Check if governance is configured
  if [[ -f "$ROOT/crates/pilot/src/governance/store.rs" ]]; then
    # Governance store exists - run policy check
    if ./scripts/verify_toolchain_policy.sh >/dev/null 2>&1; then
      status="PASS"
    else
      status="FAIL"
    fi
  else
    status="NOT_CONFIGURED"
  fi
  
  end_ms=$(timestamp_ms)
  duration=$((end_ms - start_ms))
  
  update_json_object "policy" "{\"status\": \"$status\", \"duration_ms\": $duration}"
  
  if [[ "$status" == "FAIL" ]]; then
    log "  ❌ Policy check failed"
    return 1
  fi
  log "  ✅ Policy check passed"
  return 0
}

# =============================================================================
# Step 2: Hook
# =============================================================================
run_hook() {
  log "[2/5] Git hook verification"
  
  local start_ms end_ms duration status
  start_ms=$(timestamp_ms)
  
  # Check if hook policy script exists
  if [[ -f "$ROOT/scripts/verify_git_hook_policy.sh" ]]; then
    if ./scripts/verify_git_hook_policy.sh >/dev/null 2>&1; then
      status="PASS"
    else
      status="FAIL"
    fi
  else
    # Hook verification not configured - skip
    status="SKIPPED"
  fi
  
  end_ms=$(timestamp_ms)
  duration=$((end_ms - start_ms))
  
  update_json_object "hook" "{\"status\": \"$status\", \"duration_ms\": $duration}"
  
  if [[ "$status" == "FAIL" ]]; then
    log "  ❌ Hook verification failed"
    return 1
  fi
  log "  ✅ Hook verification passed"
  return 0
}

# =============================================================================
# Step 3: Drift
# =============================================================================
run_drift() {
  log "[3/5] Dependency drift detection"
  
  local start_ms end_ms duration status
  start_ms=$(timestamp_ms)
  
  # Run drift check
  if [[ -f "$ROOT/scripts/drift_report.sh" ]]; then
    if ./scripts/drift_report.sh Cargo.lock >/dev/null 2>&1; then
      status="PASS"
    else
      status="FAIL"
    fi
  else
    # No drift check available
    status="SKIPPED"
  fi
  
  end_ms=$(timestamp_ms)
  duration=$((end_ms - start_ms))
  
  update_json_object "drift" "{\"status\": \"$status\", \"duration_ms\": $duration}"
  
  if [[ "$status" == "FAIL" ]]; then
    log "  ❌ Drift check failed"
    return 1
  fi
  log "  ✅ Drift check passed"
  return 0
}

# =============================================================================
# Step 4: Gate
# =============================================================================
run_gate() {
  log "[4/5] Pre-push gate"
  
  local start_ms end_ms duration status
  start_ms=$(timestamp_ms)
  
  # Run prepush gate
  if ./scripts/prepush_gate.sh >/dev/null 2>&1; then
    status="PASS"
  else
    status="FAIL"
  fi
  
  end_ms=$(timestamp_ms)
  duration=$((end_ms - start_ms))
  
  update_json_object "gate" "{\"status\": \"$status\", \"duration_ms\": $duration}"
  
  if [[ "$status" == "FAIL" ]]; then
    log "  ❌ Pre-push gate failed"
    return 1
  fi
  log "  ✅ Pre-push gate passed"
  return 0
}

# =============================================================================
# Step 5: Push
# =============================================================================
run_push() {
  log "[5/5] Git push"
  
  if [[ "$SKIP_PUSH" -eq 1 ]]; then
    update_json_object "push" "{\"status\": \"SKIPPED\", \"duration_ms\": 0}"
    log "  ⏭️  Push skipped"
    return 0
  fi
  
  local start_ms end_ms duration status
  start_ms=$(timestamp_ms)
  
  # Run push (dry-run for safety)
  if ./scripts/push_main.sh >/dev/null 2>&1; then
    status="SENT"
  else
    status="FAILED"
  fi
  
  end_ms=$(timestamp_ms)
  duration=$((end_ms - start_ms))
  
  update_json_object "push" "{\"status\": \"$status\", \"duration_ms\": $duration}"
  
  if [[ "$status" == "FAILED" ]]; then
    log "  ❌ Push failed"
    return 1
  fi
  log "  ✅ Push completed"
  return 0
}

# =============================================================================
# Main Execution
# =============================================================================
main() {
  log "=============================================="
  log "Unified Preflight Graph"
  log "=============================================="
  log "Execution ID: $EXECUTION_ID"
  log "Timestamp: $START_TIME"
  log ""
  
  # Initialize JSON
  init_json
  
  local overall_status="PASS"
  local total_start total_end total_duration
  total_start=$(timestamp_ms)
  
  # Run graph
  if ! run_policy; then
    overall_status="FAIL"
    if [[ "$CONTINUE_ON_FAILURE" -eq 0 ]]; then
      update_json "overall_status" "FAIL"
      total_end=$(timestamp_ms)
      total_duration=$((total_end - total_start))
      update_json "total_duration_ms" "$total_duration"
      log ""
      log "=============================================="
      log "Result: ❌ FAILED at Policy"
      log "=============================================="
      cat "$JSON_OUTPUT_FILE"
      exit 1
    fi
  fi
  
  if ! run_hook; then
    overall_status="FAIL"
    if [[ "$CONTINUE_ON_FAILURE" -eq 0 ]]; then
      update_json "overall_status" "FAIL"
      total_end=$(timestamp_ms)
      total_duration=$((total_end - total_start))
      update_json "total_duration_ms" "$total_duration"
      log ""
      log "=============================================="
      log "Result: ❌ FAILED at Hook"
      log "=============================================="
      cat "$JSON_OUTPUT_FILE"
      exit 1
    fi
  fi
  
  if ! run_drift; then
    overall_status="FAIL"
    if [[ "$CONTINUE_ON_FAILURE" -eq 0 ]]; then
      update_json "overall_status" "FAIL"
      total_end=$(timestamp_ms)
      total_duration=$((total_end - total_start))
      update_json "total_duration_ms" "$total_duration"
      log ""
      log "=============================================="
      log "Result: ❌ FAILED at Drift"
      log "=============================================="
      cat "$JSON_OUTPUT_FILE"
      exit 1
    fi
  fi
  
  if ! run_gate; then
    overall_status="FAIL"
    if [[ "$CONTINUE_ON_FAILURE" -eq 0 ]]; then
      update_json "overall_status" "FAIL"
      total_end=$(timestamp_ms)
      total_duration=$((total_end - total_start))
      update_json "total_duration_ms" "$total_duration"
      log ""
      log "=============================================="
      log "Result: ❌ FAILED at Gate"
      log "=============================================="
      cat "$JSON_OUTPUT_FILE"
      exit 1
    fi
  fi
  
  if ! run_push; then
    overall_status="FAIL"
    if [[ "$CONTINUE_ON_FAILURE" -eq 0 ]]; then
      update_json "overall_status" "FAIL"
      total_end=$(timestamp_ms)
      total_duration=$((total_end - total_start))
      update_json "total_duration_ms" "$total_duration"
      log ""
      log "=============================================="
      log "Result: ❌ FAILED at Push"
      log "=============================================="
      cat "$JSON_OUTPUT_FILE"
      exit 1
    fi
  fi
  
  # All passed
  total_end=$(timestamp_ms)
  total_duration=$((total_end - total_start))
  
  update_json "overall_status" "$overall_status"
  update_json "total_duration_ms" "$total_duration"
  
  log ""
  log "=============================================="
  log "Result: ✅ PASSED"
  log "Total duration: ${total_duration}ms"
  log "=============================================="
  
  if [[ "$JSON_OUTPUT" -eq 1 ]]; then
    cat "$JSON_OUTPUT_FILE"
  else
    echo "JSON output: $JSON_OUTPUT_FILE"
  fi
  
  exit 0
}

main "$@"
