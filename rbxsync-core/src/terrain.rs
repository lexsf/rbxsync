//! Shared terrain manifest and payload helpers.
//!
//! This module defines the stable on-disk terrain representation used by local
//! place import/export work. Milestone 1 keeps the helpers self-contained:
//! callers can read/write terrain manifests, write content-addressed payload
//! blobs, and validate blob paths without changing command behavior yet.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rbx_dom_weak::types::{BinaryString, Variant};
use rbx_dom_weak::Instance;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{asset_sha256_hex, read_asset_file};

/// Current raw terrain manifest schema version.
pub const TERRAIN_MANIFEST_VERSION: u32 = 1;

/// Terrain data shape recognized in an RbxSync project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TerrainProjectData {
    /// File-backed raw properties from a local `.rbxl` / `.rbxlx` Terrain instance.
    #[serde(rename = "rawProperties")]
    Raw(RawTerrainData),
    /// Legacy Studio `ReadVoxels` chunk JSON written by the plugin/server path.
    #[serde(rename = "chunks")]
    Chunks(ChunkTerrainData),
}

/// Raw file-backed Terrain properties that can be embedded into a place file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTerrainData {
    pub version: u32,
    #[serde(default = "raw_properties_format")]
    pub format: TerrainDataFormat,
    pub terrain_path: String,
    pub class_name: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
    #[serde(default)]
    pub metadata_properties: BTreeMap<String, Value>,
    #[serde(default)]
    pub material_colors: BTreeMap<String, Value>,
    #[serde(default)]
    pub voxel_properties: BTreeMap<String, TerrainPayloadRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<TerrainDiagnostic>,
}

/// Raw Terrain manifest format discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerrainDataFormat {
    RawProperties,
}

/// Raw Terrain data plus the bytes that must be written to blob files.
#[derive(Debug, Clone, PartialEq)]
pub struct RawTerrainExtraction {
    pub data: RawTerrainData,
    pub blobs: Vec<TerrainBlobWrite>,
}

/// One content-addressed Terrain blob to write under the project directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainBlobWrite {
    pub file: String,
    pub bytes: Vec<u8>,
}

/// Legacy plugin terrain chunk data. The raw JSON is kept so the sync path can
/// continue consuming it without lossy normalization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkTerrainData {
    pub manifest: String,
    pub chunk_count: usize,
    pub raw: Value,
}

/// One file-backed Terrain payload property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainPayloadRef {
    #[serde(rename = "type")]
    pub property_type: TerrainPayloadType,
    pub file: String,
    pub encoding: TerrainPayloadEncoding,
    pub sha256: String,
    pub byte_length: u64,
}

/// Supported raw Terrain payload variant types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerrainPayloadType {
    BinaryString,
    SharedString,
}

/// Encoding of a Terrain payload blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerrainPayloadEncoding {
    Raw,
}

/// Aggregate terrain counts for CLI summaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainSummary {
    pub mode: TerrainSummaryMode,
    pub manifest: Option<String>,
    pub raw_payloads: usize,
    pub chunk_count: Option<usize>,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub diagnostic_count: usize,
    pub diagnostics: Vec<TerrainDiagnostic>,
}

/// Summary mode for terrain data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerrainSummaryMode {
    RawProperties,
    Chunks,
    MetadataOnly,
    None,
}

/// Terrain diagnostic category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerrainDiagnosticKind {
    InvalidTerrainManifest,
    MissingTerrainPayload,
    TerrainPayloadHashMismatch,
    TerrainPayloadOutsideProject,
    UnsupportedTerrainVoxelData,
    DuplicateTerrainData,
    NoTerrainPayloadsFound,
}

/// Non-fatal terrain diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainDiagnostic {
    pub kind: TerrainDiagnosticKind,
    pub path: String,
    pub property: Option<String>,
    pub message: String,
}

/// Terrain file kind found in a project for Studio sync compatibility checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerrainProjectFileKind {
    RawProperties,
    Chunks,
}

impl TerrainProjectFileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RawProperties => "rawProperties",
            Self::Chunks => "chunks",
        }
    }
}

/// Terrain file discovered in a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainProjectFile {
    pub kind: TerrainProjectFileKind,
    pub path: PathBuf,
    pub project_relative_path: String,
}

