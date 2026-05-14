//! Shared asset manifest and discovery helpers.
//!
//! This module owns the repository-neutral asset model used by future
//! `import-place` and `extract-place` asset handling. Milestone 1 only scans
//! serialized instance JSON and reads/writes deterministic manifests; later
//! milestones will use these types to extract and embed local asset payloads.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Current manifest schema version.
pub const ASSET_MANIFEST_VERSION: u32 = 1;

/// How a command should handle asset-like values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetMode {
    /// Preserve values exactly and do not read or write local asset files.
    ReferencesOnly,
    /// Create or consume local manifest and blob files where supported.
    IncludeLocal,
    /// Ignore asset manifests and leave metadata inline.
    Disabled,
}

impl Default for AssetMode {
    fn default() -> Self {
        Self::ReferencesOnly
    }
}

/// Manifest file stored at `assets/manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetManifest {
    pub version: u32,
    pub generated_by: String,
    pub entries: Vec<AssetEntry>,
}

/// One asset-like property discovered in serialized instance metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetEntry {
    pub id: String,
    pub kind: AssetKind,
    pub source: AssetSource,
    pub instance_path: String,
    pub property: String,
    pub original: Option<String>,
    pub file: Option<String>,
    pub sha256: Option<String>,
    pub byte_length: Option<u64>,
    pub status: AssetStatus,
}

/// Asset-like property category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Content,
    BinaryString,
    SharedString,
}

/// Where the asset data currently lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetSource {
    ExternalReference,
    EmbeddedData,
    LocalFile,
}

/// Manifest entry lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetStatus {
    ReferencedOnly,
    EmbeddedInline,
    FileBacked,
    Extracted,
}

/// Aggregate asset counts for CLI summaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSummary {
    pub mode: AssetMode,
    pub manifest: Option<String>,
    pub content_references: usize,
    pub embedded_payloads: usize,
    pub files_written: usize,
    pub bytes_written: u64,
}

/// Result of extracting embedded asset payloads from serialized instances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetExtractionResult {
    pub instances: Vec<Value>,
    pub manifest: AssetManifest,
    pub summary: AssetSummary,
}

/// Category of failure while resolving or validating a local asset payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetFileErrorKind {
    Missing,
    OutsideProject,
    HashMismatch,
}

/// Error returned when a local asset payload cannot be safely read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFileError {
    kind: AssetFileErrorKind,
    message: String,
}

impl AssetFileError {
    pub fn kind(&self) -> AssetFileErrorKind {
        self.kind
    }
}

impl fmt::Display for AssetFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AssetFileError {}

/// Discover asset-like properties from serialized instances.
pub fn discover_assets(instances: &[Value]) -> Vec<AssetEntry> {
    let mut entries = Vec::new();

    for instance in instances {
        let instance_path = instance
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(properties) = instance.get("properties").and_then(Value::as_object) else {
            continue;
        };

        let mut property_names: Vec<_> = properties.keys().collect();
        property_names.sort();

        for property in property_names {
            let Some(property_value) = properties.get(property) else {
                continue;
            };
            if let Some(entry) = discover_property(instance_path, property, property_value) {
                entries.push(entry);
            }
        }
    }

    sort_entries(&mut entries);
    entries
}

/// Write embedded payloads to `assets/blobs/`, write `assets/manifest.json`,
/// and rewrite payload properties to file-backed metadata.
pub fn extract_embedded_assets(
    mut instances: Vec<Value>,
    project_dir: &Path,
    generated_by: impl Into<String>,
) -> Result<AssetExtractionResult> {
    let assets_dir = project_dir.join("assets");
    let blobs_dir = assets_dir.join("blobs");
    let manifest_path = assets_dir.join("manifest.json");
    let mut entries = Vec::new();
    let mut files_written = 0;
    let mut bytes_written = 0;

    for instance in &mut instances {
        let instance_path = instance
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let Some(properties) = instance
            .get_mut("properties")
            .and_then(Value::as_object_mut)
        else {
            continue;
        };

        let mut property_names: Vec<_> = properties.keys().cloned().collect();
        property_names.sort();

        for property in property_names {
            let Some(property_value) = properties.get_mut(&property) else {
                continue;
            };
            let Some(entry) = extract_property(
                &instance_path,
                &property,
                property_value,
                &blobs_dir,
                &mut files_written,
                &mut bytes_written,
            )?
            else {
                continue;
            };
            entries.push(entry);
        }
    }

    sort_entries(&mut entries);
    let manifest = build_asset_manifest(generated_by, entries.clone());
    write_asset_manifest(&manifest_path, &manifest)?;
    let summary = summarize_assets(
        AssetMode::IncludeLocal,
        Some("assets/manifest.json".to_string()),
        &entries,
        files_written,
        bytes_written,
    );

    Ok(AssetExtractionResult {
        instances,
        manifest,
        summary,
    })
}

