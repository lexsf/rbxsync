# Implement Command-Line Place Import

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` at the repository root. If this document is revised during implementation, keep it self-contained and update every affected section before stopping work.

## Purpose / Big Picture

After this change, a user with a local Roblox place file can run a command such as `rbxsync import-place ./Game.rbxl --output ./GameProject` and receive the same editable RbxSync project layout that `rbxsync extract` currently creates through Roblox Studio and the Studio plugin. This lets developers, CI jobs, and coding agents convert saved `.rbxl` and `.rbxlx` files into `src/` files without opening Studio, installing the plugin, or running the live sync server.

The visible proof of success is a generated project directory with `rbxsync.json`, `src/Workspace`, service folders, `.rbxjson` instance metadata, `.luau` script files, and generated tooling files. The command must use the same writer semantics as server extraction so future changes to the RbxSync file format do not split into two incompatible paths.

## Progress

- [x] (2026-05-07 17:32Z) Read `PLANS.md` and `COMMAND-LINE-PRD.md`, then inspected the current server extraction finalizer and CLI build path to ground this plan in repository code.
- [x] (2026-05-07 17:32Z) Created this initial ExecPlan with milestones for shared writer extraction, local place parsing, CLI integration, parity tests, and later published-place import.
- [x] (2026-05-07 17:45Z) Refactored server extraction writing into shared `rbxsync-core::extract_writer` code while keeping `/extract/finalize` response fields and server-side cleanup in `rbxsync-server`.
- [x] (2026-05-07 17:45Z) Added a focused core writer test covering service `_meta.rbxjson`, leaf `.rbxjson`, script source output, source removal from script metadata, and generated tooling files.
- [x] (2026-05-07 17:45Z) Ran `git diff --check`; it passed with no whitespace errors.
- [x] (2026-05-07 17:47Z) Rechecked Milestone 1 after the explicit implementation request; the shared writer files remain in place and `git diff --check` still passes.
- [x] (2026-05-07 17:58Z) After `mise use rust@stable`, verified `cargo 1.95.0` and `rustc 1.95.0` are available through `mise exec`.
- [x] (2026-05-07 17:58Z) Ran `mise exec -- rustfmt --edition 2021 --check rbxsync-core/src/extract_writer.rs`; it passed after formatting the new shared writer file.
- [x] (2026-05-07 17:58Z) Ran `mise exec -- cargo test -p rbxsync-core`; it passed with 43 unit tests and 0 doc tests.
- [x] (2026-05-07 17:58Z) Ran `mise exec -- cargo test -p rbxsync-server`; it passed with 2 unit tests, 19 integration tests, and 0 doc tests.
- [x] (2026-05-07 18:16Z) Fixed the repo-wide formatting drift by running `mise exec -- cargo fmt`; `mise exec -- cargo fmt -- --check` now passes.
- [x] (2026-05-07 18:16Z) Fixed the repo-wide CLI compile issue by adding the missing `wait_for_port_release` helper in `rbxsync-cli/src/main.rs`; `mise exec -- cargo test -p rbxsync` now passes.
- [x] (2026-05-07 18:16Z) Ran `mise exec -- cargo test --workspace`; it passed for `rbxsync`, `rbxsync-core`, `rbxsync-mcp`, `rbxsync-server`, server integration tests, and doc tests. The MCP crate still emits existing dead-code warnings.
- [x] (2026-05-07 19:05Z) Implemented Milestone 2 local `.rbxl` and `.rbxlx` import in `rbxsync-core::place_importer`, including format detection, DOM traversal, service filtering, terrain skipping, path escaping, duplicate sibling suffixes, attributes, tags, and typed property conversion.
- [x] (2026-05-07 19:05Z) Exported `import_place_file`, `PlaceImportOptions`, `PlaceImportResult`, `PlaceFileFormat`, and `ImportDiagnostic` from `rbxsync-core`.
- [x] (2026-05-07 19:05Z) Added importer unit coverage for slash escaping, duplicate path suffixes, script `Source` preservation, typed properties, attributes, tags, `.rbxlx` file loading, service filtering, and terrain exclusion.
- [x] (2026-05-07 19:05Z) Ran `mise exec -- cargo test -p rbxsync-core`; it passed with 47 unit tests and 0 doc tests.
- [x] (2026-05-07 19:05Z) Ran `mise exec -- cargo test -p rbxsync-server`, `mise exec -- cargo test -p rbxsync`, `mise exec -- cargo fmt -- --check`, and `git diff --check`; all passed.
- [x] (2026-05-07 19:08Z) Ran `mise exec -- cargo test --workspace`; it passed all workspace test targets and doc tests. Existing MCP dead-code warnings remain non-failing.
- [x] (2026-05-07 19:18Z) Implemented Milestone 3 CLI wiring for `rbxsync import-place`, including input/output/name resolution, service filtering, terrain opt-in, `--force`, backup controls, tooling controls, `--dry-run`, `--json`, and `--quiet`.
- [x] (2026-05-07 19:18Z) Connected the CLI to `rbxsync_core::import_place_file` and `rbxsync_core::write_serialized_instances`, preserving existing `rbxsync.json` files and creating a default `ProjectConfig` when missing.
- [x] (2026-05-07 19:18Z) Added clean machine-readable JSON output for `--json` by lowering RbxSync tracing for JSON and quiet imports.
- [x] (2026-05-07 19:18Z) Ran an end-to-end smoke: built a temporary RbxSync source tree to `.rbxlx`, imported it with `rbxsync import-place --force --json`, validated the JSON summary with `jq`, and verified `rbxsync.json`, service metadata, script source, and leaf `.rbxjson` files.
- [x] (2026-05-07 19:18Z) Ran `mise exec -- cargo test -p rbxsync-core`, `mise exec -- cargo test -p rbxsync`, `mise exec -- cargo test -p rbxsync-server`, `mise exec -- cargo test --workspace`, `mise exec -- cargo fmt -- --check`, and `git diff --check`; all passed.
- [x] (2026-05-07 19:31Z) Implemented Milestone 4 writer coverage for existing `src/` backup, `treeMapping`, package preservation from backup, script source output, service metadata, and disabled tooling generation.
- [x] (2026-05-07 19:31Z) Added CLI integration coverage in `rbxsync-cli/tests/import_place.rs` that builds a temporary project with the real `rbxsync build` command, imports binary `.rbxl` with `rbxsync import-place --force --json`, and verifies generated project files.
- [x] (2026-05-07 19:31Z) Added XML `.rbxlx` dry-run integration coverage that proves `import-place --dry-run --json` parses and reports counts without creating the output project.
- [x] (2026-05-07 19:31Z) Ran `mise exec -- cargo test -p rbxsync-core`, `mise exec -- cargo test -p rbxsync --test import_place`, `mise exec -- cargo test -p rbxsync`, `mise exec -- cargo test -p rbxsync-server`, `mise exec -- cargo test --workspace`, and `git diff --check`; all passed.
- [x] (2026-05-07 19:48Z) Implemented Milestone 5 import parity and diagnostic polish: binary string and shared string property conversion, typed diagnostic kinds, JSON `diagnosticSummary`, human-readable diagnostic summaries, missing requested-service diagnostics, missing script `Source` diagnostics, and terrain voxel-data limitation diagnostics.
- [x] (2026-05-07 19:48Z) Added regression coverage for binary/shared string conversion, missing service/script/terrain diagnostics, and CLI `diagnosticSummary` output.
- [x] (2026-05-07 19:48Z) Measured a larger generated fixture: 300 part metadata files and 20 scripts built to a 59.7 KB `.rbxlx`; `target/debug/rbxsync import-place --force --quiet` completed in about 0.10s wall time and produced 323 `.rbxjson` files plus 20 `.luau` files.
- [x] (2026-05-07 19:48Z) Decided published `--place-id` import stays out of this implementation branch and remains a follow-up requiring authenticated download design.
- [x] (2026-05-07 19:48Z) Ran `mise exec -- cargo test -p rbxsync-core`, `mise exec -- cargo test -p rbxsync`, `mise exec -- cargo test -p rbxsync-server`, `mise exec -- cargo test --workspace`, `mise exec -- cargo fmt -- --check`, and `git diff --check`; all passed.

## Surprises & Discoveries

- Observation: Server extraction currently has two phases that both touch existing `src/`: `handle_extract_start` backs up and clears `src/`, and `handle_extract_finalize` also backs up and clears `src/` after chunks arrive.
  Evidence: `rbxsync-server/src/lib.rs` contains the pre-extract cleanup in `handle_extract_start` and the final writer in `handle_extract_finalize`. The shared writer must define one authoritative backup point for CLI import while preserving the server behavior users already rely on.

- Observation: The existing build path is already the reverse of the requested import path, but it lives in `rbxsync-cli/src/main.rs` rather than shared code and its `json_to_variant` converter is partial.
  Evidence: `cmd_build`, `do_build`, `build_dom_from_src`, and `json_to_variant` in `rbxsync-cli/src/main.rs` write `.rbxl`, `.rbxm`, `.rbxlx`, and `.rbxmx` using `rbx_binary`, `rbx_xml`, and `rbx_dom_weak`.

- Observation: This shell does not currently have a Rust toolchain on PATH, and `mise` is not configured with Rust for this repository.
  Evidence: `cargo fmt -- --check` failed with `zsh:1: command not found: cargo`; `which cargo` and `which rustc` reported not found; `mise exec -- cargo fmt -- --check` failed with `mise ERROR "cargo" couldn't exec process: No such file or directory`; `mise ls` listed only Elixir and Erlang.

