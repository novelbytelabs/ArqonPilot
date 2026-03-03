#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
source "$ROOT/scripts/frozen_versions.sh"

echo "--- Compatibility Matrix Smoke Test Start ---"

# 1. Rust Core Check
echo -n "[1/5] Rust Core ($PILOT_CORE_RUST_VERSION): "
RUST_VER=$(rustc +$PILOT_CORE_RUST_VERSION --version 2>/dev/null || echo "MISSING")
if [[ "$RUST_VER" == *"$PILOT_CORE_RUST_VERSION"* ]]; then
  echo "✅ OK"
else
  echo "❌ FAIL ($RUST_VER)"
  exit 1
fi

# 2. Rust Packaging Check
echo -n "[2/5] Rust Packaging ($PILOT_PACKAGING_RUST_VERSION): "
RUST_PKG_VER=$(rustc +$PILOT_PACKAGING_RUST_VERSION --version 2>/dev/null || echo "MISSING")
if [[ "$RUST_PKG_VER" == *"$PILOT_PACKAGING_RUST_VERSION"* ]]; then
  echo "✅ OK"
else
  echo "❌ FAIL ($RUST_PKG_VER)"
  exit 1
fi

# 3. Protoc Check
echo -n "[3/5] Protoc (25.8): "
PROTOC_VER=$(protoc --version 2>/dev/null || echo "MISSING")
if [[ "$PROTOC_VER" == *"25.8"* ]]; then
  echo "✅ OK"
else
  echo "❌ FAIL ($PROTOC_VER)"
  exit 1
fi

# 4. Python Check
echo -n "[4/5] Python (3.10+): "
PYTHON_VER=$(python3 --version 2>/dev/null || echo "MISSING")
echo "✅ $PYTHON_VER"

# 5. OS/Platform Diagnostics
echo "[5/5] Platform Diagnostics:"
uname -a
if [ -f /etc/os-release ]; then
  grep PRETTY_NAME /etc/os-release
fi

echo "--- Compatibility Matrix Smoke Test PASSED ---"
