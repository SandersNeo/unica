# Unica Codex Plugin

Unica packages a pinned macOS arm64 toolset for 1C:Enterprise development in Codex.

Included in this plugin version:

- `bsl-analyzer` `0.1.144`
- `v8-runner` `0.3.0`
- public v8std MCP endpoint: `https://ai.v8std.ru/mcp`

The local tools are committed as binaries under `bin/darwin-arm64/`. They are never launched directly by `.mcp.json`; every launch goes through `scripts/run-tool.sh`, which checks:

- host is `Darwin arm64`;
- `third-party/manifest.json` exists;
- the binary exists and is executable;
- SHA-256 matches the pinned manifest.

## Local Codex Install

Register the repo-local marketplace from the repository root:

```sh
codex plugin marketplace add "$PWD"
```

Enable `unica@unica-local` in Codex. The plugin owns its MCP registrations through `.mcp.json`; do not add these servers separately with global `codex mcp add`.

## MCP Servers

The plugin exposes:

- `unica-bsl-reference` for platform/reference tools from `bsl-analyzer`;
- `unica-bsl-workspace` for project-local BSL metadata and code tools;
- `unica-v8-runner` for `v8-runner` build/test/syntax workflows;
- `unica-v8std` for public v8std standards and APK knowledge.

`unica-bsl-workspace` uses the current directory as `--source-dir` by default. Set `UNICA_BSL_SOURCE_DIR` when Codex should analyze a different source root.

## Updating Pinned Tools

Do not replace binaries without bumping the plugin version and updating `third-party/manifest.json`.

For every tool update:

1. build or fetch the public release for `aarch64-apple-darwin`;
2. place the binary under `bin/darwin-arm64/`;
3. update version, tag, commit, upstream URL, license, and SHA-256 in `third-party/manifest.json`;
4. run JSON validation, script syntax checks, binary version/help checks, and MCP smoke tests.
