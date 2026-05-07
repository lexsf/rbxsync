# Command-Line Place Import PRD

## Summary

Create a command-line utility that imports a Roblox place file into the same filesystem representation produced by the current RbxSync server extraction flow. The utility should accept a local `.rbxl` or `.rbxlx` place file, and optionally a published Roblox place identifier in a later phase, then create or replace a project directory containing `rbxsync.json`, `src/`, script source files, `.rbxjson` metadata files, and generated tooling files.

The recommended implementation is a new subcommand on the existing `rbxsync` CLI rather than a separate binary, because `rbxsync-cli` already depends on `rbx_binary`, `rbx_xml`, `rbx_dom_weak`, `rbxsync-core`, and the build pipeline that performs the reverse operation. A separate utility is acceptable only if it reuses the same shared extraction writer and serialization code.

## Background

Today, `rbxsync extract` requires Roblox Studio and the RbxSync Studio plugin. The CLI starts or contacts the HTTP server, posts to `/extract/start`, and waits for the plugin to serialize Studio instances. The plugin collects configured services, calls `Serializer.serializeInstance`, sends chunks to `/extract/chunk`, sends optional terrain batches to `/extract/terrain`, and finally calls `/extract/finalize`.

The server finalizer reads the chunk data and writes the repository format:

- `src/<Service>/...` mirrors DataModel paths.
- Script source is extracted to `.server.luau`, `.client.luau`, or `.luau`.
- Script `Source` is removed from the matching `.rbxjson`.
- Container instances are represented by a directory plus `_meta.rbxjson`.
- Leaf non-script instances are represented by `<Name>.rbxjson`.
- Existing `src/` is backed up to `.rbxsync-backup/src` before replacement.
- `treeMapping` in `rbxsync.json` maps DataModel paths to filesystem paths.
- Tooling files such as `default.project.json`, `selene.toml`, and `wally.toml` are generated when enabled.

This feature replaces the Studio/plugin source of serialized data with a place-file parser, while preserving the writer and output semantics.

## Problem

Users often have a saved `.rbxl` file or downloaded place file and want to convert it into an editable RbxSync project without opening Studio, installing the plugin, or running the live sync server. This is especially important for CI, bulk migration, archival conversion, and AI-assisted workflows where the environment may not have Roblox Studio available.

## Goals

1. Import a local `.rbxl` or `.rbxlx` place into RbxSync project files.
2. Produce the same directory and file format as server extraction.
3. Reuse the server finalization logic or move it into shared code so server extraction and CLI import cannot drift.
4. Preserve script source, instance hierarchy, names, properties available in the place file, references, attributes, and tags where supported by the Roblox DOM crates.
5. Provide clear progress output, useful error messages, and a non-interactive mode suitable for CI.
6. Add tests that compare importer output against existing RbxSync build and extraction behavior.

## Non-Goals

- Do not implement live bidirectional sync in this command.
- Do not require Roblox Studio for the local file import path.
- Do not redesign the RbxSync project format.
- Do not make Rojo the primary output format. `default.project.json` remains generated tooling only.
- Do not attempt to recover data not present in the saved place file. Studio extraction can read live API properties that may not be serialized into an `.rbxl`; the importer should preserve the file contents faithfully and should not invent default property values unless a later API-dump inflation feature is explicitly added.

## Users and Use Cases

- A developer receives `Game.rbxl` and wants `src/` files for code review and version control.
- A team wants to migrate multiple existing places into RbxSync projects in CI.
- An AI agent needs to inspect and edit Luau code from a place file without controlling Studio.
- A release process wants to verify `rbxsync build` output can be imported back to the same RbxSync format.

## Proposed CLI

Preferred subcommand:

```bash
rbxsync import-place <INPUT> [--output DIR] [--name NAME] [--force] [--services ServiceA,ServiceB]
```

Aliases may include:

```bash
rbxsync import <INPUT>
rbxsync extract-file <INPUT>
```

Examples:

```bash
rbxsync import-place ./MyGame.rbxl --output ./MyGame
rbxsync import-place ./MyGame.rbxlx --output ./converted --force
rbxsync import-place ./MyGame.rbxl --services Workspace,ServerScriptService
rbxsync import-place ./MyGame.rbxl --output . --name MyGame --no-tooling
```

