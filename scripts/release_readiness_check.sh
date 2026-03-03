#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/frozen_versions.sh"

core_cargo() {
  rustup run "$PILOT_CORE_RUST_VERSION" cargo "$@"
}

echo "[1/7] Toolchain and lockfile policy check"
./scripts/verify_toolchain_policy.sh

echo "[2/7] Locked compile"
core_cargo check -p pilot --locked

echo "[3/7] Full locked test suite"
core_cargo test -p pilot --locked

echo "[4/7] Command surface smoke check"
core_cargo run -q -p pilot -- --help >/tmp/pilot_help.txt
pattern="oracle|heal|navigate|branch|multi|secure|plan|create|know|init"
if grep -qE "$pattern" /tmp/pilot_help.txt; then
  echo "✅ Command surface OK"
else
  echo "❌ Command surface FAIL"
  exit 1
fi

echo "[5/7] JS Syntax Check (G-015 prevention)"
if command -v node >/dev/null 2>&1; then
  node -c crates/pilot/src/pilot_ui.js
  echo "✅ JS Syntax OK"
else
  echo "⚠️ node not found, skipping JS syntax check"
fi

echo "[6/7] Duplicate Const Check (G-015 prevention)"
./scripts/check_duplicate_consts.py

echo "[7/7] Rust toolchain pin check"
if grep -q "channel = \"${PILOT_CORE_RUST_VERSION}\"" rust-toolchain.toml; then
  echo "✅ rust-toolchain.toml OK"
else
  echo "❌ rust-toolchain.toml FAIL"
  exit 1
fi

echo "All release readiness gates passed."
