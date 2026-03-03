#!/bin/bash
set -eo pipefail
echo "[test] ensuring services running"
cargo run -p pilot -- services restart
echo "[test] simulating bus crash via SIGKILL"
pkill -9 arqon-bus || true
pkill -9 arqon-pilot || true
echo "[test] completed - the UI /api/health should degrade temporarily"
