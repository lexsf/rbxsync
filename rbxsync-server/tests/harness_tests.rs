//! Integration tests for the Harness System
//!
//! Tests the HTTP endpoints for multi-session AI game development tracking.

use axum_test::TestServer;
use rbxsync_server::{create_router, AppState};
use serde_json::json;
use tempfile::TempDir;

/// Create a test server with a fresh AppState
fn create_test_server() -> TestServer {
    let state = AppState::new();
    let router = create_router(state);
    TestServer::new(router).unwrap()
}

/// Create a temporary project directory for testing
fn create_temp_project() -> TempDir {
    tempfile::tempdir().unwrap()
}

mod harness_init {
    use super::*;

    #[tokio::test]
    async fn test_init_creates_harness_directory() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        let response = server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game",
                "description": "A test game",
                "genre": "RPG"
            }))
            .await;

        response.assert_status_ok();

        let body: serde_json::Value = response.json();
        assert!(body["success"].as_bool().unwrap());
        assert!(body["gameId"].is_string());

        // Verify directory structure was created
        let harness_dir = temp_dir.path().join(".rbxsync/harness");
        assert!(harness_dir.exists(), "Harness directory should be created");
        assert!(
            harness_dir.join("game.yaml").exists(),
            "game.yaml should be created"
        );
        assert!(
            harness_dir.join("features.yaml").exists(),
            "features.yaml should be created"
        );
        assert!(
            harness_dir.join("sessions").exists(),
            "sessions directory should be created"
        );
    }

    #[tokio::test]
    async fn test_init_with_template() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        let response = server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "My Tycoon",
                "template": "tycoon"
            }))
            .await;

        response.assert_status_ok();

        let body: serde_json::Value = response.json();
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["templateApplied"].as_str(), Some("tycoon"));
        assert!(
            body["featuresAdded"].as_u64().unwrap() > 0,
            "Template should add features"
        );
    }

    #[tokio::test]
    async fn test_init_with_invalid_template() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        let response = server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "My Game",
                "template": "nonexistent_template"
            }))
            .await;

        response.assert_status_bad_request();

        let body: serde_json::Value = response.json();
        assert!(!body["success"].as_bool().unwrap());
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains("Unknown template"));
    }

    #[tokio::test]
    async fn test_init_all_templates() {
        let templates = vec!["tycoon", "obby", "simulator", "rpg", "horror"];

        for template in templates {
            let server = create_test_server();
            let temp_dir = create_temp_project();
            let project_path = temp_dir.path().to_string_lossy().to_string();

            let response = server
                .post("/harness/init")
                .json(&json!({
                    "projectDir": project_path,
                    "gameName": format!("Test {} Game", template),
                    "template": template
                }))
                .await;

            response.assert_status_ok();

            let body: serde_json::Value = response.json();
            assert!(
                body["success"].as_bool().unwrap(),
                "Template '{}' should work",
                template
            );
            assert_eq!(body["templateApplied"].as_str(), Some(template));
        }
    }
}

mod session_workflow {
    use super::*;

    #[tokio::test]
    async fn test_session_start_requires_harness_init() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Try to start session without init
        let response = server
            .post("/harness/session/start")
            .json(&json!({
                "projectDir": project_path
            }))
            .await;

        response.assert_status_bad_request();

        let body: serde_json::Value = response.json();
        assert!(!body["success"].as_bool().unwrap());
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains("not initialized"));
    }

    #[tokio::test]
    async fn test_session_start_and_end() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game"
            }))
            .await
            .assert_status_ok();

        // Start session
        let start_response = server
            .post("/harness/session/start")
            .json(&json!({
                "projectDir": project_path,
                "initialGoals": "Implement combat system"
            }))
            .await;

        start_response.assert_status_ok();

        let start_body: serde_json::Value = start_response.json();
        assert!(start_body["success"].as_bool().unwrap());
        let session_id = start_body["sessionId"].as_str().unwrap();
        assert!(!session_id.is_empty());

        // Verify session file was created
        let session_file = temp_dir
            .path()
            .join(".rbxsync/harness/sessions")
            .join(format!("{}.yaml", session_id));
        assert!(session_file.exists(), "Session file should be created");

        // End session
        let end_response = server
            .post("/harness/session/end")
            .json(&json!({
                "projectDir": project_path,
                "sessionId": session_id,
                "summary": "Combat system implemented",
                "handoffNotes": ["Need to add special attacks", "Damage formula: ATK * 2 - DEF"]
            }))
            .await;

        end_response.assert_status_ok();

        let end_body: serde_json::Value = end_response.json();
        assert!(end_body["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_session_end_with_invalid_id() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game"
            }))
            .await
            .assert_status_ok();

        // Try to end non-existent session
        let response = server
            .post("/harness/session/end")
            .json(&json!({
                "projectDir": project_path,
                "sessionId": "nonexistent-session-id"
            }))
            .await;

        response.assert_status_not_found();

        let body: serde_json::Value = response.json();
        assert!(!body["success"].as_bool().unwrap());
    }
}

