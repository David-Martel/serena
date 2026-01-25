use serde::{Deserialize, Serialize};

/// Symbol kind enumeration matching LSP symbol kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
    /// Unknown or unsupported symbol kind
    Unknown,
}

impl From<lsp_types::SymbolKind> for SymbolKind {
    fn from(kind: lsp_types::SymbolKind) -> Self {
        match kind {
            lsp_types::SymbolKind::FILE => SymbolKind::File,
            lsp_types::SymbolKind::MODULE => SymbolKind::Module,
            lsp_types::SymbolKind::NAMESPACE => SymbolKind::Namespace,
            lsp_types::SymbolKind::PACKAGE => SymbolKind::Package,
            lsp_types::SymbolKind::CLASS => SymbolKind::Class,
            lsp_types::SymbolKind::METHOD => SymbolKind::Method,
            lsp_types::SymbolKind::PROPERTY => SymbolKind::Property,
            lsp_types::SymbolKind::FIELD => SymbolKind::Field,
            lsp_types::SymbolKind::CONSTRUCTOR => SymbolKind::Constructor,
            lsp_types::SymbolKind::ENUM => SymbolKind::Enum,
            lsp_types::SymbolKind::INTERFACE => SymbolKind::Interface,
            lsp_types::SymbolKind::FUNCTION => SymbolKind::Function,
            lsp_types::SymbolKind::VARIABLE => SymbolKind::Variable,
            lsp_types::SymbolKind::CONSTANT => SymbolKind::Constant,
            lsp_types::SymbolKind::STRING => SymbolKind::String,
            lsp_types::SymbolKind::NUMBER => SymbolKind::Number,
            lsp_types::SymbolKind::BOOLEAN => SymbolKind::Boolean,
            lsp_types::SymbolKind::ARRAY => SymbolKind::Array,
            lsp_types::SymbolKind::OBJECT => SymbolKind::Object,
            lsp_types::SymbolKind::KEY => SymbolKind::Key,
            lsp_types::SymbolKind::NULL => SymbolKind::Null,
            lsp_types::SymbolKind::ENUM_MEMBER => SymbolKind::EnumMember,
            lsp_types::SymbolKind::STRUCT => SymbolKind::Struct,
            lsp_types::SymbolKind::EVENT => SymbolKind::Event,
            lsp_types::SymbolKind::OPERATOR => SymbolKind::Operator,
            lsp_types::SymbolKind::TYPE_PARAMETER => SymbolKind::TypeParameter,
            _ => SymbolKind::Object, // fallback for unknown kinds
        }
    }
}

impl From<SymbolKind> for lsp_types::SymbolKind {
    fn from(kind: SymbolKind) -> Self {
        match kind {
            SymbolKind::File => lsp_types::SymbolKind::FILE,
            SymbolKind::Module => lsp_types::SymbolKind::MODULE,
            SymbolKind::Namespace => lsp_types::SymbolKind::NAMESPACE,
            SymbolKind::Package => lsp_types::SymbolKind::PACKAGE,
            SymbolKind::Class => lsp_types::SymbolKind::CLASS,
            SymbolKind::Method => lsp_types::SymbolKind::METHOD,
            SymbolKind::Property => lsp_types::SymbolKind::PROPERTY,
            SymbolKind::Field => lsp_types::SymbolKind::FIELD,
            SymbolKind::Constructor => lsp_types::SymbolKind::CONSTRUCTOR,
            SymbolKind::Enum => lsp_types::SymbolKind::ENUM,
            SymbolKind::Interface => lsp_types::SymbolKind::INTERFACE,
            SymbolKind::Function => lsp_types::SymbolKind::FUNCTION,
            SymbolKind::Variable => lsp_types::SymbolKind::VARIABLE,
            SymbolKind::Constant => lsp_types::SymbolKind::CONSTANT,
            SymbolKind::String => lsp_types::SymbolKind::STRING,
            SymbolKind::Number => lsp_types::SymbolKind::NUMBER,
            SymbolKind::Boolean => lsp_types::SymbolKind::BOOLEAN,
            SymbolKind::Array => lsp_types::SymbolKind::ARRAY,
            SymbolKind::Object => lsp_types::SymbolKind::OBJECT,
            SymbolKind::Key => lsp_types::SymbolKind::KEY,
            SymbolKind::Null => lsp_types::SymbolKind::NULL,
            SymbolKind::EnumMember => lsp_types::SymbolKind::ENUM_MEMBER,
            SymbolKind::Struct => lsp_types::SymbolKind::STRUCT,
            SymbolKind::Event => lsp_types::SymbolKind::EVENT,
            SymbolKind::Operator => lsp_types::SymbolKind::OPERATOR,
            SymbolKind::TypeParameter => lsp_types::SymbolKind::TYPE_PARAMETER,
            SymbolKind::Unknown => lsp_types::SymbolKind::OBJECT, // fallback for unknown
        }
    }
}

/// Position in a document (zero-based)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl From<lsp_types::Position> for Position {
    fn from(pos: lsp_types::Position) -> Self {
        Self {
            line: pos.line,
            character: pos.character,
        }
    }
}

impl From<Position> for lsp_types::Position {
    fn from(pos: Position) -> Self {
        Self {
            line: pos.line,
            character: pos.character,
        }
    }
}

/// Range in a document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl From<lsp_types::Range> for Range {
    fn from(range: lsp_types::Range) -> Self {
        Self {
            start: range.start.into(),
            end: range.end.into(),
        }
    }
}

impl From<Range> for lsp_types::Range {
    fn from(range: Range) -> Self {
        Self {
            start: range.start.into(),
            end: range.end.into(),
        }
    }
}

/// Location in a document
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

impl From<lsp_types::Location> for Location {
    fn from(loc: lsp_types::Location) -> Self {
        Self {
            uri: loc.uri.to_string(),
            range: loc.range.into(),
        }
    }
}

/// Symbol information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<SymbolInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
}

impl SymbolInfo {
    pub fn new(name: String, kind: SymbolKind, location: Location) -> Self {
        Self {
            name,
            kind,
            location,
            detail: None,
            children: Vec::new(),
            container_name: None,
        }
    }

    pub fn with_detail(mut self, detail: String) -> Self {
        self.detail = Some(detail);
        self
    }

    pub fn with_children(mut self, children: Vec<SymbolInfo>) -> Self {
        self.children = children;
        self
    }

    pub fn with_container_name(mut self, container_name: String) -> Self {
        self.container_name = Some(container_name);
        self
    }
}

/// Tool execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    Success,
    Error,
    Warning,
}

/// Tool execution result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub status: ToolStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ToolResult {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            status: ToolStatus::Success,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    pub fn success_with_message(data: serde_json::Value, message: String) -> Self {
        Self {
            status: ToolStatus::Success,
            data: Some(data),
            error: None,
            message: Some(message),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            status: ToolStatus::Error,
            data: None,
            error: Some(error),
            message: None,
        }
    }

    pub fn warning(message: String) -> Self {
        Self {
            status: ToolStatus::Warning,
            data: None,
            error: None,
            message: Some(message),
        }
    }

    pub fn warning_with_data(data: serde_json::Value, message: String) -> Self {
        Self {
            status: ToolStatus::Warning,
            data: Some(data),
            error: None,
            message: Some(message),
        }
    }
}
