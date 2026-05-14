# Implement P1 Asset Handling

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` at the repository root. If this document is revised during implementation, keep it self-contained and update every affected section before stopping work.

## Purpose / Big Picture

After this change, RbxSync users can choose what happens to asset-like data when converting between Roblox place files and local project files. The current `import-place` and `extract-place` workflows preserve asset references inside `.rbxjson` metadata, but there is no explicit command behavior, no manifest, and no local layout for asset payloads. This work makes that behavior intentional.

The visible result is that a user can run `rbxsync import-place Game.rbxl --include-assets --output ./Game --force` and receive a project with `src/` plus an `assets/manifest.json` that records every asset-like property found during import. Embedded binary payloads from `BinaryString` and `SharedString` properties are written as files under `assets/blobs/`. External `Content` references such as `rbxassetid://123456` remain references by default and are recorded in the manifest without network access. A user can then run `rbxsync extract-place --path ./Game --include-assets --output ./build/Game.rbxl --force` and have file-backed `BinaryString` and `SharedString` values embedded back into the place file while external `Content` strings stay unchanged.

## Progress

- [x] (2026-05-12 18:53Z) Read `PLANS.md`, `ROADMAP-ISSUES.md`, `COMMAND-LINE-EXECPLAN.md`, `EXTRACT-PLACE-EXECPLAN.md`, `PUBLISH-EXECPLAN.md`, and the current import/export code in `rbxsync-cli/src/main.rs`, `rbxsync-core/src/place_importer.rs`, `rbxsync-core/src/place_exporter.rs`, and `rbxsync-core/src/extract_writer.rs`.
- [x] (2026-05-12 18:53Z) Created this initial ExecPlan for P1 Asset Handling with milestones covering the asset policy, manifest and layout, importer support, exporter support, CLI wiring, tests, and documentation.
- [x] (2026-05-12 19:03Z) Implemented Milestone 1 by adding `rbxsync-core/src/assets.rs`, exporting the asset API from `rbxsync-core/src/lib.rs`, adding workspace `sha2`, and covering deterministic discovery, manifest sorting, manifest read/write, and summary counts with focused unit tests.
- [x] (2026-05-12 19:03Z) Ran `mise exec -- cargo fmt -- --check` and `mise exec -- cargo test -p rbxsync-core assets`; both passed, with 4 asset tests passing.
- [x] (2026-05-12 19:20Z) Implemented Milestone 2 by adding `extract_embedded_assets` and `AssetExtractionResult` in `rbxsync-core/src/assets.rs`, exporting the helper from `rbxsync-core/src/lib.rs`, rewriting inline `BinaryString` and `SharedString` properties into file-backed metadata, writing `assets/blobs/<sha256>.bin`, writing `assets/manifest.json`, and preserving external `Content` values as references.
- [x] (2026-05-12 19:20Z) Added focused asset extraction tests and a `place_importer` regression proving a DOM with `Content`, `BinaryString`, and `SharedString` can be imported and then extracted into a manifest plus blob files. Ran `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core assets`, `mise exec -- cargo test -p rbxsync-core place_importer`, `mise exec -- cargo test -p rbxsync-core`, and `git diff --check`; all passed.
- [x] (2026-05-12 19:36Z) Implemented Milestone 3 by adding `asset_mode` to `PlaceExportOptions`, `asset_summary` to `PlaceExportSummary`, export diagnostics for missing asset files, invalid manifests, hash mismatches, and paths outside the project, and file-backed `BinaryString` / `SharedString` conversion in `rbxsync-core/src/place_exporter.rs`.
- [x] (2026-05-12 19:36Z) Added exporter regressions for file-backed binary/shared-string embedding, hash mismatch failure, outside-project path rejection, and `IncludeLocal` manifest summary counts. Ran `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core place_exporter`, `mise exec -- cargo test -p rbxsync-core`, `mise exec -- cargo test -p rbxsync`, and `git diff --check`; all passed.
- [x] (2026-05-12 22:44Z) Implemented Milestone 4 by adding `--include-assets` and `--no-assets` to `import-place` and `extract-place`, wiring `AssetMode` through CLI import/export calls, adding asset summaries to JSON and human output, and documenting the flags plus file-backed binary formats in `README.md`, `docs/cli/commands.md`, and `docs/file-formats/property-types.md`.
- [x] (2026-05-12 22:44Z) Added CLI integration coverage for `import-place --include-assets` manifest/blob output and `extract-place --include-assets` manifest summary plus file-backed payload embedding. Ran `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync --test import_place`, `mise exec -- cargo test -p rbxsync --test extract_place`, `mise exec -- cargo run -p rbxsync -- import-place --help`, `mise exec -- cargo run -p rbxsync -- extract-place --help`, `mise exec -- cargo test -p rbxsync`, and `git diff --check`; all passed.
- [x] (2026-05-12 22:54Z) Implemented Milestone 5 by moving local asset payload path containment and SHA-256 verification into shared core helpers, adding asset-module tests for file-backed discovery, stable digest naming, outside-project rejection, and hash mismatch detection, and extending CLI integration coverage with a real `extract-place --include-assets` to `import-place --include-assets` binary round trip.
- [x] (2026-05-12 22:54Z) Ran the Milestone 5 validation set: `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core`, `mise exec -- cargo test -p rbxsync --test import_place`, `mise exec -- cargo test -p rbxsync --test extract_place`, `mise exec -- cargo test -p rbxsync`, `mise exec -- cargo test --workspace`, the offline `/private/tmp/rbxsync-assets-plan-m5` smoke, and `git diff --check`; all passed.

