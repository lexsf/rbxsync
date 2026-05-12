# Implement Publish Place Command

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This plan follows `PLANS.md` at the repository root. If this document is revised during implementation, keep it self-contained and update every affected section before stopping work.

## Purpose / Big Picture

After this change, a user who already has a local Roblox place artifact can publish it to an existing Roblox experience from the command line without opening Roblox Studio. The new workflow will be a separate `rbxsync publish-place` command. It will consume a `.rbxl` or `.rbxlx` file, require an explicit Universe ID and Place ID, authenticate with a Roblox Open Cloud API key, and upload the artifact to Roblox's place publishing API.

The visible proof of success is that a dry run validates the local artifact and command inputs without uploading, and a real publish sends the place file to Roblox and returns the created place version number. This plan deliberately does not extend `extract-place` with `--publish`; `extract-place` remains the local project-to-artifact command, while `publish-place` is the networked artifact-to-Roblox command.

## Progress

- [x] (2026-05-11 00:00Z) Created this initial ExecPlan from `ROADMAP-ISSUES.md`, `PLANS.md`, the current `extract-place` and `import-place` CLI shape in `rbxsync-cli/src/main.rs`, and the current Roblox Open Cloud place publishing documentation.
- [x] (2026-05-12 00:50Z) Implemented Milestone 1 by adding `rbxsync-core/src/place_publisher.rs`, exporting the publisher API from `rbxsync-core/src/lib.rs`, adding `reqwest` to `rbxsync-core/Cargo.toml`, and covering format detection, content types, endpoint construction, dry-run no-upload behavior, success parsing, HTTP error mapping, and network error redaction with offline unit tests.
- [x] (2026-05-12 00:50Z) Ran `mise exec -- cargo fmt` and `mise exec -- cargo test -p rbxsync-core place_publisher`; all 6 focused publisher tests passed.
- [x] (2026-05-12 00:57Z) Implemented Milestone 2 by adding `Commands::PublishPlace`, command dispatch, quiet logging suppression, API-key resolution from `--api-key` or `ROBLOX_OPEN_CLOUD_API_KEY`, `--version-type` parsing, non-dry-run confirmation enforcement, and human/JSON publish summaries that never print the API key.
- [x] (2026-05-12 00:57Z) Verified `mise exec -- cargo run -p rbxsync -- publish-place --help`, direct-binary dry-run JSON output, non-dry-run confirmation failure without `--yes`, `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core place_publisher`, `mise exec -- cargo test -p rbxsync`, and `git diff --check`; all passed.
- [x] (2026-05-12 01:03Z) Implemented Milestone 3 by hardening the production Roblox Open Cloud upload path: the `reqwest` transport now sets a timeout, `User-Agent`, `Accept: application/json`, and content-type headers; response headers are captured; success parsing still requires `versionNumber`; and error mapping now handles 400, 401, 403, 404, 413, 415, 422, 429, nested Roblox JSON error bodies, `errors[]` arrays, `errorMessage`, network failures, and `Retry-After`.
- [x] (2026-05-12 01:03Z) Expanded publisher tests from 6 to 9 offline cases, adding rate-limit retry-after mapping, bad-request multi-error parsing, and malformed success response coverage. Ran `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core place_publisher`, `mise exec -- cargo test -p rbxsync`, and `git diff --check`; all passed.
- [x] (2026-05-12 01:07Z) Implemented Milestone 4 by adding `rbxsync-cli/tests/publish_place.rs` with real-binary offline coverage for dry-run JSON output, API-key redaction, `ROBLOX_OPEN_CLOUD_API_KEY` credential discovery, required credential failures, non-interactive confirmation failures, and unsupported file extension failures.
- [x] (2026-05-12 01:07Z) Updated `INSTALL.md`, `README.md`, and `docs/cli/commands.md` to document `publish-place`, explain the separation from `extract-place`, show dry-run and upload examples, and clarify that `publish-place` uploads an existing local artifact to Roblox Open Cloud.
- [x] (2026-05-12 01:07Z) Ran `mise exec -- cargo fmt`, `mise exec -- cargo test -p rbxsync --test publish_place`, `mise exec -- cargo test -p rbxsync-core place_publisher`, `mise exec -- cargo test -p rbxsync`, and `mise exec -- cargo fmt -- --check`; all passed.
- [x] (2026-05-12 02:06Z) Implemented Milestone 5 by updating `ROADMAP-ISSUES.md` to mark P0 Publish Workflow complete, recording final outcomes in this plan, and running the full validation set: `mise exec -- cargo fmt -- --check`, `mise exec -- cargo test -p rbxsync-core`, `mise exec -- cargo test -p rbxsync --test publish_place`, `mise exec -- cargo test -p rbxsync`, `mise exec -- cargo test --workspace`, and `git diff --check`; all passed. Workspace tests still emit existing non-failing `rbxsync-mcp` dead-code warnings.

