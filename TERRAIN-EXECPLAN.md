# Implement P1 Terrain Round-Trip Parity

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` at the repository root. If this document is revised during implementation, keep it self-contained and update every affected section before stopping work.

## Purpose / Big Picture

After this change, RbxSync users can import a local Roblox place file that contains Terrain voxel payloads, keep those payloads in a stable project filesystem representation, and export the project back to a `.rbxl` or `.rbxlx` without silently dropping the terrain data. Terrain is the Roblox service-like object under `Workspace` that stores editable landscape voxels such as grass, rock, water, and occupancy values. A voxel is one small 3D terrain cell; the place file stores those cells in binary properties that normal instance metadata does not explain well.

The visible proof of success is an offline round trip: create or load a place file with a `Workspace/Terrain` instance containing binary terrain payload properties, run `rbxsync import-place --terrain --output ./Game --force --json`, observe a terrain manifest and blob files under the project, then run `rbxsync extract-place --path ./Game --output ./build/Game.rbxl --force --json` and import that output again. The second imported project must contain the same terrain payload file hashes and the command JSON must report terrain data as preserved rather than metadata-only.

## Progress

- [x] (2026-05-14T00:20Z) Read `PLANS.md`, `ROADMAP-ISSUES.md`, `EXTRACT-PLACE-EXECPLAN.md`, `ASSETS-EXECPLAN.md`, current local place import/export code in `rbxsync-core/src/place_importer.rs` and `rbxsync-core/src/place_exporter.rs`, the shared writer in `rbxsync-core/src/extract_writer.rs`, CLI wiring in `rbxsync-cli/src/main.rs`, CLI integration tests under `rbxsync-cli/tests/`, Studio terrain handling in `plugin/src/TerrainHandler.luau`, and server terrain endpoints in `rbxsync-server/src/lib.rs`.
- [x] (2026-05-14T00:20Z) Created this initial ExecPlan for P1 Terrain Round-Trip Parity with milestones for format stabilization, raw payload import, raw payload export, CLI/reporting/docs, and plugin/server compatibility.
- [x] (2026-05-14T00:43Z) Completed Milestone 1 by adding `rbxsync-core/src/terrain.rs`, exporting its public API from `rbxsync-core/src/lib.rs`, defining the raw terrain manifest and legacy chunk data model, and adding helpers for canonical/legacy path lookup, atomic raw manifest writes, SHA-256 terrain blob writes, safe payload reads, raw Terrain instance extraction, and terrain summaries.
- [x] (2026-05-14T00:43Z) Added focused terrain tests for canonical and legacy paths, raw manifest write/read, payload hash mismatch detection, legacy chunk reads, raw payload extraction from a Terrain instance, and Roblox crate round-trip feasibility through `.rbxl` / `.rbxlx`.
- [x] (2026-05-14T00:44Z) Ran `mise exec -- cargo fmt`, `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core terrain`, `mise exec -- cargo test -p rbxsync-core`, and `git diff --check`; all passed. The terrain-focused test command ran 8 matching tests because the filter also matched existing place importer terrain tests; the full core suite passed 83 tests.
- [x] (2026-05-14T00:56Z) Completed Milestone 2 by extending `PlaceImportResult` with collected raw terrain extraction data, teaching `rbxsync-core/src/place_importer.rs` to collect raw Terrain `BinaryString` and `SharedString` payloads when `include_terrain` is true, skipping those large payload properties from normal instance metadata, and suppressing the metadata-only Terrain diagnostic when payloads were preserved.
- [x] (2026-05-14T00:56Z) Wired `rbxsync-cli/src/main.rs::cmd_import_place` so non-dry-run `import-place --terrain` writes `terrain/Workspace/Terrain.rbxterrain.json` plus `terrain/blobs/<sha256>.bin` after overwrite checks, and JSON/human summaries include an optional `terrain` object/section.
- [x] (2026-05-14T00:56Z) Added importer and CLI regression coverage for Terrain payload import. `rbxsync-core` now has `place_importer::tests::extracts_terrain_payloads_without_metadata_only_diagnostic`, and `rbxsync-cli/tests/import_place.rs` now has `import_place_terrain_writes_manifest_and_blobs`.
- [x] (2026-05-14T00:56Z) Ran `mise exec -- cargo fmt`, `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core place_importer`, `mise exec -- cargo test -p rbxsync --test import_place`, `mise exec -- cargo test -p rbxsync-core`, and `git diff --check`; all passed. The full core suite passed 84 tests.
- [x] (2026-05-14T01:06Z) Completed Milestone 3 by adding a post-build Terrain application phase to `rbxsync-core/src/place_exporter.rs`. `extract-place` now reads `terrain/Workspace/Terrain.rbxterrain.json`, ensures `Workspace/Terrain` exists when `Workspace` is exported, applies raw metadata properties, reads and verifies file-backed Terrain blobs, and restores `BinaryString` / `SharedString` payload properties before writing `.rbxl` / `.rbxlx`.
- [x] (2026-05-14T01:06Z) Extended `PlaceExportSummary` and `rbxsync-cli/src/main.rs::print_export_summary` with optional Terrain summary reporting so `extract-place --json` includes the same terrain object shape as import summaries when terrain is found.
- [x] (2026-05-14T01:06Z) Added exporter diagnostics for invalid raw Terrain manifests, missing Terrain payloads, Terrain payload hash mismatches, and Terrain payload paths outside the project. Raw manifest failures are fatal to avoid silent data loss; legacy Studio chunk terrain remains a non-fatal unsupported export diagnostic unless strict mode is enabled.
- [x] (2026-05-14T01:06Z) Added focused exporter and real CLI coverage for raw terrain export, hash-mismatch failure, `--services` Workspace filtering, legacy chunk diagnostics, and `.rbxl` export followed by `import-place --terrain` re-import.
- [x] (2026-05-14T01:15Z) Completed Milestone 4 by adding `diagnosticCount` to `TerrainSummary`, asserting the command JSON terrain shape in importer/exporter CLI tests, and documenting raw Terrain import/export behavior in `docs/cli/commands.md`, `docs/file-formats/rbxjson.md`, new `docs/file-formats/terrain.md`, `docs/file-formats/index.md`, `docs/serialization.md`, and `README.md`.
- [x] (2026-05-14T01:15Z) Updated the VitePress file-format sidebar to include the new Terrain Files page and removed stale documentation that claimed Terrain voxel data is stored as base64 in `.rbxjson`.
- [x] (2026-05-14T01:24Z) Completed Milestone 5 by adding shared Studio terrain lookup in `rbxsync-core/src/terrain.rs`, wiring `cmd_sync` and `/sync/read-terrain` through it, preserving legacy chunk terrain as the Studio-applicable format, and reporting raw manifests as present but not applicable to Studio `WriteVoxels`.
- [x] (2026-05-14T01:24Z) Added plugin-side guards so raw manifest-shaped payloads return a clear "local place export/import" error instead of being treated as empty Studio terrain data.
- [x] (2026-05-14T01:24Z) Added focused core and server tests for Studio terrain lookup priority, raw-manifest detection, legacy chunk responses, and raw-manifest warning responses.

## Surprises & Discoveries

- Observation: Local place import currently treats Terrain as metadata-only even when `--terrain` is used.
  Evidence: `rbxsync-core/src/place_importer.rs::serialize_instance` pushes `ImportDiagnosticKind::UnsupportedTerrainVoxelData` for every instance whose class is `Terrain`, with message "Terrain voxel data is not converted by place import; metadata properties only".

- Observation: `import-place` already has a user-facing `--terrain` flag, but `extract-place` has no corresponding flag.
  Evidence: `rbxsync-cli/src/main.rs::Commands::ImportPlace` includes `terrain: bool`, and `cmd_import_place` passes it into `PlaceImportOptions.include_terrain`; `Commands::ExtractPlace` has output, format, strict, services, packages, and assets flags but no terrain flag.

- Observation: The Studio plugin and server already exchange a JSON terrain chunk format, but that format is not currently part of local place import/export.
  Evidence: `plugin/src/TerrainHandler.luau` returns terrain data shaped as `chunkSize`, `resolution`, `region`, `chunks`, and `properties`, while `rbxsync-server/src/lib.rs::handle_extract_terrain` writes that payload to `src/Workspace/Terrain/terrain.rbxjson`.

- Observation: The shared writer preserves an existing legacy terrain file across extraction finalization.
  Evidence: `rbxsync-core/src/extract_writer.rs::write_serialized_instances` reads `src/Workspace/Terrain/terrain.rbxjson` before replacing `src/`, then recreates that file after the new instance tree is written.

- Observation: The repository already creates a top-level `terrain/` directory for new projects, but the live Studio sync path currently looks under `src/Workspace/Terrain/terrain.rbxjson`.
  Evidence: `rbxsync-cli/src/main.rs::cmd_init` creates `terrain_dir = project_dir.join("terrain")`; `cmd_sync` checks `project_dir/src/Workspace/Terrain/terrain.rbxjson`, and the server `/sync/read-terrain` endpoint checks `src/Workspace/Terrain.rbxjson` and `src/Workspace/Terrain/terrain.rbxjson`.

- Observation: The Roblox Rust crates can preserve opaque binary properties even when RbxSync cannot decode their semantic voxel contents.
  Evidence: `rbxsync-core/src/place_importer.rs::variant_to_json_property` already converts `Variant::BinaryString` and `Variant::SharedString` into JSON, and `rbxsync-core/src/place_exporter.rs::json_to_variant` already converts those typed JSON values back into `Variant::BinaryString` and `Variant::SharedString`.

- Observation: The new terrain feasibility test proves arbitrary Terrain `BinaryString` payloads survive both `.rbxl` and `.rbxlx` serialization, while the arbitrary test `SharedString` payload survives `.rbxl` but was not preserved as a `SharedString` by the `.rbxlx` round trip.
  Evidence: `rbxsync-core/src/terrain.rs::tests::roblox_binary_and_xml_preserve_opaque_terrain_payloads` asserts `SmoothGrid` as `Variant::BinaryString` for both formats and only asserts the arbitrary `SharedVoxelData` `Variant::SharedString` for `rbxl`.

- Observation: Terrain can reuse the asset module's SHA-256 and safe file-read policy without coupling terrain manifests to the asset manifest format.
  Evidence: `rbxsync-core/src/terrain.rs` calls `asset_sha256_hex` and `read_asset_file`, but stores payload references in `TerrainPayloadRef` and raw terrain manifests under `terrain/Workspace/Terrain.rbxterrain.json`.

- Observation: The importer needs a collect-then-write Terrain flow rather than writing directly while parsing the place file.
  Evidence: Milestone 2 added `RawTerrainExtraction` and `collect_raw_terrain_from_instance` so `import_place_file` can return terrain payload bytes in memory; `cmd_import_place` writes them with `write_raw_terrain_extraction` only after non-dry-run overwrite checks and project setup.

- Observation: Once raw Terrain payloads are collected, the normal `src/Workspace/Terrain.rbxjson` metadata should not contain those blob properties.
  Evidence: `place_importer::tests::extracts_terrain_payloads_without_metadata_only_diagnostic` and `import_place_terrain_writes_manifest_and_blobs` both assert that `SmoothGrid` is absent from normal Terrain metadata while `Decoration` remains present.

- Observation: The exporter can restore Terrain payloads as a post-build DOM enrichment without needing the normal source tree to contain voxel metadata.
  Evidence: `place_exporter::tests::embeds_raw_terrain_manifest_payloads` builds `Workspace/Terrain` from ordinary metadata, applies `terrain/Workspace/Terrain.rbxterrain.json`, and asserts `SmoothGrid` is present as a `Variant::BinaryString` in the built DOM.

- Observation: `extract-place --services` is a reliable guardrail for terrain export.
  Evidence: `place_exporter::tests::raw_terrain_respects_workspace_service_filter` creates a deliberately hash-mismatched raw Terrain manifest and proves export succeeds with no terrain summary when the service filter excludes `Workspace`.

- Observation: The docs still had pre-parity Terrain language that described voxel data as inline/base64 `.rbxjson`.
  Evidence: Milestone 4 replaced the `docs/serialization.md` Terrain section and added `docs/file-formats/terrain.md`; `rg -n 'terrainData|base64-encoded-terrain|Terrain voxel data is base64' README.md docs` now returns no matches.

- Observation: The local docs build cannot run in this checkout without installing docs dependencies.
  Evidence: `cd docs && npm run build` failed with `sh: vitepress: command not found` because `docs/node_modules` is absent.

- Observation: Studio sync must prefer legacy chunk terrain even when a raw manifest exists.
  Evidence: `terrain::tests::studio_sync_lookup_prefers_legacy_chunks_over_raw_manifest` creates both files and asserts the shared lookup returns `src/Workspace/Terrain/terrain.rbxjson`.

- Observation: Raw terrain manifests can be detected by server sync endpoints without being handed to the Studio plugin.
  Evidence: `rbxsync-server::tests::sync_read_terrain_reports_raw_manifest_warning` returns `hasTerrain: true`, `terrainFormat: "rawProperties"`, the manifest path, and a warning, but no `terrain` payload.

- Observation: The repository automated test script currently fails before terrain-specific behavior because `serve --background` reports ports as unavailable in this local environment.
  Evidence: `./testing/scripts/run-all-tests.sh` failed only in CLI Test 3/4 background server health/info checks. A visible retry of `./target/release/rbxsync serve --background` reported `Port 44755 is already in use`, and `--port 44756` reported the same even though `curl` could not reach `44755` and `lsof -nP -iTCP:44755 -sTCP:LISTEN` returned no listener.

## Decision Log

- Decision: Use a raw-payload-first terrain representation for local place file parity.
  Rationale: The immediate P1 requirement is round-trip parity for `.rbxl` and `.rbxlx`, not editable voxel decoding. Preserving the exact terrain binary properties exposed by `rbx_binary` and `rbx_xml` is deterministic, offline, and testable today. Decoding Roblox's internal voxel format can be added later without blocking parity.
  Date/Author: 2026-05-14 / Codex

- Decision: Store canonical local place terrain data under the existing top-level `terrain/` project directory, with file-backed binary blobs.
  Rationale: `cmd_init` already creates `terrain/`, and keeping large terrain payloads outside `src/` prevents the normal `.rbxjson` instance writer and file watcher from treating voxel files as ordinary instance metadata. The plan still keeps legacy `src/Workspace/Terrain/terrain.rbxjson` readable for Studio sync compatibility.
  Date/Author: 2026-05-14 / Codex

- Decision: Keep `import-place --terrain` as the opt-in for local place Terrain import.
  Rationale: Existing default behavior skips Terrain unless the user asks for it. This avoids unexpectedly writing large terrain payloads during ordinary imports while making the existing flag more useful when users need parity.
  Date/Author: 2026-05-14 / Codex

- Decision: Make `extract-place` automatically include terrain payloads when a recognized terrain file is present and the exported services include `Workspace`.
  Rationale: The project already contains the intent once a terrain manifest exists. A second flag would make round-trip behavior easy to forget, while `--services` still gives users a way to exclude `Workspace` entirely.
  Date/Author: 2026-05-14 / Codex

- Decision: Treat Studio chunk terrain and raw local place terrain as two supported input shapes with different export guarantees.
  Rationale: The plugin's `ReadVoxels` / `WriteVoxels` chunk format is useful for Studio sync, but it is not the same as the opaque binary payload stored in a `.rbxl`. Until a converter from chunks to Roblox's serialized terrain binary exists, `extract-place` should preserve raw payloads and emit a clear diagnostic for chunk-only data that cannot be written to a place file.
  Date/Author: 2026-05-14 / Codex

- Decision: Keep `TerrainPayloadRef` terrain-specific instead of reusing `AssetEntry`.
  Rationale: Terrain payloads need a canonical Terrain path, voxel property names, and future import/export diagnostics that are distinct from asset references. Reusing the digest and path helpers is enough to keep safety policy consistent without making terrain files look like generic assets.
  Date/Author: 2026-05-14 / Codex

- Decision: Store collected Terrain payload bytes in `PlaceImportResult` and let the CLI write them.
  Rationale: `import_place_file` does not own command overwrite policy or the final output directory lifecycle. Returning a `RawTerrainExtraction` keeps parsing side-effect-free, supports dry-run summaries, and prevents terrain files from being created when `import-place` would later fail because `src/` already exists without `--force`.
  Date/Author: 2026-05-14 / Codex

- Decision: Apply raw Terrain data after normal project DOM construction rather than teaching every source-tree insertion path about terrain.
  Rationale: The normal exporter should continue using `src/` metadata for ordinary instance shape and diagnostics. A post-build phase keeps raw voxel payload handling localized, lets existing `Workspace/Terrain` metadata be reused, and can create the missing Terrain instance only when a canonical raw manifest exists.
  Date/Author: 2026-05-14 / Codex

- Decision: Make raw manifest read and payload verification failures fatal, while keeping legacy chunk-only terrain as a warning.
  Rationale: A raw manifest is the user's explicit local place terrain representation, so exporting without required bytes would silently lose data. Legacy Studio chunks cannot yet be converted to place-file binary data, so they should warn clearly and participate in strict-mode failure without blocking ordinary metadata-only export.
  Date/Author: 2026-05-14 / Codex

## Outcomes & Retrospective

This initial plan turns the roadmap item "P1: Terrain Round-Trip Parity" into a concrete local file implementation path. No code has been changed yet. The intended result is that local place files with Terrain binary properties can be imported, stored as deterministic file-backed data, and exported back without losing those properties. The plan deliberately separates that round-trip goal from richer voxel editing or Studio-only chunk conversion.

The main unresolved risk is exactly which terrain binary properties the current `rbx_binary` and `rbx_xml` crates expose for real Roblox-authored Terrain instances. Milestone 1 includes a fixture/probe step so implementation is driven by actual DOM data rather than assumptions. If the crates do not expose voxel payloads for some place files, the implementation must emit a diagnostic that says the terrain data was not available through the local parser, and it must keep metadata-only Terrain behavior working.

Milestone 1 is complete. `rbxsync-core/src/terrain.rs` now defines the terrain manifest schema, the `TerrainProjectData` enum for canonical raw manifests and legacy Studio chunk files, file-backed payload references, terrain summaries, and terrain diagnostics. The module is exported from `rbxsync-core/src/lib.rs` and includes focused unit coverage for manifest IO, hash checking, legacy chunk reads, and opaque payload feasibility. User-facing import/export behavior is intentionally unchanged until Milestones 2 and 3 wire this module into the command paths.

Milestone 2 is complete. `import-place --terrain` now preserves raw Terrain payloads that the local Roblox parser exposes as opaque binary properties. The importer collects those payloads into `PlaceImportResult.terrain`, omits them from normal `.rbxjson` metadata, and avoids the old metadata-only diagnostic when payloads were preserved. The CLI writes the canonical terrain manifest and blob files during non-dry-run imports and reports the terrain summary in JSON and human output. Export parity is not complete yet; Milestone 3 still needs to consume the manifest during `extract-place`.

Milestone 3 is complete. `extract-place` now consumes the canonical raw terrain manifest during export, verifies terrain blob files with the same project-relative and SHA-256 policy used by file-backed assets, and embeds the restored Terrain payload properties into the DOM before writing a place file. The implementation also reports terrain summaries in export JSON and leaves legacy Studio chunk terrain readable but unsupported for place-file conversion. Documentation and broader command coverage remain in Milestone 4, and Studio/server compatibility remains in Milestone 5.

Milestone 4 is complete. Terrain command summaries now include `diagnosticCount` inside the nested `terrain` object, and CLI integration tests assert that shape for import and export. The user-facing docs now describe the raw `.rbxterrain.json` manifest, blob layout, automatic `extract-place` inclusion, fatal raw manifest/payload errors, and the legacy Studio chunk limitation. The docs build was attempted but blocked by missing local VitePress dependencies; Rust and CLI validation passed.

Milestone 5 is complete. The Rust side now has a shared terrain-file lookup for Studio sync that finds legacy chunk data first and raw manifests second. `cmd_sync` still sends legacy chunks to Studio, but warns and skips when only raw local place terrain exists. `/sync/read-terrain` returns legacy chunks with `terrainFormat: "chunks"` and raw manifests with `terrainFormat: "rawProperties"` plus a warning that Studio `WriteVoxels` cannot apply that format yet. The plugin also rejects raw manifest-shaped terrain payloads with the same explicit message.

## Context and Orientation

RbxSync converts Roblox projects between a filesystem layout and Roblox place artifacts. A place artifact is a `.rbxl` binary file or `.rbxlx` XML file. `rbxsync import-place` reads a place artifact and writes a project with `rbxsync.json`, `src/`, optional `assets/`, and currently an empty top-level `terrain/` directory created by `rbxsync init`. `rbxsync extract-place` reads that project and writes a place artifact.

The local importer lives in `rbxsync-core/src/place_importer.rs`. It reads `.rbxl` through `rbx_binary::from_reader`, reads `.rbxlx` through `rbx_xml::from_reader_default`, walks the `rbx_dom_weak::WeakDom`, and produces plugin-compatible serialized instance JSON. `WeakDom` is the in-memory Roblox instance tree used by the Rust Roblox crates. A `Variant` is one typed property value in that tree, such as `String`, `Vector3`, `BinaryString`, `SharedString`, or `MaterialColors`.

The local exporter lives in `rbxsync-core/src/place_exporter.rs`. It reads the project `src/` tree, converts `.rbxjson` metadata into `Variant` values, builds a `WeakDom`, and writes `.rbxl` or `.rbxlx` with `rbx_binary` or `rbx_xml`. The exporter already knows how to read file-backed `BinaryString` and `SharedString` asset payloads from `assets/blobs/` when `--include-assets` is used. Terrain should follow the same safety principles: project-relative paths only, optional hashes verified, and no network access.

The shared filesystem writer lives in `rbxsync-core/src/extract_writer.rs`. It writes serialized instance JSON to `src/`, splitting scripts into `.luau` source files and metadata into `.rbxjson`. It currently has special-case logic to preserve `src/Workspace/Terrain/terrain.rbxjson` before replacing `src/`, because the Studio extraction path writes terrain chunks there.

The Studio terrain path is separate from local place import/export. `plugin/src/TerrainHandler.luau` reads terrain from Studio with `Terrain:ReadVoxels`, compresses each chunk's material IDs with simple run-length encoding, quantizes occupancy to integers from 0 through 255, and posts batches to `rbxsync-server`. The server route `rbxsync-server/src/lib.rs::handle_extract_terrain` writes the received JSON to `src/Workspace/Terrain/terrain.rbxjson`. The plugin can later apply that shape back to Studio with `Terrain:WriteVoxels`.

For this plan, "raw payload" means a binary property from a `Terrain` instance that the Roblox Rust crates expose as `Variant::BinaryString`, `Variant::SharedString`, or another exact typed value that can be serialized back without decoding into editable voxels. "Chunk terrain" means the existing Studio JSON shape with `chunkSize`, `resolution`, `region`, `chunks`, and `properties`. Raw payload terrain can be written back into local place files. Chunk terrain can be synced to Studio through the existing plugin path, but cannot be written into `.rbxl` until a converter exists.

The canonical terrain project layout after this plan should be:

    terrain/
      Workspace/
        Terrain.rbxterrain.json
      blobs/
        <sha256>.bin

`terrain/Workspace/Terrain.rbxterrain.json` is the manifest for the Terrain instance at DataModel path `Workspace/Terrain`. The file extension is intentionally not `.rbxjson` so the normal instance tree writer and server instance scanners do not mistake it for ordinary metadata. The manifest is deterministic JSON with schema version `1`, project-relative blob file paths, and sorted property names.

A raw local place terrain manifest should look like this:

    {
      "version": 1,
      "format": "rawProperties",
      "terrainPath": "Workspace/Terrain",
      "className": "Terrain",
      "name": "Terrain",
      "referenceId": "RBX...",
      "metadataProperties": {
        "Decoration": { "type": "bool", "value": true },
        "WaterTransparency": { "type": "float", "value": 0.3 }
      },
      "materialColors": {
        "Grass": { "r": 0.41568628, "g": 0.49019608, "b": 0.2784314 }
      },
      "voxelProperties": {
        "SmoothGrid": {
          "type": "BinaryString",
          "value": {
            "file": "terrain/blobs/<sha256>.bin",
            "encoding": "raw",
            "sha256": "<sha256>",
            "byteLength": 12345
          }
        }
      },
      "diagnostics": []
    }

A legacy Studio chunk file remains valid input if it has the existing shape:

    {
      "chunkSize": 32,
      "resolution": 4,
      "region": { "min": [-64, -16, -64], "max": [64, 32, 64] },
      "chunks": [
        { "x": 0, "y": 0, "z": 0, "materials": [10, 1], "occupancies": [255] }
      ],
      "properties": {
        "WaterTransparency": 0.3,
        "materialColors": {
          "Grass": { "r": 0.4, "g": 0.5, "b": 0.3 }
        }
      }
    }

The implementation should normalize both shapes into one Rust `TerrainProjectData` enum. Raw property terrain is exportable to place files. Chunk terrain is readable for server/plugin sync and should produce `UnsupportedTerrainVoxelData` when `extract-place` is asked to write it to a place file.

## Plan of Work

Milestone 1 creates the shared terrain model and file helpers. Add a new `rbxsync-core/src/terrain.rs` module and export it from `rbxsync-core/src/lib.rs`. Define `TerrainProjectData`, `RawTerrainData`, `ChunkTerrainData`, `TerrainPayloadRef`, `TerrainSummary`, `TerrainDiagnostic`, and `TerrainDiagnosticKind`. Add helper functions to locate canonical and legacy terrain files, read a terrain manifest, write a manifest atomically, write blob files by SHA-256, and resolve project-relative blob paths. Reuse the asset module's path-containment and SHA-256 helpers if they fit; otherwise move any common digest/path code into a shared helper rather than duplicating policy.

Milestone 1 must include a small feasibility probe in a test or temporary helper that constructs a `WeakDom` with a `Terrain` instance containing binary properties and proves the Rust crates can write and read those properties through both `.rbxl` and `.rbxlx`. The test fixture does not need real Roblox-authored terrain yet; it should prove the round-trip mechanism for opaque terrain payload properties. Add a note to `Surprises & Discoveries` with the observed property names and variant types from the test.

Milestone 2 wires local place import. Extend `PlaceImportOptions` with a terrain output mode if needed, but keep the user-facing behavior simple: when `include_terrain` is false, keep skipping Terrain as today; when `include_terrain` is true, serialize the `Terrain` instance metadata and also extract exportable raw terrain payloads to `terrain/Workspace/Terrain.rbxterrain.json` and `terrain/blobs/<sha256>.bin`. The importer should no longer emit the old generic `UnsupportedTerrainVoxelData` diagnostic when it successfully writes at least one raw voxel payload. It should still emit a diagnostic when Terrain exists but no exportable voxel payloads are visible to the Rust parser.

Milestone 2 should keep ordinary Terrain metadata under `src/Workspace/Terrain/_meta.rbxjson` or `src/Workspace/Terrain.rbxjson` according to the existing writer's normal rules, but it should move large raw voxel binary properties out of normal metadata and into the terrain manifest. The normal metadata should not contain huge base64 `SmoothGrid`-like values after a successful `--terrain` import. Metadata-only properties such as `Decoration`, water properties, attributes, tags, and references should remain in normal metadata when the current writer supports them.

Milestone 3 wires local place export. Extend `PlaceExportSummary` with an optional `terrain_summary`, and teach `build_project_dom` or a post-build phase to read terrain data for `Workspace/Terrain`. If the canonical raw manifest exists, ensure the DOM contains a `Terrain` instance at `Workspace/Terrain`, apply metadata properties from the manifest, read each blob file, verify any `sha256`, and set the corresponding `Variant::BinaryString` or `Variant::SharedString` property on that instance. If a legacy chunk file exists but no raw manifest exists, keep exporting the metadata-only Terrain instance if present and add `PlaceExportDiagnosticKind::UnsupportedTerrainVoxelData` explaining that Studio chunk terrain cannot yet be converted to place-file terrain binary.

Milestone 3 must respect service filtering. If `extract-place --services` excludes `Workspace`, it should not read or apply `Workspace/Terrain` terrain data. If `--services Workspace` or no service filter is used, terrain data should be included automatically when the manifest exists. Missing blob files, hash mismatches, outside-project paths, and invalid manifests should be fatal because exporting a place without required terrain bytes would silently lose data. Invalid legacy chunk files can be non-fatal diagnostics unless strict mode is enabled, because they cannot be exported anyway.

Milestone 4 adds user-visible reporting, tests, and documentation. JSON output for `import-place --terrain --json` and `extract-place --json` should include a `terrain` object when terrain was requested or found. The object should report at least `mode`, `manifest`, `rawPayloads`, `chunkCount`, `bytesWritten`, `bytesRead`, and `diagnosticCount`. Human output should include a concise terrain line only when terrain is involved. Update `docs/cli/commands.md`, `docs/file-formats/rbxjson.md` or a new terrain format page, and README command examples as needed. The docs must state that local place round-trip parity preserves raw terrain payloads offline, while Studio chunk terrain remains a sync format unless and until a converter is added.

Milestone 4 should add focused unit tests in `rbxsync-core/src/terrain.rs`, importer tests in `rbxsync-core/src/place_importer.rs`, exporter tests in `rbxsync-core/src/place_exporter.rs`, and real-binary CLI integration tests under `rbxsync-cli/tests/import_place.rs` and `rbxsync-cli/tests/extract_place.rs`. The key end-to-end test should create a temporary project or DOM with a `Workspace/Terrain` raw binary property, write it to `.rbxl`, import it with `--terrain --json`, verify the terrain manifest and blob hash, export it with `extract-place --json`, import that output again with `--terrain --json`, and assert that the final manifest references the same blob bytes.

Milestone 5 updates Studio plugin and server compatibility. Add shared terrain path lookup on the Rust side so `cmd_sync`, `/sync/read-terrain`, and any scanner that skips `terrain.rbxjson` can also recognize `terrain/Workspace/Terrain.rbxterrain.json`. Update `plugin/src/TerrainHandler.luau` only as much as necessary to keep applying legacy chunk terrain and to ignore raw terrain manifests it cannot apply. If plugin support for raw manifests is not practical in this milestone, the plugin should return a clear "raw terrain manifests require local place export/import" message rather than failing obscurely. Keep legacy `src/Workspace/Terrain/terrain.rbxjson` readable throughout.

## Concrete Steps

Work from the repository root:

    cd /Users/lexiviripaeff/Documents/LoganX/rbxsync

Before editing, inspect the current code paths:

    rg -n "Terrain|terrain|UnsupportedTerrainVoxelData|include_terrain|PlaceImportOptions|PlaceExportOptions" rbxsync-core rbxsync-cli rbxsync-server plugin/src docs
    sed -n '1,360p' rbxsync-core/src/place_importer.rs
    sed -n '1,220p' rbxsync-core/src/place_exporter.rs
    sed -n '560,900p' rbxsync-core/src/place_exporter.rs
    sed -n '240,340p' rbxsync-core/src/extract_writer.rs
    sed -n '1320,1745p' rbxsync-cli/src/main.rs
    sed -n '1,520p' plugin/src/TerrainHandler.luau
    sed -n '1950,2066p' rbxsync-server/src/lib.rs
    sed -n '2763,2818p' rbxsync-server/src/lib.rs

For Milestone 1, create `rbxsync-core/src/terrain.rs`. A reasonable API shape is:

    pub enum TerrainProjectData {
        Raw(RawTerrainData),
        Chunks(ChunkTerrainData),
    }

    pub struct RawTerrainData {
        pub version: u32,
        pub terrain_path: String,
        pub class_name: String,
        pub name: String,
        pub reference_id: Option<String>,
        pub metadata_properties: serde_json::Map<String, serde_json::Value>,
        pub material_colors: serde_json::Map<String, serde_json::Value>,
        pub voxel_properties: BTreeMap<String, TerrainPayloadRef>,
    }

    pub struct TerrainPayloadRef {
        pub property_type: TerrainPayloadType,
        pub file: String,
        pub sha256: String,
        pub byte_length: u64,
    }

    pub fn canonical_terrain_manifest(project_dir: &Path, terrain_path: &str) -> PathBuf;
    pub fn legacy_terrain_chunk_file(project_dir: &Path) -> PathBuf;
    pub fn read_terrain_project_data(project_dir: &Path) -> Result<Option<TerrainProjectData>>;
    pub fn write_raw_terrain_data(project_dir: &Path, data: &RawTerrainData) -> Result<TerrainSummary>;
    pub fn extract_raw_terrain_from_instance(instance: &rbx_dom_weak::Instance, path: &str, project_dir: &Path) -> Result<Option<RawTerrainData>>;

Use `BTreeMap` for manifest properties that must be stable in JSON. Use project-relative paths such as `terrain/blobs/<sha256>.bin`. Write manifests through a temporary sibling file followed by rename so a failed write does not leave partial JSON.

For Milestone 2, change `rbxsync-core/src/place_importer.rs`. The cleanest implementation is to make `import_dom` collect optional terrain side effects in `PlaceImportResult`, for example:

    pub struct PlaceImportResult {
        pub instances: Vec<Value>,
        pub diagnostics: Vec<ImportDiagnostic>,
        pub format: PlaceFileFormat,
        pub terrain: Option<TerrainSummary>,
    }

Because `import_place_file` currently only returns in-memory instance JSON and does not know the output project directory, the actual blob writes may need to happen in `cmd_import_place` after `project_dir` is known. In that case, return a `RawTerrainData` object with in-memory bytes or temporary extracted payloads from core, and let the CLI call `write_raw_terrain_data(&project_dir, ...)` before `write_serialized_instances`. Do not write project files from deep inside `import_place_file` unless the options explicitly include a project directory.

For Milestone 2, update `cmd_import_place` in `rbxsync-cli/src/main.rs` so a non-dry-run `--terrain` import writes the terrain manifest before or after the normal `src/` write. If `dry_run` is true, do not write blobs; instead report how many payloads would be written and how many bytes they contain. If `--terrain` is absent, do not create `terrain/` and preserve current behavior.

For Milestone 3, change `rbxsync-core/src/place_exporter.rs`. Add a phase after basic DOM construction that reads terrain data and applies it to the DOM. A function like this keeps the behavior localized:

    fn apply_project_terrain(dom: &mut WeakDom, options: &PlaceExportOptions, diagnostics: &mut Vec<PlaceExportDiagnostic>) -> Result<Option<TerrainSummary>>;

That function should find or create the `Workspace` service and then find or create the `Terrain` child under it only when raw terrain data exists and `Workspace` is selected. If a normal `src/Workspace/Terrain` metadata directory already created the instance, reuse it by DataModel path rather than inserting a duplicate. Apply raw voxel properties with exact names from the manifest.

For Milestone 3, extend diagnostics with terrain-specific fatal kinds if the generic kind is not enough:

    InvalidTerrainManifest
    MissingTerrainPayload
    TerrainPayloadHashMismatch
    TerrainPayloadOutsideProject
    UnsupportedTerrainVoxelData

If similar asset diagnostics already exist, reuse the naming pattern in `PlaceExportDiagnosticKind` but keep messages terrain-specific.

For Milestone 4, update summary printers in `rbxsync-cli/src/main.rs`. The JSON shape should be stable and automation-friendly:

    {
      "success": true,
      "command": "import-place",
      "terrain": {
        "mode": "rawProperties",
        "manifest": "terrain/Workspace/Terrain.rbxterrain.json",
        "rawPayloads": 2,
        "chunkCount": null,
        "bytesWritten": 12345,
        "diagnosticCount": 0
      }
    }

When terrain is absent and not requested, either omit `terrain` or set it to `null`; be consistent with existing `assets` summary behavior, which uses `null` when not applicable.

For Milestone 4 tests, use `CARGO_BIN_EXE_rbxsync` and `RBXSYNC_VERSION_CHECK=1`, matching `rbxsync-cli/tests/import_place.rs` and `rbxsync-cli/tests/extract_place.rs`. Do not require Roblox Studio or network access. A representative integration test should:

    create tempdir
    build a WeakDom with DataModel -> Workspace -> Terrain
    set Terrain property "SmoothGrid" to Variant::BinaryString(vec![1, 2, 3, 4, 5])
    write tempdir/source.rbxl with rbx_binary::to_writer
    run rbxsync import-place source.rbxl --output tempdir/project --force --terrain --json
    assert project/terrain/Workspace/Terrain.rbxterrain.json exists
    assert project/terrain/blobs/<sha>.bin contains [1, 2, 3, 4, 5]
    run rbxsync extract-place --path tempdir/project --output tempdir/roundtrip.rbxl --force --json
    run rbxsync import-place roundtrip.rbxl --output tempdir/roundtrip-project --force --terrain --json
    assert the final terrain blob bytes equal [1, 2, 3, 4, 5]

For Milestone 5, centralize terrain file lookup in core and update current call sites. `cmd_sync` should stop hard-coding only `src/Workspace/Terrain/terrain.rbxjson`; it should call a helper that returns the best supported terrain file and its format. `rbxsync-server/src/lib.rs::handle_sync_read_terrain` should continue returning the legacy chunk JSON for Studio when the legacy file exists. If only a raw manifest exists, return `hasTerrain: true`, `terrainFormat: "rawProperties"`, and a clear warning or error that Studio WriteVoxels sync cannot apply this format yet.

## Validation and Acceptance

After Milestone 1, run:

    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core terrain
    git diff --check

Acceptance for Milestone 1 is that terrain manifest read/write unit tests pass, path containment and hash verification are covered, and a Rust fixture proves a Terrain `BinaryString` property can survive `.rbxl` and `.rbxlx` serialization through the local crates.

Milestone 1 validation completed on 2026-05-14T00:43Z:

    mise exec -- cargo fmt
    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core terrain
    mise exec -- cargo test -p rbxsync-core
    git diff --check

The focused test command passed 8 tests. The matching test set included 6 new terrain module tests and 2 existing place importer tests whose names include terrain. The full `rbxsync-core` package test passed 83 tests and doc-tests reported 0 tests.

After Milestone 2, run:

    mise exec -- cargo test -p rbxsync-core place_importer
    mise exec -- cargo test -p rbxsync --test import_place

Acceptance for Milestone 2 is that `import-place --terrain --json` writes a canonical terrain manifest and blob files for a fixture Terrain binary payload, reports the terrain summary in JSON, and no longer reports `UnsupportedTerrainVoxelData` for terrain payloads it preserved.

Milestone 2 validation completed on 2026-05-14T00:56Z:

    mise exec -- cargo fmt
    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core place_importer
    mise exec -- cargo test -p rbxsync --test import_place
    mise exec -- cargo test -p rbxsync-core
    git diff --check

The focused importer command passed 8 tests. The CLI import-place suite passed 5 tests, including `import_place_terrain_writes_manifest_and_blobs`. The full `rbxsync-core` package test passed 84 tests and doc-tests reported 0 tests.

After Milestone 3, run:

    mise exec -- cargo test -p rbxsync-core place_exporter
    mise exec -- cargo test -p rbxsync --test extract_place

Acceptance for Milestone 3 is that `extract-place` reads the canonical raw terrain manifest, embeds the raw payload properties into the exported place file, fails on missing or hash-mismatched terrain blobs, and emits a clear unsupported diagnostic for legacy chunk-only terrain data.

Milestone 3 validation completed on 2026-05-14T01:06Z:

    mise exec -- cargo fmt
    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core place_exporter
    mise exec -- cargo test -p rbxsync --test extract_place
    mise exec -- cargo test -p rbxsync --test import_place
    mise exec -- cargo test -p rbxsync-core
    git diff --check

The focused exporter command passed 11 tests, including raw Terrain payload embedding, hash-mismatch failure, service-filter skipping, and legacy chunk diagnostics. The CLI extract-place suite passed 7 tests, including `extract_place_embeds_raw_terrain_manifest_and_reimports_payloads`, which exports a project with a raw Terrain manifest to `.rbxl` and re-imports it with `--terrain` to verify the blob bytes survive.
The import-place suite passed 5 tests as a regression check for Milestone 2, and the full `rbxsync-core` package passed 88 tests and 0 doc-tests.

After Milestone 4, run the focused local place round-trip suite:

    mise exec -- cargo test -p rbxsync-core
    mise exec -- cargo test -p rbxsync --test import_place
    mise exec -- cargo test -p rbxsync --test extract_place
    mise exec -- cargo test -p rbxsync
    mise exec -- cargo fmt -- --check
    git diff --check

Milestone 4 validation completed on 2026-05-14T01:15Z:

    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core terrain
    mise exec -- cargo test -p rbxsync-core place_exporter
    mise exec -- cargo test -p rbxsync --test import_place
    mise exec -- cargo test -p rbxsync --test extract_place
    mise exec -- cargo test -p rbxsync-core
    mise exec -- cargo test -p rbxsync
    git diff --check

The terrain-focused command passed 13 matching tests, the exporter command passed 11 tests, the import-place CLI suite passed 5 tests, the extract-place CLI suite passed 7 tests, the full `rbxsync-core` package passed 88 tests and 0 doc-tests, and the full `rbxsync` package passed 17 tests across its CLI integration suites. Documentation build validation was attempted with `cd docs && npm run build`, but it failed before rendering because `vitepress` is not installed in local `docs/node_modules`.

Before declaring the full plan complete, run:

    mise exec -- cargo test --workspace
    ./testing/scripts/run-all-tests.sh

Milestone 5 validation completed on 2026-05-14T01:24Z:

    mise exec -- cargo fmt
    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core terrain
    mise exec -- cargo test -p rbxsync-server sync_read_terrain
    mise exec -- cargo test -p rbxsync-core
    mise exec -- cargo test -p rbxsync-server
    mise exec -- cargo test -p rbxsync
    mise exec -- cargo test --workspace
    git diff --check

The terrain-focused core command passed 15 matching tests, the focused server command passed 2 terrain sync tests, the full `rbxsync-core` package passed 90 tests, the full `rbxsync-server` package passed 23 tests plus 0 doc-tests, the full `rbxsync` package passed 17 CLI tests, and the workspace suite passed. `./testing/scripts/run-all-tests.sh` was attempted; it built the release binary, passed migration tests and documentation tests, but failed the CLI background-server smoke checks because the local `serve --background` path reported ports as already in use before terrain behavior was exercised.

The final acceptance behavior is:

1. `rbxsync import-place Game.rbxl --terrain --output ./Game --force --json` writes `terrain/Workspace/Terrain.rbxterrain.json` and blob files when the place exposes raw terrain payload properties.
2. The import JSON reports terrain payload counts and byte counts.
3. `rbxsync extract-place --path ./Game --output ./build/Game.rbxl --force --json` writes a non-empty place file that contains the same raw Terrain payload properties.
4. Re-importing the exported file with `--terrain` produces terrain blob files with the same bytes or the same SHA-256 hashes as the first import.
5. Terrain metadata-only cases still work and still produce a diagnostic that explains no exportable voxel payload was found.
6. Legacy Studio chunk terrain at `src/Workspace/Terrain/terrain.rbxjson` remains readable by sync paths and produces a clear unsupported-place-export diagnostic rather than silent data loss.

## Idempotence and Recovery

All new terrain writes must be safe to retry. Blob files are content-addressed by SHA-256, so rewriting the same payload should be a no-op or overwrite with identical bytes. Manifest writes should use a temporary file in the same directory followed by rename. If manifest serialization fails, the previous manifest must remain intact.

No implementation step should delete existing `src/Workspace/Terrain/terrain.rbxjson` unless it has first been copied or normalized into the new representation and tests prove the compatibility path. If both canonical raw terrain and legacy chunk terrain exist, prefer canonical raw terrain for local place export and keep the legacy file for Studio sync. Emit a diagnostic noting that both were found so users know which one was used.

If tests reveal that `rbx_binary` or `rbx_xml` do not expose actual Roblox-authored terrain voxel bytes for a class of files, do not fake success. Preserve metadata, emit `UnsupportedTerrainVoxelData` with a message naming the parser limitation, and document the limitation in this plan and in user docs.

The implementation should not require Roblox Studio, Open Cloud credentials, internet access, or user home directory state for local validation. Use temporary directories in all tests and set `RBXSYNC_VERSION_CHECK=1` for CLI tests to avoid duplicate installation noise.

## Artifacts and Notes

The current local importer terrain limitation is in `rbxsync-core/src/place_importer.rs::serialize_instance`, where every `Terrain` instance gets `UnsupportedTerrainVoxelData`.

The existing local place export path is in `rbxsync-core/src/place_exporter.rs::export_place`, `build_project_dom`, `DomBuilder::insert_directory`, `DomBuilder::insert_metadata_file`, and `DomBuilder::json_to_variant`.

The CLI import and export command functions are `cmd_import_place` and `cmd_extract_place` in `rbxsync-cli/src/main.rs`. The summary printers nearby should be updated so JSON output stays clean and parseable.

The existing Studio terrain shape is defined by `plugin/src/TerrainHandler.luau`. Its `extractTerrain` function returns chunk data, and its `applyTerrain` function expects that same chunk data. Do not assume the plugin can apply raw local place terrain manifests unless this plan is revised with an implementation and validation path for that conversion.

The server currently writes legacy terrain batches in `rbxsync-server/src/lib.rs::handle_extract_terrain` and reads legacy terrain in `handle_sync_read_terrain`. Any compatibility update should include server tests or focused handler-level tests if the crate already has a pattern for those.

## Interfaces and Dependencies

The terrain module should use only existing workspace dependencies unless a focused dependency is justified. The workspace already has `sha2` from asset handling, `serde`, `serde_json`, `anyhow`, `rbx_dom_weak`, `rbx_binary`, and `rbx_xml`.

At the end of this plan, `rbxsync-core/src/lib.rs` should export the public terrain types needed by the CLI and server, but internal parsing helpers can remain private. Public APIs should be stable enough for future Studio chunk conversion work:

    pub use terrain::{
        canonical_terrain_manifest, legacy_terrain_chunk_file, read_terrain_project_data,
        write_raw_terrain_data, TerrainDiagnostic, TerrainDiagnosticKind, TerrainProjectData,
        TerrainSummary,
    };

`PlaceImportResult` and `PlaceExportSummary` should each expose an optional `TerrainSummary`. CLI JSON should derive from those summaries instead of re-scanning files after the fact.

`TerrainSummary` should be small and serializable:

    pub struct TerrainSummary {
        pub mode: TerrainSummaryMode,
        pub manifest: Option<String>,
        pub raw_payloads: usize,
        pub chunk_count: Option<usize>,
        pub bytes_read: u64,
        pub bytes_written: u64,
        pub diagnostics: Vec<TerrainDiagnostic>,
    }

`TerrainDiagnosticKind` should include:

    InvalidTerrainManifest
    MissingTerrainPayload
    TerrainPayloadHashMismatch
    TerrainPayloadOutsideProject
    UnsupportedTerrainVoxelData
    DuplicateTerrainData
    NoTerrainPayloadsFound

When mapping terrain diagnostics into import/export command diagnostics, preserve the specific terrain message. Users need to know whether data was preserved, unsupported because only legacy chunks were present, or unavailable because the local parser did not expose voxel payloads.

Revision note: Initial plan created from `ROADMAP-ISSUES.md` P1 Terrain Round-Trip Parity after reading current importer/exporter, plugin terrain handler, and server terrain endpoints. The plan chooses a raw-payload-first representation to deliver deterministic local place round-trip parity while preserving legacy Studio chunk support.

Revision note: Milestone 1 implemented on 2026-05-14. Added the shared terrain core module, exported its API, validated manifest/blob helpers and opaque payload crate behavior, and recorded the `.rbxlx` arbitrary `SharedString` caveat discovered during testing.

Revision note: Milestone 2 implemented on 2026-05-14. Added side-effect-free raw Terrain collection to local place import, wrote collected payloads from `cmd_import_place`, added terrain summaries to import output, and validated the new behavior with core and real-binary CLI tests.

Revision note: Milestone 3 implemented on 2026-05-14. Added raw Terrain manifest consumption to local place export, restored verified Terrain payload blobs into the exported DOM, surfaced export terrain summaries, and validated raw export/re-import parity with focused core and CLI tests.

Revision note: Milestone 4 implemented on 2026-05-14. Added `diagnosticCount` to Terrain summaries, asserted import/export JSON terrain summaries in CLI tests, documented the raw Terrain manifest and legacy chunk limitation, and recorded the local docs-build dependency blocker.

Revision note: Milestone 5 implemented on 2026-05-14. Added shared Studio terrain file lookup, updated CLI sync and server `/sync/read-terrain` to distinguish legacy chunks from raw manifests, added plugin raw-manifest guards, and recorded the background-server smoke blocker from the automated test script.