## Surprises & Discoveries

- Observation: `Content`, `BinaryString`, and `SharedString` are already converted by the local place importer.
  Evidence: `rbxsync-core/src/place_importer.rs::variant_to_json_property` emits `Content` as a string, `BinaryString` as base64 data, and `SharedString` as `{ hash, file, data }` where `file` is currently null.

- Observation: The exporter already embeds inline `BinaryString` and `SharedString` data back into Roblox place files.
  Evidence: `rbxsync-core/src/place_exporter.rs::json_to_variant` decodes `BinaryString.value` from base64 and decodes `SharedString.value.data` from base64 before creating `Variant::BinaryString` or `Variant::SharedString`.

- Observation: There is no current asset-specific CLI surface for local place import or export.
  Evidence: `rbxsync-cli/src/main.rs::Commands::ImportPlace` has `--terrain`, `--force`, backup, tooling, dry-run, JSON, and quiet flags, while `Commands::ExtractPlace` has output, format, strict, services, and package flags. Neither command has `--include-assets` or `--no-assets`.

- Observation: The Studio extraction command has an older `--assets` flag that does not define the local place-file asset behavior requested by this plan.
  Evidence: `Commands::Extract` contains `assets: bool` with help text "Include binary assets", and `rbxsync-server/src/lib.rs` serializes an `includeAssets` request field for the Studio plugin. This is separate from `import-place` and `extract-place`.

- Observation: The file format docs currently document `Content` only as an asset URL string.
  Evidence: `docs/file-formats/property-types.md` shows `{ "type": "Content", "value": "rbxassetid://123456" }` and does not define an `assets/` folder or manifest.

- Observation: Milestone 1 needed a hashing dependency, but no hex dependency was necessary.
  Evidence: `Cargo.toml` now declares workspace `sha2 = "0.10"`, `rbxsync-core/Cargo.toml` consumes it, and `rbxsync-core/src/assets.rs` formats SHA-256 bytes as lowercase hex directly.

- Observation: Manifest file paths are stored as project-relative strings rather than `PathBuf`.
  Evidence: `AssetEntry.file` is `Option<String>`, and the scanner normalizes backslashes to forward slashes for file-backed entries. This keeps JSON deterministic across platforms and satisfies the plan requirement that manifests not store absolute paths.

- Observation: Import-time asset extraction can stay outside `extract_writer` for now.
  Evidence: `rbxsync-core/src/assets.rs::extract_embedded_assets` accepts a `Vec<serde_json::Value>` and a project directory, returns a mutated `Vec<Value>` plus manifest and summary, and writes the asset files before any caller hands the instances to `write_serialized_instances`.

- Observation: Duplicate embedded payloads naturally collapse to one blob file.
  Evidence: `assets::tests::reuses_existing_blob_files_for_duplicate_payloads` extracts two identical `BinaryString` properties, verifies both rewritten properties point at the same `assets/blobs/<sha256>.bin`, and verifies only one file write and four bytes written in the summary.

- Observation: Export-side file-backed property conversion needed access to `project_dir`, so it could not stay as a free `json_to_variant` helper.
  Evidence: `rbxsync-core/src/place_exporter.rs` now routes property conversion through `DomBuilder::json_to_variant`, which can resolve asset paths relative to `PlaceExportOptions.project_dir` and push structured asset diagnostics.

- Observation: `extract-place` can report manifest counts without requiring the manifest for file-backed properties.
  Evidence: `place_exporter::tests::include_local_summary_reports_manifest_asset_counts` sets `asset_mode` to `IncludeLocal`, writes `assets/manifest.json`, and verifies `PlaceExportSummary.asset_summary`; file-backed property tests do not require a manifest because each property includes its own file path.

- Observation: A build-generated `.rbxl` fixture may contain extra embedded binary payloads beyond the two explicit test properties.
  Evidence: `rbxsync-cli/tests/import_place.rs::import_place_include_assets_writes_manifest_and_blobs` asserts the minimum expected payload behavior rather than an exact count because the serialized place fixture produced one extra embedded payload during round-trip.