## Surprises & Discoveries

- Observation: The repository already has the dependencies needed for an async HTTP publishing client.
  Evidence: `Cargo.toml` defines workspace `reqwest = { version = "0.11", features = ["json"] }`, and `rbxsync-cli/Cargo.toml` already depends on `reqwest`, `tokio`, `serde`, `serde_json`, `anyhow`, and `clap`.

- Observation: The current CLI already has the right summary-output pattern to copy for publish-place.
  Evidence: `rbxsync-cli/src/main.rs::cmd_extract_place` parses command flags, suppresses logs for `--json` and `--quiet`, calls core behavior, and prints either JSON or a concise human summary.

- Observation: Roblox's current place publishing API accepts raw place-file bytes and returns a version number.
  Evidence: The official Creator Hub place publishing guide documents `POST https://apis.roblox.com/universes/v1/{universeId}/places/{placeId}/versions?versionType=Published`, `x-api-key`, `Content-Type: application/octet-stream` for `.rbxl`, `Content-Type: application/xml` for `.rbxlx`, and a success response shaped like `{ "versionNumber": 7 }`.

- Observation: `reqwest` was already a workspace dependency but was not yet available to `rbxsync-core`.
  Evidence: Milestone 1 added `reqwest = { workspace = true }` to `rbxsync-core/Cargo.toml` so the production `ReqwestPublishPlaceTransport` can live beside the core publisher API instead of inside the CLI.

- Observation: The core publisher can be validated without network access.
  Evidence: `place_publisher` tests use a fake `PublishPlaceTransport`; the dry-run test asserts zero requests, the success test inspects the constructed URL, content type, API key, and body, and the error tests verify mapped diagnostics without contacting Roblox.

- Observation: A dry-run must not even initialize the production HTTP client.
  Evidence: The first direct dry-run invocation panicked in the macOS `system-configuration` crate while constructing `reqwest::Client` for a request that would never be sent. The production transport is now lazy and builds a `reqwest` client only in `post`, with `.no_proxy()` to avoid system proxy lookup.

- Observation: `cargo run -- ... --json` is not a valid clean-JSON proof because Cargo prints build/run lines before the program output.
  Evidence: `mise exec -- cargo run -p rbxsync -- publish-place ... --dry-run --json` produced Cargo status lines before JSON, while `target/debug/rbxsync publish-place ... --dry-run --json` emitted only the JSON object.

- Observation: Roblox error bodies may not use one stable JSON shape.
  Evidence: Milestone 3 parsing now accepts top-level `message`, nested `error.message`, top-level `errorMessage`, and `errors[].message` arrays, with tests covering both nested error and multi-error array forms.

- Observation: Rate-limit responses need header context in addition to body text.
  Evidence: `PublishPlaceHttpResponse` now stores normalized response headers, and `maps_rate_limit_with_retry_after_header` verifies that HTTP 429 maps to `RateLimited` and includes `Retry after: 30` in the actionable error message.

