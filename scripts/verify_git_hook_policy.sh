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

echo "[hook-policy] required files exist"
test -f .githooks/pre-push
test -f scripts/prepush_gate.sh
test -f scripts/install_git_hooks.sh

echo "[hook-policy] pre-push calls gate script"
search_pattern '^\./scripts/prepush_gate\.sh$' .githooks/pre-push

echo "[hook-policy] installer configures hooks path"
search_pattern 'git config core\.hooksPath \.githooks' scripts/install_git_hooks.sh
search_pattern 'chmod \+x \.githooks/pre-push' scripts/install_git_hooks.sh

echo "[hook-policy] gate includes mandatory locked compile"
search_pattern '^cargo check -p pilot --locked$' scripts/prepush_gate.sh

echo "Git hook policy checks passed."
