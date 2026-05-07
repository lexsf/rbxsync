//! Shared extraction writer for RbxSync serialized instance data.
//!
//! This module owns the filesystem layout produced by Studio extraction. Both
//! the HTTP server finalizer and future command-line place import should call
//! this code so the on-disk RbxSync format stays consistent.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use futures::stream::{self, StreamExt};
use serde_json::Value;

/// Options controlling how serialized instances are written to disk.
#[derive(Debug, Clone)]
pub struct ExtractWriterOptions {
    pub project_dir: PathBuf,
    pub tree_mapping: HashMap<String, String>,
    pub preserve_packages: bool,
    pub packages_folder: String,
    pub generate_tooling_files: bool,
    pub project_name: Option<String>,
}

/// Summary of the files written by the shared extraction writer.
#[derive(Debug, Clone)]
pub struct ExtractWriterSummary {
    pub total_instances: usize,
    pub files_written: usize,
    pub scripts_written: usize,
    pub service_count: usize,
    pub packages_preserved: bool,
    pub warnings: Vec<String>,
}

/// Apply tree mapping to convert DataModel path to filesystem path.
fn apply_tree_mapping(datamodel_path: &str, tree_mapping: &HashMap<String, String>) -> String {
    let mut best_match: Option<(&str, &str)> = None;
    let mut best_len = 0;

    for (dm_prefix, fs_prefix) in tree_mapping {
        if (datamodel_path == dm_prefix || datamodel_path.starts_with(&format!("{}/", dm_prefix)))
            && dm_prefix.len() > best_len
        {
            best_match = Some((dm_prefix.as_str(), fs_prefix.as_str()));
            best_len = dm_prefix.len();
        }
    }

    if let Some((dm_prefix, fs_prefix)) = best_match {
        if datamodel_path == dm_prefix {
            fs_prefix.to_string()
        } else {
            let suffix = &datamodel_path[dm_prefix.len() + 1..];
            format!("{}/{}", fs_prefix, suffix)
        }
    } else {
        datamodel_path.to_string()
    }
}

const SKIP_DIRS: &[&str] = &[
    ".rbxsync-trash",
    ".rbxsync-backup",
    ".rbxsync",
    ".git",
    "node_modules",
];

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    let resolved_src = src.canonicalize().unwrap_or_else(|_| src.to_path_buf());
    let resolved_dst = dst.canonicalize().unwrap_or_else(|_| dst.to_path_buf());

    if resolved_dst.starts_with(&resolved_src) {
        tracing::warn!("Skipping circular copy: {:?} is inside {:?}", dst, src);
        return Ok(());
    }

    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        if path.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                if SKIP_DIRS.contains(&name) {
                    continue;
                }
            }
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}

/// Known Roblox services for project.json generation.
const KNOWN_SERVICES: &[(&str, &str)] = &[
    ("Workspace", "Workspace"),
    ("ServerScriptService", "ServerScriptService"),
    ("ServerStorage", "ServerStorage"),
    ("ReplicatedStorage", "ReplicatedStorage"),
    ("ReplicatedFirst", "ReplicatedFirst"),
    ("StarterGui", "StarterGui"),
    ("StarterPack", "StarterPack"),
    ("StarterPlayer", "StarterPlayer"),
    ("StarterPlayerScripts", "StarterPlayerScripts"),
    ("StarterCharacterScripts", "StarterCharacterScripts"),
    ("Players", "Players"),
    ("Lighting", "Lighting"),
    ("SoundService", "SoundService"),
    ("Chat", "Chat"),
    ("LocalizationService", "LocalizationService"),
    ("TestService", "TestService"),
    ("Teams", "Teams"),
    ("TextChatService", "TextChatService"),
    ("VoiceChatService", "VoiceChatService"),
];