- Observation: Useful publish CLI coverage does not need real Roblox credentials.
  Evidence: `rbxsync-cli/tests/publish_place.rs` runs the real `rbxsync` binary only in dry-run or pre-upload failure modes, so it validates command parsing, credential resolution, JSON output, redaction, and confirmation safety without contacting Roblox.

- Observation: Existing docs still described cloud publishing as unimplemented after the command was added.
  Evidence: Milestone 4 changed `INSTALL.md` and `docs/cli/commands.md` so `extract-place` is documented as local artifact generation and `publish-place` is documented as the separate Open Cloud upload step.

- Observation: The roadmap needed completion state rather than deletion.
  Evidence: Milestone 5 changed `ROADMAP-ISSUES.md` from an open P0 Publish Workflow entry to `P0: Publish Workflow - Completed`, preserving the implemented decision and completed scope for future contributors.

## Decision Log

- Decision: Implement publishing as a separate `publish-place` command instead of `extract-place --publish`.
  Rationale: `extract-place` is a local artifact generation workflow. Publishing is a networked, externally visible operation with credentials, permission failures, rate limits, and production risk. A separate command keeps local export safe, scriptable, and testable without network access while making the externally visible operation explicit.
  Date/Author: 2026-05-11 / Codex

- Decision: Require both `--universe-id` and `--place-id` for MVP publishing.
  Rationale: Roblox's publishing endpoint requires both identifiers. Inferring one from the other would require extra API calls and more permissions, which is not necessary for a first publish workflow.
  Date/Author: 2026-05-11 / Codex

- Decision: Use API-key authentication through `--api-key` or `ROBLOX_OPEN_CLOUD_API_KEY`.
  Rationale: The official place publishing guide uses an `x-api-key` header. Supporting an environment variable keeps CI usage practical and avoids requiring users to expose secrets in shell history.
  Date/Author: 2026-05-11 / Codex

- Decision: Make a real publish require confirmation unless `--yes` is passed.
  Rationale: Publishing updates a live Roblox place. The command should protect interactive users from accidental production changes while still allowing CI automation.
  Date/Author: 2026-05-11 / Codex

- Decision: Expose `publish_place_with_transport` and `PublishPlaceTransport` from `rbxsync-core`.
  Rationale: The default `publish_place` function uses the production `reqwest` transport, while tests and later CLI integration can inject a fake transport to prove behavior without real Roblox credentials or network access.
  Date/Author: 2026-05-12 / Codex

- Decision: Keep API-key resolution in the CLI and keep API keys out of summaries.
  Rationale: CLI users need both command-line and environment-variable credential discovery, but the shared core summary should be safe to serialize without accidentally leaking secrets.
  Date/Author: 2026-05-12 / Codex

- Decision: Treat `--json` and `--quiet` as non-interactive publish modes unless `--yes` is passed.
  Rationale: Prompting would corrupt machine-readable JSON and block automation. A real publish updates a live place, so automation must opt in explicitly with `--yes`.
  Date/Author: 2026-05-12 / Codex

- Decision: Add a `PublishPlaceError` wrapper for publish diagnostics.
  Rationale: Publish failures should keep a machine-identifiable diagnostic kind close to the human message even when they flow through `anyhow` to the CLI. This preserves the future path for structured CLI or API error handling without changing the current command surface.
  Date/Author: 2026-05-12 / Codex

- Decision: Keep Milestone 4 CLI tests offline and limited to dry-run/pre-upload paths.
  Rationale: The default test suite must not require real Roblox credentials, real place ownership, or network access. The production upload path is covered in core through fake transports, while CLI tests prove the user-facing behavior before any network call.
  Date/Author: 2026-05-12 / Codex

## Outcomes & Retrospective

