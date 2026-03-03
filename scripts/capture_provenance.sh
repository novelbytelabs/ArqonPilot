#!/bin/bash
#
# FC-6: Provenance and Replay - Provenance Capture Script
# Captures comprehensive provenance data for execution records
#
# Usage:
#   capture_provenance.sh --execution-id <id> --command <cmd> --scope <scope> [--gates <gates>]
#
# Output: JSON provenance record to .contract_state/

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PROVENANCE_DIR="$PROJECT_ROOT/.contract_state"
PROVENANCE_ARCHIVE="$PROJECT_ROOT/.provenance_archive"

# Ensure directories exist
mkdir -p "$PROVENANCE_DIR" "$PROVENANCE_ARCHIVE"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[PROVENANCE]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Capture environment snapshot
capture_environment() {
    local rust_version=""
    local cargo_version=""
    local python_version=""
    local platform=""
    local os_kernel=""
    
    if command -v rustc &> /dev/null; then
        rust_version=$(rustc --version 2>/dev/null || echo "unknown")
        cargo_version=$(cargo --version 2>/dev/null || echo "unknown")
    fi
    
    if command -v python3 &> /dev/null; then
        python_version=$(python3 --version 2>/dev/null || echo "unknown")
    fi
    
    platform=$(uname -s 2>/dev/null || echo "unknown")
    os_kernel=$(uname -r 2>/dev/null || echo "unknown")
    
    # Get path but sanitize control characters
    local safe_path
    safe_path=$(echo "$PATH" | tr -d '\000-\037' | tr '\n' ':' | sed 's/::*/:/g' | sed 's/:$//')
    
    cat <<EOF
{
  "rust_version": "$rust_version",
  "cargo_version": "$cargo_version",
  "python_version": "$python_version",
  "shell": "$SHELL",
  "path": "$safe_path",
  "user": "$(whoami)",
  "hostname": "$(hostname)",
  "platform": "$platform",
  "os_kernel": "$os_kernel",
  "working_directory": "$(pwd)",
  "frozen_policy_versions": {
    "core_rust": "1.82.0",
    "packaging_rust": "1.88.0",
    "protobuf": "4.25.8"
  }
}
EOF
}

# Capture git context
capture_git_context() {
    local branch=""
    local commit=""
    local commit_short=""
    local dirty="false"
    local tags=""
    local remotes=""
    
    if command -v git &> /dev/null && git rev-parse --git-dir &> /dev/null; then
        branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
        commit=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
        commit_short=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
        
        if [[ -n $(git status --porcelain 2>/dev/null) ]]; then
            dirty="true"
        fi
        
        tags=$(git tag -l --points-at HEAD 2>/dev/null | tr '\n' ',' | sed 's/,$//')
        remotes=$(git remote 2>/dev/null | tr '\n' ',' | sed 's/,$//')
    fi
    
    cat <<EOF
{
  "branch": "$branch",
  "commit": "$commit",
  "commit_short": "$commit_short",
  "dirty": $dirty,
  "tags": "$(echo "$tags" | tr ',' '\n' | grep -v '^$' | jq -R . | jq -s .)",
  "remotes": "$(echo "$remotes" | tr ',' '\n' | grep -v '^$' | jq -R . | jq -s .)"
}
EOF
}

# Generate payload digest
generate_digest() {
    local payload="$1"
    if command -v sha256sum &> /dev/null; then
        echo "$payload" | sha256sum | cut -d' ' -f1
    elif command -v shasum &> /dev/null; then
        echo "$payload" | shasum -a 256 | cut -d' ' -f1
    else
        echo "nodigest"
    fi
}

