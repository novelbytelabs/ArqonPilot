#!/usr/bin/env bash
set -euo pipefail

BRANCH="main"
TIMEOUT_SEC=1200
POLL_SEC=15
LOOKBACK_SEC=900
REPO_SLUG=""

usage() {
  cat <<'EOF'
Usage: ./scripts/gh_actions_watch_latest.sh [--branch <name>] [--timeout-sec <seconds>] [--poll-sec <seconds>] [--lookback-sec <seconds>] [--repo <owner/repo>]

Watches a fresh GitHub Actions run for a branch until completion.
Prints a parseable summary block at the end.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --branch)
      BRANCH="${2:-}"
      shift 2
      ;;
    --timeout-sec)
      TIMEOUT_SEC="${2:-}"
      shift 2
      ;;
    --poll-sec)
      POLL_SEC="${2:-}"
      shift 2
      ;;
    --lookback-sec)
      LOOKBACK_SEC="${2:-}"
      shift 2
      ;;
    --repo)
      REPO_SLUG="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if ! [[ "$TIMEOUT_SEC" =~ ^[0-9]+$ ]] || ! [[ "$POLL_SEC" =~ ^[0-9]+$ ]] || ! [[ "$LOOKBACK_SEC" =~ ^[0-9]+$ ]]; then
  echo "ERROR: timeout/poll/lookback values must be numeric" >&2
  exit 2
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "ERROR: GitHub CLI (gh) is not installed." >&2
  exit 127
fi

if ! gh auth status -h github.com >/dev/null 2>&1; then
  echo "ERROR: gh authentication is not configured. Run: gh auth login" >&2
  exit 6
fi

if [[ -z "$REPO_SLUG" ]]; then
  remote_url="$(git config --get remote.origin.url 2>/dev/null || true)"
  if [[ -n "$remote_url" ]]; then
    # Supports both HTTPS and SSH remotes.
    REPO_SLUG="$(echo "$remote_url" | sed -E 's#(git@github.com:|https://github.com/)##; s#\.git$##')"
  fi
fi

if [[ -z "$REPO_SLUG" ]]; then
  REPO_SLUG="$(gh repo view --json nameWithOwner --jq '.nameWithOwner' 2>/dev/null || true)"
fi

if [[ -z "$REPO_SLUG" ]]; then
  echo "ERROR: unable to resolve repository slug (owner/repo)." >&2
  exit 7
fi

start_ts="$(date +%s)"
window_start_ts="$((start_ts - LOOKBACK_SEC))"
window_start_iso="$(date -u -d "@${window_start_ts}" +%Y-%m-%dT%H:%M:%SZ)"
expected_head_sha="$(git rev-parse "${BRANCH}" 2>/dev/null || git rev-parse HEAD 2>/dev/null || true)"

echo "[ci-watch] repo=${REPO_SLUG} branch=${BRANCH} timeout=${TIMEOUT_SEC}s poll=${POLL_SEC}s lookback=${LOOKBACK_SEC}s"
echo "[ci-watch] fresh-run window starts at ${window_start_iso}"
if [[ -n "${expected_head_sha}" ]]; then
  echo "[ci-watch] expected head sha=${expected_head_sha}"
fi

fresh_candidate_ids() {
  gh run list --repo "$REPO_SLUG" --limit 80 --json databaseId,headBranch,headSha,createdAt --jq \
    ".[] | select(.headBranch==\"${BRANCH}\") | select((.createdAt // \"\") >= \"${window_start_iso}\") | .databaseId" 2>/dev/null || true
}

fresh_candidate_ids_by_sha() {
  if [[ -z "${expected_head_sha}" ]]; then
    return 0
  fi
  gh run list --repo "$REPO_SLUG" --limit 80 --json databaseId,headBranch,headSha,createdAt --jq \
    ".[] | select(.headBranch==\"${BRANCH}\") | select((.createdAt // \"\") >= \"${window_start_iso}\") | select((.headSha // \"\") == \"${expected_head_sha}\") | .databaseId" 2>/dev/null || true
}

pick_ci_run_from_ids() {
  local ids="$1"
  local rid ci_job_count
  if [[ -z "$ids" ]]; then
    echo ""
    return 0
  fi
  while read -r rid; do
    [[ -z "$rid" ]] && continue
    ci_job_count="$(gh api "repos/${REPO_SLUG}/actions/runs/${rid}/jobs?per_page=100" --jq '[.jobs[]? | select(((.name // "")|ascii_downcase)=="rust" or ((.name // "")|ascii_downcase)=="ui-smoke" or ((.name // "")|ascii_downcase)=="packaging-parity")] | length' 2>/dev/null || echo "0")"
    if [[ "${ci_job_count:-0}" != "0" ]]; then
      echo "$rid"
      return 0
    fi
  done <<< "$ids"
  echo ""
}

