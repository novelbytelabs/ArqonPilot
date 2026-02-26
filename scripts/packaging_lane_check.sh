#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/frozen_versions.sh"

if [[ ! -f Cargo.lock.packaging ]]; then
  echo "ERROR: Cargo.lock.packaging not found" >&2
  exit 1
fi

TMP_BAK="$(mktemp)"
cp Cargo.lock "$TMP_BAK"
restore_lock() {
  cp "$TMP_BAK" Cargo.lock
  rm -f "$TMP_BAK"
}
trap restore_lock EXIT

echo "[packaging-lane] using Rust ${PILOT_PACKAGING_RUST_VERSION}"
rustup run "$PILOT_PACKAGING_RUST_VERSION" cargo --version

echo "[packaging-lane] syncing Cargo.lock <- Cargo.lock.packaging"
cp Cargo.lock.packaging Cargo.lock

echo "[packaging-lane] check"
rustup run "$PILOT_PACKAGING_RUST_VERSION" cargo check -p pilot --locked

echo "[packaging-lane] help surface smoke"
rustup run "$PILOT_PACKAGING_RUST_VERSION" cargo run -q -p pilot -- --help >/tmp/pilot_help_packaging.txt
if command -v rg >/dev/null 2>&1; then
  rg -n "oracle|heal|navigate|branch|multi|secure|plan|create|know|init|serve" /tmp/pilot_help_packaging.txt >/dev/null
else
  grep -En "oracle|heal|navigate|branch|multi|secure|plan|create|know|init|serve" /tmp/pilot_help_packaging.txt >/dev/null
fi

echo "Packaging lane check passed."
