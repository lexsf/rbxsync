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
  "referenceId": "baseplate-ref",
  "properties": {
    "Anchored": { "type": "bool", "value": true },
    "Position": { "type": "Vector3", "value": { "x": 1.0, "y": 2.0, "z": 3.0 } }
  },
  "attributes": {
    "Level": { "type": "int", "value": 7 }
  },
  "tags": ["fixture"]
}"#,
    )
    .expect("baseplate metadata");

    std::fs::write(server.join("Main.server.luau"), "print('extract fixture')").expect("script");
}

fn extract_place(
    source_project: &Path,
    output_path: &Path,
    format: Option<&str>,
) -> serde_json::Value {
    let mut command = command();
    command.args([
        "extract-place",
        "--path",
        source_project.to_str().unwrap(),
        "--output",
        output_path.to_str().unwrap(),
        "--force",
        "--json",
    ]);
    if let Some(format) = format {
        command.args(["--format", format]);
    }

    let output = command.output().expect("run extract-place");
    assert!(
        output.status.success(),
        "extract-place failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("clean json stdout")
}

#[test]
fn extract_place_writes_binary_place_and_reimports_project() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_project = temp.path().join("source");
    let place_file = temp.path().join("game.rbxl");
    let imported_project = temp.path().join("imported");
    write_fixture_project(&source_project);

    let summary = extract_place(&source_project, &place_file, Some("rbxl"));
    assert_eq!(summary["success"], true);
    assert_eq!(summary["command"], "extract-place");
    assert_eq!(summary["format"], "rbxl");
    assert_eq!(summary["scripts"], 1);
    assert_eq!(summary["diagnosticCount"], 0);
    assert!(summary["bytesWritten"].as_u64().unwrap_or_default() > 0);
    assert!(place_file.exists(), "place output missing");

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
    let import_summary: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("clean import json");
    assert_eq!(import_summary["success"], true);
    assert_eq!(import_summary["scriptsWritten"], 1);
    assert!(imported_project
        .join("src/ServerScriptService/Main.server.luau")
        .exists());
    assert!(imported_project
        .join("src/Workspace/Baseplate.rbxjson")
        .exists());
}

#[test]
fn extract_place_writes_xml_place_from_output_extension() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_project = temp.path().join("source");
    let place_file = temp.path().join("game.rbxlx");
    write_fixture_project(&source_project);

    let summary = extract_place(&source_project, &place_file, None);
    assert_eq!(summary["success"], true);
    assert_eq!(summary["format"], "rbxlx");
    assert!(summary["bytesWritten"].as_u64().unwrap_or_default() > 0);
    assert!(place_file.exists(), "xml place output missing");
}

#[test]
fn extract_place_dry_run_json_does_not_write_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_project = temp.path().join("source");
    let place_file = temp.path().join("dry-run.rbxl");
    write_fixture_project(&source_project);

    let output = command()
        .args([
            "extract-place",
            "--path",
            source_project.to_str().unwrap(),
            "--output",
            place_file.to_str().unwrap(),
            "--dry-run",
            "--json",
        ])
        .output()
        .expect("run extract-place dry-run");
    assert!(
        output.status.success(),
        "dry-run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let summary: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("clean json stdout");
    assert_eq!(summary["success"], true);
    assert_eq!(summary["dryRun"], true);
    assert_eq!(summary["bytesWritten"], serde_json::Value::Null);
    assert_eq!(summary["scripts"], 1);
    assert!(!place_file.exists(), "dry-run should not write output");
}

#[test]
fn extract_place_requires_force_for_existing_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_project = temp.path().join("source");
    let place_file = temp.path().join("game.rbxl");
    write_fixture_project(&source_project);
    extract_place(&source_project, &place_file, Some("rbxl"));

    let output = command()
        .args([
            "extract-place",
            "--path",
            source_project.to_str().unwrap(),
            "--output",
            place_file.to_str().unwrap(),
        ])
        .output()
        .expect("run extract-place without force");

    assert!(!output.status.success(), "existing output should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Output file already exists"),
        "unexpected stderr: {}",
        stderr
    );
}

#[test]
fn extract_place_rejects_format_output_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let source_project = temp.path().join("source");
    let place_file = temp.path().join("game.rbxl");
    write_fixture_project(&source_project);

    let output = command()
        .args([
            "extract-place",
            "--path",
            source_project.to_str().unwrap(),
            "--output",
            place_file.to_str().unwrap(),
            "--format",
            "rbxlx",
        ])
        .output()
        .expect("run extract-place mismatch");

    assert!(!output.status.success(), "format mismatch should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not match output extension"),
        "unexpected stderr: {}",
        stderr
    );
}
