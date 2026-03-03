#!/usr/bin/env bash
# =============================================================================
# Proactive Preflight Check Script
# =============================================================================
# Purpose: Verify common dependencies and environment conditions BEFORE
#          running gates to prevent predictable failures.
#
# This script performs proactive checks for:
# - Required tools (cargo, rustc, rustup, git, protoc)
# - Required toolchains installed
# - Network connectivity
# - Disk space
# - Lockfile existence
# - Workspace integrity
#
# Usage:
#   ./scripts/preflight_proactive_check.sh [--json]
# =============================================================================

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Output mode
JSON_MODE=0
if [[ "${1:-}" == "--json" ]]; then
  JSON_MODE=1
fi

# Track failures
FAILED_CHECKS=()
WARNINGS=()

log() {
  if [[ "$JSON_MODE" -eq 0 ]]; then
    echo "$@"
  fi
}

json_output() {
  if [[ "$JSON_MODE" -eq 1 ]]; then
    printf '{"ok":%s,"failed_checks":[%s],"warnings":[%s]}\n' \
      "$([[ ${#FAILED_CHECKS[@]} -eq 0 ]] && echo 'true' || echo 'false')" \
      "$(IFS=,; echo "${FAILED_CHECKS[*]}")" \
      "$(IFS=,; echo "${WARNINGS[*]}")"
  fi
}

add_failure() {
  FAILED_CHECKS+=("\"$1\"")
}

add_warning() {
  WARNINGS+=("\"$1\"")
}

