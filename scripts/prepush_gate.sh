#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/frozen_versions.sh"

# Cargo network hardening for transient crates.io/DNS issues.
export CARGO_REGISTRIES_CRATES_IO_PROTOCOL="${CARGO_REGISTRIES_CRATES_IO_PROTOCOL:-sparse}"
export CARGO_NET_RETRY="${CARGO_NET_RETRY:-10}"
export CARGO_HTTP_TIMEOUT="${CARGO_HTTP_TIMEOUT:-60}"

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

TRANSIENT_NET_PATTERN='Could not resolve host|Temporary failure in name resolution|error sending request for url|failed to download from|spurious network error|operation timed out|failed to query replaced source registry'

print_dns_diag() {
  echo "[net] DNS diagnostics:"
  getent hosts index.crates.io || true
  getent hosts static.crates.io || true
}

core_cargo() {
  rustup run "$PILOT_CORE_RUST_VERSION" cargo "$@"
}

run_with_retry() {
  local label="$1"
  local max_attempts="$2"
  shift 2
  local attempt=1
  local delay=2
  local tmp
  local rc

  while (( attempt <= max_attempts )); do
    echo "[$label] attempt ${attempt}/${max_attempts}"
    tmp="$(mktemp)"
    set +e
    "$@" 2>&1 | tee "$tmp"
    rc="${PIPESTATUS[0]}"
    set -e
    if [[ "$rc" -eq 0 ]]; then
      rm -f "$tmp"
      return 0
    fi

    if grep -Eiq "$TRANSIENT_NET_PATTERN" "$tmp"; then
      echo "[$label] transient network/DNS failure detected."
      print_dns_diag
      if (( attempt < max_attempts )); then
        echo "[$label] retrying in ${delay}s..."
        sleep "$delay"
        delay=$((delay * 2))
        attempt=$((attempt + 1))
        rm -f "$tmp"
        continue
      fi
      echo "[$label] retries exhausted."
    fi

    rm -f "$tmp"
    return "$rc"
  done
}

finish() {
  local code="$1"
  if [[ "$code" -eq 0 ]]; then
    echo "[pre-push] status: PASS"
  else
    echo "[pre-push] status: FAIL"
    echo "[pre-push] remediation:"
    echo "  1) Inspect log: $LOG_FILE"
    echo "  2) Run: ./scripts/repair_lock_182.sh --no-gate"
    echo "  3) Re-run: ./scripts/prepush_gate.sh"
    echo "  4) Confirm lane parity: ./scripts/ci_parity_check.sh"
  fi
}
trap 'finish $?' EXIT

echo "[1/4] Toolchain policy"
./scripts/verify_toolchain_policy.sh

echo "[2/4] Locked compile gate (mandatory)"
run_with_retry "cargo-check" 3 core_cargo check -p pilot --locked

echo "[3/4] Targeted locked CLI tests"
run_with_retry "cargo-test" 3 core_cargo test -p pilot --locked \
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
run_with_retry "cargo-help" 3 core_cargo run -q -p pilot -- --help >/tmp/pilot_help.txt
if command -v rg >/dev/null 2>&1; then
  rg -n "oracle|heal|navigate|branch|multi|secure|plan|create|know|init|serve" /tmp/pilot_help.txt >/dev/null
else
  grep -En "oracle|heal|navigate|branch|multi|secure|plan|create|know|init|serve" /tmp/pilot_help.txt >/dev/null
fi

echo "Pre-push gate passed."
