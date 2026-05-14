//! Shared Roblox place/model exporter.
//!
//! This module owns the project-to-DOM build path used by CLI artifact
//! commands. It reads RbxSync project files, constructs an in-memory Roblox
//! DOM, resolves metadata references, and writes Roblox place/model artifacts.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use rbx_dom_weak::types::*;
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    is_package_path, load_asset_manifest, read_asset_file, summarize_assets, AssetFileErrorKind,
    AssetMode, AssetSummary, ProjectConfig,
};

/// Roblox artifact format produced from project files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaceExportFormat {
    Rbxl,
    Rbxlx,
    Rbxm,
    Rbxmx,
}

impl PlaceExportFormat {
    pub fn from_build_format(format: &str) -> Result<Self> {
        match format.to_ascii_lowercase().as_str() {
            "rbxl" | "place" => Ok(Self::Rbxl),
            "rbxm" | "model" => Ok(Self::Rbxm),
            "rbxlx" | "place-xml" => Ok(Self::Rbxlx),
            "rbxmx" | "model-xml" => Ok(Self::Rbxmx),
            _ => bail!(
                "Unknown format: {}. Use rbxl, rbxm, rbxlx, or rbxmx",
                format
            ),
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Rbxl => "rbxl",
            Self::Rbxlx => "rbxlx",
            Self::Rbxm => "rbxm",
            Self::Rbxmx => "rbxmx",
        }
    }

    fn is_xml(self) -> bool {
        matches!(self, Self::Rbxlx | Self::Rbxmx)
    }

    fn is_place(self) -> bool {
        matches!(self, Self::Rbxl | Self::Rbxlx)
    }
}

impl std::fmt::Display for PlaceExportFormat {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.extension())
    }
}

/// Options for exporting project files to a Roblox artifact.
#[derive(Debug, Clone)]
pub struct PlaceExportOptions {
    pub project_dir: PathBuf,
    pub source_dir: PathBuf,
    pub output_path: PathBuf,
    pub format: PlaceExportFormat,
    pub force: bool,
    pub dry_run: bool,
    pub strict: bool,
    pub services: Option<HashSet<String>>,
    pub include_packages: bool,
    pub tree_mapping: HashMap<String, String>,
    pub asset_mode: AssetMode,
}

/// Non-fatal export diagnostic category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaceExportDiagnosticKind {
    InvalidProjectConfig,
    MissingSourceTree,
    InvalidMetadataJson,
    UnsupportedProperty,
    UnsupportedAttribute,
    UnsupportedTag,
    UnresolvedReference,
    DuplicateSource,
    ClassConflict,
    AmbiguousTreeMapping,
    SkippedFile,
    SkippedPackage,
    UnsupportedTerrainVoxelData,
    OutputExists,
    PublishNotImplemented,
    MissingAssetFile,
    InvalidAssetManifest,
    AssetHashMismatch,
    AssetPathOutsideProject,
}

/// Non-fatal exporter diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceExportDiagnostic {
    pub kind: PlaceExportDiagnosticKind,
    pub path: String,
    pub property: Option<String>,
    pub message: String,
}

/// Summary of an export operation.
#[derive(Debug, Clone)]
pub struct PlaceExportSummary {
    pub output_path: PathBuf,
    pub format: PlaceExportFormat,
    pub instances: usize,
    pub scripts: usize,
    pub metadata_files: usize,
    pub services: Vec<String>,
    pub bytes_written: Option<u64>,
    pub diagnostics: Vec<PlaceExportDiagnostic>,
    pub asset_summary: Option<AssetSummary>,
}

#[derive(Debug)]
struct PendingRef {
    owner: Ref,
    property: String,
    target: Option<String>,
    path: String,
}

#[derive(Debug)]
struct BuildResult {
    dom: WeakDom,
    diagnostics: Vec<PlaceExportDiagnostic>,
}

struct DomBuilder {
    dom: WeakDom,
    options: PlaceExportOptions,
    path_refs: HashMap<String, Ref>,
    reference_refs: HashMap<String, Ref>,
    pending_refs: Vec<PendingRef>,
    diagnostics: Vec<PlaceExportDiagnostic>,
}

/// Export project files to a Roblox place/model artifact.
pub fn export_place(options: PlaceExportOptions) -> Result<PlaceExportSummary> {
    let (options, mut diagnostics) = apply_project_config(options);

    if !options.source_dir.exists() {
        diagnostics.push(PlaceExportDiagnostic {
            kind: PlaceExportDiagnosticKind::MissingSourceTree,
            path: options.source_dir.display().to_string(),
            property: None,
            message: format!(
                "Source directory not found: {}",
                options.source_dir.display()
            ),
        });
        bail!(
            "Source directory not found: {}",
            options.source_dir.display()
        );
    }

    if options.output_path.exists() && !options.force && !options.dry_run {
        diagnostics.push(PlaceExportDiagnostic {
            kind: PlaceExportDiagnosticKind::OutputExists,
            path: options.output_path.display().to_string(),
            property: None,
            message: format!(
                "Output file already exists: {}. Use --force to replace it.",
                options.output_path.display()
            ),
        });
        bail!(
            "Output file already exists: {}. Use --force to replace it.",
            options.output_path.display()
        );
    }

    diagnostics.extend(validate_tree_mapping(&options));
    let asset_summary = summarize_manifest_assets(&options, &mut diagnostics);
    let BuildResult {
        dom,
        diagnostics: build_diagnostics,
    } = build_project_dom(options.clone())?;
    diagnostics.extend(build_diagnostics);

    if let Some(diagnostic) = first_fatal_asset_diagnostic(&diagnostics) {
        bail!(
            "Export failed with asset diagnostic: {}",
            diagnostic.message
        );
    }

    if options.strict && !diagnostics.is_empty() {
        bail!(
            "Export failed in strict mode with {} diagnostic(s): {}",
            diagnostics.len(),
            diagnostics[0].message
        );
    }

    let mut summary = summarize_dom(&dom, &options, diagnostics, asset_summary);

    if !options.dry_run {
        write_dom(&dom, &options)?;
        summary.bytes_written = std::fs::metadata(&options.output_path)
            .ok()
            .map(|metadata| metadata.len());
    }

    Ok(summary)
}

/// Build a Roblox DOM from an RbxSync source tree.
pub fn build_dom_from_project(options: &PlaceExportOptions) -> Result<WeakDom> {
    Ok(build_project_dom(options.clone())?.dom)
}

