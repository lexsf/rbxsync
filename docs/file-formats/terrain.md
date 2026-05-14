# Terrain Files

RbxSync stores local place Terrain payloads outside `src/` so large voxel data
does not become ordinary instance metadata.

## Layout

```
MyGame/
├── terrain/
│   ├── Workspace/
│   │   └── Terrain.rbxterrain.json
│   └── blobs/
│       └── <sha256>.bin
└── src/
    └── Workspace/
        └── Terrain.rbxjson
```

`src/Workspace/Terrain.rbxjson` or `src/Workspace/Terrain/_meta.rbxjson` stores
normal Terrain metadata. `terrain/Workspace/Terrain.rbxterrain.json` stores
references to raw payload blobs that can be embedded back into a local
`.rbxl` or `.rbxlx`.

## Raw Manifest

```json
{
  "version": 1,
  "format": "rawProperties",
  "terrainPath": "Workspace/Terrain",
  "className": "Terrain",
  "name": "Terrain",
  "metadataProperties": {
    "Decoration": { "type": "bool", "value": true }
  },
  "materialColors": {},
  "voxelProperties": {
    "SmoothGrid": {
      "type": "binaryString",
      "file": "terrain/blobs/<sha256>.bin",
      "encoding": "raw",
      "sha256": "<sha256>",
      "byteLength": 12345
    }
  }
}
```

`voxelProperties` keys are exact Terrain property names from the local place
file. Each payload file path must be project-relative and must stay inside the
project directory. `extract-place` verifies the `sha256` before embedding the
payload into the output place file.

## Commands

Use `import-place --terrain` to preserve raw Terrain payloads from a local
place file:

```bash
rbxsync import-place Game.rbxl --terrain --output MyGame --force --json
```

Use `extract-place` to embed a stored raw manifest back into a local place file:

```bash
rbxsync extract-place --path MyGame --output build/Game.rbxl --force --json
```

No export flag is required. If the raw manifest exists and `Workspace` is
included by `--services`, the payloads are included automatically.

## JSON Summary

When terrain is preserved or detected, command JSON includes a `terrain` object:

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

`bytesWritten` is used by import because it writes blob files. `bytesRead` is
used by export because it reads existing blob files into the generated place.

## Legacy Studio Chunks

Studio sync terrain may still exist as
`src/Workspace/Terrain/terrain.rbxjson`. That chunk format is used by the
plugin/server `ReadVoxels` and `WriteVoxels` path. It is readable for sync
compatibility, but `extract-place` cannot convert it into Roblox place-file
Terrain binary yet. Chunk-only projects export Terrain metadata and report
`unsupportedTerrainVoxelData`; `--strict` turns that diagnostic into a failed
export.