/// Build a deterministic manifest from entries.
pub fn build_asset_manifest(
    generated_by: impl Into<String>,
    mut entries: Vec<AssetEntry>,
) -> AssetManifest {
    sort_entries(&mut entries);
    AssetManifest {
        version: ASSET_MANIFEST_VERSION,
        generated_by: generated_by.into(),
        entries,
    }
}

/// Load and normalize an asset manifest.
pub fn load_asset_manifest(path: &Path) -> Result<AssetManifest> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read asset manifest {}", path.display()))?;
    let mut manifest: AssetManifest = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse asset manifest {}", path.display()))?;
    sort_entries(&mut manifest.entries);
    Ok(manifest)
}

/// Write a pretty, deterministic asset manifest.
pub fn write_asset_manifest(path: &Path, manifest: &AssetManifest) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create asset manifest directory {}",
                parent.display()
            )
        })?;
    }

    let mut normalized = manifest.clone();
    sort_entries(&mut normalized.entries);
    let content = serde_json::to_string_pretty(&normalized)? + "\n";
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write asset manifest {}", path.display()))?;
    Ok(())
}

/// Summarize entries for command output.
pub fn summarize_assets(
    mode: AssetMode,
    manifest: Option<String>,
    entries: &[AssetEntry],
    files_written: usize,
    bytes_written: u64,
) -> AssetSummary {
    AssetSummary {
        mode,
        manifest,
        content_references: entries
            .iter()
            .filter(|entry| entry.kind == AssetKind::Content)
            .count(),
        embedded_payloads: entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.source,
                    AssetSource::EmbeddedData | AssetSource::LocalFile
                )
            })
            .count(),
        files_written,
        bytes_written,
    }
}

/// Resolve a project-relative asset path and verify it stays under the project.
pub fn resolve_asset_file(
    project_dir: &Path,
    file: &str,
) -> std::result::Result<PathBuf, AssetFileError> {
    let file_path = PathBuf::from(file);
    if file_path.is_absolute() {
        return Err(asset_file_error(
            AssetFileErrorKind::OutsideProject,
            format!("Asset file path '{}' must be relative to the project", file),
        ));
    }

    let project_root = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());
    let candidate = project_dir.join(&file_path);
    if !candidate.exists() {
        return Err(asset_file_error(
            AssetFileErrorKind::Missing,
            format!("Asset file '{}' was not found", candidate.display()),
        ));
    }

    let canonical = candidate.canonicalize().map_err(|error| {
        asset_file_error(
            AssetFileErrorKind::Missing,
            format!(
                "Failed to resolve asset file '{}': {}",
                candidate.display(),
                error
            ),
        )
    })?;

    if !canonical.starts_with(&project_root) {
        return Err(asset_file_error(
            AssetFileErrorKind::OutsideProject,
            format!(
                "Asset file '{}' resolves outside project '{}'",
                canonical.display(),
                project_root.display()
            ),
        ));
    }

    Ok(canonical)
}