fn apply_project_config(
    mut options: PlaceExportOptions,
) -> (PlaceExportOptions, Vec<PlaceExportDiagnostic>) {
    let mut diagnostics = Vec::new();
    let config_path = options.project_dir.join("rbxsync.json");

    if config_path.exists() {
        match std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|content| serde_json::from_str::<ProjectConfig>(&content).ok())
        {
            Some(config) => {
                if options.tree_mapping.is_empty() {
                    options.tree_mapping = config.tree_mapping;
                }
                if !options.source_dir.exists() {
                    options.source_dir = options.project_dir.join(config.tree);
                }
            }
            None => diagnostics.push(PlaceExportDiagnostic {
                kind: PlaceExportDiagnosticKind::InvalidProjectConfig,
                path: config_path.display().to_string(),
                property: None,
                message: "Failed to parse rbxsync.json".to_string(),
            }),
        }
    }

    (options, diagnostics)
}

fn validate_tree_mapping(options: &PlaceExportOptions) -> Vec<PlaceExportDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: HashMap<String, String> = HashMap::new();

    for (dm_path, fs_path) in &options.tree_mapping {
        if let Some(previous) = seen.insert(fs_path.clone(), dm_path.clone()) {
            diagnostics.push(PlaceExportDiagnostic {
                kind: PlaceExportDiagnosticKind::AmbiguousTreeMapping,
                path: fs_path.clone(),
                property: Some("treeMapping".to_string()),
                message: format!(
                    "Filesystem path '{}' is mapped from both '{}' and '{}'",
                    fs_path, previous, dm_path
                ),
            });
        }
    }

    diagnostics
}

fn build_project_dom(options: PlaceExportOptions) -> Result<BuildResult> {
    let root_class = if options.format.is_place() {
        "DataModel"
    } else {
        "Folder"
    };
    let root_name = if options.format.is_place() {
        "game"
    } else {
        "Model"
    };

    let dom = WeakDom::new(InstanceBuilder::new(root_class).with_name(root_name));
    let mut builder = DomBuilder {
        dom,
        options,
        path_refs: HashMap::new(),
        reference_refs: HashMap::new(),
        pending_refs: Vec::new(),
        diagnostics: Vec::new(),
    };

    let roots = discover_roots(&builder.options)?;
    let root_ref = builder.dom.root_ref();

    for (fs_path, dm_path) in roots {
        let service_name = dm_path.split('/').next().unwrap_or(dm_path.as_str());
        if !service_selected(service_name, builder.options.services.as_ref()) {
            continue;
        }
        builder.insert_mapped_directory(root_ref, &fs_path, &dm_path)?;
    }

    builder.resolve_pending_refs();

    Ok(BuildResult {
        dom: builder.dom,
        diagnostics: builder.diagnostics,
    })
}

fn discover_roots(options: &PlaceExportOptions) -> Result<Vec<(PathBuf, String)>> {
    let mut roots = Vec::new();
    let mut mapped_paths = HashSet::new();

    let mut mappings: Vec<_> = options.tree_mapping.iter().collect();
    mappings.sort_by_key(|(dm_path, _)| dm_path.len());

    for (dm_path, fs_path) in mappings {
        let abs_path = resolve_project_path(options, fs_path);
        if abs_path.exists() {
            mapped_paths.insert(abs_path.clone());
            roots.push((abs_path, dm_path.clone()));
        }
    }

    let mut entries: Vec<_> = std::fs::read_dir(&options.source_dir)
        .context("Failed to read src directory")?
        .filter_map(|entry| entry.ok())
        .collect();
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        if mapped_paths
            .iter()
            .any(|mapped| path == *mapped || path.starts_with(mapped))
        {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        roots.push((path, unescape_name(&name)));
    }

    Ok(roots)
}

fn resolve_project_path(options: &PlaceExportOptions, fs_path: &str) -> PathBuf {
    let raw = PathBuf::from(fs_path);
    if raw.is_absolute() {
        raw
    } else {
        let project_relative = options.project_dir.join(&raw);
        if project_relative.exists() {
            project_relative
        } else {
            options.source_dir.join(raw)
        }
    }
}

fn write_dom(dom: &WeakDom, options: &PlaceExportOptions) -> Result<()> {
    if let Some(parent) = options.output_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    let output_file =
        BufWriter::new(File::create(&options.output_path).context("Failed to create output file")?);
    let refs_to_export: Vec<_> = dom.root().children().to_vec();

    if options.format.is_xml() {
        rbx_xml::to_writer_default(output_file, dom, &refs_to_export)
            .context("Failed to write XML output file")?;
    } else {
        rbx_binary::to_writer(output_file, dom, &refs_to_export)
            .context("Failed to write binary output file")?;
    }

    Ok(())
}

fn summarize_dom(
    dom: &WeakDom,
    options: &PlaceExportOptions,
    diagnostics: Vec<PlaceExportDiagnostic>,
    asset_summary: Option<AssetSummary>,
) -> PlaceExportSummary {
    let mut instances = 0;
    let mut scripts = 0;
    let mut services = Vec::new();

    for child_ref in dom.root().children() {
        if let Some(instance) = dom.get_by_ref(*child_ref) {
            services.push(instance.name.clone());
            count_instance_tree(dom, *child_ref, &mut instances, &mut scripts);
        }
    }

    PlaceExportSummary {
        output_path: options.output_path.clone(),
        format: options.format,
        instances,
        scripts,
        metadata_files: count_metadata_files(&options.source_dir),
        services,
        bytes_written: None,
        diagnostics,
        asset_summary,
    }
}

fn summarize_manifest_assets(
    options: &PlaceExportOptions,
    diagnostics: &mut Vec<PlaceExportDiagnostic>,
) -> Option<AssetSummary> {
    if options.asset_mode != AssetMode::IncludeLocal {
        return None;
    }

    let manifest_path = options.project_dir.join("assets/manifest.json");
    if !manifest_path.exists() {
        return None;
    }

    match load_asset_manifest(&manifest_path) {
        Ok(manifest) => Some(summarize_assets(
            options.asset_mode,
            Some("assets/manifest.json".to_string()),
            &manifest.entries,
            0,
            0,
        )),
        Err(error) => {
            diagnostics.push(PlaceExportDiagnostic {
                kind: PlaceExportDiagnosticKind::InvalidAssetManifest,
                path: manifest_path.display().to_string(),
                property: None,
                message: error.to_string(),
            });
            None
        }
    }
}

fn first_fatal_asset_diagnostic(
    diagnostics: &[PlaceExportDiagnostic],
) -> Option<&PlaceExportDiagnostic> {
    diagnostics.iter().find(|diagnostic| {
        matches!(
            diagnostic.kind,
            PlaceExportDiagnosticKind::MissingAssetFile
                | PlaceExportDiagnosticKind::AssetHashMismatch
                | PlaceExportDiagnosticKind::AssetPathOutsideProject
        )
    })
}

fn count_instance_tree(dom: &WeakDom, referent: Ref, instances: &mut usize, scripts: &mut usize) {
    let Some(instance) = dom.get_by_ref(referent) else {
        return;
    };

    *instances += 1;
    if matches!(
        instance.class.as_str(),
        "Script" | "LocalScript" | "ModuleScript"
    ) {
        *scripts += 1;
    }

    for child_ref in instance.children() {
        count_instance_tree(dom, *child_ref, instances, scripts);
    }
}

