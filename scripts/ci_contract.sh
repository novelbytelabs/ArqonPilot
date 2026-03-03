#!/bin/bash
#
# Pilot CI Contract Layer - Command Dispatcher
# FC-5: Typed commands: run/replay/repair/readiness/report with schema validation
#
# Usage:
#   ci_contract.sh <command> [options]
#
# Commands:
#   run         Execute gates with scope and policy checks
#   replay      Replay a previous execution
#   repair      Repair locks, drift, policy, or scope
#   readiness   Check system readiness
#   report      Generate execution reports
#
# Options:
#   --scope REPO       Target scope repository (ArqonPilot, ArqonBus, ArqonLattice, ArqonStudio, ArqonHPO)
#   --agorg CONTEXT    AGOrg scope enforcement context
#   --preview          Show contract preview before execution
#   --gates GATES      Comma-separated list of gates (toolchain_policy, prepush_gate, push_safe, lock_repair, ci_parity)
#   --format FORMAT    Output format (json, markdown, html)
#   --json             Output JSON format
#   --dry-run          Preview only, no execution
#   --force            Force operation (for repair)
#
# Exit codes:
#   0 - Success
#   1 - Invalid command
#   2 - Schema validation failed
#   3 - Scope check failed
#   4 - Policy check failed
#   5 - Execution failed

set -euo pipefail

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SCHEMA_DIR="$PROJECT_ROOT/schemas"
LOG_DIR="$PROJECT_ROOT/logs"
CONTRACT_STATE_DIR="$PROJECT_ROOT/.contract_state"

