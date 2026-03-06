#!/usr/bin/env bash
set -euo pipefail

BRANCH="main"
TIMEOUT_SEC=1200
POLL_SEC=15
REPO_SLUG=""

usage() {
  cat <<'EOF'
Usage: ./scripts/gh_actions_watch_latest.sh [--branch <name>] [--timeout-sec <seconds>] [--poll-sec <seconds>] [--repo <owner/repo>]

Watches the latest GitHub Actions run for a branch until completion.
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

if ! [[ "$TIMEOUT_SEC" =~ ^[0-9]+$ ]] || ! [[ "$POLL_SEC" =~ ^[0-9]+$ ]]; then
  echo "ERROR: timeout/poll values must be numeric" >&2
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

echo "[ci-watch] repo=${REPO_SLUG} branch=${BRANCH} timeout=${TIMEOUT_SEC}s poll=${POLL_SEC}s"

run_id="$(gh run list --repo "$REPO_SLUG" --branch "$BRANCH" --limit 1 --json databaseId --jq '.[0].databaseId' 2>/dev/null || true)"
if [[ -z "$run_id" || "$run_id" == "null" ]]; then
  echo "ERROR: no workflow run found for branch '${BRANCH}'." >&2
  exit 4
fi

echo "[ci-watch] watching run_id=${run_id}"
start_ts="$(date +%s)"
status=""
conclusion=""
workflow=""
run_url=""

while true; do
  status="$(gh run view "$run_id" --repo "$REPO_SLUG" --json status --jq '.status' 2>/dev/null || true)"
  conclusion="$(gh run view "$run_id" --repo "$REPO_SLUG" --json conclusion --jq '.conclusion' 2>/dev/null || true)"
  workflow="$(gh run view "$run_id" --repo "$REPO_SLUG" --json workflowName --jq '.workflowName' 2>/dev/null || true)"
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

failed_jobs_count="$(gh run view "$run_id" --repo "$REPO_SLUG" --json jobs --jq '[.jobs[] | select(.conclusion != "success" and .conclusion != "skipped" and .conclusion != null)] | length' 2>/dev/null || echo "0")"
failed_jobs_names="$(gh run view "$run_id" --repo "$REPO_SLUG" --json jobs --jq '[.jobs[] | select(.conclusion != "success" and .conclusion != "skipped" and .conclusion != null) | .name] | join(\", \")' 2>/dev/null || echo "")"
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
echo "======================================"

exit "$exit_code"