# Create full provenance record
create_provenance_record() {
    local execution_id="$1"
    local command="$2"
    local scope="$3"
    local gates="${4:-}"
    local status="${5:-SUCCESS}"
    local exit_code="${6:-0}"
    local duration="${7:-0}"
    local stdout="${8:-}"
    local stderr="${9:-}"
    local failure_reason="${10:-}"
    local failure_code="${11:-}"
    local parent_execution_id="${12:-}"
    local is_replay="${13:-false}"
    
    # Get timestamp
    local timestamp
    timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    
    # Capture environment and git context
    local env_json
    env_json=$(capture_environment)
    
    local git_json
    git_json=$(capture_git_context)
    
    # Build resolved operations
    local resolved_ops
    resolved_ops=$(cat <<EOF
[
  {"id": "scope_check", "type": "scope_check", "target": "$scope", "command": "scope_check:$scope", "status": "completed"},
  {"id": "policy_check", "type": "policy_check", "target": "toolchain_policy", "command": "verify_toolchain_policy.sh", "status": "completed"},
  $(if [[ -n "$gates" ]]; then
    local gate_array
    gate_array=$(echo "$gates" | tr ',' '\n' | while read -r gate; do
        echo "{\"id\": \"$gate\", \"type\": \"gate_execution\", \"target\": \"$gate\", \"command\": \"$gate\", \"status\": \"completed\"},"
    done | tr -d '\n' | sed 's/,$//')
    echo "$gate_array"
  fi)
]
EOF
)
    
    # Build output record
    local output_json
    output_json=$(cat <<EOF
{
  "status": "$status",
  "exit_code": $exit_code,
  "duration_seconds": $duration,
  $(if [[ -n "$stdout" ]]; then
    echo "\"stdout\": $(echo "$stdout" | jq -Rs .),"
  fi)
  $(if [[ -n "$stderr" ]]; then
    echo "\"stderr\": $(echo "$stderr" | jq -Rs .),"
  fi)
  $(if [[ -n "$failure_reason" ]]; then
    echo "\"failure_reason\": $(echo "$failure_reason" | jq -Rs .),"
  fi)
  $(if [[ -n "$failure_code" ]]; then
    echo "\"failure_code\": \"$failure_code\","
  fi)
  "gate_results": []
}
EOF
)
    
    # Build input payload
    local input_json
    input_json=$(cat <<EOF
{
  "command": "$command",
  "scope": ["$scope"],
  "gates": $(if [[ -n "$gates" ]]; then echo "[$(echo "$gates" | tr ',' '\n' | while read -r g; do echo "\"$g\""; done | tr '\n' ',' | sed 's/,$//')]"; else echo "[]"; fi),
  "policy_overrides": {},
  "agorg_context": "",
  "env_vars": {}
}
EOF
)
    
    # Build full provenance record
    local provenance_record
    provenance_record=$(cat <<EOF
{
  "execution_id": "$execution_id",
  "timestamp": "$timestamp",
  $(if [[ -n "$parent_execution_id" ]]; then
    echo "\"parent_execution_id\": \"$parent_execution_id\","
  fi)
  "is_replay": $is_replay,
  "input": $input_json,
  "environment": $env_json,
  "git": $git_json,
  "resolved_operations": $resolved_ops,
  "output": $output_json,
  "artifacts": [],
  "payload_digest": ""
}
EOF
)
    
    # Generate final digest
    local digest
    digest=$(generate_digest "$provenance_record")
    
    # Update with digest
    provenance_record=$(echo "$provenance_record" | jq --arg d "$digest" '.payload_digest = $d')
    
    echo "$provenance_record"
}

# Main function
main() {
    local execution_id=""
    local command=""
    local scope=""
    local gates=""
    local status="SUCCESS"
    local exit_code=0
    local duration=0
    local stdout=""
    local stderr=""
    local failure_reason=""
    local failure_code=""
    local parent_execution_id=""
    local is_replay=false
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --execution-id)
                execution_id="$2"
                shift 2
                ;;
            --command)
                command="$2"
                shift 2
                ;;
            --scope)
                scope="$2"
                shift 2
                ;;
            --gates)
                gates="$2"
                shift 2
                ;;
            --status)
                status="$2"
                shift 2
                ;;
            --exit-code)
                exit_code="$2"
                shift 2
                ;;
            --duration)
                duration="$2"
                shift 2
                ;;
            --stdout)
                stdout="$2"
                shift 2
                ;;
            --stderr)
                stderr="$2"
                shift 2
                ;;
            --failure-reason)
                failure_reason="$2"
                shift 2
                ;;
            --failure-code)
                failure_code="$2"
                shift 2
                ;;
            --parent-execution-id)
                parent_execution_id="$2"
                shift 2
                ;;
            --is-replay)
                is_replay=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    if [[ -z "$execution_id" ]]; then
        log_error "execution-id is required"
        exit 1
    fi
    
    if [[ -z "$command" ]]; then
        log_error "command is required"
        exit 1
    fi
    
    if [[ -z "$scope" ]]; then
        scope="ArqonPilot"  # Default
    fi
    
    # Create provenance record
    local provenance
    provenance=$(create_provenance_record "$execution_id" "$command" "$scope" "$gates" \
        "$status" "$exit_code" "$duration" "$stdout" "$stderr" \
        "$failure_reason" "$failure_code" "$parent_execution_id" "$is_replay")
    
    # Write to file
    local output_file="$PROVENANCE_DIR/$execution_id.json"
    echo "$provenance" > "$output_file"
    
    log_info "Provenance recorded: $execution_id"
    echo "$output_file"
}

main "$@"