/// Build the canonical raw terrain manifest path for a DataModel Terrain path.
pub fn canonical_terrain_manifest(project_dir: &Path, terrain_path: &str) -> PathBuf {
    let mut segments = terrain_path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let file_stem = segments.pop().unwrap_or("Terrain");
    let mut path = project_dir.join("terrain");
    for segment in segments {
        path = path.join(segment);
    }
    path.join(format!("{}.rbxterrain.json", file_stem))
}

/// Legacy Studio chunk terrain file written by the server/plugin path.
pub fn legacy_terrain_chunk_file(project_dir: &Path) -> PathBuf {
    project_dir
        .join("src")
        .join("Workspace")
        .join("Terrain")
        .join("terrain.rbxjson")
}

/// Older legacy Studio terrain file shape checked by server sync.
pub fn legacy_flat_terrain_chunk_file(project_dir: &Path) -> PathBuf {
    project_dir
        .join("src")
        .join("Workspace")
        .join("Terrain.rbxjson")
}

/// Project-relative path to the canonical raw terrain manifest.
pub fn raw_terrain_manifest_relative_path(terrain_path: &str) -> String {
    let mut segments = terrain_path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let file_stem = segments.pop().unwrap_or("Terrain");
    let mut parts = vec!["terrain".to_string()];
    parts.extend(segments.into_iter().map(ToString::to_string));
    parts.push(format!("{}.rbxterrain.json", file_stem));
    parts.join("/")
}

/// Find the best terrain file for Studio sync.
///
/// Legacy chunk terrain is returned before raw local place terrain because the
/// Studio plugin can apply chunks through `WriteVoxels`, but it cannot apply raw
/// place-file payload manifests yet.
pub fn find_studio_sync_terrain_file(project_dir: &Path) -> Option<TerrainProjectFile> {
    for (kind, path) in [
        (
            TerrainProjectFileKind::Chunks,
            legacy_terrain_chunk_file(project_dir),
        ),
        (
            TerrainProjectFileKind::Chunks,
            legacy_flat_terrain_chunk_file(project_dir),
        ),
        (
            TerrainProjectFileKind::RawProperties,
            canonical_terrain_manifest(project_dir, "Workspace/Terrain"),
        ),
    ] {
        if path.exists() {
            return Some(TerrainProjectFile {
                kind,
                project_relative_path: path_to_project_string(project_dir, &path),
                path,
            });
        }
    }

    None
}

/// Read canonical raw terrain data or legacy chunk terrain data from a project.
pub fn read_terrain_project_data(project_dir: &Path) -> Result<Option<TerrainProjectData>> {
    let raw_manifest = canonical_terrain_manifest(project_dir, "Workspace/Terrain");
    if raw_manifest.exists() {
        return read_raw_terrain_data(&raw_manifest)
            .map(TerrainProjectData::Raw)
            .map(Some);
    }

    for legacy in [
        legacy_terrain_chunk_file(project_dir),
        legacy_flat_terrain_chunk_file(project_dir),
    ] {
        if legacy.exists() {
            return read_chunk_terrain_data(&legacy)
                .map(TerrainProjectData::Chunks)
                .map(Some);
        }
    }

    Ok(None)
}

/// Read a raw terrain manifest.
pub fn read_raw_terrain_data(path: &Path) -> Result<RawTerrainData> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read terrain manifest {}", path.display()))?;
    let data = serde_json::from_str::<RawTerrainData>(&content)
        .with_context(|| format!("Failed to parse terrain manifest {}", path.display()))?;
    Ok(data)
}

/// Write a raw terrain manifest through a temporary sibling file and rename.
pub fn write_raw_terrain_data(project_dir: &Path, data: &RawTerrainData) -> Result<TerrainSummary> {
    let manifest_path = canonical_terrain_manifest(project_dir, &data.terrain_path);
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create terrain manifest directory {}",
                parent.display()
            )
        })?;
    }

    let mut normalized = data.clone();
    normalized.version = TERRAIN_MANIFEST_VERSION;
    normalized.format = TerrainDataFormat::RawProperties;
    let content = serde_json::to_string_pretty(&normalized)? + "\n";
    let tmp_path = manifest_path.with_extension("rbxterrain.json.tmp");
    std::fs::write(&tmp_path, content).with_context(|| {
        format!(
            "Failed to write temporary terrain manifest {}",
            tmp_path.display()
        )
    })?;
    std::fs::rename(&tmp_path, &manifest_path).with_context(|| {
        format!(
            "Failed to replace terrain manifest {}",
            manifest_path.display()
        )
    })?;

    Ok(summarize_raw_terrain(project_dir, &normalized, 0, 0))
}

