//! Integration tests for the Serena App
//!
//! Tests the application lifecycle, project activation, and tool registration flow.
//!
//! NOTE: These tests use a custom config file to avoid picking up the user's
//! real configuration which may have incompatible formats.

use std::fs;
use tempfile::tempdir;

/// Create a minimal valid config file for testing
fn create_test_config(dir: &std::path::Path) -> std::path::PathBuf {
    let config_path = dir.join("serena_config.yml");
    fs::write(&config_path, r#"
# Minimal test configuration
name: "test-config"
"#).expect("Failed to write test config");
    config_path
}

/// Test App creation with a controlled config
#[tokio::test]
async fn test_app_creation_with_config() {
    let temp = tempdir().unwrap();
    let config_path = create_test_config(temp.path());

    let result = serena::App::new(Some(config_path), Some(temp.path().to_path_buf())).await;

    // App creation should succeed with a test config
    assert!(result.is_ok(), "App creation failed: {:?}", result.err());
}

/// Test project activation flow (without real LSP servers)
#[tokio::test]
async fn test_activate_deactivate_project() {
    let temp = tempdir().unwrap();
    let config_path = create_test_config(temp.path());

    // Create app
    let app = serena::App::new(Some(config_path), Some(temp.path().to_path_buf()))
        .await
        .expect("Failed to create app");

    // Activation may fail if no LSP servers are available, that's OK for this test
    // We're testing the flow, not the actual LSP functionality
    let _ = app.activate_project(temp.path().to_path_buf()).await;

    // Deactivation should always succeed
    let result = app.deactivate_project().await;
    assert!(result.is_ok(), "Deactivation failed: {:?}", result.err());
}

/// Test shutdown sequence
#[tokio::test]
async fn test_app_shutdown() {
    let temp = tempdir().unwrap();
    let config_path = create_test_config(temp.path());

    let app = serena::App::new(Some(config_path), Some(temp.path().to_path_buf()))
        .await
        .expect("Failed to create app");

    // Shutdown should complete without error
    let result = app.shutdown().await;
    assert!(result.is_ok(), "Shutdown failed: {:?}", result.err());
}

/// Test configuration access
#[tokio::test]
async fn test_config_access() {
    let temp = tempdir().unwrap();
    let config_path = create_test_config(temp.path());

    let app = serena::App::new(Some(config_path), Some(temp.path().to_path_buf()))
        .await
        .expect("Failed to create app");

    // Should be able to get config without panicking
    let _config = app.get_config().await;

    // If we get here without panic, config access works
}

/// Test project config access before/after activation
#[tokio::test]
async fn test_project_config_access() {
    let temp = tempdir().unwrap();
    let config_path = create_test_config(temp.path());

    let app = serena::App::new(Some(config_path), None)
        .await
        .expect("Failed to create app");

    // Before activation, project config should be None
    let proj_config = app.get_project_config().await;
    assert!(proj_config.is_none());

    // After activation (even if LSP fails), project config should exist
    let _ = app.activate_project(temp.path().to_path_buf()).await;
    let proj_config = app.get_project_config().await;
    assert!(proj_config.is_some());
}

/// Test mode setting (currently a no-op but should not error)
#[tokio::test]
async fn test_set_mode() {
    let temp = tempdir().unwrap();
    let config_path = create_test_config(temp.path());

    let mut app = serena::App::new(Some(config_path), Some(temp.path().to_path_buf()))
        .await
        .expect("Failed to create app");

    // Should not error
    let result = app.set_mode("planning");
    assert!(result.is_ok());

    let result = app.set_mode("editing");
    assert!(result.is_ok());
}

/// Test context setting (currently a no-op but should not error)
#[tokio::test]
async fn test_set_context() {
    let temp = tempdir().unwrap();
    let config_path = create_test_config(temp.path());

    let mut app = serena::App::new(Some(config_path), Some(temp.path().to_path_buf()))
        .await
        .expect("Failed to create app");

    // Should not error
    let result = app.set_context("ide-assistant");
    assert!(result.is_ok());

    let result = app.set_context("agent");
    assert!(result.is_ok());
}

/// Integration test requiring language servers
#[tokio::test]
#[ignore = "Requires language servers installed"]
async fn test_full_lsp_activation() {
    let temp = tempdir().unwrap();
    let config_path = create_test_config(temp.path());

    // Create a minimal Rust project for testing
    fs::create_dir(temp.path().join("src")).unwrap();
    fs::write(temp.path().join("Cargo.toml"), r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#).unwrap();
    fs::write(temp.path().join("src/main.rs"), r#"
fn main() {
    println!("Hello, world!");
}
"#).unwrap();

    let app = serena::App::new(Some(config_path), Some(temp.path().to_path_buf()))
        .await
        .expect("Failed to create app");

    // This should start rust-analyzer and wire symbol tools
    let result = app.activate_project(temp.path().to_path_buf()).await;
    assert!(result.is_ok(), "Activation failed: {:?}", result.err());

    // Cleanup
    let result = app.shutdown().await;
    assert!(result.is_ok());
}
