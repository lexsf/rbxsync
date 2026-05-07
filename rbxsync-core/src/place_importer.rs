//! Roblox place file importer.
//!
//! This module converts saved `.rbxl` and `.rbxlx` files into the same flat,
//! plugin-compatible serialized instance JSON that the shared extraction writer
//! consumes.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use rbx_dom_weak::types::{self, Variant};
use rbx_dom_weak::WeakDom;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Supported Roblox place file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaceFileFormat {
    Rbxl,
    Rbxlx,
}

/// Options for importing a local Roblox place file.
#[derive(Debug, Clone)]
pub struct PlaceImportOptions {
    pub input_path: PathBuf,
    pub services: Option<HashSet<String>>,
    pub include_terrain: bool,
}

/// Non-fatal importer diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportDiagnosticKind {
    UnsupportedProperty,
    UnsupportedAttribute,
    MissingService,
    MissingScriptSource,
    UnsupportedTerrainVoxelData,
}

/// Non-fatal importer diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDiagnostic {
    pub kind: ImportDiagnosticKind,
    pub path: String,
    pub property: Option<String>,
    pub message: String,
}

/// Result of converting a place file into serialized instance JSON.
#[derive(Debug, Clone)]
pub struct PlaceImportResult {
    pub instances: Vec<Value>,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub format: PlaceFileFormat,
}

/// Import a local `.rbxl` or `.rbxlx` file.
pub fn import_place_file(options: PlaceImportOptions) -> Result<PlaceImportResult> {
    let format = detect_place_file_format(&options.input_path)?;
    let file = File::open(&options.input_path)
        .with_context(|| format!("Failed to open {}", options.input_path.display()))?;
    let reader = BufReader::new(file);

    let dom = match format {
        PlaceFileFormat::Rbxl => {
            rbx_binary::from_reader(reader).context("Failed to read binary .rbxl file")?
        }
        PlaceFileFormat::Rbxlx => {
            rbx_xml::from_reader_default(reader).context("Failed to read XML .rbxlx file")?
        }
    };

    Ok(import_dom(&dom, &options, format))
}

fn detect_place_file_format(path: &std::path::Path) -> Result<PlaceFileFormat> {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("rbxl") => Ok(PlaceFileFormat::Rbxl),
        Some("rbxlx") => Ok(PlaceFileFormat::Rbxlx),
        Some(ext) => bail!("Unsupported place file extension '.{}'", ext),
        None => bail!("Place file must have a .rbxl or .rbxlx extension"),
    }
}

fn import_dom(
    dom: &WeakDom,
    options: &PlaceImportOptions,
    format: PlaceFileFormat,
) -> PlaceImportResult {
    let mut instances = Vec::new();
    let mut diagnostics = Vec::new();
    let root_ref = dom.root_ref();

    let root_children = dom.root().children().to_vec();
    let root_paths = child_path_segments(dom, &root_children);
    record_missing_services(
        dom,
        &root_children,
        options.services.as_ref(),
        &mut diagnostics,
    );

    for child_ref in root_children {
        let Some(instance) = dom.get_by_ref(child_ref) else {
            continue;
        };

        if !service_selected(instance, options.services.as_ref()) {
            continue;
        }

        if should_skip_terrain(instance, options.include_terrain) {
            continue;
        }

        let Some(path) = root_paths.get(&child_ref).cloned() else {
            continue;
        };

        serialize_instance_tree(
            dom,
            child_ref,
            root_ref,
            path,
            options,
            &mut instances,
            &mut diagnostics,
        );
    }

    PlaceImportResult {
        instances,
        diagnostics,
        format,
    }
}

fn service_selected(instance: &rbx_dom_weak::Instance, services: Option<&HashSet<String>>) -> bool {
    let Some(services) = services else {
        return true;
    };

    services.contains(&instance.name) || services.contains(&instance.class)
}

fn should_skip_terrain(instance: &rbx_dom_weak::Instance, include_terrain: bool) -> bool {
    !include_terrain && instance.class == "Terrain"
}