/// Compute the SHA-256 digest used for content-addressed asset blob names.
pub fn asset_sha256_hex(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

/// Read a project-relative asset payload and verify an optional SHA-256 digest.
pub fn read_asset_file(
    project_dir: &Path,
    file: &str,
    expected_sha256: Option<&str>,
) -> std::result::Result<Vec<u8>, AssetFileError> {
    let canonical = resolve_asset_file(project_dir, file)?;
    let bytes = std::fs::read(&canonical).map_err(|error| {
        asset_file_error(
            AssetFileErrorKind::Missing,
            format!(
                "Failed to read asset file '{}': {}",
                canonical.display(),
                error
            ),
        )
    })?;

    if let Some(expected) = expected_sha256 {
        let actual = sha256_hex(&bytes);
        if actual != expected {
            return Err(asset_file_error(
                AssetFileErrorKind::HashMismatch,
                format!(
                    "Asset file '{}' sha256 mismatch: expected {}, got {}",
                    canonical.display(),
                    expected,
                    actual
                ),
            ));
        }
    }

    Ok(bytes)
}

fn discover_property(
    instance_path: &str,
    property: &str,
    property_value: &Value,
) -> Option<AssetEntry> {
    let property_type = property_value.get("type").and_then(Value::as_str)?;
    let value = property_value.get("value").unwrap_or(&Value::Null);

    match property_type {
        "Content" => value.as_str().map(|original| AssetEntry {
            id: format!("content:{}:{}", instance_path, property),
            kind: AssetKind::Content,
            source: AssetSource::ExternalReference,
            instance_path: instance_path.to_string(),
            property: property.to_string(),
            original: Some(original.to_string()),
            file: None,
            sha256: None,
            byte_length: None,
            status: AssetStatus::ReferencedOnly,
        }),
        "BinaryString" => binary_entry(instance_path, property, value),
        "SharedString" => shared_string_entry(instance_path, property, value),
        _ => None,
    }
}

fn extract_property(
    instance_path: &str,
    property: &str,
    property_value: &mut Value,
    blobs_dir: &Path,
    files_written: &mut usize,
    bytes_written: &mut u64,
) -> Result<Option<AssetEntry>> {
    let property_type = property_value
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(property_type) = property_type else {
        return Ok(None);
    };

    match property_type.as_str() {
        "Content" => Ok(discover_property(instance_path, property, property_value)),
        "BinaryString" => extract_binary_string(
            instance_path,
            property,
            property_value,
            blobs_dir,
            files_written,
            bytes_written,
        )
        .map(Some),
        "SharedString" => extract_shared_string(
            instance_path,
            property,
            property_value,
            blobs_dir,
            files_written,
            bytes_written,
        )
        .map(Some),
        _ => Ok(None),
    }
}

fn extract_binary_string(
    instance_path: &str,
    property: &str,
    property_value: &mut Value,
    blobs_dir: &Path,
    files_written: &mut usize,
    bytes_written: &mut u64,
) -> Result<AssetEntry> {
    let value = property_value.get("value").unwrap_or(&Value::Null);

    if value.is_object() {
        return discover_property(instance_path, property, property_value).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid file-backed BinaryString property {}.{}",
                instance_path,
                property
            )
        });
    }

    let encoded = value.as_str().ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid BinaryString property {}.{}: value must be base64 string",
            instance_path,
            property
        )
    })?;
    let bytes = general_purpose::STANDARD.decode(encoded).with_context(|| {
        format!(
            "Invalid BinaryString property {}.{}: value is not valid base64",
            instance_path, property
        )
    })?;
    let (file, digest, byte_length, wrote_file) = write_blob(blobs_dir, &bytes)?;
    if wrote_file {
        *files_written += 1;
        *bytes_written += byte_length;
    }

    property_value["value"] = serde_json::json!({
        "file": file,
        "encoding": "raw",
        "sha256": digest,
        "byteLength": byte_length,
    });

    Ok(extracted_entry(
        AssetKind::BinaryString,
        instance_path,
        property,
        None,
        file,
        digest,
        byte_length,
    ))
}