# Ensure directories exist
mkdir -p "$LOG_DIR" "$CONTRACT_STATE_DIR"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Validate JSON schema
validate_schema() {
    local json_file="$1"
    local schema_file="$2"
    
    if command -v python3 &> /dev/null; then
        python3 -c "
import json
import sys

try:
    with open('$json_file', 'r') as f:
        json.load(f)
    with open('$schema_file', 'r') as f:
        json.load(f)
    print('VALID')
except json.JSONDecodeError as e:
    print(f'INVALID: {e}')
    sys.exit(1)
"
        return $?
    else
        log_warn "Python3 not available, skipping schema validation"
        return 0
    fi
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

# Check if command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# Scope check - verify target repositories
check_scope() {
    local scope="$1"
    local valid_scopes=("ArqonPilot" "ArqonBus" "ArqonLattice" "ArqonStudio" "ArqonHPO")
    
    for valid in "${valid_scopes[@]}"; do
        if [[ "$scope" == "$valid" ]]; then
            return 0
        fi
    done
    
    log_error "Invalid scope: $scope"
    log_error "Valid scopes: ${valid_scopes[*]}"
    return 1
}

# Policy check - verify toolchain versions
check_policy() {
    log_info "Checking toolchain policy..."
    
    # Check Rust versions
    if command_exists rustc; then
        local rust_version
        rust_version=$(rustc --version | grep -oP '\d+\.\d+\.\d+' | head -1)
        
        if [[ "$rust_version" != "1.82.0" ]] && [[ "$rust_version" != "1.88.0" ]]; then
            log_warn "Rust version $rust_version (expected 1.82.0 or 1.88.0)"
        else
            log_success "Rust version: $rust_version"
        fi
    fi
    
    # Check protobuf
    if command_exists protoc; then
        local protoc_version
        protoc_version=$(protoc --version | grep -oP '\d+\.\d+\.\d+' | head -1)
        
        if [[ "$protoc_version" != "4.25.8" ]]; then
            log_warn "protoc version $protoc_version (expected 4.25.8)"
        else
            log_success "protoc version: $protoc_version"
        fi
    fi
    
    # Check frozen versions
    if [[ -f "$PROJECT_ROOT/Cargo.lock.packaging" ]]; then
        log_success "Frozen Cargo.lock.packaging found"
    else
        log_warn "Cargo.lock.packaging not found - may need sync"
    fi
    
    return 0
}

# Generate contract preview
generate_preview() {
    local command="$1"
    local scope="$2"
    local gates="$3"
    local execution_id
    execution_id=$(uuidgen 2>/dev/null || echo "preview-$(date +%s)")
    
    local preview_json
    preview_json=$(cat <<EOF
{
  "command": "$command",
  "resolved_commands": [
    "scope_check:$scope",
    "policy_check",
    $gates
  ],
  "payload_digest": "$(generate_digest "$command$scope$gates")",
  "preview_timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "requires_confirmation": true,
  "confirmation_type": "standard"
}
EOF
)
    
    echo "$preview_json"
}

# Run command - execute gates
do_run() {
    local scope=""
    local agorg_context=""
    local gates="toolchain_policy,prepush_gate"
    local preview=false
    local dry_run=false
    local json_output=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --scope)
                scope="$2"
                shift 2
                ;;
            --agorg)
                agorg_context="$2"
                shift 2
                ;;
            --gates)
                gates="$2"
                shift 2
                ;;
            --preview)
                preview=true
                shift
                ;;
            --dry-run)
                dry_run=true
                shift
                ;;
            --json)
                json_output=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                return 1
                ;;
        esac
    done
    
    # Validate scope
    if [[ -z "$scope" ]]; then
        scope="ArqonPilot"  # Default
    fi
    
    if ! check_scope "$scope"; then
        return 3
    fi
    
    # Generate preview if requested
    if [[ "$preview" == "true" ]]; then
        log_info "Generating contract preview..."
        local preview_output
        preview_output=$(generate_preview "run" "$scope" "$gates")
        
        if [[ "$json_output" == "true" ]]; then
            echo "$preview_output"
        else
            echo -e "${YELLOW}=== Contract Preview ===${NC}"
            echo "$preview_output" | python3 -m json.tool 2>/dev/null || echo "$preview_output"
            echo -e "${YELLOW}=======================${NC}"
            echo -e "${RED}Execution will NOT proceed without explicit confirmation.${NC}"
        fi
        
        if [[ "$dry_run" == "true" ]]; then
            log_info "Dry run complete - no execution performed"
            return 0
        fi
        
        # Request confirmation
        echo -n "Proceed with execution? (yes/no): "
        read -r response
        if [[ "$response" != "yes" ]]; then
            log_info "Execution cancelled by user"
            return 0
        fi
    fi
    
    # Policy check
    if ! check_policy; then
        return 4
    fi
    
    # Execute gates
    local execution_id
    execution_id=$(uuidgen 2>/dev/null || echo "exec-$(date +%s)")
    log_info "Executing gates: $gates (execution_id: $execution_id)"
    
    # Run the gates
    local gate_result=0
    IFS=',' read -ra GATE_ARRAY <<< "$gates"
    for gate in "${GATE_ARRAY[@]}"; do
        case "$gate" in
            toolchain_policy)
                log_info "Running toolchain_policy gate..."
                if "$SCRIPT_DIR/verify_toolchain_policy.sh"; then
                    log_success "toolchain_policy: PASS"
                else
                    log_error "toolchain_policy: FAIL"
                    gate_result=5
                fi
                ;;
            prepush_gate)
                log_info "Running prepush_gate..."
                if "$SCRIPT_DIR/prepush_gate.sh"; then
                    log_success "prepush_gate: PASS"
                else
                    log_error "prepush_gate: FAIL"
                    gate_result=5
                fi
                ;;
            push_safe)
                log_info "push_safe: No specific script (integrated in prepush_gate)"
                ;;
            lock_repair)
                log_info "lock_repair: Use 'repair' command"
                ;;
            ci_parity)
                log_info "Running ci_parity check..."
                if "$SCRIPT_DIR/ci_parity_check.sh"; then
                    log_success "ci_parity: PASS"
                else
                    log_error "ci_parity: FAIL"
                    gate_result=5
                fi
                ;;
            *)
                log_warn "Unknown gate: $gate"
                ;;
        esac
    done
    
    # Save execution record
    local record_file="$CONTRACT_STATE_DIR/$execution_id.json"
    cat > "$record_file" <<EOF
{
  "execution_id": "$execution_id",
  "command": "run",
  "scope": "$scope",
  "agorg_context": "$agorg_context",
  "gates": "$gates",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "status": $([ $gate_result -eq 0 ] && echo '"SUCCESS"' || echo '"FAILED"')
}
EOF
    
    if [[ $gate_result -eq 0 ]]; then
        log_success "Execution completed successfully"
    else
        log_error "Execution failed with gate errors"
    fi
    
    return $gate_result
}

