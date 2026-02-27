#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'EOF'
Usage: ./scripts/verify_pypi_release.sh [--index pypi|testpypi] [--package <name>] [--version <ver>] [--retries <n>] [--sleep <sec>]

Verifies that a released package/version is visible on the selected index.
Checks both version metadata and release files via JSON API.
EOF
}

INDEX="pypi"
PACKAGE=""
VERSION=""
RETRIES=20
SLEEP_SEC=15

while [[ $# -gt 0 ]]; do
  case "$1" in
    --index)
      INDEX="${2:-}"
      shift 2
      ;;
    --package)
      PACKAGE="${2:-}"
      shift 2
      ;;
    --version)
      VERSION="${2:-}"
      shift 2
      ;;
    --retries)
      RETRIES="${2:-}"
      shift 2
      ;;
    --sleep)
      SLEEP_SEC="${2:-}"
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

if [[ -z "$PACKAGE" ]]; then
  PACKAGE="$(sed -n 's/^name = "\([^"]*\)"/\1/p' pyproject.toml | head -n1)"
fi
if [[ -z "$VERSION" ]]; then
  VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' pyproject.toml | head -n1)"
fi

if [[ -z "$PACKAGE" || -z "$VERSION" ]]; then
  echo "ERROR: could not determine package/version; pass --package and --version." >&2
  exit 1
fi

case "$INDEX" in
  pypi)
    API_URL="https://pypi.org/pypi/${PACKAGE}/${VERSION}/json"
    ;;
  testpypi)
    API_URL="https://test.pypi.org/pypi/${PACKAGE}/${VERSION}/json"
    ;;
  *)
    echo "ERROR: --index must be pypi or testpypi" >&2
    exit 2
    ;;
esac

echo "[pypi-verify] index=$INDEX package=$PACKAGE version=$VERSION"
echo "[pypi-verify] api=$API_URL"

attempt=1
while (( attempt <= RETRIES )); do
  echo "[pypi-verify] attempt ${attempt}/${RETRIES}"
  if python - "$API_URL" "$VERSION" <<'PY'
import json
import sys
import urllib.request
from urllib.error import HTTPError, URLError

url = sys.argv[1]
version = sys.argv[2]
try:
    with urllib.request.urlopen(url, timeout=15) as r:
        data = json.load(r)
except HTTPError as e:
    # 404 is expected while index propagation is pending.
    if e.code == 404:
        sys.exit(2)
    print(f"[pypi-verify] HTTP error: {e}", file=sys.stderr)
    sys.exit(1)
except URLError as e:
    print(f"[pypi-verify] URL error: {e}", file=sys.stderr)
    sys.exit(1)

info = data.get("info", {})
releases = data.get("releases", {}).get(version, [])
if info.get("version") != version:
    print(f"[pypi-verify] version mismatch: info.version={info.get('version')} expected={version}", file=sys.stderr)
    sys.exit(1)
if not releases:
    print(f"[pypi-verify] release has no files yet for {version}", file=sys.stderr)
    sys.exit(2)

print(f"[pypi-verify] visible: files={len(releases)}")
sys.exit(0)
PY
  then
    echo "[pypi-verify] PASS"
    exit 0
  else
    rc=$?
    if [[ "$rc" -eq 2 ]]; then
      if (( attempt < RETRIES )); then
        echo "[pypi-verify] not visible yet; sleeping ${SLEEP_SEC}s..."
        sleep "$SLEEP_SEC"
        attempt=$((attempt + 1))
        continue
      fi
      echo "ERROR: package/version did not become visible within retry window." >&2
      exit 1
    fi
    echo "ERROR: verification failed due to non-retryable error." >&2
    exit 1
  fi
done

echo "ERROR: exhausted verification attempts." >&2
exit 1