fn extract_shared_string(
    instance_path: &str,
    property: &str,
    property_value: &mut Value,
    blobs_dir: &Path,
    files_written: &mut usize,
    bytes_written: &mut u64,
) -> Result<AssetEntry> {
    let value = property_value.get_mut("value").ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid SharedString property {}.{}: missing value",
            instance_path,
            property
        )
    })?;
    let object = value.as_object_mut().ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid SharedString property {}.{}: value must be an object",
            instance_path,
            property
        )
    })?;

    if !object.contains_key("data") {
        return discover_property(instance_path, property, property_value).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid file-backed SharedString property {}.{}",
                instance_path,
                property
            )
        });
    }

    let original = object
        .get("hash")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let encoded = object.get("data").and_then(Value::as_str).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid SharedString property {}.{}: data must be base64 string",
            instance_path,
            property
        )
    })?;
    let bytes = general_purpose::STANDARD.decode(encoded).with_context(|| {
        format!(
            "Invalid SharedString property {}.{}: data is not valid base64",
            instance_path, property
        )
    })?;
    let (file, digest, byte_length, wrote_file) = write_blob(blobs_dir, &bytes)?;
    if wrote_file {
        *files_written += 1;
        *bytes_written += byte_length;
    }

    object.remove("data");
    object.insert("file".to_string(), Value::String(file.clone()));
    object.insert("sha256".to_string(), Value::String(digest.clone()));
    object.insert(
        "byteLength".to_string(),
        Value::Number(serde_json::Number::from(byte_length)),
    );

    Ok(extracted_entry(
        AssetKind::SharedString,
        instance_path,
        property,
        original,
        file,
        digest,
        byte_length,
    ))
}

fn binary_entry(instance_path: &str, property: &str, value: &Value) -> Option<AssetEntry> {
    if let Some(encoded) = value.as_str() {
        let decoded = general_purpose::STANDARD.decode(encoded).ok()?;
        let digest = sha256_hex(&decoded);
        return Some(embedded_entry(
            AssetKind::BinaryString,
            instance_path,
            property,
            None,
            digest,
            decoded.len() as u64,
        ));
    }

    let object = value.as_object()?;
    let file = object.get("file").and_then(Value::as_str)?;
    Some(file_backed_entry(
        AssetKind::BinaryString,
        instance_path,
        property,
        None,
        file,
        object.get("sha256").and_then(Value::as_str),
        object.get("byteLength").and_then(Value::as_u64),
    ))
}

fn shared_string_entry(instance_path: &str, property: &str, value: &Value) -> Option<AssetEntry> {
    let object = value.as_object()?;
    let original = object
        .get("hash")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    if let Some(encoded) = object.get("data").and_then(Value::as_str) {
        let decoded = general_purpose::STANDARD.decode(encoded).ok()?;
        let digest = sha256_hex(&decoded);
        return Some(embedded_entry(
            AssetKind::SharedString,
            instance_path,
            property,
            original,
            digest,
            decoded.len() as u64,
        ));
    }

    let file = object.get("file").and_then(Value::as_str)?;
    Some(file_backed_entry(
        AssetKind::SharedString,
        instance_path,
        property,
        original,
        file,
        object.get("sha256").and_then(Value::as_str),
        object.get("byteLength").and_then(Value::as_u64),
    ))
}

fn extracted_entry(
    kind: AssetKind,
    instance_path: &str,
    property: &str,
    original: Option<String>,
    file: String,
    digest: String,
    byte_length: u64,
) -> AssetEntry {
    AssetEntry {
        id: format!("sha256:{}", digest),
        kind,
        source: AssetSource::LocalFile,
        instance_path: instance_path.to_string(),
        property: property.to_string(),
        original,
        file: Some(file),
        sha256: Some(digest),
        byte_length: Some(byte_length),
        status: AssetStatus::Extracted,
    }
}

fn embedded_entry(
    kind: AssetKind,
    instance_path: &str,
    property: &str,
    original: Option<String>,
    digest: String,
    byte_length: u64,
) -> AssetEntry {
    AssetEntry {
        id: format!("sha256:{}", digest),
        kind,
        source: AssetSource::EmbeddedData,
        instance_path: instance_path.to_string(),
        property: property.to_string(),
        original,
        file: None,
        sha256: Some(digest),
        byte_length: Some(byte_length),
        status: AssetStatus::EmbeddedInline,
    }
}