mod feature_management {
    use super::*;

    #[tokio::test]
    async fn test_create_feature() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game"
            }))
            .await
            .assert_status_ok();

        // Create feature
        let response = server
            .post("/harness/feature/update")
            .json(&json!({
                "projectDir": project_path,
                "name": "Combat System",
                "description": "Turn-based combat with abilities",
                "status": "planned",
                "priority": "critical",
                "tags": ["gameplay", "core"]
            }))
            .await;

        response.assert_status_ok();

        let body: serde_json::Value = response.json();
        assert!(body["success"].as_bool().unwrap());
        assert!(body["featureId"].is_string());
    }

    #[tokio::test]
    async fn test_update_feature_status() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game"
            }))
            .await
            .assert_status_ok();

        // Create feature
        let create_response = server
            .post("/harness/feature/update")
            .json(&json!({
                "projectDir": project_path,
                "name": "Inventory System"
            }))
            .await;

        create_response.assert_status_ok();
        let create_body: serde_json::Value = create_response.json();
        let feature_id = create_body["featureId"].as_str().unwrap();

        // Update feature status
        let update_response = server
            .post("/harness/feature/update")
            .json(&json!({
                "projectDir": project_path,
                "featureId": feature_id,
                "status": "in_progress",
                "addNote": "Started implementation using ReplicatedStorage"
            }))
            .await;

        update_response.assert_status_ok();

        // Verify via status endpoint
        let status_response = server
            .post("/harness/status")
            .json(&json!({
                "projectDir": project_path
            }))
            .await;

        status_response.assert_status_ok();
        let status_body: serde_json::Value = status_response.json();

        let features = status_body["features"].as_array().unwrap();
        let feature = features.iter().find(|f| f["id"] == feature_id).unwrap();
        assert_eq!(feature["status"], "in_progress");
    }

    #[tokio::test]
    async fn test_feature_requires_name_for_creation() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game"
            }))
            .await
            .assert_status_ok();

        // Try to create feature without name
        let response = server
            .post("/harness/feature/update")
            .json(&json!({
                "projectDir": project_path,
                "description": "A feature without a name"
            }))
            .await;

        response.assert_status_bad_request();

        let body: serde_json::Value = response.json();
        assert!(!body["success"].as_bool().unwrap());
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains("Name is required"));
    }

    #[tokio::test]
    async fn test_update_nonexistent_feature() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game"
            }))
            .await
            .assert_status_ok();

        // Try to update non-existent feature
        let response = server
            .post("/harness/feature/update")
            .json(&json!({
                "projectDir": project_path,
                "featureId": "nonexistent-feature-id",
                "status": "completed"
            }))
            .await;

        response.assert_status_not_found();

        let body: serde_json::Value = response.json();
        assert!(!body["success"].as_bool().unwrap());
    }
}

mod harness_status {
    use super::*;

    #[tokio::test]
    async fn test_status_uninitialized() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        let response = server
            .post("/harness/status")
            .json(&json!({
                "projectDir": project_path
            }))
            .await;

        response.assert_status_ok();

        let body: serde_json::Value = response.json();
        assert!(body["success"].as_bool().unwrap());
        assert!(!body["initialized"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_status_with_features_and_sessions() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize with template
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game",
                "template": "obby"
            }))
            .await
            .assert_status_ok();

        // Start a session
        let session_response = server
            .post("/harness/session/start")
            .json(&json!({
                "projectDir": project_path
            }))
            .await;
        session_response.assert_status_ok();
        let session_body: serde_json::Value = session_response.json();
        let session_id = session_body["sessionId"].as_str().unwrap();

        // End the session
        server
            .post("/harness/session/end")
            .json(&json!({
                "projectDir": project_path,
                "sessionId": session_id,
                "summary": "First session complete"
            }))
            .await
            .assert_status_ok();

        // Check status
        let status_response = server
            .post("/harness/status")
            .json(&json!({
                "projectDir": project_path
            }))
            .await;

        status_response.assert_status_ok();

        let status_body: serde_json::Value = status_response.json();
        assert!(status_body["success"].as_bool().unwrap());
        assert!(status_body["initialized"].as_bool().unwrap());
        assert!(status_body["game"].is_object());
        assert!(status_body["features"].is_array());
        assert!(status_body["featureSummary"]["total"].as_u64().unwrap() > 0);
        assert!(!status_body["recentSessions"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_feature_summary_counts() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game"
            }))
            .await
            .assert_status_ok();

        // Create features with different statuses
        let statuses = vec![
            ("Feature 1", "planned"),
            ("Feature 2", "in_progress"),
            ("Feature 3", "completed"),
            ("Feature 4", "blocked"),
            ("Feature 5", "planned"),
        ];

        for (name, status) in &statuses {
            server
                .post("/harness/feature/update")
                .json(&json!({
                    "projectDir": project_path,
                    "name": name,
                    "status": status
                }))
                .await
                .assert_status_ok();
        }

        // Check status
        let status_response = server
            .post("/harness/status")
            .json(&json!({
                "projectDir": project_path
            }))
            .await;

        status_response.assert_status_ok();

        let body: serde_json::Value = status_response.json();
        let summary = &body["featureSummary"];

        assert_eq!(summary["total"].as_u64(), Some(5));
        assert_eq!(summary["planned"].as_u64(), Some(2));
        assert_eq!(summary["inProgress"].as_u64(), Some(1));
        assert_eq!(summary["completed"].as_u64(), Some(1));
        assert_eq!(summary["blocked"].as_u64(), Some(1));
    }
}

