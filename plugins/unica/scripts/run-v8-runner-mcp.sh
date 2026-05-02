#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -z "${V8TR_CONFIG:-}" ] && [ ! -f "v8project.yaml" ]; then
  echo "v8-runner MCP requires v8project.yaml in the current directory or V8TR_CONFIG pointing to a config file." >&2
  echo "Run 'plugins/unica/scripts/run-v8-runner.sh config init' in a 1C project before starting this MCP server." >&2
  exit 66
fi

exec "$SCRIPT_DIR/run-v8-runner.sh" mcp serve stdio
