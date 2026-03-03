#!/bin/bash
#
# CI Contract Report Command - Generate execution reports
#
# This wraps ci_contract.sh for the 'report' command with FC-5 contract layer

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/ci_contract.sh" report "$@"
