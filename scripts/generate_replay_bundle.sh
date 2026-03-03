#!/bin/bash
#
# FC-6: Provenance and Replay - Replay Bundle Generator
# Generates deterministic replay bundles for failed and successful runs
#
# Usage:
#   generate_replay_bundle.sh --execution-id <id> [--output-dir <dir>]
#
# Output: Replay bundle with provenance + replay script + dependencies

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PROVENANCE_DIR="$PROJECT_ROOT/.contract_state"
PROVENANCE_ARCHIVE="$PROJECT_ROOT/.provenance_archive"

# Ensure directories exist
mkdir -p "$PROVENANCE_ARCHIVE"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[REPLAY]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_success() { echo -e "${GREEN}[OK]${NC} $*"; }

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

# Generate replay script from provenance
generate_replay_script() {
    local provenance_file="$1"
    local command scope gates
    
    # Handle null values gracefully - use 'run' as default
    command=$(jq -r 'try (.input.command // .command // "run") catch "run"' "$provenance_file" 2>/dev/null || echo "run")
    if [[ -z "$command" ]]; then
        command="run"
    fi
    
    scope=$(jq -r 'try (.input.scope[0] // .scope // "ArqonPilot") catch "ArqonPilot"' "$provenance_file" 2>/dev/null || echo "ArqonPilot")
    if [[ -z "$scope" ]]; then
        scope="ArqonPilot"
    fi
    
    gates=$(jq -r 'try (.input.gates[0] // .input.gates // .gates // "") catch ""' "$provenance_file" 2>/dev/null || echo "")
    if [[ "$gates" == "" ]]; then
        gates=$(jq -r 'try (.gates[0] // .gates // "") catch ""' "$provenance_file" 2>/dev/null || echo "")
    fi
    
    log_info "Replaying command: $command, scope: $scope, gates: $gates"
    
    cat <<EOF
#!/bin/bash
#
# Replay Bundle Script
# Generated from execution: $(basename "$provenance_file" .json)
#
# This script will reproduce the execution with the same parameters
set -euo pipefail

echo "=== Replay Execution ==="
echo "Original execution: $(basename "$provenance_file" .json)"
echo "Command: $command"
echo "Scope: $scope"
echo "Gates: $gates"
echo ""

SCRIPT_DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"

# Run the command with same parameters
case "$command" in
    run)
        if [[ -n "$gates" ]]; then
            echo "Running: ci_run.sh --scope $scope --gates $gates"
            "\$SCRIPT_DIR/ci_run.sh" --scope "$scope" --gates "$gates"
        else
            echo "Running: ci_run.sh --scope $scope"
            "\$SCRIPT_DIR/ci_run.sh" --scope "$scope"
        fi
        ;;
    repair)
        echo "Running: ci_repair.sh --type $gates"
        "\$SCRIPT_DIR/ci_repair.sh" --type "$gates" --force
        ;;
    readiness)
        echo "Running: ci_readiness.sh --scope $scope"
        "\$SCRIPT_DIR/ci_readiness.sh" --scope "$scope"
        ;;
    report)
        echo "Running: ci_report.sh"
        "\$SCRIPT_DIR/ci_report.sh"
        ;;
    *)
        echo "Unknown command: $command, defaulting to run"
        "\$SCRIPT_DIR/ci_run.sh" --scope "$scope"
        ;;
esac

echo ""
echo "=== Replay Complete ==="
EOF
}

# Collect dependencies for replay
collect_dependencies() {
    local provenance_file="$1"
    local deps_file="$2"
    
    # Collect key files
    {
        echo "$PROVENANCE_DIR/$(basename "$provenance_file")"
        
        # Add scripts referenced in resolved operations
        jq -r '.resolved_operations[].command' "$provenance_file" 2>/dev/null | while read -r cmd; do
            if [[ -f "$SCRIPT_DIR/$cmd.sh" ]]; then
                echo "$SCRIPT_DIR/$cmd.sh"
            fi
        done
        
        # Add schema files
        ls "$PROJECT_ROOT/schemas/"*.json 2>/dev/null || true
    } > "$deps_file"
}

# Generate signature for replay verification
generate_signature() {
    local provenance_file="$1"
    local content
    content=$(cat "$provenance_file")
    generate_digest "$content"
}

# Main function
main() {
    local execution_id=""
    local output_dir="$PROVENANCE_ARCHIVE"
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --execution-id)
                execution_id="$2"
                shift 2
                ;;
            --output-dir)
                output_dir="$2"
                shift 2
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
    
    local provenance_file="$PROVENANCE_DIR/$execution_id.json"
    
    if [[ ! -f "$provenance_file" ]]; then
        log_error "Provenance file not found: $provenance_file"
        exit 1
    fi
    
    # Create bundle directory
    local bundle_name="replay-$execution_id"
    local bundle_dir="$output_dir/$bundle_name"
    mkdir -p "$bundle_dir"
    
    log_info "Generating replay bundle: $bundle_name"
    
    # Generate replay script
    local replay_script="$bundle_dir/replay.sh"
    generate_replay_script "$provenance_file" > "$replay_script"
    chmod +x "$replay_script"
    
    # Generate dependencies list
    local deps_file="$bundle_dir/dependencies.txt"
    collect_dependencies "$provenance_file" "$deps_file"
    
    # Copy provenance record
    cp "$provenance_file" "$bundle_dir/provenance.json"
    
    # Generate signature
    local signature
    signature=$(generate_signature "$provenance_file")
    echo "$signature" > "$bundle_dir/signature.txt"
    
    # Create metadata
    cat > "$bundle_dir/bundle.json" <<EOF
{
  "execution_id": "$execution_id",
  "bundle_path": "$bundle_dir",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "signature": "$signature",
  "replay_script": "replay.sh",
  "provenance": "provenance.json",
  "dependencies_file": "dependencies.txt"
}
EOF
    
    # Create tarball
    local tarball="$output_dir/${bundle_name}.tar.gz"
    tar -czf "$tarball" -C "$output_dir" "$bundle_name"
    
    log_success "Replay bundle created: $tarball"
    echo ""
    echo "Bundle contents:"
    echo "  - replay.sh       : Reproduces the execution"
    echo "  - provenance.json : Complete execution record"
    echo "  - signature.txt   : SHA256 for verification"
    echo "  - dependencies.txt: Files needed for replay"
    echo ""
    echo "To replay: tar -xzf $tarball && cd $bundle_name && ./replay.sh"
}

main "$@"