fn generate_tooling_files(
    project_dir: &Path,
    service_folders: &HashSet<String>,
    generate_tooling_files: bool,
    project_name: Option<&str>,
) {
    if !generate_tooling_files {
        tracing::info!("Tooling file generation disabled in config");
        return;
    }

    let src_dir = project_dir.join("src");
    let project_name = project_name.map(str::to_string).unwrap_or_else(|| {
        project_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("MyGame")
            .to_string()
    });

    let project_json_path = project_dir.join("default.project.json");
    if !project_json_path.exists() && src_dir.exists() {
        if let Ok(project_json) = generate_project_json(&project_name, &src_dir, service_folders) {
            match std::fs::write(&project_json_path, project_json) {
                Ok(_) => tracing::info!("Generated default.project.json"),
                Err(e) => tracing::warn!("Failed to write default.project.json: {}", e),
            }
        }
    }

    let selene_toml_path = project_dir.join("selene.toml");
    if !selene_toml_path.exists() {
        let selene_content = r#"std = "roblox"
"#;
        match std::fs::write(&selene_toml_path, selene_content) {
            Ok(_) => tracing::info!("Generated selene.toml"),
            Err(e) => tracing::warn!("Failed to write selene.toml: {}", e),
        }
    }

    let wally_toml_path = project_dir.join("wally.toml");
    if !wally_toml_path.exists() {
        let sanitized_name: String = project_name
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let wally_content = format!(
            r#"[package]
name = "your-username/{}"
version = "0.1.0"
registry = "https://github.com/UpliftGames/wally-index"
realm = "shared"

[dependencies]
"#,
            sanitized_name
        );
        match std::fs::write(&wally_toml_path, wally_content) {
            Ok(_) => tracing::info!("Generated wally.toml"),
            Err(e) => tracing::warn!("Failed to write wally.toml: {}", e),
        }
    }
}

fn generate_project_json(
    project_name: &str,
    src_dir: &Path,
    service_folders: &HashSet<String>,
) -> std::result::Result<String, serde_json::Error> {
    let mut tree = serde_json::json!({
        "$className": "DataModel"
    });

    let service_map: HashMap<&str, &str> = KNOWN_SERVICES.iter().cloned().collect();

    for service_name in service_folders {
        let class_name = service_map.get(service_name.as_str());

        if service_name == "StarterPlayer" {
            let starter_player_dir = src_dir.join("StarterPlayer");
            if starter_player_dir.exists() {
                let mut sp_node = serde_json::json!({
                    "$className": "StarterPlayer"
                });

                if let Ok(entries) = std::fs::read_dir(&starter_player_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            let child_name = entry.file_name().to_string_lossy().to_string();
                            let child_class = service_map.get(child_name.as_str());
                            if let Some(class) = child_class {
                                sp_node[&child_name] = serde_json::json!({
                                    "$className": class,
                                    "$path": format!("src/StarterPlayer/{}", child_name)
                                });
                            } else {
                                sp_node[&child_name] = serde_json::json!({
                                    "$path": format!("src/StarterPlayer/{}", child_name)
                                });
                            }
                        }
                    }
                }

                tree[service_name] = sp_node;
                continue;
            }
        }

        if let Some(class) = class_name {
            tree[service_name] = serde_json::json!({
                "$className": class,
                "$path": format!("src/{}", service_name)
            });
        } else {
            tree[service_name] = serde_json::json!({
                "$path": format!("src/{}", service_name)
            });
        }
    }

    let project = serde_json::json!({
        "name": project_name,
        "tree": tree,
        "globIgnorePaths": ["**/node_modules"]
    });

    serde_json::to_string_pretty(&project)
}