/// Write collected raw Terrain payload blobs and the raw terrain manifest.
pub fn write_raw_terrain_extraction(
    project_dir: &Path,
    extraction: &RawTerrainExtraction,
) -> Result<TerrainSummary> {
    let mut bytes_written = 0;
    for blob in &extraction.blobs {
        let path = project_dir.join(&blob.file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create terrain blob directory {}",
                    parent.display()
                )
            })?;
        }
        std::fs::write(&path, &blob.bytes)
            .with_context(|| format!("Failed to write terrain blob {}", path.display()))?;
        bytes_written += blob.bytes.len() as u64;
    }

    write_raw_terrain_data(project_dir, &extraction.data)?;
    Ok(summarize_raw_terrain(
        project_dir,
        &extraction.data,
        0,
        bytes_written,
    ))
}

/// Write a Terrain payload blob under `terrain/blobs/<sha256>.bin`.
pub fn write_terrain_blob(project_dir: &Path, bytes: &[u8]) -> Result<TerrainPayloadRef> {
    let digest = asset_sha256_hex(bytes);
    let file = format!("terrain/blobs/{}.bin", digest);
    let path = project_dir.join(&file);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create terrain blob directory {}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&path, bytes)
        .with_context(|| format!("Failed to write terrain blob {}", path.display()))?;

    Ok(TerrainPayloadRef {
        property_type: TerrainPayloadType::BinaryString,
        file,
        encoding: TerrainPayloadEncoding::Raw,
        sha256: digest,
        byte_length: bytes.len() as u64,
    })
}

/// Read a project-relative terrain payload and verify its SHA-256 digest.
pub fn read_terrain_payload(project_dir: &Path, payload: &TerrainPayloadRef) -> Result<Vec<u8>> {
    read_asset_file(project_dir, &payload.file, Some(&payload.sha256))
        .with_context(|| format!("Failed to read terrain payload {}", payload.file))
}

/// Build a raw terrain data model from a Terrain instance, writing payload blobs
/// for supported opaque binary properties.
pub fn extract_raw_terrain_from_instance(
    instance: &Instance,
    terrain_path: &str,
    project_dir: &Path,
) -> Result<Option<RawTerrainData>> {
    let Some(extraction) = collect_raw_terrain_from_instance(instance, terrain_path)? else {
        return Ok(None);
    };
    write_raw_terrain_extraction(project_dir, &extraction)?;
    Ok(Some(extraction.data))
}

/// Build a raw Terrain extraction from an instance without writing files.
pub fn collect_raw_terrain_from_instance(
    instance: &Instance,
    terrain_path: &str,
) -> Result<Option<RawTerrainExtraction>> {
    let mut metadata_properties = BTreeMap::new();
    let mut material_colors = BTreeMap::new();
    let mut voxel_properties = BTreeMap::new();
    let mut blobs = Vec::new();

    let mut property_names = instance.properties.keys().collect::<Vec<_>>();
    property_names.sort();

    for property_name in property_names {
        let Some(variant) = instance.properties.get(property_name) else {
            continue;
        };

        match variant {
            Variant::BinaryString(value) => {
                let bytes = <BinaryString as AsRef<[u8]>>::as_ref(value).to_vec();
                let (mut payload, blob) = payload_ref_and_blob(&bytes);
                payload.property_type = TerrainPayloadType::BinaryString;
                voxel_properties.insert(property_name.clone(), payload);
                blobs.push(blob);
            }
            Variant::SharedString(value) => {
                let (mut payload, blob) = payload_ref_and_blob(value.data());
                payload.property_type = TerrainPayloadType::SharedString;
                voxel_properties.insert(property_name.clone(), payload);
                blobs.push(blob);
            }
            Variant::MaterialColors(value) => {
                material_colors.insert(property_name.clone(), serde_json::to_value(value)?);
            }
            other => {
                if let Some(encoded) = terrain_metadata_property(other) {
                    metadata_properties.insert(property_name.clone(), encoded);
                }
            }
        }
    }

    if voxel_properties.is_empty() {
        return Ok(None);
    }

    Ok(Some(RawTerrainExtraction {
        data: RawTerrainData {
            version: TERRAIN_MANIFEST_VERSION,
            format: TerrainDataFormat::RawProperties,
            terrain_path: terrain_path.to_string(),
            class_name: instance.class.clone(),
            name: instance.name.clone(),
            reference_id: Some(instance.referent().to_string()),
            metadata_properties,
            material_colors,
            voxel_properties,
            diagnostics: Vec::new(),
        },
        blobs,
    }))
}

