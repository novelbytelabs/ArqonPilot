#!/bin/bash
#
# CI Contract Repair Command - Repair locks, drift, policy, or scope
#
# This wraps ci_contract.sh for the 'repair' command with FC-5 contract layer
#
# Usage:
#   ci_repair.sh --type <repair_type> [--scope <scope>] [--preview] [--force] [--dry-run]
#
# Repair types:
#   lock_repair      - Repair Cargo.lock drift
#   drift_correction - Correct version drift
#   policy_reset     - Reset policy to defaults
#   scope_recovery   - Recover scope state

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/ci_contract.sh" repair "$@"