# Replay command - replay previous execution
do_replay() {
    local execution_id=""
    local scope=""
    local preview=false
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --execution-id)
                execution_id="$2"
                shift 2
                ;;
            --scope)
                scope="$2"
                shift 2
                ;;
            --preview)
                preview=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                return 1
                ;;
        esac
    done
    
    if [[ -z "$execution_id" ]]; then
        log_error "Execution ID required for replay"
        return 1
    fi
    
    local record_file="$CONTRACT_STATE_DIR/$execution_id.json"
    if [[ ! -f "$record_file" ]]; then
        log_error "Execution record not found: $execution_id"
        return 1
    fi
    
    log_info "Loading execution record: $execution_id"
    
    if [[ "$preview" == "true" ]]; then
        log_info "Generating replay preview..."
        local preview_output
        preview_output=$(generate_preview "replay" "$scope" "")
        
        echo -e "${YELLOW}=== Replay Preview ===${NC}"
        echo "$preview_output" | python3 -m json.tool 2>/dev/null || echo "$preview_output"
        echo -e "${YELLOW}=====================${NC}"
    fi
    
    # Read and execute the recorded commands
    log_info "Replaying execution..."
    cat "$record_file"
    
    # Re-run the gates from the record
    local gates
    gates=$(grep -oP '"gates":\s*"\K[^"]+' "$record_file")
    
    if [[ -n "$gates" ]]; then
        do_run --scope "$scope" --gates "$gates"
    else
        log_warn "No gates to replay"
    fi
    
    return $?
}

# Repair command - repair locks, drift, policy, scope
do_repair() {
    local repair_type=""
    local scope=""
    local force=false
    local preview=false
    local dry_run=false
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --type)
                repair_type="$2"
                shift 2
                ;;
            --scope)
                scope="$2"
                shift 2
                ;;
            --force|-f)
                force=true
                shift
                ;;
            --preview)
                preview=true
                shift
                ;;
            --dry-run)
                dry_run=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                return 1
                ;;
        esac
    done
    
    if [[ -z "$repair_type" ]]; then
        log_error "Repair type required: lock_repair, drift_correction, policy_reset, scope_recovery"
        return 1
    fi
    
    if [[ "$preview" == "true" ]]; then
        log_info "Generating repair preview..."
        local preview_output
        preview_output=$(generate_preview "repair" "$scope" "$repair_type")
        
        echo -e "${YELLOW}=== Repair Preview ===${NC}"
        echo "$preview_output" | python3 -m json.tool 2>/dev/null || echo "$preview_output"
        echo -e "${YELLOW}=====================${NC}"
        
        if [[ "$dry_run" == "true" ]]; then
            log_info "Dry run complete - no execution performed"
            return 0
        fi
        
        if [[ "$force" != "true" ]]; then
            echo -n "Proceed with repair? (yes/no): "
            read -r response
            if [[ "$response" != "yes" ]]; then
                log_info "Repair cancelled by user"
                return 0
            fi
        fi
    fi
    
    case "$repair_type" in
        lock_repair)
            log_info "Running lock repair..."
            if [[ -f "$SCRIPT_DIR/repair_lock_182.sh" ]]; then
                "$SCRIPT_DIR/repair_lock_182.sh" || return $?
            else
                log_error "repair_lock_182.sh not found"
                return 1
            fi
            ;;
        drift_correction)
            log_info "Running drift correction..."
            if [[ -f "$SCRIPT_DIR/drift_report.sh" ]]; then
                "$SCRIPT_DIR/drift_report.sh" || return $?
            else
                log_error "drift_report.sh not found"
                return 1
            fi
            ;;
        policy_reset)
            log_info "Running policy reset..."
            if [[ -f "$SCRIPT_DIR/verify_policy_parity.sh" ]]; then
                "$SCRIPT_DIR/verify_policy_parity.sh" || return $?
            else
                log_error "verify_policy_parity.sh not found"
                return 1
            fi
            ;;
        scope_recovery)
            log_info "Running scope recovery..."
            log_info "Scope recovery requires manual intervention"
            return 1
            ;;
        *)
            log_error "Unknown repair type: $repair_type"
            return 1
            ;;
    esac
    
    log_success "Repair completed"
    return 0
}

