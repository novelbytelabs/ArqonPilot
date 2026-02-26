#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/frozen_versions.sh"

# Cargo network hardening for transient crates.io/DNS issues.
export CARGO_REGISTRIES_CRATES_IO_PROTOCOL="${CARGO_REGISTRIES_CRATES_IO_PROTOCOL:-sparse}"
export CARGO_NET_RETRY="${CARGO_NET_RETRY:-10}"
export CARGO_HTTP_TIMEOUT="${CARGO_HTTP_TIMEOUT:-60}"

usage() {
  cat <<'EOF'
Usage: ./scripts/repair_lock_182.sh [--commit <sha>] [--no-gate] [--dry-run] [--verbose]

Repairs lockfile drift for Rust 1.82 core lane by restoring a compatible
Cargo.lock from git history. If no compatible commit is found, it falls back
to force-pinning known-safe dependency versions via `cargo update --precise`.

Options:
  --commit <sha>   Use this exact commit as lockfile source.
  --no-gate        Skip running ./scripts/prepush_gate.sh after repair.
  --dry-run        Print chosen commit and actions without changing files.
  --verbose        Print commit scanning progress.
EOF
}

TARGET_COMMIT=""
RUN_GATE=1
DRY_RUN=0
VERBOSE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --commit)
      TARGET_COMMIT="${2:-}"
      shift 2
      ;;
    --no-gate)
      RUN_GATE=0
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --verbose)
      VERBOSE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: Unknown option: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "ERROR: Not inside a git repository." >&2
  exit 1
fi

is_lockfile_182_compatible() {
  local file="$1"
  awk '
    function incompatible(name, version) {
      if (name == "comfy-table" && version ~ /^7\.(2|3|4|5|6|7|8|9)/) return 1;
      if (name == "time" && version ~ /^0\.3\.(4[7-9]|[5-9][0-9])/) return 1;
      if (name == "time-core" && version ~ /^0\.1\.(8|9|[1-9][0-9])/) return 1;
      if (name == "wit-bindgen" && version ~ /^0\.5[1-9]\./) return 1;
      if (name == "icu_collections" && version ~ /^2\.1\./) return 1;
      if (name == "icu_locale_core" && version ~ /^2\.1\./) return 1;
      if (name == "icu_normalizer" && version ~ /^2\.1\./) return 1;
      if (name == "icu_normalizer_data" && version ~ /^2\.1\./) return 1;
      if (name == "icu_properties" && version ~ /^2\.1\./) return 1;
      if (name == "icu_properties_data" && version ~ /^2\.1\./) return 1;
      if (name == "icu_provider" && version ~ /^2\.1\./) return 1;
      return 0;
    }

    /^\[\[package\]\]/ {
      if (n != "" && v != "" && incompatible(n, v)) bad = 1;
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
      if (n != "" && v != "" && incompatible(n, v)) bad = 1;
      if (bad == 1) exit 1;
    }
  ' "$file"
}

commit_has_compatible_core_lock() {
  local commit="$1"
  local tmp
  if ! git cat-file -e "${commit}:Cargo.lock" 2>/dev/null; then
    return 1
  fi
  tmp="$(mktemp)"
  git show "${commit}:Cargo.lock" >"$tmp"
  if ! is_lockfile_182_compatible "$tmp"; then
    rm -f "$tmp"
    return 1
  fi
  rm -f "$tmp"
  return 0
}

find_compatible_commit() {
  local c
  while IFS= read -r c; do
    if [[ "$VERBOSE" -eq 1 ]]; then
      echo "[scan] checking $c" >&2
    fi
    if commit_has_compatible_core_lock "$c"; then
      echo "$c"
      return 0
    fi
  done < <(git rev-list --all -- Cargo.lock)
  return 1
}

pin_if_present() {
  local dep="$1"
  local ver="$2"
  local tmp
  tmp="$(mktemp)"
  if cargo update -p "$dep" --precise "$ver" >"$tmp" 2>&1; then
    rm -f "$tmp"
    echo "[pin] $dep -> $ver"
    return 0
  fi
  echo "[pin] failed: $dep -> $ver" >&2
  sed -n '1,40p' "$tmp" >&2 || true
  rm -f "$tmp"
  return 1
}

pin_optional() {
  local dep="$1"
  local ver="$2"
  if pin_if_present "$dep" "$ver"; then
    return 0
  fi
  echo "[pin] optional dependency could not be pinned: $dep" >&2
  return 0
}

lock_has_pkg_version() {
  local dep="$1"
  local ver="$2"
  awk -v dep="$dep" -v ver="$ver" '
    /^\[\[package\]\]/ { in_pkg=0; name=""; version=""; next }
    /^name = "/ {
      name=$0; sub(/^name = "/, "", name); sub(/"$/, "", name)
      in_pkg=(name==dep)
      next
    }
    /^version = "/ && in_pkg {
      version=$0; sub(/^version = "/, "", version); sub(/"$/, "", version)
      if (version==ver) { found=1; exit 0 }
    }
    END { if (!found) exit 1 }
  ' Cargo.lock
}