- Observation: CLI help now exposes the asset flags without promising downloads.
  Evidence: `mise exec -- cargo run -p rbxsync -- import-place --help` lists `--include-assets` as writing `assets/manifest.json` and local embedded payload files, and `mise exec -- cargo run -p rbxsync -- extract-place --help` lists `--include-assets` as reading the local manifest and file-backed binary payloads.

- Observation: Path containment and digest verification are shared asset concerns, not exporter-only behavior.
  Evidence: `rbxsync-core/src/assets.rs` now exports `resolve_asset_file`, `read_asset_file`, `asset_sha256_hex`, `AssetFileError`, and `AssetFileErrorKind`; exporter diagnostics map those shared error kinds back to `MissingAssetFile`, `AssetPathOutsideProject`, and `AssetHashMismatch`.

- Observation: `extract-place --include-assets` only reports an asset summary when it has an existing `assets/manifest.json` to summarize.
  Evidence: The manual smoke project with only a `Content` property and no manifest emitted `"assets": null` from `extract-place`, then `import-place --include-assets` created `assets/manifest.json` with one referenced content asset and zero embedded payloads.

- Observation: The full workspace test suite still emits pre-existing dead-code warnings from `rbxsync-mcp`.
  Evidence: `mise exec -- cargo test --workspace` passed, while warning that `TestStartResponse`, `TestStopResponse`, `ConsoleMessage` fields, and several `RbxSyncClient` test-control methods in `rbxsync-mcp/src/tools/mod.rs` are unused.

## Decision Log

- Decision: Default behavior remains reference-only and network-free.
  Rationale: Existing projects rely on `Content` strings being preserved exactly. Downloading or rewriting external assets by default would be slow, surprising, and dependent on network, permissions, and Roblox API behavior.
  Date/Author: 2026-05-12 / Codex

- Decision: `--include-assets` creates and consumes a local asset manifest but does not download external `Content` assets in this P1 implementation.
  Rationale: The roadmap asks to avoid network access unless explicitly requested. This plan makes `--include-assets` explicit for local asset handling, but it still keeps external downloads out of scope because downloading Roblox assets needs API design, rate-limit behavior, permission handling, file-type inference, and cache invalidation. The manifest records external references so a later networked downloader has stable inputs.
  Date/Author: 2026-05-12 / Codex

- Decision: Extract embedded `BinaryString` and `SharedString` payloads to files when `import-place --include-assets` is used.
  Rationale: These payloads are already available inside local place files, so extracting them is deterministic, offline, and useful. It reduces large base64 blobs in metadata while preserving round-trip behavior.
  Date/Author: 2026-05-12 / Codex

- Decision: Keep `Content` properties as original URI strings unless a later explicit rewrite feature is added.
  Rationale: Roblox `Content` values often point at external asset IDs or built-in `rbxasset://` resources. Rewriting them to local paths could break Studio behavior unless the upload and publishing semantics are designed as a separate feature.
  Date/Author: 2026-05-12 / Codex

- Decision: Represent manifest file paths as relative UTF-8 strings in `AssetEntry.file`.
  Rationale: Manifest JSON must be stable and portable. `PathBuf` serialization can reflect platform-specific path details, while the asset layout intentionally uses forward-slash project-relative paths such as `assets/blobs/<sha256>.bin`.
  Date/Author: 2026-05-12 / Codex

- Decision: Keep import-time extraction as a pure core pre-writer transform instead of adding asset fields to `ExtractWriterOptions` in Milestone 2.
  Rationale: The writer already has a broad responsibility for `src/` layout, backups, package preservation, and tooling. A pre-writer transform is easier to test directly, keeps the writer contract unchanged, and still gives `cmd_import_place` one core helper to call when CLI flags are added later.
  Date/Author: 2026-05-12 / Codex

- Decision: Make missing asset files, hash mismatches, and outside-project asset paths fatal even when `strict` is false.
  Rationale: These cases mean the exporter cannot accurately reconstruct the Roblox place file. Treating them as warning-only would silently drop or corrupt binary-backed properties.
  Date/Author: 2026-05-12 / Codex

- Decision: Keep invalid `assets/manifest.json` as a diagnostic that only fails under strict mode.
  Rationale: File-backed properties contain the file path and optional hash needed to embed payloads. The manifest is useful for summary counts and future tooling, but it is not required to build a correct place when metadata properties are self-contained.
  Date/Author: 2026-05-12 / Codex

- Decision: Emit an `assets` field in JSON summaries for import and export.
  Rationale: Automation should not need to parse human text to see asset mode, manifest path, reference counts, embedded payload counts, and local file-write totals. Commands that do not enable assets report `null`, preserving a clear default while keeping a stable key for consumers.
  Date/Author: 2026-05-12 / Codex