run_id=""
while [[ -z "$run_id" ]]; do
  candidate_ids="$(fresh_candidate_ids_by_sha)"
  run_id="$(pick_ci_run_from_ids "$candidate_ids")"
  if [[ -z "$run_id" ]]; then
    candidate_ids="$(fresh_candidate_ids)"
    run_id="$(pick_ci_run_from_ids "$candidate_ids")"
  fi
  if [[ -n "$run_id" ]]; then
    break
  fi
  elapsed=$(( "$(date +%s)" - start_ts ))
  if (( elapsed >= TIMEOUT_SEC )); then
    break
  fi
  echo "[ci-watch] waiting for fresh CI run on branch=${BRANCH} (elapsed=${elapsed}s)"
  sleep "$POLL_SEC"
done

if [[ -z "$run_id" || "$run_id" == "null" ]]; then
  echo "ERROR: no fresh workflow run found for branch '${BRANCH}' (window >= ${window_start_iso})." >&2
  echo "========== gh_watch summary =========="
  echo "result:                FAIL"
  echo "repo:                  ${REPO_SLUG}"
  echo "branch:                ${BRANCH}"
  echo "run_id:                none"
  echo "workflow:              unknown"
  echo "status:                timeout"
  echo "conclusion:            timed_out"
  echo "failed_jobs:           0"
  echo "failed_job_names:      none"
  echo "run_url:               unknown"
  echo "likely_cause:          no_fresh_run_detected"
  echo "expected_head_sha:     ${expected_head_sha:-unknown}"
  echo "window_start:          ${window_start_iso}"
  echo "======================================"
  exit 124
fi

echo "[ci-watch] watching run_id=${run_id}"
status=""
conclusion=""
workflow=""
run_url=""

while true; do
  status="$(gh run view "$run_id" --repo "$REPO_SLUG" --json status --jq '.status' 2>/dev/null || true)"
  conclusion="$(gh run view "$run_id" --repo "$REPO_SLUG" --json conclusion --jq '.conclusion' 2>/dev/null || true)"
  workflow="$(gh run view "$run_id" --repo "$REPO_SLUG" --json name --jq '.name // "unknown"' 2>/dev/null || true)"
  run_url="$(gh run view "$run_id" --repo "$REPO_SLUG" --json url --jq '.url' 2>/dev/null || true)"
  elapsed=$(( "$(date +%s)" - start_ts ))
  echo "[ci-watch] status=${status:-unknown} conclusion=${conclusion:-pending} elapsed=${elapsed}s"

  if [[ "$status" == "completed" ]]; then
    break
  fi
  if (( elapsed >= TIMEOUT_SEC )); then
    echo "ERROR: timeout waiting for run ${run_id} after ${elapsed}s." >&2
    status="timeout"
    conclusion="timed_out"
    break
  fi
  sleep "$POLL_SEC"
done

failed_jobs_count="$(gh api "repos/${REPO_SLUG}/actions/runs/${run_id}/jobs?per_page=100" --jq '[.jobs[] | select((.conclusion != "success") and (.conclusion != "skipped") and (.conclusion != null))] | length' 2>/dev/null || echo "0")"
failed_jobs_names="$(gh api "repos/${REPO_SLUG}/actions/runs/${run_id}/jobs?per_page=100" --jq '[.jobs[] | select((.conclusion != "success") and (.conclusion != "skipped") and (.conclusion != null)) | .name] | join(\", \")' 2>/dev/null || echo "")"
[[ "$failed_jobs_names" == "null" ]] && failed_jobs_names=""

result="FAIL"
likely_cause="workflow_failure"
exit_code=1
if [[ "$status" == "completed" && "$conclusion" == "success" ]]; then
  result="SUCCESS"
  likely_cause="none"
  exit_code=0
elif [[ "$status" == "timeout" || "$conclusion" == "timed_out" ]]; then
  likely_cause="watch_timeout"
  exit_code=124
elif [[ "$conclusion" == "cancelled" ]]; then
  likely_cause="workflow_cancelled"
  exit_code=125
elif [[ "$conclusion" == "failure" && "$failed_jobs_count" != "0" ]]; then
  likely_cause="job_failures"
  exit_code=1
fi

echo "========== gh_watch summary =========="
echo "result:                ${result}"
echo "repo:                  ${REPO_SLUG}"
echo "branch:                ${BRANCH}"
echo "run_id:                ${run_id}"
echo "workflow:              ${workflow:-unknown}"
echo "status:                ${status:-unknown}"
echo "conclusion:            ${conclusion:-unknown}"
echo "failed_jobs:           ${failed_jobs_count}"
echo "failed_job_names:      ${failed_jobs_names:-none}"
echo "run_url:               ${run_url:-unknown}"
echo "likely_cause:          ${likely_cause}"
echo "expected_head_sha:     ${expected_head_sha:-unknown}"
echo "window_start:          ${window_start_iso}"
echo "======================================"

exit "$exit_code"