pin_exact_transition() {
  local dep="$1"
  local from_ver="$2"
  local to_ver="$3"
  local tmp

  if ! lock_has_pkg_version "$dep" "$from_ver"; then
    echo "[pin] skip: ${dep}@${from_ver} not present"
    return 0
  fi

  tmp="$(mktemp)"
  if cargo update -p "${dep}@${from_ver}" --precise "$to_ver" >"$tmp" 2>&1; then
    rm -f "$tmp"
    echo "[pin] ${dep}@${from_ver} -> ${to_ver}"
    return 0
  fi
  echo "[pin] failed: ${dep}@${from_ver} -> ${to_ver}" >&2
  sed -n '1,40p' "$tmp" >&2 || true
  rm -f "$tmp"
  return 1
}

fallback_force_pin() {
  echo "[repair] no compatible lock commit found; attempting force-pin fallback..."
  local ok=0

  # First eliminate the 0.4.x getrandom chain that pulls wasip3 -> wit-bindgen 0.51.
  pin_exact_transition "uuid" "1.21.0" "1.11.0" || ok=1
  pin_exact_transition "getrandom" "0.4.1" "0.3.4" || ok=1
  pin_exact_transition "wasip2" "1.0.2+wasi-0.2.9" "1.0.1+wasi-0.2.4" || ok=1
  pin_exact_transition "tempfile" "3.26.0" "3.12.0" || ok=1

  # Then pin known Rust 1.82-safe packages.
  pin_exact_transition "comfy-table" "7.2.2" "7.1.3" || ok=1
  pin_exact_transition "blake3" "1.8.3" "1.5.5" || ok=1
  pin_exact_transition "constant_time_eq" "0.4.2" "0.3.1" || ok=1
  pin_exact_transition "constant_time_eq" "0.4.1" "0.3.1" || ok=1
  pin_exact_transition "globset" "0.4.18" "0.4.15" || ok=1
  pin_exact_transition "globset" "0.4.19" "0.4.15" || ok=1
  pin_exact_transition "time" "0.3.47" "0.3.36" || ok=1
  pin_exact_transition "time-core" "0.1.8" "0.1.2" || ok=1
  pin_exact_transition "time-macros" "0.2.27" "0.2.18" || ok=1

  # Keep wit-bindgen family in sync with pinned wasip2.
  pin_exact_transition "wit-bindgen" "0.51.0" "0.50.0" || ok=1
  pin_exact_transition "wit-bindgen-core" "0.51.0" "0.50.0" || ok=1
  pin_exact_transition "wit-bindgen-rust" "0.51.0" "0.50.0" || ok=1
  pin_exact_transition "wit-bindgen-rust-macro" "0.51.0" "0.50.0" || ok=1
  pin_optional "wit-bindgen-rt" "0.50.0"

  # Keep ICU chain Rust-1.82 compatible.
  pin_exact_transition "icu_collections" "2.1.1" "2.0.0" || ok=1
  pin_exact_transition "icu_locale_core" "2.1.1" "2.0.0" || ok=1
  pin_exact_transition "icu_normalizer" "2.1.1" "2.0.0" || ok=1
  pin_exact_transition "icu_normalizer_data" "2.1.1" "2.0.0" || ok=1
  pin_exact_transition "icu_properties" "2.1.2" "2.0.0" || ok=1
  pin_exact_transition "icu_properties_data" "2.1.2" "2.0.0" || ok=1
  pin_exact_transition "icu_provider" "2.1.1" "2.0.0" || ok=1

  if [[ "$ok" -ne 0 ]]; then
    echo "ERROR: force-pin fallback did not fully succeed." >&2
    return 1
  fi
  return 0
}

if [[ -z "$TARGET_COMMIT" ]]; then
  echo "[repair] finding latest compatible lockfile commit..."
  TARGET_COMMIT="$(find_compatible_commit || true)"
fi

if [[ -z "$TARGET_COMMIT" ]]; then
  if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[repair] dry-run: no compatible commit found; would attempt force-pin fallback."
    exit 0
  fi
  fallback_force_pin
  echo "[repair] verifying toolchain policy..."
  ./scripts/verify_toolchain_policy.sh
  if [[ "$RUN_GATE" -eq 1 ]]; then
    echo "[repair] running full pre-push gate..."
    ./scripts/prepush_gate.sh
  fi
  echo "[repair] lockfile repair complete (force-pin fallback)."
  exit 0
fi

if ! commit_has_compatible_core_lock "$TARGET_COMMIT"; then
  echo "ERROR: Commit '$TARGET_COMMIT' is not Rust 1.82 lockfile-compatible." >&2
  exit 1
fi

echo "[repair] selected commit: $TARGET_COMMIT"
git --no-pager log --oneline -n 1 "$TARGET_COMMIT"

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "[repair] dry-run mode: no files changed."
  exit 0
fi

echo "[repair] restoring Cargo.lock from selected commit..."
git checkout "$TARGET_COMMIT" -- Cargo.lock

if git cat-file -e "${TARGET_COMMIT}:Cargo.lock.packaging" 2>/dev/null; then
  echo "[repair] restoring Cargo.lock.packaging from selected commit..."
  git checkout "$TARGET_COMMIT" -- Cargo.lock.packaging
else
  echo "[repair] note: selected commit has no Cargo.lock.packaging; keeping current file."
fi

echo "[repair] verifying toolchain policy..."
./scripts/verify_toolchain_policy.sh

if [[ "$RUN_GATE" -eq 1 ]]; then
  echo "[repair] running full pre-push gate..."
  ./scripts/prepush_gate.sh
fi

echo "[repair] lockfile repair complete."
