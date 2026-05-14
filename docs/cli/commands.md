# CLI Commands

Complete reference for all RbxSync CLI commands.

## Core Commands

### init
Initialize a new RbxSync project.

```bash
rbxsync init [--name NAME]
```

Creates `rbxsync.json` and the `src/` directory structure.

### serve
Start the sync server.

```bash
rbxsync serve [--port PORT] [--background]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--port` | 44755 | Server port |
| `--background, -b` | false | Run server as a background daemon |

Run in background mode for a cleaner terminal:

```bash
# Start in background
rbxsync serve --background

# Stop the background server
rbxsync stop
```

### stop
Stop the running server.

```bash
rbxsync stop
```

### status
Show connection status.

```bash
rbxsync status
```

### extract
Extract game from connected Studio to files.

```bash
rbxsync extract
```

Requires an active Studio connection.

### sync
Push local changes to Studio.

```bash
rbxsync sync [--path DIR]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--path` | Current dir | Project path |

## Build Commands

### build
Build project to Roblox format.

```bash
rbxsync build [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-f, --format` | rbxl | Output format: rbxl, rbxm, rbxlx, rbxmx |
| `-o, --output` | build/ | Output path |
| `--watch` | false | Watch for changes and rebuild |
| `--plugin` | - | Build directly to Studio plugins folder |

Examples:

```bash
# Build place file
rbxsync build

# Build model file
rbxsync build -f rbxm

# Build XML format
rbxsync build -f rbxlx

# Watch mode
rbxsync build --watch

# Build as plugin
rbxsync build --plugin MyPlugin.rbxm
```

### import-place
Import a local Roblox place file into a RbxSync project.

