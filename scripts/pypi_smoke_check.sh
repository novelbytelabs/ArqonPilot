#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "[1/4] Build wheel with maturin"
maturin build --release --out dist

echo "[2/4] Create isolated venv"
TMPDIR="$(mktemp -d)"
python3 -m venv "$TMPDIR/venv"
source "$TMPDIR/venv/bin/activate"

echo "[3/4] Install built wheel"
WHEEL="$(ls -t dist/arqon_pilot-*.whl | head -n 1)"
pip install "$WHEEL"

echo "[4/4] Smoke test CLI"
pilot --help >/tmp/pilot_pypi_help.txt
rg -n "oracle|heal|navigate|branch|multi|secure|plan|create|know|init" /tmp/pilot_pypi_help.txt >/dev/null

echo "PyPI smoke check passed with wheel: $WHEEL"
