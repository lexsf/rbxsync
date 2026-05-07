# Repository Guidelines

## Project Structure & Module Organization

RbxSync is a mixed Rust, TypeScript, and Luau repository. Rust workspace crates live in `rbxsync-core/`, `rbxsync-server/`, `rbxsync-cli/`, and `rbxsync-mcp/`, with source under each crate's `src/` directory and Rust integration tests under `rbxsync-server/tests/`. The VS Code extension is in `rbxsync-vscode/`, with commands, views, LSP, and server client code in `rbxsync-vscode/src/`. The Roblox Studio plugin lives in `plugin/src/`. Documentation and release content are in `docs/`, `testing/`, `website/`, `README.md`, and `RELEASING.md`.

## Build, Test, and Development Commands

- `make` or `make build`: build all Rust crates in release mode.
- `make all`: build Rust crates, the VS Code extension, and the Studio plugin.
- `make check`: run Clippy, Rust tests, and `cargo fmt --check`.
- `make fmt-fix`: apply Rust formatting.
- `cd rbxsync-vscode && npm ci && npm run build`: install extension dependencies and build the production bundle.
- `cd rbxsync-vscode && npm run lint`: lint TypeScript extension sources.
- `cd docs && npm install && npm run dev`: run local VitePress docs.
- `./testing/scripts/run-all-tests.sh`: run automated CLI, migration, and documentation tests.

## Coding Style & Naming Conventions

Use Rust 2021 conventions and keep Rust code formatted with `cargo fmt`; Clippy warnings are treated as failures by `make check`. TypeScript in `rbxsync-vscode/src/` should pass the extension ESLint configuration and use existing module patterns such as `commands/`, `views/`, `lsp/`, and `server/`. Luau plugin files use consistent 4-space indentation. Prefer descriptive names tied to sync concepts, such as `file_watcher`, `projectJson`, or `completionProvider`.

## Testing Guidelines

Add Rust tests near the affected crate or under `rbxsync-server/tests/` for integration coverage. Use the scripts in `testing/scripts/` for release-oriented checks that do not require Roblox Studio. Manual Studio behavior, including plugin sync, auto-connect, settings, and file watcher behavior, should be documented or verified with `testing/TESTING.md`.

## Commit & Pull Request Guidelines

Recent history uses Conventional Commit prefixes such as `feat:` and `fix:`, often with issue references like `RBXSYNC-111` or `Fixes RBXSYNC-119`. Keep commits scoped and descriptive. Pull requests should explain the user-visible change, link the relevant issue, list validation commands run, and include screenshots or screen recordings for VS Code UI or Studio plugin changes.

## Agent-Specific Instructions

Read relevant source before editing. Keep changes focused on the requested component, avoid unrelated formatting churn, and run the narrowest meaningful validation before reporting completion.