# =============================================================================
# Check 1: Required Tools
# =============================================================================
check_required_tools() {
  log "[check] Required tools"
  
  local tools=("cargo" "rustc" "rustup" "git")
  local missing=()
  
  for tool in "${tools[@]}"; do
    if ! command -v "$tool" >/dev/null 2>&1; then
      missing+=("$tool")
      add_failure "missing_tool:$tool"
    fi
  done
  
  if [[ ${#missing[@]} -gt 0 ]]; then
    log "  ❌ Missing tools: ${missing[*]}"
    add_warning "Some required tools are missing"
  else
    log "  ✅ All required tools present"
  fi
}

# =============================================================================
# Check 2: Required Toolchains
# =============================================================================
check_required_toolchains() {
  log "[check] Required toolchains"
  
  source "$ROOT/scripts/frozen_versions.sh"
  
  # Check core Rust toolchain
  if ! rustup toolchain list 2>/dev/null | grep -Fq "$PILOT_CORE_RUST_VERSION"; then
    log "  ❌ Core Rust $PILOT_CORE_RUST_VERSION not installed"
    add_failure "missing_toolchain:core:$PILOT_CORE_RUST_VERSION"
  else
    log "  ✅ Core Rust $PILOT_CORE_RUST_VERSION installed"
  fi
  
  # Check packaging Rust toolchain (optional for local dev)
  if ! rustup toolchain list 2>/dev/null | grep -Fq "$PILOT_PACKAGING_RUST_VERSION"; then
    log "  ⚠️  Packaging Rust $PILOT_PACKAGING_RUST_VERSION not installed"
    add_warning "Packaging toolchain not installed (needed for PyPI lane)"
  else
    log "  ✅ Packaging Rust $PILOT_PACKAGING_RUST_VERSION installed"
  fi
}

# =============================================================================
# Check 3: Protoc
# =============================================================================
check_protoc() {
  log "[check] Protoc"
  
  source "$ROOT/scripts/frozen_versions.sh"
  
  # Check if protoc is available
  if ! command -v protoc >/dev/null 2>&1; then
    log "  ❌ protoc not found"
    add_failure "missing_protoc"
    return
  fi
  
  # Check version
  local protoc_version
  protoc_version=$(protoc --version 2>/dev/null | awk '{print $2}' || echo "unknown")
  
  if [[ "$protoc_version" == "$PILOT_PROTOC_VERSION" ]]; then
    log "  ✅ protoc $protoc_version (pinned)"
  else
    log "  ⚠️  protoc $protoc_version (expected $PILOT_PROTOC_VERSION)"
    add_warning "protoc_version_mismatch:$protoc_version"
  fi
}

# =============================================================================
# Check 4: Network Connectivity
# =============================================================================
check_network() {
  log "[check] Network connectivity"
  
  local hosts=("index.crates.io" "github.com" "pypi.org")
  local failed=()
  
  for host in "${hosts[@]}"; do
    if ! getent hosts "$host" >/dev/null 2>&1; then
      failed+=("$host")
    fi
  done
  
  if [[ ${#failed[@]} -gt 0 ]]; then
    log "  ❌ Network unreachable: ${failed[*]}"
    add_failure "network_unreachable:${failed[*]}"
  else
    log "  ✅ Network connectivity OK"
  fi
}

# =============================================================================
# Check 5: Disk Space
# =============================================================================
check_disk_space() {
  log "[check] Disk space"
  
  local required_mb=500
  local available_mb
  
  available_mb=$(df -m "$ROOT" 2>/dev/null | awk 'NR==2 {print $4}' || echo "0")
  
  if [[ "$available_mb" -lt "$required_mb" ]]; then
    log "  ❌ Low disk space: ${available_mb}MB available"
    add_failure "low_disk_space:${available_mb}MB"
  else
    log "  ✅ Disk space OK (${available_mb}MB available)"
  fi
}

# =============================================================================
# Check 6: Lockfile Existence
# =============================================================================
check_lockfiles() {
  log "[check] Lockfiles"
  
  if [[ ! -f "$ROOT/Cargo.lock" ]]; then
    log "  ❌ Cargo.lock missing"
    add_failure "missing_lockfile:Cargo.lock"
  else
    log "  ✅ Cargo.lock present"
  fi
  
  if [[ ! -f "$ROOT/Cargo.lock.packaging" ]]; then
    log "  ⚠️  Cargo.lock.packaging missing (needed for PyPI lane)"
    add_warning "missing_lockfile:Cargo.lock.packaging"
  else
    log "  ✅ Cargo.lock.packaging present"
  fi
}

# =============================================================================
# Check 7: Workspace Integrity
# =============================================================================
check_workspace_integrity() {
  log "[check] Workspace integrity"
  
  # Check for Cargo.toml
  if [[ ! -f "$ROOT/Cargo.toml" ]]; then
    log "  ❌ Cargo.toml missing"
    add_failure "missing_manifest:Cargo.toml"
  else
    log "  ✅ Cargo.toml present"
  fi
  
  # Check for rust-toolchain.toml
  if [[ ! -f "$ROOT/rust-toolchain.toml" ]]; then
    log "  ❌ rust-toolchain.toml missing"
    add_failure "missing_toolchain_config"
  else
    log "  ✅ rust-toolchain.toml present"
  fi
  
  # Check for required scripts
  local scripts=("verify_toolchain_policy.sh" "prepush_gate.sh" "push_main.sh")
  for script in "${scripts[@]}"; do
    if [[ ! -f "$ROOT/scripts/$script" ]]; then
      log "  ❌ scripts/$script missing"
      add_failure "missing_script:$script"
    fi
  done
}

# =============================================================================
# Check 8: Git State
# =============================================================================
check_git_state() {
  log "[check] Git state"
  
  # Check if we're in a git repo
  if ! git -C "$ROOT" rev-parse --git-dir >/dev/null 2>&1; then
    log "  ⚠️  Not a git repository"
    add_warning "not_a_git_repo"
    return
  fi
  
  # Check for uncommitted changes
  if ! git -C "$ROOT" diff --quiet 2>/dev/null; then
    log "  ⚠️  Uncommitted changes present"
    add_warning "uncommitted_changes"
  else
    log "  ✅ Working tree clean"
  fi
  
  # Check branch
  local branch
  branch=$(git -C "$ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
  log "  ✅ On branch: $branch"
}

# =============================================================================
# Check 9: Rust Environment
# =============================================================================
check_rust_env() {
  log "[check] Rust environment"
  
  # Check RUST_BACKTRACE
  if [[ "${RUST_BACKTRACE:-0}" == "0" ]]; then
    log "  ⚠️  RUST_BACKTRACE not set (recommended: 1)"
    add_warning "rust_backtrace_not_set"
  else
    log "  ✅ RUST_BACKTRACE=$RUST_BACKTRACE"
  fi
  
  # Check CARGO_REGISTRIES_CRATES_IO_PROTOCOL
  local protocol="${CARGO_REGISTRIES_CRATES_IO_PROTOCOL:-sparse}"
  if [[ "$protocol" == "sparse" ]]; then
    log "  ✅ Using sparse protocol (recommended)"
  else
    log "  ⚠️  Using $protocol protocol (sparse recommended)"
    add_warning "cargo_protocol:$protocol"
  fi
}

# =============================================================================
# Main
# =============================================================================
main() {
  log "=============================================="
  log "Proactive Preflight Check"
  log "=============================================="
  log ""
  
  check_required_tools
  check_required_toolchains
  check_protoc
  check_network
  check_disk_space
  check_lockfiles
  check_workspace_integrity
  check_git_state
  check_rust_env
  
  log ""
  log "=============================================="
  
  if [[ ${#FAILED_CHECKS[@]} -eq 0 ]]; then
    log "Result: ✅ PASSED"
    log ""
    json_output
    exit 0
  else
    log "Result: ❌ FAILED (${#FAILED_CHECKS[@]} checks failed)"
    log ""
    if [[ ${#WARNINGS[@]} -gt 0 ]]; then
      log "Warnings: ${#WARNINGS[@]}"
    fi
    log ""
    json_output
    exit 1
  fi
}

main "$@"
