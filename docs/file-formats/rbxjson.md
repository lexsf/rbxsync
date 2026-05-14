# .rbxjson Format

The `.rbxjson` format stores non-script instances with full property preservation.

## Basic Structure

```json
{
  "className": "Part",
  "name": "Baseplate",
  "properties": {
    "Anchored": {
      "type": "bool",
      "value": true
    },
    "Size": {
      "type": "Vector3",
      "value": { "x": 512, "y": 20, "z": 512 }
    }
  }
}
```

## Fields

| Field | Required | Description |
|-------|----------|-------------|
| `className` | Yes | Roblox class name |
| `name` | No | Instance name (defaults to filename) |
| `properties` | No | Property definitions |

## Property Format

Each property has a `type` and `value`:

```json
"PropertyName": {
  "type": "TypeName",
  "value": <value>
}
```

## Example: Full Part

```json
{
  "className": "Part",
  "properties": {
    "Anchored": {
      "type": "bool",
      "value": true
    },
    "Size": {
      "type": "Vector3",
      "value": { "x": 512, "y": 20, "z": 512 }
    },
    "Color": {
      "type": "Color3",
      "value": { "r": 0.388, "g": 0.372, "b": 0.384 }
    },
    "Material": {
      "type": "Enum",
      "value": { "enumType": "Material", "value": "Grass" }
    },
    "Transparency": {
      "type": "float",
      "value": 0
    }
  }
}
```

## Example: GUI Element

```json
{
  "className": "Frame",
  "properties": {
    "Size": {
      "type": "UDim2",
      "value": {
        "x": { "scale": 0.5, "offset": 0 },
        "y": { "scale": 0.5, "offset": 0 }
      }
    },
    "Position": {
      "type": "UDim2",
      "value": {
        "x": { "scale": 0.25, "offset": 0 },
        "y": { "scale": 0.25, "offset": 0 }
      }
    },
    "BackgroundColor3": {
      "type": "Color3",
      "value": { "r": 0.2, "g": 0.2, "b": 0.2 }
    }
  }
}
```

## Formatting

Run `rbxsync fmt-project` to format all .rbxjson files consistently.

For CI/CD, use `rbxsync fmt-project --check` to verify formatting.

See [Property Types](/file-formats/property-types) for all supported types.

## Terrain Metadata

`Workspace/Terrain` may still have ordinary `.rbxjson` metadata for supported
properties such as water settings or `Decoration`:

```json
{
  "className": "Terrain",
  "properties": {
    "Decoration": { "type": "bool", "value": true }
  }
}
```

Raw Terrain voxel payloads are not stored inline in `.rbxjson`. When
`import-place --terrain` can preserve exportable Terrain binary properties, it
writes those payloads to the terrain format instead:
`terrain/Workspace/Terrain.rbxterrain.json` plus `terrain/blobs/<sha256>.bin`.
See [Terrain Files](/file-formats/terrain) for the manifest format and
round-trip behavior.
