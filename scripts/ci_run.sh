#!/bin/bash
#
# CI Contract Run Command - Execute gates with scope and policy checks
#
# This wraps ci_contract.sh for the 'run' command with FC-5 contract layer

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/ci_contract.sh" run "$@"