fn file_backed_entry(
    kind: AssetKind,
    instance_path: &str,
    property: &str,
    original: Option<String>,
    file: &str,
    sha256: Option<&str>,
    byte_length: Option<u64>,
) -> AssetEntry {
    let id = sha256
        .map(|digest| format!("sha256:{}", digest))
        .unwrap_or_else(|| format!("file:{}:{}:{}", instance_path, property, file));

    AssetEntry {
        id,
        kind,
        source: AssetSource::LocalFile,
        instance_path: instance_path.to_string(),
        property: property.to_string(),
        original,
        file: Some(file.replace('\\', "/")),
        sha256: sha256.map(ToString::to_string),
        byte_length,
        status: AssetStatus::FileBacked,
    }
}

fn sort_entries(entries: &mut [AssetEntry]) {
    entries.sort_by(|a, b| {
        (&a.instance_path, &a.property, a.kind, &a.id, &a.file).cmp(&(
            &b.instance_path,
            &b.property,
            b.kind,
            &b.id,
            &b.file,
        ))
    });
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{:02x}", byte));
    }
    hex
}

fn asset_file_error(kind: AssetFileErrorKind, message: String) -> AssetFileError {
    AssetFileError { kind, message }
}

fn write_blob(blobs_dir: &Path, bytes: &[u8]) -> Result<(String, String, u64, bool)> {
    let digest = sha256_hex(bytes);
    let relative_file = format!("assets/blobs/{}.bin", digest);
    let blob_path = blobs_dir.join(format!("{}.bin", digest));

    let wrote_file = if blob_path.exists() {
        false
    } else {
        std::fs::create_dir_all(blobs_dir).with_context(|| {
            format!(
                "Failed to create asset blob directory {}",
                blobs_dir.display()
            )
        })?;
        std::fs::write(&blob_path, bytes)
            .with_context(|| format!("Failed to write asset blob {}", blob_path.display()))?;
        true
    };

    let byte_length =
        u64::try_from(bytes.len()).context("Asset blob is too large to record in manifest")?;

    Ok((relative_file, digest, byte_length, wrote_file))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn discovers_asset_properties_in_deterministic_order() {
        let instances = vec![json!({
            "path": "Workspace/Sound",
            "properties": {
                "SharedData": {
                    "type": "SharedString",
                    "value": {
                        "hash": "shared-hash",
                        "file": null,
                        "data": "BQYHCA=="
                    }
                },
                "SoundId": { "type": "Content", "value": "rbxassetid://123456" },
                "BinaryData": { "type": "BinaryString", "value": "AQIDBA==" },
                "Name": { "type": "string", "value": "Ignored" }
            }
        })];

        let first = discover_assets(&instances);
        let second = discover_assets(&instances);

        assert_eq!(first, second);
        assert_eq!(first.len(), 3);
        assert_eq!(first[0].property, "BinaryData");
        assert_eq!(first[0].kind, AssetKind::BinaryString);
        assert_eq!(first[0].source, AssetSource::EmbeddedData);
        assert_eq!(first[0].byte_length, Some(4));
        assert!(first[0].sha256.as_ref().unwrap().len() == 64);
        assert_eq!(first[1].property, "SharedData");
        assert_eq!(first[1].original.as_deref(), Some("shared-hash"));
        assert_eq!(first[2].property, "SoundId");
        assert_eq!(first[2].original.as_deref(), Some("rbxassetid://123456"));
        assert_eq!(first[2].status, AssetStatus::ReferencedOnly);
    }

    #[test]
    fn discovers_file_backed_payloads_with_normalized_paths() {
        let entries = discover_assets(&[json!({
            "path": "Workspace/AssetHolder",
            "properties": {
                "BinaryData": {
                    "type": "BinaryString",
                    "value": {
                        "file": "assets\\blobs\\blob.bin",
                        "sha256": "digest",
                        "byteLength": 4
                    }
                }
            }
        })]);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, AssetSource::LocalFile);
        assert_eq!(entries[0].status, AssetStatus::FileBacked);
        assert_eq!(entries[0].file.as_deref(), Some("assets/blobs/blob.bin"));
        assert_eq!(entries[0].sha256.as_deref(), Some("digest"));
    }

    #[test]
    fn sha256_blob_names_are_stable() {
        let temp = tempfile::tempdir().unwrap();
        let instances = vec![json!({
            "path": "Workspace/AssetHolder",
            "properties": {
                "BinaryData": { "type": "BinaryString", "value": "AQIDBA==" }
            }
        })];
        let expected = "9f64a747e1b97f131fabb6b447296c9b6f0201e79fb3c5356e6c77e89b6a806a";

        let result = extract_embedded_assets(instances, temp.path(), "test").unwrap();

        let value = &result.instances[0]["properties"]["BinaryData"]["value"];
        assert_eq!(asset_sha256_hex(&[1, 2, 3, 4]), expected);
        assert_eq!(value["sha256"], expected);
        assert_eq!(value["file"], format!("assets/blobs/{}.bin", expected));
        assert!(temp
            .path()
            .join(format!("assets/blobs/{}.bin", expected))
            .exists());
    }

    #[test]
    fn builds_and_serializes_sorted_manifest() {
        let mut entries = vec![
            AssetEntry {
                id: "content:Workspace/Sound:SoundId".to_string(),
                kind: AssetKind::Content,
                source: AssetSource::ExternalReference,
                instance_path: "Workspace/Sound".to_string(),
                property: "SoundId".to_string(),
                original: Some("rbxassetid://123456".to_string()),
                file: None,
                sha256: None,
                byte_length: None,
                status: AssetStatus::ReferencedOnly,
            },
            AssetEntry {
                id: "sha256:bbbb".to_string(),
                kind: AssetKind::BinaryString,
                source: AssetSource::EmbeddedData,
                instance_path: "Workspace/A".to_string(),
                property: "Blob".to_string(),
                original: None,
                file: None,
                sha256: Some("bbbb".to_string()),
                byte_length: Some(2),
                status: AssetStatus::EmbeddedInline,
            },
        ];
        entries.reverse();

        let manifest = build_asset_manifest("test", entries);
        let serialized = serde_json::to_string_pretty(&manifest).unwrap();

        assert_eq!(manifest.version, ASSET_MANIFEST_VERSION);
        assert_eq!(manifest.entries[0].instance_path, "Workspace/A");
        assert!(serialized.contains("\"generatedBy\": \"test\""));
        assert!(serialized.contains("\"referencedOnly\""));
    }

    #[test]
    fn reads_and_writes_manifest_with_normalized_order() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("assets/manifest.json");
        let entries = discover_assets(&[json!({
            "path": "Workspace/Sound",
            "properties": {
                "SoundId": { "type": "Content", "value": "rbxassetid://123456" },
                "BinaryData": { "type": "BinaryString", "value": "AQIDBA==" }
            }
        })]);
        let manifest = build_asset_manifest("rbxsync import-place", entries.clone());

        write_asset_manifest(&path, &manifest).unwrap();
        let loaded = load_asset_manifest(&path).unwrap();

        assert_eq!(loaded, manifest);
        assert!(std::fs::read_to_string(path).unwrap().ends_with('\n'));
    }

    #[test]
    fn summarizes_asset_counts() {
        let entries = discover_assets(&[json!({
            "path": "Workspace/Sound",
            "properties": {
                "SoundId": { "type": "Content", "value": "rbxassetid://123456" },
                "BinaryData": { "type": "BinaryString", "value": "AQIDBA==" },
                "SharedData": {
                    "type": "SharedString",
                    "value": {
                        "hash": "shared-hash",
                        "file": "assets/blobs/blob.bin",
                        "sha256": "abc",
                        "byteLength": 3
                    }
                }
            }
        })]);

        let summary = summarize_assets(
            AssetMode::IncludeLocal,
            Some("assets/manifest.json".to_string()),
            &entries,
            1,
            4,
        );

        assert_eq!(summary.mode, AssetMode::IncludeLocal);
        assert_eq!(summary.content_references, 1);
        assert_eq!(summary.embedded_payloads, 2);
        assert_eq!(summary.files_written, 1);
        assert_eq!(summary.bytes_written, 4);
    }

    #[test]
    fn extracts_embedded_payloads_and_rewrites_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let instances = vec![json!({
            "path": "Workspace/AssetHolder",
            "properties": {
                "SoundId": { "type": "Content", "value": "rbxassetid://123456" },
                "BinaryData": { "type": "BinaryString", "value": "AQIDBA==" },
                "SharedData": {
                    "type": "SharedString",
                    "value": {
                        "hash": "shared-hash",
                        "file": null,
                        "data": "BQYHCA=="
                    }
                }
            }
        })];

        let result =
            extract_embedded_assets(instances, temp.path(), "rbxsync import-place").unwrap();

        assert_eq!(result.manifest.version, ASSET_MANIFEST_VERSION);
        assert_eq!(result.manifest.entries.len(), 3);
        assert_eq!(result.summary.content_references, 1);
        assert_eq!(result.summary.embedded_payloads, 2);
        assert_eq!(result.summary.files_written, 2);
        assert_eq!(result.summary.bytes_written, 8);
        assert!(temp.path().join("assets/manifest.json").exists());

        let binary_value = &result.instances[0]["properties"]["BinaryData"]["value"];
        let binary_file = binary_value["file"].as_str().unwrap();
        assert!(binary_file.starts_with("assets/blobs/"));
        assert_eq!(binary_value["encoding"], "raw");
        assert_eq!(binary_value["byteLength"], 4);
        assert!(temp.path().join(binary_file).exists());
        assert_eq!(
            std::fs::read(temp.path().join(binary_file)).unwrap(),
            vec![1, 2, 3, 4]
        );

        let shared_value = &result.instances[0]["properties"]["SharedData"]["value"];
        let shared_file = shared_value["file"].as_str().unwrap();
        assert_eq!(shared_value["hash"], "shared-hash");
        assert!(shared_value.get("data").is_none());
        assert_eq!(
            std::fs::read(temp.path().join(shared_file)).unwrap(),
            vec![5, 6, 7, 8]
        );

        let content_value = &result.instances[0]["properties"]["SoundId"]["value"];
        assert_eq!(content_value, "rbxassetid://123456");
        assert!(result
            .manifest
            .entries
            .iter()
            .any(|entry| entry.status == AssetStatus::ReferencedOnly));
        assert!(result
            .manifest
            .entries
            .iter()
            .any(|entry| entry.status == AssetStatus::Extracted));
    }

    #[test]
    fn reuses_existing_blob_files_for_duplicate_payloads() {
        let temp = tempfile::tempdir().unwrap();
        let instances = vec![json!({
            "path": "Workspace/AssetHolder",
            "properties": {
                "First": { "type": "BinaryString", "value": "AQIDBA==" },
                "Second": { "type": "BinaryString", "value": "AQIDBA==" }
            }
        })];

        let result = extract_embedded_assets(instances, temp.path(), "test").unwrap();

        assert_eq!(result.summary.files_written, 1);
        assert_eq!(result.summary.bytes_written, 4);
        let first_file = result.instances[0]["properties"]["First"]["value"]["file"]
            .as_str()
            .unwrap();
        let second_file = result.instances[0]["properties"]["Second"]["value"]["file"]
            .as_str()
            .unwrap();
        assert_eq!(first_file, second_file);
    }

    #[test]
    fn read_asset_file_rejects_paths_outside_project() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(temp.path().join("outside.bin"), [1, 2, 3, 4]).unwrap();

        let error = read_asset_file(&project, "../outside.bin", None).unwrap_err();

        assert_eq!(error.kind(), AssetFileErrorKind::OutsideProject);
        assert!(error.to_string().contains("resolves outside project"));
    }

    #[test]
    fn read_asset_file_detects_hash_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        std::fs::create_dir_all(project.join("assets/blobs")).unwrap();
        std::fs::write(project.join("assets/blobs/blob.bin"), [1, 2, 3, 4]).unwrap();

        let error = read_asset_file(project, "assets/blobs/blob.bin", Some("0000")).unwrap_err();

        assert_eq!(error.kind(), AssetFileErrorKind::HashMismatch);
        assert!(error.to_string().contains("sha256 mismatch"));
    }
}
