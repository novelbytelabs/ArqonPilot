#!/usr/bin/env bash
set -euo pipefail

BRANCH="main"
REPO_SLUG=""

usage() {
  cat <<'EOF'
Usage: ./scripts/gh_actions_trigger_ci.sh [--branch <name>] [--repo <owner/repo>]

Triggers ArqonPilot CI workflow via workflow_dispatch and prints a parseable summary.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --branch)
      BRANCH="${2:-}"
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

if ! grep -Eq '^[[:space:]]*workflow_dispatch[[:space:]]*:' .github/workflows/ci.yml; then
  echo "ERROR: .github/workflows/ci.yml does not declare workflow_dispatch." >&2
  echo "Add workflow_dispatch to ci.yml to allow CI trigger without a new push." >&2
  exit 4
fi

echo "[ci-trigger] repo=${REPO_SLUG} branch=${BRANCH} workflow=ci.yml"
gh workflow run ci.yml --repo "$REPO_SLUG" --ref "$BRANCH"
sleep 2
run_id="$(gh run list --repo "$REPO_SLUG" --workflow ci.yml --branch "$BRANCH" --limit 1 --json databaseId --jq '.[0].databaseId // ""' 2>/dev/null || true)"
run_url=""
if [[ -n "$run_id" ]]; then
  run_url="https://github.com/${REPO_SLUG}/actions/runs/${run_id}"
fi

echo "========== gh_trigger summary =========="
echo "result:                SUCCESS"
echo "repo:                  ${REPO_SLUG}"
echo "branch:                ${BRANCH}"
echo "workflow:              ci.yml"
echo "run_id:                ${run_id:-unknown}"
echo "run_url:               ${run_url:-unknown}"
echo "======================================"

exit 0
