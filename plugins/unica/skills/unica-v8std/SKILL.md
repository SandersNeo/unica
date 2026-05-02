---
name: unica-v8std
description: Use when looking up 1C standards, APK diagnostic codes, BSLLS/v8-code-style context, or public v8std knowledge through Unica.
---

# Unica v8std

Always distinguish the documentation site from the MCP endpoint.

## First Checks

- Liveness: `https://ai.v8std.ru/healthz` and `https://ai.v8std.ru/version`
- MCP endpoint: `https://ai.v8std.ru/mcp`
- Plain GET `/mcp` can return protocol errors; use MCP initialize/tools/list semantics for real verification.

## Usage Guidance

- Use `unica-v8std` first for APK codes, 1C standards, and v8-code-style lookups.
- Re-list live MCP tools before scripting against exact tool names; public tool names can change.
- If the endpoint is unreachable, report reachability separately from plugin configuration.
