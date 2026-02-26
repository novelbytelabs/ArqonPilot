#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BRANCH="${1:-dev}"
REMOTE="${2:-origin}"
REPORT_DIR="${PILOT_REPORT_DIR:-$HOME/.pilot/reports}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_FILE="$REPORT_DIR/push_${BRANCH}_${STAMP}.log"

mkdir -p "$REPORT_DIR" 2>/dev/null || true
if ! touch "$LOG_FILE" 2>/dev/null; then
  REPORT_DIR="/tmp/pilot-reports"
  mkdir -p "$REPORT_DIR"
  LOG_FILE="$REPORT_DIR/push_${BRANCH}_${STAMP}.log"
  touch "$LOG_FILE"
fi

exec > >(tee -a "$LOG_FILE") 2>&1

echo "[push-safe] branch=$BRANCH remote=$REMOTE"
echo "[push-safe] log file: $LOG_FILE"

if [[ "$(git rev-parse --abbrev-ref HEAD)" != "$BRANCH" ]]; then
  echo "ERROR: current branch is not '$BRANCH'." >&2
  echo "Run: git checkout $BRANCH" >&2
  exit 1
fi

echo "[push-safe] fetch remote state"
git fetch "$REMOTE"
git status -sb

echo "[push-safe] pre-push gate"
./scripts/prepush_gate.sh

echo "[push-safe] push (verbose diagnostics)"
set +e
GIT_TRACE=1 GIT_CURL_VERBOSE=1 git push "$REMOTE" "${BRANCH}:${BRANCH}"
rc=$?
set -e

if [[ "$rc" -eq 0 ]]; then
  echo "[push-safe] SUCCESS"
  exit 0
fi

echo "[push-safe] FAILED"
echo "[push-safe] quick diagnosis:"
if grep -Eq "non-fast-forward|fetch first|failed to push some refs" "$LOG_FILE"; then
  echo "  - Likely remote divergence. Try: git pull --rebase $REMOTE $BRANCH"
fi
if grep -Eq "Authentication failed|403|401|denied to" "$LOG_FILE"; then
  echo "  - Likely auth/token permission issue. Re-authenticate GitHub credentials."
fi
if grep -Eq "Could not resolve host|Temporary failure in name resolution" "$LOG_FILE"; then
  echo "  - DNS/network issue detected. Retry once network stabilizes."
fi

echo "[push-safe] inspect full log: $LOG_FILE"
exit "$rc"