- Decision: Keep local asset path resolution and SHA-256 verification in `rbxsync-core/src/assets.rs`.
  Rationale: Import, export, and future asset tooling need the same containment and digest rules. Sharing the helpers gives Milestone 5 direct unit coverage for the policy while preserving exporter-specific diagnostic mapping at the command boundary.
  Date/Author: 2026-05-12 / Codex

## Outcomes & Retrospective

Milestone 1 is complete. `rbxsync-core/src/assets.rs` now defines the shared asset model, manifest schema version, deterministic discovery scanner, manifest read/write helpers, SHA-256 naming support, and summary counts. The module is exported from `rbxsync-core/src/lib.rs` but is not yet wired into `import-place` or `extract-place`, so user-facing behavior is unchanged. The intended next outcome is Milestone 2: using this core model during import to write `assets/manifest.json` and local blob files when `--include-assets` is eventually wired.

Milestone 2 is complete. The core asset module can now perform the import-time transformation needed by future `import-place --include-assets`: inline `BinaryString` and `SharedString` payloads are decoded, hashed, written once under `assets/blobs/`, and rewritten to file-backed metadata; `Content` values remain unchanged and are recorded as referenced-only manifest entries. This is validated through both direct asset-module tests and a `place_importer` regression. The CLI is still unchanged because `--include-assets` / `--no-assets` remain Milestone 4.

Milestone 3 is complete. The exporter now reads file-backed `BinaryString` and `SharedString` property metadata, verifies optional SHA-256 hashes, rejects asset paths outside the project, and fails when a required payload file is missing or invalid. `AssetMode::IncludeLocal` also loads `assets/manifest.json` when present and includes manifest-derived counts in `PlaceExportSummary.asset_summary`. Existing CLI callers still use `AssetMode::ReferencesOnly`, so command behavior remains unchanged until Milestone 4 adds flags.

Milestone 4 is complete. The user-facing CLI now exposes `--include-assets` and `--no-assets` on both local place conversion commands. `import-place --include-assets` writes `assets/manifest.json` and local blob files before the shared writer writes `src/`, while `extract-place --include-assets` reads file-backed payloads and includes manifest counts when present. Documentation now describes the offline behavior: embedded `BinaryString` and `SharedString` payloads can be extracted and re-embedded, while external `Content` references are recorded and preserved rather than downloaded.

Milestone 5 is complete. The shared asset module now owns the reusable file-read policy for local payloads: project-relative paths only, canonical containment under the project root, and optional SHA-256 verification. Focused core tests cover deterministic manifest/scanner behavior, stable digest naming, file-backed path normalization, outside-project rejection, and hash mismatch errors. The CLI integration suite now proves a file-backed `BinaryString` and `SharedString` can be exported into a real binary place file, re-imported with `--include-assets`, and recovered as local blob files with the original bytes. The full workspace validation set passed offline.

## Context and Orientation

RbxSync converts Roblox games between local filesystem projects and Roblox place files. A Roblox place file is a saved game artifact with extension `.rbxl` for the binary format or `.rbxlx` for the XML format. `rbxsync import-place` reads a local place file and writes a project under `src/`. `rbxsync extract-place` reads a project and writes a place file. Both commands are implemented in `rbxsync-cli/src/main.rs` and delegate most conversion work to `rbxsync-core`.

An asset-like property is a property whose value points at or contains non-source asset data. In this plan, the important property types are `Content`, `BinaryString`, and `SharedString`. A `Content` value is a string such as `rbxassetid://123456` or `rbxasset://fonts/families/GothamSSm.json`. It usually references an external Roblox asset or built-in resource. A `BinaryString` value is raw bytes encoded in project JSON as base64. A `SharedString` value is also raw bytes, typically content-addressed by a hash, and is currently represented as `{ "hash": "...", "file": null, "data": "<base64>" }`.

The current importer lives in `rbxsync-core/src/place_importer.rs`. It reads `.rbxl` using `rbx_binary::from_reader`, reads `.rbxlx` using `rbx_xml::from_reader_default`, walks the in-memory Roblox DOM, and serializes instances into plugin-compatible JSON. The current exporter lives in `rbxsync-core/src/place_exporter.rs`. It reads project files from `src/`, converts `.rbxjson` metadata into `rbx_dom_weak::types::Variant` values, builds a Roblox DOM, and writes `.rbxl` or `.rbxlx`.

The shared extraction writer lives in `rbxsync-core/src/extract_writer.rs`. It writes serialized instances to the project filesystem, including `.luau` script files and `.rbxjson` metadata files. This is the natural place to add import-time asset file writes because it already owns the conversion from serialized instance data to the local project layout.

The new asset layout should be rooted at the project directory, not inside `src/`:

    assets/
      manifest.json
      blobs/
        <sha256>.bin