fn count_metadata_files(source_dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(source_dir) else {
        return 0;
    };

    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                count_metadata_files(&path)
            } else if path
                .extension()
                .is_some_and(|extension| extension == "rbxjson")
            {
                1
            } else {
                0
            }
        })
        .sum()
}

impl DomBuilder {
    fn insert_mapped_directory(
        &mut self,
        root_ref: Ref,
        fs_path: &Path,
        dm_path: &str,
    ) -> Result<Ref> {
        let segments: Vec<_> = dm_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        let mut parent = root_ref;
        let mut current_path = String::new();

        for (index, segment) in segments.iter().enumerate() {
            if !current_path.is_empty() {
                current_path.push('/');
            }
            current_path.push_str(segment);

            if index + 1 == segments.len() {
                return self.insert_directory(parent, fs_path, &current_path, segment);
            }

            if let Some(existing) = self.path_refs.get(&current_path) {
                parent = *existing;
            } else {
                let class_name = if index == 0 {
                    service_class_name(segment)
                } else {
                    "Folder"
                };
                parent = self.insert_instance(
                    parent,
                    &current_path,
                    segment,
                    class_name,
                    None,
                    None,
                    None,
                );
            }
        }

        Ok(parent)
    }

    fn insert_directory(
        &mut self,
        parent: Ref,
        dir_path: &Path,
        dm_path: &str,
        fallback_name: &str,
    ) -> Result<Ref> {
        if !self.options.include_packages && is_package_path(dir_path) {
            self.diagnostics.push(PlaceExportDiagnostic {
                kind: PlaceExportDiagnosticKind::SkippedPackage,
                path: dm_path.to_string(),
                property: None,
                message: format!("Skipped package directory {}", dir_path.display()),
            });
            return Ok(parent);
        }

        if let Some(existing) = self.path_refs.get(dm_path).copied() {
            self.insert_directory_children(existing, dir_path, dm_path)?;
            return Ok(existing);
        }

        let metadata = self.read_metadata(&dir_path.join("_meta.rbxjson"), dm_path);
        let init_source = read_init_source(dir_path);
        let has_init = init_source.is_some();
        let class_name = if has_init {
            init_class(dir_path)
        } else {
            metadata_class(metadata.as_ref()).unwrap_or_else(|| {
                if dm_path.find('/').is_none() {
                    service_class_name(fallback_name)
                } else {
                    "Folder"
                }
            })
        };
        let name = metadata_name(metadata.as_ref()).unwrap_or_else(|| unescape_name(fallback_name));
        let referent = self.insert_instance(
            parent,
            dm_path,
            &name,
            class_name,
            metadata.as_ref(),
            init_source,
            None,
        );
        self.insert_directory_children(referent, dir_path, dm_path)?;
        Ok(referent)
    }

