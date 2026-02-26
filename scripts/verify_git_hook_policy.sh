#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

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

log "[hook-policy] required files exist"
run_check "hooks_prepush_file" test -f .githooks/pre-push || true
run_check "gate_script_file" test -f scripts/prepush_gate.sh || true
run_check "hook_installer_file" test -f scripts/install_git_hooks.sh || true

log "[hook-policy] pre-push calls gate script"
run_check "prepush_calls_gate" search_pattern '^\./scripts/prepush_gate\.sh$' .githooks/pre-push || true

log "[hook-policy] installer configures hooks path"
run_check "installer_sets_hooks_path" search_pattern 'git config core\.hooksPath \.githooks' scripts/install_git_hooks.sh || true
run_check "installer_chmods_hook" search_pattern 'chmod \+x \.githooks/pre-push' scripts/install_git_hooks.sh || true

log "[hook-policy] gate includes mandatory locked compile"
run_check "gate_has_locked_check" search_pattern '^cargo check -p pilot --locked$' scripts/prepush_gate.sh || true

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
  echo "Git hook policy checks passed."
fi
