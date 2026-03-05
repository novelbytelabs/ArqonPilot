#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$ROOT" ]]; then
  echo "[discipline] ERROR: not inside a git repository" >&2
  exit 2
fi

"$ROOT/scripts/repo_boundary_guard.sh" >/dev/null

UI_PORT="${PILOT_UI_PORT:-7788}"
BASE_URL="http://127.0.0.1:${UI_PORT}"
TREE_URL="${BASE_URL}/api/agorg/tree"
SCOPE_URL="${BASE_URL}/api/agorg/scope_snapshot"
TMP_TREE="$(mktemp)"
TMP_SCOPE="$(mktemp)"
trap 'rm -f "$TMP_TREE" "$TMP_SCOPE"' EXIT

fetch_with_retry() {
  local url="$1"
  local out="$2"
  local max_attempts="${3:-8}"
  local delay_sec="${4:-1}"
  local attempt
  for ((attempt = 1; attempt <= max_attempts; attempt++)); do
    if curl -fsS "$url" >"$out" 2>/dev/null; then
      return 0
    fi
    if (( attempt < max_attempts )); then
      sleep "$delay_sec"
    fi
  done
  return 1
}

if ! fetch_with_retry "$SCOPE_URL" "$TMP_SCOPE"; then
  echo "[discipline] ERROR: Pilot UI API is unavailable on ${BASE_URL}" >&2
  echo "[discipline] remediation:" >&2
  echo "  1) Start UI: cargo run -p pilot -- serve --ws-url ws://127.0.0.1:9100 --room pilot --channel control --telemetry-channel telemetry --ui-port ${UI_PORT} --ui-allow-mutations" >&2
  echo "  2) Select active AGOrg via header chip (AGOrg: ...)" >&2
  echo "  3) Re-run: ./scripts/prepush_gate.sh" >&2
  exit 3
fi

if ! fetch_with_retry "$TREE_URL" "$TMP_TREE"; then
  echo "[discipline] ERROR: failed to query AGOrg tree from ${TREE_URL}" >&2
  exit 4
fi

python3 - "$ROOT" "$TMP_SCOPE" "$TMP_TREE" <<'PY'
import json
import os
import sys
from pathlib import Path

repo_root = Path(sys.argv[1]).resolve()
scope_path = Path(sys.argv[2])
tree_path = Path(sys.argv[3])

try:
    scope = json.loads(scope_path.read_text())
except Exception as exc:
    print(f"[discipline] ERROR: invalid scope snapshot JSON: {exc}", file=sys.stderr)
    sys.exit(5)

try:
    tree = json.loads(tree_path.read_text())
except Exception as exc:
    print(f"[discipline] ERROR: invalid AGOrg tree JSON: {exc}", file=sys.stderr)
    sys.exit(6)

active = (scope or {}).get("active") or {}
active_id = active.get("id")
active_name = active.get("name")
active_root = active.get("root_path")
active_master = active.get("master_path")

if not active_id:
    print("[discipline] ERROR: no active AGOrg scope selected", file=sys.stderr)
    print("[discipline] remediation:", file=sys.stderr)
    print("  1) Open UI on :7788 and select AGOrg in header chip.", file=sys.stderr)
    print("  2) Confirm /api/agorg/scope_snapshot returns non-null active.id.", file=sys.stderr)
    sys.exit(7)

items = (tree or {}).get("tree") or []
active_node = None
for item in items:
    agorg = (item or {}).get("agorg") or {}
    if agorg.get("id") == active_id:
        active_node = item
        break

if not active_node:
    print(f"[discipline] ERROR: active AGOrg id {active_id} not present in tree payload", file=sys.stderr)
    sys.exit(8)

agos = active_node.get("agos") or []
repo_norm = repo_root.as_posix().rstrip("/")
matched = None
for ago in agos:
    repo_path = (ago or {}).get("repo_path")
    if not repo_path:
        continue
    try:
        ago_norm = Path(repo_path).resolve().as_posix().rstrip("/")
    except Exception:
        continue
    if ago_norm == repo_norm:
        matched = ago
        break

if not matched:
    print("[discipline] ERROR: current repo is not registered as an AGO under active AGOrg", file=sys.stderr)
    print(f"[discipline] repo:   {repo_norm}", file=sys.stderr)
    print(f"[discipline] agorg:  {active_name} ({active_id})", file=sys.stderr)
    print(f"[discipline] root:   {active_root}", file=sys.stderr)
    print(f"[discipline] master: {active_master}", file=sys.stderr)
    print("[discipline] remediation:", file=sys.stderr)
    print("  1) Open AGOrg panel and import/register this repo as AGO.", file=sys.stderr)
    print("  2) Ensure active scope is the AGOrg that owns this AGO.", file=sys.stderr)
    print("  3) Re-run: ./scripts/prepush_gate.sh", file=sys.stderr)
    sys.exit(9)

ago_name = matched.get("name", "unknown")
print(f"[discipline] PASS: active_agorg={active_name} ago={ago_name} repo={repo_norm}")
PY