`assets/manifest.json` is a machine-readable index. The manifest should be deterministic, sorted, and safe to regenerate. The `assets/blobs/` directory stores embedded payloads extracted from `BinaryString` and `SharedString` values. The file name should be a SHA-256 hex digest plus `.bin`, where SHA-256 means the standard 256-bit cryptographic hash of the raw bytes. If the repository does not already depend on a SHA-256 crate, add `sha2` to workspace dependencies and `rbxsync-core/Cargo.toml`.

## Plan of Work

Milestone 1 creates the shared asset model. Add a new module `rbxsync-core/src/assets.rs` and export it from `rbxsync-core/src/lib.rs`. Define `AssetMode`, `AssetManifest`, `AssetEntry`, `AssetKind`, `AssetSource`, and summary types in core. `AssetMode` should have at least `ReferencesOnly`, `IncludeLocal`, and `Disabled`. `ReferencesOnly` means preserve property values exactly and optionally count them. `IncludeLocal` means write or read `assets/manifest.json` and local payload files. `Disabled` means do not write or consume the manifest and keep metadata inline. Use `ReferencesOnly` as the default for both import and export. Define manifest version `1` so future formats can be detected cleanly.

Milestone 1 should also add helpers to scan serialized instance JSON for asset-like properties. The scanner must record the instance path, property name, property type, original value, and whether the property is external reference data or embedded bytes. It should not mutate the instances yet. Add unit tests that feed small JSON instances into the scanner and verify deterministic entries for `Content`, `BinaryString`, and `SharedString`.

Milestone 2 adds import-time manifest writing. Extend `PlaceImportOptions` and the `cmd_import_place` call path with an asset mode, but keep the CLI wiring for Milestone 4 if it is cleaner. When the effective mode is `IncludeLocal`, `import-place` should write `assets/manifest.json` and extract embedded `BinaryString` and `SharedString` byte payloads to `assets/blobs/<sha256>.bin`. External `Content` values should be recorded in the manifest with `status: "referencedOnly"` and no file path. The metadata written under `src/` should reference file-backed payloads for `SharedString` and `BinaryString` by setting a `file` field relative to the project root, while keeping enough type information for the exporter to reconstruct the original Roblox variant.

The exact project JSON shape after `--include-assets` should be:

    "BinaryBlob": {
      "type": "BinaryString",
      "value": {
        "file": "assets/blobs/<sha256>.bin",
        "encoding": "raw",
        "sha256": "<sha256>",
        "byteLength": 123
      }
    }

    "SharedBlob": {
      "type": "SharedString",
      "value": {
        "hash": "<roblox shared string hash>",
        "file": "assets/blobs/<sha256>.bin",
        "sha256": "<sha256>",
        "byteLength": 123
      }
    }

For backward compatibility, the exporter must continue accepting the current inline forms:

    "BinaryBlob": { "type": "BinaryString", "value": "<base64>" }
    "SharedBlob": { "type": "SharedString", "value": { "hash": "...", "file": null, "data": "<base64>" } }

Milestone 3 adds export-time manifest consumption and file-backed value embedding. Extend `PlaceExportOptions` with an asset mode. Teach `rbxsync-core/src/place_exporter.rs::json_to_variant` to accept both inline and file-backed `BinaryString` and `SharedString` values. File paths must resolve relative to `project_dir`, must stay within the project directory after canonicalization, and must fail with a structured export diagnostic if they point outside the project or cannot be read. External `Content` values remain strings. If `extract-place --include-assets` sees an `assets/manifest.json`, it should include asset counts in the summary, but the manifest should not be required for file-backed properties because the property itself contains the file path. This is implemented by moving conversion into `DomBuilder::json_to_variant` and adding manifest summary loading to `export_place`.

Milestone 3 should add diagnostics to `PlaceExportDiagnosticKind`, such as `MissingAssetFile`, `InvalidAssetManifest`, `AssetHashMismatch`, and `AssetPathOutsideProject`. If a file-backed payload has a `sha256` field and the bytes do not match it, export should produce an `AssetHashMismatch` diagnostic. In non-strict mode, a missing or invalid asset payload should fail export because the place file cannot be correctly constructed; this is not a warning-only condition. In strict mode, manifest inconsistencies that are not otherwise fatal should also fail.

Milestone 4 wires the CLI and documentation. Add `--include-assets` and `--no-assets` to `Commands::ImportPlace` and `Commands::ExtractPlace` in `rbxsync-cli/src/main.rs`. These flags should conflict with each other. No flag means `ReferencesOnly`, which preserves current behavior and does not create an `assets/` directory. `--include-assets` means `IncludeLocal`. `--no-assets` means `Disabled`, useful for users who want to ignore an existing manifest during export. Update `print_import_summary` and `print_export_summary` so human output includes asset counts when assets are included, and JSON output includes an `assets` object with fields such as `mode`, `manifest`, `contentReferences`, `embeddedPayloads`, `filesWritten`, and `bytesWritten`. This is implemented in `rbxsync-cli/src/main.rs`, with default CLI behavior mapped to `AssetMode::ReferencesOnly`.

