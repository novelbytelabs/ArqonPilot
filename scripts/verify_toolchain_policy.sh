#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

search_pattern() {
  local pattern="$1"
  local file="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -n "$pattern" "$file" >/dev/null
  else
    grep -En "$pattern" "$file" >/dev/null
  fi
}

contains_pattern() {
  local pattern="$1"
  local file="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -n "$pattern" "$file" >/dev/null
  else
    grep -En "$pattern" "$file" >/dev/null
  fi
}

echo "[policy] rust-toolchain pin"
search_pattern '^channel = "1\.82\.0"$' rust-toolchain.toml

echo "[policy] CI lane pin"
search_pattern 'toolchain:\s*"1\.82\.0"' .github/workflows/ci.yml

echo "[policy] packaging lane pin"
search_pattern 'toolchain:\s*"1\.88\.0"' .github/workflows/pypi.yml

echo "[policy] packaging lockfile policy"
search_pattern 'Cargo\.lock\.packaging' .github/workflows/pypi.yml
search_pattern '\--locked' .github/workflows/pypi.yml
if contains_pattern 'cargo\s+update' .github/workflows/pypi.yml; then
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
