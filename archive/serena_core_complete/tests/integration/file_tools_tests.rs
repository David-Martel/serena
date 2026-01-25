//! Integration tests for file tools

use rust_mcp_sdk::schema::ContentBlock;
use serena_core::mcp::tools::{
    CreateTextFileTool, FileService, FindFileTool, ListDirTool, ReadFileTool, ReplaceContentTool,
    SearchForPatternTool,
};
use std::fs;
use tempfile::TempDir;

/// Helper to create a FileService with a temp directory
fn setup_file_service() -> (TempDir, FileService) {
    let temp_dir = TempDir::new().unwrap();
    let service = FileService::new(temp_dir.path()).unwrap();
    (temp_dir, service)
}

#[tokio::test]
async fn test_read_file_basic() {
    let (temp_dir, service) = setup_file_service();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, World!\nLine 2\nLine 3").unwrap();

    let tool = ReadFileTool {
        relative_path: "test.txt".to_string(),
        start_line: None,
        limit: None,
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok());

    let content = result.unwrap();
    assert_eq!(content.content.len(), 1);
    if let ContentBlock::TextContent(text) = &content.content[0] {
        assert_eq!(text.text, "Hello, World!\nLine 2\nLine 3");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_read_file_with_line_limits() {
    let (temp_dir, service) = setup_file_service();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Line 1\nLine 2\nLine 3\nLine 4\nLine 5").unwrap();

    // Read lines 1-2 (0-indexed start, limit=2)
    let tool = ReadFileTool {
        relative_path: "test.txt".to_string(),
        start_line: Some(1),
        limit: Some(2),
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        assert_eq!(text.text, "Line 2\nLine 3");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_read_file_not_found() {
    let (_temp_dir, service) = setup_file_service();

    let tool = ReadFileTool {
        relative_path: "nonexistent.txt".to_string(),
        start_line: None,
        limit: None,
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_and_read_file() {
    let (_temp_dir, service) = setup_file_service();

    // Create file
    let create_tool = CreateTextFileTool {
        relative_path: "new_file.txt".to_string(),
        content: "Test content\nSecond line".to_string(),
    };

    let create_result = create_tool.run_tool(&service).await;
    assert!(create_result.is_ok());

    // Read it back
    let read_tool = ReadFileTool {
        relative_path: "new_file.txt".to_string(),
        start_line: None,
        limit: None,
        max_answer_chars: -1,
    };

    let read_result = read_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &read_result.content[0] {
        assert_eq!(text.text, "Test content\nSecond line");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_create_file_in_subdirectory() {
    let (_temp_dir, service) = setup_file_service();

    let create_tool = CreateTextFileTool {
        relative_path: "subdir/nested/file.txt".to_string(),
        content: "Nested file content".to_string(),
    };

    let result = create_tool.run_tool(&service).await;
    assert!(result.is_ok());

    // Verify file exists
    let read_tool = ReadFileTool {
        relative_path: "subdir/nested/file.txt".to_string(),
        start_line: None,
        limit: None,
        max_answer_chars: -1,
    };

    let read_result = read_tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &read_result.content[0] {
        assert_eq!(text.text, "Nested file content");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_list_dir_non_recursive() {
    let (temp_dir, service) = setup_file_service();

    // Create some files and directories
    fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();
    fs::create_dir(temp_dir.path().join("subdir")).unwrap();
    fs::write(temp_dir.path().join("subdir").join("file3.txt"), "content3").unwrap();

    let tool = ListDirTool {
        relative_path: ".".to_string(),
        recursive: false,
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let files = data["files"].as_array().unwrap();
        let dirs = data["dirs"].as_array().unwrap();

        // Should have 2 files in root
        assert_eq!(files.len(), 2);
        // Should have 1 directory
        assert_eq!(dirs.len(), 1);

        // Files in subdir should not be listed
        assert!(files.iter().all(|f| !f.as_str().unwrap().contains("file3")));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_list_dir_recursive() {
    let (temp_dir, service) = setup_file_service();

    // Create nested structure
    fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
    fs::create_dir(temp_dir.path().join("subdir")).unwrap();
    fs::write(temp_dir.path().join("subdir").join("file2.txt"), "content2").unwrap();
    fs::create_dir(temp_dir.path().join("subdir").join("nested")).unwrap();
    fs::write(
        temp_dir
            .path()
            .join("subdir")
            .join("nested")
            .join("file3.txt"),
        "content3",
    )
    .unwrap();

    let tool = ListDirTool {
        relative_path: ".".to_string(),
        recursive: true,
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let files = data["files"].as_array().unwrap();

        // Should have all 3 files
        assert!(files.len() >= 3);

        // Check for specific files
        let file_names: Vec<&str> = files.iter().map(|f| f.as_str().unwrap()).collect();
        assert!(file_names.iter().any(|f| f.ends_with("file1.txt")));
        assert!(file_names.iter().any(|f| f.ends_with("file2.txt")));
        assert!(file_names.iter().any(|f| f.ends_with("file3.txt")));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_find_file_glob_pattern() {
    let (temp_dir, service) = setup_file_service();

    // Create test files
    fs::write(temp_dir.path().join("test.rs"), "rust code").unwrap();
    fs::write(temp_dir.path().join("test.txt"), "text").unwrap();
    fs::create_dir(temp_dir.path().join("src")).unwrap();
    fs::write(temp_dir.path().join("src").join("lib.rs"), "lib").unwrap();
    fs::write(temp_dir.path().join("src").join("main.rs"), "main").unwrap();

    let tool = FindFileTool {
        file_mask: "**/*.rs".to_string(),
        relative_path: ".".to_string(),
    };

    let result = tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let files = data["files"].as_array().unwrap();

        // Should find all .rs files
        assert!(files.len() >= 3);

        let file_names: Vec<&str> = files.iter().map(|f| f.as_str().unwrap()).collect();
        assert!(file_names.iter().any(|f| f.ends_with("test.rs")));
        assert!(file_names.iter().any(|f| f.ends_with("lib.rs")));
        assert!(file_names.iter().any(|f| f.ends_with("main.rs")));
        // Should not find .txt files
        assert!(!file_names.iter().any(|f| f.ends_with("test.txt")));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_replace_content_literal() {
    let (temp_dir, service) = setup_file_service();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, World!").unwrap();

    let tool = ReplaceContentTool {
        relative_path: "test.txt".to_string(),
        needle: "World".to_string(),
        repl: "Rust".to_string(),
        mode: "literal".to_string(),
        allow_multiple_occurrences: false,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok());

    // Verify the replacement
    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "Hello, Rust!");
}

#[tokio::test]
async fn test_replace_content_regex() {
    let (temp_dir, service) = setup_file_service();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "foo 123 bar 456 baz").unwrap();

    let tool = ReplaceContentTool {
        relative_path: "test.txt".to_string(),
        needle: r"\d+".to_string(),
        repl: "NUM".to_string(),
        mode: "regex".to_string(),
        allow_multiple_occurrences: true,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok());

    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "foo NUM bar NUM baz");
}

#[tokio::test]
async fn test_replace_content_multiple_occurrences_error() {
    let (temp_dir, service) = setup_file_service();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "foo bar foo baz").unwrap();

    let tool = ReplaceContentTool {
        relative_path: "test.txt".to_string(),
        needle: "foo".to_string(),
        repl: "replaced".to_string(),
        mode: "literal".to_string(),
        allow_multiple_occurrences: false,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_err());
    // Should error because multiple matches exist
}

#[tokio::test]
async fn test_replace_content_pattern_not_found() {
    let (temp_dir, service) = setup_file_service();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, World!").unwrap();

    let tool = ReplaceContentTool {
        relative_path: "test.txt".to_string(),
        needle: "NotFound".to_string(),
        repl: "Replacement".to_string(),
        mode: "literal".to_string(),
        allow_multiple_occurrences: false,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_search_for_pattern() {
    let (temp_dir, service) = setup_file_service();

    // Create test files with searchable content
    fs::write(
        temp_dir.path().join("file1.rs"),
        "fn main() {\n    println!(\"hello\");\n}",
    )
    .unwrap();
    fs::write(temp_dir.path().join("file2.rs"), "fn other() {}").unwrap();
    fs::write(
        temp_dir.path().join("file3.txt"),
        "Some text with println macro",
    )
    .unwrap();

    let tool = SearchForPatternTool {
        substring_pattern: "println".to_string(),
        relative_path: None,
        context_lines_before: 0,
        context_lines_after: 0,
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let obj = data.as_object().unwrap();

        // Should find matches in file1.rs and file3.txt
        assert!(obj.len() >= 2);
        assert!(obj.keys().any(|k| k.ends_with("file1.rs")));
        assert!(obj.keys().any(|k| k.ends_with("file3.txt")));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_search_for_pattern_with_context() {
    let (temp_dir, service) = setup_file_service();

    fs::write(
        temp_dir.path().join("test.txt"),
        "Line 1\nLine 2\nMATCH\nLine 4\nLine 5",
    )
    .unwrap();

    let tool = SearchForPatternTool {
        substring_pattern: "MATCH".to_string(),
        relative_path: None,
        context_lines_before: 1,
        context_lines_after: 1,
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let obj = data.as_object().unwrap();

        // Get the matches for test.txt
        let matches = obj
            .iter()
            .find(|(k, _)| k.ends_with("test.txt"))
            .unwrap()
            .1
            .as_array()
            .unwrap();

        assert_eq!(matches.len(), 1);
        let context = matches[0]["context"].as_array().unwrap();

        // Should have 3 lines: before, match, after
        assert_eq!(context.len(), 3);

        let context_text: Vec<&str> = context.iter().map(|v| v.as_str().unwrap()).collect();

        assert!(context_text.iter().any(|line| line.contains("Line 2")));
        assert!(context_text.iter().any(|line| line.contains("MATCH")));
        assert!(context_text.iter().any(|line| line.contains("Line 4")));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
async fn test_search_restricted_path() {
    let (temp_dir, service) = setup_file_service();

    // Create structure
    fs::create_dir(temp_dir.path().join("src")).unwrap();
    fs::create_dir(temp_dir.path().join("tests")).unwrap();

    fs::write(temp_dir.path().join("src").join("lib.rs"), "target").unwrap();
    fs::write(temp_dir.path().join("tests").join("test.rs"), "target").unwrap();

    let tool = SearchForPatternTool {
        substring_pattern: "target".to_string(),
        relative_path: Some("src".to_string()),
        context_lines_before: 0,
        context_lines_after: 0,
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await.unwrap();
    if let ContentBlock::TextContent(text) = &result.content[0] {
        let data: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        let obj = data.as_object().unwrap();

        // Should only find in src/
        assert!(obj.keys().all(|k| k.contains("src")));
        assert!(!obj.keys().any(|k| k.contains("tests")));
    } else {
        panic!("Expected text content");
    }
}
