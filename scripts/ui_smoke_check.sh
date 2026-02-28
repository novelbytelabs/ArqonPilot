#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

UI_PORT="${PILOT_UI_PORT:-7791}"
WS_URL="${PILOT_WS_URL:-ws://127.0.0.1:9100}"
ROOM="${PILOT_UI_ROOM:-pilot}"
CHANNEL="${PILOT_UI_CHANNEL:-control}"
TELEMETRY_CHANNEL="${PILOT_UI_TELEMETRY_CHANNEL:-telemetry}"
START_SHIM="${PILOT_UI_SMOKE_START_SHIM:-1}"
REUSE_UI="${PILOT_UI_SMOKE_REUSE_UI:-0}"
INCLUDE_COMMANDS="${PILOT_UI_SMOKE_INCLUDE_COMMANDS:-1}"
REPORT_DIR="${PILOT_REPORT_DIR:-$HOME/.pilot/reports}"
CURL_TIMEOUT_SEC="${PILOT_UI_SMOKE_CURL_TIMEOUT_SEC:-35}"
STARTUP_TIMEOUT_SEC="${PILOT_UI_SMOKE_STARTUP_TIMEOUT_SEC:-900}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_FILE="$REPORT_DIR/ui_smoke_${STAMP}.log"

mkdir -p "$REPORT_DIR" 2>/dev/null || true
if ! touch "$LOG_FILE" 2>/dev/null; then
  REPORT_DIR="/tmp/pilot-reports"
  mkdir -p "$REPORT_DIR"
  LOG_FILE="$REPORT_DIR/ui_smoke_${STAMP}.log"
  touch "$LOG_FILE"
fi

SERVE_PID=""

cleanup() {
  if [[ -n "${SERVE_PID:-}" ]]; then
    kill "$SERVE_PID" 2>/dev/null || true
    wait "$SERVE_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

say() {
  echo "[ui-smoke] $*"
}

wait_http() {
  local url="$1"
  local attempts="${2:-60}"
  local delay="${3:-1}"
  local i
  for ((i = 1; i <= attempts; i++)); do
    if curl -sS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep "$delay"
  done
  return 1
}

require_json_key() {
  local key="$1"
  local file="$2"
  python3 - "$key" "$file" <<'PY'
import json
import sys
key = sys.argv[1]
path = sys.argv[2]
with open(path, "r", encoding="utf-8") as f:
    data = json.load(f)
if key not in data:
    print(f"missing key: {key}", file=sys.stderr)
    sys.exit(1)
PY
}

post_json() {
  local path="$1"
  local payload="$2"
  local out_file="$3"
  curl -sS \
    --max-time "$CURL_TIMEOUT_SEC" \
    -X POST \
    -H "content-type: application/json" \
    "http://127.0.0.1:${UI_PORT}${path}" \
    -d "$payload" >"$out_file"
}

say "log file: $LOG_FILE"
say "startup timeout sec: $STARTUP_TIMEOUT_SEC"

if [[ "$START_SHIM" == "1" ]]; then
  say "starting/checking ArqonBus shim..."
  if ! PILOT_REPORT_DIR="$REPORT_DIR" ./scripts/arqonbus_shim.sh start >>"$LOG_FILE" 2>&1; then
    say "shim start returned non-zero, checking status and continuing if reachable..."
    PILOT_REPORT_DIR="$REPORT_DIR" ./scripts/arqonbus_shim.sh status >>"$LOG_FILE" 2>&1 || true
  fi
fi

if [[ "$REUSE_UI" == "1" ]] && wait_http "http://127.0.0.1:${UI_PORT}/" 1 1; then
  say "reusing existing pilot serve on :${UI_PORT}"
else
  say "starting pilot serve on :${UI_PORT} (ws=${WS_URL})..."
  cargo run -q -p pilot -- \
    serve \
    --ws-url "$WS_URL" \
    --room "$ROOM" \
    --channel "$CHANNEL" \
    --telemetry-channel "$TELEMETRY_CHANNEL" \
    --ui-port "$UI_PORT" >>"$LOG_FILE" 2>&1 &
  SERVE_PID="$!"

  startup_attempts=$(( (STARTUP_TIMEOUT_SEC / 2) + 1 ))
  if ! wait_http "http://127.0.0.1:${UI_PORT}/" "$startup_attempts" 2; then
    say "ERROR: UI failed to start; tailing log"
    tail -n 120 "$LOG_FILE" || true
    exit 1
  fi
fi

say "checking dashboard HTML..."
curl -sS "http://127.0.0.1:${UI_PORT}/" | grep -q "Arqon Pilot"

say "checking read endpoints..."
curl -sS --max-time "$CURL_TIMEOUT_SEC" "http://127.0.0.1:${UI_PORT}/api/history" >/dev/null
curl -sS --max-time "$CURL_TIMEOUT_SEC" "http://127.0.0.1:${UI_PORT}/api/reports?limit=5" >/dev/null
curl -sS --max-time "$CURL_TIMEOUT_SEC" "http://127.0.0.1:${UI_PORT}/api/dependencies/logs" >/dev/null

tmp_dep="$(mktemp)"
tmp_cmd="$(mktemp)"
trap 'rm -f "$tmp_dep" "$tmp_cmd"; cleanup' EXIT

say "checking dependency action endpoint..."
post_json "/api/dependencies/run" '{"action":"bus-status"}' "$tmp_dep"
require_json_key "ok" "$tmp_dep"

if [[ "$INCLUDE_COMMANDS" == "1" ]]; then
  say "checking command endpoint (multi status)..."
  post_json "/api/command" '{"command":"pilot.multi.status","payload":{"group":"core","tags":["apply-pilot"],"schema_version":1}}' "$tmp_cmd"
  require_json_key "ok" "$tmp_cmd"

  say "checking command endpoint (multi dag dry-run)..."
  post_json "/api/command" '{"command":"pilot.multi.dag","payload":{"group":"core","tags":["apply-pilot"],"dry_run":true,"schema_version":1}}' "$tmp_cmd"
  require_json_key "ok" "$tmp_cmd"

  say "checking command endpoint (multi apply dry-run)..."
  post_json "/api/command" '{"command":"pilot.multi.apply","payload":{"branch":"feat/ui-smoke","base_branch":"dev","pr_base_branch":"main","group":"core","tags":["apply-pilot"],"stage_size":2,"continue_on_failure":false,"apply":false,"schema_version":1}}' "$tmp_cmd"
  require_json_key "ok" "$tmp_cmd"
else
  say "skipping command-lane checks (set PILOT_UI_SMOKE_INCLUDE_COMMANDS=1 to enable)"
fi

say "UI smoke check passed."
