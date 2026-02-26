#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "[policy] rust-toolchain pin"
rg -n '^channel = "1\.82\.0"$' rust-toolchain.toml >/dev/null

echo "[policy] CI lane pin"
rg -n 'toolchain:\s*"1\.82\.0"' .github/workflows/ci.yml >/dev/null

echo "[policy] packaging lane pin"
rg -n 'toolchain:\s*"1\.88\.0"' .github/workflows/pypi.yml >/dev/null

echo "[policy] packaging lockfile policy"
rg -n 'Cargo\.lock\.packaging' .github/workflows/pypi.yml >/dev/null
rg -n '\--locked' .github/workflows/pypi.yml >/dev/null
if rg -n 'cargo\s+update' .github/workflows/pypi.yml >/dev/null; then
  echo "ERROR: pypi.yml must not run cargo update in CI" >&2
  exit 1
fi

echo "[policy] lockfiles exist"
test -f Cargo.lock
test -f Cargo.lock.packaging

echo "[policy] python and cargo versions aligned"
PY_VER="$(sed -n 's/^version = \"\([0-9][0-9.]*\)\"$/\1/p' pyproject.toml | head -n1)"
CARGO_VER="$(sed -n 's/^version = \"\([0-9][0-9.]*\)\"$/\1/p' Cargo.toml | head -n1)"
if [[ "$PY_VER" != "$CARGO_VER" ]]; then
  echo "ERROR: version mismatch pyproject.toml=$PY_VER Cargo.toml=$CARGO_VER" >&2
  exit 1
fi

echo "Toolchain policy checks passed."
