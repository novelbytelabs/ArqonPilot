#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/frozen_versions.sh"

PILOT_BIN="./scripts/pilot_local.sh"
TMP_HOME="$(mktemp -d /tmp/pilot-migration-smoke.XXXXXX)"
ORIG_HOME="${HOME}"
ORIG_CARGO_HOME="${CARGO_HOME:-$ORIG_HOME/.cargo}"
ORIG_RUSTUP_HOME="${RUSTUP_HOME:-$ORIG_HOME/.rustup}"

pilot_cmd() {
  HOME="$TMP_HOME" CARGO_HOME="$ORIG_CARGO_HOME" RUSTUP_HOME="$ORIG_RUSTUP_HOME" "$PILOT_BIN" "$@"
}

cleanup() {
  pilot_cmd db stop >/dev/null 2>&1 || true
  rm -rf "$TMP_HOME"
}
trap cleanup EXIT

echo "--- Migration Smoke Test Start ---"
echo "[info] Isolated HOME for deterministic run: $TMP_HOME"
echo "[info] Preserving cargo/rustup cache from: $ORIG_HOME"

# 0. Safety Cleanup
echo "[0/3] Stopping any orphaned postgres on port 9132..."
pilot_cmd db stop || true
# Force kill if still there
PID=$(lsof -t -i :9132 || true)
if [ -n "$PID" ]; then
  echo "Force killing stale postgres PID $PID"
  kill -9 $PID || true
fi

# 1. Clean Startup Test
echo "[1/3] Cold startup (no DB)..."
rm -rf "$TMP_HOME/.arqon/pilot/db/" "$TMP_HOME/.arqon/pilot/run/"
pilot_cmd db start
sleep 4 # Give it more time

# Verify tables exist
echo "[1/3] Checking agorg table existence..."
pilot_cmd agorg list > /dev/null
echo "✅ Cold startup OK"

# 2. Warm Startup Test (Migration)
echo "[2/3] Warm startup (existing DB)..."
# Force a restart of the binary to trigger migration logic
pilot_cmd db stop
pilot_cmd db start
sleep 2

# Verify still healthy
pilot_cmd agorg list > /dev/null
echo "✅ Warm startup OK"

# 3. Data Accessibility Test
echo "[3/3] Data accessibility test..."
# Create a dummy AGOrg if none exists
COUNT=$(pilot_cmd agorg list | grep -c "|" || true)
if [ "$COUNT" -eq 0 ]; then
  echo "No AGOrgs found, creating a dummy to test persistency..."
  # Use current dir as a dummy
  pilot_cmd agorg create --name "MigrationTest" --root "$ROOT" > /dev/null
fi

# Query after restart
pilot_cmd db stop
pilot_cmd db start
sleep 2

NEW_COUNT=$(pilot_cmd agorg list | grep -c "|" || true)
if [ "$NEW_COUNT" -gt 0 ]; then
  echo "✅ Data accessibility OK (Found $NEW_COUNT AGOrgs)"
else
  echo "❌ Data accessibility FAIL (Lost data after restart)"
  exit 1
fi

echo "--- Migration Smoke Test PASSED ---"
