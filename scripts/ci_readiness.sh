#!/bin/bash
#
# CI Contract Readiness Command - Check system readiness
#
# This wraps ci_contract.sh for the 'readiness' command with FC-5 contract layer

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/ci_contract.sh" readiness "$@"