    fn insert_directory_children(
        &mut self,
        parent: Ref,
        dir_path: &Path,
        dm_path: &str,
    ) -> Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir_path)
            .with_context(|| format!("Failed to read directory {}", dir_path.display()))?
            .filter_map(|entry| entry.ok())
            .collect();
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let path = entry.path();
            let entry_name = entry.file_name().to_string_lossy().to_string();

            if entry_name == "_meta.rbxjson" || is_init_file(&entry_name) {
                continue;
            }

            if path.is_dir() {
                let child_dm_path = format!("{}/{}", dm_path, unescape_name(&entry_name));
                self.insert_directory(parent, &path, &child_dm_path, &entry_name)?;
            } else if is_luau_file(&path) {
                self.insert_script_file(parent, &path, dm_path, &entry_name)?;
            } else if path
                .extension()
                .is_some_and(|extension| extension == "rbxjson")
            {
                if has_sibling_script_file(&path) {
                    continue;
                }
                self.insert_metadata_file(parent, &path, dm_path, &entry_name);
            }
        }

        Ok(())
    }

    fn insert_script_file(
        &mut self,
        parent: Ref,
        path: &Path,
        parent_dm_path: &str,
        entry_name: &str,
    ) -> Result<Ref> {
        let (fallback_name, suffix_class) = parse_script_name(entry_name);
        let metadata_path = path.with_file_name(format!("{}.rbxjson", fallback_name));
        let metadata = self.read_metadata(
            &metadata_path,
            &format!("{}/{}", parent_dm_path, fallback_name),
        );
        let name =
            metadata_name(metadata.as_ref()).unwrap_or_else(|| unescape_name(&fallback_name));
        let dm_path = format!("{}/{}", parent_dm_path, name);
        let class_name = if let Some(metadata_class) = metadata_class(metadata.as_ref()) {
            if metadata_class != suffix_class {
                self.diagnostics.push(PlaceExportDiagnostic {
                    kind: PlaceExportDiagnosticKind::ClassConflict,
                    path: dm_path.clone(),
                    property: Some("className".to_string()),
                    message: format!(
                        "Script suffix implies class '{}' but metadata contains '{}'",
                        suffix_class, metadata_class
                    ),
                });
            }
            suffix_class
        } else {
            suffix_class
        };
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read script source {}", path.display()))?;
        Ok(self.insert_instance(
            parent,
            &dm_path,
            &name,
            class_name,
            metadata.as_ref(),
            Some(source),
            None,
        ))
    }

    fn insert_metadata_file(
        &mut self,
        parent: Ref,
        path: &Path,
        parent_dm_path: &str,
        entry_name: &str,
    ) {
        let fallback_name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| entry_name.trim_end_matches(".rbxjson").to_string());
        let metadata = self.read_metadata(path, &format!("{}/{}", parent_dm_path, fallback_name));
        let name =
            metadata_name(metadata.as_ref()).unwrap_or_else(|| unescape_name(&fallback_name));
        let dm_path = format!("{}/{}", parent_dm_path, name);
        let class_name = metadata_class(metadata.as_ref()).unwrap_or("Folder");
        self.insert_instance(
            parent,
            &dm_path,
            &name,
            class_name,
            metadata.as_ref(),
            None,
            None,
        );
    }

    fn insert_instance(
        &mut self,
        parent: Ref,
        dm_path: &str,
        name: &str,
        class_name: &str,
        metadata: Option<&Value>,
        source: Option<String>,
        reference_id: Option<String>,
    ) -> Ref {
        let mut builder = InstanceBuilder::new(class_name).with_name(name);

        if let Some(metadata) = metadata {
            builder = self.apply_metadata(builder, metadata, dm_path, source.is_some());
        }

        if let Some(source) = source {
            builder = builder.with_property("Source", Variant::String(source));
        }

        let referent = self.dom.insert(parent, builder);
        self.path_refs.insert(dm_path.to_string(), referent);

        if let Some(reference_id) =
            reference_id.or_else(|| metadata.and_then(metadata_reference_id))
        {
            self.reference_refs.insert(reference_id, referent);
        }

        referent
    }

    fn apply_metadata(
        &mut self,
        mut builder: InstanceBuilder,
        metadata: &Value,
        dm_path: &str,
        source_file_wins: bool,
    ) -> InstanceBuilder {
        if let Some(properties) = metadata
            .get("properties")
            .and_then(|value| value.as_object())
        {
            let mut names: Vec<_> = properties.keys().collect();
            names.sort();
            for property_name in names {
                let property_value = &properties[property_name];
                if property_name == "Source" && source_file_wins {
                    self.diagnostics.push(PlaceExportDiagnostic {
                        kind: PlaceExportDiagnosticKind::DuplicateSource,
                        path: dm_path.to_string(),
                        property: Some("Source".to_string()),
                        message: "Script source file overrides metadata Source property"
                            .to_string(),
                    });
                    continue;
                }
                match self.json_to_variant(property_value, dm_path, property_name) {
                    PropertyConversion::Value(value) => {
                        builder = builder.with_property(property_name, value);
                    }
                    PropertyConversion::PendingRef(target) => {
                        self.pending_refs.push(PendingRef {
                            owner: Ref::none(),
                            property: property_name.clone(),
                            target,
                            path: dm_path.to_string(),
                        });
                    }
                    PropertyConversion::Unsupported => {
                        self.diagnostics.push(PlaceExportDiagnostic {
                            kind: PlaceExportDiagnosticKind::UnsupportedProperty,
                            path: dm_path.to_string(),
                            property: Some(property_name.clone()),
                            message: format!("Unsupported property value for '{}'", property_name),
                        })
                    }
                }
            }
        }

        if let Some(attributes) = self.attributes_to_variant(metadata, dm_path) {
            builder = builder.with_property("Attributes", attributes);
        }

        if let Some(tags) = self.tags_to_variant(metadata, dm_path) {
            builder = builder.with_property("Tags", tags);
        }

        builder
    }

    fn attributes_to_variant(&mut self, metadata: &Value, dm_path: &str) -> Option<Variant> {
        let attrs = metadata.get("attributes")?.as_object()?;
        let mut attributes = Attributes::new();

        for (name, value) in attrs {
            let property_name = format!("attributes.{}", name);
            match self.json_to_variant(value, dm_path, &property_name) {
                PropertyConversion::Value(value) => {
                    attributes.insert(name.clone(), value);
                }
                PropertyConversion::PendingRef(_) | PropertyConversion::Unsupported => {
                    self.diagnostics.push(PlaceExportDiagnostic {
                        kind: PlaceExportDiagnosticKind::UnsupportedAttribute,
                        path: dm_path.to_string(),
                        property: Some(property_name),
                        message: format!("Unsupported attribute value for '{}'", name),
                    });
                }
            }
        }

        Some(Variant::Attributes(attributes))
    }

    fn tags_to_variant(&mut self, metadata: &Value, dm_path: &str) -> Option<Variant> {
        let tags_value = metadata.get("tags")?;
        let Some(tags_array) = tags_value.as_array() else {
            self.diagnostics.push(PlaceExportDiagnostic {
                kind: PlaceExportDiagnosticKind::UnsupportedTag,
                path: dm_path.to_string(),
                property: Some("tags".to_string()),
                message: "Tags must be an array of strings".to_string(),
            });
            return None;
        };

        let mut tags = Tags::new();
        for tag in tags_array {
            if let Some(tag) = tag.as_str() {
                tags.push(tag);
            } else {
                self.diagnostics.push(PlaceExportDiagnostic {
                    kind: PlaceExportDiagnosticKind::UnsupportedTag,
                    path: dm_path.to_string(),
                    property: Some("tags".to_string()),
                    message: "Skipped non-string tag".to_string(),
                });
            }
        }

        Some(Variant::Tags(tags))
    }

    fn json_to_variant(
        &mut self,
        value: &Value,
        dm_path: &str,
        property_name: &str,
    ) -> PropertyConversion {
        let Some(obj) = value.as_object() else {
            return direct_json_to_variant(value)
                .map(PropertyConversion::Value)
                .unwrap_or(PropertyConversion::Unsupported);
        };
        let Some(type_str) = obj
            .get("type")
            .and_then(|property_type| property_type.as_str())
        else {
            return direct_json_to_variant(value)
                .map(PropertyConversion::Value)
                .unwrap_or(PropertyConversion::Unsupported);
        };
        let val = obj.get("value").unwrap_or(&Value::Null);

        let converted = match type_str {
            "string" => val.as_str().map(|value| Variant::String(value.to_string())),
            "int" | "int32" => val.as_i64().map(|value| Variant::Int32(value as i32)),
            "int64" => val.as_i64().map(Variant::Int64),
            "float" | "float32" => val.as_f64().map(|value| Variant::Float32(value as f32)),
            "float64" | "double" => val.as_f64().map(Variant::Float64),
            "bool" => val.as_bool().map(Variant::Bool),
            "nil" => None,
            "Vector2" => vector2(val).map(Variant::Vector2),
            "Vector2int16" => vector2int16(val).map(Variant::Vector2int16),
            "Vector3" => vector3(val).map(Variant::Vector3),
            "Vector3int16" => vector3int16(val).map(Variant::Vector3int16),
            "Color3" => color3(val).map(Variant::Color3),
            "Color3uint8" => color3uint8(val).map(Variant::Color3uint8),
            "BrickColor" => val.as_u64().map(|value| {
                Variant::BrickColor(
                    BrickColor::from_number(value as u16).unwrap_or(BrickColor::MediumStoneGrey),
                )
            }),
            "UDim" => udim(val).map(Variant::UDim),
            "UDim2" => udim2(val).map(Variant::UDim2),
            "CFrame" => cframe(val).map(Variant::CFrame),
            "OptionalCFrame" => {
                if val.is_null() {
                    Some(Variant::OptionalCFrame(None))
                } else {
                    cframe(val).map(|value| Variant::OptionalCFrame(Some(value)))
                }
            }
            "Enum" => {
                let enum_value = val
                    .as_object()
                    .and_then(|value| value.get("value"))
                    .unwrap_or(val);
                if let Some(value) = enum_value.as_u64() {
                    Some(Variant::Enum(rbx_dom_weak::types::Enum::from_u32(
                        value as u32,
                    )))
                } else {
                    Some(Variant::Enum(rbx_dom_weak::types::Enum::from_u32(0)))
                }
            }
            "Rect" => rect(val).map(Variant::Rect),
            "NumberRange" => number_range(val).map(Variant::NumberRange),
            "NumberSequence" => number_sequence(val).map(Variant::NumberSequence),
            "ColorSequence" => color_sequence(val).map(Variant::ColorSequence),
            "Font" => font(val).map(Variant::Font),
            "Content" => val
                .as_str()
                .map(|value| Variant::Content(Content::from(value.to_string()))),
            "BinaryString" => self.binary_string_variant(val, dm_path, property_name),
            "SharedString" => self.shared_string_variant(val, dm_path, property_name),
            "UniqueId" => val
                .as_str()
                .and_then(|value| UniqueId::from_str(value).ok())
                .map(Variant::UniqueId),
            "SecurityCapabilities" => val
                .as_u64()
                .map(|value| Variant::SecurityCapabilities(SecurityCapabilities::from_bits(value))),
            "Faces" => faces(val).map(Variant::Faces),
            "Axes" => axes(val).map(Variant::Axes),
            "PhysicalProperties" => physical_properties(val).map(Variant::PhysicalProperties),
            "Ray" => ray(val).map(Variant::Ray),
            "Region3" => region3(val).map(Variant::Region3),
            "Region3int16" => region3int16(val).map(Variant::Region3int16),
            "Ref" => {
                let target = if val.is_null() {
                    None
                } else {
                    val.as_str().map(ToString::to_string)
                };
                return PropertyConversion::PendingRef(target);
            }
            _ => None,
        };

        converted
            .map(PropertyConversion::Value)
            .unwrap_or(PropertyConversion::Unsupported)
    }

    fn binary_string_variant(
        &mut self,
        value: &Value,
        dm_path: &str,
        property_name: &str,
    ) -> Option<Variant> {
        if let Some(encoded) = value.as_str() {
            return general_purpose::STANDARD
                .decode(encoded)
                .ok()
                .map(|value| Variant::BinaryString(BinaryString::from(value)));
        }

        let object = value.as_object()?;
        let bytes = self.read_file_backed_asset(object, dm_path, property_name)?;
        Some(Variant::BinaryString(BinaryString::from(bytes)))
    }

    fn shared_string_variant(
        &mut self,
        value: &Value,
        dm_path: &str,
        property_name: &str,
    ) -> Option<Variant> {
        let object = value.as_object()?;
        if let Some(encoded) = object.get("data").and_then(|value| value.as_str()) {
            return general_purpose::STANDARD
                .decode(encoded)
                .ok()
                .map(|value| Variant::SharedString(SharedString::new(value)));
        }

        let bytes = self.read_file_backed_asset(object, dm_path, property_name)?;
        Some(Variant::SharedString(SharedString::new(bytes)))
    }

    fn read_file_backed_asset(
        &mut self,
        object: &serde_json::Map<String, Value>,
        dm_path: &str,
        property_name: &str,
    ) -> Option<Vec<u8>> {
        if self.options.asset_mode == AssetMode::Disabled {
            return None;
        }

        let file = object.get("file").and_then(Value::as_str)?;
        Some(
            match read_asset_file(
                &self.options.project_dir,
                file,
                object.get("sha256").and_then(Value::as_str),
            ) {
                Ok(bytes) => bytes,
                Err(error) => {
                    let kind = match error.kind() {
                        AssetFileErrorKind::Missing => PlaceExportDiagnosticKind::MissingAssetFile,
                        AssetFileErrorKind::OutsideProject => {
                            PlaceExportDiagnosticKind::AssetPathOutsideProject
                        }
                        AssetFileErrorKind::HashMismatch => {
                            PlaceExportDiagnosticKind::AssetHashMismatch
                        }
                    };
                    self.push_asset_diagnostic(kind, dm_path, property_name, error.to_string());
                    return None;
                }
            },
        )
    }

    fn push_asset_diagnostic(
        &mut self,
        kind: PlaceExportDiagnosticKind,
        dm_path: &str,
        property_name: &str,
        message: String,
    ) {
        self.diagnostics.push(PlaceExportDiagnostic {
            kind,
            path: dm_path.to_string(),
            property: Some(property_name.to_string()),
            message,
        });
    }

    fn read_metadata(&mut self, path: &Path, dm_path: &str) -> Option<Value> {
        if !path.exists() {
            return None;
        }

        match std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        {
            Some(value) => Some(value),
            None => {
                self.diagnostics.push(PlaceExportDiagnostic {
                    kind: PlaceExportDiagnosticKind::InvalidMetadataJson,
                    path: dm_path.to_string(),
                    property: None,
                    message: format!("Failed to parse metadata JSON {}", path.display()),
                });
                None
            }
        }
    }

    fn resolve_pending_refs(&mut self) {
        let pending_refs = std::mem::take(&mut self.pending_refs);
        for mut pending in pending_refs {
            let resolved = match pending.target.as_deref() {
                None => Some(Ref::none()),
                Some(target) => self
                    .reference_refs
                    .get(target)
                    .copied()
                    .or_else(|| self.path_refs.get(target).copied())
                    .or_else(|| Ref::from_str(target).ok()),
            };

            let Some(resolved) = resolved else {
                self.diagnostics.push(PlaceExportDiagnostic {
                    kind: PlaceExportDiagnosticKind::UnresolvedReference,
                    path: pending.path,
                    property: Some(pending.property),
                    message: format!(
                        "Reference target '{}' was not found",
                        pending.target.unwrap_or_default()
                    ),
                });
                continue;
            };

            if pending.owner.is_none() {
                if let Some(owner) = self.path_refs.get(&pending.path) {
                    pending.owner = *owner;
                }
            }
            if let Some(instance) = self.dom.get_by_ref_mut(pending.owner) {
                instance
                    .properties
                    .insert(pending.property, Variant::Ref(resolved));
            }
        }
    }
}

