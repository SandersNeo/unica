---
name: unica-bsl
description: Use when working with bsl-analyzer through the Unica plugin for 1C BSL search, diagnostics, metadata, or MCP setup.
---

# Unica BSL Analyzer

Start with real availability checks.

## First Checks

- Wrapper/version: `plugins/unica/scripts/run-bsl-analyzer.sh --version`
- Reference MCP: `plugins/unica/scripts/run-bsl-reference.sh`
- Workspace MCP: `UNICA_BSL_SOURCE_DIR=<source-root> plugins/unica/scripts/run-bsl-workspace.sh`

Use `UNICA_BSL_SOURCE_DIR` when the 1C source root is not the current directory.

## Usage Guidance

- Use the `reference` profile for platform syntax, ITS-style help, and documentation search.
- Use the `workspace` profile for project code, metadata, SDBL validation, and optional live 1C tools.
- Do not assume live 1C execution/debugging is available; it requires a published `bsl-analyzer` HTTP service and credentials.
- If MCP behavior is unclear, list tools and call a safe status/search action before relying on it.