fn raw_properties_format() -> TerrainDataFormat {
    TerrainDataFormat::RawProperties
}

fn payload_ref_and_blob(bytes: &[u8]) -> (TerrainPayloadRef, TerrainBlobWrite) {
    let digest = asset_sha256_hex(bytes);
    let file = format!("terrain/blobs/{}.bin", digest);
    (
        TerrainPayloadRef {
            property_type: TerrainPayloadType::BinaryString,
            file: file.clone(),
            encoding: TerrainPayloadEncoding::Raw,
            sha256: digest,
            byte_length: bytes.len() as u64,
        },
        TerrainBlobWrite {
            file,
            bytes: bytes.to_vec(),
        },
    )
}

/// Summarize raw terrain data with optional IO counts supplied by the caller.
pub fn summarize_raw_terrain(
    project_dir: &Path,
    data: &RawTerrainData,
    bytes_read: u64,
    bytes_written: u64,
) -> TerrainSummary {
    let diagnostics = data.diagnostics.clone();
    TerrainSummary {
        mode: TerrainSummaryMode::RawProperties,
        manifest: Some(path_to_project_string(
            project_dir,
            &canonical_terrain_manifest(project_dir, &data.terrain_path),
        )),
        raw_payloads: data.voxel_properties.len(),
        chunk_count: None,
        bytes_read,
        bytes_written,
        diagnostic_count: diagnostics.len(),
        diagnostics,
    }
}

fn read_chunk_terrain_data(path: &Path) -> Result<ChunkTerrainData> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read terrain chunk data {}", path.display()))?;
    let raw = serde_json::from_str::<Value>(&content)
        .with_context(|| format!("Failed to parse terrain chunk data {}", path.display()))?;
    let chunk_count = raw
        .get("chunks")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    Ok(ChunkTerrainData {
        manifest: path.to_string_lossy().to_string(),
        chunk_count,
        raw,
    })
}

fn terrain_metadata_property(variant: &Variant) -> Option<Value> {
    Some(match variant {
        Variant::Bool(value) => serde_json::json!({ "type": "bool", "value": value }),
        Variant::Int32(value) => serde_json::json!({ "type": "int", "value": value }),
        Variant::Int64(value) => serde_json::json!({ "type": "int64", "value": value }),
        Variant::Float32(value) => serde_json::json!({ "type": "float", "value": value }),
        Variant::Float64(value) => serde_json::json!({ "type": "double", "value": value }),
        Variant::String(value) => serde_json::json!({ "type": "string", "value": value }),
        Variant::Vector3(value) => serde_json::json!({
            "type": "Vector3",
            "value": { "x": value.x, "y": value.y, "z": value.z }
        }),
        Variant::Color3(value) => serde_json::json!({
            "type": "Color3",
            "value": { "r": value.r, "g": value.g, "b": value.b }
        }),
        Variant::Enum(value) => serde_json::json!({
            "type": "Enum",
            "value": { "enumType": Value::Null, "value": value.to_u32() }
        }),
        _ => return None,
    })
}

