//! RbxSync Core Library
//!
//! This crate provides the core functionality for RbxSync:
//! - Roblox property type definitions and serialization
//! - Instance representation
//! - Project configuration
//! - Plugin building (.rbxm generation)
//! - Rojo project file parsing and migration
//! - Luau obfuscation for build-time transforms

pub mod extract_writer;
pub mod obfuscator;
pub mod path_utils;
pub mod place_exporter;
pub mod place_importer;
pub mod place_publisher;
pub mod plugin_builder;
pub mod rojo;
pub mod types;

// Re-export commonly used types
pub use extract_writer::{write_serialized_instances, ExtractWriterOptions, ExtractWriterSummary};
pub use obfuscator::{ObfuscationResult, Obfuscator, ObfuscatorConfig};
pub use path_utils::{
    normalize_path, path_to_string, path_with_suffix, pathbuf_with_suffix, sanitize_filename,
};
pub use place_exporter::{
    build_dom_from_project, export_place, PlaceExportDiagnostic, PlaceExportDiagnosticKind,
    PlaceExportFormat, PlaceExportOptions, PlaceExportSummary,
};
pub use place_importer::{
    import_place_file, ImportDiagnostic, ImportDiagnosticKind, PlaceFileFormat, PlaceImportOptions,
    PlaceImportResult,
};
pub use place_publisher::{
    publish_place, publish_place_url, publish_place_with_transport, PublishPlaceDiagnostic,
    PublishPlaceDiagnosticKind, PublishPlaceFormat, PublishPlaceHttpRequest,
    PublishPlaceHttpResponse, PublishPlaceOptions, PublishPlaceSummary, PublishPlaceTransport,
    PublishVersionType, ReqwestPublishPlaceTransport,
};
pub use plugin_builder::{
    build_plugin, build_plugin_with_stats, find_existing_rbxsync_plugin, get_studio_plugins_folder,
    install_plugin, PluginBuildConfig, PluginBuildStats,
};
pub use rojo::{
    find_rojo_project, parse_rojo_project, rojo_to_tree_mapping, RojoError, RojoProject, RojoTree,
};
pub use types::{
    find_wally_lock,
    find_wally_manifest,
    is_package_path,
    AttributeValue,
    CFrame,
    Color3,
    EnumValue,
    // Harness system for multi-session AI development
    Feature,
    FeaturePriority,
    FeatureStatus,
    FeaturesFile,
    GameDefinition,
    HarnessState,
    Instance,
    InstanceMeta,
    // Wally package support
    PackageConfig,
    PackageDirectories,
    ProjectConfig,
    PropertyValue,
    SessionLog,
    SessionLogEntry,
    Vector2,
    Vector3,
    WallyError,
    WallyLock,
    WallyLockedPackage,
    WallyManifest,
    WallyPackageInfo,
};