Milestone 4 should update `docs/file-formats/property-types.md` to document file-backed `BinaryString` and `SharedString` values, and update `docs/cli/commands.md` plus the relevant README command examples to show the new flags. The docs must say clearly that `Content` asset IDs are recorded and preserved, not downloaded, by this implementation.

Milestone 5 adds tests and round-trip proof. Add unit tests in `rbxsync-core/src/assets.rs` for manifest sorting, SHA-256 naming, scanner behavior, path containment, and hash mismatch detection. Add importer tests in `rbxsync-core/src/place_importer.rs` or adjacent modules that create a DOM with `Content`, `BinaryString`, and `SharedString`, run import with `IncludeLocal`, and verify the manifest plus blob files. Add exporter tests in `rbxsync-core/src/place_exporter.rs` that read file-backed payloads and produce the expected `Variant::BinaryString` and `Variant::SharedString`. Add real-binary CLI tests under `rbxsync-cli/tests/` that run `import-place --include-assets --json` and `extract-place --include-assets --json` against temporary projects without Roblox Studio or network access. This is implemented with shared asset-file helpers, 10 focused `assets` tests, importer/exporter regressions, and an `extract_place` integration test that exports file-backed binary/shared-string payloads to `.rbxl` and reimports them back into local blobs.

## Concrete Steps

Work from the repository root:

    cd /Users/lexiviripaeff/Documents/LoganX/rbxsync

Before editing, inspect the current code paths:

    rg -n "ImportPlace|ExtractPlace|PlaceImportOptions|PlaceExportOptions|variant_to_json_property|json_to_variant|BinaryString|SharedString|Content" rbxsync-cli/src/main.rs rbxsync-core/src
    sed -n '1320,1605p' rbxsync-cli/src/main.rs
    sed -n '360,610p' rbxsync-core/src/place_importer.rs
    sed -n '960,1085p' rbxsync-core/src/place_exporter.rs
    sed -n '330,560p' rbxsync-core/src/extract_writer.rs