fn service_selected(service_name: &str, services: Option<&HashSet<String>>) -> bool {
    services
        .map(|services| services.contains(service_name))
        .unwrap_or(true)
}

fn metadata_name(metadata: Option<&Value>) -> Option<String> {
    metadata?
        .get("name")
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn metadata_class(metadata: Option<&Value>) -> Option<&str> {
    metadata?.get("className").and_then(|value| value.as_str())
}

fn metadata_reference_id(metadata: &Value) -> Option<String> {
    metadata
        .get("referenceId")
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn unescape_name(name: &str) -> String {
    name.replace("[SLASH]", "/")
}

fn read_init_source(dir_path: &Path) -> Option<String> {
    for init_name in ["init.server.luau", "init.client.luau", "init.luau"] {
        let init_path = dir_path.join(init_name);
        if init_path.exists() {
            return std::fs::read_to_string(init_path).ok();
        }
    }
    None
}

fn init_class(dir_path: &Path) -> &'static str {
    if dir_path.join("init.server.luau").exists() {
        "Script"
    } else if dir_path.join("init.client.luau").exists() {
        "LocalScript"
    } else {
        "ModuleScript"
    }
}

fn is_init_file(name: &str) -> bool {
    matches!(name, "init.luau" | "init.server.luau" | "init.client.luau")
}

fn is_luau_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "luau" || extension == "lua")
}

