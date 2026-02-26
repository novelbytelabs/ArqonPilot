#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "ERROR: Not inside a git repository." >&2
  exit 1
fi

chmod +x .githooks/pre-push
git config core.hooksPath .githooks

echo "Installed git hooks:"
echo "  core.hooksPath=$(git config --get core.hooksPath)"
echo "  pre-push -> .githooks/pre-push"
