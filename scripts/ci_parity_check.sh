#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "[ci-parity] toolchain and lock policy"
./scripts/verify_toolchain_policy.sh

echo "[ci-parity] core lane"
rustup run 1.82.0 cargo check -p pilot --locked

echo "[ci-parity] packaging lane"
./scripts/packaging_lane_check.sh

echo "CI parity check passed."