- Observation: The Rust toolchain is now available through `mise`, and the earlier repo-wide blockers have been repaired.
  Evidence: `mise exec -- cargo --version` reported `cargo 1.95.0`; `mise exec -- rustc --version` reported `rustc 1.95.0`; `mise exec -- cargo test -p rbxsync-core` passed 43 tests; `mise exec -- cargo test -p rbxsync-server` passed 21 total tests; `mise exec -- cargo test -p rbxsync` now passes after adding `wait_for_port_release`; `mise exec -- cargo fmt -- --check` now passes after applying `cargo fmt`.

- Observation: Full-workspace tests pass after the shared writer refactor and repo-wide fixes, with non-failing warnings in the MCP binary test target.
  Evidence: `mise exec -- cargo test --workspace` passed all Rust test targets and doc tests. The remaining output is warning-only dead code in `rbxsync-mcp/src/tools/mod.rs` for test response structs and methods.

- Observation: The Roblox file readers needed for Milestone 2 are available directly from the existing crates.
  Evidence: `rbx_binary::from_reader` reads `.rbxl`; `rbx_xml::from_reader_default` reads `.rbxlx`; `rbx_xml` was added to `rbxsync-core` so the importer can live beside the shared writer instead of in the CLI.

- Observation: `rbx_dom_weak` stores enum properties as numeric `Enum` values, not reflection-backed enum names.
  Evidence: `Variant::Enum` exposes `to_u32()` only. The importer emits plugin-shaped `{ type = "Enum", value = { enumType = null, value = <number> } }`, preserving the value for the current build path while leaving richer enum names to a later reflection-backed parity pass.

