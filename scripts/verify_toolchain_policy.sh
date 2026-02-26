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

check_lock_compat() {
  local file="$1"
  awk -v f="$file" '
    function check_one(name, version) {
      if (name == "comfy-table" && version ~ /^7\.(2|3|4|5|6|7|8|9)/) {
        printf("ERROR: %s has comfy-table %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "time" && version ~ /^0\.3\.(4[7-9]|[5-9][0-9])/) {
        printf("ERROR: %s has time %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "time-core" && version ~ /^0\.1\.(8|9|[1-9][0-9])/) {
        printf("ERROR: %s has time-core %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "wit-bindgen" && version ~ /^0\.5[1-9]\./) {
        printf("ERROR: %s has wit-bindgen %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "constant_time_eq" && version ~ /^0\.4\./) {
        printf("ERROR: %s has constant_time_eq %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
    }

    /^\[\[package\]\]/ {
      if (n != "" && v != "") check_one(n, v);
      n = ""; v = "";
      next;
    }
    /^name = "/ {
      n = $0;
      sub(/^name = "/, "", n);
      sub(/"$/, "", n);
      next;
    }
    /^version = "/ {
      v = $0;
      sub(/^version = "/, "", v);
      sub(/"$/, "", v);
      next;
    }
    END {
      if (n != "" && v != "") check_one(n, v);
      if (bad == 1) exit 1;
    }
  ' "$file"
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

echo "[policy] lockfile compatibility for Rust 1.82 core lane"
check_lock_compat Cargo.lock

if [[ "${VERIFY_PACKAGING_LOCK_182:-0}" == "1" ]]; then
  echo "[policy] optional packaging lock compatibility check for Rust 1.82"
  check_lock_compat Cargo.lock.packaging
fi

echo "[policy] python and cargo versions aligned"
PY_VER="$(sed -n 's/^version = \"\([0-9][0-9.]*\)\"$/\1/p' pyproject.toml | head -n1)"
CARGO_VER="$(sed -n 's/^version = \"\([0-9][0-9.]*\)\"$/\1/p' Cargo.toml | head -n1)"
if [[ "$PY_VER" != "$CARGO_VER" ]]; then
  echo "ERROR: version mismatch pyproject.toml=$PY_VER Cargo.toml=$CARGO_VER" >&2
  exit 1
fi

echo "Toolchain policy checks passed."
