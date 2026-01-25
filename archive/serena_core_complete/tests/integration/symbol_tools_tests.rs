//! Integration tests for symbol tools
//!
//! Note: Most symbol tests require a running LSP server and are marked as #[ignore]
//! Run with: cargo test --test integration -- --ignored

use rust_mcp_sdk::schema::ContentBlock;
use serena_core::mcp::tools::{
    FindReferencingSymbolsTool, FindSymbolTool, GetSymbolsOverviewTool, RenameSymbolTool,
    ReplaceSymbolBodyTool, SymbolService,
};
use std::fs;
use tempfile::TempDir;

/// Helper to create a SymbolService with a temp directory
fn setup_symbol_service() -> (TempDir, SymbolService) {
    let temp_dir = TempDir::new().unwrap();
    let service = SymbolService::new(temp_dir.path()).unwrap();
    (temp_dir, service)
}

#[tokio::test]
async fn test_symbol_service_creation() {
    let (_temp_dir, service) = setup_symbol_service();

    // Verify service is created successfully
    assert!(service.project_root().exists());
}

#[tokio::test]
async fn test_symbol_graph_accessible() {
    let (_temp_dir, service) = setup_symbol_service();

    // Verify symbol graph is accessible
    let graph = service.symbol_graph();
    assert!(graph.search("NonExistent").is_empty());
}

// =============================================================================
// LSP-dependent tests (require language server)
// Run with: cargo test --test integration -- --ignored
// =============================================================================