- Observation: Some Roblox binary-backed variants did not have a current RbxSync JSON writer contract at the end of Milestone 2.
  Evidence: Milestone 5 added `BinaryString` and `SharedString` JSON conversion. `MaterialColors` remains diagnostic-only because the current project JSON contract does not define a stable editable representation for it. Required P0 variants from Milestone 2 are converted, and additional common variants such as sequences, fonts, faces, axes, physical properties, rays, regions, optional CFrames, unique IDs, security capabilities, attributes, and tags are also handled.

- Observation: `--json` needs quieter logging than the default CLI because the shared writer emits info-level tracing during successful imports.
  Evidence: The first smoke import produced a valid JSON summary but also showed `rbxsync_core::extract_writer` info logs in the combined command output. CLI startup now parses the command before initializing tracing and uses `rbxsync=warn` for `import-place --json` and `import-place --quiet`.

- Observation: Milestone 3 can validate the command without Roblox Studio by using the existing build command as a fixture producer.
  Evidence: A temporary project with `Workspace/Baseplate.rbxjson` and `ServerScriptService/Main.server.luau` was built via `cargo run -p rbxsync -- build --format rbxlx`, then imported via `cargo run -p rbxsync -- import-place <file> --output <dir> --force --json`; the output JSON reported 4 instances, 2 services, 1 script, and 4 `.rbxjson` files, and the expected files existed under the imported project.

- Observation: CLI integration tests can run the built `rbxsync` binary directly without shelling through Cargo or requiring Studio.
  Evidence: `rbxsync-cli/tests/import_place.rs` uses `CARGO_BIN_EXE_rbxsync`, sets `RBXSYNC_VERSION_CHECK=1` to suppress duplicate-installation checks, builds temporary `.rbxl`/`.rbxlx` fixtures with the real CLI, and validates import output from clean JSON stdout.

- Observation: The shared writer already has the server-style backup behavior needed by `import-place`; Milestone 4 now pins it with a focused regression test.
  Evidence: `extract_writer::tests::backs_up_existing_src_applies_tree_mapping_and_preserves_packages` creates an existing `src/`, writes a package folder under `ReplicatedStorage/Packages`, runs the writer with `treeMapping` and package preservation, then verifies `.rbxsync-backup/src`, mapped script output, restored packages, and disabled tooling behavior.