fn path_to_project_string(project_dir: &Path, path: &Path) -> String {
    path.strip_prefix(project_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbx_dom_weak::types::SharedString;
    use rbx_dom_weak::{InstanceBuilder, WeakDom};

    #[test]
    fn builds_canonical_and_legacy_paths() {
        let project = Path::new("/tmp/project");
        assert_eq!(
            canonical_terrain_manifest(project, "Workspace/Terrain"),
            project.join("terrain/Workspace/Terrain.rbxterrain.json")
        );
        assert_eq!(
            raw_terrain_manifest_relative_path("Workspace/Terrain"),
            "terrain/Workspace/Terrain.rbxterrain.json"
        );
        assert_eq!(
            legacy_terrain_chunk_file(project),
            project.join("src/Workspace/Terrain/terrain.rbxjson")
        );
    }

    #[test]
    fn studio_sync_lookup_prefers_legacy_chunks_over_raw_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        let legacy_path = legacy_terrain_chunk_file(project);
        std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        std::fs::write(&legacy_path, r#"{"chunks":[]}"#).unwrap();

        let raw_path = canonical_terrain_manifest(project, "Workspace/Terrain");
        std::fs::create_dir_all(raw_path.parent().unwrap()).unwrap();
        std::fs::write(&raw_path, "{}").unwrap();

        let terrain_file = find_studio_sync_terrain_file(project).unwrap();
        assert_eq!(terrain_file.kind, TerrainProjectFileKind::Chunks);
        assert_eq!(
            terrain_file.project_relative_path,
            "src/Workspace/Terrain/terrain.rbxjson"
        );
    }

    #[test]
    fn studio_sync_lookup_detects_raw_manifest_when_no_chunks_exist() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        let raw_path = canonical_terrain_manifest(project, "Workspace/Terrain");
        std::fs::create_dir_all(raw_path.parent().unwrap()).unwrap();
        std::fs::write(&raw_path, "{}").unwrap();

        let terrain_file = find_studio_sync_terrain_file(project).unwrap();
        assert_eq!(terrain_file.kind, TerrainProjectFileKind::RawProperties);
        assert_eq!(
            terrain_file.project_relative_path,
            "terrain/Workspace/Terrain.rbxterrain.json"
        );
    }

    #[test]
    fn writes_and_reads_raw_manifest_and_payload() {
        let temp = tempfile::tempdir().unwrap();
        let bytes = vec![1, 2, 3, 4, 5];
        let mut payload = write_terrain_blob(temp.path(), &bytes).unwrap();
        payload.property_type = TerrainPayloadType::BinaryString;

        let mut voxel_properties = BTreeMap::new();
        voxel_properties.insert("SmoothGrid".to_string(), payload.clone());
        let data = RawTerrainData {
            version: 0,
            format: TerrainDataFormat::RawProperties,
            terrain_path: "Workspace/Terrain".to_string(),
            class_name: "Terrain".to_string(),
            name: "Terrain".to_string(),
            reference_id: Some("terrain-ref".to_string()),
            metadata_properties: BTreeMap::new(),
            material_colors: BTreeMap::new(),
            voxel_properties,
            diagnostics: Vec::new(),
        };

        let summary = write_raw_terrain_data(temp.path(), &data).unwrap();
        assert_eq!(summary.mode, TerrainSummaryMode::RawProperties);
        assert_eq!(summary.raw_payloads, 1);
        assert_eq!(
            summary.manifest.as_deref(),
            Some("terrain/Workspace/Terrain.rbxterrain.json")
        );

        let read = read_terrain_project_data(temp.path()).unwrap().unwrap();
        let TerrainProjectData::Raw(read) = read else {
            panic!("expected raw terrain data");
        };
        assert_eq!(read.version, TERRAIN_MANIFEST_VERSION);
        assert_eq!(read.voxel_properties["SmoothGrid"], payload);
        assert_eq!(
            read_terrain_payload(temp.path(), &read.voxel_properties["SmoothGrid"]).unwrap(),
            bytes
        );
    }

    #[test]
    fn rejects_payload_hash_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let mut payload = write_terrain_blob(temp.path(), &[9, 8, 7]).unwrap();
        std::fs::write(temp.path().join(&payload.file), [1, 2, 3]).unwrap();
        payload.sha256 = asset_sha256_hex(&[9, 8, 7]);

        let error = read_terrain_payload(temp.path(), &payload).unwrap_err();
        assert!(
            error.root_cause().to_string().contains("sha256 mismatch"),
            "unexpected error: {error:?}"
        );
    }

    #[test]
    fn reads_legacy_chunk_data() {
        let temp = tempfile::tempdir().unwrap();
        let path = legacy_terrain_chunk_file(temp.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{
  "chunkSize": 32,
  "resolution": 4,
  "chunks": [
    { "x": 0, "y": 0, "z": 0, "materials": [1, 2], "occupancies": [255] },
    { "x": 1, "y": 0, "z": 0, "materials": [3, 4], "occupancies": [128] }
  ],
  "properties": {}
}"#,
        )
        .unwrap();

        let read = read_terrain_project_data(temp.path()).unwrap().unwrap();
        let TerrainProjectData::Chunks(chunks) = read else {
            panic!("expected chunk terrain data");
        };
        assert_eq!(chunks.chunk_count, 2);
        assert_eq!(chunks.raw["resolution"], 4);
    }

    #[test]
    fn extracts_raw_payloads_from_terrain_instance() {
        let temp = tempfile::tempdir().unwrap();
        let mut dom = WeakDom::new(InstanceBuilder::new("DataModel").with_name("game"));
        let terrain_ref = dom.insert(
            dom.root_ref(),
            InstanceBuilder::new("Terrain")
                .with_name("Terrain")
                .with_property("SmoothGrid", BinaryString::from(vec![1, 2, 3]))
                .with_property("Decoration", true)
                .with_property("SharedVoxelData", SharedString::new(vec![4, 5, 6])),
        );
        let instance = dom.get_by_ref(terrain_ref).unwrap();

        let raw = extract_raw_terrain_from_instance(&instance, "Workspace/Terrain", temp.path())
            .unwrap()
            .unwrap();

        assert_eq!(raw.voxel_properties.len(), 2);
        assert_eq!(
            read_terrain_payload(temp.path(), &raw.voxel_properties["SmoothGrid"]).unwrap(),
            vec![1, 2, 3]
        );
        assert_eq!(
            read_terrain_payload(temp.path(), &raw.voxel_properties["SharedVoxelData"]).unwrap(),
            vec![4, 5, 6]
        );
        assert_eq!(raw.metadata_properties["Decoration"]["value"], true);
    }

    #[test]
    fn roblox_binary_and_xml_preserve_opaque_terrain_payloads() {
        for format in ["rbxl", "rbxlx"] {
            let mut dom = WeakDom::new(InstanceBuilder::new("DataModel").with_name("game"));
            let root = dom.root_ref();
            let workspace = dom.insert(root, InstanceBuilder::new("Workspace"));
            dom.insert(
                workspace,
                InstanceBuilder::new("Terrain")
                    .with_name("Terrain")
                    .with_property("SmoothGrid", BinaryString::from(vec![1, 2, 3, 4]))
                    .with_property("SharedVoxelData", SharedString::new(vec![5, 6, 7, 8])),
            );
            let refs = dom.root().children().to_vec();
            let mut buffer = Vec::new();

            match format {
                "rbxl" => rbx_binary::to_writer(&mut buffer, &dom, &refs).unwrap(),
                "rbxlx" => rbx_xml::to_writer_default(&mut buffer, &dom, &refs).unwrap(),
                _ => unreachable!(),
            }

            let decoded = match format {
                "rbxl" => rbx_binary::from_reader(buffer.as_slice()).unwrap(),
                "rbxlx" => rbx_xml::from_reader_default(buffer.as_slice()).unwrap(),
                _ => unreachable!(),
            };
            let workspace_ref = decoded.root().children()[0];
            let workspace = decoded.get_by_ref(workspace_ref).unwrap();
            let terrain = decoded.get_by_ref(workspace.children()[0]).unwrap();

            assert_eq!(terrain.class, "Terrain");
            assert_eq!(
                terrain
                    .properties
                    .get("SmoothGrid")
                    .and_then(|variant| match variant {
                        Variant::BinaryString(value) => {
                            Some(<BinaryString as AsRef<[u8]>>::as_ref(value).to_vec())
                        }
                        _ => None,
                    }),
                Some(vec![1, 2, 3, 4]),
                "format {format} did not preserve BinaryString terrain payload"
            );
            if format == "rbxl" {
                assert_eq!(
                    terrain
                        .properties
                        .get("SharedVoxelData")
                        .and_then(|variant| match variant {
                            Variant::SharedString(value) => Some(value.data().to_vec()),
                            _ => None,
                        }),
                    Some(vec![5, 6, 7, 8]),
                    "format {format} did not preserve SharedString terrain payload"
                );
            }
        }
    }
}