Future published-place support:

```bash
rbxsync import-place --place-id 123456789 --output ./MyGame --api-key "$ROBLOX_OPEN_CLOUD_API_KEY"
```

Published-place support should be gated behind a separate implementation phase because it requires authenticated asset download behavior, API error handling, and likely additional documentation.

## CLI Options

- `INPUT`: required for MVP. Path to `.rbxl` or `.rbxlx`.
- `--output DIR`, `-o DIR`: destination project directory. Defaults to current directory.
- `--name NAME`: project name for generated `rbxsync.json` and `default.project.json`. Defaults to the input filename stem.
- `--force`: allow replacing an existing `src/`. Without this, fail if `src/` exists unless the command can create a backup and the user confirms.
- `--backup / --no-backup`: control `.rbxsync-backup/src` behavior. Default should match server extraction and create a backup.
- `--services LIST`: comma-separated DataModel service names to import. Defaults to the same service set used by plugin extraction: `Workspace`, `ReplicatedStorage`, `ReplicatedFirst`, `ServerScriptService`, `ServerStorage`, `StarterGui`, `StarterPack`, `StarterPlayer`, `Lighting`, `SoundService`, `Teams`, `Chat`, `LocalizationService`, `TestService`, and `MaterialService`.
- `--include-terrain / --no-terrain`: import terrain when it can be represented from the file. Default should be best-effort enabled, with a warning if unsupported.
- `--tooling / --no-tooling`: generate `default.project.json`, `selene.toml`, and `wally.toml`. Default should match `generateToolingFiles = true`.
- `--dry-run`: parse the place and print planned counts without writing files.
- `--json`: emit machine-readable summary for CI.
- `--quiet`: suppress progress output except errors.

## Output Format Requirements

The output must match the server finalizer, not a new importer-specific layout.

Example output:

```text
MyGame/
  rbxsync.json
  src/
    Workspace/
      _meta.rbxjson
      Baseplate.rbxjson
      Terrain/
        _meta.rbxjson
        terrain.rbxjson
    ServerScriptService/
      _meta.rbxjson
      Main.server.luau
      Main.rbxjson
    ReplicatedStorage/
      Shared/
        _meta.rbxjson
      Shared.luau
  default.project.json
  selene.toml
  wally.toml
```

The importer must preserve these existing conventions:

- `Script` source file suffix: `.server.luau`.
- `LocalScript` source file suffix: `.client.luau`.
- `ModuleScript` source file suffix: `.luau`.
- Container metadata filename: `_meta.rbxjson`.
- Leaf metadata filename: `<InstanceName>.rbxjson`.
- `Name`, `className`, `referenceId`, `parentId`, `path`, `properties`, `attributes`, `tags`, and special fields such as `materialOverrides` should follow the plugin serialized shape where possible.
- Script `.rbxjson` must omit `Source`; the source file is the editable source of truth.
- Instance names containing `/` must use the same `[SLASH]` path escaping as `Serializer.luau`.
- Duplicate sibling names must be disambiguated deterministically and must not overwrite each other. The preferred path suffix is compatible with current plugin path generation: `Name~<shortRef>`. If the shared writer still applies its own duplicate handling, document and preserve that exact behavior.

## Functional Requirements

### Input Loading

The command must detect format by extension:

- `.rbxl`: parse with `rbx_binary`.
- `.rbxlx`: parse with `rbx_xml`.

If extension detection fails, the command may sniff the file header, but it should still produce a clear unsupported-format error for invalid inputs.

### DOM Traversal

The importer must traverse the loaded `WeakDom` and produce a flat list of serialized instances equivalent to the plugin chunks. For each included service:

1. Include the service instance itself.
2. Include all descendants in parent-before-child order.
3. Build a DataModel path from service root to descendant.
4. Escape path segments using `[SLASH]`.
5. Generate stable `referenceId` strings from DOM referents. These do not need to be UUIDs, but they must be unique within the import and must be used consistently for `parentId` and `Ref` properties.

### Property Conversion

Implement a `Variant -> serde_json::Value` converter that mirrors `plugin/src/Serializer.luau` and is the inverse of the current `json_to_variant` build helper. It should live in shared code, not only in `main.rs`.

Minimum P0 property support:

