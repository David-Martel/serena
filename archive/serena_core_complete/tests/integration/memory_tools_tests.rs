//! Integration tests for memory tools

use rust_mcp_sdk::schema::ContentBlock;
use serena_core::mcp::tools::{
    DeleteMemoryTool, EditMemoryTool, ListMemoriesTool, MemoryService, ReadMemoryTool,
    WriteMemoryTool,
};
use tempfile::TempDir;

/// Helper to create a MemoryService with a temp directory
fn setup_memory_service() -> (TempDir, MemoryService) {
    let temp_dir = TempDir::new().unwrap();
    let service = MemoryService::new(temp_dir.path()).unwrap();
    (temp_dir, service)
}

#[tokio::test]
async fn test_write_and_read_memory() {
    let (_temp_dir, service) = setup_memory_service();

    // Write memory
    let write_tool = WriteMemoryTool {
        memory_file_name: "test_memory".to_string(),
        content: "# Test Memory\n\nThis is test content.".to_string(),
    };

    let write_result = write_tool.run_tool(&service).await;
    assert!(write_result.is_ok());

    // Read it back
    let read_tool = ReadMemoryTool {
        memory_file_name: "test_memory".to_string(),
        max_answer_chars: -1,
    };

    let read_result = read_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &read_result.content[0] {
        assert_eq!(text.text, "# Test Memory\n\nThis is test content.");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_memory_crud_workflow() {
    let (_temp_dir, service) = setup_memory_service();

    // 1. Write memory
    let write_tool = WriteMemoryTool {
        memory_file_name: "workflow_test".to_string(),
        content: "Initial content".to_string(),
    };
    assert!(write_tool.run_tool(&service).await.is_ok());

    // 2. List memories - verify exists
    let list_tool = ListMemoriesTool {
        max_answer_chars: -1,
    };
    let list_result = list_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &list_result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let memories = data["memories"].as_array().unwrap();
        assert!(memories
            .iter()
            .any(|m| m.as_str().unwrap() == "workflow_test"));
    } else {
        panic!("Expected text content");
    }

    // 3. Read memory - verify content
    let read_tool = ReadMemoryTool {
        memory_file_name: "workflow_test".to_string(),
        max_answer_chars: -1,
    };
    let read_result = read_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &read_result.content[0] {
        assert_eq!(text.text, "Initial content");
    } else {
        panic!("Expected text content");
    }

    // 4. Edit memory
    let edit_tool = EditMemoryTool {
        memory_file_name: "workflow_test".to_string(),
        needle: "Initial".to_string(),
        repl: "Modified".to_string(),
        mode: "literal".to_string(),
    };
    assert!(edit_tool.run_tool(&service).await.is_ok());

    // 5. Read again - verify change
    let read_tool2 = ReadMemoryTool {
        memory_file_name: "workflow_test".to_string(),
        max_answer_chars: -1,
    };
    let read_result2 = read_tool2.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &read_result2.content[0] {
        assert_eq!(text.text, "Modified content");
    } else {
        panic!("Expected text content");
    }

    // 6. Delete memory
    let delete_tool = DeleteMemoryTool {
        memory_file_name: "workflow_test".to_string(),
    };
    assert!(delete_tool.run_tool(&service).await.is_ok());

    // 7. List - verify gone
    let list_tool2 = ListMemoriesTool {
        max_answer_chars: -1,
    };
    let list_result2 = list_tool2.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &list_result2.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let memories = data["memories"].as_array().unwrap();
        assert!(!memories
            .iter()
            .any(|m| m.as_str().unwrap() == "workflow_test"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_list_memories_empty() {
    let (_temp_dir, service) = setup_memory_service();

    let list_tool = ListMemoriesTool {
        max_answer_chars: -1,
    };
    let result = list_tool.run_tool(&service).await.unwrap();

    if let ContentBlock::TextContent(text) = &result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let memories = data["memories"].as_array().unwrap();
        assert_eq!(memories.len(), 0);
        assert_eq!(data["count"].as_i64().unwrap(), 0);
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_list_memories_multiple() {
    let (_temp_dir, service) = setup_memory_service();

    // Create multiple memories
    for i in 1..=5 {
        let write_tool = WriteMemoryTool {
            memory_file_name: format!("memory_{}", i),
            content: format!("Content {}", i),
        };
        assert!(write_tool.run_tool(&service).await.is_ok());
    }

    let list_tool = ListMemoriesTool {
        max_answer_chars: -1,
    };
    let result = list_tool.run_tool(&service).await.unwrap();

    if let ContentBlock::TextContent(text) = &result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let memories = data["memories"].as_array().unwrap();
        assert_eq!(memories.len(), 5);
        assert_eq!(data["count"].as_i64().unwrap(), 5);

        // Verify all memories are present
        for i in 1..=5 {
            let name = format!("memory_{}", i);
            assert!(memories.iter().any(|m| m.as_str().unwrap() == name));
        }
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_read_nonexistent_memory() {
    let (_temp_dir, service) = setup_memory_service();

    let read_tool = ReadMemoryTool {
        memory_file_name: "nonexistent".to_string(),
        max_answer_chars: -1,
    };

    let result = read_tool.run_tool(&service).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_nonexistent_memory() {
    let (_temp_dir, service) = setup_memory_service();

    let delete_tool = DeleteMemoryTool {
        memory_file_name: "nonexistent".to_string(),
    };

    let result = delete_tool.run_tool(&service).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_edit_memory_literal() {
    let (_temp_dir, service) = setup_memory_service();

    // Create memory
    let write_tool = WriteMemoryTool {
        memory_file_name: "edit_test".to_string(),
        content: "Hello, World!\nSecond line.".to_string(),
    };
    assert!(write_tool.run_tool(&service).await.is_ok());

    // Edit with literal mode
    let edit_tool = EditMemoryTool {
        memory_file_name: "edit_test".to_string(),
        needle: "World".to_string(),
        repl: "Rust".to_string(),
        mode: "literal".to_string(),
    };
    assert!(edit_tool.run_tool(&service).await.is_ok());

    // Verify
    let read_tool = ReadMemoryTool {
        memory_file_name: "edit_test".to_string(),
        max_answer_chars: -1,
    };
    let result = read_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        assert_eq!(text.text, "Hello, Rust!\nSecond line.");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_edit_memory_regex() {
    let (_temp_dir, service) = setup_memory_service();

    // Create memory
    let write_tool = WriteMemoryTool {
        memory_file_name: "regex_test".to_string(),
        content: "Value: 123\nAnother: 456".to_string(),
    };
    assert!(write_tool.run_tool(&service).await.is_ok());

    // Edit with regex mode
    let edit_tool = EditMemoryTool {
        memory_file_name: "regex_test".to_string(),
        needle: r"\d+".to_string(),
        repl: "NUM".to_string(),
        mode: "regex".to_string(),
    };
    assert!(edit_tool.run_tool(&service).await.is_ok());

    // Verify
    let read_tool = ReadMemoryTool {
        memory_file_name: "regex_test".to_string(),
        max_answer_chars: -1,
    };
    let result = read_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        assert_eq!(text.text, "Value: NUM\nAnother: NUM");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_edit_memory_pattern_not_found() {
    let (_temp_dir, service) = setup_memory_service();

    // Create memory
    let write_tool = WriteMemoryTool {
        memory_file_name: "not_found_test".to_string(),
        content: "Some content".to_string(),
    };
    assert!(write_tool.run_tool(&service).await.is_ok());

    // Try to edit with pattern that doesn't exist
    let edit_tool = EditMemoryTool {
        memory_file_name: "not_found_test".to_string(),
        needle: "NonExistent".to_string(),
        repl: "Replacement".to_string(),
        mode: "literal".to_string(),
    };

    let result = edit_tool.run_tool(&service).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_overwrite_existing_memory() {
    let (_temp_dir, service) = setup_memory_service();

    // Write initial memory
    let write_tool1 = WriteMemoryTool {
        memory_file_name: "overwrite_test".to_string(),
        content: "Original content".to_string(),
    };
    assert!(write_tool1.run_tool(&service).await.is_ok());

    // Overwrite with new content
    let write_tool2 = WriteMemoryTool {
        memory_file_name: "overwrite_test".to_string(),
        content: "New content".to_string(),
    };
    assert!(write_tool2.run_tool(&service).await.is_ok());

    // Verify new content
    let read_tool = ReadMemoryTool {
        memory_file_name: "overwrite_test".to_string(),
        max_answer_chars: -1,
    };
    let result = read_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        assert_eq!(text.text, "New content");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_memory_with_special_characters() {
    let (_temp_dir, service) = setup_memory_service();

    // Content with special characters
    let content = "# Memory\n\nCode:\n```rust\nfn main() {\n    println!(\"Hello!\");\n}\n```";

    let write_tool = WriteMemoryTool {
        memory_file_name: "special_chars".to_string(),
        content: content.to_string(),
    };
    assert!(write_tool.run_tool(&service).await.is_ok());

    let read_tool = ReadMemoryTool {
        memory_file_name: "special_chars".to_string(),
        max_answer_chars: -1,
    };
    let result = read_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        assert_eq!(text.text, content);
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_memory_empty_content() {
    let (_temp_dir, service) = setup_memory_service();

    let write_tool = WriteMemoryTool {
        memory_file_name: "empty".to_string(),
        content: "".to_string(),
    };
    assert!(write_tool.run_tool(&service).await.is_ok());

    let read_tool = ReadMemoryTool {
        memory_file_name: "empty".to_string(),
        max_answer_chars: -1,
    };
    let result = read_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        assert_eq!(text.text, "");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_memory_large_content() {
    let (_temp_dir, service) = setup_memory_service();

    // Create large content (10KB)
    let large_content = "Lorem ipsum dolor sit amet. ".repeat(400);

    let write_tool = WriteMemoryTool {
        memory_file_name: "large".to_string(),
        content: large_content.clone(),
    };
    assert!(write_tool.run_tool(&service).await.is_ok());

    let read_tool = ReadMemoryTool {
        memory_file_name: "large".to_string(),
        max_answer_chars: -1,
    };
    let result = read_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        assert_eq!(text.text, large_content);
    } else {
        panic!("Expected text content");
    }
}
