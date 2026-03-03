#!/bin/bash
set -euo pipefail

# verify_policy_parity.sh
# Tests: pilot policy set-draft -> pilot policy get
# Must be deterministic and schema-correct.

export PILOT_DB_PORT=9340
export PILOT_DB_MODE=unix_socket
echo "=== Phase P1 Remediation: Policy Parity Verification (Isolated Port 9340) ==="

PILOT_BIN="./scripts/pilot_local.sh"
mkdir -p .pilot_test_tmp
TEMP_DIR=$(mktemp -d ".pilot_test_tmp/parity.XXXXXX")
trap 'rm -rf "$TEMP_DIR"' EXIT
export PILOT_HOME="$PWD/.p1db"
mkdir -p "$PILOT_HOME"

ORG_NAME="RemediationOrg-$(basename "$TEMP_DIR")"
REPO_ROOT="$TEMP_DIR/mock-repo"
mkdir -p "$REPO_ROOT"

echo "Using AGOrg: $ORG_NAME with root: $REPO_ROOT"

# Preflight DB startup in this runtime. Some constrained environments
# deny unix socket/shared memory operations; skip parity in that case.
DB_PRECHECK_ERR="$TEMP_DIR/db_precheck.err"
if ! $PILOT_BIN db start > /dev/null 2> "$DB_PRECHECK_ERR"; then
    if grep -Eq "Operation not permitted|could not open shared memory segment|Permission denied" "$DB_PRECHECK_ERR"; then
        echo "[SKIP] verify_policy_parity: runtime denied managed Postgres socket/shared-memory operations."
        echo "       This is an environment constraint, not a policy parity regression."
        exit 0
    fi
    echo "[FAILURE] verify_policy_parity: unable to start managed Postgres."
    cat "$DB_PRECHECK_ERR"
    exit 1
fi
$PILOT_BIN db stop > /dev/null 2>&1 || true

# 1. Setup deterministic test AGOrg
# Hide output but fail on error
$PILOT_BIN agorg create --name "$ORG_NAME" --root "$REPO_ROOT" > /dev/null 2>&1 || true
$PILOT_BIN agorg use "$ORG_NAME" > /dev/null

# 2. Define schema-correct payloads matching model.rs structs
KINDS=("branch" "dependency" "release" "security" "quality" "runtime")

for KIND in "${KINDS[@]}"; do
    echo "--- Testing $KIND policy round-trip ---"
    
    POLICY_FILE="$TEMP_DIR/policy_${KIND}.json"
    
    case $KIND in
        branch)
            cat <<EOF > "$POLICY_FILE"
{
  "kind": "branch",
  "version": 1,
  "naming": {
    "level": "block",
    "required_prefix": ["feat", "fix", "docs", "test"],
    "separator": "/",
    "body_format": "kebab-case",
    "max_length": 80
  },
  "protected_branches": {
    "level": "block",
    "patterns": ["main", "master"],
    "confirmation_type": "typed_phrase",
    "confirmation_phrase": "CONFIRM"
  },
  "lifecycle": {
    "auto_prune_merged": { "level": "off", "enabled": false },
    "prune_requires_confirmation": true,
    "confirmation_phrase": "PRUNE",
    "prune_confirmation_type": "typed_phrase",
    "max_stale_days": { "level": "off", "days": 30 }
  },
  "sync": {
    "strategy": "ff-only",
    "auto_fetch_before_sync": true
  },
  "create": {
    "require_preview": true,
    "base_branch_default": "main"
  }
}
EOF
            ;;
        dependency)
            cat <<EOF > "$POLICY_FILE"
{
  "kind": "dependency",
  "version": 1,
  "allowed_registries": { "level": "off", "items": [] },
  "banned_packages": { "level": "block", "items": ["left-pad"] },
  "allowed_licenses": { "level": "warn", "items": ["MIT", "Apache-2.0"] },
  "require_lockfile": { "level": "block", "enabled": true }
}
EOF
            ;;
        release)
            cat <<EOF > "$POLICY_FILE"
{
  "kind": "release",
  "version": 1,
  "require_changelog": { "level": "block", "enabled": true },
  "require_semver": { "level": "block", "enabled": true },
  "version_strategy": "semver",
  "allowed_channels": { "level": "block", "items": ["alpha", "beta", "stable"] },
  "forbidden_days": { "level": "warn", "items": ["Friday", "Saturday", "Sunday"] }
}
EOF
            ;;
        security)
            cat <<EOF > "$POLICY_FILE"
{
  "kind": "security",
  "version": 1,
  "max_cve_severity": "critical",
  "block_naked_secrets": { "level": "block", "enabled": true }
}
EOF
            ;;
        quality)
            cat <<EOF > "$POLICY_FILE"
{
  "kind": "quality",
  "version": 1,
  "require_lint_pass": { "level": "warn", "enabled": true },
  "require_format_pass": { "level": "warn", "enabled": true },
  "require_coverage": { "level": "off", "enabled": false },
  "min_test_coverage": 0.0
}
EOF
            ;;
        runtime)
            cat <<EOF > "$POLICY_FILE"
{
  "kind": "runtime",
  "version": 1,
  "require_dockerfile": { "level": "off", "enabled": false },
  "require_healthcheck": { "level": "off", "enabled": false },
  "allowed_base_images": { "level": "block", "items": ["alpine:3.18", "ubuntu:22.04"] }
}
EOF
            ;;
    esac

    # Set draft
    echo "Action: set-draft $KIND"
    $PILOT_BIN policy set-draft --kind "$KIND" --file "$POLICY_FILE" > /dev/null
    
    # Get and compare (canonicalize with jq)
    echo "Action: get $KIND"
    GET_OUT=$($PILOT_BIN policy get --kind "$KIND" | jq -S .)
    EXPECTED=$(jq -S . < "$POLICY_FILE")
    
    if [ "$GET_OUT" == "$EXPECTED" ]; then
        echo "[SUCCESS] $KIND policy parity verified."
    else
        echo "[FAILURE] $KIND policy mismatch!"
        echo "--- EXPECTED ---"
        echo "$EXPECTED"
        echo "--- ACTUAL ---"
        echo "$GET_OUT"
        # Print diff for better actionability
        diff <(echo "$EXPECTED") <(echo "$GET_OUT") || true
        exit 1
    fi
done

echo "========================================"
echo "Policy parity check: ALL PASSED"
echo "========================================"
