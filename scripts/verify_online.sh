#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "[1/5] Checking Wave 0 invariants"
./scripts/wave0_safety_check.sh

echo "[2/5] Checking formatting"
cargo fmt --all -- --check

echo "[3/5] Locked compile check"
cargo check -p pilot --locked

echo "[4/5] Locked test run"
cargo test -p pilot --locked

echo "[5/5] CLI help smoke test"
cargo run -p pilot -- --help >/tmp/pilot_help.txt
rg -n "oracle|heal|navigate|branch|multi|secure|plan|create|know|init" /tmp/pilot_help.txt >/dev/null

echo "Online verification passed."