fn has_sibling_script_file(path: &Path) -> bool {
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    let Some(parent) = path.parent() else {
        return false;
    };
    [
        format!("{}.server.luau", stem),
        format!("{}.client.luau", stem),
        format!("{}.luau", stem),
        format!("{}.lua", stem),
    ]
    .iter()
    .any(|script_name| parent.join(script_name).exists())
}

fn parse_script_name(filename: &str) -> (String, &'static str) {
    let name = filename
        .trim_end_matches(".luau")
        .trim_end_matches(".lua")
        .to_string();

    if name.ends_with(".server") {
        (name.trim_end_matches(".server").to_string(), "Script")
    } else if name.ends_with(".client") {
        (name.trim_end_matches(".client").to_string(), "LocalScript")
    } else {
        (name, "ModuleScript")
    }
}

fn service_class_name(name: &str) -> &'static str {
    match name {
        "Workspace" => "Workspace",
        "ReplicatedStorage" => "ReplicatedStorage",
        "ReplicatedFirst" => "ReplicatedFirst",
        "ServerScriptService" => "ServerScriptService",
        "ServerStorage" => "ServerStorage",
        "StarterGui" => "StarterGui",
        "StarterPack" => "StarterPack",
        "StarterPlayer" => "StarterPlayer",
        "Lighting" => "Lighting",
        "SoundService" => "SoundService",
        "Chat" => "Chat",
        "Teams" => "Teams",
        "TestService" => "TestService",
        "Players" => "Players",
        "MaterialService" => "MaterialService",
        _ => "Folder",
    }
}

enum PropertyConversion {
    Value(Variant),
    PendingRef(Option<String>),
    Unsupported,
}

fn direct_json_to_variant(value: &Value) -> Option<Variant> {
    match value {
        Value::String(value) => Some(Variant::String(value.clone())),
        Value::Bool(value) => Some(Variant::Bool(*value)),
        Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                Some(Variant::Int32(integer as i32))
            } else {
                value.as_f64().map(Variant::Float64)
            }
        }
        _ => None,
    }
}

fn object(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    value.as_object()
}

fn f32_field(value: &serde_json::Map<String, Value>, name: &str) -> Option<f32> {
    value.get(name)?.as_f64().map(|value| value as f32)
}

fn i16_field(value: &serde_json::Map<String, Value>, name: &str) -> Option<i16> {
    value.get(name)?.as_i64().map(|value| value as i16)
}

fn vector2(value: &Value) -> Option<Vector2> {
    let value = object(value)?;
    Some(Vector2::new(f32_field(value, "x")?, f32_field(value, "y")?))
}

fn vector2int16(value: &Value) -> Option<Vector2int16> {
    let value = object(value)?;
    Some(Vector2int16::new(
        i16_field(value, "x")?,
        i16_field(value, "y")?,
    ))
}

fn vector3(value: &Value) -> Option<Vector3> {
    let value = object(value)?;
    Some(Vector3::new(
        f32_field(value, "x")?,
        f32_field(value, "y")?,
        f32_field(value, "z")?,
    ))
}

fn vector3int16(value: &Value) -> Option<Vector3int16> {
    let value = object(value)?;
    Some(Vector3int16::new(
        i16_field(value, "x")?,
        i16_field(value, "y")?,
        i16_field(value, "z")?,
    ))
}

fn color3(value: &Value) -> Option<Color3> {
    let value = object(value)?;
    Some(Color3::new(
        f32_field(value, "r")?,
        f32_field(value, "g")?,
        f32_field(value, "b")?,
    ))
}

fn color3uint8(value: &Value) -> Option<Color3uint8> {
    let value = object(value)?;
    Some(Color3uint8::new(
        value.get("r")?.as_u64()? as u8,
        value.get("g")?.as_u64()? as u8,
        value.get("b")?.as_u64()? as u8,
    ))
}

fn udim(value: &Value) -> Option<UDim> {
    let value = object(value)?;
    Some(UDim::new(
        f32_field(value, "scale")?,
        value.get("offset")?.as_i64()? as i32,
    ))
}

fn udim2(value: &Value) -> Option<UDim2> {
    let value = object(value)?;
    Some(UDim2::new(udim(value.get("x")?)?, udim(value.get("y")?)?))
}

fn cframe(value: &Value) -> Option<CFrame> {
    let value = object(value)?;
    let pos = value.get("position")?.as_array()?;
    let rot = value.get("rotation")?.as_array()?;
    if pos.len() < 3 || rot.len() < 9 {
        return None;
    }
    Some(CFrame::new(
        Vector3::new(
            pos[0].as_f64()? as f32,
            pos[1].as_f64()? as f32,
            pos[2].as_f64()? as f32,
        ),
        Matrix3::new(
            Vector3::new(
                rot[0].as_f64()? as f32,
                rot[1].as_f64()? as f32,
                rot[2].as_f64()? as f32,
            ),
            Vector3::new(
                rot[3].as_f64()? as f32,
                rot[4].as_f64()? as f32,
                rot[5].as_f64()? as f32,
            ),
            Vector3::new(
                rot[6].as_f64()? as f32,
                rot[7].as_f64()? as f32,
                rot[8].as_f64()? as f32,
            ),
        ),
    ))
}

fn rect(value: &Value) -> Option<Rect> {
    let value = object(value)?;
    Some(Rect::new(
        vector2(value.get("min")?)?,
        vector2(value.get("max")?)?,
    ))
}

fn number_range(value: &Value) -> Option<NumberRange> {
    let value = object(value)?;
    Some(NumberRange::new(
        f32_field(value, "min")?,
        f32_field(value, "max")?,
    ))
}

fn number_sequence(value: &Value) -> Option<NumberSequence> {
    let keypoints = object(value)?.get("keypoints")?.as_array()?;
    Some(NumberSequence {
        keypoints: keypoints
            .iter()
            .filter_map(|keypoint| {
                let keypoint = object(keypoint)?;
                Some(NumberSequenceKeypoint::new(
                    f32_field(keypoint, "time")?,
                    f32_field(keypoint, "value")?,
                    f32_field(keypoint, "envelope")?,
                ))
            })
            .collect(),
    })
}

fn color_sequence(value: &Value) -> Option<ColorSequence> {
    let keypoints = object(value)?.get("keypoints")?.as_array()?;
    Some(ColorSequence {
        keypoints: keypoints
            .iter()
            .filter_map(|keypoint| {
                let keypoint = object(keypoint)?;
                Some(ColorSequenceKeypoint::new(
                    f32_field(keypoint, "time")?,
                    color3(keypoint.get("color")?)?,
                ))
            })
            .collect(),
    })
}

