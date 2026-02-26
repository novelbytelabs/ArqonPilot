#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/frozen_versions.sh"

JSON_MODE=0
if [[ "${1:-}" == "--json" ]]; then
  JSON_MODE=1
fi

FAILED_CHECKS=()

log() {
  if [[ "$JSON_MODE" -eq 0 ]]; then
    echo "$@"
  fi
}

run_check() {
  local check_id="$1"
  shift
  if ! "$@"; then
    FAILED_CHECKS+=("$check_id")
    return 1
  fi
  return 0
}

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

toolchain_installed() {
  local ver="$1"
  rustup toolchain list 2>/dev/null | grep -Fq "$ver"
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
      if (name == "globset" && version ~ /^0\.4\.1[8-9]/) {
        printf("ERROR: %s has globset %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "icu_collections" && version ~ /^2\.1\./) {
        printf("ERROR: %s has icu_collections %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "icu_locale_core" && version ~ /^2\.1\./) {
        printf("ERROR: %s has icu_locale_core %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "icu_normalizer" && version ~ /^2\.1\./) {
        printf("ERROR: %s has icu_normalizer %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "icu_normalizer_data" && version ~ /^2\.1\./) {
        printf("ERROR: %s has icu_normalizer_data %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "icu_properties" && version ~ /^2\.1\./) {
        printf("ERROR: %s has icu_properties %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "icu_properties_data" && version ~ /^2\.1\./) {
        printf("ERROR: %s has icu_properties_data %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
        bad = 1;
      }
      if (name == "icu_provider" && version ~ /^2\.1\./) {
        printf("ERROR: %s has icu_provider %s (Rust 1.82 incompatible)\n", f, version) > "/dev/stderr";
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

log "[policy] rust-toolchain pin"
run_check "rust_toolchain_pin" search_pattern "^channel = \"${PILOT_CORE_RUST_VERSION//./\\.}\"$" rust-toolchain.toml || true
run_check "rustup_available" command -v rustup >/dev/null 2>&1 || true
run_check "core_toolchain_installed" toolchain_installed "$PILOT_CORE_RUST_VERSION" || true

log "[policy] CI lane pin"
run_check "ci_lane_pin" search_pattern "toolchain:\\s*\"${PILOT_CORE_RUST_VERSION//./\\.}\"" .github/workflows/ci.yml || true
run_check "ci_packaging_lane_pin" search_pattern "toolchain:\\s*\"${PILOT_PACKAGING_RUST_VERSION//./\\.}\"" .github/workflows/ci.yml || true
run_check "ci_packaging_lane_check_step" search_pattern 'scripts/packaging_lane_check\.sh' .github/workflows/ci.yml || true

log "[policy] packaging lane pin"
run_check "packaging_lane_pin" search_pattern "toolchain:\\s*\"${PILOT_PACKAGING_RUST_VERSION//./\\.}\"" .github/workflows/pypi.yml || true

log "[policy] protobuf/protoc freeze pin"
run_check "packaging_protoc_pin" search_pattern "protoc-${PILOT_PROTOC_VERSION//./\\.}-linux-x86_64\\.zip" .github/workflows/pypi.yml || true
run_check "packaging_protoc_release_pin" search_pattern "releases/download/v${PILOT_PROTOC_VERSION//./\\.}/protoc-${PILOT_PROTOC_VERSION//./\\.}-linux-x86_64\\.zip" .github/workflows/pypi.yml || true
if contains_pattern 'apt-get\s+install\s+-y\s+protobuf-compiler' .github/workflows/pypi.yml; then
  echo "ERROR: pypi.yml must not use unpinned protobuf-compiler apt package; use protoc ${PILOT_PROTOC_VERSION} archive" >&2
  FAILED_CHECKS+=("packaging_no_unpinned_protobuf_apt")
fi

log "[policy] packaging lockfile policy"
run_check "packaging_lockfile_reference" search_pattern 'Cargo\.lock\.packaging' .github/workflows/pypi.yml || true
run_check "packaging_locked_flag" search_pattern '\--locked' .github/workflows/pypi.yml || true
if contains_pattern 'cargo\s+update' .github/workflows/pypi.yml; then
  echo "ERROR: pypi.yml must not run cargo update in CI" >&2
  FAILED_CHECKS+=("packaging_no_cargo_update")
fi

log "[policy] lockfiles exist"
run_check "core_lock_exists" test -f Cargo.lock || true
run_check "packaging_lock_exists" test -f Cargo.lock.packaging || true

log "[policy] lockfile compatibility for Rust 1.82 core lane"
run_check "core_lock_compatibility" check_lock_compat Cargo.lock || true

if [[ "${VERIFY_PACKAGING_LOCK_182:-0}" == "1" ]]; then
  log "[policy] optional packaging lock compatibility check for Rust 1.82"
  run_check "packaging_lock_compatibility" check_lock_compat Cargo.lock.packaging || true
fi

log "[policy] python and cargo versions aligned"
PY_VER="$(sed -n 's/^version = \"\([0-9][0-9.]*\)\"$/\1/p' pyproject.toml | head -n1)"
CARGO_VER="$(sed -n 's/^version = \"\([0-9][0-9.]*\)\"$/\1/p' Cargo.toml | head -n1)"
if [[ "$PY_VER" != "$CARGO_VER" ]]; then
  echo "ERROR: version mismatch pyproject.toml=$PY_VER Cargo.toml=$CARGO_VER" >&2
  FAILED_CHECKS+=("version_alignment")
fi

if [[ "${#FAILED_CHECKS[@]}" -gt 0 ]]; then
  if [[ "$JSON_MODE" -eq 1 ]]; then
    printf '{"ok":false,"failed_checks":['
    for i in "${!FAILED_CHECKS[@]}"; do
      if [[ "$i" -gt 0 ]]; then printf ','; fi
      printf '"%s"' "${FAILED_CHECKS[$i]}"
    done
    printf ']}\n'
  fi
  exit 1
fi

if [[ "$JSON_MODE" -eq 1 ]]; then
  printf '{"ok":true,"failed_checks":[]}\n'
else
  echo "Toolchain policy checks passed."
fi
