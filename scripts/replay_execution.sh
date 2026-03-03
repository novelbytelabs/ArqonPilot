#!/bin/bash
#
# FC-6: Provenance and Replay - One-Command Replay Entry
# One-click/one-command replay from Pilot
#
# Usage:
#   replay_execution.sh <execution-id>
#   replay_execution.sh --latest
#   replay_execution.sh --bundle <tarball-path>

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PROVENANCE_DIR="$PROJECT_ROOT/.contract_state"
PROVENANCE_ARCHIVE="$PROJECT_ROOT/.provenance_archive"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[REPLAY]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_success() { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }

# Find latest execution
find_latest_execution() {
    local latest
    latest=$(ls -t "$PROVENANCE_DIR"/*.json 2>/dev/null | head -1)
    
    if [[ -z "$latest" ]]; then
        log_error "No execution records found"
        exit 1
    fi
    
    basename "$latest" .json
}

# Verify replay bundle integrity
verify_bundle() {
    local bundle_dir="$1"
    local provenance_file="$bundle_dir/provenance.json"
    local signature_file="$bundle_dir/signature.txt"
    
    if [[ ! -f "$provenance_file" ]]; then
        log_error "Missing provenance.json in bundle"
        return 1
    fi
    
    if [[ ! -f "$signature_file" ]]; then
        log_warn "No signature file - skipping verification"
        return 0
    fi
    
    # Verify signature
    local expected_signature
    expected_signature=$(cat "$signature_file")
    
    local actual_signature
    actual_signature=$(sha256sum "$provenance_file" | cut -d' ' -f1)
    
    if [[ "$expected_signature" != "$actual_signature" ]]; then
        log_error "Signature mismatch! Bundle may be corrupted"
        log_error "Expected: $expected_signature"
        log_error "Actual:   $actual_signature"
        return 1
    fi
    
    log_success "Bundle signature verified"
    return 0
}

# Extract and run from tarball
replay_from_bundle() {
    local tarball="$1"
    local temp_dir
    
    temp_dir=$(mktemp -d)
    
    log_info "Extracting bundle: $tarball"
    
    if ! tar -xzf "$tarball" -C "$temp_dir"; then
        log_error "Failed to extract bundle"
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Find the extracted directory
    local bundle_dir
    bundle_dir=$(find "$temp_dir" -type d -name "replay-*" | head -1)
    
    if [[ -z "$bundle_dir" ]]; then
        log_error "Invalid bundle format"
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Verify bundle
    if ! verify_bundle "$bundle_dir"; then
        log_error "Bundle verification failed"
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Run replay
    local replay_script="$bundle_dir/replay.sh"
    
    if [[ ! -x "$replay_script" ]]; then
        chmod +x "$replay_script"
    fi
    
    log_info "Executing replay..."
    echo ""
    
    # Copy to temp location for execution
    local work_dir
    work_dir=$(mktemp -d)
    cp -r "$bundle_dir"/* "$work_dir/"
    
    cd "$work_dir"
    "./replay.sh"
    local result=$?
    
    # Cleanup
    rm -rf "$temp_dir" "$work_dir"
    
    return $result
}

# Replay from stored execution
replay_from_execution() {
    local execution_id="$1"
    local provenance_file="$PROVENANCE_DIR/$execution_id.json"
    
    if [[ ! -f "$provenance_file" ]]; then
        log_error "Execution not found: $execution_id"
        log_info "Available executions:"
        ls -1 "$PROVENANCE_DIR"/*.json 2>/dev/null | xargs -I{} basename {} .json | head -10
        exit 1
    fi
    
    log_info "Loading execution: $execution_id"
    
    # Display provenance summary
    echo ""
    echo "=== Execution Summary ==="
    echo "ID:        $(jq -r '.execution_id' "$provenance_file")"
    echo "Command:   $(jq -r '.input.command' "$provenance_file")"
    echo "Scope:     $(jq -r '.input.scope | join(", ")' "$provenance_file")"
    echo "Gates:     $(jq -r '.input.gates | join(", ")' "$provenance_file")"
    echo "Timestamp: $(jq -r '.timestamp' "$provenance_file")"
    echo "Status:    $(jq -r '.output.status' "$provenance_file")"
    echo ""
    
    # Show environment
    echo "=== Environment ==="
    echo "Rust:      $(jq -r '.environment.rust_version' "$provenance_file")"
    echo "Branch:    $(jq -r '.git.branch' "$provenance_file")"
    echo "Commit:    $(jq -r '.git.commit_short' "$provenance_file")"
    echo ""
    
    # Confirm replay
    echo -n "Proceed with replay? (yes/no): "
    read -r response
    
    if [[ "$response" != "yes" ]]; then
        log_info "Replay cancelled"
        return 0
    fi
    
    # Generate and run replay bundle
    log_info "Generating replay bundle..."
    
    "$SCRIPT_DIR/generate_replay_bundle.sh" --execution-id "$execution_id" --output-dir "$PROVENANCE_ARCHIVE"
    
    # Find generated bundle
    local tarball="$PROVENANCE_ARCHIVE/replay-$execution_id.tar.gz"
    
    if [[ -f "$tarball" ]]; then
        replay_from_bundle "$tarball"
    else
        log_error "Failed to generate replay bundle"
        exit 1
    fi
}

# List available executions
list_executions() {
    echo "=== Available Executions ==="
    echo ""
    
    local count=0
    for f in "$PROVENANCE_DIR"/*.json; do
        if [[ -f "$f" ]]; then
            local id
            id=$(basename "$f" .json)
            local cmd scope timestamp status
            
            cmd=$(jq -r '.input.command' "$f" 2>/dev/null || echo "unknown")
            scope=$(jq -r '.input.scope[0]' "$f" 2>/dev/null || echo "unknown")
            timestamp=$(jq -r '.timestamp' "$f" 2>/dev/null || echo "unknown")
            status=$(jq -r '.output.status' "$f" 2>/dev/null || echo "unknown")
            
            printf "%-40s %-10s %-15s %s\n" "$id" "$cmd" "$scope" "$status"
            ((count++))
        fi
    done
    
    echo ""
    echo "Total: $count executions"
    
    # Also check archives
    if [[ -d "$PROVENANCE_ARCHIVE" ]]; then
        local archive_count
        archive_count=$(ls -1 "$PROVENANCE_ARCHIVE"/*.tar.gz 2>/dev/null | wc -l)
        
        if [[ $archive_count -gt 0 ]]; then
            echo ""
            echo "=== Archived Replay Bundles ==="
            ls -1 "$PROVENANCE_ARCHIVE"/*.tar.gz 2>/dev/null | xargs -I{} basename {} | head -10
        fi
    fi
}

# Main
main() {
    if [[ $# -eq 0 ]]; then
        echo "Usage: $0 <execution-id>"
        echo "       $0 --latest"
        echo "       $0 --bundle <tarball-path>"
        echo "       $0 --list"
        echo ""
        echo "One-command replay entry for Pilot CI"
        exit 1
    fi
    
    case "$1" in
        --latest)
            local latest_id
            latest_id=$(find_latest_execution)
            replay_from_execution "$latest_id"
            ;;
        --list)
            list_executions
            ;;
        --bundle)
            if [[ -z "${2:-}" ]]; then
                log_error "Bundle path required"
                exit 1
            fi
            replay_from_bundle "$2"
            ;;
        --help|-h)
            echo "Usage: $0 <execution-id>"
            echo "       $0 --latest"
            echo "       $0 --bundle <tarball-path>"
            echo "       $0 --list"
            ;;
        *)
            replay_from_execution "$1"
            ;;
    esac
}

main "$@"
