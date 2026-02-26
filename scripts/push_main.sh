#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
START_TS="$(date +%s)"

CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$CURRENT_BRANCH" == "HEAD" ]]; then
  echo "ERROR: detached HEAD; checkout a branch first." >&2
  exit 1
fi

BRANCH="${1:-$CURRENT_BRANCH}"
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
echo "[push-safe] current_branch=$CURRENT_BRANCH"
echo "[push-safe] log file: $LOG_FILE"

count_matches() {
  local pattern="$1"
  local file="$2"
  if command -v rg >/dev/null 2>&1; then
    rg -i -N "$pattern" "$file" 2>/dev/null | wc -l | tr -d ' '
  else
    grep -Ei "$pattern" "$file" 2>/dev/null | wc -l | tr -d ' '
  fi
}

divergence_for() {
  local remote_ref="$1"
  local local_ref="$2"
  if ! git rev-parse --verify "$remote_ref" >/dev/null 2>&1; then
    echo "missing 0 0"
    return 0
  fi
  local counts
  counts="$(git rev-list --left-right --count "${remote_ref}...${local_ref}" 2>/dev/null || echo "0 0")"
  echo "ok ${counts}"
}

summarize_result() {
  local result="$1"
  local push_rc="$2"
  local gate_rc="$3"
  local end_ts duration warn_count err_count
  local before_state after_state before_behind before_ahead after_behind after_ahead
  local likely_cause="none"

  end_ts="$(date +%s)"
  duration="$((end_ts - START_TS))"
  warn_count="$(count_matches 'warning:' "$LOG_FILE")"
  err_count="$(count_matches '(^error:|^fatal:| failed |Could not resolve host|Temporary failure in name resolution|Authentication failed|HTTP/2 401|HTTP/2 403)' "$LOG_FILE")"

  before_state="$(divergence_for "${REMOTE}/${BRANCH}" "${BRANCH}")"
  after_state="$(divergence_for "${REMOTE}/${BRANCH}" "${BRANCH}")"
  read -r _ before_behind before_ahead <<<"$before_state"
  read -r _ after_behind after_ahead <<<"$after_state"

  if grep -Eiq "non-fast-forward|fetch first|failed to push some refs" "$LOG_FILE"; then
    likely_cause="remote_divergence_or_hook_reject"
  fi
  if grep -Eiq "Authentication failed|403|401|denied to" "$LOG_FILE"; then
    likely_cause="auth_or_token"
  fi
  if grep -Eiq "Could not resolve host|Temporary failure in name resolution" "$LOG_FILE"; then
    likely_cause="dns_or_network"
  fi
  if [[ "$gate_rc" -ne 0 ]]; then
    likely_cause="prepush_gate_failed"
  fi

  echo ""
  echo "========== push_main summary =========="
  echo "result:                $result"
  echo "branch:                $BRANCH"
  echo "remote:                $REMOTE"
  echo "duration_sec:          $duration"
  echo "prepush_gate_rc:       $gate_rc"
  echo "git_push_rc:           $push_rc"
  echo "warnings_in_log:       $warn_count"
  echo "errors_in_log:         $err_count"
  echo "divergence_after_push: behind=${after_behind:-0} ahead=${after_ahead:-0}"
  echo "likely_cause:          $likely_cause"
  echo "full_log:              $LOG_FILE"
  echo "======================================="

  if [[ "$result" != "SUCCESS" ]]; then
    echo "[push-safe] top error lines:"
    if command -v rg >/dev/null 2>&1; then
      rg -i -N '(^error:|^fatal:|failed to push some refs|non-fast-forward|Authentication failed|Could not resolve host|Temporary failure in name resolution|HTTP/2 401|HTTP/2 403)' "$LOG_FILE" | tail -n 15 || true
    else
      grep -Ei '(^error:|^fatal:|failed to push some refs|non-fast-forward|Authentication failed|Could not resolve host|Temporary failure in name resolution|HTTP/2 401|HTTP/2 403)' "$LOG_FILE" | tail -n 15 || true
    fi
  fi
}

if [[ "$CURRENT_BRANCH" != "$BRANCH" ]]; then
  echo "ERROR: current branch is not '$BRANCH'." >&2
  echo "Run: git checkout $BRANCH" >&2
  exit 1
fi

echo "[push-safe] fetch remote state"
git fetch "$REMOTE"
git status -sb

echo "[push-safe] pre-push gate"
set +e
./scripts/prepush_gate.sh
gate_rc=$?
set -e
if [[ "$gate_rc" -ne 0 ]]; then
  echo "[push-safe] FAILED (pre-push gate)"
  summarize_result "FAILED" 99 "$gate_rc"
  exit "$gate_rc"
fi

echo "[push-safe] push (verbose diagnostics)"
set +e
GIT_TRACE=1 GIT_CURL_VERBOSE=1 git push "$REMOTE" "${BRANCH}:${BRANCH}"
rc=$?
set -e

if [[ "$rc" -eq 0 ]]; then
  echo "[push-safe] SUCCESS"
  summarize_result "SUCCESS" 0 "$gate_rc"
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

summarize_result "FAILED" "$rc" "$gate_rc"
exit "$rc"
