//! Shared Roblox place/model exporter.
//!
//! This module owns the project-to-DOM build path used by CLI artifact
//! commands. The initial implementation preserves the historical `rbxsync
//! build` behavior while moving the logic out of the CLI so place-focused
//! export commands can share it.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use rbx_dom_weak::types::Variant;
use rbx_dom_weak::{InstanceBuilder, WeakDom};
use serde::{Deserialize, Serialize};

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
}

/// Non-fatal export diagnostic placeholder for later parity milestones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaceExportDiagnosticKind {
    UnsupportedProperty,
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
}

/// Export project files to a Roblox place/model artifact.
pub fn export_place(options: PlaceExportOptions) -> Result<PlaceExportSummary> {
    if !options.source_dir.exists() {
        bail!(
            "Source directory not found: {}",
            options.source_dir.display()
        );
    }

    if options.output_path.exists() && !options.force && !options.dry_run {
        bail!(
            "Output file already exists: {}. Use --force to replace it.",
            options.output_path.display()
        );
    }

    let dom = build_dom_from_project(&options)?;
    let summary = summarize_dom(&dom, &options);

    if !options.dry_run {
        write_dom(&dom, &options)?;
    }

    let bytes_written = if options.dry_run {
        None
    } else {
        std::fs::metadata(&options.output_path)
            .ok()
            .map(|metadata| metadata.len())
    };

    Ok(PlaceExportSummary {
        bytes_written,
        ..summary
    })
}

/// Build a Roblox DOM from an RbxSync source tree.
pub fn build_dom_from_project(options: &PlaceExportOptions) -> Result<WeakDom> {
    build_dom_from_src(&options.source_dir, options.format.is_place())
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

fn summarize_dom(dom: &WeakDom, options: &PlaceExportOptions) -> PlaceExportSummary {
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
        diagnostics: Vec::new(),
    }
}

fn count_instance_tree(
    dom: &WeakDom,
    referent: rbx_dom_weak::types::Ref,
    instances: &mut usize,
    scripts: &mut usize,
) {
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

/// Build a DOM from the src directory.
fn build_dom_from_src(src_dir: &Path, is_place: bool) -> Result<WeakDom> {
    let root_class = if is_place { "DataModel" } else { "Folder" };
    let root_name = if is_place { "game" } else { "Model" };

    let mut dom = WeakDom::new(InstanceBuilder::new(root_class).with_name(root_name));
    let root_ref = dom.root_ref();

    let mut entries: Vec<_> = std::fs::read_dir(src_dir)
        .context("Failed to read src directory")?
        .filter_map(|entry| entry.ok())
        .collect();

    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let entry_path = entry.path();
        let entry_name = entry.file_name().to_string_lossy().to_string();

        if entry_path.is_dir() {
            let class_name = service_class_name(&entry_name);
            let service_ref = dom.insert(
                root_ref,
                InstanceBuilder::new(class_name).with_name(&entry_name),
            );

            build_dom_children(&mut dom, service_ref, &entry_path)?;
        } else if entry_path
            .extension()
            .is_some_and(|extension| extension == "rbxjson")
        {
            let instance_name = entry_path
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_default();

            if let Ok(content) = std::fs::read_to_string(&entry_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let class_name = json
                        .get("className")
                        .and_then(|class| class.as_str())
                        .unwrap_or("Folder");

                    let mut builder = InstanceBuilder::new(class_name).with_name(&instance_name);

                    if let Some(props) = json.get("properties").and_then(|props| props.as_object())
                    {
                        for (prop_name, prop_value) in props {
                            if let Some(value) = json_to_variant(prop_value) {
                                builder = builder.with_property(prop_name, value);
                            }
                        }
                    }

                    dom.insert(root_ref, builder);
                }
            }
        } else if entry_path
            .extension()
            .is_some_and(|extension| extension == "luau" || extension == "lua")
        {
            let (script_name, class_name) = parse_script_name(&entry_name);
            if let Ok(source) = std::fs::read_to_string(&entry_path) {
                dom.insert(
                    root_ref,
                    InstanceBuilder::new(class_name)
                        .with_name(&script_name)
                        .with_property("Source", Variant::String(source)),
                );
            }
        }
    }

    Ok(dom)
}

