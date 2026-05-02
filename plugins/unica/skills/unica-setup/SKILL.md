---
name: unica-setup
description: Use when setting up or verifying the Unica Codex plugin and its pinned 1C development tools.
---

# Unica Setup

Always verify the real tool state before giving setup advice.

## Checks

1. Confirm the plugin files exist: `plugins/unica/.codex-plugin/plugin.json`, `plugins/unica/.mcp.json`, and `plugins/unica/third-party/manifest.json`.
2. Validate JSON with `python3 -m json.tool`.
3. Validate scripts with `bash -n plugins/unica/scripts/*.sh`.
4. Verify pinned binaries through wrappers:
   - `plugins/unica/scripts/run-bsl-analyzer.sh --version`
   - `plugins/unica/scripts/run-v8-runner.sh --help`
5. If Codex visibility is in question, use a fresh-session check such as `codex debug prompt-input 'test'`; do not rely only on files existing on disk.

## Rules

- Do not mutate global `codex mcp` config for Unica-managed MCP servers.
- Treat `third-party/manifest.json` as the source of truth for pinned tool versions and checksums.
- If a binary checksum fails, fix the binary/manifest mismatch rather than bypassing the wrapper.
