#!/bin/bash
#
# CI Contract Replay Command - Replay a previous execution
#
# This wraps ci_contract.sh for the 'replay' command with FC-5 contract layer

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/ci_contract.sh" replay "$@"