/// Recursively build DOM children from a directory.
fn build_dom_children(
    dom: &mut WeakDom,
    parent_ref: rbx_dom_weak::types::Ref,
    dir_path: &Path,
) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir_path)
        .context("Failed to read directory")?
        .filter_map(|entry| entry.ok())
        .collect();

    entries.sort_by_key(|entry| entry.file_name());

    let init_files = ["init.luau", "init.server.luau", "init.client.luau"];
    for init_name in init_files {
        let init_path = dir_path.join(init_name);
        if init_path.exists() {
            if let Ok(source) = std::fs::read_to_string(&init_path) {
                if let Some(instance) = dom.get_by_ref_mut(parent_ref) {
                    instance
                        .properties
                        .insert("Source".to_string(), Variant::String(source));
                }
            }
            break;
        }
    }

    for entry in entries {
        let entry_path = entry.path();
        let entry_name = entry.file_name().to_string_lossy().to_string();

        if init_files.iter().any(|&name| entry_name == name) || entry_name == "_meta.rbxjson" {
            continue;
        }

        if entry_path.is_dir() {
            let meta_path = entry_path.join("_meta.rbxjson");
            let meta_data: Option<serde_json::Value> = if meta_path.exists() {
                std::fs::read_to_string(&meta_path)
                    .ok()
                    .and_then(|content| serde_json::from_str(&content).ok())
            } else {
                None
            };

            let has_init = init_files
                .iter()
                .any(|&name| entry_path.join(name).exists());

            let class_name = if has_init {
                if entry_path.join("init.server.luau").exists() {
                    "Script"
                } else if entry_path.join("init.client.luau").exists() {
                    "LocalScript"
                } else {
                    "ModuleScript"
                }
            } else if let Some(ref meta) = meta_data {
                meta.get("className")
                    .and_then(|class| class.as_str())
                    .unwrap_or("Folder")
            } else {
                "Folder"
            };

            let mut builder = InstanceBuilder::new(class_name).with_name(&entry_name);

            if let Some(ref meta) = meta_data {
                if let Some(props) = meta.get("properties").and_then(|props| props.as_object()) {
                    for (prop_name, prop_value) in props {
                        if let Some(value) = json_to_variant(prop_value) {
                            builder = builder.with_property(prop_name, value);
                        }
                    }
                }
            }

            let child_ref = dom.insert(parent_ref, builder);

            build_dom_children(dom, child_ref, &entry_path)?;
        } else if entry_path
            .extension()
            .is_some_and(|extension| extension == "rbxjson")
        {
            let instance_name = entry_path
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_default();

            if let Ok(content) = std::fs::read_to_string(&entry_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let class_name = json
                        .get("className")
                        .and_then(|class| class.as_str())
                        .unwrap_or("Folder");

                    let mut builder = InstanceBuilder::new(class_name).with_name(&instance_name);

                    if let Some(props) = json.get("properties").and_then(|props| props.as_object())
                    {
                        for (prop_name, prop_value) in props {
                            if let Some(value) = json_to_variant(prop_value) {
                                builder = builder.with_property(prop_name, value);
                            }
                        }
                    }

                    dom.insert(parent_ref, builder);
                }
            }
        } else if entry_path
            .extension()
            .is_some_and(|extension| extension == "luau" || extension == "lua")
        {
            let (script_name, class_name) = parse_script_name(&entry_name);
            if let Ok(source) = std::fs::read_to_string(&entry_path) {
                dom.insert(
                    parent_ref,
                    InstanceBuilder::new(class_name)
                        .with_name(&script_name)
                        .with_property("Source", Variant::String(source)),
                );
            }
        }
    }

    Ok(())
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
        _ => "Folder",
    }
}

