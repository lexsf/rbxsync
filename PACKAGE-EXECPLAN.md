# Implement P2 Package Default Semantics

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` at the repository root. If this document is revised during implementation, keep it self-contained and update every affected section before stopping work.

## Purpose / Big Picture

After this change, `rbxsync extract-place` will have explicit, tested package behavior that matches the roadmap and the product requirement: package folders are exported by default when they are part of the exported project tree, and users can deliberately skip them with `--no-packages`. A package folder means a Wally-style folder named `Packages`, often under `ReplicatedStorage/Packages`, `ServerScriptService/Packages`, or mapped from a top-level project folder such as `Packages`.

The visible proof is that a project containing `src/ReplicatedStorage/Packages/MyPackage/init.luau` can be exported with no package flags, re-imported, and still contain that package script as `src/ReplicatedStorage/Packages/MyPackage.luau`, the repository's leaf ModuleScript form. The same project exported with `--no-packages` should omit the package tree and report a skipped-package count. A project whose `rbxsync.json` explicitly sets `"packages": { "enabled": false }` should skip packages by default, while `--include-packages` should still force inclusion for compatibility and automation.

## Progress

- [x] (2026-05-14 16:05Z) Read `PLANS.md`, `ROADMAP-ISSUES.md`, existing ExecPlans, current `extract-place` CLI wiring in `rbxsync-cli/src/main.rs`, current export package filtering in `rbxsync-core/src/place_exporter.rs`, package config types in `rbxsync-core/src/types/project.rs`, package path detection in `rbxsync-core/src/types/wally.rs`, docs in `docs/cli/commands.md` and `docs/getting-started/configuration.md`, and current CLI integration tests in `rbxsync-cli/tests/extract_place.rs`.
- [x] (2026-05-14 16:05Z) Created this initial ExecPlan for P2 Package Default Semantics with milestones for policy, core export summary, CLI/docs semantics, and regression tests.
- [x] (2026-05-14 16:12Z) Implemented Milestone 1 by adding `PackageExportMode`, `PackageExportSummary`, config-derived effective package inclusion, package root include/skip counts, and core regressions for default include, config opt-out, and forced include override.
- [x] (2026-05-14 16:12Z) Updated current core and CLI callers to construct `PlaceExportOptions.package_mode` instead of the previous `include_packages` boolean; `rbxsync build` now forces `PackageExportMode::Include`, and `extract-place` now resolves package flags to `Auto`, `Include`, or `Skip`.
- [x] (2026-05-14 16:12Z) Ran `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core place_exporter`, and `mise exec -- cargo test -p rbxsync`; all passed.
- [x] (2026-05-14 16:34Z) Implemented Milestone 2 by adding `packages` to `extract-place --json`, adding a human `Packages:` summary line, updating package flag help text, documenting package export defaults and overrides in `docs/cli/commands.md`, and clarifying package configuration semantics in `docs/getting-started/configuration.md`.
- [x] (2026-05-14 16:34Z) Verified Milestone 2 with `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core place_exporter`, `mise exec -- cargo test -p rbxsync --test extract_place`, `mise exec -- cargo run -p rbxsync -- extract-place --help`, and `git diff --check`; all passed.
- [x] (2026-05-14 18:02Z) Implemented Milestone 3 by adding real-binary `extract_place` integration tests for default package inclusion and re-import, `--no-packages` skipping with summary diagnostics, and top-level `Packages` mapped through `treeMapping` to `ReplicatedStorage/Packages`.
- [x] (2026-05-14 18:02Z) Verified Milestone 3 with `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync --test extract_place`, `mise exec -- cargo test -p rbxsync-core place_exporter`, `mise exec -- cargo test -p rbxsync`, and `git diff --check`; all passed.
- [x] (2026-05-14 18:08Z) Completed Milestone 4 by running the final validation set, confirming docs build is environment-blocked because `docs/node_modules` is missing, and recording final outcomes and validation evidence in this plan.
- [x] (2026-05-14 18:08Z) Final validation passed: `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core place_exporter`, `mise exec -- cargo test -p rbxsync --test extract_place`, `mise exec -- cargo test -p rbxsync`, `mise exec -- cargo test --workspace`, and `git diff --check`.

## Surprises & Discoveries

- Observation: The current roadmap text says the CLI requires `--include-packages`, but the current command dispatcher already includes packages by default.
  Evidence: `rbxsync-cli/src/main.rs` resolves `let include_packages = include_packages || !no_packages;`, so no flags means `include_packages` is true.

- Observation: The current docs already omit `--include-packages` from the `extract-place` options table and document only `--no-packages`.
  Evidence: `docs/cli/commands.md` lists `--no-packages` as "Skip package folders" for `extract-place`, but does not list `--include-packages`.

- Observation: Core package filtering is currently a boolean and has no included/skipped package summary field.
  Evidence: `PlaceExportOptions` has `include_packages: bool`, `DomBuilder::insert_directory` emits `PlaceExportDiagnosticKind::SkippedPackage` when that bool is false and `is_package_path(dir_path)` is true, and `PlaceExportSummary` reports instances, scripts, metadata files, services, assets, and terrain, but not package counts.

- Observation: `rbxsync.json` already has package settings, but only some are relevant to export defaults.
  Evidence: `PackageConfig` in `rbxsync-core/src/types/project.rs` contains `enabled`, `excludeFromWatch`, `preserveOnExtract`, and `packagesFolder`. `excludeFromWatch` is a file-watcher setting and `preserveOnExtract` is an import/extraction writer setting, so neither should make `extract-place` skip package folders.

- Observation: Server-side Studio extraction has a broader package auto-detection path than the current local place exporter.
  Evidence: `rbxsync-server/src/lib.rs` reads a top-level `Packages` folder when package support is enabled or when that folder exists, and maps it to configured DataModel paths such as `ReplicatedStorage/Packages`. The local exporter instead discovers roots through `treeMapping` and `src/`, then filters directories using `is_package_path`.

- Observation: `build_dom_from_project` previously bypassed `apply_project_config`.
  Evidence: During Milestone 1, `export_place` called `apply_project_config` before `build_project_dom`, but `build_dom_from_project` called `build_project_dom` directly. Milestone 1 now applies project config in `build_dom_from_project` too, so config-derived package defaults and `treeMapping` behavior are consistent in both core entry points.

- Observation: Package summary counts are most accurate when collected during DOM construction.
  Evidence: Milestone 1 records `included_roots` and `skipped_roots` inside `DomBuilder::insert_directory`, the same location where package directories are either walked or skipped. This avoids a second filesystem scan that could disagree with service filtering or `treeMapping` discovery.

- Observation: The existing `extract-place` integration tests tolerate additive JSON fields.
  Evidence: After Milestone 2 added a top-level `packages` object to the JSON summary, `mise exec -- cargo test -p rbxsync --test extract_place` still passed all 7 existing tests.

- Observation: The command help now states the package overrides without changing flag names.
  Evidence: `mise exec -- cargo run -p rbxsync -- extract-place --help` lists `--include-packages` as forcing inclusion even if disabled by `rbxsync.json`, and `--no-packages` as skipping packages even when present or enabled by `rbxsync.json`.

- Observation: Re-importing a package directory whose source was `MyPackage/init.luau` writes the supported leaf ModuleScript form `MyPackage.luau`.
  Evidence: The first Milestone 3 integration test attempt looked for `src/ReplicatedStorage/Packages/MyPackage/init.luau` and failed. A manual round trip showed the importer wrote `src/ReplicatedStorage/Packages/MyPackage.luau` plus `MyPackage.rbxjson`, so the final tests assert the actual supported layout.

- Observation: Final workspace validation still emits pre-existing `rbxsync-mcp` dead-code warnings.
  Evidence: `mise exec -- cargo test --workspace` passed, while warning that `TestStartResponse`, `TestStopResponse`, `ConsoleMessage` fields, and several `RbxSyncClient` test-control methods in `rbxsync-mcp/src/tools/mod.rs` are unused.

- Observation: Documentation build validation could not be run in this local checkout without installing dependencies.
  Evidence: `docs/node_modules` does not exist, so `cd docs && npm run build` was skipped per this plan's recovery instructions rather than installing packages during Milestone 4.

## Decision Log

- Decision: Treat `extract-place` package behavior as an export policy, not as import package preservation.
  Rationale: `preserveOnExtract` says whether local packages should be restored while writing a project from Studio or a place file. It does not describe whether an already-present package tree should be serialized into a Roblox place artifact. Reusing it for export would surprise users by making export behavior depend on an import-only setting.
  Date/Author: 2026-05-14 / Codex

- Decision: Default package export mode is `Auto`: include package folders unless `rbxsync.json` explicitly disables package support with `"packages": { "enabled": false }`.
  Rationale: This satisfies the PRD requirement that packages under the exported tree are included by default, while still honoring the one package config field that clearly means "do not use package support for this project." If there is no package config, or if a package config exists and omits `enabled`, the existing default is enabled.
  Date/Author: 2026-05-14 / Codex

- Decision: Keep `--include-packages` and make it force inclusion.
  Rationale: The flag already exists and may be used by scripts. Under the new explicit policy it is useful as an override for projects that set `packages.enabled` false but still want a one-off export with packages.
  Date/Author: 2026-05-14 / Codex

- Decision: Keep `--no-packages` as the strongest CLI override.
  Rationale: A direct command-line skip should win over config and default behavior because it is the most local expression of user intent for that single export.
  Date/Author: 2026-05-14 / Codex

- Decision: Add package counts to the export summary instead of relying only on diagnostics.
  Rationale: The roadmap explicitly asks for summary counts. Diagnostics are warnings and only describe skipped packages; automation also needs to see that package roots were included during default exports.
  Date/Author: 2026-05-14 / Codex

- Decision: Replace the core `include_packages` option with `PackageExportMode`.
  Rationale: The boolean could not distinguish no CLI flag from explicit `--include-packages`, so it could not support config-sensitive `Auto` behavior. The enum makes the requested policy explicit and lets summaries report both requested mode and effective include behavior.
  Date/Author: 2026-05-14 / Codex

## Outcomes & Retrospective

This initial plan scopes P2 Package Default Semantics. The intended end state is a small policy cleanup, not a package system rewrite: default local exports include package folders, config can opt out, CLI flags are clear, summaries include package counts, and tests pin the behavior.

Milestone 1 is complete. `rbxsync-core/src/place_exporter.rs` now exposes `PackageExportMode` and `PackageExportSummary`, applies `rbxsync.json` package config when computing effective package inclusion, and records package root include/skip counts while the DOM is built. Core regressions cover `Auto` including packages by default, `Auto` respecting `"packages": { "enabled": false }`, and `Include` overriding that disabled config. `rbxsync-cli/src/main.rs` was updated only as needed to construct the new core option and keep existing callers compiling; user-facing docs and summary printing remain in Milestone 2.

Milestone 2 is complete. `rbxsync extract-place --json` now includes a `packages` object with the requested mode, effective include decision, included package root count, and skipped package root count. Human output prints the same information as a concise `Packages:` line. The CLI help text now explains the two package overrides, and the docs now describe default inclusion, config opt-out through `packages.enabled`, forced inclusion with `--include-packages`, forced skipping with `--no-packages`, and the distinction between export behavior and `preserveOnExtract`.

Milestone 3 is complete. `rbxsync-cli/tests/extract_place.rs` now proves package behavior through the real binary and through re-imported filesystem output. Default `extract-place` includes `src/ReplicatedStorage/Packages/MyPackage/init.luau`, reports `packages.mode == "auto"` and one included root, and re-imports the package as `src/ReplicatedStorage/Packages/MyPackage.luau`. `extract-place --no-packages` reports `mode == "skip"`, `effectiveInclude == false`, one skipped root, and a `skippedPackage` diagnostic summary, and the re-imported project does not contain the package module. A top-level `Packages/MyPackage/init.luau` mapped by `treeMapping` to `ReplicatedStorage/Packages` is included by default and re-imports as `src/ReplicatedStorage/Packages/MyPackage.luau`.

Milestone 4 is complete. The full package default semantics work is implemented and validated. The final behavior is that `extract-place` defaults to package mode `Auto`, includes package folders unless `rbxsync.json` explicitly disables package support, lets `--include-packages` force inclusion, lets `--no-packages` force skipping, reports package counts in JSON and human summaries, and documents the config/flag behavior. Focused, CLI, and workspace Rust validation all passed. The only validation not run was the docs build because local docs dependencies are absent.

## Context and Orientation

RbxSync converts Roblox projects between local files and Roblox place files. A Roblox place file has extension `.rbxl` for binary or `.rbxlx` for XML. The command involved in this plan is `rbxsync extract-place`, implemented in `rbxsync-cli/src/main.rs`, which reads a local project and delegates conversion to `rbxsync-core/src/place_exporter.rs`.

A Wally package is third-party Roblox code installed into a folder usually named `Packages`. In a Roblox DataModel, shared packages commonly appear at `ReplicatedStorage/Packages`. Server-only packages can appear at `ServerScriptService/Packages` or `ServerStorage/Packages`. In a filesystem project, packages might live directly under `src/ReplicatedStorage/Packages`, or a `rbxsync.json` `treeMapping` entry might map a top-level folder such as `Packages` to the DataModel path `ReplicatedStorage/Packages`.

The current export pipeline works like this. `Commands::ExtractPlace` in `rbxsync-cli/src/main.rs` parses flags. The command dispatcher resolves `--include-packages`, `--no-packages`, or no package flag into `PackageExportMode::Include`, `PackageExportMode::Skip`, or `PackageExportMode::Auto`. `cmd_extract_place` reads `rbxsync.json`, resolves the source directory and `treeMapping`, creates `PlaceExportOptions`, and calls `rbxsync_core::export_place`. In core, `PlaceExportOptions` stores `package_mode: PackageExportMode`. `DomBuilder::insert_directory` checks the effective package policy and emits a `SkippedPackage` diagnostic instead of walking the directory when packages should be skipped. The same builder also records included and skipped package root counts for the export summary.

`rbxsync.json` is represented by `ProjectConfig` in `rbxsync-core/src/types/project.rs`. Its optional `packages` field uses `PackageConfig`, whose `enabled` field defaults to true when the package section exists. This plan uses only `packages.enabled` for default export policy. The `excludeFromWatch` field belongs to live file watching, and `preserveOnExtract` belongs to project writing during import or Studio extraction, so they must not silently change `extract-place` export inclusion.

## Plan of Work

Milestone 1 makes the core export policy explicit. In `rbxsync-core/src/place_exporter.rs`, replace the public boolean `PlaceExportOptions.include_packages` with an enum such as `PackageExportMode` or add the enum alongside the boolean during a short compatibility step. The enum should have `Auto`, `Include`, and `Skip`. `Auto` means include package folders unless the loaded project config has `packages.enabled == false`. `Include` means include regardless of config. `Skip` means skip regardless of config. If keeping the public bool for compatibility is simpler, still add an internal enum after applying project config so the behavior is named and testable.

Milestone 1 also adds a package summary to `PlaceExportSummary`. A concrete shape is:

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PackageExportSummary {
        pub mode: PackageExportMode,
        pub effective_include: bool,
        pub included_roots: usize,
        pub skipped_roots: usize,
    }

The exact field names can follow the repository's JSON style, but the summary must tell users and automation whether the export effectively included packages, how many package roots were included, and how many were skipped. A package root should count a directory whose final path component is `Packages` or `ServerPackages`, case-insensitively. Do not count every nested file or package dependency as a separate root.

Milestone 1 should keep package path filtering in core. Add a small helper near the current `is_package_path` use, or in `rbxsync-core/src/types/wally.rs` if it is generally useful, to identify package root directories without changing the broader `is_package_path` behavior used by existing file-watcher code. The existing `is_package_path` function should continue to answer whether any path is inside a package tree.

Milestone 2 wires CLI semantics and docs. This is complete. In `rbxsync-cli/src/main.rs`, package flags are resolved into the new package export mode:

    --no-packages       => PackageExportMode::Skip
    --include-packages  => PackageExportMode::Include
    no package flag     => PackageExportMode::Auto

The clap attributes keep `--include-packages` and `--no-packages` mutually exclusive. Help text says `--include-packages` forces package folder inclusion and `--no-packages` skips package folders even when present or enabled by config. `print_export_summary` JSON includes `packages: summary.package_summary`. Human output includes a concise line with mode, effective behavior, included roots, and skipped roots.

Milestone 2 also updated `docs/cli/commands.md` and `docs/getting-started/configuration.md`. The command docs list both package flags and state the default in prose: no package flag includes package folders by default unless `rbxsync.json` explicitly sets `packages.enabled` to false. The configuration docs clarify that `preserveOnExtract` affects import and Studio extraction, not `extract-place`, and that `packages.enabled` is the config-level opt-out for default package export.

Milestone 3 adds tests. This is complete. In `rbxsync-core/src/place_exporter.rs`, focused unit tests build a temporary project with a package root and call `export_place` with the relevant modes. The core tests show `Auto` includes a package root without package config, `Auto` skips when `rbxsync.json` contains `"packages": { "enabled": false }`, and `Include` overrides that disabled config. Skip-mode behavior is covered at the real-binary layer where the CLI flag is exercised end to end.

Milestone 3 also added real-binary CLI coverage in `rbxsync-cli/tests/extract_place.rs`. One test creates `src/ReplicatedStorage/Packages/MyPackage/init.luau`, runs `rbxsync extract-place --path <project> --output <temp>/game.rbxl --force --json`, then runs `rbxsync import-place <temp>/game.rbxl --output <imported> --force --json`, and asserts that `imported/src/ReplicatedStorage/Packages/MyPackage.luau` exists. A second test runs the same export with `--no-packages`, re-imports, and asserts that the package script is absent and the export JSON reports a skipped package root. A third test covers a `treeMapping` entry:

    {
      "tree": "src",
      "treeMapping": {
        "ReplicatedStorage/Packages": "Packages"
      }
    }

With that config, a top-level `Packages/MyPackage/init.luau` folder exports by default and re-imports under `src/ReplicatedStorage/Packages/MyPackage.luau`.

Milestone 4 is validation and plan closeout. This is complete. Focused package-related tests, the broader local validation set, and the full workspace test suite passed. `Progress`, `Surprises & Discoveries`, `Outcomes & Retrospective`, and `Artifacts and Notes` have been updated with exact results.

## Concrete Steps

Work from the repository root:

    cd /Users/lexiviripaeff/Documents/LoganX/rbxsync

Before editing, inspect the current package-related code:

    rg -n "include_packages|include-packages|no-packages|SkippedPackage|PackageConfig|is_package_path" rbxsync-cli/src/main.rs rbxsync-core/src docs/cli/commands.md docs/getting-started/configuration.md
    sed -n '180,250p' rbxsync-cli/src/main.rs
    sed -n '740,790p' rbxsync-cli/src/main.rs
    sed -n '60,125p' rbxsync-core/src/place_exporter.rs
    sed -n '260,335p' rbxsync-core/src/types/project.rs
    sed -n '186,215p' rbxsync-core/src/types/wally.rs

For Milestone 1, add the package mode and summary types in `rbxsync-core/src/place_exporter.rs`. Update `PlaceExportOptions`, `apply_project_config`, `build_project_dom`, and `DomBuilder::insert_directory` so every package decision goes through one effective policy. Keep errors non-fatal for skipped packages, matching current behavior, but record summary counts. If a public API change requires updating callers, update the core tests' `options(project_dir)` helper and the CLI construction in `cmd_extract_place`.

For Milestone 2, `Commands::ExtractPlace` help text and the dispatcher in `rbxsync-cli/src/main.rs` have been updated. Package flag resolution now uses `resolve_package_export_mode`, and `print_export_summary` emits package summary data in both JSON and human output. The matching docs have been updated after the CLI behavior was pinned.

For Milestone 3, add fixtures directly in tests using `tempfile`, matching the style already used in `rbxsync-cli/tests/extract_place.rs`. Avoid Roblox Studio and network access. Use the built binary through `CARGO_BIN_EXE_rbxsync` and keep `RBXSYNC_VERSION_CHECK=1` set through the existing `command()` helper.

Milestone 3 is complete. The tests are named `extract_place_includes_packages_by_default_and_reimports_them`, `extract_place_no_packages_skips_packages_and_reports_summary`, and `extract_place_includes_tree_mapped_top_level_packages_by_default`.

For Milestone 4, run:

    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core place_exporter
    mise exec -- cargo test -p rbxsync --test extract_place
    mise exec -- cargo test -p rbxsync
    mise exec -- cargo test --workspace
    git diff --check

If the docs dependency tree is available locally, also run:

    cd docs
    npm run build

If `npm run build` fails because dependencies are not installed, record that as an environment-blocked validation note and do not install dependencies unless the user asks for it.

Milestone 4 is complete. The Rust validation commands listed above passed. The docs build command was not run because `docs/node_modules` is missing in this checkout.

## Validation and Acceptance

Acceptance is behavior-first. A default export of a project with `src/ReplicatedStorage/Packages/MyPackage/init.luau` must include the package in the generated place file. Re-importing that place file must recreate the package script under `src/ReplicatedStorage/Packages/MyPackage.luau`.

The skip path must be equally visible. Running the same export with `--no-packages` must omit the package script after re-import. The export JSON must include a package summary showing packages were effectively skipped and at least one package root was skipped. The command may also include the existing `SkippedPackage` diagnostic.

Config behavior must be pinned. With `rbxsync.json` containing `"packages": { "enabled": false }`, no package flag should skip package folders by default. With the same config and `--include-packages`, packages must be included. `preserveOnExtract` and `excludeFromWatch` must not change `extract-place` package inclusion.

Tree-mapped package behavior must be covered. A top-level `Packages` folder mapped by `treeMapping` to `ReplicatedStorage/Packages` must be included by default and re-import to the expected DataModel path. This proves the behavior is not limited to packages physically under `src/`.

Final validation should pass the focused core exporter tests, the real-binary `extract_place` integration suite, the full `rbxsync` package tests, formatting checks, and `git diff --check`. Before marking the plan complete, update this file with exact command outcomes.

Final validation passed. `mise exec -- cargo test --workspace` also passed, providing coverage beyond the minimum final acceptance. The behavior can be observed by running the three package tests in `rbxsync-cli/tests/extract_place.rs`.

## Idempotence and Recovery

This work is safe to retry. Tests should create temporary directories and should not depend on Roblox Studio, user home state, network access, or existing package installations. The export command must not delete package files from the source project. `--no-packages` should only skip reading package directories while constructing the exported DOM.

If changing `PlaceExportOptions` breaks unrelated callers, introduce a compatibility helper or constructor instead of broad refactoring. If package summary counting becomes ambiguous, prefer a conservative root count and document the exact definition in code comments and docs. If a test exposes that top-level package mapping does not currently work, fix only the root discovery or package policy needed for that behavior; do not broaden into Wally installation, package downloading, or package manifest generation.

If `mise` is unavailable in a local environment, run the same `cargo` commands directly and record the substitution. If workspace tests fail in unrelated crates, keep the focused validation evidence and record the unrelated failure with its exact message before asking for direction.

## Artifacts and Notes

The key code locations discovered while creating this plan are:

    rbxsync-cli/src/main.rs
      Commands::ExtractPlace defines package flags.
      The command dispatcher currently computes include_packages with include_packages || !no_packages.
      cmd_extract_place passes package behavior into PlaceExportOptions.
      print_export_summary owns extract-place JSON and human output.

    rbxsync-core/src/place_exporter.rs
      PlaceExportOptions currently stores include_packages: bool.
      DomBuilder::insert_directory skips package directories when include_packages is false.
      PlaceExportDiagnosticKind::SkippedPackage already exists.

    rbxsync-core/src/types/project.rs
      ProjectConfig.packages is Option<PackageConfig>.
      PackageConfig.enabled defaults to true.
      preserve_on_extract is import/extraction writer behavior, not export behavior.

    rbxsync-core/src/types/wally.rs
      is_package_path detects paths inside package directories for current skip behavior.

    docs/cli/commands.md
      extract-place currently documents --no-packages, assets flags, and local place export examples.

Milestone 1 validation:

    mise exec -- cargo fmt -- --check
      Passed.

    mise exec -- cargo test -p rbxsync-core place_exporter
      Passed: 14 place_exporter tests, including package_auto_includes_and_reports_package_root, package_auto_respects_disabled_project_config, and package_include_overrides_disabled_project_config.

    mise exec -- cargo test -p rbxsync
      Passed: 0 main unit tests, 7 extract_place integration tests, 5 import_place integration tests, and 5 publish_place integration tests.

Milestone 2 validation:

    mise exec -- cargo fmt -- --check
      Passed.

    mise exec -- cargo test -p rbxsync-core place_exporter
      Passed: 14 place_exporter tests.

    mise exec -- cargo test -p rbxsync --test extract_place
      Passed: 7 extract_place integration tests.

    mise exec -- cargo run -p rbxsync -- extract-place --help
      Passed and showed the updated --include-packages and --no-packages descriptions.

    git diff --check
      Passed.

Milestone 3 validation:

    mise exec -- cargo fmt -- --check
      Passed.

    mise exec -- cargo test -p rbxsync --test extract_place
      Passed: 10 extract_place integration tests, including the three package default/skip/treeMapping tests.

    mise exec -- cargo test -p rbxsync-core place_exporter
      Passed: 14 place_exporter tests.

    mise exec -- cargo test -p rbxsync
      Passed: 0 main unit tests, 10 extract_place integration tests, 5 import_place integration tests, and 5 publish_place integration tests.

    git diff --check
      Passed.

Milestone 4 final validation:

    mise exec -- cargo fmt -- --check
      Passed.

    mise exec -- cargo test -p rbxsync-core place_exporter
      Passed: 14 place_exporter tests.

    mise exec -- cargo test -p rbxsync --test extract_place
      Passed: 10 extract_place integration tests.

    mise exec -- cargo test -p rbxsync
      Passed: 0 main unit tests, 10 extract_place integration tests, 5 import_place integration tests, and 5 publish_place integration tests.

    mise exec -- cargo test --workspace
      Passed: CLI, core, MCP, server, harness integration tests, and doc-tests. The command emitted existing non-failing dead-code warnings from `rbxsync-mcp/src/tools/mod.rs`.

    git diff --check
      Passed.

    cd docs && npm run build
      Not run: `docs/node_modules` is missing in this checkout, and this plan says not to install dependencies unless requested.

Revision note: 2026-05-14 / Codex created the initial package default semantics ExecPlan from `ROADMAP-ISSUES.md` after verifying the current implementation and docs. The plan intentionally treats the roadmap statement about `--include-packages` as stale for the current tree and focuses implementation on explicit policy, config semantics, package summary counts, and regression coverage.

Revision note: 2026-05-14 / Codex completed Milestone 1 and updated this plan to record the new core package policy API, the caller adaptation, package summary regressions, and validation evidence.

Revision note: 2026-05-14 / Codex completed Milestone 2 and updated this plan to record the user-facing package summary output, command help changes, docs updates, and focused validation evidence.

Revision note: 2026-05-14 / Codex completed Milestone 3 and updated this plan to record the real-binary package tests, the re-imported ModuleScript filename discovery, and validation evidence.

Revision note: 2026-05-14 / Codex completed Milestone 4 and updated this plan to record final validation, the docs build environment limitation, and the completed outcome.

## Interfaces and Dependencies

No new external crates are required. Use the existing Rust crates and test dependencies already present in the repository.

The intended core interface at completion should be equivalent to:

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum PackageExportMode {
        Auto,
        Include,
        Skip,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PackageExportSummary {
        pub mode: PackageExportMode,
        pub effective_include: bool,
        pub included_roots: usize,
        pub skipped_roots: usize,
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
        pub package_mode: PackageExportMode,
        pub tree_mapping: HashMap<String, String>,
        pub asset_mode: AssetMode,
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
        pub package_summary: PackageExportSummary,
        pub asset_summary: Option<AssetSummary>,
        pub terrain_summary: Option<TerrainSummary>,
    }

If retaining `include_packages: bool` temporarily is less disruptive, the implementation may keep it internal for one milestone, but the final user-visible behavior and summary must still match this plan.
