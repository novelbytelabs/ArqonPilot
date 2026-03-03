#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARQONBUS_DIR="${ARQONBUS_DIR:-$ROOT/../ArqonBus}"
CONDA_ENV="${CONDA_ENV:-helios-gpu-118}"
PYTHON_BIN="${PYTHON_BIN:-$HOME/miniconda3/envs/$CONDA_ENV/bin/python}"
HOST="${ARQONBUS_HOST:-127.0.0.1}"
PORT="${ARQONBUS_PORT:-9100}"
REPORT_DIR="${PILOT_REPORT_DIR:-$HOME/.pilot/reports}"
PID_FILE="$REPORT_DIR/arqonbus_shim_${PORT}.pid"
LOG_FILE="$REPORT_DIR/arqonbus_shim_${PORT}.log"

mkdir -p "$REPORT_DIR"

usage() {
  cat <<'EOF'
Usage: ./scripts/arqonbus_shim.sh [start|stop|status|logs]

Starts a compatibility ArqonBus websocket server for Arqon Pilot Control Panel
without modifying frozen ArqonBus source.

Env overrides:
  ARQONBUS_DIR   Path to ArqonBus repo (default: ../ArqonBus)
  CONDA_ENV      Conda env name (default: helios-gpu-118)
  ARQONBUS_HOST  Host to bind (default: 127.0.0.1)
  ARQONBUS_PORT  Port to bind (default: 9100)
EOF
}

is_running() {
  local pid
  pid="$(listener_pid)"
  [[ -n "${pid:-}" ]]
}

listener_pid() {
  ss -ltnp 2>/dev/null | awk -v p=":${PORT}" '
    $4 ~ p"$" {
      if (match($0, /pid=[0-9]+/)) {
        print substr($0, RSTART + 4, RLENGTH - 4);
        exit 0;
      }
    }
  '
}

start() {
  if is_running; then
    echo "[shim] already running (pid=$(listener_pid)) on ${HOST}:${PORT}"
    return 0
  fi

  if [[ ! -d "$ARQONBUS_DIR" ]]; then
    echo "[shim] ERROR: ARQONBUS_DIR not found: $ARQONBUS_DIR" >&2
    exit 1
  fi
  if [[ ! -x "$PYTHON_BIN" ]]; then
    echo "[shim] ERROR: python not found/executable: $PYTHON_BIN" >&2
    exit 1
  fi

  echo "[shim] starting ArqonBus compatibility shim on ${HOST}:${PORT}"
  echo "[shim] log: $LOG_FILE"

  local launch_cmd="
    cd '$ARQONBUS_DIR'
    PYTHONPATH=src '$PYTHON_BIN' -u - <<'PY'
import asyncio
from arqonbus.config.config import get_config
from arqonbus.routing.client_registry import ClientRegistry
from arqonbus.transport.websocket_bus import WebSocketBus

HOST = '${HOST}'
PORT = int('${PORT}')

async def main():
    cfg = get_config()
    cfg.server.host = HOST
    cfg.server.port = PORT
    bus = WebSocketBus(ClientRegistry(), config=cfg)
    await bus.start_server(host=HOST, port=PORT)
    print(f'ARQONBUS_SHIM_READY {HOST}:{PORT}', flush=True)
    await asyncio.Event().wait()

asyncio.run(main())
PY
  "

  if command -v setsid >/dev/null 2>&1; then
    setsid bash --noprofile --norc -lc "$launch_cmd" >>"$LOG_FILE" 2>&1 < /dev/null &
  else
    nohup bash --noprofile --norc -lc "$launch_cmd" >>"$LOG_FILE" 2>&1 < /dev/null &
  fi

  local launcher_pid="$!"
  local pid=""
  for _ in 1 2 3 4 5 6 7 8; do
    pid="$(listener_pid)"
    if [[ -n "${pid:-}" ]]; then
      echo "$pid" > "$PID_FILE"
      echo "[shim] started (pid=$pid, launcher_pid=$launcher_pid)"
      return 0
    fi
    sleep 1
  done

  if [[ -z "${pid:-}" ]]; then
    echo "[shim] ERROR: failed to start, see log: $LOG_FILE" >&2
    tail -n 80 "$LOG_FILE" >&2 || true
    exit 1
  fi
}

stop() {
  if ! is_running; then
    echo "[shim] not running"
    rm -f "$PID_FILE"
    return 0
  fi

  local pid=""
  if [[ -f "$PID_FILE" ]]; then
    pid="$(cat "$PID_FILE" 2>/dev/null || true)"
  fi
  if [[ -z "${pid:-}" ]]; then
    pid="$(listener_pid)"
  fi

  echo "[shim] stopping pid=$pid"
  kill "$pid" 2>/dev/null || true
  sleep 2

  if kill -0 "$pid" 2>/dev/null; then
    kill -9 "$pid" 2>/dev/null || true
  fi

  rm -f "$PID_FILE"
  echo "[shim] stopped"
}

status() {
  if is_running; then
    local pid
    pid="$(listener_pid)"
    echo "$pid" > "$PID_FILE"
    echo "[shim] RUNNING pid=$pid host=${HOST} port=${PORT}"
  else
    echo "[shim] STOPPED host=${HOST} port=${PORT}"
  fi
}

logs() {
  if [[ ! -f "$LOG_FILE" ]]; then
    echo "[shim] no log file yet: $LOG_FILE"
    return 0
  fi
  tail -n 120 "$LOG_FILE"
}

cmd="${1:-start}"
case "$cmd" in
  start) start ;;
  stop) stop ;;
  status) status ;;
  logs) logs ;;
  -h|--help|help) usage ;;
  *)
    echo "[shim] unknown command: $cmd" >&2
    usage
    exit 2
    ;;
esac