- `String`, `Bool`, `Int32`, `Int64`, `Float32`, `Float64`
- `Vector2`, `Vector2int16`, `Vector3`, `Vector3int16`
- `CFrame`
- `Color3`, `Color3uint8`, `BrickColor`
- `UDim`, `UDim2`, `Rect`
- `NumberRange`
- `Enum`
- `Ref`
- `Content` and asset identifiers
- `ProtectedString` or string variants used for script `Source`
- `BinaryString` and `SharedString` as base64 or stable references if exposed by the DOM library

Additional supported variants should be added as the crate exposes them: sequences, fonts, faces, axes, physical properties, rays, regions, optional CFrames, security capabilities, and unique IDs.

Unknown property types must not crash the import. They should be skipped with a warning count, and `--json` output should include skipped property totals.

### Script Extraction

For `Script`, `LocalScript`, and `ModuleScript`, extract the `Source` property into the correct `.luau` file. The corresponding instance JSON must retain all other properties but omit `Source`. Empty scripts should still produce an empty source file if the place contains a script instance.

If source cannot be read from the file, write metadata only and report the script in a warning summary.

### Config Generation

If `rbxsync.json` does not exist, create one using `ProjectConfig` defaults:

```json
{
  "name": "MyGame",
  "tree": "./src",
  "assets": "./assets",
  "config": {
    "generateToolingFiles": true
  }
}
```

If `rbxsync.json` exists, preserve it and use its `treeMapping`, package preservation settings, and tooling settings. The importer must not rewrite unrelated user configuration unless `--write-config` or equivalent is explicitly requested.

### Existing Directory Handling

Default behavior should match server extraction:

1. If `src/` exists and backup is enabled, move it to `.rbxsync-backup/src`.
2. If `.rbxsync-backup/src` exists, replace it.
3. If backup cannot be created, fail before deleting `src/`.
4. If `--force --no-backup` is provided, replace `src/` directly.

The command must never delete files outside the destination project directory.

### Summary Output

On success, print:

- Input file path and detected format.
- Output project directory.
- Total instances imported.
- Scripts written.
- `.rbxjson` files written.
- Services imported.
- Warnings count.
- Generated tooling files.

With `--json`, output a JSON object containing the same values and no decorative text.

## Architecture

### Shared Writer

Move the server finalization writer into reusable code, for example:

```text
rbxsync-core/src/extract_writer.rs
```

Suggested API:

```rust
pub struct ExtractWriterOptions {
    pub project_dir: PathBuf,
    pub backup_existing_src: bool,
    pub force: bool,
    pub generate_tooling_files: bool,
    pub tree_mapping: HashMap<String, String>,
    pub preserve_packages: bool,
    pub packages_folder: String,
}

pub struct ExtractWriterSummary {
    pub total_instances: usize,
    pub files_written: usize,
    pub scripts_written: usize,
    pub services: Vec<String>,
    pub warnings: Vec<String>,
}

pub async fn write_serialized_instances(
    instances: Vec<serde_json::Value>,
    options: ExtractWriterOptions,
) -> anyhow::Result<ExtractWriterSummary>;
```

Then update `rbxsync-server::handle_extract_finalize` to call this shared writer after reading chunks. The new CLI command should call the same writer after parsing the place file. This is the most important implementation requirement for format parity.

### Import Parser

Add an importer module, for example:

```text
rbxsync-core/src/place_importer.rs
```

Responsibilities:

- Read `.rbxl` and `.rbxlx`.
- Traverse `WeakDom`.
- Convert DOM instances to plugin-compatible serialized JSON values.
- Return `Vec<serde_json::Value>` plus diagnostics.

Suggested API:

```rust
pub struct PlaceImportOptions {
    pub input_path: PathBuf,
    pub services: Option<HashSet<String>>,
    pub include_terrain: bool,
}

pub struct PlaceImportResult {
    pub instances: Vec<serde_json::Value>,
    pub diagnostics: Vec<ImportDiagnostic>,
}

pub fn import_place_file(options: PlaceImportOptions) -> anyhow::Result<PlaceImportResult>;
```

### CLI Integration

Add a `Commands::ImportPlace` variant in `rbxsync-cli/src/main.rs`. The command should:

1. Resolve input and output paths.
2. Load or create `rbxsync.json`.
3. Parse the place file with `place_importer`.
4. If `--dry-run`, print summary and exit.
5. Call `write_serialized_instances`.
6. Print human or JSON summary.