fn font(value: &Value) -> Option<Font> {
    let value = object(value)?;
    let family = value.get("family")?.as_str()?.to_string();
    let weight = value
        .get("weight")
        .and_then(|weight| weight.as_u64())
        .unwrap_or(400) as u16;
    let style = value
        .get("style")
        .and_then(|style| style.as_str())
        .unwrap_or("Normal");
    Some(Font {
        family,
        weight: FontWeight::from_u16(weight).unwrap_or(FontWeight::Regular),
        style: if style == "Italic" {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        cached_face_id: None,
    })
}

fn faces(value: &Value) -> Option<Faces> {
    let value = object(value)?;
    let mut bits = 0;
    if value.get("right").and_then(Value::as_bool).unwrap_or(false) {
        bits |= Faces::RIGHT.bits();
    }
    if value.get("top").and_then(Value::as_bool).unwrap_or(false) {
        bits |= Faces::TOP.bits();
    }
    if value.get("back").and_then(Value::as_bool).unwrap_or(false) {
        bits |= Faces::BACK.bits();
    }
    if value.get("left").and_then(Value::as_bool).unwrap_or(false) {
        bits |= Faces::LEFT.bits();
    }
    if value
        .get("bottom")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        bits |= Faces::BOTTOM.bits();
    }
    if value.get("front").and_then(Value::as_bool).unwrap_or(false) {
        bits |= Faces::FRONT.bits();
    }
    Faces::from_bits(bits)
}

fn axes(value: &Value) -> Option<Axes> {
    let value = object(value)?;
    let mut bits = 0;
    if value.get("x").and_then(Value::as_bool).unwrap_or(false) {
        bits |= Axes::X.bits();
    }
    if value.get("y").and_then(Value::as_bool).unwrap_or(false) {
        bits |= Axes::Y.bits();
    }
    if value.get("z").and_then(Value::as_bool).unwrap_or(false) {
        bits |= Axes::Z.bits();
    }
    Axes::from_bits(bits)
}

fn physical_properties(value: &Value) -> Option<PhysicalProperties> {
    if value.is_null() {
        return Some(PhysicalProperties::Default);
    }
    let value = object(value)?;
    Some(PhysicalProperties::Custom(CustomPhysicalProperties {
        density: f32_field(value, "density")?,
        friction: f32_field(value, "friction")?,
        elasticity: f32_field(value, "elasticity")?,
        friction_weight: f32_field(value, "frictionWeight")?,
        elasticity_weight: f32_field(value, "elasticityWeight")?,
    }))
}

fn ray(value: &Value) -> Option<Ray> {
    let value = object(value)?;
    Some(Ray::new(
        vector3(value.get("origin")?)?,
        vector3(value.get("direction")?)?,
    ))
}

fn region3(value: &Value) -> Option<Region3> {
    let value = object(value)?;
    Some(Region3::new(
        vector3(value.get("min")?)?,
        vector3(value.get("max")?)?,
    ))
}

