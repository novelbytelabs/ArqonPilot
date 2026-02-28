#!/usr/bin/env bash
set -euo pipefail

wave="I"
profile="quick"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --wave)
      wave="${2:-I}"
      shift 2
      ;;
    --profile)
      profile="${2:-quick}"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ "$wave" != "I" ]]; then
  echo "only wave I is currently supported" >&2
  exit 2
fi

if [[ "$profile" != "quick" && "$profile" != "full" ]]; then
  echo "profile must be quick or full" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/pilot_wave_matrix_XXXXXX")"
manifest_file="${work_dir}/manifest.tsv"

run_check() {
  local name="$1"
  local cmd="$2"
  local out_file="${work_dir}/${name}.out"
  local err_file="${work_dir}/${name}.err"
  local start_epoch end_epoch duration started_at ended_at rc ok
  start_epoch="$(date +%s)"
  started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  if /bin/bash -lc "$cmd" >"$out_file" 2>"$err_file"; then
    rc=0
    ok=true
  else
    rc=$?
    ok=false
  fi
  end_epoch="$(date +%s)"
  ended_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  duration="$((end_epoch - start_epoch))"
  printf "%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n" \
    "$name" "$cmd" "$ok" "$rc" "$started_at" "$ended_at" "$duration" "$work_dir" >>"$manifest_file"
}

run_check "toolchain_policy" "./scripts/verify_toolchain_policy.sh"
run_check "js_syntax" "node -c crates/pilot/src/pilot_ui.js"
run_check "cargo_locked_check" "cargo check -p pilot --locked"

if [[ "$profile" == "full" ]]; then
  run_check "prepush_gate" "./scripts/prepush_gate.sh"
fi

python3 - "$manifest_file" "$wave" "$profile" <<'PY'
import json
import pathlib
import sys

manifest = pathlib.Path(sys.argv[1])
wave = sys.argv[2]
profile = sys.argv[3]

checks = []
overall_ok = True

for line in manifest.read_text(encoding="utf-8").splitlines():
    if not line.strip():
        continue
    name, cmd, ok_raw, rc_raw, started, ended, duration, work_dir = line.split("\t")
    out_path = pathlib.Path(work_dir) / f"{name}.out"
    err_path = pathlib.Path(work_dir) / f"{name}.err"
    ok = ok_raw == "true"
    overall_ok = overall_ok and ok
    checks.append(
        {
            "name": name,
            "command": cmd,
            "ok": ok,
            "exit_code": int(rc_raw),
            "started_at": started,
            "ended_at": ended,
            "duration_sec": int(duration),
            "stdout": out_path.read_text(encoding="utf-8", errors="replace"),
            "stderr": err_path.read_text(encoding="utf-8", errors="replace"),
        }
    )

print(
    json.dumps(
        {
            "ok": overall_ok,
            "wave": wave,
            "profile": profile,
            "checks": checks,
        },
        indent=2,
    )
)
PY

rm -rf "$work_dir"