- Observation: Binary-backed properties can be represented without adding a new artifact layout.
  Evidence: `Variant::BinaryString` is now emitted as `{ type = "BinaryString", value = <base64> }`; `Variant::SharedString` is emitted as `{ type = "SharedString", value = { hash, file = null, data = <base64> } }`. `MaterialColors` still remains diagnostic-only because the current RbxSync project JSON contract does not define a stable editable representation for it.

- Observation: Import diagnostics need machine-readable categories for CI and concise human summaries.
  Evidence: `ImportDiagnosticKind` now distinguishes `unsupportedProperty`, `unsupportedAttribute`, `missingService`, `missingScriptSource`, and `unsupportedTerrainVoxelData`. `rbxsync import-place --json` includes `diagnosticSummary`, and human output prints grouped warning counts before sample diagnostics.

- Observation: Larger local imports are fast enough for the MVP path, but this is only a smoke measurement.
  Evidence: A generated fixture with 300 parts and 20 scripts built to a 59.7 KB `.rbxlx`; importing it with the debug binary and `--force --quiet` took about 0.10s wall time and produced 323 `.rbxjson` files and 20 `.luau` files. This measures local conversion overhead, not very large production places or release-mode performance.

## Decision Log

- Decision: Implement the MVP as a new `rbxsync import-place` subcommand in `rbxsync-cli`, not as a separate binary.
  Rationale: The CLI already owns user-facing file conversion commands and already depends on the Roblox DOM libraries needed to read and write place files. A separate binary would duplicate command wiring and packaging.
  Date/Author: 2026-05-07 / Codex

- Decision: Move the extraction file writer into `rbxsync-core` before adding the importer.
  Rationale: `COMMAND-LINE-PRD.md` explicitly requires output parity with server extraction. Sharing the writer is the simplest way to prevent server extraction and place-file import from drifting.
  Date/Author: 2026-05-07 / Codex

- Decision: Treat local `.rbxl` and `.rbxlx` import as the MVP; published `--place-id` import is a later milestone unless the user explicitly promotes it.
  Rationale: Published place import requires authenticated download behavior and external API handling, while the immediate user request asks for a place or `.rbxl` file and can be satisfied with local file parsing first.
  Date/Author: 2026-05-07 / Codex

- Decision: Do not implement published `--place-id` import in this branch.
  Rationale: Local file import is now implemented, validated, and covered. Published place import needs Open Cloud authentication, permissions, rate-limit and error handling, and potentially separate UX for API keys; adding that now would broaden a completed local-file import branch.
  Date/Author: 2026-05-07 / Codex

## Outcomes & Retrospective

The initial plan converted `COMMAND-LINE-PRD.md` into a restartable sequence of repository edits and validation steps.

Milestone 1 implementation completed the shared writer refactor. `rbxsync-core/src/extract_writer.rs` now owns the filesystem extraction writer, `rbxsync-core/src/lib.rs` re-exports the writer API, and `rbxsync-server/src/lib.rs::handle_extract_finalize` now delegates file output to `rbxsync_core::write_serialized_instances` while retaining chunk reading, session finalization, operation-state cleanup, and file watcher cleanup.

After configuring Rust with `mise`, Milestone 1 targeted validation passed for `rbxsync-core` and `rbxsync-server`. A follow-up repo-wide cleanup also fixed the global formatter drift and the missing CLI `wait_for_port_release` helper. `mise exec -- cargo test --workspace`, `mise exec -- cargo fmt -- --check`, and `git diff --check` now pass. The only remaining validation noise is non-failing dead-code warnings in `rbxsync-mcp/src/tools/mod.rs`.

Milestone 2 implementation completed the local place parser in core. `rbxsync-core/src/place_importer.rs` now reads `.rbxl` and `.rbxlx` into a `WeakDom`, serializes selected services in parent-before-child order, preserves script `Source` for the shared writer, and produces diagnostics for unsupported property variants instead of failing the import. CLI wiring remains Milestone 3, so no user-facing `rbxsync import-place` command exists yet. Full workspace tests pass after this milestone.

Milestone 3 implementation completed the user-facing CLI path. `rbxsync import-place` now resolves input and output paths, creates or preserves `rbxsync.json`, applies config-derived tree mapping, package preservation, and tooling settings, writes through the shared extraction writer, and supports human or clean JSON summaries. The CLI requires `--force` before replacing an existing `src/`; by default it keeps server-style backup behavior, while `--no-backup --force` directly replaces `src/` inside the selected output directory.