```bash
rbxsync import-place <INPUT> [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `<INPUT>` | Required | Input `.rbxl` or `.rbxlx` file |
| `-o, --output` | Current dir | Output project directory |
| `--name` | Input stem | Project name for generated config |
| `--services` | All services | Comma-separated services to import |
| `--terrain` | false | Include Terrain metadata and exportable raw Terrain payloads |
| `--force` | false | Replace an existing `src/` tree |
| `--no-backup` | false | Replace `src/` without `.rbxsync-backup/src` |
| `--tooling` / `--no-tooling` | Project config | Control generated tooling files |
| `--include-assets` | false | Write `assets/manifest.json` and local embedded payload files |
| `--no-assets` | false | Preserve inline asset metadata and do not write `assets/` |
| `--dry-run` | false | Parse and summarize without writing |
| `--json` | false | Emit machine-readable JSON |

`--include-assets` extracts embedded `BinaryString` and `SharedString`
payloads to `assets/blobs/` and records them in `assets/manifest.json`.
External `Content` asset IDs are preserved as references and are not
downloaded.

`--terrain` preserves raw `Workspace/Terrain` payload properties exposed by the
local place parser. Exportable payloads are written to
`terrain/Workspace/Terrain.rbxterrain.json` and `terrain/blobs/<sha256>.bin`;
ordinary Terrain metadata remains under `src/Workspace/Terrain.rbxjson` or
`src/Workspace/Terrain/_meta.rbxjson`.

### extract-place
Export a RbxSync project to a local Roblox place file.

```bash
rbxsync extract-place [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-p, --path` | Current dir | Project directory |
| `-o, --output` | build/game.rbxl | Output `.rbxl` or `.rbxlx` file |
| `-f, --format` | Output extension or rbxl | Output format: rbxl or rbxlx |
| `--force` | false | Replace an existing output file |
| `--dry-run` | false | Validate and summarize without writing |
| `--json` | false | Emit machine-readable JSON |
| `--strict` | false | Fail if diagnostics are produced |
| `--services` | All services | Comma-separated services to export |
| `--include-packages` | Auto | Force package folders to be included, even if disabled by `rbxsync.json` |
| `--no-packages` | Auto | Skip package folders even when present or enabled by `rbxsync.json` |
| `--include-assets` | false | Read `assets/manifest.json` and embed file-backed payloads |
| `--no-assets` | false | Ignore `assets/manifest.json` and only use inline metadata |

By default, `extract-place` includes `Packages` and `ServerPackages` folders
that are present in the exported tree. If `rbxsync.json` explicitly sets
`"packages": { "enabled": false }`, the default switches to skipping package
folders. Use `--include-packages` to force a one-off export with packages, or
`--no-packages` to force a one-off export without packages.

Examples:

```bash
rbxsync extract-place --path ./GameProject --output ./build/Game.rbxl --force
rbxsync extract-place --path ./GameProject --output ./build/Game.rbxlx --force
rbxsync extract-place --path ./GameProject --dry-run --json
rbxsync extract-place --path ./GameProject --no-packages --output ./build/Game.rbxl --force
rbxsync extract-place --path ./GameProject --include-assets --output ./build/Game.rbxl --force
```

This command creates local place files only. Use `publish-place` to upload the
artifact to Roblox Open Cloud.

If `terrain/Workspace/Terrain.rbxterrain.json` exists and `Workspace` is included
by the service filter, `extract-place` automatically embeds the referenced raw
Terrain payloads into the generated `.rbxl` or `.rbxlx`. Missing payload files,
hash mismatches, paths outside the project, and invalid raw terrain manifests
fail the export to avoid silently dropping terrain data. Legacy Studio chunk
terrain at `src/Workspace/Terrain/terrain.rbxjson` is reported as unsupported
for place-file export until a converter exists.

When terrain is involved, `--json` includes a `terrain` object:

```json
{
  "mode": "rawProperties",
  "manifest": "terrain/Workspace/Terrain.rbxterrain.json",
  "rawPayloads": 1,
  "chunkCount": null,
  "bytesRead": 12345,
  "bytesWritten": 0,
  "diagnosticCount": 0,
  "diagnostics": []
}
```

### publish-place
Publish an existing `.rbxl` or `.rbxlx` place file to Roblox Open Cloud.

```bash
rbxsync publish-place <INPUT> --universe-id <ID> --place-id <ID> [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `<INPUT>` | Required | Place file to publish |
| `--universe-id` | Required | Roblox universe ID that owns the place |
| `--place-id` | Required | Roblox place ID to update |
| `--api-key` | `ROBLOX_OPEN_CLOUD_API_KEY` | Roblox Open Cloud API key |
| `--version-type` | published | Version type: published or saved |
| `--dry-run` | false | Validate and summarize without uploading |
| `--json` | false | Emit machine-readable JSON |
| `--quiet` | false | Suppress human-readable output |
| `--yes` | false | Confirm upload for CI/non-interactive use |

Examples:

```bash
export ROBLOX_OPEN_CLOUD_API_KEY="your-open-cloud-api-key"
rbxsync publish-place ./build/Game.rbxl --universe-id 123456 --place-id 789012 --dry-run --json
rbxsync publish-place ./build/Game.rbxl --universe-id 123456 --place-id 789012 --yes
rbxsync publish-place ./build/Game.rbxlx --universe-id 123456 --place-id 789012 --version-type saved --yes
```

The command never prints the API key in summaries. Real uploads update a live
Roblox place; use `--dry-run` first in scripts and `--yes` only when ready.

### build-plugin
Build the RbxSync Studio plugin.

```bash
rbxsync build-plugin [--install]
```

| Option | Description |
|--------|-------------|
| `--install` | Copy to Studio plugins folder |

## Utility Commands

### sourcemap
Generate sourcemap.json for Luau LSP.

```bash
rbxsync sourcemap
```

### fmt-project
Format all .rbxjson files.

```bash
rbxsync fmt-project [--check]
```

| Option | Description |
|--------|-------------|
| `--check` | Check only, don't modify (for CI) |

### studio
Launch Roblox Studio.

```bash
rbxsync studio [file.rbxl]
```

### doc
Open documentation in browser.

```bash
rbxsync doc
```

## Update Commands

### version
Show version and git commit.

```bash
rbxsync version
```

### update
Pull latest changes and rebuild.

```bash
rbxsync update [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--vscode` | Also rebuild VS Code extension |
| `--no-pull` | Skip git pull, just rebuild |

This command:
1. Pulls latest from GitHub
2. Rebuilds the CLI
3. Rebuilds and installs the Studio plugin

Then restart Studio to load the updated plugin.

### uninstall
Completely remove RbxSync from your system.

```bash
rbxsync uninstall [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--vscode` | Also remove VS Code extension |
| `--keep-repo` | Keep the cloned repo at ~/.rbxsync/repo |
| `-y, --yes` | Skip confirmation prompt |

## Migration Commands

### migrate
Migrate from another sync tool to RbxSync.

```bash
rbxsync migrate [--from FORMAT] [--path DIR] [--force]
```

| Option | Default | Description |
|--------|---------|-------------|
| `--from` | rojo | Source format to migrate from |
| `--path` | Current dir | Project directory |
| `--force` | false | Overwrite existing rbxsync.json |

Currently supports migrating from Rojo projects.

Example:

```bash
# Migrate a Rojo project
cd my-rojo-project
rbxsync migrate

# Or specify the path
rbxsync migrate --path /path/to/rojo/project

# Force overwrite existing config
rbxsync migrate --force
```

This reads your `default.project.json` (or `*.project.json`) and creates an equivalent `rbxsync.json` with:
- Project name
- Tree mappings (DataModel path → filesystem path)
- Default RbxSync settings

Your Rojo project file is preserved—you can use both tools side-by-side.