/// Write plugin-compatible serialized instances into a RbxSync project `src/` tree.
pub async fn write_serialized_instances(
    all_instances: Vec<Value>,
    options: ExtractWriterOptions,
) -> Result<ExtractWriterSummary> {
    let src_dir = options.project_dir.join("src");
    let backup_dir = options.project_dir.join(".rbxsync-backup");
    let backup_src = backup_dir.join("src");
    let mut warnings = Vec::new();

    let terrain_file = src_dir
        .join("Workspace")
        .join("Terrain")
        .join("terrain.rbxjson");
    let terrain_data = if terrain_file.exists() {
        tracing::info!("Backing up terrain.rbxjson before finalize");
        std::fs::read_to_string(&terrain_file).ok()
    } else {
        None
    };

    if src_dir.exists() {
        if backup_src.exists() {
            let _ = std::fs::remove_dir_all(&backup_src);
        }
        let _ = std::fs::create_dir_all(&backup_dir);
        if let Err(e) = std::fs::rename(&src_dir, &backup_src) {
            tracing::warn!("Rename failed, falling back to copy: {}", e);
            if let Err(e) = copy_dir_recursive(&src_dir, &backup_src) {
                let message = format!("Failed to backup src directory: {}", e);
                tracing::warn!("{}", message);
                warnings.push(message);
            }

            for entry in std::fs::read_dir(&src_dir)
                .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
                .flatten()
            {
                let path = entry.path();
                if path.is_dir() {
                    let _ = std::fs::remove_dir_all(&path);
                } else {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
        tracing::info!("Backed up src to .rbxsync-backup/src");
    }

    tracing::info!(
        "Finalizing {} instances to {}",
        all_instances.len(),
        src_dir.display()
    );

    let _ = std::fs::create_dir_all(&src_dir);

    if let Some(data) = terrain_data {
        let terrain_dir = src_dir.join("Workspace").join("Terrain");
        let _ = std::fs::create_dir_all(&terrain_dir);
        if std::fs::write(terrain_dir.join("terrain.rbxjson"), &data).is_ok() {
            tracing::info!("Restored terrain.rbxjson after finalize");
        }
    }

    let mut service_folders: HashSet<String> = HashSet::new();
    let mut path_to_count: HashMap<String, usize> = HashMap::new();
    let mut ref_to_path: HashMap<String, String> = HashMap::new();
    let mut duplicate_count = 0;

    for inst in &all_instances {
        if let Some(path) = inst.get("path").and_then(|v| v.as_str()) {
            if !path.is_empty() {
                let ref_id = inst
                    .get("referenceId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let count = path_to_count.entry(path.to_string()).or_insert(0);
                *count += 1;

                let disambiguated_path = if *count > 1 {
                    let suffix = if ref_id.len() >= 8 {
                        &ref_id[..8]
                    } else {
                        ref_id
                    };
                    let class_name = inst
                        .get("className")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let message = format!(
                        "Duplicate instance path detected: '{}' ({}). Disambiguating to '{}_{}'",
                        path, class_name, path, suffix
                    );
                    tracing::warn!("{}", message);
                    warnings.push(message);
                    duplicate_count += 1;
                    format!("{}_{}", path, suffix)
                } else {
                    path.to_string()
                };

                if !ref_id.is_empty() {
                    ref_to_path.insert(ref_id.to_string(), disambiguated_path);
                }
            }
        }
    }

    if duplicate_count > 0 {
        tracing::info!(
            "Found {} duplicate instance paths - these have been disambiguated",
            duplicate_count
        );
    }

    let all_paths: HashSet<String> = ref_to_path.values().cloned().collect();
    let has_children = |path: &str| -> bool {
        let prefix = format!("{}/", path);
        all_paths.iter().any(|p| p.starts_with(&prefix))
    };

    let normalize_path = |path: &str| -> String {
        let mut normalized = path.to_string();
        let patterns = [
            ("Packages/Packages/", "Packages/"),
            ("packages/packages/", "packages/"),
            ("Packages/packages/", "Packages/"),
            ("packages/Packages/", "Packages/"),
        ];

        for (from, to) in patterns {
            while normalized.contains(from) {
                normalized = normalized.replace(from, to);
            }
        }

        normalized
    };

    const MAX_CONCURRENT_WRITES: usize = 64;

    struct WriteOp {
        path: PathBuf,
        content: String,
    }

    let mut directories_needed: HashSet<PathBuf> = HashSet::new();
    let mut script_write_ops: Vec<WriteOp> = Vec::new();
    let mut json_write_ops: Vec<WriteOp> = Vec::new();

    tracing::info!(
        "Preparing {} instances for parallel write...",
        all_instances.len()
    );
    let prep_start = std::time::Instant::now();

    for inst in &all_instances {
        let class_name = inst
            .get("className")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        let ref_id = inst
            .get("referenceId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let inst_path = if !ref_id.is_empty() {
            ref_to_path.get(ref_id).map(|s| s.as_str()).unwrap_or("")
        } else {
            inst.get("path").and_then(|v| v.as_str()).unwrap_or("")
        };
        if inst_path.is_empty() {
            continue;
        }

        let inst_path = normalize_path(inst_path);
        let fs_path = apply_tree_mapping(&inst_path, &options.tree_mapping);
        let full_path = src_dir.join(&fs_path);

        if let Some(service_name) = fs_path.split('/').next() {
            service_folders.insert(service_name.to_string());
        }

        if let Some(parent) = full_path.parent() {
            directories_needed.insert(parent.to_path_buf());
        }

        let is_container = has_children(&inst_path);
        let is_script = matches!(class_name, "Script" | "LocalScript" | "ModuleScript");

        if is_script {
            if let Some(props) = inst.get("properties") {
                if let Some(source) = props
                    .get("Source")
                    .and_then(|v| v.get("value"))
                    .and_then(|v| v.as_str())
                {
                    let extension = match class_name {
                        "Script" => ".server.luau",
                        "LocalScript" => ".client.luau",
                        _ => ".luau",
                    };
                    let script_path = crate::path_with_suffix(&full_path, extension);
                    script_write_ops.push(WriteOp {
                        path: PathBuf::from(script_path),
                        content: source.to_string(),
                    });
                }
            }
        }

        let json_path = if is_container {
            directories_needed.insert(full_path.clone());
            full_path.join("_meta.rbxjson")
        } else {
            crate::pathbuf_with_suffix(&full_path, ".rbxjson")
        };

        let mut clean_inst = inst.clone();
        if is_script {
            if let Some(props) = clean_inst.get_mut("properties") {
                if let Some(obj) = props.as_object_mut() {
                    obj.remove("Source");
                }
            }
        }

        if let Ok(json) = serde_json::to_string_pretty(&clean_inst) {
            json_write_ops.push(WriteOp {
                path: json_path,
                content: json,
            });
        }
    }

    tracing::info!(
        "Preparation complete in {:?}: {} directories, {} scripts, {} json files",
        prep_start.elapsed(),
        directories_needed.len(),
        script_write_ops.len(),
        json_write_ops.len()
    );

    let dirs_to_create: Vec<PathBuf> = directories_needed.into_iter().collect();
    let dir_count = dirs_to_create.len();

    let dir_start = std::time::Instant::now();
    tokio::task::spawn_blocking(move || {
        for dir in dirs_to_create {
            let _ = std::fs::create_dir_all(&dir);
        }
    })
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Failed to create directories: {}", e);
    });
    tracing::info!(
        "Created {} directories in {:?}",
        dir_count,
        dir_start.elapsed()
    );

    let write_start = std::time::Instant::now();

    let script_count = script_write_ops.len();
    let script_results: Vec<bool> = stream::iter(script_write_ops)
        .map(|op| async move { tokio::fs::write(&op.path, &op.content).await.is_ok() })
        .buffer_unordered(MAX_CONCURRENT_WRITES)
        .collect()
        .await;
    let scripts_written = script_results.iter().filter(|&&ok| ok).count();

    let json_count = json_write_ops.len();
    let json_results: Vec<bool> = stream::iter(json_write_ops)
        .map(|op| async move { tokio::fs::write(&op.path, &op.content).await.is_ok() })
        .buffer_unordered(MAX_CONCURRENT_WRITES)
        .collect()
        .await;
    let files_written = json_results.iter().filter(|&&ok| ok).count();

    tracing::info!(
        "Wrote {} scripts and {} json files in {:?} ({} concurrent writes)",
        scripts_written,
        files_written,
        write_start.elapsed(),
        MAX_CONCURRENT_WRITES
    );

    let script_failures = script_count - scripts_written;
    let json_failures = json_count - files_written;
    if script_failures > 0 || json_failures > 0 {
        let message = format!(
            "Write failures: {} scripts, {} json files",
            script_failures, json_failures
        );
        tracing::warn!("{}", message);
        warnings.push(message);
    }

    if let Ok(entries) = std::fs::read_dir(&src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("chunk_") && name.ends_with(".json") {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    for service in &service_folders {
        let service_folder = src_dir.join(service);
        let _ = std::fs::create_dir_all(&service_folder);
    }

    let mut packages_preserved = false;
    if options.preserve_packages {
        let package_restore_locations: Vec<(String, String)> = vec![
            (
                "ReplicatedStorage/Packages".to_string(),
                "ReplicatedStorage/Packages".to_string(),
            ),
            (
                "ServerScriptService/Packages".to_string(),
                "ServerScriptService/Packages".to_string(),
            ),
            (
                "ServerStorage/Packages".to_string(),
                "ServerStorage/Packages".to_string(),
            ),
            (
                options.packages_folder.clone(),
                options.packages_folder.clone(),
            ),
        ];

        for (backup_rel, dest_rel) in &package_restore_locations {
            let backup_packages = backup_src.join(backup_rel);
            let dest_packages = src_dir.join(dest_rel);

            if backup_packages.exists() && backup_packages.is_dir() {
                if dest_packages.exists() {
                    let _ = std::fs::remove_dir_all(&dest_packages);
                }

                if let Some(parent) = dest_packages.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                if let Err(e) = copy_dir_recursive(&backup_packages, &dest_packages) {
                    let message = format!("Failed to restore packages from {}: {}", backup_rel, e);
                    tracing::warn!("{}", message);
                    warnings.push(message);
                } else {
                    tracing::info!("Restored Wally packages from backup: {}", backup_rel);
                    packages_preserved = true;
                }
            }
        }
    }

    tracing::info!(
        "Finalize complete: {} .rbxjson files, {} .luau scripts, {} services{}",
        files_written,
        scripts_written,
        service_folders.len(),
        if packages_preserved {
            ", packages preserved"
        } else {
            ""
        }
    );

    generate_tooling_files(
        &options.project_dir,
        &service_folders,
        options.generate_tooling_files,
        options.project_name.as_deref(),
    );

    Ok(ExtractWriterSummary {
        total_instances: all_instances.len(),
        files_written,
        scripts_written,
        service_count: service_folders.len(),
        packages_preserved,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn writes_service_metadata_leaf_json_and_script_source() {
        let temp = tempfile::tempdir().expect("tempdir");
        let instances = vec![
            json!({
                "className": "Workspace",
                "name": "Workspace",
                "referenceId": "workspace-ref",
                "path": "Workspace",
                "properties": {}
            }),
            json!({
                "className": "Part",
                "name": "Baseplate",
                "referenceId": "part-ref",
                "parentId": "workspace-ref",
                "path": "Workspace/Baseplate",
                "properties": {
                    "Anchored": { "type": "bool", "value": true }
                }
            }),
            json!({
                "className": "ServerScriptService",
                "name": "ServerScriptService",
                "referenceId": "sss-ref",
                "path": "ServerScriptService",
                "properties": {}
            }),
            json!({
                "className": "Script",
                "name": "Main",
                "referenceId": "script-ref",
                "parentId": "sss-ref",
                "path": "ServerScriptService/Main",
                "properties": {
                    "Source": { "type": "ProtectedString", "value": "print(\"hello\")" },
                    "Enabled": { "type": "bool", "value": true }
                }
            }),
        ];

        let summary = write_serialized_instances(
            instances,
            ExtractWriterOptions {
                project_dir: temp.path().to_path_buf(),
                tree_mapping: HashMap::new(),
                preserve_packages: false,
                packages_folder: "Packages".to_string(),
                generate_tooling_files: true,
                project_name: Some("TestGame".to_string()),
            },
        )
        .await
        .expect("write instances");

        assert_eq!(summary.total_instances, 4);
        assert_eq!(summary.files_written, 4);
        assert_eq!(summary.scripts_written, 1);

        let src = temp.path().join("src");
        assert!(src.join("Workspace/_meta.rbxjson").exists());
        assert!(src.join("Workspace/Baseplate.rbxjson").exists());
        assert!(src.join("ServerScriptService/_meta.rbxjson").exists());
        assert!(src.join("ServerScriptService/Main.rbxjson").exists());
        assert_eq!(
            std::fs::read_to_string(src.join("ServerScriptService/Main.server.luau"))
                .expect("script source"),
            "print(\"hello\")"
        );

        let script_meta: Value = serde_json::from_str(
            &std::fs::read_to_string(src.join("ServerScriptService/Main.rbxjson"))
                .expect("script metadata"),
        )
        .expect("script metadata json");
        assert!(script_meta["properties"].get("Source").is_none());
        assert!(temp.path().join("default.project.json").exists());
        assert!(temp.path().join("selene.toml").exists());
        assert!(temp.path().join("wally.toml").exists());
    }

    #[tokio::test]
    async fn backs_up_existing_src_applies_tree_mapping_and_preserves_packages() {
        let temp = tempfile::tempdir().expect("tempdir");
        let src = temp.path().join("src");
        std::fs::create_dir_all(src.join("ReplicatedStorage/Packages/Keep")).expect("packages");
        std::fs::write(src.join("old.txt"), "old contents").expect("old file");
        std::fs::write(
            src.join("ReplicatedStorage/Packages/Keep/init.luau"),
            "return {}",
        )
        .expect("package file");

        let mut tree_mapping = HashMap::new();
        tree_mapping.insert("ServerScriptService".to_string(), "server".to_string());

        let summary = write_serialized_instances(
            vec![
                json!({
                    "className": "ServerScriptService",
                    "name": "ServerScriptService",
                    "referenceId": "sss-ref",
                    "path": "ServerScriptService",
                    "properties": {}
                }),
                json!({
                    "className": "Script",
                    "name": "Main",
                    "referenceId": "script-ref",
                    "parentId": "sss-ref",
                    "path": "ServerScriptService/Main",
                    "properties": {
                        "Source": { "type": "ProtectedString", "value": "print(\"mapped\")" }
                    }
                }),
            ],
            ExtractWriterOptions {
                project_dir: temp.path().to_path_buf(),
                tree_mapping,
                preserve_packages: true,
                packages_folder: "Packages".to_string(),
                generate_tooling_files: false,
                project_name: Some("MappedGame".to_string()),
            },
        )
        .await
        .expect("write instances");

        assert_eq!(summary.total_instances, 2);
        assert_eq!(summary.scripts_written, 1);
        assert!(summary.packages_preserved);
        assert!(temp.path().join(".rbxsync-backup/src/old.txt").exists());
        assert_eq!(
            std::fs::read_to_string(src.join("server/Main.server.luau")).expect("script"),
            "print(\"mapped\")"
        );
        assert!(src.join("server/_meta.rbxjson").exists());
        assert!(src
            .join("ReplicatedStorage/Packages/Keep/init.luau")
            .exists());
        assert!(!temp.path().join("default.project.json").exists());
    }
}