Milestone 4 implementation added CI-compatible regression coverage for the importer path. Core tests now pin writer backup, tree mapping, package preservation, and tooling behavior. CLI integration tests now prove both binary `.rbxl` import and XML `.rbxlx` dry-run behavior through the real `rbxsync` binary without Roblox Studio.

Milestone 5 implementation completed the MVP polish pass. Importer diagnostics are now structured, summarized in CLI output, and cover missing requested services, missing script source, terrain voxel limitations, and unsupported variants. Binary and shared string properties now serialize to JSON instead of being skipped. Published place download remains explicitly deferred.

## Context and Orientation

RbxSync converts Roblox games between a live Roblox Studio DataModel and a local filesystem representation. A DataModel is the tree of Roblox services and instances that make up a game. A service is a top-level Roblox container such as `Workspace`, `ReplicatedStorage`, or `ServerScriptService`. An instance is any Roblox object in that tree, such as a `Part`, `Folder`, `Script`, or `ModuleScript`.

The current Studio-to-files path starts in `rbxsync-cli/src/main.rs` with `cmd_extract`. That command talks to the HTTP server in `rbxsync-server/src/lib.rs`. The server queues an `extract:start` command for the Studio plugin. The plugin code in `plugin/src/init.server.luau` collects services, serializes each Roblox instance with `plugin/src/Serializer.luau`, sends chunks to `/extract/chunk`, optionally sends terrain to `/extract/terrain`, and calls `/extract/finalize`.

The important server writer is currently embedded inside `handle_extract_finalize` in `rbxsync-server/src/lib.rs`. It reads serialized instances from temporary chunk files, loads `rbxsync.json`, applies `treeMapping`, backs up existing `src/`, writes script source files, writes `.rbxjson` metadata files, creates service folders, restores preserved package folders, and generates tooling files.

The requested command must bypass Studio and the plugin. It should read a saved Roblox place file directly. Roblox binary place files use the `.rbxl` extension. Roblox XML place files use the `.rbxlx` extension. The repository already writes these formats in `cmd_build` using the `rbx_binary`, `rbx_xml`, and `rbx_dom_weak` crates. The new importer should use the same crates to read a place file into a `WeakDom`, which is the in-memory Roblox object tree type from `rbx_dom_weak`.

The existing local project format is documented in `README.md` and shown by server extraction. Scripts become editable Luau source files. `Script` uses `.server.luau`, `LocalScript` uses `.client.luau`, and `ModuleScript` uses `.luau`. Non-script and metadata data lives in `.rbxjson`. A folder or container instance is represented as a directory with `_meta.rbxjson`; a leaf instance is represented as `<InstanceName>.rbxjson`.

## Plan of Work

The first milestone is a refactor only. Add a shared module in `rbxsync-core`, tentatively `rbxsync-core/src/extract_writer.rs`, and move the file-writing behavior from `rbxsync-server/src/lib.rs::handle_extract_finalize` into it. The public API should accept a vector of plugin-compatible serialized instance JSON values and an options struct containing the project directory, tree mapping, backup behavior, package preservation behavior, and tooling generation behavior. Keep server-specific state management, chunk reading, operation state cleanup, and file watcher cleanup in the server. The server should call the shared writer after it has read the chunks.

The second milestone implements the parser. Add `rbxsync-core/src/place_importer.rs`. It should load `.rbxl` with `rbx_binary` and `.rbxlx` with `rbx_xml`, traverse the resulting `WeakDom`, and produce the same flat instance JSON shape as `Serializer.serializeInstance` in `plugin/src/Serializer.luau`. This module owns path generation, slash escaping with `[SLASH]`, stable reference ID assignment from DOM referents, parent ID assignment, service filtering, and conversion from `rbx_dom_weak::types::Variant` to RbxSync typed JSON property values.

The third milestone wires the CLI. Add a `Commands::ImportPlace` variant in `rbxsync-cli/src/main.rs`. The command should accept an input path, output directory, project name, service list, backup options, `--force`, `--dry-run`, `--json`, and tooling controls. It should load or create `rbxsync.json`, call the importer, and call the shared writer unless `--dry-run` is set. On success it prints the input, output, instance count, script count, `.rbxjson` count, imported services, warnings, and generated tooling files.

The fourth milestone adds tests and fixtures. Unit tests should cover path escaping, duplicate sibling disambiguation, reference mapping, property conversion, script suffix selection, backup behavior, and `treeMapping`. Fixture tests should build a tiny RbxSync source tree to `.rbxl` and `.rbxlx`, import it into a temporary directory, and compare normalized output. The comparison should focus first on the supported MVP property types and explicitly document known limitations instead of hiding differences.

