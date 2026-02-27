#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LOCK_FILE="${1:-Cargo.lock}"
JSON_MODE=0
if [[ "${2:-}" == "--json" || "${1:-}" == "--json" ]]; then
  JSON_MODE=1
  LOCK_FILE="Cargo.lock"
fi

if [[ ! -f "$LOCK_FILE" ]]; then
  echo "ERROR: lockfile not found: $LOCK_FILE" >&2
  exit 1
fi

awk_extract_version() {
  local crate="$1"
  awk -v target="$crate" '
    /^\[\[package\]\]/ { in_pkg=1; n=""; v=""; next }
    in_pkg && /^name = "/ {
      n=$0; sub(/^name = "/, "", n); sub(/"$/, "", n); next
    }
    in_pkg && /^version = "/ {
      v=$0; sub(/^version = "/, "", v); sub(/"$/, "", v);
      if (n == target) print v;
      in_pkg=0; n=""; v="";
    }
  ' "$LOCK_FILE"
}

first_version() {
  awk_extract_version "$1" | head -n1
}

add_finding() {
  local crate="$1"
  local version="$2"
  local family="$3"
  local reason="$4"
  local fix="$5"
  FINDINGS+=("${crate}|${version}|${family}|${reason}|${fix}")
}

version_matches() {
  local version="$1"
  local regex="$2"
  [[ "$version" =~ $regex ]]
}

FINDINGS=()

check_one() {
  local crate="$1"
  local family="$2"
  local bad_regex="$3"
  local reason="$4"
  local fix="$5"
  while IFS= read -r v; do
    [[ -n "$v" ]] || continue
    if version_matches "$v" "$bad_regex"; then
      add_finding "$crate" "$v" "$family" "$reason" "$fix"
    fi
  done < <(awk_extract_version "$crate")
}

check_one "time" "time/comfy-table/wit-bindgen" '^0\.3\.(4[7-9]|[5-9][0-9])$' \
  "Rust 1.82 incompatible (edition2024 era crate)" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "time-core" "time/comfy-table/wit-bindgen" '^0\.1\.(8|9|[1-9][0-9])$' \
  "Rust 1.82 incompatible (edition2024 era crate)" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "time-macros" "time/comfy-table/wit-bindgen" '^0\.2\.(2[7-9]|[3-9][0-9])$' \
  "Likely Rust 1.82 incompatible with newer time stack" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "comfy-table" "time/comfy-table/wit-bindgen" '^7\.(2|3|4|5|6|7|8|9)\..*$' \
  "Rust 1.82 incompatible (edition2024 era crate)" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "wit-bindgen" "time/comfy-table/wit-bindgen" '^0\.5[1-9]\..*$' \
  "Rust 1.82 incompatible (edition2024 era crate)" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "constant_time_eq" "blake3/constant_time_eq" '^0\.4\..*$' \
  "Rust 1.82 incompatible via blake3 1.8.x chain" \
  "cargo update -p blake3@1.8.3 --precise 1.5.5 && ./scripts/prepush_gate.sh"
check_one "globset" "globset" '^0\.4\.1[8-9].*$' \
  "Rust 1.82 incompatible (edition2024 era crate)" \
  "./scripts/repair_lock_182.sh --no-gate"

check_one "icu_collections" "icu_2.1.x" '^2\.1\..*$' \
  "Rust 1.82 incompatible (requires >=1.83)" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "icu_locale_core" "icu_2.1.x" '^2\.1\..*$' \
  "Rust 1.82 incompatible (requires >=1.83)" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "icu_normalizer" "icu_2.1.x" '^2\.1\..*$' \
  "Rust 1.82 incompatible (requires >=1.83)" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "icu_normalizer_data" "icu_2.1.x" '^2\.1\..*$' \
  "Rust 1.82 incompatible (requires >=1.83)" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "icu_properties" "icu_2.1.x" '^2\.1\..*$' \
  "Rust 1.82 incompatible (requires >=1.83)" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "icu_properties_data" "icu_2.1.x" '^2\.1\..*$' \
  "Rust 1.82 incompatible (requires >=1.83)" \
  "./scripts/repair_lock_182.sh --no-gate"
check_one "icu_provider" "icu_2.1.x" '^2\.1\..*$' \
  "Rust 1.82 incompatible (requires >=1.83)" \
  "./scripts/repair_lock_182.sh --no-gate"

if [[ "$JSON_MODE" -eq 1 ]]; then
  printf '{"ok":%s,"lockfile":"%s","finding_count":%d,"findings":[' \
    "$([[ "${#FINDINGS[@]}" -eq 0 ]] && echo "true" || echo "false")" \
    "$LOCK_FILE" \
    "${#FINDINGS[@]}"
  for i in "${!FINDINGS[@]}"; do
    IFS='|' read -r crate version family reason fix <<<"${FINDINGS[$i]}"
    [[ "$i" -gt 0 ]] && printf ','
    printf '{"crate":"%s","version":"%s","family":"%s","reason":"%s","fix":"%s"}' \
      "$crate" "$version" "$family" "$reason" "$fix"
  done
  printf ']}\n'
  exit 0
fi

echo "[drift] lockfile: $LOCK_FILE"
echo "[drift] known-family scan:"
if [[ "${#FINDINGS[@]}" -eq 0 ]]; then
  echo "PASS: no known Rust-1.82 drift signatures detected."
  exit 0
fi

for row in "${FINDINGS[@]}"; do
  IFS='|' read -r crate version family reason fix <<<"$row"
  echo "- crate=${crate} version=${version}"
  echo "  family=${family}"
  echo "  reason=${reason}"
  echo "  fix=${fix}"
done

echo ""
echo "Suggested immediate flow:"
echo "1) ./scripts/repair_lock_182.sh --no-gate"
echo "2) ./scripts/prepush_gate.sh"
echo "3) ./scripts/push_main.sh"

exit 1