This plan has not been implemented yet. The desired outcome is a tested and documented `rbxsync publish-place` command that can publish a local `.rbxl` or `.rbxlx` file and report the resulting Roblox place version. The MVP must be useful in CI, must not leak API keys in logs or JSON output, and must be testable without contacting Roblox by using a mock HTTP transport in automated tests.

Milestone 1 is complete. The core crate now has a reusable `place_publisher` module with format detection, content-type selection, Roblox endpoint construction, dry-run summaries, request construction, success response parsing, and basic diagnostic mapping for authentication, permission, not-found, rate-limit, invalid-place-file, network, and unexpected-response failures. The user-facing `publish-place` command does not exist yet; that remains Milestone 2.

Milestone 2 is complete. The `rbxsync publish-place` command now exists, exposes the expected arguments, resolves credentials from either a flag or environment variable, validates dry-run requests through the core publisher, prints clean JSON from the built binary, and refuses to perform a live publish unless the user confirms interactively or passes `--yes`. The command now has a real production upload path through the core `reqwest` transport, but live Roblox error behavior and broader mocked CLI coverage remain for Milestones 3 and 4.

Milestone 3 is complete. The production upload path is now ready for real Open Cloud requests from the CLI: it sends the raw place file with the correct Roblox endpoint, API key header, content type, accept header, timeout, and user agent; parses successful `versionNumber` responses; and turns common HTTP, network, and malformed-response cases into actionable publish diagnostics. The implementation remains fully validated offline, so Milestone 4 can focus on real-binary CLI tests and user documentation without needing Roblox credentials.

Milestone 4 is complete. The CLI now has dedicated `publish-place` integration tests that execute the real binary while staying offline. User-facing docs now present the full local workflow: `import-place` converts saved places into project files, `extract-place` creates local place artifacts from projects, and `publish-place` uploads those artifacts to Roblox Open Cloud when supplied with explicit IDs and credentials.

Milestone 5 is complete. The publish workflow is implemented as a separate, documented `publish-place` command with offline test coverage and full workspace validation. The remaining roadmap items are outside this publish workflow, including published-place import, atomic `extract-place` writes, asset handling, terrain parity, package defaults, and `import-place --strict`.

## Context and Orientation

RbxSync now has two local place-file workflows. `rbxsync import-place` reads a local `.rbxl` or `.rbxlx` place file and writes an editable RbxSync project under `src/`. `rbxsync extract-place` reads a RbxSync project and writes a local `.rbxl` or `.rbxlx` artifact. The new command sits after `extract-place`: it uploads an already-generated place artifact to Roblox.

A Roblox place is a saved game artifact. A `.rbxl` file is the binary place format and should be uploaded with `Content-Type: application/octet-stream`. A `.rbxlx` file is the XML place format and should be uploaded with `Content-Type: application/xml`. A Universe ID identifies the Roblox experience. A Place ID identifies one place inside that experience. Open Cloud is Roblox's HTTPS API surface for automation; for this feature, authentication is an API key sent in the `x-api-key` header.

The user-facing CLI is implemented in `rbxsync-cli/src/main.rs`. The command enum is named `Commands`. Existing command handlers such as `cmd_import_place` and `cmd_extract_place` live in the same file. Shared library code is in `rbxsync-core/src/`. This feature should put reusable publishing logic in a new core module, tentatively `rbxsync-core/src/place_publisher.rs`, so the CLI is thin and tests can exercise publishing behavior without invoking the full binary.

The current roadmap item is in `ROADMAP-ISSUES.md` under `P0: Publish Workflow`. It asks for confirmation prompts, `--yes` for CI, dry-run behavior, JSON output, and retry/rate-limit/rollback guidance. This ExecPlan chooses the separate-command option.

## Plan of Work

Milestone 1 creates a core publishing API without wiring it to the CLI. Add `rbxsync-core/src/place_publisher.rs` and export it from `rbxsync-core/src/lib.rs`. Define types that represent the request, summary, and diagnostics. The core API should validate the input path, detect `.rbxl` versus `.rbxlx`, choose the correct content type, build the Roblox endpoint URL, and parse the success response. To keep tests offline, introduce a small transport trait or function abstraction that can be implemented by a fake in tests and by `reqwest` in production.

