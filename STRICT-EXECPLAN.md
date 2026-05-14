# Implement P2 Import Strict Mode

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` at the repository root. If this document is revised during implementation, keep it self-contained and update every affected section before stopping work.

## Purpose / Big Picture

After this change, a user can run `rbxsync import-place --strict` to treat importer warnings as hard failures. This is useful in CI because a place file that imports with missing services, unsupported properties, missing script source, or terrain limitations should not silently produce a partial project when deterministic validation is required.

The visible proof is a pair of imports against the same warning-producing place file. Without `--strict`, `rbxsync import-place --services Workspace,MissingService --dry-run --json` succeeds and reports a `missingService` diagnostic. With `--strict`, the command exits non-zero, prints a concise strict-mode error, does not write `src/`, and still lets JSON callers see `"strict": true`, `"diagnosticCount": 1`, and `"diagnosticSummary": { "missingService": 1 }`.

## Progress

- [x] (2026-05-14 18:21Z) Read `PLANS.md`, the `P2: Import Strict Mode` section in `ROADMAP-ISSUES.md`, current `import-place` CLI wiring in `rbxsync-cli/src/main.rs`, importer diagnostics in `rbxsync-core/src/place_importer.rs`, current CLI import tests in `rbxsync-cli/tests/import_place.rs`, current import docs in `docs/cli/commands.md`, and the existing `extract-place` strict-mode behavior for parity.
- [x] (2026-05-14 18:21Z) Created this initial ExecPlan for P2 Import Strict Mode with milestones for CLI/core option flow, strict failure semantics, JSON summaries, tests, docs, and final validation.
- [x] (2026-05-14 18:25Z) Completed Milestone 1 by adding the `--strict` CLI flag to `import-place`, threading the strict setting through `cmd_import_place`, adding `"strict"` to import-place JSON summaries, and printing `Strict: enabled` in human output only when the flag is set.
- [x] (2026-05-14 18:25Z) Verified Milestone 1 with `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync --test import_place`, `mise exec -- cargo run -p rbxsync -- import-place --help`, and `git diff --check`; all passed.
- [x] (2026-05-14 18:28Z) Completed Milestone 2 by adding a strict diagnostic gate immediately after import diagnostics and dry-run summaries are computed, before dry-run success returns and before any non-dry-run filesystem writes.
- [x] (2026-05-14 18:28Z) Verified Milestone 2 with `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync --test import_place`, two manual `target/debug/rbxsync import-place --strict --json` smokes for dry-run and non-dry-run missing-service diagnostics, and `git diff --check`; all passed.
- [x] (2026-05-14 18:43Z) Completed Milestone 3 by adding real-binary CLI regressions for non-strict missing-service diagnostics, strict dry-run failure JSON, and strict non-dry-run pre-write safety in `rbxsync-cli/tests/import_place.rs`.
- [x] (2026-05-14 18:43Z) Completed Milestone 4 by documenting `import-place --strict` in `docs/cli/commands.md` and adding `--dry-run --strict --json` CI/script examples to `INSTALL.md`.
- [x] (2026-05-14 18:43Z) Completed Milestone 5 final validation with focused importer tests, full `rbxsync` crate tests, full workspace tests, formatting, and whitespace checks; all passed.

## Surprises & Discoveries

- Observation: `import-place` already has structured diagnostics and JSON diagnostic summaries, but no strict flag.
  Evidence: `rbxsync-cli/src/main.rs::print_import_summary` emits `diagnostics`, `diagnosticCount`, and `diagnosticSummary`; `Commands::ImportPlace` currently exposes `--dry-run`, `--json`, `--quiet`, `--include-assets`, and `--no-assets`, but not `--strict`.

- Observation: The importer diagnostics are collected in core but are currently non-fatal.
  Evidence: `rbxsync-core/src/place_importer.rs::PlaceImportResult` contains `diagnostics: Vec<ImportDiagnostic>`, and `import_dom` returns a result even after pushing `MissingService`, `MissingScriptSource`, or `UnsupportedTerrainVoxelData`.

- Observation: `extract-place` already implements the intended strict-mode precedent.
  Evidence: `rbxsync-core/src/place_exporter.rs::export_place` checks `options.strict && !diagnostics.is_empty()` and bails with a strict-mode error; `docs/cli/commands.md` documents `extract-place --strict` as "Fail if diagnostics are produced".

- Observation: The easiest existing warning-producing CLI fixture is a missing requested service.
  Evidence: `rbxsync-cli/tests/import_place.rs::import_place_reports_missing_requested_service` imports with `--services Workspace,MissingService --dry-run --json`, succeeds today, and asserts `diagnosticSummary.missingService == 1`.

- Observation: Milestone 1 can expose `--strict` without changing import success or failure behavior.
  Evidence: After adding the flag and summary field, `mise exec -- cargo test -p rbxsync --test import_place` still passed all 5 existing import-place integration tests, including the non-strict missing-service diagnostic test.

- Observation: The strict gate can run before both dry-run success handling and write-path overwrite handling.
  Evidence: The Milestone 2 smoke with `--dry-run --strict --json` exited with status 1 and parseable JSON containing `"success": false`, `"strict": true`, `"dryRun": true`, and `diagnosticSummary.missingService == 1`; the output project directory was not created.

- Observation: The non-dry-run strict path now fails before project files are written.
  Evidence: The Milestone 2 smoke without `--dry-run` exited with status 1 and parseable JSON containing `"success": false`, `"strict": true`, `"dryRun": false`, and `diagnosticSummary.missingService == 1`; neither `imported/src` nor `imported/rbxsync.json` existed afterward.

- Observation: The new strict tests can use the same missing-service fixture as the existing warning test.
  Evidence: `rbxsync-cli/tests/import_place.rs` now has 7 integration tests. The two new strict tests both build a temporary `.rbxlx`, request `Workspace,MissingService`, parse failure JSON from stdout, and assert no output files are written where appropriate.

- Observation: Final workspace validation still emits pre-existing `rbxsync-mcp` dead-code warnings.
  Evidence: `mise exec -- cargo test --workspace` passed while warning that `TestStartResponse`, `TestStopResponse`, `ConsoleMessage` fields, and several `RbxSyncClient` test-control methods in `rbxsync-mcp/src/tools/mod.rs` are unused.

## Decision Log

- Decision: Implement strict-mode failure in the CLI after `import_place_file` returns diagnostics, not by making every diagnostic push in core return an error.
  Rationale: The importer already returns useful partial conversion data and structured diagnostics. A CLI-level gate can fail before writing files while preserving core's ability to inspect diagnostics in tests, future callers, and dry-run summaries. This matches the current architecture where `cmd_import_place` decides whether to write files.
  Date/Author: 2026-05-14 / Codex

- Decision: Add `strict` to `PlaceImportOptions` only if it materially simplifies shared behavior; otherwise keep it as a `cmd_import_place` parameter.
  Rationale: The roadmap asks for `import-place --strict`, a command behavior. The existing core import API has no writer side effects and no JSON output responsibilities, so core does not need to own the policy unless a test or caller benefits from the option.
  Date/Author: 2026-05-14 / Codex

- Decision: JSON output under strict failure should remain parseable and include the strict setting and diagnostic summary.
  Rationale: The roadmap explicitly requires `--json` to include strict setting and diagnostic summary. CI callers need machine-readable failure details more than human prose.
  Date/Author: 2026-05-14 / Codex

## Outcomes & Retrospective

This initial plan scopes `P2: Import Strict Mode`. The intended end state is small and user-visible: `import-place` gains `--strict`, default imports continue warning and succeeding, strict imports fail on diagnostics before writing files, JSON includes a `strict` boolean and diagnostic summary, and docs show how to use strict mode in CI.

Milestone 1 is complete. `rbxsync-cli/src/main.rs` now exposes `--strict` on `import-place`, threads the boolean through `cmd_import_place`, includes `"strict"` in JSON summaries, and prints a human `Strict: enabled` line only when the flag is present. This milestone intentionally does not make diagnostics fatal yet; that remains the Milestone 2 behavior.

Milestone 2 is complete. `cmd_import_place` now checks `strict && !import_result.diagnostics.is_empty()` after parsing the place and computing dry-run summaries, but before the dry-run success branch and before any non-dry-run filesystem writes. In JSON mode, the failure path prints the normal import summary shape with `"success": false`; in human or quiet mode, it returns a strict-mode error through the normal error path. Formal regression tests remain Milestone 3.

Milestone 3 is complete. The import-place integration suite now proves the strict behavior through the real `rbxsync` binary. The existing non-strict missing-service test asserts `"strict": false`, while the strict dry-run and strict non-dry-run tests assert non-zero exit status, parseable JSON with `"success": false`, `"strict": true`, the `missingService` diagnostic summary, strict-mode stderr, and no unintended project writes.

Milestone 4 is complete. `docs/cli/commands.md` lists `--strict` for `import-place`, explains that diagnostics are warnings by default and failures under strict mode, and shows a CI-friendly `--dry-run --strict --json` command. `INSTALL.md` now includes the same strict validation command in useful import options and the scripts/CI JSON section.

Milestone 5 is complete. The full import strict-mode plan is implemented and validated. The final behavior is that default imports still succeed with diagnostics, `--strict` turns diagnostics into a failing validation result before writes, JSON callers receive parseable success or failure summaries with `strict` and diagnostic fields, and docs describe the deterministic validation workflow.

## Context and Orientation

RbxSync converts Roblox place files into editable project files. A Roblox place file has extension `.rbxl` for binary or `.rbxlx` for XML. The local import command is `rbxsync import-place`, implemented in `rbxsync-cli/src/main.rs` by the `Commands::ImportPlace` clap enum variant and the `cmd_import_place` function.

The importer itself lives in `rbxsync-core/src/place_importer.rs`. The function `import_place_file(options: PlaceImportOptions) -> anyhow::Result<PlaceImportResult>` opens the input file, detects whether it is binary `.rbxl` or XML `.rbxlx`, parses it into a Roblox DOM, and converts selected services into serialized instance JSON. A DOM is the in-memory tree of Roblox instances loaded from the place file. A service is a top-level Roblox container such as `Workspace`, `Lighting`, or `ServerScriptService`.

Diagnostics are structured warnings. They describe information the importer could not fully preserve or validate, but they do not currently stop the import. `ImportDiagnosticKind` currently includes `UnsupportedProperty`, `UnsupportedAttribute`, `MissingService`, `MissingScriptSource`, and `UnsupportedTerrainVoxelData`. For example, requesting `--services Workspace,MissingService` against a place that has only `Workspace` records a `MissingService` diagnostic at path `game`.

The `cmd_import_place` flow is currently:

    Resolve the input path and output project directory.
    Read any existing `rbxsync.json`.
    Parse service filters and print human progress unless JSON or quiet output is requested.
    Call `import_place_file(PlaceImportOptions { input_path, services, include_terrain })`.
    Compute imported service names, script counts, tooling files, optional asset summaries, and optional terrain summaries.
    If `--dry-run`, print an import summary and return without writing.
    If not dry-run, check `src/` overwrite safety, create config/tooling/assets/terrain as needed, write serialized instances, and print an import summary.

`print_import_summary` already owns both human and JSON summary output. In JSON mode it prints a top-level object containing `"success": true`, `"dryRun"`, `"input"`, `"output"`, `"format"`, counts, `"diagnostics"`, `"diagnosticCount"`, `"diagnosticSummary"`, tooling, package, asset, and terrain fields. The strict-mode work should extend this existing summary rather than inventing a second JSON schema.

`extract-place` provides the parity target. It has a `--strict` flag in `Commands::ExtractPlace`, a `strict: bool` field in `PlaceExportOptions`, JSON output with `"strict": strict`, and core exporter behavior that fails if diagnostics exist. `import-place` does not need to copy every implementation detail, but the user-facing rule should match: diagnostics are warnings by default and failures under strict mode.

## Plan of Work

Milestone 1 adds the strict flag and summary shape without changing default behavior. In `rbxsync-cli/src/main.rs`, add `strict: bool` to `Commands::ImportPlace` with help text such as "Fail if diagnostics are produced". Thread it through the command dispatcher into `cmd_import_place`. Add a `strict: bool` parameter near `dry_run` and `json_output` so the function has all output policy in one place. Update every call to `print_import_summary` to pass `strict`, and update `print_import_summary` so JSON includes `"strict": strict`. Human output should print `Strict: enabled` only when strict is true, or a concise equivalent, so normal output does not become noisy.

Milestone 1 must preserve all existing non-strict behavior. Running the existing `import_place_reports_missing_requested_service` test without `--strict` should still succeed and should still report the diagnostic summary. Existing callers of `import_place_file` should not need to change unless the implementation deliberately adds `strict` to `PlaceImportOptions`.

Milestone 2 implements the strict failure gate. After `import_place_file` returns and after the dry-run asset or terrain summaries have been computed, check whether `strict` is true and `import_result.diagnostics` is non-empty. This check must happen before any non-dry-run filesystem writes, especially before deleting or backing up `src/`, writing `rbxsync.json`, extracting assets, writing terrain blobs, or calling `write_serialized_instances`.

Milestone 2 is complete. The implemented gate lives in `rbxsync-cli/src/main.rs::cmd_import_place` immediately before the existing `if dry_run` branch. It calls `print_import_summary` with `success: false` only when `--json` is set, then bails with `Import failed in strict mode with N diagnostic(s): <first diagnostic message>`. This preserves JSON-only stdout for automation and avoids printing a human warning summary on strict failure.

In human output mode, strict failure should return an error like:

    Import failed in strict mode with 1 diagnostic(s): Requested service 'MissingService' was not found in the place

In JSON output mode, strict failure should print a parseable summary object before returning an error or exiting non-zero. The summary should include:

    "success": false
    "strict": true
    "dryRun": true or false
    "diagnostics": [...]
    "diagnosticCount": 1
    "diagnosticSummary": { "missingService": 1 }

The simplest implementation is to add a `success: bool` parameter to `print_import_summary`, default all successful calls to true, and call it with false before `bail!` in the strict failure path. If adding a success parameter causes too much churn, add a small `print_import_strict_failure_summary` helper that reuses `diagnostic_summary` and matches the existing JSON field names. Do not print both JSON and a human error to stdout; with `--json`, stdout should remain JSON-only and any error text should go to stderr through the normal error path.

Milestone 2 should apply to dry runs too. A strict dry-run is a validation mode: it parses and summarizes, then fails if diagnostics exist. It should not write files. This gives CI a safe command such as:

    rbxsync import-place ./Game.rbxl --output ./GameProject --dry-run --strict --json

Milestone 3 adds tests. Extend `rbxsync-cli/tests/import_place.rs` because this is command behavior and the real-binary pattern already exists there. Reuse the `command()` helper, which sets `RBXSYNC_VERSION_CHECK=1`, and the existing fixture pattern that builds a temporary place with `build_place`.

Milestone 3 is complete. The tests added are `import_place_strict_dry_run_fails_on_diagnostics_with_json_summary` and `import_place_strict_fails_before_writing_project_files`. The existing `import_place_reports_missing_requested_service` test now also asserts `"strict": false`, preserving the default warning behavior.

Add a non-strict regression test or extend the existing `import_place_reports_missing_requested_service` only if needed. The key new test should run:

    rbxsync import-place <place> --output <imported> --services Workspace,MissingService --dry-run --strict --json

It should assert the command exits non-zero, stdout is parseable JSON, `summary["success"] == false`, `summary["strict"] == true`, `summary["dryRun"] == true`, `summary["diagnosticCount"] == 1`, `summary["diagnosticSummary"]["missingService"] == 1`, and the output project directory does not exist.

Add a second test for non-dry-run strict safety. Run:

    rbxsync import-place <place> --output <imported> --services Workspace,MissingService --strict --json

It should assert non-zero exit, parseable JSON with the same strict diagnostic fields, and no `imported/src` directory. This proves strict mode fails before writes. If `cmd_import_place` creates the output directory only after the strict gate, assert `!imported.exists()`. If it intentionally creates the project directory before the gate in a later implementation, the acceptance criterion is that no project files, `src/`, assets, terrain blobs, or config are written.

If a core-level strict option is introduced, add a focused unit test in `rbxsync-core/src/place_importer.rs` that constructs a DOM with a missing service or missing script source and confirms the strict helper reports an error. Do not replace the real-binary CLI tests with only unit tests, because the roadmap acceptance includes CLI flags and JSON output.

Milestone 4 updates documentation and examples. In `docs/cli/commands.md`, add `--strict` to the `import-place` options table with default `false` and description "Fail if diagnostics are produced". Add a short paragraph explaining that diagnostics remain warnings by default, and strict mode turns them into failures for deterministic validation. Add a CI-oriented example using `--dry-run --strict --json`.

Milestone 4 is complete. `docs/cli/commands.md` and `INSTALL.md` now document strict import validation. No README change was needed because `INSTALL.md` already owns the script/CI import examples referenced by this plan.

Also update `INSTALL.md` or `README.md` only where import-place examples already discuss automation or CI. Keep docs concise; do not broaden the feature into Open Cloud import, package behavior, asset downloading, or Studio automation.

Milestone 5 is final validation and closeout. Run focused checks first, then broader Rust validation. Update `Progress`, `Surprises & Discoveries`, `Outcomes & Retrospective`, and `Artifacts and Notes` with exact command outcomes before marking the plan complete.

Milestone 5 is complete. The validation commands listed below all passed, and this plan has been updated with final outcomes and evidence.

## Concrete Steps

Work from the repository root:

    cd /Users/lexiviripaeff/Documents/LoganX/rbxsync

Before editing, inspect the current import and strict-export paths:

    rg -n "ImportPlace|cmd_import_place|print_import_summary|diagnostic_summary|PlaceImportOptions|ImportDiagnosticKind" rbxsync-cli/src/main.rs rbxsync-core/src/place_importer.rs rbxsync-cli/tests/import_place.rs docs/cli/commands.md
    rg -n "strict|Export failed in strict mode|print_export_summary" rbxsync-cli/src/main.rs rbxsync-core/src/place_exporter.rs docs/cli/commands.md

For Milestone 1, edit `rbxsync-cli/src/main.rs`. Add the clap flag to `Commands::ImportPlace`, include `strict` in the dispatcher pattern, pass it to `cmd_import_place`, and include it in every import summary call. Prefer the existing naming style: `json_output`, `dry_run`, and `quiet` are booleans passed explicitly into the command function.

For Milestone 2, add a helper near `print_import_summary` if needed:

    fn first_import_diagnostic_message(diagnostics: &[rbxsync_core::ImportDiagnostic]) -> String

or simply use `import_result.diagnostics[0].message.clone()` after checking the vector is non-empty. The error message should include the total diagnostic count and the first diagnostic message. Keep the exact text stable enough for tests to assert `"strict mode"` rather than the full sentence.

For Milestone 3, edit `rbxsync-cli/tests/import_place.rs`. Add new tests near `import_place_reports_missing_requested_service`, because they use the same missing-service diagnostic. Use `serde_json::from_slice(&output.stdout)` to verify JSON remains clean in failure mode. Use `String::from_utf8_lossy(&output.stderr)` only for assertion messages and optional checks that stderr contains `strict mode`.

For Milestone 4, edit `docs/cli/commands.md` first. If CI examples exist in `INSTALL.md` or `README.md`, add one strict import validation example there too. Keep examples local-file focused:

    rbxsync import-place ./Game.rbxl --output ./GameProject --dry-run --strict --json

For Milestone 5, run:

    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync --test import_place
    mise exec -- cargo test -p rbxsync-core place_importer
    mise exec -- cargo test -p rbxsync
    mise exec -- cargo test --workspace
    git diff --check

If `mise` is unavailable, run the equivalent `cargo` commands directly and record that substitution in this plan.

## Validation and Acceptance

Default behavior must remain unchanged. A warning-producing import without `--strict` must exit successfully, must print warnings in human output or diagnostics in JSON output, and must continue to write files when not a dry run and when overwrite rules allow it.

Strict behavior must be visible. A warning-producing import with `--strict` must exit non-zero. The failure must happen for both dry-run and non-dry-run invocations. Non-dry-run strict failure must not write `src/`, generated tooling files, asset manifests, terrain blobs, or serialized instance files.

JSON behavior must meet the roadmap. Every `import-place --json` summary should include `"strict": false` or `"strict": true`. Under strict failure with `--json`, stdout must be parseable JSON and must include `"success": false`, `"strict": true`, `"diagnosticCount"`, and `"diagnosticSummary"`. The process still exits non-zero so shell scripts and CI jobs fail correctly.

Docs must show deterministic validation. `docs/cli/commands.md` must list `--strict` for `import-place` and include an example that combines `--dry-run`, `--strict`, and `--json`.

Final validation should pass the import-place integration suite, the core importer tests, the full `rbxsync` package tests, formatting checks, workspace tests, and `git diff --check`. If workspace tests fail for unrelated reasons, keep the focused import validation evidence and record the unrelated failure exactly before asking for direction.

## Idempotence and Recovery

This work is safe to retry. Tests should create temporary directories and should not depend on Roblox Studio, user home state, network access, or existing package installations. The strict failure path should run before writes, so rerunning strict tests should not require cleanup beyond `tempfile` directory deletion.

If strict JSON failure accidentally prints human progress before JSON, check the existing `json_output || quiet` suppression behavior at the top of `cmd_import_place`. The command should not print "Importing place" when `--json` is set.

If adding `success: bool` to `print_import_summary` creates excessive call-site churn, use a separate strict-failure JSON helper for Milestone 2 and consider unifying the summary shape later. The acceptance requirement is the emitted JSON shape and exit status, not a particular helper design.

If a diagnostic fixture based on missing services becomes brittle, use the core missing-script-source diagnostic instead. Build a place with a `Script` instance that has no `Source` property and import it with `--strict`. Missing service is preferred because an existing CLI test already proves it works.

If docs validation requires Node dependencies that are not installed, do not install packages unless the user asks. Record the docs build as environment-blocked and rely on Markdown diff review plus Rust validation.

## Artifacts and Notes

The key code locations discovered while creating this plan are:

    rbxsync-cli/src/main.rs
      Commands::ImportPlace defines the current import-place flags.
      cmd_import_place calls import_place_file, then decides dry-run versus write behavior.
      print_import_summary owns import-place JSON and human summaries.
      diagnostic_summary groups ImportDiagnosticKind values into camelCase JSON keys.

    rbxsync-core/src/place_importer.rs
      PlaceImportOptions currently stores input_path, services, and include_terrain.
      ImportDiagnosticKind defines unsupportedProperty, unsupportedAttribute, missingService, missingScriptSource, and unsupportedTerrainVoxelData diagnostics.
      PlaceImportResult returns instances, diagnostics, format, and optional terrain extraction.
      record_missing_services is the simplest deterministic warning source for CLI tests.

    rbxsync-cli/tests/import_place.rs
      command() invokes the real built rbxsync binary and sets RBXSYNC_VERSION_CHECK=1.
      write_fixture_project and build_place create local place fixtures without Roblox Studio.
      import_place_reports_missing_requested_service already asserts the non-strict missingService warning path.

    docs/cli/commands.md
      import-place currently documents dry-run, JSON, terrain, assets, tooling, and overwrite flags, but not strict mode.
      extract-place already documents --strict as the parity target.

Expected JSON shape for successful non-strict import after Milestone 1:

    {
      "success": true,
      "strict": false,
      "dryRun": true,
      "diagnosticCount": 1,
      "diagnosticSummary": {
        "missingService": 1
      }
    }

Milestone 1 validation:

    mise exec -- cargo fmt -- --check
      Passed.

    mise exec -- cargo test -p rbxsync --test import_place
      Passed: 5 import_place integration tests.

    mise exec -- cargo run -p rbxsync -- import-place --help
      Passed and showed `--strict` with the description "Fail if diagnostics are produced".

    git diff --check
      Passed.

Milestone 2 validation:

    mise exec -- cargo fmt -- --check
      Passed after applying rustfmt's preferred multiline formatting for the new `bail!`.

    mise exec -- cargo test -p rbxsync --test import_place
      Passed: 5 import_place integration tests.

    target/debug/rbxsync import-place <temp>/game.rbxlx --output <temp>/imported --services Workspace,MissingService --dry-run --strict --json
      Passed as a manual smoke: process exited with status 1, stdout parsed as JSON with success=false, strict=true, dryRun=true, and missingService=1, stderr began with the strict-mode error, and the output project directory was not created.

    target/debug/rbxsync import-place <temp>/game.rbxlx --output <temp>/imported --services Workspace,MissingService --strict --json
      Passed as a manual smoke: process exited with status 1, stdout parsed as JSON with success=false, strict=true, dryRun=false, and missingService=1, stderr began with the strict-mode error, and no `imported/src` or `imported/rbxsync.json` was written.

    git diff --check
      Passed.

Milestone 3 validation:

    mise exec -- cargo fmt -- --check
      Passed.

    mise exec -- cargo test -p rbxsync --test import_place
      Passed: 7 import_place integration tests, including the two strict-mode failure regressions.

    mise exec -- cargo test -p rbxsync-core place_importer
      Passed: 8 place_importer tests.

Milestone 4 validation:

    Documentation was updated in `docs/cli/commands.md` and `INSTALL.md`. The strict import command reference now lists `--strict`, and the install guide includes `rbxsync import-place ./Game.rbxl --output ./GameProject --dry-run --strict --json` for CI/script validation.

Milestone 5 final validation:

    mise exec -- cargo fmt -- --check
      Passed.

    mise exec -- cargo test -p rbxsync --test import_place
      Passed: 7 import_place integration tests.

    mise exec -- cargo test -p rbxsync-core place_importer
      Passed: 8 place_importer tests.

    mise exec -- cargo test -p rbxsync
      Passed: 0 main unit tests, 10 extract_place integration tests, 7 import_place integration tests, and 5 publish_place integration tests.

    mise exec -- cargo test --workspace
      Passed: CLI, core, MCP, server, harness integration tests, and doc-tests. The command emitted existing non-failing dead-code warnings from `rbxsync-mcp/src/tools/mod.rs`.

    git diff --check
      Passed.

Expected JSON shape for strict failure after Milestone 2:

    {
      "success": false,
      "strict": true,
      "dryRun": true,
      "diagnosticCount": 1,
      "diagnosticSummary": {
        "missingService": 1
      }
    }

Revision note: 2026-05-14 / Codex created the initial import strict-mode ExecPlan from `ROADMAP-ISSUES.md` after verifying current importer diagnostics, import-place CLI output, real-binary tests, docs, and extract-place strict-mode precedent. The plan intentionally keeps the implementation narrow: warnings stay warnings by default, `--strict` fails before writes, and JSON becomes suitable for CI validation.

Revision note: 2026-05-14 / Codex completed Milestone 1 and updated this plan to record the new CLI flag, summary plumbing, help output verification, and focused import-place validation. Strict diagnostic failure remains explicitly deferred to Milestone 2.

Revision note: 2026-05-14 / Codex completed Milestone 2 and updated this plan to record the strict diagnostic gate, parseable failure JSON, pre-write safety proof, focused validation, and manual smoke evidence. Formal CLI regression tests and docs remain deferred to Milestones 3 and 4.

Revision note: 2026-05-14 / Codex completed the rest of the plan by adding strict-mode CLI regression tests, documenting strict import validation in command and install docs, running final focused and workspace validation, and recording the completed outcomes.

## Interfaces and Dependencies

No new external crates are required. Use the existing Rust crates, clap flag parsing, serde JSON output, anyhow error handling, and test dependencies already present in the repository.

The intended CLI surface at completion is:

    rbxsync import-place <INPUT> [OPTIONS] --strict

The intended `Commands::ImportPlace` addition in `rbxsync-cli/src/main.rs` is:

    /// Fail if diagnostics are produced
    #[arg(long)]
    strict: bool,

The intended command function shape is equivalent to:

    async fn cmd_import_place(
        input: PathBuf,
        output: Option<PathBuf>,
        name: Option<String>,
        services: Option<Vec<String>>,
        terrain: bool,
        force: bool,
        backup_existing_src: bool,
        tooling_override: Option<bool>,
        dry_run: bool,
        strict: bool,
        json_output: bool,
        quiet: bool,
        asset_mode: AssetMode,
    ) -> Result<()>

The intended JSON summary should include at least these strict-related fields for both success and strict failure:

    {
      "success": true or false,
      "strict": true or false,
      "diagnostics": [...],
      "diagnosticCount": 0,
      "diagnosticSummary": {}
    }

If an implementation adds `strict` to `rbxsync-core/src/place_importer.rs::PlaceImportOptions`, update all construction sites in core tests and CLI code. The final behavior should still fail before writing project files, because filesystem writes are owned by `cmd_import_place`, not by `import_place_file`.
