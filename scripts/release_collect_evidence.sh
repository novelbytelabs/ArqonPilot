#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORTS_DIR="${HOME}/.pilot/reports"
TMP_REPORTS_DIR="/tmp/pilot-reports"

usage() {
  cat <<'EOF'
Usage: ./scripts/release_collect_evidence.sh --label <release_label> [--out <dir>]

Collects latest release evidence logs/artifacts into a single directory and writes
a summary markdown file.

Examples:
  ./scripts/release_collect_evidence.sh --label 0.2.0a1
  ./scripts/release_collect_evidence.sh --label 0.2.0a1 --out ~/.pilot/release_evidence
EOF
}

LABEL=""
OUT_BASE="${HOME}/.pilot/release_evidence"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --label)
      LABEL="${2:-}"
      shift 2
      ;;
    --out)
      OUT_BASE="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown option '$1'" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$LABEL" ]]; then
  echo "ERROR: --label is required." >&2
  exit 2
fi

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="${OUT_BASE}/release_${LABEL}_${STAMP}"
mkdir -p "${OUT_DIR}"

copy_latest() {
  local pattern="$1"
  local src_dir="$2"
  local dst_name="$3"
  local file
  file="$(ls -1t "${src_dir}"/${pattern} 2>/dev/null | head -n 1 || true)"
  if [[ -n "${file}" && -f "${file}" ]]; then
    cp "${file}" "${OUT_DIR}/${dst_name}"
    echo "copied: ${file} -> ${OUT_DIR}/${dst_name}"
  else
    echo "missing: ${src_dir}/${pattern}"
  fi
}

copy_latest_any() {
  local pattern="$1"
  local dst_name="$2"
  shift 2
  local newest=""
  local candidate=""
  for dir in "$@"; do
    candidate="$(ls -1t "${dir}"/${pattern} 2>/dev/null | head -n 1 || true)"
    if [[ -n "${candidate}" && -f "${candidate}" ]]; then
      if [[ -z "${newest}" || "${candidate}" -nt "${newest}" ]]; then
        newest="${candidate}"
      fi
    fi
  done
  if [[ -n "${newest}" ]]; then
    cp "${newest}" "${OUT_DIR}/${dst_name}"
    echo "copied: ${newest} -> ${OUT_DIR}/${dst_name}"
  else
    echo "missing: ${pattern} in candidate dirs: $*"
  fi
}

copy_latest_any "prepush_gate_*.log" "prepush_gate_latest.log" "${TMP_REPORTS_DIR}" "${REPORTS_DIR}"
copy_latest "push_main_*.log" "${REPORTS_DIR}" "push_main_latest.log"
copy_latest "acceptance_matrix_wave_i_full_*.json" "${REPORTS_DIR}" "acceptance_matrix_wave_i_full_latest.json"
copy_latest "acceptance_matrix_wave_j_full_*.json" "${REPORTS_DIR}" "acceptance_matrix_wave_j_full_latest.json"
copy_latest_any "ui_smoke_*.log" "ui_smoke_latest.log" "${TMP_REPORTS_DIR}" "${REPORTS_DIR}"

GIT_SHA="$(git -C "${ROOT}" rev-parse HEAD)"
GIT_BRANCH="$(git -C "${ROOT}" branch --show-current)"
PY_VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "${ROOT}/pyproject.toml" | head -n1)"
CRATE_VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "${ROOT}/Cargo.toml" | head -n1)"

cat > "${OUT_DIR}/SUMMARY.md" <<EOF
# Release Evidence Summary

- label: ${LABEL}
- collected_at_utc: ${STAMP}
- git_branch: ${GIT_BRANCH}
- git_sha: ${GIT_SHA}
- pyproject_version: ${PY_VERSION}
- cargo_workspace_version: ${CRATE_VERSION}

## Files

$(ls -1 "${OUT_DIR}" | sed 's/^/- /')
EOF

# Generate JSON manifest with hashes
echo "Generating manifest.json..."
(
  cd "${OUT_DIR}"
  echo "[" > manifest.json
  first=true
  for f in *; do
    if [[ "$f" == "manifest.json" ]]; then continue; fi
    hash=$(sha256sum "$f" | cut -d' ' -f1)
    if [ "$first" = true ]; then first=false; else echo "," >> manifest.json; fi
    echo "  {\"file\": \"$f\", \"sha256\": \"$hash\", \"timestamp\": \"$STAMP\"}" >> manifest.json
  done
  echo "]" >> manifest.json
)

# Generate verify_bundle.sh
cat > "${OUT_DIR}/verify_bundle.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

echo "Checking Arqon Pilot Release Evidence Integrity..."
if [ ! -f manifest.json ]; then
  echo "❌ FAIL: manifest.json missing"
  exit 1
fi

MISM_COUNT=0
TOTAL=$(jq '. | length' manifest.json)
for i in $(seq 0 $(($TOTAL - 1))); do
    item=$(jq -r ".[$i]" manifest.json)
    file=$(echo "$item" | jq -r '.file')
    expected=$(echo "$item" | jq -r '.sha256')
    
    if [ ! -f "$file" ]; then
        echo "❌ MISSING: $file"
        MISM_COUNT=$((MISM_COUNT + 1))
        continue
    fi
    
    actual=$(sha256sum "$file" | cut -d' ' -f1)
    if [ "$actual" == "$expected" ]; then
        echo "✅ OK: $file"
    else
        echo "❌ MISMATCH: $file (expected $expected, got $actual)"
        MISM_COUNT=$((MISM_COUNT + 1))
    fi
done

if [ "$MISM_COUNT" -eq 0 ]; then
    echo "--- ALL OK: Release Evidence Integrity Verified ---"
    exit 0
else
    echo "--- FAILED: $MISM_COUNT integrity errors found ---"
    exit 1
fi
EOF
chmod +x "${OUT_DIR}/verify_bundle.sh"

echo ""
echo "Release evidence collected and manifest generated at:"
echo "  ${OUT_DIR}"
echo "Summary:"
echo "  ${OUT_DIR}/SUMMARY.md"
echo "Manifest:"
echo "  ${OUT_DIR}/manifest.json"
echo "Verification Script:"
echo "  ${OUT_DIR}/verify_bundle.sh"
