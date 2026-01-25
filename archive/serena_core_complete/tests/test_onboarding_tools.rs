//! Tests for onboarding tools

use serena_core::mcp::tools::{CheckOnboardingPerformedTool, MemoryService, OnboardingTool};

#[tokio::test]
async fn test_onboarding_tool() {
    let tool = OnboardingTool {};
    let result = tool.run_tool().await;

    assert!(result.is_ok());
    let result = result.unwrap();

    // Verify the result contains onboarding instructions
    let content = result.content;
    assert!(!content.is_empty());

    // Check if it contains expected sections
    // Content is TextContent, we can serialize to JSON to verify
    let json_str = serde_json::to_string(&content).unwrap();

    assert!(json_str.contains("Project Onboarding"));
    assert!(json_str.contains("Step 1: Explore Structure"));
    assert!(json_str.contains("Step 2: Identify Key Components"));
    assert!(json_str.contains("Step 3: Save Your Findings"));
    assert!(json_str.contains("Step 4: Ready to Work"));
}

#[tokio::test]
async fn test_check_onboarding_performed_no_memories() {
    // Create a temporary directory for testing
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let temp_dir = std::env::temp_dir().join(format!("serena_test_{}", timestamp));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let memory_service = MemoryService::new(&temp_dir).unwrap();
    let tool = CheckOnboardingPerformedTool {};

    let result = tool.run_tool(&memory_service).await;
    assert!(result.is_ok());

    let result = result.unwrap();
    let content = result.content;

    // Verify the result indicates no onboarding
    let json_str = serde_json::to_string(&content).unwrap();

    assert!(json_str.contains("onboarding_performed"));
    assert!(json_str.contains("false"));
    assert!(json_str.contains("No project memories found"));

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_check_onboarding_performed_with_memories() {
    // Create a temporary directory for testing
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let temp_dir = std::env::temp_dir().join(format!("serena_test_{}", timestamp));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let memory_service = MemoryService::new(&temp_dir).unwrap();

    // Create a test memory
    memory_service
        .write("test_memory", "This is a test memory")
        .await
        .unwrap();

    let tool = CheckOnboardingPerformedTool {};
    let result = tool.run_tool(&memory_service).await;

    assert!(result.is_ok());
    let result = result.unwrap();
    let content = result.content;

    // Verify the result indicates onboarding was performed
    let json_str = serde_json::to_string(&content).unwrap();

    assert!(json_str.contains("onboarding_performed"));
    assert!(json_str.contains("true"));
    assert!(json_str.contains("existing memories"));
    assert!(json_str.contains("test_memory"));

    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}