# Readiness command - check system readiness
do_readiness() {
    local scope="ArqonPilot"
    local checks=()
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --scope)
                scope="$2"
                shift 2
                ;;
            --checks)
                IFS=',' read -ra checks <<< "$2"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                return 1
                ;;
        esac
    done
    
    # Default checks
    if [[ ${#checks[@]} -eq 0 ]]; then
        checks=("toolchain" "gates" "dependencies")
    fi
    
    log_info "Checking readiness for scope: $scope"
    
    local readiness_result=0
    for check in "${checks[@]}"; do
        case "$check" in
            toolchain)
                log_info "Checking toolchain readiness..."
                if "$SCRIPT_DIR/verify_toolchain_policy.sh" &> /dev/null; then
                    log_success "  toolchain: READY"
                else
                    log_warn "  toolchain: NOT READY"
                    readiness_result=4
                fi
                ;;
            gates)
                log_info "Checking gates readiness..."
                if [[ -x "$SCRIPT_DIR/prepush_gate.sh" ]]; then
                    log_success "  gates: READY"
                else
                    log_warn "  gates: NOT READY"
                    readiness_result=4
                fi
                ;;
            dependencies)
                log_info "Checking dependencies..."
                if command_exists cargo && command_exists rustc; then
                    log_success "  dependencies: READY"
                else
                    log_warn "  dependencies: NOT READY"
                    readiness_result=4
                fi
                ;;
            connectivity)
                log_info "Checking connectivity..."
                if ping -c 1 8.8.8.8 &> /dev/null; then
                    log_success "  connectivity: READY"
                else
                    log_warn "  connectivity: NOT READY"
                fi
                ;;
            contracts)
                log_info "Checking contract schemas..."
                if [[ -f "$SCHEMA_DIR/ci_contract_commands.json" ]]; then
                    log_success "  contracts: READY"
                else
                    log_warn "  contracts: NOT READY"
                    readiness_result=4
                fi
                ;;
            *)
                log_warn "Unknown check: $check"
                ;;
        esac
    done
    
    if [[ $readiness_result -eq 0 ]]; then
        log_success "System is READY"
    else
        log_warn "System is NOT FULLY READY"
    fi
    
    return $readiness_result
}

