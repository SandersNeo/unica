#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_DIR="${UNICA_BSL_SOURCE_DIR:-.}"

exec "$SCRIPT_DIR/run-bsl-analyzer.sh" mcp serve --profile workspace --source-dir "$SOURCE_DIR"
