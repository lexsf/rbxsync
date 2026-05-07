use std::path::Path;
use std::process::Command;

fn rbxsync() -> &'static str {
    env!("CARGO_BIN_EXE_rbxsync")
}

fn command() -> Command {
    let mut command = Command::new(rbxsync());
    command.env("RBXSYNC_VERSION_CHECK", "1");
    command
}

fn write_fixture_project(project_dir: &Path) {
    let workspace = project_dir.join("src/Workspace");
    let server = project_dir.join("src/ServerScriptService");
    std::fs::create_dir_all(&workspace).expect("workspace dir");
    std::fs::create_dir_all(&server).expect("server dir");

    std::fs::write(
        workspace.join("Baseplate.rbxjson"),
        r#"{
  "className": "Part",
  "name": "Baseplate",
  "properties": {
    "Anchored": { "type": "bool", "value": true },
    "Position": { "type": "Vector3", "value": { "x": 1.0, "y": 2.0, "z": 3.0 } }
  }
}"#,
    )
    .expect("baseplate metadata");

    std::fs::write(server.join("Main.server.luau"), "print('import fixture')").expect("script");
}

fn build_place(source_project: &Path, output: &Path, format: &str) {
    let status = command()
        .args([
            "build",
            "--path",
            source_project.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--format",
            format,
        ])
        .status()
        .expect("run build");

    assert!(status.success(), "build command failed");
    assert!(output.exists(), "build output missing");
}

#[test]
fn import_place_writes_project_from_binary_place() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_project = temp.path().join("source");
    let place_file = temp.path().join("basic.rbxl");
    let imported_project = temp.path().join("imported");
    write_fixture_project(&source_project);
    build_place(&source_project, &place_file, "rbxl");

    let output = command()
        .args([
            "import-place",
            place_file.to_str().unwrap(),
            "--output",
            imported_project.to_str().unwrap(),
            "--force",
            "--json",
        ])
        .output()
        .expect("run import-place");

    assert!(
        output.status.success(),
        "import-place failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let summary: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("clean json stdout");
    assert_eq!(summary["success"], true);
    assert_eq!(summary["format"], "rbxl");
    assert_eq!(summary["scriptsWritten"], 1);
    assert_eq!(summary["jsonFilesWritten"], 4);
    assert_eq!(summary["diagnosticCount"], 0);

    assert!(imported_project.join("rbxsync.json").exists());
    assert!(imported_project.join("default.project.json").exists());
    assert!(imported_project
        .join("src/Workspace/_meta.rbxjson")
        .exists());
    assert!(imported_project
        .join("src/Workspace/Baseplate.rbxjson")
        .exists());
    assert!(imported_project
        .join("src/ServerScriptService/Main.server.luau")
        .exists());

    let script_meta =
        std::fs::read_to_string(imported_project.join("src/ServerScriptService/Main.rbxjson"))
            .expect("script metadata");
    assert!(
        !script_meta.contains("\"Source\""),
        "script Source should be split out of metadata"
    );
}

#[test]
fn import_place_dry_run_for_xml_place_does_not_write_project() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_project = temp.path().join("source");
    let place_file = temp.path().join("basic.rbxlx");
    let imported_project = temp.path().join("imported");
    write_fixture_project(&source_project);
    build_place(&source_project, &place_file, "rbxlx");

    let output = command()
        .args([
            "import-place",
            place_file.to_str().unwrap(),
            "--output",
            imported_project.to_str().unwrap(),
            "--dry-run",
            "--json",
        ])
        .output()
        .expect("run import-place dry-run");

    assert!(
        output.status.success(),
        "dry-run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let summary: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("clean json stdout");
    assert_eq!(summary["success"], true);
    assert_eq!(summary["dryRun"], true);
    assert_eq!(summary["format"], "rbxlx");
    assert_eq!(summary["totalInstances"], 4);
    assert_eq!(summary["scripts"], 1);
    assert!(
        !imported_project.exists(),
        "dry-run should not create output project"
    );
}

#[test]
fn import_place_reports_missing_requested_service() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_project = temp.path().join("source");
    let place_file = temp.path().join("basic.rbxlx");
    let imported_project = temp.path().join("imported");
    write_fixture_project(&source_project);
    build_place(&source_project, &place_file, "rbxlx");

    let output = command()
        .args([
            "import-place",
            place_file.to_str().unwrap(),
            "--output",
            imported_project.to_str().unwrap(),
            "--services",
            "Workspace,MissingService",
            "--dry-run",
            "--json",
        ])
        .output()
        .expect("run import-place dry-run");

    assert!(
        output.status.success(),
        "dry-run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let summary: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("clean json stdout");
    assert_eq!(summary["success"], true);
    assert_eq!(summary["diagnosticCount"], 1);
    assert_eq!(summary["diagnosticSummary"]["missingService"], 1);
    assert_eq!(summary["diagnostics"][0]["kind"], "missingService");
    assert_eq!(summary["services"], serde_json::json!(["Workspace"]));
}