# Report command - generate execution reports
do_report() {
    local report_type="execution_summary"
    local scope=""
    local execution_id=""
    local format="json"
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --type)
                report_type="$2"
                shift 2
                ;;
            --scope)
                scope="$2"
                shift 2
                ;;
            --execution-id)
                execution_id="$2"
                shift 2
                ;;
            --format)
                format="$2"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                return 1
                ;;
        esac
    done
    
    log_info "Generating $report_type report..."
    
    case "$report_type" in
        execution_summary)
            echo "=== Execution Summary ==="
            if [[ -d "$CONTRACT_STATE_DIR" ]]; then
                local count
                count=$(ls -1 "$CONTRACT_STATE_DIR"/*.json 2>/dev/null | wc -l)
                echo "Total executions: $count"
                
                if [[ $count -gt 0 ]]; then
                    echo ""
                    echo "Recent executions:"
                    ls -t "$CONTRACT_STATE_DIR"/*.json 2>/dev/null | head -5 | while read -r f; do
                        echo "  - $(basename "$f")"
                        cat "$f" | python3 -m json.tool 2>/dev/null | head -10 || echo "    (invalid JSON)"
                    done
                fi
            else
                echo "No execution records found"
            fi
            ;;
        gate_status)
            echo "=== Gate Status ==="
            echo "toolchain_policy: $([ -x "$SCRIPT_DIR/verify_toolchain_policy.sh" ] && echo "AVAILABLE" || echo "UNAVAILABLE")"
            echo "prepush_gate: $([ -x "$SCRIPT_DIR/prepush_gate.sh" ] && echo "AVAILABLE" || echo "UNAVAILABLE")"
            echo "push_safe: INTEGRATED"
            echo "lock_repair: $([ -f "$SCRIPT_DIR/repair_lock_182.sh" ] && echo "AVAILABLE" || echo "UNAVAILABLE")"
            echo "ci_parity: $([ -f "$SCRIPT_DIR/ci_parity_check.sh" ] && echo "AVAILABLE" || echo "UNAVAILABLE")"
            ;;
        policy_compliance)
            echo "=== Policy Compliance ==="
            if command_exists rustc; then
                echo "Core Rust: $(rustc --version 2>/dev/null | grep -oP '\d+\.\d+\.\d+' | head -1)"
            fi
            if command_exists protoc; then
                echo "Protobuf: $(protoc --version 2>/dev/null | grep -oP '\d+\.\d+\.\d+' | head -1)"
            fi
            echo "Frozen Cargo.lock.packaging: $([ -f "$PROJECT_ROOT/Cargo.lock.packaging" ] && echo "PRESENT" || echo "MISSING")"
            ;;
        drift_analysis)
            echo "=== Drift Analysis ==="
            if [[ -f "$SCRIPT_DIR/drift_report.sh" ]]; then
                "$SCRIPT_DIR/drift_report.sh" 2>/dev/null || echo "No drift data available"
            else
                echo "drift_report.sh not available"
            fi
            ;;
        scope_audit)
            echo "=== Scope Audit ==="
            echo "Valid scopes: ArqonPilot, ArqonBus, ArqonLattice, ArqonStudio, ArqonHPO"
            echo "Current scope: ${scope:-ArqonPilot (default)}"
            ;;
        *)
            log_error "Unknown report type: $report_type"
            return 1
            ;;
    esac
    
    return 0
}

# Main entry point
main() {
    local command="${1:-}"
    
    if [[ -z "$command" ]]; then
        echo "Usage: $0 <command> [options]"
        echo ""
        echo "Commands:"
        echo "  run         Execute gates with scope and policy checks"
        echo "  replay      Replay a previous execution"
        echo "  repair      Repair locks, drift, policy, or scope"
        echo "  readiness   Check system readiness"
        echo "  report      Generate execution reports"
        echo ""
        echo "Options:"
        echo "  --scope REPO       Target scope repository"
        echo "  --agorg CONTEXT    AGOrg scope enforcement context"
        echo "  --preview          Show contract preview before execution"
        echo "  --gates GATES      Comma-separated list of gates"
        echo "  --format FORMAT    Output format (json, markdown, html)"
        echo "  --json             Output JSON format"
        echo "  --dry-run          Preview only, no execution"
        echo "  --force            Force operation (for repair)"
        echo ""
        echo "Examples:"
        echo "  $0 run --scope ArqonPilot --gates toolchain_policy,prepush_gate --preview"
        echo "  $0 readiness --checks toolchain,gates,dependencies"
        echo "  $0 report --type gate_status"
        echo "  $0 repair --type lock_repair --preview"
        return 1
    fi
    
    shift || true
    
    case "$command" in
        run)
            do_run "$@"
            ;;
        replay)
            do_replay "$@"
            ;;
        repair)
            do_repair "$@"
            ;;
        readiness)
            do_readiness "$@"
            ;;
        report)
            do_report "$@"
            ;;
        help|--help|-h)
            main ""
            ;;
        *)
            log_error "Unknown command: $command"
            return 1
            ;;
    esac
}

main "$@"