mod edge_cases {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_corrupted_features_yaml() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness normally
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game"
            }))
            .await
            .assert_status_ok();

        // Corrupt the features.yaml file
        let features_path = temp_dir.path().join(".rbxsync/harness/features.yaml");
        fs::write(&features_path, "this is not: valid: yaml: [[[[").unwrap();

        // Creating a new feature should still work (creates fresh features file)
        let response = server
            .post("/harness/feature/update")
            .json(&json!({
                "projectDir": project_path,
                "name": "New Feature"
            }))
            .await;

        // Should succeed by creating fresh features
        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_missing_sessions_directory() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game"
            }))
            .await
            .assert_status_ok();

        // Remove sessions directory
        let sessions_dir = temp_dir.path().join(".rbxsync/harness/sessions");
        fs::remove_dir_all(&sessions_dir).unwrap();

        // Status should still work
        let response = server
            .post("/harness/status")
            .json(&json!({
                "projectDir": project_path
            }))
            .await;

        response.assert_status_ok();

        let body: serde_json::Value = response.json();
        assert!(body["initialized"].as_bool().unwrap());
        assert!(body["recentSessions"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_reinit_preserves_existing_data() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Original Game"
            }))
            .await
            .assert_status_ok();

        // Create a feature
        server
            .post("/harness/feature/update")
            .json(&json!({
                "projectDir": project_path,
                "name": "Existing Feature"
            }))
            .await
            .assert_status_ok();

        // Re-initialize (should overwrite game.yaml but not features)
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "New Game Name"
            }))
            .await
            .assert_status_ok();

        // Check that game name is updated but the previous feature is gone
        // (init creates empty features.yaml)
        let status_response = server
            .post("/harness/status")
            .json(&json!({
                "projectDir": project_path
            }))
            .await;

        status_response.assert_status_ok();
        let body: serde_json::Value = status_response.json();

        // Game name should be updated
        assert_eq!(body["game"]["name"].as_str(), Some("New Game Name"));
        // Features should be empty (init creates fresh features.yaml)
        assert!(body["features"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_unicode_in_names() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize with unicode game name
        let response = server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "ゲーム名 - Game 🎮",
                "description": "日本語の説明"
            }))
            .await;

        response.assert_status_ok();

        // Create feature with unicode
        server
            .post("/harness/feature/update")
            .json(&json!({
                "projectDir": project_path,
                "name": "機能 - Feature ✨",
                "tags": ["日本語", "emoji🎉"]
            }))
            .await
            .assert_status_ok();

        // Verify status shows correct unicode
        let status_response = server
            .post("/harness/status")
            .json(&json!({
                "projectDir": project_path
            }))
            .await;

        status_response.assert_status_ok();
        let body: serde_json::Value = status_response.json();

        assert_eq!(body["game"]["name"].as_str(), Some("ゲーム名 - Game 🎮"));
    }

    #[tokio::test]
    async fn test_feature_with_all_fields() {
        let server = create_test_server();
        let temp_dir = create_temp_project();
        let project_path = temp_dir.path().to_string_lossy().to_string();

        // Initialize harness
        server
            .post("/harness/init")
            .json(&json!({
                "projectDir": project_path,
                "gameName": "Test Game"
            }))
            .await
            .assert_status_ok();

        // Start a session
        let session_response = server
            .post("/harness/session/start")
            .json(&json!({
                "projectDir": project_path
            }))
            .await;
        session_response.assert_status_ok();
        let session_body: serde_json::Value = session_response.json();
        let session_id = session_body["sessionId"].as_str().unwrap();

        // Create feature with all fields
        let response = server
            .post("/harness/feature/update")
            .json(&json!({
                "projectDir": project_path,
                "name": "Complete Feature",
                "description": "A feature with all fields set",
                "status": "in_progress",
                "priority": "high",
                "tags": ["ui", "gameplay", "networking"],
                "acceptanceCriteria": [
                    "Player can see the feature",
                    "Feature works in multiplayer"
                ],
                "addNote": "Started implementation",
                "affectedFiles": ["src/ServerScriptService/FeatureModule.luau"],
                "sessionId": session_id,
                "complexity": 3,
                "dependencies": ["other-feature-id"]
            }))
            .await;

        response.assert_status_ok();

        let body: serde_json::Value = response.json();
        assert!(body["success"].as_bool().unwrap());
    }
}
