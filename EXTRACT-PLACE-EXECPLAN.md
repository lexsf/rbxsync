# Implement Extract Place

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` at the repository root. If this document is revised during implementation, keep it self-contained and update every affected section before stopping work.

## Purpose / Big Picture

After this change, a user with an existing RbxSync project can run a command such as `rbxsync extract-place --path ./Game --output ./build/Game.rbxl --force` and receive a Roblox place file that can be opened in Roblox Studio, archived, or passed to a later publishing process. This is the reverse of `rbxsync import-place`: import-place converts `.rbxl` or `.rbxlx` files into editable `src/` files, while extract-place converts those editable project files back into a place artifact.

The visible proof of success is a generated `.rbxl` or `.rbxlx` file that can be read back by `rbxsync import-place`. The implementation must preserve the existing `rbxsync build` command by moving shared export behavior into `rbxsync-core` and making both commands use the same project-to-place exporter.

## Progress

- [x] (2026-05-08 23:18Z) Read `PLANS.md`, `EXTRACT-PLACE.PRD`, `COMMAND-LINE-EXECPLAN.md`, the current CLI build path in `rbxsync-cli/src/main.rs`, the import-place integration tests in `rbxsync-cli/tests/import_place.rs`, and the core exports in `rbxsync-core/src/lib.rs`.
- [x] (2026-05-08 23:18Z) Created this initial ExecPlan with milestones for shared exporter refactoring, place-focused export semantics, CLI wiring, parity tests, and publishing-preparation polish.
- [x] (2026-05-10 23:19Z) Completed Milestone 1 by adding `rbxsync-core/src/place_exporter.rs`, exporting the shared API from `rbxsync-core/src/lib.rs`, and refactoring `rbxsync-cli/src/main.rs::cmd_build` to call `rbxsync_core::export_place`.
- [x] (2026-05-10 23:19Z) Ran `mise exec -- cargo fmt`, `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core`, `mise exec -- cargo test -p rbxsync`, and `git diff --check`; all passed.
- [x] (2026-05-10 23:19Z) Verified Milestone 1 smoke behavior by creating `/tmp/rbxsync-extract-plan/source/src/ServerScriptService/Main.server.luau`, running `mise exec -- cargo run -p rbxsync -- build --path /tmp/rbxsync-extract-plan/source --output /tmp/rbxsync-extract-plan/game.rbxl --format rbxl`, and confirming `/tmp/rbxsync-extract-plan/game.rbxl` is non-empty.
- [x] (2026-05-10 23:38Z) Completed Milestone 2 by expanding `rbxsync-core::place_exporter` with `rbxsync.json` config loading, `treeMapping` root discovery, metadata name precedence, `[SLASH]` unescaping, attributes, tags, package skipping, two-pass `Ref` resolution, strict-mode diagnostic failure, and richer typed JSON to `Variant` conversion.
- [x] (2026-05-10 23:38Z) Added focused core exporter tests for mapped roots, metadata names, script source precedence, attributes, tags, resolved refs, strict unresolved-ref failure, dry-run summaries, and unsupported-property diagnostics.
- [x] (2026-05-10 23:38Z) Ran `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core`, `mise exec -- cargo test -p rbxsync`, and `git diff --check`; all passed.
- [ ] Milestone 3: Add the `rbxsync extract-place` command with safe output handling, `--dry-run`, `--json`, `--quiet`, `--strict`, `--services`, and place-only format handling.
- [ ] Milestone 4: Add unit and CLI integration tests proving `.rbxl`, `.rbxlx`, dry-run, strict diagnostics, build compatibility, and import/export/import round-trip behavior.
- [ ] Milestone 5: Polish user output, document the future publishing boundary, update this plan with final validation evidence, and record any remaining follow-up work.

## Surprises & Discoveries

- Observation: The repository already has a reverse path from project files to Roblox artifacts, but it is implemented inside `rbxsync-cli/src/main.rs`.
  Evidence: `cmd_build`, `do_build`, `build_dom_from_src`, `build_dom_children`, `service_class_name`, and `json_to_variant` in `rbxsync-cli/src/main.rs` currently write `.rbxl`, `.rbxlx`, `.rbxm`, and `.rbxmx` using `rbx_dom_weak`, `rbx_binary`, and `rbx_xml`.

- Observation: The current build converter is not yet a full inverse of `import-place`.
  Evidence: `EXTRACT-PLACE.PRD` calls out that `json_to_variant` is partial, while `rbxsync-core/src/place_importer.rs` now emits richer typed JSON for variants such as `BinaryString`, `SharedString`, attributes, tags, and diagnostic-only terrain limitations.

- Observation: CLI integration tests can validate the export workflow without Roblox Studio.
  Evidence: `rbxsync-cli/tests/import_place.rs` uses the built `rbxsync` binary through `CARGO_BIN_EXE_rbxsync`, creates temporary projects, runs `rbxsync build`, and then runs `rbxsync import-place` against the generated files.

- Observation: The Rust toolchain is expected to run through `mise` in this repository.
  Evidence: `COMMAND-LINE-EXECPLAN.md` records `mise exec -- cargo test --workspace` and `mise exec -- cargo fmt -- --check` as passing after `mise use rust@stable`.

- Observation: Milestone 1 can preserve existing `rbxsync build` behavior by keeping output-path resolution, watch-mode orchestration, and user-facing status prints in the CLI while moving DOM construction and Roblox file writing into core.
  Evidence: `rbxsync-cli/src/main.rs::cmd_build` still resolves `--plugin`, default `build/game.<ext>`, and watch rebuild behavior, but each build now calls `rbxsync_core::export_place` with `force: true` to match the previous overwrite behavior.

- Observation: Reference properties require a two-pass exporter because a `.rbxjson` file can refer to an instance that appears later in filesystem traversal or in a mapped root outside `src/`.
  Evidence: `rbxsync-core/src/place_exporter.rs` now stores `PendingRef` records while applying metadata, records both metadata `referenceId` and normalized DataModel path in lookup maps after insertion, and resolves pending refs after all roots have been walked.

- Observation: Script source files need to remain the authoritative source for `Source` even when importer-created script metadata also contains a `Source` property.
  Evidence: The new `exports_tree_mapping_metadata_names_attributes_tags_and_refs` test writes both `Main.server.luau` and `Main.rbxjson` with `properties.Source`; the exporter emits a `DuplicateSource` diagnostic and keeps the `.server.luau` contents in the DOM.

## Decision Log

- Decision: Name the implementation plan `EXTRACT-PLACE-EXECPLAN.md`.
  Rationale: The repository already uses `COMMAND-LINE-EXECPLAN.md` for `COMMAND-LINE-PRD.md`, and this name keeps the plan discoverable beside `EXTRACT-PLACE.PRD`.
  Date/Author: 2026-05-08 / Codex

- Decision: Implement `extract-place` as a new subcommand on the existing `rbxsync` binary, not as a separate executable.
  Rationale: `EXTRACT-PLACE.PRD` continues the import-place decision that local file workflows belong in the existing CLI. The CLI already owns build/import commands and packaging.
  Date/Author: 2026-05-08 / Codex

- Decision: Preserve `rbxsync build` and make it delegate to the same shared exporter used by `extract-place`.
  Rationale: Existing users may already rely on `rbxsync build`, including model formats and watch mode. Sharing the exporter prevents two separate project-to-DOM implementations from drifting.
  Date/Author: 2026-05-08 / Codex

- Decision: Keep authenticated cloud publishing out of the MVP.
  Rationale: The PRD describes `--publish` as a future workflow that needs authentication, permission, rate-limit, and API error handling design. The immediately demonstrable behavior is creating a local `.rbxl` or `.rbxlx` artifact.
  Date/Author: 2026-05-08 / Codex

## Outcomes & Retrospective

This initial plan translates `EXTRACT-PLACE.PRD` into an implementation path. No code has been changed yet for `extract-place`. The first concrete outcome should be a shared `rbxsync-core::place_exporter` used by the existing build command with no user-visible regression. The final outcome should be a new `rbxsync extract-place` command that produces a place file, emits clean JSON when requested, and can be verified by importing its output back into a temporary project.

Milestone 1 is complete. `rbxsync-core/src/place_exporter.rs` now owns the shared project-to-DOM and DOM-to-artifact implementation, `rbxsync-core/src/lib.rs` re-exports the exporter API, and `rbxsync-cli/src/main.rs::cmd_build` delegates to `export_place` while retaining existing build command UX. This is not yet the new `extract-place` command; it is the required shared foundation for the later command.

Milestone 2 is complete. The exporter now has the richer project semantics needed by `extract-place`: it can discover roots from `treeMapping`, use metadata names instead of filesystem-safe names, unescape `[SLASH]`, apply attributes and tags, resolve `Ref` properties after all instances exist, and return structured diagnostics. The existing `rbxsync build` command still passes its CLI tests, so this deeper exporter behavior did not regress the current user-facing build path.

## Context and Orientation

RbxSync converts Roblox games between Roblox Studio place files and a local filesystem project. A Roblox place file is a saved game artifact with extension `.rbxl` for binary format or `.rbxlx` for XML format. A DataModel is the root Roblox object tree inside a place. A service is a top-level DataModel child such as `Workspace`, `ReplicatedStorage`, or `ServerScriptService`. An instance is any object in the tree, such as `Part`, `Folder`, `Script`, `LocalScript`, or `ModuleScript`.

The repository has several crates. `rbxsync-cli` builds the user-facing `rbxsync` binary. `rbxsync-core` holds shared library code. `rbxsync-server` runs the local HTTP server used by Studio extraction and live sync. The newly implemented import path already uses `rbxsync-core/src/place_importer.rs` to read `.rbxl` and `.rbxlx` files and `rbxsync-core/src/extract_writer.rs` to write the same project layout produced by server extraction.

The current reverse build path starts in `rbxsync-cli/src/main.rs::cmd_build`. It resolves the project directory, chooses an output format, calls `do_build`, builds a `WeakDom` through `build_dom_from_src`, and writes the root children to `rbx_binary::to_writer` or `rbx_xml::to_writer_default`. A `WeakDom` is the in-memory Roblox object tree from the `rbx_dom_weak` crate. This code works for basic project-to-place output, but it is CLI-local and its `json_to_variant` helper only supports some typed `.rbxjson` property values.

RbxSync project files live under a source tree, usually `src/`. Container instances are represented by a directory with `_meta.rbxjson`. Leaf non-script instances are represented by `<Name>.rbxjson`. Script source is represented by `<Name>.server.luau` for `Script`, `<Name>.client.luau` for `LocalScript`, and `<Name>.luau` for `ModuleScript`. A script-like directory may contain `init.server.luau`, `init.client.luau`, or `init.luau`. The exporter must convert this filesystem layout into a Roblox DataModel and then write it to a place file.

## Plan of Work

Milestone 1 is a behavior-preserving refactor. Add `rbxsync-core/src/place_exporter.rs` and move the current build implementation into it. The new core module should expose a small API that can build a DOM and write Roblox artifacts. Update `rbxsync-core/src/lib.rs` to export the new types and functions. Then change `rbxsync-cli/src/main.rs::cmd_build` and `do_build` so they call core instead of owning the implementation directly. At the end of this milestone, `rbxsync build --format rbxl` and `rbxsync build --format rbxlx` should still work exactly as before.

Milestone 2 improves the exporter until it is a real inverse of `import-place` for supported data. Teach the exporter to load `rbxsync.json`, honor `tree` and `treeMapping`, prefer metadata names, unescape `[SLASH]` in derived names, read attributes and tags, perform two-pass reference resolution, and emit structured diagnostics. Expand the typed JSON to `Variant` conversion to support property shapes emitted by `rbxsync-core/src/place_importer.rs`. Keep unsupported values non-fatal by default and make them fatal under strict mode.

Milestone 3 adds the user-facing command. Add a new `Commands::ExtractPlace` variant in `rbxsync-cli/src/main.rs`. The command should accept `--path`, `--output`, `--format`, `--force`, `--dry-run`, `--json`, `--quiet`, `--strict`, `--services`, `--include-packages`, and `--no-packages`. It should default to the current directory and `build/game.rbxl`. It should infer format from `--output` when possible, reject disagreement between `--format` and the output extension, and refuse to overwrite an existing output file unless `--force` is provided. With `--json`, stdout must contain only parseable JSON.

Milestone 4 adds tests and round-trip proof. Unit tests in `rbxsync-core/src/place_exporter.rs` should cover format inference, script suffixes, init files, metadata names, `[SLASH]` unescaping, tree mapping inversion, property conversion, reference resolution, diagnostics, and strict-mode failure. CLI integration tests under `rbxsync-cli/tests/` should create temporary projects, run the real binary, and verify `.rbxl`, `.rbxlx`, dry-run, JSON, output overwrite safety, and re-import behavior. The most important end-to-end proof is `extract-place` followed by `import-place`, with normalized scripts and `.rbxjson` metadata still present.

Milestone 5 is polish and documentation of boundaries. The command should print concise human summaries, grouped warning counts, and file sizes. The plan should record final validation transcripts. Any `--publish` flag should remain unimplemented unless deliberately scoped later; if a placeholder exists, it must fail clearly with a `publishNotImplemented` diagnostic rather than attempting any network call.

## Concrete Steps

Work from the repository root:

    cd /Users/lexiviripaeff/Documents/LoganX/rbxsync

Before editing, inspect the relevant current code:

    rg -n "cmd_build|do_build|build_dom_from_src|json_to_variant|ImportPlace|PlaceImport" rbxsync-cli/src/main.rs rbxsync-core/src
    sed -n '2490,2925p' rbxsync-cli/src/main.rs
    sed -n '1,260p' rbxsync-core/src/place_importer.rs
    sed -n '1,220p' rbxsync-cli/tests/import_place.rs

For Milestone 1, create `rbxsync-core/src/place_exporter.rs`. Define an initial API similar to:

    pub enum PlaceExportFormat {
        Rbxl,
        Rbxlx,
        Rbxm,
        Rbxmx,
    }

    pub struct PlaceExportOptions {
        pub project_dir: PathBuf,
        pub source_dir: PathBuf,
        pub output_path: PathBuf,
        pub format: PlaceExportFormat,
        pub force: bool,
        pub dry_run: bool,
        pub strict: bool,
        pub services: Option<HashSet<String>>,
        pub include_packages: bool,
        pub tree_mapping: HashMap<String, String>,
    }

    pub struct PlaceExportSummary {
        pub output_path: PathBuf,
        pub format: PlaceExportFormat,
        pub instances: usize,
        pub scripts: usize,
        pub metadata_files: usize,
        pub services: Vec<String>,
        pub bytes_written: Option<u64>,
        pub diagnostics: Vec<PlaceExportDiagnostic>,
    }

    pub fn export_place(options: PlaceExportOptions) -> anyhow::Result<PlaceExportSummary>;
    pub fn build_dom_from_project(options: &PlaceExportOptions) -> anyhow::Result<WeakDom>;

During Milestone 1, keep the API minimal if needed, but preserve enough structure that Milestones 2 and 3 can add diagnostics without rewriting the module. Update `rbxsync-core/src/lib.rs` to expose the exporter.

For Milestone 2, extend the exporter with internal helper phases. Use a first pass to discover all instances and assign DOM referents, then a second pass to apply properties that may contain references. A referent is the object identifier used by `rbx_dom_weak` to link instances and `Ref` properties. Use metadata `referenceId` when present, and otherwise use normalized DataModel paths as a fallback key. Define `PlaceExportDiagnosticKind` with at least `invalidProjectConfig`, `missingSourceTree`, `invalidMetadataJson`, `unsupportedProperty`, `unsupportedAttribute`, `unsupportedTag`, `unresolvedReference`, `duplicateSource`, `classConflict`, `ambiguousTreeMapping`, `skippedFile`, `skippedPackage`, `unsupportedTerrainVoxelData`, `outputExists`, and `publishNotImplemented`.

For Milestone 3, add the `ExtractPlace` command in the `Commands` enum in `rbxsync-cli/src/main.rs`. Add a `cmd_extract_place` function near `cmd_import_place` and `cmd_build`. Keep the CLI thin: it should parse flags, load config if needed, call `rbxsync_core::export_place`, and print either a human summary or JSON. Use the same logging suppression pattern already used by `import-place --json` so stdout remains clean.

For Milestone 4, add tests. Create `rbxsync-cli/tests/extract_place.rs` using the same `CARGO_BIN_EXE_rbxsync` pattern as `rbxsync-cli/tests/import_place.rs`. Set `RBXSYNC_VERSION_CHECK=1` in tests to avoid duplicate-installation noise. Use `tempfile` projects and write fixture files directly. A representative test should:

    create temp source project with src/Workspace/Baseplate.rbxjson and src/ServerScriptService/Main.server.luau
    run rbxsync extract-place --path <project> --output <temp>/game.rbxl --force --json
    parse stdout as JSON and assert success, format rbxl, scripts 1, and bytesWritten > 0
    run rbxsync import-place <temp>/game.rbxl --output <temp>/imported --force --json
    assert imported/src/ServerScriptService/Main.server.luau exists

For Milestone 5, update this plan, and if implementation changes user-facing command help or docs, update `INSTALL.md` or the relevant README section only if the change is clearly useful and scoped.

## Validation and Acceptance

After Milestone 1, run:

    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core
    mise exec -- cargo test -p rbxsync

Acceptance for Milestone 1 is that the existing build command still creates a non-empty place file:

    mkdir -p /tmp/rbxsync-extract-plan/source/src/ServerScriptService
    printf "print('hello')\n" > /tmp/rbxsync-extract-plan/source/src/ServerScriptService/Main.server.luau
    mise exec -- cargo run -p rbxsync -- build --path /tmp/rbxsync-extract-plan/source --output /tmp/rbxsync-extract-plan/game.rbxl --format rbxl
    test -s /tmp/rbxsync-extract-plan/game.rbxl

After Milestone 3, run the new command:

    mise exec -- cargo run -p rbxsync -- extract-place --path /tmp/rbxsync-extract-plan/source --output /tmp/rbxsync-extract-plan/game.rbxl --force --json

Expected JSON should include fields similar to:

    {
      "success": true,
      "command": "extract-place",
      "format": "rbxl",
      "output": "/tmp/rbxsync-extract-plan/game.rbxl",
      "scripts": 1,
      "diagnosticCount": 0
    }

After Milestone 4, run:

    mise exec -- cargo test -p rbxsync-core
    mise exec -- cargo test -p rbxsync --test extract_place
    mise exec -- cargo test -p rbxsync
    mise exec -- cargo fmt -- --check
    git diff --check

Before declaring the full plan complete, run:

    mise exec -- cargo test --workspace

The final acceptance behavior is:

1. `rbxsync extract-place --path ./Game --output ./build/Game.rbxl --force` writes a non-empty `.rbxl`.
2. `rbxsync extract-place --path ./Game --output ./build/Game.rbxlx --force` writes a non-empty `.rbxlx`.
3. `rbxsync extract-place --dry-run --json` validates without creating the output file.
4. `rbxsync extract-place --json` writes parseable JSON to stdout with no log prefixes.
5. Existing `rbxsync build` still creates `.rbxl`, `.rbxlx`, `.rbxm`, and `.rbxmx` artifacts.
6. Re-importing an exported place with `rbxsync import-place` recreates supported scripts and metadata.

## Idempotence and Recovery

The implementation should be safe to retry. Core refactors should preserve existing public behavior until each milestone adds new behavior. Tests should use temporary directories and should not depend on Roblox Studio, user home directories, or network access.

The export command must not delete project files. If the output file exists and `--force` is absent, fail before writing. When writing, create a temporary sibling file and rename it only after successful serialization. If serialization fails, clean up the temporary file when possible and leave the previous output untouched.

If a refactor breaks `rbxsync build`, stop and restore build compatibility before continuing to `extract-place`. Do not broaden into publishing, Open Cloud upload, or Studio automation while the local artifact workflow is incomplete.

## Artifacts and Notes

The current build path to refactor is in `rbxsync-cli/src/main.rs` around `cmd_build`, `do_build`, `build_dom_from_src`, `build_dom_children`, and `json_to_variant`.

The existing importer to use for round-trip validation is `rbxsync-core/src/place_importer.rs`, exposed from `rbxsync-core/src/lib.rs` as `import_place_file`, `PlaceImportOptions`, `PlaceImportResult`, `PlaceFileFormat`, `ImportDiagnostic`, and `ImportDiagnosticKind`.

The integration-test pattern to follow is `rbxsync-cli/tests/import_place.rs`, which invokes the real `rbxsync` binary with:

    env!("CARGO_BIN_EXE_rbxsync")
    RBXSYNC_VERSION_CHECK=1

Expected successful human output for the new command should be concise:

    Extracted place: /tmp/rbxsync-extract-plan/game.rbxl
    Format: rbxl
    Instances: 4
    Scripts: 1
    Warnings: 0

## Interfaces and Dependencies

Use these existing dependencies and modules:

- `rbx_dom_weak` for building the in-memory Roblox object tree.
- `rbx_binary::to_writer` for binary `.rbxl` and `.rbxm` output.
- `rbx_xml::to_writer_default` for XML `.rbxlx` and `.rbxmx` output.
- `serde_json` for reading `.rbxjson` metadata.
- `anyhow` for contextual errors.
- `rbxsync-core/src/types/project.rs` through the re-exported `ProjectConfig` type for reading `rbxsync.json`.
- `rbxsync-core/src/place_importer.rs` as the source of truth for typed JSON shapes that export must be able to consume.

At the end of the plan, `rbxsync-core/src/lib.rs` should export the main place exporter API. A novice should be able to search for `export_place` and find the core implementation, the CLI caller, and tests.

## Revision Notes

2026-05-08: Initial ExecPlan created from `EXTRACT-PLACE.PRD` and current repository inspection. The plan intentionally defers authenticated publishing and focuses on a local, testable project-to-place workflow.

2026-05-10: Milestone 1 completed. The current build/export implementation was moved into `rbxsync-core::place_exporter`, the existing `rbxsync build` command now delegates to the shared exporter, and validation confirmed the build command still writes a non-empty `.rbxl`.

2026-05-10: Milestone 2 completed. The shared exporter now understands project config, tree mapping, metadata names, attributes, tags, package filtering, pending reference resolution, strict-mode diagnostics, and a broader importer-compatible property conversion surface.
