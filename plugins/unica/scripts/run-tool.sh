#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "usage: run-tool.sh <tool-name> [args...]" >&2
  exit 64
fi

TOOL_NAME="$1"
shift

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MANIFEST="$PLUGIN_ROOT/third-party/manifest.json"

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"

if [ "$HOST_OS" != "Darwin" ] || [ "$HOST_ARCH" != "arm64" ]; then
  echo "Unica v1 ships binaries only for darwin-arm64; current host is ${HOST_OS}-${HOST_ARCH}." >&2
  exit 78
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to read Unica third-party manifest." >&2
  exit 69
fi

if [ ! -f "$MANIFEST" ]; then
  echo "Unica third-party manifest not found: $MANIFEST" >&2
  exit 66
fi

TOOL_METADATA="$(python3 - "$MANIFEST" "$TOOL_NAME" <<'PY'
import json
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
tool_name = sys.argv[2]
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
for tool in manifest.get("tools", []):
    if tool.get("name") == tool_name:
        print(tool["binaryPath"])
        print(tool["sha256"])
        sys.exit(0)

print(f"tool not found in manifest: {tool_name}", file=sys.stderr)
sys.exit(1)
PY
)"

BINARY_RELATIVE="$(printf '%s\n' "$TOOL_METADATA" | sed -n '1p')"
EXPECTED_SHA="$(printf '%s\n' "$TOOL_METADATA" | sed -n '2p')"
BINARY="$PLUGIN_ROOT/$BINARY_RELATIVE"

if [ ! -x "$BINARY" ]; then
  echo "Unica binary is missing or not executable: $BINARY" >&2
  exit 66
fi

ACTUAL_SHA="$(shasum -a 256 "$BINARY" | awk '{print $1}')"
if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
  echo "Unica binary checksum mismatch for $TOOL_NAME." >&2
  echo "expected: $EXPECTED_SHA" >&2
  echo "actual:   $ACTUAL_SHA" >&2
  exit 65
fi

exec "$BINARY" "$@"
