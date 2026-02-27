#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

check_absent() {
  local pattern="$1"
  if grep -F -R -n "$pattern" "$ROOT/crates/pilot" >/tmp/wave0_rg.txt 2>&1; then
    echo "[FAIL] Found forbidden pattern: $pattern"
    cat /tmp/wave0_rg.txt
    exit 1
  fi
  echo "[OK] Absent: $pattern"
}

check_present() {
  local pattern="$1"
  local file="$2"
  if ! grep -F -n "$pattern" "$file" >/tmp/wave0_rg.txt 2>&1; then
    echo "[FAIL] Missing required pattern '$pattern' in $file"
    cat /tmp/wave0_rg.txt
    exit 1
  fi
  echo "[OK] Present: $pattern in $file"
}

# Legacy project path ".arqon/config.toml" must not remain after rename.
# Note: managed runtime paths under "~/.arqon/pilot" are now valid and expected.
check_absent ".arqon/config.toml"
check_absent "#[command(name = \"arqon\")]"
check_absent "mod ship;"
check_absent "Commands::Ship"
check_absent "crates/core/Cargo.toml"

check_present "#[command(name = \"pilot\")]" "$ROOT/crates/pilot/src/main.rs"
check_present "Navigate(NavigateArgs)" "$ROOT/crates/pilot/src/main.rs"
check_present "Oracle(OracleArgs)" "$ROOT/crates/pilot/src/main.rs"
check_present ".pilot/config.toml" "$ROOT/crates/pilot/src/main.rs"

echo "Wave 0 safety checks passed."
