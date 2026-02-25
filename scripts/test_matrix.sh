#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SUITE="${1:-all}"

run_unit() {
  echo "[unit] cargo test --workspace --lib --locked"
  cargo test --workspace --lib --locked
}

run_integration() {
  echo "[integration] pilot integration test set"
  cargo test -p pilot --locked \
    --test branch_cli_test \
    --test create_cli_test \
    --test heal_cli_test \
    --test know_cli_test \
    --test multi_cli_test \
    --test navigate_cli_test \
    --test oracle_cli_test \
    --test plan_cli_test \
    --test report_cli_test \
    --test secure_cli_test
}

run_e2e() {
  echo "[e2e] pilot end-to-end dry-run set"
  cargo test -p pilot --locked \
    --test e2e_ship \
    --test e2e_wave6_dryrun_test
}

run_regression() {
  echo "[regression] regression-focused tests"
  cargo test -p pilot --locked \
    --test ship_test \
    --test heal_test \
    --test oracle_test \
    --test graph_test \
    --test vector_test \
    --test regression_cli_error_report_test
}

run_adversarial() {
  echo "[adversarial] hostile-input safety tests"
  cargo test -p pilot --locked --test adversarial_cli_test
}

case "$SUITE" in
  unit) run_unit ;;
  integration) run_integration ;;
  e2e) run_e2e ;;
  regression) run_regression ;;
  adversarial) run_adversarial ;;
  all)
    run_unit
    run_integration
    run_e2e
    run_regression
    run_adversarial
    ;;
  *)
    echo "Usage: $0 {unit|integration|e2e|regression|adversarial|all}" >&2
    exit 2
    ;;
esac

echo "Test suite '$SUITE' passed."