The fifth milestone improves parity after the MVP is demonstrably working. Expand property conversion, improve diagnostics for skipped property types, measure large-place behavior, and decide whether to enhance the current `json_to_variant` build converter at the same time. Published place download should stay out of the MVP unless a decision is recorded in the Decision Log.

## Concrete Steps

Work from the repository root:

    cd /Users/lexiviripaeff/Documents/LoganX/rbxsync

Before editing, inspect the exact current APIs exposed by the Roblox crates in the local Cargo cache or generated docs, because this plan intentionally does not assume the exact reader function names. Search first:

    rg "from_reader|from_reader_default|to_writer" ~/.cargo/registry/src -g '*.rs' | rg "rbx_(binary|xml)"

If the local Cargo registry is absent, use `cargo test -p rbxsync --no-run` to fetch/build dependencies if the environment permits network access, then repeat the search. Record any API discoveries in `Surprises & Discoveries`.

Milestone 1, shared writer:

1. Create `rbxsync-core/src/extract_writer.rs`.
2. Move or recreate these helpers from `rbxsync-server/src/lib.rs` into core with public or crate-visible names as needed: `apply_tree_mapping`, `copy_dir_recursive`, package path normalization, known service mapping, tooling generation, project JSON generation, and the write operation preparation logic.
3. Define an API similar to:

        pub struct ExtractWriterOptions {
            pub project_dir: PathBuf,
            pub backup_existing_src: bool,
            pub force_replace_src: bool,
            pub tree_mapping: HashMap<String, String>,
            pub preserve_packages: bool,
            pub packages_folder: String,
            pub generate_tooling_files: bool,
            pub project_name: Option<String>,
            pub restore_terrain_data: Option<String>,
        }

        pub struct ExtractWriterSummary {
            pub total_instances: usize,
            pub files_written: usize,
            pub scripts_written: usize,
            pub services: Vec<String>,
            pub packages_preserved: bool,
            pub warnings: Vec<String>,
        }

        pub async fn write_serialized_instances(
            instances: Vec<serde_json::Value>,
            options: ExtractWriterOptions,
        ) -> anyhow::Result<ExtractWriterSummary>

4. Export the module from `rbxsync-core/src/lib.rs`.
5. Update `rbxsync-server/src/lib.rs::handle_extract_finalize` to read chunks, load config, compute writer options, call `rbxsync_core::write_serialized_instances`, and return the same JSON response fields it returns today.
6. Keep server-only cleanup in the server: marking the extraction session finalized, clearing operation state, draining file watcher events, and resuming live sync.

Milestone 2, importer:

1. Create `rbxsync-core/src/place_importer.rs` and export it from `rbxsync-core/src/lib.rs`.
2. Define:

        pub struct PlaceImportOptions {
            pub input_path: PathBuf,
            pub services: Option<HashSet<String>>,
            pub include_terrain: bool,
        }

        pub struct PlaceImportResult {
            pub instances: Vec<serde_json::Value>,
            pub diagnostics: Vec<ImportDiagnostic>,
            pub format: PlaceFileFormat,
        }

3. Implement format detection for `.rbxl` and `.rbxlx`.
4. Implement a DOM traversal that includes the selected service and all descendants in parent-before-child order.
5. Implement `escape_path_segment` by replacing `/` with `[SLASH]`, matching `plugin/src/Serializer.luau`.
6. Implement deterministic duplicate handling for siblings with the same name. Prefer suffixing duplicate path segments with `~<shortRef>` so duplicate files cannot overwrite each other.
7. Implement `variant_to_json_property`. At minimum support string, bool, 32-bit and 64-bit integers, 32-bit and 64-bit floats, `Vector2`, `Vector2int16`, `Vector3`, `Vector3int16`, `CFrame`, `Color3`, `Color3uint8`, `BrickColor`, `UDim`, `UDim2`, `Rect`, `NumberRange`, `Enum`, `Ref`, and script source string values. Unknown variants should add diagnostics and skip that property.
8. For scripts, keep `Source` in the serialized instance properties so the shared writer can write the source file and remove `Source` from `.rbxjson`, matching current server finalization behavior.

Milestone 3, CLI:

1. Add the `ImportPlace` variant to `Commands` in `rbxsync-cli/src/main.rs`.
2. Add a `cmd_import_place` async function.
3. Resolve project directory from `--output` or current directory. Resolve project name from `--name`, existing `rbxsync.json`, or input file stem.
4. If `rbxsync.json` is missing, write a minimal `ProjectConfig` JSON before calling the writer. If it exists, preserve it.
5. Load `treeMapping`, package preservation, and tooling settings from existing config using the same semantics as server extraction.
6. If `--dry-run` is present, print planned counts and do not write `src/`.
7. If writing, call `write_serialized_instances`.
8. Print either human-readable output or clean JSON for `--json`.