fn record_missing_services(
    dom: &WeakDom,
    root_children: &[types::Ref],
    requested_services: Option<&HashSet<String>>,
    diagnostics: &mut Vec<ImportDiagnostic>,
) {
    let Some(requested_services) = requested_services else {
        return;
    };

    let mut available = HashSet::new();
    for child_ref in root_children {
        if let Some(instance) = dom.get_by_ref(*child_ref) {
            available.insert(instance.name.clone());
            available.insert(instance.class.clone());
        }
    }

    for service in requested_services {
        if !available.contains(service) {
            diagnostics.push(ImportDiagnostic {
                kind: ImportDiagnosticKind::MissingService,
                path: "game".to_string(),
                property: None,
                message: format!("Requested service '{}' was not found in the place", service),
            });
        }
    }
}

fn serialize_instance_tree(
    dom: &WeakDom,
    referent: types::Ref,
    root_ref: types::Ref,
    path: String,
    options: &PlaceImportOptions,
    instances: &mut Vec<Value>,
    diagnostics: &mut Vec<ImportDiagnostic>,
) {
    let Some(instance) = dom.get_by_ref(referent) else {
        return;
    };

    let serialized = serialize_instance(instance, root_ref, &path, diagnostics);
    instances.push(serialized);

    let child_refs = instance.children().to_vec();
    let child_paths = child_path_segments(dom, &child_refs);

    for child_ref in child_refs {
        let Some(child) = dom.get_by_ref(child_ref) else {
            continue;
        };

        if should_skip_terrain(child, options.include_terrain) {
            continue;
        }

        let Some(child_segment) = child_paths.get(&child_ref) else {
            continue;
        };
        let child_path = format!("{}/{}", path, child_segment);

        serialize_instance_tree(
            dom,
            child_ref,
            root_ref,
            child_path,
            options,
            instances,
            diagnostics,
        );
    }
}