For Milestone 1, add `rbxsync-core/src/assets.rs`. Define public types similar to:

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum AssetMode {
        ReferencesOnly,
        IncludeLocal,
        Disabled,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AssetManifest {
        pub version: u32,
        pub generated_by: String,
        pub entries: Vec<AssetEntry>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AssetEntry {
        pub id: String,
        pub kind: AssetKind,
        pub source: AssetSource,
        pub instance_path: String,
        pub property: String,
        pub original: Option<String>,
        pub file: Option<String>,
        pub sha256: Option<String>,
        pub byte_length: Option<u64>,
        pub status: AssetStatus,
    }

Keep manifest paths serialized with forward slashes and relative to the project root. Do not store absolute paths in the manifest.

For Milestone 2, extend `ExtractWriterOptions` with asset options only if the writer owns mutation and file writes. A reasonable shape is:

    pub asset_mode: AssetMode,
    pub asset_manifest_path: Option<PathBuf>,

The implementation chose the second path: `extract_embedded_assets` takes the `Vec<Value>` before `write_serialized_instances`, returns a mutated `Vec<Value>`, writes the asset files, writes the manifest, and returns an `AssetSummary` for later CLI output.

For Milestone 3, change `json_to_variant` in `rbxsync-core/src/place_exporter.rs`. The current `BinaryString` branch accepts only a base64 string. Keep that branch, and add an object branch that reads `value.file`, resolves it under `project_dir`, checks optional `sha256`, and returns `Variant::BinaryString(BinaryString::from(bytes))`. Do the same for `SharedString`, returning `Variant::SharedString(SharedString::new(bytes))`. Because `json_to_variant` currently has no access to `project_dir`, this milestone may need to turn it into a method on `DomBuilder` or pass a conversion context.

For Milestone 4, add CLI fields:

    #[arg(long, conflicts_with = "no_assets")]
    include_assets: bool,

    #[arg(long)]
    no_assets: bool,

Map them with:

    fn resolve_asset_mode(include_assets: bool, no_assets: bool) -> AssetMode {
        if include_assets {
            AssetMode::IncludeLocal
        } else if no_assets {
            AssetMode::Disabled
        } else {
            AssetMode::ReferencesOnly
        }
    }

For Milestone 5, add or update tests and run the validation commands below. Keep all tests offline. Do not call Roblox APIs, do not require a Roblox account, and do not rely on the Studio plugin.

## Validation and Acceptance

After Milestone 1, run:

    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core assets

Acceptance for Milestone 1 is that asset scanning and manifest serialization are deterministic. This has been validated: `mise exec -- cargo fmt -- --check` passed, and `mise exec -- cargo test -p rbxsync-core assets` passed 4 tests covering deterministic discovery, manifest sorting, manifest read/write, and summary counts.

After Milestone 2, run:

    mise exec -- cargo test -p rbxsync-core place_importer

Acceptance for Milestone 2 is that a DOM containing one `Content`, one `BinaryString`, and one `SharedString` imports with the core asset extraction helper into a project where `assets/manifest.json` exists, two blob files exist under `assets/blobs/`, and the `Content` value remains unchanged in the mutated instance metadata. This has been validated: `mise exec -- cargo test -p rbxsync-core place_importer` passed 7 tests including `imported_asset_payloads_can_be_extracted_to_manifest_and_blobs`. `mise exec -- cargo test -p rbxsync-core assets` also passed 6 tests covering direct extraction behavior, and `mise exec -- cargo test -p rbxsync-core` passed all 69 core tests.

After Milestone 3, run:

    mise exec -- cargo test -p rbxsync-core place_exporter

Acceptance for Milestone 3 is that file-backed `BinaryString` and `SharedString` metadata can be exported into a DOM and then serialized to `.rbxl` or `.rbxlx`. A hash mismatch should fail with an actionable diagnostic naming the property and file path. This has been validated: `mise exec -- cargo test -p rbxsync-core place_exporter` passed 7 tests including file-backed payload embedding, hash mismatch failure, outside-project path rejection, and manifest count summaries. `mise exec -- cargo test -p rbxsync-core` passed all 73 core tests, and `mise exec -- cargo test -p rbxsync` passed the CLI package and existing real-binary integration tests.

After Milestone 4, validate the user-facing help:

    mise exec -- cargo run -p rbxsync -- import-place --help
    mise exec -- cargo run -p rbxsync -- extract-place --help

Expected help should show `--include-assets` and `--no-assets` on both commands. The help text should avoid promising external downloads. This has been validated with `mise exec -- cargo run -p rbxsync -- import-place --help` and `mise exec -- cargo run -p rbxsync -- extract-place --help`. Focused CLI tests also passed: `mise exec -- cargo test -p rbxsync --test import_place`, `mise exec -- cargo test -p rbxsync --test extract_place`, and `mise exec -- cargo test -p rbxsync`.

After Milestone 5, run the focused and broad validation set:

    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core
    mise exec -- cargo test -p rbxsync --test import_place
    mise exec -- cargo test -p rbxsync --test extract_place
    mise exec -- cargo test -p rbxsync
    mise exec -- cargo test --workspace
    git diff --check

This has been validated. `mise exec -- cargo fmt -- --check` passed. `mise exec -- cargo test -p rbxsync-core` passed 77 tests, including 10 asset tests. `mise exec -- cargo test -p rbxsync --test import_place` passed 4 tests. `mise exec -- cargo test -p rbxsync --test extract_place` passed 6 tests, including the file-backed payload round trip. `mise exec -- cargo test -p rbxsync` passed the CLI package tests. `mise exec -- cargo test --workspace` passed all workspace tests and only emitted existing `rbxsync-mcp` dead-code warnings. `git diff --check` passed.

Add a manual smoke using a temporary project:

    rm -rf /tmp/rbxsync-assets-plan
    mkdir -p /tmp/rbxsync-assets-plan/source/src/Workspace
    cat > /tmp/rbxsync-assets-plan/source/src/Workspace/Sound.rbxjson <<'JSON'
    {
      "className": "Sound",
      "name": "Sound",
      "referenceId": "sound-ref",
      "path": "Workspace/Sound",
      "properties": {
        "SoundId": { "type": "Content", "value": "rbxassetid://123456" }
      }
    }
    JSON
    mise exec -- cargo run -p rbxsync -- extract-place --path /tmp/rbxsync-assets-plan/source --output /tmp/rbxsync-assets-plan/game.rbxl --force --include-assets --json
    mise exec -- cargo run -p rbxsync -- import-place /tmp/rbxsync-assets-plan/game.rbxl --output /tmp/rbxsync-assets-plan/imported --force --include-assets --json
    test -f /tmp/rbxsync-assets-plan/imported/assets/manifest.json

The smoke proves the CLI accepts the flags, emits JSON, keeps a `Content` asset reference as a reference, and creates a manifest. Unit tests should carry the stronger proof for embedded `BinaryString` and `SharedString` payloads.

This smoke was run at `/private/tmp/rbxsync-assets-plan-m5`. `extract-place --include-assets --json` wrote `/private/tmp/rbxsync-assets-plan-m5/game.rbxl` with zero diagnostics. Because the source project had no existing manifest, export reported `"assets": null`. `import-place --include-assets --json` then wrote `/private/tmp/rbxsync-assets-plan-m5/imported/assets/manifest.json` with one `Content` reference, zero embedded payloads, and zero local files written, which matches the no-download policy for external `Content`.

## Idempotence and Recovery

The implementation must be safe to run repeatedly. Re-running `import-place --include-assets --force` may replace `src/` according to the existing import backup behavior, but it should write `assets/manifest.json` and `assets/blobs/` deterministically. Existing blob files with the same SHA-256 name can be reused. If a manifest write fails, the command should fail before reporting success.

For export, `--no-assets` should ignore `assets/manifest.json` and only use inline metadata values. `--include-assets` should read file-backed properties when they appear and should fail clearly when required files are missing. It should not delete asset files. If the user wants to recover from a bad manifest, they can re-run import with `--include-assets --force` from the original place file or edit metadata back to inline base64.

Any path from metadata or manifest must be resolved relative to the project directory and rejected if it escapes the project after canonicalization. This prevents a malformed `.rbxjson` file from causing export to read arbitrary files outside the project.

## Artifacts and Notes

The manifest should look like this for a project with one external content reference and one extracted shared string:

    {
      "version": 1,
      "generatedBy": "rbxsync import-place",
      "entries": [
        {
          "id": "content:Workspace/Sound:SoundId",
          "kind": "content",
          "instancePath": "Workspace/Sound",
          "property": "SoundId",
          "original": "rbxassetid://123456",
          "file": null,
          "sha256": null,
          "byteLength": null,
          "status": "referencedOnly"
        },
        {
          "id": "sha256:<digest>",
          "kind": "sharedString",
          "instancePath": "Workspace/Mesh",
          "property": "SharedMeshData",
          "original": null,
          "file": "assets/blobs/<digest>.bin",
          "sha256": "<digest>",
          "byteLength": 128,
          "status": "extracted"
        }
      ]
    }

Keep JSON key casing camelCase to match existing CLI summaries and serialized diagnostics.

## Interfaces and Dependencies

Add `rbxsync-core/src/assets.rs` and export the important types from `rbxsync-core/src/lib.rs`:

    pub use assets::{
        build_asset_manifest, discover_assets, extract_embedded_assets, load_asset_manifest,
        summarize_assets, write_asset_manifest, AssetEntry, AssetExtractionResult, AssetKind,
        AssetManifest, AssetMode, AssetSource, AssetStatus, AssetSummary, ASSET_MANIFEST_VERSION,
    };

The exact function names can change if a better local naming pattern emerges, but the final API must let both importer and exporter share the same manifest and path-safety logic.

Extend `PlaceImportOptions` in `rbxsync-core/src/place_importer.rs` or the import command pipeline with an asset mode and a project directory when needed:

    pub struct PlaceImportOptions {
        pub input_path: PathBuf,
        pub services: Option<HashSet<String>>,
        pub include_terrain: bool,
        pub asset_mode: AssetMode,
    }

Extend `PlaceImportResult` or the writer summary with an optional `AssetSummary` so `cmd_import_place` can print asset counts.

Extend `PlaceExportOptions` in `rbxsync-core/src/place_exporter.rs`:

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
        pub asset_mode: AssetMode,
    }

Extend `PlaceExportSummary` with:

    pub asset_summary: Option<AssetSummary>

If `sha2` is added, add it to the workspace root `Cargo.toml` and to `rbxsync-core/Cargo.toml`. Keep hashing in core so the CLI does not duplicate byte-level asset logic.

The current `AssetEntry.file` type is `Option<String>` so manifest paths stay forward-slash, project-relative, and platform-independent.

## Revision Notes

2026-05-12 / Codex: Created the initial P1 Asset Handling ExecPlan from `ROADMAP-ISSUES.md` and current import/export code. The plan deliberately keeps external `Content` downloads out of scope while making local embedded payload extraction and manifest-based export concrete.

2026-05-12 / Codex: Completed Milestone 1. Added the core asset model and deterministic scanner in `rbxsync-core/src/assets.rs`, exported the API, added `sha2`, and recorded passing focused validation. Updated the plan to reflect the actual manifest path representation as `Option<String>`.

2026-05-12 / Codex: Completed Milestone 2. Added core import-time embedded asset extraction, blob writing, manifest writing, metadata rewriting, duplicate-payload reuse, and importer-level regression coverage. Recorded that CLI flag wiring remains Milestone 4.

2026-05-12 / Codex: Completed Milestone 3. Added export-side file-backed asset embedding, asset diagnostics, fatal handling for missing or unsafe payloads, manifest count summaries under `AssetMode::IncludeLocal`, and exporter regression coverage. Recorded that existing CLI callers still default to `AssetMode::ReferencesOnly`.

2026-05-12 / Codex: Completed Milestone 4. Added import/export CLI asset flags, asset summary output, docs, and CLI integration coverage. Confirmed help text describes offline local asset behavior and avoids promising external downloads.

2026-05-12 / Codex: Completed Milestone 5. Added shared asset-file resolution and read helpers, direct asset tests for digest naming/path containment/hash mismatch behavior, a real binary CLI round-trip proof for file-backed binary and shared-string payloads, and final workspace validation plus manual smoke evidence.
