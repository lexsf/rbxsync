use std::process::Command;

fn rbxsync() -> &'static str {
    env!("CARGO_BIN_EXE_rbxsync")
}

fn command() -> Command {
    let mut command = Command::new(rbxsync());
    command.env("RBXSYNC_VERSION_CHECK", "1");
    command
}

#[test]
fn publish_place_dry_run_json_does_not_upload_or_leak_api_key() {
    let temp = tempfile::tempdir().expect("tempdir");
    let place_file = temp.path().join("game.rbxl");
    std::fs::write(&place_file, b"placeholder").expect("place file");

    let output = command()
        .args([
            "publish-place",
            place_file.to_str().unwrap(),
            "--universe-id",
            "123",
            "--place-id",
            "456",
            "--api-key",
            "test-secret-key",
            "--dry-run",
            "--json",
        ])
        .output()
        .expect("run publish-place dry-run");

    assert!(
        output.status.success(),
        "dry-run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains("test-secret-key"));
    assert!(!stderr.contains("test-secret-key"));

    let summary: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("clean json stdout");
    assert_eq!(summary["success"], true);
    assert_eq!(summary["command"], "publish-place");
    assert_eq!(summary["dryRun"], true);
    assert_eq!(summary["format"], "rbxl");
    assert_eq!(summary["contentType"], "application/octet-stream");
    assert_eq!(summary["bytes"], 11);
    assert_eq!(summary["universeId"], 123);
    assert_eq!(summary["placeId"], 456);
    assert_eq!(summary["versionType"], "Published");
    assert_eq!(summary["versionNumber"], serde_json::Value::Null);
    assert_eq!(summary["diagnosticCount"], 0);
}

#[test]
fn publish_place_uses_api_key_environment_variable_for_dry_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let place_file = temp.path().join("game.rbxlx");
    std::fs::write(&place_file, b"<roblox />").expect("place file");

    let output = command()
        .env("ROBLOX_OPEN_CLOUD_API_KEY", "env-secret-key")
        .args([
            "publish-place",
            place_file.to_str().unwrap(),
            "--universe-id",
            "123",
            "--place-id",
            "456",
            "--version-type",
            "saved",
            "--dry-run",
            "--json",
        ])
        .output()
        .expect("run publish-place dry-run");

    assert!(
        output.status.success(),
        "dry-run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains("env-secret-key"));
    assert!(!stderr.contains("env-secret-key"));

    let summary: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("clean json stdout");
    assert_eq!(summary["format"], "rbxlx");
    assert_eq!(summary["contentType"], "application/xml");
    assert_eq!(summary["versionType"], "Saved");
}

#[test]
fn publish_place_requires_api_key_before_upload() {
    let temp = tempfile::tempdir().expect("tempdir");
    let place_file = temp.path().join("game.rbxl");
    std::fs::write(&place_file, b"placeholder").expect("place file");

    let output = command()
        .env_remove("ROBLOX_OPEN_CLOUD_API_KEY")
        .args([
            "publish-place",
            place_file.to_str().unwrap(),
            "--universe-id",
            "123",
            "--place-id",
            "456",
            "--dry-run",
            "--json",
        ])
        .output()
        .expect("run publish-place without api key");

    assert!(!output.status.success(), "missing API key should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Open Cloud API key is required"));
}

#[test]
fn publish_place_requires_yes_for_non_interactive_json_publish() {
    let temp = tempfile::tempdir().expect("tempdir");
    let place_file = temp.path().join("game.rbxl");
    std::fs::write(&place_file, b"placeholder").expect("place file");

    let output = command()
        .args([
            "publish-place",
            place_file.to_str().unwrap(),
            "--universe-id",
            "123",
            "--place-id",
            "456",
            "--api-key",
            "test-secret-key",
            "--json",
        ])
        .output()
        .expect("run publish-place without confirmation");

    assert!(!output.status.success(), "missing --yes should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Re-run with --yes"));
    assert!(!stderr.contains("test-secret-key"));
}

#[test]
fn publish_place_rejects_unsupported_place_extension() {
    let temp = tempfile::tempdir().expect("tempdir");
    let place_file = temp.path().join("game.txt");
    std::fs::write(&place_file, b"placeholder").expect("place file");

    let output = command()
        .args([
            "publish-place",
            place_file.to_str().unwrap(),
            "--universe-id",
            "123",
            "--place-id",
            "456",
            "--api-key",
            "test-secret-key",
            "--dry-run",
            "--json",
        ])
        .output()
        .expect("run publish-place with unsupported extension");

    assert!(
        !output.status.success(),
        "unsupported extension should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("only supports .rbxl and .rbxlx"));
    assert!(!stderr.contains("test-secret-key"));
}
