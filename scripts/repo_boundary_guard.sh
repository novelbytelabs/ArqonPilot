#!/usr/bin/env bash
set -euo pipefail

# Fail fast if this script is executed outside the ArqonPilot repo root.
root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$root" ]]; then
  echo "[boundary] ERROR: not inside a git repository" >&2
  exit 2
fi
base="$(basename "$root")"
if [[ "$base" != "ArqonPilot" ]]; then
  echo "[boundary] ERROR: wrong repository root: $root" >&2
  echo "[boundary] expected repository: ArqonPilot" >&2
  echo "[boundary] aborting to prevent cross-repo drift" >&2
  exit 3
fi

echo "[boundary] OK: repository root is $root"