fn serialize_instance(
    instance: &rbx_dom_weak::Instance,
    root_ref: types::Ref,
    path: &str,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> Value {
    let mut properties = Map::new();
    let mut attributes = Map::new();
    let mut tags: Vec<String> = Vec::new();

    let mut property_names: Vec<_> = instance.properties.keys().collect();
    property_names.sort();
    let mut has_script_source = false;

    for property_name in property_names {
        let Some(variant) = instance.properties.get(property_name) else {
            continue;
        };
        if property_name == "Source" {
            has_script_source = true;
        }

        match (property_name.as_str(), variant) {
            ("Attributes", Variant::Attributes(values)) => {
                for (name, value) in values {
                    if let Some(encoded) = variant_to_json_property(value) {
                        attributes.insert(name.clone(), encoded);
                    } else {
                        diagnostics.push(ImportDiagnostic {
                            kind: ImportDiagnosticKind::UnsupportedAttribute,
                            path: path.to_string(),
                            property: Some(format!("Attributes.{}", name)),
                            message: format!(
                                "Skipped unsupported attribute variant {:?}",
                                value.ty()
                            ),
                        });
                    }
                }
            }
            ("Tags", Variant::Tags(values)) => {
                tags.extend(values.iter().map(str::to_string));
            }
            _ => {
                if let Some(encoded) = variant_to_json_property(variant) {
                    properties.insert(property_name.clone(), encoded);
                } else {
                    diagnostics.push(ImportDiagnostic {
                        kind: ImportDiagnosticKind::UnsupportedProperty,
                        path: path.to_string(),
                        property: Some(property_name.clone()),
                        message: format!("Skipped unsupported property variant {:?}", variant.ty()),
                    });
                }
            }
        }
    }

    if matches!(
        instance.class.as_str(),
        "Script" | "LocalScript" | "ModuleScript"
    ) && !has_script_source
    {
        diagnostics.push(ImportDiagnostic {
            kind: ImportDiagnosticKind::MissingScriptSource,
            path: path.to_string(),
            property: Some("Source".to_string()),
            message: format!(
                "{} has no Source property in the place file",
                instance.class
            ),
        });
    }

    if instance.class == "Terrain" {
        diagnostics.push(ImportDiagnostic {
            kind: ImportDiagnosticKind::UnsupportedTerrainVoxelData,
            path: path.to_string(),
            property: None,
            message:
                "Terrain voxel data is not converted by place import; metadata properties only"
                    .to_string(),
        });
    }

    let parent_id = if instance.parent().is_none() {
        None
    } else if instance.parent() == root_ref {
        Some(root_ref.to_string())
    } else {
        Some(instance.parent().to_string())
    };

    let mut serialized = Map::new();
    serialized.insert("className".to_string(), json!(instance.class));
    serialized.insert("name".to_string(), json!(instance.name));
    serialized.insert(
        "referenceId".to_string(),
        json!(instance.referent().to_string()),
    );
    serialized.insert("parentId".to_string(), json!(parent_id));
    serialized.insert("path".to_string(), json!(path));
    serialized.insert("properties".to_string(), Value::Object(properties));

    if !attributes.is_empty() {
        serialized.insert("attributes".to_string(), Value::Object(attributes));
    }

    if !tags.is_empty() {
        serialized.insert("tags".to_string(), json!(tags));
    }

    Value::Object(serialized)
}

fn child_path_segments(dom: &WeakDom, child_refs: &[types::Ref]) -> HashMap<types::Ref, String> {
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for child_ref in child_refs {
        if let Some(child) = dom.get_by_ref(*child_ref) {
            *name_counts.entry(child.name.clone()).or_insert(0) += 1;
        }
    }

    let mut name_seen: HashMap<String, usize> = HashMap::new();
    let mut paths = HashMap::new();

    for child_ref in child_refs {
        let Some(child) = dom.get_by_ref(*child_ref) else {
            continue;
        };

        let seen = name_seen.entry(child.name.clone()).or_insert(0);
        *seen += 1;

        let mut segment = escape_path_segment(&child.name);
        if name_counts.get(&child.name).copied().unwrap_or(0) > 1 && *seen > 1 {
            segment.push('~');
            segment.push_str(&short_referent(*child_ref));
        }

        paths.insert(*child_ref, segment);
    }

    paths
}

fn escape_path_segment(name: &str) -> String {
    name.replace('/', "[SLASH]")
}

fn short_referent(referent: types::Ref) -> String {
    referent.to_string().chars().take(8).collect()
}

fn variant_to_json_property(variant: &Variant) -> Option<Value> {
    Some(match variant {
        Variant::Bool(value) => json!({ "type": "bool", "value": value }),
        Variant::Int32(value) => json!({ "type": "int", "value": value }),
        Variant::Int64(value) => json!({ "type": "int64", "value": value }),
        Variant::Float32(value) => json!({ "type": "float", "value": value }),
        Variant::Float64(value) => json!({ "type": "double", "value": value }),
        Variant::String(value) => json!({ "type": "string", "value": value }),
        Variant::Content(value) => {
            json!({ "type": "Content", "value": <types::Content as AsRef<str>>::as_ref(value) })
        }
        Variant::Vector2(value) => json!({
            "type": "Vector2",
            "value": { "x": value.x, "y": value.y }
        }),
        Variant::Vector2int16(value) => json!({
            "type": "Vector2int16",
            "value": { "x": value.x, "y": value.y }
        }),
        Variant::Vector3(value) => json!({
            "type": "Vector3",
            "value": { "x": value.x, "y": value.y, "z": value.z }
        }),
        Variant::Vector3int16(value) => json!({
            "type": "Vector3int16",
            "value": { "x": value.x, "y": value.y, "z": value.z }
        }),
        Variant::CFrame(value) => json!({
            "type": "CFrame",
            "value": {
                "position": [value.position.x, value.position.y, value.position.z],
                "rotation": [
                    value.orientation.x.x, value.orientation.x.y, value.orientation.x.z,
                    value.orientation.y.x, value.orientation.y.y, value.orientation.y.z,
                    value.orientation.z.x, value.orientation.z.y, value.orientation.z.z
                ]
            }
        }),
        Variant::Color3(value) => json!({
            "type": "Color3",
            "value": { "r": value.r, "g": value.g, "b": value.b }
        }),
        Variant::Color3uint8(value) => json!({
            "type": "Color3uint8",
            "value": { "r": value.r, "g": value.g, "b": value.b }
        }),
        Variant::BrickColor(value) => json!({ "type": "BrickColor", "value": *value as u16 }),
        Variant::UDim(value) => json!({
            "type": "UDim",
            "value": { "scale": value.scale, "offset": value.offset }
        }),
        Variant::UDim2(value) => json!({
            "type": "UDim2",
            "value": {
                "x": { "scale": value.x.scale, "offset": value.x.offset },
                "y": { "scale": value.y.scale, "offset": value.y.offset }
            }
        }),
        Variant::Rect(value) => json!({
            "type": "Rect",
            "value": {
                "min": { "x": value.min.x, "y": value.min.y },
                "max": { "x": value.max.x, "y": value.max.y }
            }
        }),
        Variant::NumberRange(value) => json!({
            "type": "NumberRange",
            "value": { "min": value.min, "max": value.max }
        }),
        Variant::Enum(value) => json!({
            "type": "Enum",
            "value": { "enumType": Value::Null, "value": value.to_u32() }
        }),
        Variant::Ref(value) => {
            let serialized = if value.is_none() {
                Value::Null
            } else {
                json!(value.to_string())
            };
            json!({ "type": "Ref", "value": serialized })
        }
        Variant::NumberSequence(value) => json!({
            "type": "NumberSequence",
            "value": {
                "keypoints": value.keypoints.iter().map(|kp| json!({
                    "time": kp.time,
                    "value": kp.value,
                    "envelope": kp.envelope
                })).collect::<Vec<_>>()
            }
        }),
        Variant::ColorSequence(value) => json!({
            "type": "ColorSequence",
            "value": {
                "keypoints": value.keypoints.iter().map(|kp| json!({
                    "time": kp.time,
                    "color": { "r": kp.color.r, "g": kp.color.g, "b": kp.color.b }
                })).collect::<Vec<_>>()
            }
        }),
        Variant::Font(value) => json!({
            "type": "Font",
            "value": {
                "family": value.family,
                "weight": value.weight.as_u16(),
                "style": format!("{:?}", value.style)
            }
        }),
        Variant::Faces(value) => json!({
            "type": "Faces",
            "value": {
                "top": value.contains(types::Faces::TOP),
                "bottom": value.contains(types::Faces::BOTTOM),
                "left": value.contains(types::Faces::LEFT),
                "right": value.contains(types::Faces::RIGHT),
                "front": value.contains(types::Faces::FRONT),
                "back": value.contains(types::Faces::BACK)
            }
        }),
        Variant::Axes(value) => json!({
            "type": "Axes",
            "value": {
                "x": value.contains(types::Axes::X),
                "y": value.contains(types::Axes::Y),
                "z": value.contains(types::Axes::Z)
            }
        }),
        Variant::PhysicalProperties(value) => match value {
            types::PhysicalProperties::Default => {
                json!({ "type": "PhysicalProperties", "value": Value::Null })
            }
            types::PhysicalProperties::Custom(custom) => json!({
                "type": "PhysicalProperties",
                "value": {
                    "density": custom.density,
                    "friction": custom.friction,
                    "elasticity": custom.elasticity,
                    "frictionWeight": custom.friction_weight,
                    "elasticityWeight": custom.elasticity_weight
                }
            }),
        },
        Variant::Ray(value) => json!({
            "type": "Ray",
            "value": {
                "origin": { "x": value.origin.x, "y": value.origin.y, "z": value.origin.z },
                "direction": {
                    "x": value.direction.x,
                    "y": value.direction.y,
                    "z": value.direction.z
                }
            }
        }),
        Variant::Region3(value) => json!({
            "type": "Region3",
            "value": {
                "min": { "x": value.min.x, "y": value.min.y, "z": value.min.z },
                "max": { "x": value.max.x, "y": value.max.y, "z": value.max.z }
            }
        }),
        Variant::Region3int16(value) => json!({
            "type": "Region3int16",
            "value": {
                "min": { "x": value.min.x, "y": value.min.y, "z": value.min.z },
                "max": { "x": value.max.x, "y": value.max.y, "z": value.max.z }
            }
        }),
        Variant::OptionalCFrame(value) => {
            let encoded = value.as_ref().map(|cframe| {
                json!({
                    "position": [cframe.position.x, cframe.position.y, cframe.position.z],
                    "rotation": [
                        cframe.orientation.x.x, cframe.orientation.x.y, cframe.orientation.x.z,
                        cframe.orientation.y.x, cframe.orientation.y.y, cframe.orientation.y.z,
                        cframe.orientation.z.x, cframe.orientation.z.y, cframe.orientation.z.z
                    ]
                })
            });
            json!({ "type": "OptionalCFrame", "value": encoded })
        }
        Variant::UniqueId(value) => json!({ "type": "UniqueId", "value": value.to_string() }),
        Variant::SecurityCapabilities(value) => {
            json!({ "type": "SecurityCapabilities", "value": value.bits() })
        }
        Variant::BinaryString(value) => json!({
            "type": "BinaryString",
            "value": general_purpose::STANDARD.encode(
                <types::BinaryString as AsRef<[u8]>>::as_ref(value)
            )
        }),
        Variant::SharedString(value) => json!({
            "type": "SharedString",
            "value": {
                "hash": value.hash().to_string(),
                "file": Value::Null,
                "data": general_purpose::STANDARD.encode(value.data())
            }
        }),
        Variant::MaterialColors(_) | Variant::Tags(_) | Variant::Attributes(_) => return None,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbx_dom_weak::types::{
        Attributes, BinaryString, CFrame, Color3, Matrix3, SharedString, Tags, Vector3,
    };
    use rbx_dom_weak::{InstanceBuilder, WeakDom};

    fn test_options() -> PlaceImportOptions {
        PlaceImportOptions {
            input_path: PathBuf::from("test.rbxl"),
            services: None,
            include_terrain: true,
        }
    }

    #[test]
    fn serializes_paths_with_slash_escaping_and_duplicate_suffixes() {
        let mut dom = WeakDom::new(InstanceBuilder::new("DataModel").with_name("game"));
        let root = dom.root_ref();
        let workspace = dom.insert(root, InstanceBuilder::new("Workspace"));
        let first = dom.insert(workspace, InstanceBuilder::new("Folder").with_name("A/B"));
        let second = dom.insert(workspace, InstanceBuilder::new("Folder").with_name("A/B"));
        dom.insert(
            first,
            InstanceBuilder::new("Script")
                .with_name("Run")
                .with_property("Source", "print('hello')"),
        );

        let result = import_dom(&dom, &test_options(), PlaceFileFormat::Rbxl);
        let paths: Vec<_> = result
            .instances
            .iter()
            .map(|inst| inst["path"].as_str().unwrap().to_string())
            .collect();

        assert!(paths.contains(&"Workspace/A[SLASH]B".to_string()));
        assert!(paths.contains(&format!("Workspace/A[SLASH]B~{}", short_referent(second))));
        assert!(paths.contains(&"Workspace/A[SLASH]B/Run".to_string()));

        let script = result
            .instances
            .iter()
            .find(|inst| inst["className"] == "Script")
            .unwrap();
        assert_eq!(script["properties"]["Source"]["value"], "print('hello')");
    }

    #[test]
    fn serializes_properties_attributes_and_tags() {
        let mut attrs = Attributes::new();
        attrs.insert("Level".to_string(), Variant::Int32(7));

        let mut tags = Tags::new();
        tags.push("Enemy");

        let cframe = CFrame::new(Vector3::new(1.0, 2.0, 3.0), Matrix3::identity());
        let mut dom = WeakDom::new(InstanceBuilder::new("DataModel").with_name("game"));
        let root = dom.root_ref();
        let workspace = dom.insert(root, InstanceBuilder::new("Workspace"));
        dom.insert(
            workspace,
            InstanceBuilder::new("Part")
                .with_name("Block")
                .with_property("Color", Color3::new(0.1, 0.2, 0.3))
                .with_property("CFrame", cframe)
                .with_property("Attributes", attrs)
                .with_property("Tags", tags),
        );

        let result = import_dom(&dom, &test_options(), PlaceFileFormat::Rbxl);
        let part = result
            .instances
            .iter()
            .find(|inst| inst["name"] == "Block")
            .unwrap();

        assert_eq!(part["properties"]["Color"]["type"], "Color3");
        assert_eq!(
            part["properties"]["CFrame"]["value"]["position"],
            json!([1.0, 2.0, 3.0])
        );
        assert_eq!(part["attributes"]["Level"]["value"], 7);
        assert_eq!(part["tags"], json!(["Enemy"]));
    }

    #[test]
    fn serializes_binary_and_shared_string_properties() {
        let binary = Variant::BinaryString(BinaryString::from(vec![1, 2, 3, 4]));
        let shared = Variant::SharedString(SharedString::new(vec![5, 6, 7, 8]));

        assert_eq!(
            variant_to_json_property(&binary).unwrap(),
            json!({ "type": "BinaryString", "value": "AQIDBA==" })
        );

        let shared_json = variant_to_json_property(&shared).unwrap();
        assert_eq!(shared_json["type"], "SharedString");
        assert_eq!(shared_json["value"]["data"], "BQYHCA==");
        assert!(shared_json["value"]["hash"].as_str().unwrap().len() > 16);
        assert!(shared_json["value"]["file"].is_null());
    }

    #[test]
    fn reports_missing_services_script_source_and_terrain_limitations() {
        let mut dom = WeakDom::new(InstanceBuilder::new("DataModel").with_name("game"));
        let root = dom.root_ref();
        let workspace = dom.insert(root, InstanceBuilder::new("Workspace"));
        dom.insert(workspace, InstanceBuilder::new("Terrain"));
        dom.insert(
            workspace,
            InstanceBuilder::new("Script").with_name("EmptyScript"),
        );

        let mut services = HashSet::new();
        services.insert("Workspace".to_string());
        services.insert("MissingService".to_string());

        let result = import_dom(
            &dom,
            &PlaceImportOptions {
                input_path: PathBuf::from("test.rbxl"),
                services: Some(services),
                include_terrain: true,
            },
            PlaceFileFormat::Rbxl,
        );

        let kinds = result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect::<Vec<_>>();

        assert!(kinds.contains(&ImportDiagnosticKind::MissingService));
        assert!(kinds.contains(&ImportDiagnosticKind::MissingScriptSource));
        assert!(kinds.contains(&ImportDiagnosticKind::UnsupportedTerrainVoxelData));
    }

    #[test]
    fn imports_xml_place_files() {
        let temp = tempfile::tempdir().unwrap();
        let place_path = temp.path().join("basic.rbxlx");

        let mut dom = WeakDom::new(InstanceBuilder::new("DataModel").with_name("game"));
        let root = dom.root_ref();
        dom.insert(root, InstanceBuilder::new("Workspace"));
        let refs_to_export = dom.root().children().to_vec();

        let file = File::create(&place_path).unwrap();
        rbx_xml::to_writer_default(file, &dom, &refs_to_export).unwrap();

        let result = import_place_file(PlaceImportOptions {
            input_path: place_path,
            services: None,
            include_terrain: true,
        })
        .unwrap();

        assert_eq!(result.format, PlaceFileFormat::Rbxlx);
        assert_eq!(result.instances.len(), 1);
        assert_eq!(result.instances[0]["path"], "Workspace");
    }

    #[test]
    fn filters_services_and_terrain() {
        let mut dom = WeakDom::new(InstanceBuilder::new("DataModel").with_name("game"));
        let root = dom.root_ref();
        let workspace = dom.insert(root, InstanceBuilder::new("Workspace"));
        dom.insert(workspace, InstanceBuilder::new("Terrain"));
        dom.insert(root, InstanceBuilder::new("Lighting"));

        let mut services = HashSet::new();
        services.insert("Workspace".to_string());

        let result = import_dom(
            &dom,
            &PlaceImportOptions {
                input_path: PathBuf::from("test.rbxl"),
                services: Some(services),
                include_terrain: false,
            },
            PlaceFileFormat::Rbxl,
        );

        let paths: Vec<_> = result
            .instances
            .iter()
            .map(|inst| inst["path"].as_str().unwrap())
            .collect();
        assert_eq!(paths, vec!["Workspace"]);
    }
}