The core API should expose a shape similar to:

    pub enum PublishPlaceFormat {
        Rbxl,
        Rbxlx,
    }

    pub enum PublishVersionType {
        Published,
        Saved,
    }

    pub struct PublishPlaceOptions {
        pub input_path: PathBuf,
        pub universe_id: u64,
        pub place_id: u64,
        pub api_key: String,
        pub version_type: PublishVersionType,
        pub dry_run: bool,
    }

    pub struct PublishPlaceSummary {
        pub input_path: PathBuf,
        pub universe_id: u64,
        pub place_id: u64,
        pub format: PublishPlaceFormat,
        pub content_type: String,
        pub bytes: u64,
        pub version_type: PublishVersionType,
        pub dry_run: bool,
        pub version_number: Option<u64>,
        pub diagnostics: Vec<PublishPlaceDiagnostic>,
    }

    pub async fn publish_place(options: PublishPlaceOptions) -> anyhow::Result<PublishPlaceSummary>;

Milestone 2 adds the CLI command. Add a `Commands::PublishPlace` variant in `rbxsync-cli/src/main.rs`. The command should accept the input place file as a positional argument, `--universe-id`, `--place-id`, `--api-key`, `--version-type`, `--dry-run`, `--json`, `--quiet`, and `--yes`. API key resolution should use `--api-key` first, then `ROBLOX_OPEN_CLOUD_API_KEY`. Human output should never print the key. JSON output should also omit the key and should include `success`, `command`, `dryRun`, `input`, `universeId`, `placeId`, `format`, `bytes`, `versionType`, `versionNumber`, `diagnostics`, and `diagnosticSummary`.

The command should fail before any network call if the input file is missing, the extension is not `.rbxl` or `.rbxlx`, the IDs are zero or invalid, or no API key is available. A non-dry-run publish should require either `--yes` or an interactive confirmation. If stdin is not a terminal and `--yes` is not present, fail with a clear message instructing the user to pass `--yes` for CI.

Milestone 3 implements real HTTP upload and error mapping. Use `reqwest` to POST to:

    https://apis.roblox.com/universes/v1/{universeId}/places/{placeId}/versions?versionType={Published|Saved}

For `.rbxl`, send `Content-Type: application/octet-stream`. For `.rbxlx`, send `Content-Type: application/xml`. Send the raw file bytes as the body and the API key in the `x-api-key` header. Parse the success body for `versionNumber`. For non-success status codes, parse any JSON error body if present and return an actionable error message. Map common cases to diagnostics such as `authenticationFailed`, `permissionDenied`, `notFound`, `rateLimited`, `invalidPlaceFile`, `networkError`, and `unexpectedResponse`.

Milestone 4 adds tests and docs. Add core unit tests for format detection, content-type selection, endpoint construction, dry-run no-upload behavior, success parsing, and HTTP error mapping. Add CLI tests under `rbxsync-cli/tests/publish_place.rs` that run the real binary against dry-run scenarios and mocked publish behavior if the transport can be injected through a test-only local endpoint. Update `INSTALL.md`, `README.md`, and `docs/cli/commands.md` with examples. The docs must make it clear that `extract-place` creates local artifacts and `publish-place` uploads one of those artifacts.

Milestone 5 finalizes validation and roadmap state. Run focused tests, full workspace tests, format checks, and `git diff --check`. Update this plan's `Progress`, `Surprises & Discoveries`, and `Outcomes & Retrospective`. Update `ROADMAP-ISSUES.md` to mark the publish workflow as complete or to split any remaining work into narrower follow-up issues.

## Concrete Steps

Work from the repository root:

    cd /Users/lexiviripaeff/Documents/LoganX/rbxsync

