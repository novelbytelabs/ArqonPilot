#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "[1/4] Locked compile"
cargo check -p pilot --locked

echo "[2/4] Targeted locked CLI/E2E tests"
cargo test -p pilot --locked \
  --test e2e_wave6_dryrun_test \
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

echo "[3/4] Command surface smoke check"
cargo run -q -p pilot -- --help >/tmp/pilot_help.txt
rg -n "oracle|heal|navigate|branch|multi|secure|plan|create|know|init" /tmp/pilot_help.txt >/dev/null

echo "[4/4] Rust toolchain pin check"
rg -n 'channel = "1\.82\.0"' rust-toolchain.toml >/dev/null

echo "[policy] Toolchain and lockfile policy checks"
./scripts/verify_toolchain_policy.sh

echo "Release readiness check passed."
