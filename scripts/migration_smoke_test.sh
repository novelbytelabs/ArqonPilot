#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/frozen_versions.sh"

PILOT_BIN="./scripts/pilot_local.sh"

echo "--- Migration Smoke Test Start ---"

# 0. Safety Cleanup
echo "[0/3] Stopping any orphaned postgres on port 9132..."
$PILOT_BIN db stop || true
# Force kill if still there
PID=$(lsof -t -i :9132 || true)
if [ -n "$PID" ]; then
  echo "Force killing stale postgres PID $PID"
  kill -9 $PID || true
fi

# 1. Clean Startup Test
echo "[1/3] Cold startup (no DB)..."
rm -rf ~/.arqon/pilot/db/ ~/.arqon/pilot/run/
$PILOT_BIN db start
sleep 4 # Give it more time

# Verify tables exist
echo "[1/3] Checking agorg table existence..."
$PILOT_BIN agorg list > /dev/null
echo "✅ Cold startup OK"

# 2. Warm Startup Test (Migration)
echo "[2/3] Warm startup (existing DB)..."
# Force a restart of the binary to trigger migration logic
$PILOT_BIN db stop
$PILOT_BIN db start
sleep 2

# Verify still healthy
$PILOT_BIN agorg list > /dev/null
echo "✅ Warm startup OK"

# 3. Data Accessibility Test
echo "[3/3] Data accessibility test..."
# Create a dummy AGOrg if none exists
COUNT=$($PILOT_BIN agorg list | grep -c "|" || true)
if [ "$COUNT" -eq 0 ]; then
  echo "No AGOrgs found, creating a dummy to test persistency..."
  # Use current dir as a dummy
  $PILOT_BIN agorg create --name "MigrationTest" --root "$ROOT" > /dev/null
fi

# Query after restart
$PILOT_BIN db stop
$PILOT_BIN db start
sleep 2

NEW_COUNT=$($PILOT_BIN agorg list | grep -c "|" || true)
if [ "$NEW_COUNT" -gt 0 ]; then
  echo "✅ Data accessibility OK (Found $NEW_COUNT AGOrgs)"
else
  echo "❌ Data accessibility FAIL (Lost data after restart)"
  exit 1
fi

echo "--- Migration Smoke Test PASSED ---"