Before editing, inspect the current command and summary patterns:

    rg -n "ExtractPlace|ImportPlace|cmd_extract_place|cmd_import_place|print_export_summary|print_import_summary" rbxsync-cli/src/main.rs
    sed -n '180,286p' rbxsync-cli/src/main.rs
    sed -n '1250,1765p' rbxsync-cli/src/main.rs
    sed -n '1,120p' rbxsync-cli/Cargo.toml
    sed -n '1,160p' rbxsync-core/src/lib.rs

Create `rbxsync-core/src/place_publisher.rs`. Keep raw HTTP details out of the CLI. If a transport trait is used, define it in this module and provide a production implementation that wraps `reqwest::Client`. Keep the API key inside request construction only; do not store it in summaries or diagnostics.

Update `rbxsync-core/src/lib.rs` to export the publishing types and `publish_place`. Add core tests in the same module first, using fake responses to prove behavior without network access.

Update `rbxsync-cli/src/main.rs`. Add `Commands::PublishPlace` near `ExtractPlace` and `ImportPlace`. In the top-level command match, route it to `cmd_publish_place`. Add helper functions for API key resolution, confirmation, version type parsing, and publish summary printing. Follow the existing `--json` and `--quiet` log-suppression pattern used by `ExtractPlace`.

Add CLI tests in `rbxsync-cli/tests/publish_place.rs`. At minimum, dry-run tests should create a temporary `.rbxl` file, run:

    rbxsync publish-place game.rbxl --universe-id 123 --place-id 456 --api-key test-key --dry-run --json

and assert that stdout is valid JSON, `dryRun` is true, `versionNumber` is null, and no secret appears in stdout or stderr.

## Validation and Acceptance

After Milestone 1, run:

    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core place_publisher

After Milestone 2, run:

    mise exec -- cargo run -p rbxsync -- publish-place --help

Expected help should show a separate `publish-place` command with an input file, `--universe-id`, `--place-id`, `--api-key`, `--dry-run`, `--json`, `--quiet`, and `--yes`.

After Milestone 3, validate dry-run behavior without network:

    mkdir -p /tmp/rbxsync-publish-plan
    printf "placeholder" > /tmp/rbxsync-publish-plan/game.rbxl
    mise exec -- cargo run -p rbxsync -- publish-place /tmp/rbxsync-publish-plan/game.rbxl --universe-id 123 --place-id 456 --api-key test-key --dry-run --json

The output should be parseable JSON similar to:

    {
      "success": true,
      "command": "publish-place",
      "dryRun": true,
      "input": "/tmp/rbxsync-publish-plan/game.rbxl",
      "universeId": 123,
      "placeId": 456,
      "format": "rbxl",
      "bytes": 11,
      "versionType": "Published",
      "versionNumber": null,
      "diagnosticCount": 0
    }

Before declaring the plan complete, run:

    mise exec -- cargo fmt -- --check
    mise exec -- cargo test -p rbxsync-core
    mise exec -- cargo test -p rbxsync --test publish_place
    mise exec -- cargo test -p rbxsync
    mise exec -- cargo test --workspace
    git diff --check

Final acceptance is:

1. `rbxsync publish-place --help` documents a standalone command.
2. `rbxsync publish-place ./game.rbxl --universe-id 123 --place-id 456 --api-key test --dry-run --json` returns clean JSON and makes no network request.
3. A real publish sends `.rbxl` with `application/octet-stream` and `.rbxlx` with `application/xml`.
4. A success response returns and prints the Roblox `versionNumber`.
5. API keys are never printed in human output, JSON output, diagnostics, or test failure messages.
6. Non-success Roblox responses produce actionable errors and structured diagnostics.
7. Non-dry-run publishing requires confirmation unless `--yes` is passed.

## Idempotence and Recovery

Dry runs must be safe to repeat and must not contact Roblox. Tests must not require real Roblox credentials, real Universe IDs, or network access. Real publishing is externally visible and cannot be undone by this CLI. The command should tell users that rollback must be performed through Roblox version history or a later dedicated rollback workflow.