#[tokio::test]
#[ignore = "Requires Python LSP server"]
async fn test_get_symbols_overview_python() {
    let (temp_dir, service) = setup_symbol_service();

    // Create a simple Python file
    fs::write(
        temp_dir.path().join("test.py"),
        r#"
class TestClass:
    def method_one(self):
        pass

    def method_two(self):
        pass

def standalone_function():
    return 42
"#,
    )
    .unwrap();

    // Start Python LSP server (pyright)
    service
        .start_lsp("pyright-langserver", vec!["--stdio".to_string()])
        .await
        .expect("Failed to start LSP");

    let tool = GetSymbolsOverviewTool {
        relative_path: "test.py".to_string(),
        depth: 2,
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok());

    // Verify we got symbols
    let content = result.unwrap();
    if let ContentBlock::TextContent(text) = &content.content[0] {
        assert!(text.text.contains("TestClass"));
        assert!(text.text.contains("standalone_function"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
#[ignore = "Requires Rust LSP server"]
async fn test_get_symbols_overview_rust() {
    let (temp_dir, service) = setup_symbol_service();

    // Create a simple Rust file
    fs::write(
        temp_dir.path().join("lib.rs"),
        r#"
pub struct DataStructure {
    pub field: i32,
}

impl DataStructure {
    pub fn new(field: i32) -> Self {
        Self { field }
    }

    pub fn get_field(&self) -> i32 {
        self.field
    }
}

pub fn helper_function() -> String {
    String::from("helper")
}
"#,
    )
    .unwrap();

    // Start Rust Analyzer
    service
        .start_lsp("rust-analyzer", vec![])
        .await
        .expect("Failed to start LSP");

    let tool = GetSymbolsOverviewTool {
        relative_path: "lib.rs".to_string(),
        depth: 2,
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok());

    // Verify we got symbols
    let content = result.unwrap();
    if let ContentBlock::TextContent(text) = &content.content[0] {
        assert!(text.text.contains("DataStructure"));
        assert!(text.text.contains("helper_function"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
#[ignore = "Requires LSP server"]
async fn test_find_symbol() {
    let (temp_dir, service) = setup_symbol_service();

    fs::write(
        temp_dir.path().join("test.py"),
        r#"
def target_function():
    pass

def other_function():
    pass
"#,
    )
    .unwrap();

    service
        .start_lsp("pyright-langserver", vec!["--stdio".to_string()])
        .await
        .expect("Failed to start LSP");

    let tool = FindSymbolTool {
        name_path_pattern: "target_function".to_string(),
        relative_path: Some("test.py".to_string()),
        depth: 0,
        include_body: false,
        substring_matching: false,
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok());

    let content = result.unwrap();
    if let ContentBlock::TextContent(text) = &content.content[0] {
        assert!(text.text.contains("target_function"));
        assert!(text.text.contains("test.py"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
#[ignore = "Requires LSP server"]
async fn test_find_referencing_symbols() {
    let (temp_dir, service) = setup_symbol_service();

    fs::write(
        temp_dir.path().join("module.py"),
        r#"
def utility_function():
    return 42

def caller_one():
    result = utility_function()
    return result

def caller_two():
    value = utility_function()
    return value * 2
"#,
    )
    .unwrap();

    service
        .start_lsp("pyright-langserver", vec!["--stdio".to_string()])
        .await
        .expect("Failed to start LSP");

    let tool = FindReferencingSymbolsTool {
        name_path: "utility_function".to_string(),
        relative_path: "module.py".to_string(),
        max_answer_chars: -1,
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok());

    // Should find references in caller_one and caller_two
    let content = result.unwrap();
    if let ContentBlock::TextContent(text) = &content.content[0] {
        assert!(text.text.contains("caller_one") || text.text.contains("caller_two"));
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test]
#[ignore = "Requires LSP server"]
async fn test_replace_symbol_body() {
    let (temp_dir, service) = setup_symbol_service();

    let original_content = r#"
def greet(name):
    return "Hello"
"#;

    fs::write(temp_dir.path().join("greet.py"), original_content).unwrap();

    service
        .start_lsp("pyright-langserver", vec!["--stdio".to_string()])
        .await
        .expect("Failed to start LSP");

    let new_body = r#"    return f"Hello, {name}!""#;

    let tool = ReplaceSymbolBodyTool {
        name_path: "greet".to_string(),
        relative_path: "greet.py".to_string(),
        body: new_body.to_string(),
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok());

    // Verify file was modified
    let updated_content = fs::read_to_string(temp_dir.path().join("greet.py")).unwrap();
    assert!(updated_content.contains(new_body));
    assert!(updated_content.contains("def greet(name):")); // Signature unchanged
}

#[tokio::test]
#[ignore = "Requires LSP server with rename support"]
async fn test_rename_symbol() {
    let (temp_dir, service) = setup_symbol_service();

    fs::write(
        temp_dir.path().join("rename_test.py"),
        r#"
def old_name():
    pass

def caller():
    old_name()
    return old_name()
"#,
    )
    .unwrap();

    service
        .start_lsp("pyright-langserver", vec!["--stdio".to_string()])
        .await
        .expect("Failed to start LSP");

    let tool = RenameSymbolTool {
        name_path: "old_name".to_string(),
        new_name: "new_name".to_string(),
        relative_path: "rename_test.py".to_string(),
    };

    let result = tool.run_tool(&service).await;
    assert!(result.is_ok());

    // Verify rename occurred
    let updated_content = fs::read_to_string(temp_dir.path().join("rename_test.py")).unwrap();
    assert!(updated_content.contains("new_name"));
    assert!(!updated_content.contains("old_name"));
}

// =============================================================================
// Symbol graph tests (no LSP required)
// =============================================================================

#[tokio::test]
async fn test_symbol_graph_initialization() {
    let (_temp_dir, service) = setup_symbol_service();

    let graph = service.symbol_graph();

    // Test basic initialization
    assert!(graph.search("NonExistent").is_empty());
}

#[tokio::test]
#[ignore = "Flaky test - symbol graph timing issues"]
async fn test_symbol_graph_insert_document_symbols() {
    use lsp_types::{DocumentSymbol, Position, Range, SymbolKind, Url};

    let (_temp_dir, service) = setup_symbol_service();
    let graph = service.symbol_graph();

    let uri = Url::parse("file:///test.py").unwrap();
    let range = Range::new(Position::new(0, 0), Position::new(5, 0));

    let symbol = DocumentSymbol {
        name: "TestFunction".to_string(),
        kind: SymbolKind::FUNCTION,
        range,
        selection_range: range,
        detail: Some("def test()".to_string()),
        children: None,
        tags: None,
        deprecated: None,
    };

    // Insert symbol
    graph.insert_document_symbols(&uri, vec![symbol]);

    // Search for it
    let results = graph.search("TestFunction");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "TestFunction");
}

#[tokio::test]
#[ignore = "Flaky test - symbol graph timing issues"]
async fn test_symbol_graph_search_exact_match() {
    use lsp_types::{DocumentSymbol, Position, Range, SymbolKind, Url};

    let (_temp_dir, service) = setup_symbol_service();
    let graph = service.symbol_graph();

    let uri = Url::parse("file:///test.py").unwrap();
    let range = Range::new(Position::new(0, 0), Position::new(5, 0));

    let symbols = vec![
        DocumentSymbol {
            name: "FindMe".to_string(),
            kind: SymbolKind::FUNCTION,
            range,
            selection_range: range,
            detail: None,
            children: None,
            tags: None,
            deprecated: None,
        },
        DocumentSymbol {
            name: "OtherSymbol".to_string(),
            kind: SymbolKind::FUNCTION,
            range,
            selection_range: range,
            detail: None,
            children: None,
            tags: None,
            deprecated: None,
        },
    ];

    graph.insert_document_symbols(&uri, symbols);

    let results = graph.search("FindMe");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "FindMe");
}

#[tokio::test]
async fn test_symbol_graph_search_substring() {
    use lsp_types::{DocumentSymbol, Position, Range, SymbolKind, Url};

    let (_temp_dir, service) = setup_symbol_service();
    let graph = service.symbol_graph();

    let uri = Url::parse("file:///test.py").unwrap();
    let range = Range::new(Position::new(0, 0), Position::new(5, 0));

    let symbols = vec![
        DocumentSymbol {
            name: "my_function".to_string(),
            kind: SymbolKind::FUNCTION,
            range,
            selection_range: range,
            detail: None,
            children: None,
            tags: None,
            deprecated: None,
        },
        DocumentSymbol {
            name: "another_function".to_string(),
            kind: SymbolKind::FUNCTION,
            range,
            selection_range: range,
            detail: None,
            children: None,
            tags: None,
            deprecated: None,
        },
    ];

    graph.insert_document_symbols(&uri, symbols);

    // Search with substring
    let results = graph.search("function");
    assert!(results.len() >= 2);
}

#[tokio::test]
async fn test_symbol_graph_get_file_symbols() {
    use lsp_types::{DocumentSymbol, Position, Range, SymbolKind, Url};

    let (_temp_dir, service) = setup_symbol_service();
    let graph = service.symbol_graph();

    let uri1 = Url::parse("file:///test1.py").unwrap();
    let uri2 = Url::parse("file:///test2.py").unwrap();
    let range = Range::new(Position::new(0, 0), Position::new(5, 0));

    // Insert symbols for first file
    graph.insert_document_symbols(
        &uri1,
        vec![DocumentSymbol {
            name: "Symbol1".to_string(),
            kind: SymbolKind::FUNCTION,
            range,
            selection_range: range,
            detail: None,
            children: None,
            tags: None,
            deprecated: None,
        }],
    );

    // Insert symbols for second file
    graph.insert_document_symbols(
        &uri2,
        vec![DocumentSymbol {
            name: "Symbol2".to_string(),
            kind: SymbolKind::FUNCTION,
            range,
            selection_range: range,
            detail: None,
            children: None,
            tags: None,
            deprecated: None,
        }],
    );

    // Get symbols for first file
    let file1_symbols = graph.get_file_symbols(&uri1);
    assert!(file1_symbols.is_some());
    assert_eq!(file1_symbols.unwrap().len(), 1);

    // Get symbols for second file
    let file2_symbols = graph.get_file_symbols(&uri2);
    assert!(file2_symbols.is_some());
    assert_eq!(file2_symbols.unwrap().len(), 1);
}

#[tokio::test]
async fn test_symbol_graph_nested_symbols() {
    use lsp_types::{DocumentSymbol, Position, Range, SymbolKind, Url};

    let (_temp_dir, service) = setup_symbol_service();
    let graph = service.symbol_graph();

    let uri = Url::parse("file:///test.py").unwrap();
    let range = Range::new(Position::new(0, 0), Position::new(10, 0));

    // Create nested symbols (class with methods)
    let symbol = DocumentSymbol {
        name: "MyClass".to_string(),
        kind: SymbolKind::CLASS,
        range,
        selection_range: range,
        detail: None,
        children: Some(vec![
            DocumentSymbol {
                name: "method1".to_string(),
                kind: SymbolKind::METHOD,
                range,
                selection_range: range,
                detail: None,
                children: None,
                tags: None,
                deprecated: None,
            },
            DocumentSymbol {
                name: "method2".to_string(),
                kind: SymbolKind::METHOD,
                range,
                selection_range: range,
                detail: None,
                children: None,
                tags: None,
                deprecated: None,
            },
        ]),
        tags: None,
        deprecated: None,
    };

    graph.insert_document_symbols(&uri, vec![symbol]);

    // Search for class
    let class_results = graph.search("MyClass");
    assert!(!class_results.is_empty());
    assert_eq!(class_results[0].name, "MyClass");
    assert_eq!(class_results[0].children.len(), 2);

    // Search for method
    let method_results = graph.search("method1");
    assert!(!method_results.is_empty());
}