fn json_to_variant(value: &serde_json::Value) -> Option<Variant> {
    use rbx_dom_weak::types::*;

    if let Some(obj) = value.as_object() {
        if let Some(type_str) = obj
            .get("type")
            .and_then(|property_type| property_type.as_str())
        {
            let val = obj.get("value");
            return match type_str {
                "string" => val?
                    .as_str()
                    .map(|value| Variant::String(value.to_string())),
                "int" | "int32" => val?.as_i64().map(|value| Variant::Int32(value as i32)),
                "int64" => val?.as_i64().map(Variant::Int64),
                "float" | "float32" => val?.as_f64().map(|value| Variant::Float32(value as f32)),
                "float64" | "double" => val?.as_f64().map(Variant::Float64),
                "bool" => val?.as_bool().map(Variant::Bool),
                "nil" => None,
                "Vector2" => {
                    let value = val?.as_object()?;
                    Some(Variant::Vector2(Vector2::new(
                        value.get("x")?.as_f64()? as f32,
                        value.get("y")?.as_f64()? as f32,
                    )))
                }
                "Vector3" => {
                    let value = val?.as_object()?;
                    Some(Variant::Vector3(Vector3::new(
                        value.get("x")?.as_f64()? as f32,
                        value.get("y")?.as_f64()? as f32,
                        value.get("z")?.as_f64()? as f32,
                    )))
                }
                "Color3" => {
                    let value = val?.as_object()?;
                    Some(Variant::Color3(Color3::new(
                        value.get("r")?.as_f64()? as f32,
                        value.get("g")?.as_f64()? as f32,
                        value.get("b")?.as_f64()? as f32,
                    )))
                }
                "Color3uint8" => {
                    let value = val?.as_object()?;
                    Some(Variant::Color3uint8(Color3uint8::new(
                        value.get("r")?.as_u64()? as u8,
                        value.get("g")?.as_u64()? as u8,
                        value.get("b")?.as_u64()? as u8,
                    )))
                }
                "BrickColor" => val?.as_u64().map(|value| {
                    Variant::BrickColor(
                        BrickColor::from_number(value as u16)
                            .unwrap_or(BrickColor::MediumStoneGrey),
                    )
                }),
                "UDim" => {
                    let value = val?.as_object()?;
                    Some(Variant::UDim(UDim::new(
                        value.get("scale")?.as_f64()? as f32,
                        value.get("offset")?.as_i64()? as i32,
                    )))
                }
                "UDim2" => {
                    let value = val?.as_object()?;
                    let x = value.get("x")?.as_object()?;
                    let y = value.get("y")?.as_object()?;
                    Some(Variant::UDim2(UDim2::new(
                        UDim::new(
                            x.get("scale")?.as_f64()? as f32,
                            x.get("offset")?.as_i64()? as i32,
                        ),
                        UDim::new(
                            y.get("scale")?.as_f64()? as f32,
                            y.get("offset")?.as_i64()? as i32,
                        ),
                    )))
                }
                "CFrame" => {
                    let value = val?.as_object()?;
                    let pos = value.get("position")?.as_array()?;
                    let rot = value.get("rotation")?.as_array()?;
                    if pos.len() >= 3 && rot.len() >= 9 {
                        Some(Variant::CFrame(CFrame::new(
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
                        )))
                    } else {
                        None
                    }
                }
                "Enum" => {
                    let value = val?.as_object()?;
                    let enum_value = value.get("value")?;
                    if let Some(value) = enum_value.as_u64() {
                        Some(Variant::Enum(rbx_dom_weak::types::Enum::from_u32(
                            value as u32,
                        )))
                    } else {
                        Some(Variant::Enum(rbx_dom_weak::types::Enum::from_u32(0)))
                    }
                }
                "Rect" => {
                    let value = val?.as_object()?;
                    let min = value.get("min")?.as_object()?;
                    let max = value.get("max")?.as_object()?;
                    Some(Variant::Rect(Rect::new(
                        Vector2::new(
                            min.get("x")?.as_f64()? as f32,
                            min.get("y")?.as_f64()? as f32,
                        ),
                        Vector2::new(
                            max.get("x")?.as_f64()? as f32,
                            max.get("y")?.as_f64()? as f32,
                        ),
                    )))
                }
                "NumberRange" => {
                    let value = val?.as_object()?;
                    Some(Variant::NumberRange(NumberRange::new(
                        value.get("min")?.as_f64()? as f32,
                        value.get("max")?.as_f64()? as f32,
                    )))
                }
                "Font" => {
                    let value = val?.as_object()?;
                    let family = value.get("family")?.as_str()?.to_string();
                    let weight = value
                        .get("weight")
                        .and_then(|weight| weight.as_u64())
                        .unwrap_or(400) as u16;
                    let style = value
                        .get("style")
                        .and_then(|style| style.as_str())
                        .unwrap_or("Normal");
                    Some(Variant::Font(Font {
                        family,
                        weight: FontWeight::from_u16(weight).unwrap_or(FontWeight::Regular),
                        style: if style == "Italic" {
                            FontStyle::Italic
                        } else {
                            FontStyle::Normal
                        },
                        cached_face_id: None,
                    }))
                }
                "Content" => val?
                    .as_str()
                    .map(|value| Variant::Content(Content::from(value.to_string()))),
                "Ref" => None,
                _ => {
                    tracing::debug!("Unsupported property type: {}", type_str);
                    None
                }
            };
        }
    }

    match value {
        serde_json::Value::String(value) => Some(Variant::String(value.clone())),
        serde_json::Value::Bool(value) => Some(Variant::Bool(*value)),
        serde_json::Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                Some(Variant::Int32(integer as i32))
            } else {
                value.as_f64().map(Variant::Float64)
            }
        }
        _ => None,
    }
}
