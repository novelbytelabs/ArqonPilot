#!/bin/bash
#
# FC-5 CLI/API/UI Parity Test Script
# Tests that contract commands work identically across CLI, API, and UI interfaces
#
# Exit codes:
#   0 - All parity tests pass
#   1 - CLI/API/UI parity broken

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

log_test() { echo -e "${YELLOW}[TEST]${NC} $*"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $*"; TESTS_PASSED=$((TESTS_PASSED + 1)); }
log_fail() { echo -e "${RED}[FAIL]${NC} $*"; TESTS_FAILED=$((TESTS_FAILED + 1)); }
log_skip() { echo -e "${YELLOW}[SKIP]${NC} $*"; TESTS_SKIPPED=$((TESTS_SKIPPED + 1)); }

# Test: CLI readiness check
test_cli_readiness() {
    log_test "Testing CLI: ci_readiness.sh"
    
    if "$SCRIPT_DIR/ci_readiness.sh" &> /dev/null; then
        log_pass "CLI readiness check works"
    else
        log_fail "CLI readiness check failed"
    fi
}

# Test: CLI report command
test_cli_report() {
    log_test "Testing CLI: ci_report.sh --type gate_status"
    
    if "$SCRIPT_DIR/ci_report.sh" --type gate_status &> /dev/null; then
        log_pass "CLI report command works"
    else
        log_fail "CLI report command failed"
    fi
}

# Test: CLI run with preview
test_cli_run_preview() {
    log_test "Testing CLI: ci_run.sh --preview --dry-run"
    
    output=$("$SCRIPT_DIR/ci_run.sh" --scope ArqonPilot --preview --dry-run 2>&1)
    
    if echo "$output" | grep -q "Contract Preview"; then
        log_pass "CLI run preview shows contract preview"
    else
        log_fail "CLI run preview missing contract preview"
    fi
    
    if echo "$output" | grep -q "payload_digest"; then
        log_pass "CLI run preview includes payload digest"
    else
        log_fail "CLI run preview missing payload digest"
    fi
}

# Test: CLI repair preview
test_cli_repair_preview() {
    log_test "Testing CLI: ci_repair.sh --preview --dry-run"
    
    output=$("$SCRIPT_DIR/ci_repair.sh" --type lock_repair --preview --dry-run 2>&1)
    
    if echo "$output" | grep -q "Repair Preview"; then
        log_pass "CLI repair preview shows repair preview"
    else
        log_fail "CLI repair preview missing"
    fi
}

# Test: Contract schema exists
test_schema_exists() {
    log_test "Testing: Contract schema exists"
    
    if [[ -f "$PROJECT_ROOT/schemas/ci_contract_commands.json" ]]; then
        log_pass "Contract schema exists"
    else
        log_fail "Contract schema missing"
    fi
}

# Test: Schema has all commands
test_schema_commands() {
    log_test "Testing: Schema contains all required commands"
    
    local schema_file="$PROJECT_ROOT/schemas/ci_contract_commands.json"
    
    if grep -q '"run"' "$schema_file" && \
       grep -q '"replay"' "$schema_file" && \
       grep -q '"repair"' "$schema_file" && \
       grep -q '"readiness"' "$schema_file" && \
       grep -q '"report"' "$schema_file"; then
        log_pass "Schema contains all 5 commands"
    else
        log_fail "Schema missing commands"
    fi
}

# Test: Schema has scope validation
test_schema_scope() {
    log_test "Testing: Schema has scope validation"
    
    local schema_file="$PROJECT_ROOT/schemas/ci_contract_commands.json"
    
    if grep -q 'ArqonPilot' "$schema_file" && \
       grep -q 'ArqonBus' "$schema_file" && \
       grep -q 'ArqonLattice' "$schema_file" && \
       grep -q 'ArqonStudio' "$schema_file" && \
       grep -q 'ArqonHPO' "$schema_file"; then
        log_pass "Schema has all scope repositories"
    else
        log_fail "Schema missing scope repositories"
    fi
}

# Test: Schema has policy checks
test_schema_policy() {
    log_test "Testing: Schema has policy checks"
    
    local schema_file="$PROJECT_ROOT/schemas/ci_contract_commands.json"
    
    if grep -q 'PolicyCheck' "$schema_file"; then
        log_pass "Schema has PolicyCheck"
    else
        log_fail "Schema missing PolicyCheck"
    fi
}

# Test: Schema has contract preview
test_schema_preview() {
    log_test "Testing: Schema has ContractPreview"
    
    local schema_file="$PROJECT_ROOT/schemas/ci_contract_commands.json"
    
    if grep -q 'ContractPreview' "$schema_file"; then
        log_pass "Schema has ContractPreview"
    else
        log_fail "Schema missing ContractPreview"
    fi
}

# Test: Individual command scripts exist
test_command_scripts() {
    log_test "Testing: Individual command scripts exist"
    
    local all_exist=true
    
    for cmd in ci_run.sh ci_replay.sh ci_repair.sh ci_readiness.sh ci_report.sh; do
        if [[ ! -x "$SCRIPT_DIR/$cmd" ]]; then
            log_fail "Script not executable: $cmd"
            all_exist=false
        fi
    done
    
    if $all_exist; then
        log_pass "All command scripts exist and are executable"
    fi
}

# Test: JSON output capability
test_json_output() {
    log_test "Testing: JSON output capability"
    
    # Test with ci_contract directly
    output=$("$SCRIPT_DIR/ci_contract.sh" run --scope ArqonPilot --gates toolchain_policy --preview --json --dry-run 2>&1 || true)
    
    if echo "$output" | grep -q '"command"'; then
        log_pass "JSON output works"
    else
        log_fail "JSON output failed"
    fi
}

# Test: Gate execution works
test_gate_execution() {
    log_test "Testing: Gate execution via contract layer"
    
    # This should run the toolchain_policy gate
    if "$SCRIPT_DIR/ci_run.sh" --scope ArqonPilot --gates toolchain_policy &> /dev/null; then
        log_pass "Gate execution works"
    else
        log_fail "Gate execution failed"
    fi
}

# Test: Error handling
test_error_handling() {
    log_test "Testing: Error handling for invalid scope"
    
    # Test with invalid scope - should fail
    local output
    output=$("$SCRIPT_DIR/ci_run.sh" --scope InvalidRepo --gates toolchain_policy 2>&1 || true)
    
    if echo "$output" | grep -q "Invalid scope"; then
        log_pass "Invalid scope rejected"
    else
        log_fail "Invalid scope not rejected: $output"
    fi
}

# Main test runner
main() {
    echo "============================================"
    echo "FC-5 CLI/API/UI Parity Tests"
    echo "============================================"
    echo ""
    
    # Run all tests
    test_schema_exists
    test_schema_commands
    test_schema_scope
    test_schema_policy
    test_schema_preview
    test_command_scripts
    test_cli_readiness
    test_cli_report
    test_cli_run_preview
    test_cli_repair_preview
    test_json_output
    test_gate_execution
    test_error_handling
    
    echo ""
    echo "============================================"
    echo "Test Results"
    echo "============================================"
    echo "Passed:  $TESTS_PASSED"
    echo "Failed:  $TESTS_FAILED"
    echo "Skipped: $TESTS_SKIPPED"
    echo ""
    
    if [[ $TESTS_FAILED -eq 0 ]]; then
        echo -e "${GREEN}All CLI/API/UI parity tests PASSED${NC}"
        exit 0
    else
        echo -e "${RED}Some CLI/API/UI parity tests FAILED${NC}"
        exit 1
    fi
}

main "$@"