fn region3int16(value: &Value) -> Option<Region3int16> {
    let value = object(value)?;
    Some(Region3int16::new(
        vector3int16(value.get("min")?)?,
        vector3int16(value.get("max")?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn options(project_dir: &Path) -> PlaceExportOptions {
        PlaceExportOptions {
            project_dir: project_dir.to_path_buf(),
            source_dir: project_dir.join("src"),
            output_path: project_dir.join("build/game.rbxl"),
            format: PlaceExportFormat::Rbxl,
            force: true,
            dry_run: true,
            strict: false,
            services: None,
            include_packages: true,
            tree_mapping: HashMap::new(),
            asset_mode: AssetMode::ReferencesOnly,
        }
    }

    fn child_named(dom: &WeakDom, parent: Ref, name: &str) -> Ref {
        let parent = dom.get_by_ref(parent).expect("parent exists");
        *parent
            .children()
            .iter()
            .find(|referent| {
                dom.get_by_ref(**referent)
                    .is_some_and(|child| child.name == name)
            })
            .expect("child exists")
    }

    #[test]
    fn exports_tree_mapping_metadata_names_attributes_tags_and_refs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project = temp.path();
        std::fs::create_dir_all(project.join("server")).expect("server dir");
        std::fs::create_dir_all(project.join("src/Workspace")).expect("workspace dir");

        std::fs::write(project.join("server/Main.server.luau"), "print('mapped')").expect("script");
        std::fs::write(
            project.join("server/Main.rbxjson"),
            serde_json::to_string_pretty(&json!({
                "className": "Script",
                "name": "Main/Real",
                "referenceId": "script-ref",
                "properties": {
                    "Source": { "type": "string", "value": "print('metadata')" }
                },
                "attributes": {
                    "Level": { "type": "int", "value": 7 }
                },
                "tags": ["entry"]
            }))
            .unwrap(),
        )
        .expect("script metadata");
        std::fs::write(
            project.join("src/Workspace/Part[SLASH]One.rbxjson"),
            serde_json::to_string_pretty(&json!({
                "className": "ObjectValue",
                "properties": {
                    "Value": { "type": "Ref", "value": "script-ref" }
                }
            }))
            .unwrap(),
        )
        .expect("part metadata");

        let mut options = options(project);
        options
            .tree_mapping
            .insert("ServerScriptService".to_string(), "server".to_string());

        let result = build_project_dom(options).expect("build dom");
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].kind,
            PlaceExportDiagnosticKind::DuplicateSource
        );

        let root = result.dom.root_ref();
        let server = child_named(&result.dom, root, "ServerScriptService");
        let workspace = child_named(&result.dom, root, "Workspace");
        let script = child_named(&result.dom, server, "Main/Real");
        let part = child_named(&result.dom, workspace, "Part/One");

        let script_instance = result.dom.get_by_ref(script).expect("script instance");
        assert_eq!(script_instance.class, "Script");
        assert!(matches!(
            script_instance.properties.get("Source"),
            Some(Variant::String(source)) if source == "print('mapped')"
        ));
        assert!(matches!(
            script_instance.properties.get("Attributes"),
            Some(Variant::Attributes(attributes)) if matches!(attributes.get("Level"), Some(Variant::Int32(7)))
        ));
        assert!(matches!(
            script_instance.properties.get("Tags"),
            Some(Variant::Tags(tags)) if tags.iter().collect::<Vec<_>>() == ["entry"]
        ));

        let part_instance = result.dom.get_by_ref(part).expect("part instance");
        assert!(matches!(
            part_instance.properties.get("Value"),
            Some(Variant::Ref(target)) if *target == script
        ));
    }

    #[test]
    fn strict_mode_fails_on_unresolved_reference() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project = temp.path();
        std::fs::create_dir_all(project.join("src/Workspace")).expect("workspace dir");
        std::fs::write(
            project.join("src/Workspace/Broken.rbxjson"),
            serde_json::to_string_pretty(&json!({
                "className": "ObjectValue",
                "properties": {
                    "Value": { "type": "Ref", "value": "missing-ref" }
                }
            }))
            .unwrap(),
        )
        .expect("metadata");

        let mut options = options(project);
        options.strict = true;

        let error = export_place(options).expect_err("strict export should fail");
        assert!(error.to_string().contains("strict mode"));
    }

    #[test]
    fn dry_run_summary_reports_diagnostics_without_writing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project = temp.path();
        std::fs::create_dir_all(project.join("src/Workspace")).expect("workspace dir");
        std::fs::write(
            project.join("src/Workspace/Unsupported.rbxjson"),
            serde_json::to_string_pretty(&json!({
                "className": "Part",
                "properties": {
                    "Mystery": { "type": "DefinitelyUnsupported", "value": true }
                }
            }))
            .unwrap(),
        )
        .expect("metadata");

        let summary = export_place(options(project)).expect("dry run summary");
        assert_eq!(summary.instances, 2);
        assert_eq!(summary.bytes_written, None);
        assert_eq!(summary.diagnostics.len(), 1);
        assert_eq!(
            summary.diagnostics[0].kind,
            PlaceExportDiagnosticKind::UnsupportedProperty
        );
        assert!(!project.join("build/game.rbxl").exists());
    }

    #[test]
    fn embeds_file_backed_binary_and_shared_string_values() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project = temp.path();
        std::fs::create_dir_all(project.join("src/Workspace")).expect("workspace dir");
        std::fs::create_dir_all(project.join("assets/blobs")).expect("blobs dir");

        let binary_bytes = vec![1, 2, 3, 4];
        let shared_bytes = vec![5, 6, 7, 8];
        let binary_hash = crate::asset_sha256_hex(&binary_bytes);
        let shared_hash = crate::asset_sha256_hex(&shared_bytes);
        let binary_file = format!("assets/blobs/{}.bin", binary_hash);
        let shared_file = format!("assets/blobs/{}.bin", shared_hash);
        std::fs::write(project.join(&binary_file), &binary_bytes).expect("binary blob");
        std::fs::write(project.join(&shared_file), &shared_bytes).expect("shared blob");

        std::fs::write(
            project.join("src/Workspace/AssetHolder.rbxjson"),
            serde_json::to_string_pretty(&json!({
                "className": "Folder",
                "properties": {
                    "BinaryData": {
                        "type": "BinaryString",
                        "value": {
                            "file": binary_file,
                            "encoding": "raw",
                            "sha256": binary_hash,
                            "byteLength": 4
                        }
                    },
                    "SharedData": {
                        "type": "SharedString",
                        "value": {
                            "hash": "shared-hash",
                            "file": shared_file,
                            "sha256": shared_hash,
                            "byteLength": 4
                        }
                    }
                }
            }))
            .unwrap(),
        )
        .expect("metadata");

        let result = build_project_dom(options(project)).expect("build dom");
        assert!(result.diagnostics.is_empty());

        let root = result.dom.root_ref();
        let workspace = child_named(&result.dom, root, "Workspace");
        let asset_holder = child_named(&result.dom, workspace, "AssetHolder");
        let instance = result
            .dom
            .get_by_ref(asset_holder)
            .expect("asset holder instance");

        assert!(matches!(
            instance.properties.get("BinaryData"),
            Some(Variant::BinaryString(value))
                if <BinaryString as AsRef<[u8]>>::as_ref(value) == binary_bytes.as_slice()
        ));
        assert!(matches!(
            instance.properties.get("SharedData"),
            Some(Variant::SharedString(value)) if value.data() == shared_bytes.as_slice()
        ));
    }

    #[test]
    fn file_backed_asset_hash_mismatch_fails_export() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project = temp.path();
        std::fs::create_dir_all(project.join("src/Workspace")).expect("workspace dir");
        std::fs::create_dir_all(project.join("assets/blobs")).expect("blobs dir");
        std::fs::write(project.join("assets/blobs/blob.bin"), [1, 2, 3, 4]).expect("blob");
        std::fs::write(
            project.join("src/Workspace/AssetHolder.rbxjson"),
            serde_json::to_string_pretty(&json!({
                "className": "Folder",
                "properties": {
                    "BinaryData": {
                        "type": "BinaryString",
                        "value": {
                            "file": "assets/blobs/blob.bin",
                            "sha256": "0000"
                        }
                    }
                }
            }))
            .unwrap(),
        )
        .expect("metadata");

        let error = export_place(options(project)).expect_err("hash mismatch should fail");
        assert!(error.to_string().contains("sha256 mismatch"));
    }

    #[test]
    fn file_backed_asset_outside_project_fails_export() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::NamedTempFile::new().expect("outside file");
        std::fs::write(outside.path(), [1, 2, 3, 4]).expect("outside bytes");

        let project = temp.path();
        std::fs::create_dir_all(project.join("src/Workspace")).expect("workspace dir");
        std::fs::write(
            project.join("src/Workspace/AssetHolder.rbxjson"),
            serde_json::to_string_pretty(&json!({
                "className": "Folder",
                "properties": {
                    "BinaryData": {
                        "type": "BinaryString",
                        "value": {
                            "file": outside.path().display().to_string()
                        }
                    }
                }
            }))
            .unwrap(),
        )
        .expect("metadata");

        let error = export_place(options(project)).expect_err("outside path should fail");
        assert!(error
            .to_string()
            .contains("must be relative to the project"));
    }

    #[test]
    fn include_local_summary_reports_manifest_asset_counts() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project = temp.path();
        std::fs::create_dir_all(project.join("src/Workspace")).expect("workspace dir");
        let manifest = crate::build_asset_manifest(
            "test",
            vec![crate::AssetEntry {
                id: "content:Workspace/Sound:SoundId".to_string(),
                kind: crate::AssetKind::Content,
                source: crate::AssetSource::ExternalReference,
                instance_path: "Workspace/Sound".to_string(),
                property: "SoundId".to_string(),
                original: Some("rbxassetid://123456".to_string()),
                file: None,
                sha256: None,
                byte_length: None,
                status: crate::AssetStatus::ReferencedOnly,
            }],
        );
        crate::write_asset_manifest(&project.join("assets/manifest.json"), &manifest)
            .expect("write manifest");

        let mut options = options(project);
        options.asset_mode = AssetMode::IncludeLocal;
        let summary = export_place(options).expect("export summary");

        let asset_summary = summary.asset_summary.expect("asset summary");
        assert_eq!(asset_summary.mode, AssetMode::IncludeLocal);
        assert_eq!(
            asset_summary.manifest.as_deref(),
            Some("assets/manifest.json")
        );
        assert_eq!(asset_summary.content_references, 1);
        assert_eq!(asset_summary.embedded_payloads, 0);
    }
}
