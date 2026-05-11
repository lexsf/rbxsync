# Install and Run Locally

This guide explains how to build this repository on your local machine and run
the command-line place import and export workflows.

## Prerequisites

- Git
- Rust stable, preferably through `mise`
- A local Roblox place file: `.rbxl` or `.rbxlx`

This repository includes `mise.toml`:

```toml
[tools]
rust = "stable"
```

If Rust is not already active, run:

```bash
mise install
mise exec -- cargo --version
```

## Build the CLI

From the repository root:

```bash
mise exec -- cargo build -p rbxsync
```

The debug binary is written to:

```bash
target/debug/rbxsync
```

For an optimized local binary:

```bash
mise exec -- cargo build -p rbxsync --release
```

The release binary is written to:

```bash
target/release/rbxsync
```

## Optional: Install the CLI on Your PATH

To install the local source build as `rbxsync`:

```bash
mise exec -- cargo install --path rbxsync-cli --force
```

After this, verify:

```bash
rbxsync version
```

## Import a Place File

Use `import-place` to convert a saved Roblox place file into a RbxSync project
without opening Roblox Studio:

```bash
target/debug/rbxsync import-place ./Game.rbxl --output ./GameProject --force
```

For XML place files:

```bash
target/debug/rbxsync import-place ./Game.rbxlx --output ./GameProject --force
```

If you installed the CLI on your PATH, use `rbxsync` instead of
`target/debug/rbxsync`.

The output project contains:

- `rbxsync.json`
- `src/<Service>/...`
- `.rbxjson` metadata files
- `.server.luau`, `.client.luau`, or `.luau` script files
- generated tooling files such as `default.project.json`, `selene.toml`, and `wally.toml`

## Useful Import Options

```bash
rbxsync import-place ./Game.rbxl --output ./GameProject --force
rbxsync import-place ./Game.rbxl --output ./GameProject --dry-run
rbxsync import-place ./Game.rbxl --output ./GameProject --json
rbxsync import-place ./Game.rbxl --services Workspace,ServerScriptService
rbxsync import-place ./Game.rbxl --output ./GameProject --name MyGame --no-tooling
```

Notes:

- `--force` is required when replacing an existing `src/` directory.
- By default, existing `src/` is backed up to `.rbxsync-backup/src`.
- Use `--no-backup --force` only when you intentionally want direct replacement.
- `--dry-run` parses the file and reports counts without writing files.
- `--json` prints a machine-readable summary including diagnostics.

## Export a Place Back from Files

After importing or editing a RbxSync project, use `extract-place` to create a
Roblox place file from project files:

```bash
rbxsync extract-place --path ./GameProject --output ./GameProject/build/game.rbxl --force
```

For XML output:

```bash
rbxsync extract-place --path ./GameProject --output ./GameProject/build/game.rbxlx --force
```

Useful export options:

```bash
rbxsync extract-place --path ./GameProject --dry-run --json
rbxsync extract-place --path ./GameProject --services Workspace,ServerScriptService
rbxsync extract-place --path ./GameProject --output ./build/game.rbxlx --format rbxlx --force
rbxsync extract-place --path ./GameProject --no-packages --force
```

Notes:

- `--force` is required when replacing an existing output file.
- `--dry-run` validates and summarizes without writing a place file.
- `--json` prints a machine-readable summary including diagnostics.
- `extract-place` creates local `.rbxl` or `.rbxlx` files. It does not publish
  to Roblox cloud services.

The older `build` command remains available for generic artifact creation,
including `.rbxm` and `.rbxmx` model outputs:

```bash
rbxsync build --path ./GameProject --output ./GameProject/build/game.rbxl
rbxsync build --path ./GameProject --output ./GameProject/build/model.rbxm --format rbxm
```

## Run Validation

Focused checks for the importer:

```bash
mise exec -- cargo test -p rbxsync-core
mise exec -- cargo test -p rbxsync --test import_place
mise exec -- cargo test -p rbxsync --test extract_place
```

Full workspace validation:

```bash
mise exec -- cargo test --workspace
mise exec -- cargo fmt -- --check
git diff --check
```

`rbxsync-mcp` currently emits non-failing dead-code warnings during workspace
tests.

## Troubleshooting

If `cargo` is not found, activate the toolchain:

```bash
mise install
mise exec -- cargo --version
```

If the import refuses to overwrite `src/`, rerun with `--force`.

If JSON output is needed for scripts or CI, use:

```bash
rbxsync import-place ./Game.rbxl --output ./GameProject --force --json
```

Published `--place-id` import and cloud publishing are not implemented in this
local-file workflow.