If an upload fails before Roblox accepts it, the local input artifact must remain untouched. If Roblox accepts the upload and then the CLI fails while parsing output, the operation may already have changed the live place; error messages should state that the user should verify the target place in Creator Dashboard or Studio.

If implementation needs a real API smoke test, do not add it to the default test suite. Document it as manual validation requiring `ROBLOX_OPEN_CLOUD_API_KEY`, `RBXSYNC_TEST_UNIVERSE_ID`, and `RBXSYNC_TEST_PLACE_ID`, and require the caller to opt in explicitly.

## Artifacts and Notes

Official Roblox place publishing behavior, verified while writing this plan, is:

    POST https://apis.roblox.com/universes/v1/{universeId}/places/{placeId}/versions?versionType=Published
    Header: x-api-key: <api key>
    Header for .rbxl: Content-Type: application/octet-stream
    Header for .rbxlx: Content-Type: application/xml
    Body: raw place-file bytes
    Success JSON: { "versionNumber": 7 }

The current local export command remains:

    rbxsync extract-place --path ./Game --output ./build/game.rbxl --force

The new publish command should then be:

    rbxsync publish-place ./build/game.rbxl --universe-id 123456 --place-id 789012 --yes

## Interfaces and Dependencies

Use these existing dependencies:

- `reqwest` for HTTPS requests to Roblox Open Cloud.
- `tokio` for async command execution.
- `serde` and `serde_json` for request-independent response and summary serialization.
- `clap` for CLI parsing.
- `anyhow` for contextual errors.

Add or expose these core interfaces in `rbxsync-core/src/place_publisher.rs`:

    pub enum PublishPlaceFormat;
    pub enum PublishVersionType;
    pub enum PublishPlaceDiagnosticKind;
    pub struct PublishPlaceDiagnostic;
    pub struct PublishPlaceError;
    pub struct PublishPlaceHttpRequest;
    pub struct PublishPlaceHttpResponse;
    pub struct PublishPlaceOptions;
    pub struct PublishPlaceSummary;
    pub trait PublishPlaceTransport;
    pub async fn publish_place(options: PublishPlaceOptions) -> anyhow::Result<PublishPlaceSummary>;
    pub async fn publish_place_with_transport(options: PublishPlaceOptions, transport: &dyn PublishPlaceTransport) -> anyhow::Result<PublishPlaceSummary>;

If testability requires dependency injection, add an internal transport abstraction that can be fake in tests and backed by `reqwest` in production. The production path must remain simple enough that a novice can search for `publish_place` and find the core implementation, the CLI caller, and tests.

## Revision Notes

2026-05-11: Initial ExecPlan created from `ROADMAP-ISSUES.md` P0 Publish Workflow. The plan chooses a separate `publish-place` command, keeps `extract-place` local-only, and uses the current Roblox Open Cloud place publishing endpoint documented by Creator Hub.

2026-05-12: Milestone 1 completed. Added the core publisher module, exported its API, added the core `reqwest` dependency, introduced the injectable transport abstraction, and validated the module with six offline unit tests.

2026-05-12: Milestone 2 completed. Added the user-facing `publish-place` command, API-key resolution, version-type parsing, confirmation enforcement, publish summaries, dry-run validation, and focused CLI/core validation.

2026-05-12: Milestone 3 completed. Hardened the real HTTP transport and response handling, added structured publish errors, captured response headers for rate-limit guidance, expanded error parsing, and validated the publisher plus CLI crates with focused tests.

2026-05-12: Milestone 4 completed. Added offline real-binary CLI tests for `publish-place`, updated install/readme/command docs, and validated the new coverage alongside core publisher and CLI crate tests.

2026-05-12: Milestone 5 completed. Marked the roadmap Publish Workflow item complete, recorded final outcomes, and ran the full required validation set, including full workspace tests.
