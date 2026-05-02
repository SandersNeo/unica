---
name: unica-v8-runner
description: Use when building, testing, validating, dumping, or launching 1C projects through the pinned v8-runner packaged by Unica.
---

# Unica v8-runner

Verify the runner before using it.

## First Checks

- Help: `plugins/unica/scripts/run-v8-runner.sh --help`
- MCP launcher syntax: `bash -n plugins/unica/scripts/run-v8-runner-mcp.sh`
- Project config: check for `v8project.yaml` before running project commands.

## Usage Guidance

- Prefer `v8-runner config init` only when the user wants Unica to create project config.
- Use `build`, `syntax`, and `test` through the pinned wrapper, not through a globally installed `v8-runner`.
- For MCP, use `unica-v8-runner`; its tool surface is intentionally narrower than the CLI.
- For 1C runtime failures, inspect the actual `v8project.yaml`, platform paths, and workPath logs instead of patching symptoms.
