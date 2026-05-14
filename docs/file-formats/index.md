# File Formats

RbxSync uses text metadata plus optional local payload files to represent Roblox
instances and large binary data.

## Overview

| Format | Extension | Use Case |
|--------|-----------|----------|
| Luau Scripts | `.luau` | Script source code |
| Instance Data | `.rbxjson` | Properties and metadata |
| Terrain Manifest | `.rbxterrain.json` | Raw local place Terrain payload references |

## Script Files

Scripts are stored as plain `.luau` files with naming conventions:

```
MyScript.server.luau  → Script (runs on server)
MyScript.client.luau  → LocalScript (runs on client)
MyScript.luau         → ModuleScript
```

See [.luau Scripts](/file-formats/luau) for details.

## Instance Files

Non-script instances use `.rbxjson` for full property preservation:

```json
{
  "className": "Part",
  "properties": {
    "Anchored": { "type": "bool", "value": true },
    "Size": { "type": "Vector3", "value": { "x": 4, "y": 1, "z": 2 } }
  }
}
```

See [.rbxjson Format](/file-formats/rbxjson) for details.

## Terrain Files

Local place terrain round trips use a top-level `terrain/` directory. The
canonical manifest for `Workspace/Terrain` is
`terrain/Workspace/Terrain.rbxterrain.json`, and raw payload bytes live under
`terrain/blobs/<sha256>.bin`.

See [Terrain Files](/file-formats/terrain) for details.

## Project Structure

```
MyGame/
├── rbxsync.json          # Project config
├── terrain/
│   ├── Workspace/
│   │   └── Terrain.rbxterrain.json
│   └── blobs/
│       └── <sha256>.bin
├── src/
│   ├── Workspace/
│   │   ├── Terrain.rbxjson
│   │   ├── Baseplate.rbxjson
│   │   └── SpawnLocation.rbxjson
│   ├── ServerScriptService/
│   │   └── Main.server.luau
│   ├── ReplicatedStorage/
│   │   └── Modules/
│   │       ├── _meta.rbxjson    # Folder metadata
│   │       └── Utils.luau
│   └── Lighting.rbxjson
└── sourcemap.json        # For Luau LSP
```

## Meta Files

Use `_meta.rbxjson` to set properties on folder instances:

```
src/
├── Workspace/
│   ├── _meta.rbxjson      # Properties for Workspace service
│   ├── Baseplate.rbxjson
│   └── SpawnLocation.rbxjson
```
