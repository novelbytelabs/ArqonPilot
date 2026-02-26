#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

REPORT_DIR="${PILOT_REPORT_DIR:-$HOME/.pilot/reports}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_FILE="$REPORT_DIR/prepush_gate_${STAMP}.log"
mkdir -p "$REPORT_DIR" 2>/dev/null || true
if ! touch "$LOG_FILE" 2>/dev/null; then
  REPORT_DIR="/tmp/pilot-reports"
  mkdir -p "$REPORT_DIR"
  LOG_FILE="$REPORT_DIR/prepush_gate_${STAMP}.log"
  touch "$LOG_FILE"
fi
exec > >(tee -a "$LOG_FILE") 2>&1

echo "[pre-push] log file: $LOG_FILE"

finish() {
  local code="$1"
  if [[ "$code" -eq 0 ]]; then
    echo "[pre-push] status: PASS"
  else
    echo "[pre-push] status: FAIL"
    echo "[pre-push] remediation:"
    echo "  1) Inspect log: $LOG_FILE"
    echo "  2) Run: ./scripts/repair_lock_182.sh"
    echo "  3) Re-run: ./scripts/prepush_gate.sh"
  fi
}
trap 'finish $?' EXIT

echo "[1/4] Toolchain policy"
./scripts/verify_toolchain_policy.sh

echo "[2/4] Locked compile gate (mandatory)"
cargo check -p pilot --locked

echo "[3/4] Targeted locked CLI tests"
cargo test -p pilot --locked \
  --test branch_cli_test \
  --test multi_cli_test \
  --test navigate_cli_test \
  --test secure_cli_test \
  --test plan_cli_test \
  --test create_cli_test \
  --test know_cli_test \
  --test heal_cli_test \
  --test oracle_cli_test \
  --test report_cli_test

echo "[4/4] Help surface smoke check"
cargo run -q -p pilot -- --help >/tmp/pilot_help.txt
if command -v rg >/dev/null 2>&1; then
  rg -n "oracle|heal|navigate|branch|multi|secure|plan|create|know|init|serve" /tmp/pilot_help.txt >/dev/null
else
  grep -En "oracle|heal|navigate|branch|multi|secure|plan|create|know|init|serve" /tmp/pilot_help.txt >/dev/null
fi

echo "Pre-push gate passed."
