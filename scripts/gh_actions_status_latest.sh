#!/usr/bin/env bash
set -euo pipefail

BRANCH="main"
REPO_SLUG=""

usage() {
  cat <<'EOF'
Usage: ./scripts/gh_actions_status_latest.sh [--branch <name>] [--repo <owner/repo>]

Returns a lightweight status snapshot for latest relevant GitHub Actions runs
(docs + CI jobs: rust/ui-smoke/packaging-parity), in a parseable summary block.
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

map_state() {
  local status="$1"
  local conclusion="$2"
  if [[ "$status" != "completed" ]]; then
    echo "running"
    return
  fi
  if [[ "$conclusion" == "success" || "$conclusion" == "skipped" ]]; then
    echo "pass"
  elif [[ -n "$conclusion" && "$conclusion" != "null" ]]; then
    echo "fail"
  else
    echo "unknown"
  fi
}

docs_state="unknown"
rust_state="unknown"
ui_smoke_state="unknown"
packaging_parity_state="unknown"
overall_state="unknown"
overall_conclusion="unknown"
run_url=""
ci_run_id="unknown"
docs_run_id="unknown"

run_ids="$(gh run list --repo "$REPO_SLUG" --limit 30 --json databaseId,headBranch --jq ".[] | select(.headBranch==\"${BRANCH}\") | .databaseId" 2>/dev/null || true)"
if [[ -z "$run_ids" ]]; then
  # Compatibility fallback for older gh/jq combos.
  run_ids="$(gh run list --repo "$REPO_SLUG" --limit 30 --json databaseId,headBranch --template "{{range .}}{{if eq .headBranch \"${BRANCH}\"}}{{.databaseId}}{{\"\\n\"}}{{end}}{{end}}" 2>/dev/null || true)"
fi

if [[ -n "$run_ids" ]]; then
  while read -r rid; do
    [[ -z "$rid" ]] && continue
    workflow_name="$(gh run view "$rid" --repo "$REPO_SLUG" --json workflowName --jq '.workflowName // ""' 2>/dev/null || true)"
    status="$(gh run view "$rid" --repo "$REPO_SLUG" --json status --jq '.status // ""' 2>/dev/null || true)"
    conclusion="$(gh run view "$rid" --repo "$REPO_SLUG" --json conclusion --jq '.conclusion // ""' 2>/dev/null || true)"
    url="$(gh run view "$rid" --repo "$REPO_SLUG" --json url --jq '.url // ""' 2>/dev/null || true)"

    lower_wf="$(echo "$workflow_name" | tr '[:upper:]' '[:lower:]')"
    if [[ "$docs_run_id" == "unknown" && ( "$lower_wf" == *"mkdocs"* || "$lower_wf" == *"docs"* ) ]]; then
      docs_run_id="$rid"
      docs_state="$(map_state "${status:-unknown}" "${conclusion:-unknown}")"
    fi

    if [[ "$ci_run_id" == "unknown" ]]; then
      rust_job="$(gh api "repos/${REPO_SLUG}/actions/runs/${rid}/jobs?per_page=100" --jq '.jobs[]? | select((.name|ascii_downcase)=="rust") | "\(.status // "")\t\(.conclusion // "")"' 2>/dev/null || true)"
      ui_job="$(gh api "repos/${REPO_SLUG}/actions/runs/${rid}/jobs?per_page=100" --jq '.jobs[]? | select((.name|ascii_downcase)=="ui-smoke") | "\(.status // "")\t\(.conclusion // "")"' 2>/dev/null || true)"
      pkg_job="$(gh api "repos/${REPO_SLUG}/actions/runs/${rid}/jobs?per_page=100" --jq '.jobs[]? | select((.name|ascii_downcase)=="packaging-parity") | "\(.status // "")\t\(.conclusion // "")"' 2>/dev/null || true)"
      if [[ -n "$rust_job" || -n "$ui_job" || -n "$pkg_job" ]]; then
        ci_run_id="$rid"
        run_url="$url"
        overall_state="$(map_state "${status:-unknown}" "${conclusion:-unknown}")"
        overall_conclusion="${conclusion:-unknown}"
        if [[ -n "$rust_job" ]]; then
          rust_state="$(map_state "$(echo "$rust_job" | cut -f1)" "$(echo "$rust_job" | cut -f2)")"
        fi
        if [[ -n "$ui_job" ]]; then
          ui_smoke_state="$(map_state "$(echo "$ui_job" | cut -f1)" "$(echo "$ui_job" | cut -f2)")"
        fi
        if [[ -n "$pkg_job" ]]; then
          packaging_parity_state="$(map_state "$(echo "$pkg_job" | cut -f1)" "$(echo "$pkg_job" | cut -f2)")"
        fi
      fi
    fi

    if [[ "$ci_run_id" != "unknown" && "$docs_run_id" != "unknown" ]]; then
      break
    fi
  done <<< "$run_ids"
fi

# If a CI run is active, default unresolved job chips to "running" instead of "unknown".
if [[ "$ci_run_id" != "unknown" ]]; then
  if [[ "$overall_state" == "running" ]]; then
    [[ "$rust_state" == "unknown" ]] && rust_state="running"
    [[ "$ui_smoke_state" == "unknown" ]] && ui_smoke_state="running"
    [[ "$packaging_parity_state" == "unknown" ]] && packaging_parity_state="running"
  fi
fi

echo "========== gh_status summary =========="
echo "repo:                  ${REPO_SLUG}"
echo "branch:                ${BRANCH}"
echo "ci_run_id:             ${ci_run_id}"
echo "docs_run_id:           ${docs_run_id}"
echo "overall_state:         ${overall_state}"
echo "overall_conclusion:    ${overall_conclusion}"
echo "docs_state:            ${docs_state}"
echo "rust_state:            ${rust_state}"
echo "ui_smoke_state:        ${ui_smoke_state}"
echo "packaging_parity_state: ${packaging_parity_state}"
echo "run_url:               ${run_url:-unknown}"
echo "======================================"

exit 0