Milestone 4, tests:

1. Add unit tests inside `rbxsync-core/src/extract_writer.rs` for writer behavior using temporary directories.
2. Add unit tests inside `rbxsync-core/src/place_importer.rs` for path escaping, duplicate names, reference IDs, and property conversion.
3. Add an integration test that creates a small source tree in a temporary directory, runs the existing build helper or CLI to create `.rbxl`, imports it, and verifies files exist with expected contents.
4. Add an `.rbxlx` test for XML parsing.
5. If using CLI integration tests, place them under the appropriate crate test directory and avoid requiring Roblox Studio.

Milestone 5, parity and polish:

1. Expand property conversion beyond P0.
2. Add warning summaries for skipped variants, missing script source, unsupported terrain voxel data, and unknown services.
3. Add `--quiet` and polish `--json` output if not already complete.
4. Measure import performance on a larger fixture and record results in `Surprises & Discoveries`.
5. Decide in this plan whether published place import belongs in the same implementation branch.

## Validation and Acceptance

Run these commands from the repository root after each milestone:

    cargo fmt -- --check
    cargo test -p rbxsync-core
    cargo test -p rbxsync-server
    cargo test -p rbxsync

After the CLI exists, run a local end-to-end command against a fixture place file:

    cargo run -p rbxsync -- import-place testing/fixtures/place-import/basic.rbxl --output /tmp/rbxsync-import-basic --force

Expected human-readable output should include the input path, output path, total instances, scripts written, `.rbxjson` files written, and imported services. Then verify files:

    test -f /tmp/rbxsync-import-basic/rbxsync.json
    test -d /tmp/rbxsync-import-basic/src/Workspace
    find /tmp/rbxsync-import-basic/src -name '*.luau' -o -name '*.rbxjson' | sort | head

Expected behavior is that the generated project contains service folders and script files without requiring Roblox Studio to be open.

Validate round-trip behavior:

    cargo run -p rbxsync -- build --path /tmp/rbxsync-import-basic --output /tmp/rbxsync-import-basic/build/game.rbxl

Expected behavior is that the build command succeeds and writes a non-empty `.rbxl` file.

Before completion, run:

    make check

Expected behavior is that Clippy, tests, and format checks all pass. If a test is environment-blocked, record the exact command and error in `Surprises & Discoveries` and explain the remaining risk in `Outcomes & Retrospective`.

Acceptance is complete when `rbxsync import-place ./Game.rbxl --output ./GameProject --force` creates a usable RbxSync project without Studio, the output layout matches server extraction conventions, existing `src/` backup behavior is covered by tests, and the new tests pass in CI-compatible commands.

## Idempotence and Recovery

The implementation must be safe to rerun. The importer should only write under the selected output project directory. It must not delete files outside that directory. When replacing `src/`, default to moving the prior tree to `.rbxsync-backup/src`, matching server extraction. If backup creation fails, fail before deleting `src/`.

During development, use temporary directories such as `/tmp/rbxsync-import-basic` for manual validation. They can be removed manually after inspection. Do not run destructive cleanup commands against repository paths unless the exact path is a generated temporary output and has been verified.

If Milestone 1 breaks server extraction tests, revert only the refactor changes for that milestone or repair the shared writer until the server response shape and file output match the old behavior. Do not continue to CLI import while server extraction is regressed.

If a property type is hard to convert, skip it with a diagnostic and keep the importer working. Record the skipped type in `Surprises & Discoveries` and add it to Milestone 5 unless it is part of the P0 property list.

## Artifacts and Notes

Important repository paths:

    COMMAND-LINE-PRD.md
    PLANS.md
    rbxsync-cli/src/main.rs
    rbxsync-core/src/lib.rs
    rbxsync-core/src/types/project.rs
    rbxsync-core/src/types/properties.rs
    rbxsync-server/src/lib.rs
    plugin/src/init.server.luau
    plugin/src/Serializer.luau

Current server extraction writer behavior to preserve:

    handle_extract_finalize reads chunk JSON values.
    It backs up existing src to .rbxsync-backup/src.
    It writes script source to .server.luau, .client.luau, or .luau.
    It removes Source from script .rbxjson.
    It writes containers as directories with _meta.rbxjson.
    It writes leaves as sibling .rbxjson files.
    It applies treeMapping from rbxsync.json.
    It generates default.project.json, selene.toml, and wally.toml.