## Validation Plan

### Unit Tests

Add tests for:

- Path escaping and duplicate sibling disambiguation.
- DOM referent to `referenceId` mapping.
- `Variant -> JSON` conversion for every supported property type.
- Script suffix selection.
- Existing `src/` backup behavior.
- `treeMapping` application.

### Fixture Tests

Create small test fixtures under a new directory such as `testing/fixtures/place-import/`:

- Basic place with Workspace part and service metadata.
- Scripts in `ServerScriptService`, `StarterPlayerScripts`, and `ReplicatedStorage`.
- Duplicate sibling names.
- Nested scripts with child instances.
- Instance reference properties, such as constraints or ObjectValues.
- Names containing `/`.
- `.rbxlx` XML fixture.

### Round-Trip Tests

Use the existing build path as an oracle:

1. Start with an RbxSync `src/` fixture.
2. Run `rbxsync build -f rbxl -o fixture.rbxl`.
3. Run `rbxsync import-place fixture.rbxl --output imported --force`.
4. Compare normalized `src/` output against the original fixture.

Some property-level differences may be expected because the current build path does not support every serialized type. These must be documented in test snapshots rather than hidden.

### Manual Acceptance

For a real saved game:

1. Run `rbxsync import-place Game.rbxl --output GameProject`.
2. Open `GameProject` in VS Code.
3. Confirm scripts are editable as Luau files.
4. Run `rbxsync build --path GameProject -o build/game.rbxl`.
5. Open the built place in Studio and confirm the hierarchy and scripts load.

## Acceptance Criteria

- `rbxsync import-place ./Game.rbxl --output ./GameProject` creates a usable RbxSync project without Studio running.
- Output uses the same `src/` layout as server extraction.
- Existing `src/` is backed up before replacement unless explicitly disabled.
- `rbxsync.json` is created when missing and preserved when present.
- Script source is extracted into Luau files and omitted from script metadata JSON.
- At least the P0 property types are preserved.
- Duplicate names and slash-containing names do not overwrite files.
- `make check` passes.
- New importer tests pass in CI without Roblox Studio.

## Risks and Constraints

- Place files may not contain every live property that Studio reflection can read. The importer should preserve file data faithfully and warn about unsupported variants.
- Terrain parity may be limited by what `rbx_binary` and `rbx_xml` expose. If full terrain extraction cannot be implemented from place files, P0 should preserve Terrain instance metadata and report voxel import as unsupported.
- Some asset content may be references only, not embedded data. The importer should preserve asset IDs and content URIs first; binary asset downloading can be a later feature.
- The current build path has a limited `json_to_variant` converter. Round-trip parity may require improving both import and build converters.
- Shared writer extraction is required to avoid divergence. Duplicating server finalizer logic in the CLI should be rejected.

## Milestones

### Milestone 1: Shared Writer Refactor

- Extract server finalize file-writing logic into `rbxsync-core`.
- Keep `/extract/finalize` behavior unchanged.
- Add tests for writer behavior using serialized JSON fixtures.

### Milestone 2: Local Place Import MVP

- Add `.rbxl` and `.rbxlx` parsing.
- Implement DOM traversal and P0 property conversion.
- Add `rbxsync import-place`.
- Write scripts and metadata using the shared writer.
- Add fixture and round-trip tests.

### Milestone 3: Parity Improvements

- Expand property type coverage.
- Improve reference property handling.
- Add better diagnostics for skipped or lossy properties.
- Validate large place performance.

### Milestone 4: Published Place Import

- Add `--place-id`.
- Support authenticated place download through Roblox Open Cloud or another documented API path.
- Cache downloads in `.rbxsync/`.
- Add rate-limit, permission, and API-key error handling.

## Open Questions

1. Should published place import be part of the first implementation, or is local `.rbxl`/`.rbxlx` sufficient for MVP?
2. Should default property inflation be attempted using the Roblox API dump, or should the importer only serialize properties present in the place file?
3. Should `.rbxm` and `.rbxmx` model import be accepted under the same command after place import lands?
4. Should `rbxsync build` be upgraded in the same PR to improve round-trip property coverage, or should importer parity be scoped to the current build behavior first?
