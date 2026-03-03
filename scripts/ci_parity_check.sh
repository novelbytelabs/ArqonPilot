#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/frozen_versions.sh"

echo "[ci-parity] toolchain and lock policy"
./scripts/verify_toolchain_policy.sh

echo "[ci-parity] core drift scan"
./scripts/drift_report.sh Cargo.lock

echo "[ci-parity] packaging drift scan"
./scripts/drift_report.sh Cargo.lock.packaging --json

echo "[ci-parity] core lane"
rustup run "$PILOT_CORE_RUST_VERSION" cargo check -p pilot --locked

echo "[ci-parity] packaging lane"
./scripts/packaging_lane_check.sh

if [[ "${PILOT_CI_PARITY_INCLUDE_UI_SMOKE:-1}" == "1" ]]; then
  echo "[ci-parity] ui smoke lane"
  ./scripts/ui_smoke_check.sh
fi

echo "CI parity check passed."