Expected success transcript shape for the new command:

    Importing place: testing/fixtures/place-import/basic.rbxl
    Format: rbxl
    Output: /tmp/rbxsync-import-basic
    Imported 12 instances across 3 services
    Wrote 2 scripts and 12 .rbxjson files
    Generated default.project.json, selene.toml, wally.toml

The exact counts will depend on the fixture, but the output must provide this level of information.

## Interfaces and Dependencies

Use the existing Rust crates already present in this repository:

- `rbx_dom_weak` represents the Roblox object tree in memory. The importer reads from this tree and the build command writes from it.
- `rbx_binary` reads and writes binary Roblox files such as `.rbxl` and `.rbxm`.
- `rbx_xml` reads and writes XML Roblox files such as `.rbxlx` and `.rbxmx`.
- `serde_json` represents the plugin-compatible serialized instance shape consumed by the shared writer.
- `anyhow` should be used for user-facing error propagation in CLI and core helper APIs, consistent with existing CLI code.

At the end of Milestone 1, `rbxsync-core` should expose an extraction writer API equivalent to:

    pub mod extract_writer;
    pub use extract_writer::{
        write_serialized_instances,
        ExtractWriterOptions,
        ExtractWriterSummary,
    };

At the end of Milestone 2, `rbxsync-core` should expose a place importer API equivalent to:

    pub mod place_importer;
    pub use place_importer::{
        import_place_file,
        ImportDiagnostic,
        PlaceFileFormat,
        PlaceImportOptions,
        PlaceImportResult,
    };

At the end of Milestone 3, `rbxsync-cli` should support:

    rbxsync import-place <INPUT> --output <DIR> --force
    rbxsync import-place <INPUT> --output <DIR> --dry-run
    rbxsync import-place <INPUT> --output <DIR> --json

At the end of Milestone 5, `rbxsync import-place --json` includes:

    diagnosticCount
    diagnosticSummary
    diagnostics[].kind
    diagnostics[].path
    diagnostics[].property
    diagnostics[].message

## Revision Notes

2026-05-07 / Codex: Initial ExecPlan created from `COMMAND-LINE-PRD.md` and current repository inspection. The plan chooses local place-file import as MVP and requires shared writer extraction before CLI wiring to preserve output parity.

2026-05-07 / Codex: Milestone 1 implementation completed. Updated progress, discoveries, and retrospective to record the new shared writer module, server delegation, focused writer test, successful whitespace check, and blocked Rust validation due missing `cargo`/`rustc`.

2026-05-07 / Codex: Follow-up Milestone 1 verification pass completed after the user explicitly requested the milestone implementation again. No additional code changes were required; `git diff --check` still passes, and Rust validation remains blocked by the missing Rust toolchain.

2026-05-07 / Codex: Rust toolchain became available after `mise use rust@stable`. Ran targeted Milestone 1 validation: `rbxsync-core` tests passed, `rbxsync-server` tests passed, and the new writer file passes rustfmt. Recorded remaining repo-wide blockers for global formatting and CLI tests.

2026-05-07 / Codex: Fixed the existing repo-wide issues reported by validation. Applied `cargo fmt` across the workspace, added the missing CLI `wait_for_port_release` helper, reran `cargo test -p rbxsync`, `cargo test --workspace`, `cargo fmt -- --check`, and `git diff --check`, and updated this plan with the resolved validation state.

2026-05-07 / Codex: Milestone 2 implementation completed. Added the core place importer for `.rbxl` and `.rbxlx`, exported its API, added unit coverage for path generation and property conversion behavior, recorded unsupported variant diagnostics, corrected the CLI package name in validation commands, and reran focused core/server/CLI validation plus format, diff, and full workspace checks.

2026-05-07 / Codex: Milestone 3 implementation completed. Added the `import-place` CLI command, wired it to the core importer and shared writer, implemented config creation/preservation, force/backup/tooling/dry-run/json/quiet options, verified a temporary build/import smoke, and reran focused and full workspace validation.

2026-05-07 / Codex: Milestone 4 implementation completed. Added focused shared-writer tests, added CLI integration tests for `.rbxl` import and `.rbxlx` dry-run using temporary fixtures and the real binary, added the CLI test dev dependency, and reran focused plus full workspace validation.

2026-05-07 / Codex: Milestone 5 implementation completed. Added binary/shared string conversion, structured diagnostics and summaries, missing-service/script/terrain diagnostics, larger fixture timing, an explicit published-place follow-up decision, and focused validation.
