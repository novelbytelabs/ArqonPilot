[0;34m[REPLAY][0m Replaying command: run, scope: ArqonPilot, gates: toolchain_policy
#!/bin/bash
#
# Replay Bundle Script
# Generated from execution: 0fe7a404-ea25-45af-ae3d-557ed7343edb
#
# This script will reproduce the execution with the same parameters
set -euo pipefail

echo "=== Replay Execution ==="
echo "Original execution: 0fe7a404-ea25-45af-ae3d-557ed7343edb"
echo "Command: run"
echo "Scope: ArqonPilot"
echo "Gates: toolchain_policy"
echo ""

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Run the command with same parameters
case "run" in
    run)
        if [[ -n "toolchain_policy" ]]; then
            echo "Running: ci_run.sh --scope ArqonPilot --gates toolchain_policy"
            "$SCRIPT_DIR/ci_run.sh" --scope "ArqonPilot" --gates "toolchain_policy"
        else
            echo "Running: ci_run.sh --scope ArqonPilot"
            "$SCRIPT_DIR/ci_run.sh" --scope "ArqonPilot"
        fi
        ;;
    repair)
        echo "Running: ci_repair.sh --type toolchain_policy"
        "$SCRIPT_DIR/ci_repair.sh" --type "toolchain_policy" --force
        ;;
    readiness)
        echo "Running: ci_readiness.sh --scope ArqonPilot"
        "$SCRIPT_DIR/ci_readiness.sh" --scope "ArqonPilot"
        ;;
    report)
        echo "Running: ci_report.sh"
        "$SCRIPT_DIR/ci_report.sh"
        ;;
    *)
        echo "Unknown command: run, defaulting to run"
        "$SCRIPT_DIR/ci_run.sh" --scope "ArqonPilot"
        ;;
esac

echo ""
echo "=== Replay Complete ==="
