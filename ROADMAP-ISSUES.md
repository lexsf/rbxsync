# Roadmap Issues

This document tracks follow-up functionality identified after the local `import-place` and `extract-place` MVPs. These items are not blockers for the current local `.rbxl` / `.rbxlx` workflows, but they should be scoped before claiming full publish/import parity.

## P0: Published Place Import

Add support for importing a published Roblox place by place ID.

Proposed shape:

```bash
rbxsync import-place --place-id 123456789 --output ./Game --api-key "$ROBLOX_OPEN_CLOUD_API_KEY"
```

Scope needs:

- Open Cloud authentication and credential discovery.
- Place download API integration.
- Permission, ownership, rate-limit, and error handling behavior.
- JSON output compatible with the existing local `import-place --json` response.
- Clear diagnostics for authentication failures, missing places, and unsupported formats.

## P0: Publish Workflow - Completed

Implemented as `rbxsync publish-place`, a separate command that uploads an
existing `.rbxl` or `.rbxlx` artifact to Roblox Open Cloud. The current
`extract-place` command intentionally remains local-only.

Implemented decision:

- Add a separate `publish-place` command that consumes an existing `.rbxl` or
  `.rbxlx`.

Completed scope:

- Confirmation prompts before overwriting a live place.
- `--yes` for CI and non-interactive uploads.
- Dry-run behavior that validates local inputs without uploading.
- JSON output for automation, without printing API keys.
- Rate-limit diagnostics with `Retry-After` guidance.
- Documentation that rollback must use Roblox version history or a later
  dedicated rollback workflow.

## P1: Atomic Export Writes

Make `extract-place` writes atomic. The PRD calls for writing to a temporary sibling file and renaming it into place after serialization succeeds.

Acceptance:

- Existing output is never truncated unless the new artifact has been fully written.
- Failed serialization cleans up temporary files where possible.
- Tests cover failure behavior with `--force`.

## P1: Asset Handling

Define and implement asset behavior for import and export. Current workflows preserve asset references, but do not download, embed, or rewrite external assets.

Future options:

```bash
rbxsync extract-place --include-assets
rbxsync extract-place --no-assets
rbxsync import-place --include-assets
```

Scope needs:

- Decide whether assets are referenced, copied, downloaded, or embedded.
- Define local `assets/` layout and manifest format.
- Preserve `Content`, `BinaryString`, and `SharedString` behavior.
- Avoid network access unless explicitly requested.

## P1: Terrain Round-Trip Parity

Terrain voxel data is not fully represented in the current project format. Import and export should not claim full terrain parity until a stable filesystem representation exists.

Scope needs:

- Define terrain storage format under the project directory.
- Import terrain voxel data when available from `.rbxl` / `.rbxlx`.
- Export terrain voxel data back into the place artifact.
- Keep metadata-only terrain support and diagnostics for unsupported cases.

## P2: Package Default Semantics

Reconcile package export defaults with the PRD. The PRD says package folders should be included by default when present under the exported tree, while the current CLI requires `--include-packages`.

Scope needs:

- Decide whether default behavior should follow `rbxsync.json` package preservation settings.
- Clarify `--include-packages` and `--no-packages` semantics in command docs.
- Add tests for package inclusion, package skipping, and summary counts.

## P2: Import Strict Mode

Add `--strict` to `import-place` for parity with `extract-place`.

Acceptance:

- Diagnostics that are warnings by default can fail the import under `--strict`.
- `--json` includes the strict setting and diagnostic summary.
- CI examples document strict mode for deterministic validation.
