# Serena Rust Architecture Diagrams

## System Architecture

```mermaid
graph TB
    subgraph "External Clients"
        AI[AI Agent<br/>Claude/GPT]
        CLI[CLI User]
        WEB[Web Browser]
    end

    subgraph "Serena Core"
        MCP[serena-mcp<br/>MCP Server]
        AGENT[serena<br/>Main Agent]
        CLI_MOD[serena-cli<br/>CLI Interface]
        WEB_MOD[serena-web<br/>Web Dashboard]
    end

    subgraph "Core Services"
        TOOLS[serena-tools<br/>Tool Registry]
        LSP[serena-lsp<br/>LSP Manager]
        MEMORY[serena-memory<br/>Knowledge Store]
        CONFIG[serena-config<br/>Configuration]
    end

    subgraph "Foundation"
        CORE[serena-core<br/>Types & Traits]
    end

    subgraph "External Systems"
        LS_PY[Python LSP]
        LS_RS[Rust LSP]
        LS_TS[TypeScript LSP]
        LS_GO[Go LSP]
        DB[(SQLite<br/>Memory DB)]
        FS[(File System)]
    end

    AI -->|JSON-RPC| MCP
    CLI -->|CLI Args| CLI_MOD
    WEB -->|HTTP/WS| WEB_MOD

    MCP --> AGENT
    CLI_MOD --> AGENT
    WEB_MOD --> AGENT

    AGENT --> TOOLS
    AGENT --> LSP
    AGENT --> MEMORY
    AGENT --> CONFIG

    TOOLS --> CORE
    LSP --> CORE
    MEMORY --> CORE
    CONFIG --> CORE

    TOOLS --> FS
    LSP --> LS_PY
    LSP --> LS_RS
    LSP --> LS_TS
    LSP --> LS_GO
    MEMORY --> DB

    style AGENT fill:#f9f,stroke:#333,stroke-width:4px
    style MCP fill:#bbf,stroke:#333,stroke-width:2px
    style CORE fill:#bfb,stroke:#333,stroke-width:2px
```

## Crate Dependency Graph

```mermaid
graph LR
    SERENA[serena]
    CLI[serena-cli]
    WEB[serena-web]
    MCP[serena-mcp]
    TOOLS[serena-tools]
    LSP[serena-lsp]
    MEMORY[serena-memory]
    CONFIG[serena-config]
    CORE[serena-core]

    SERENA --> CLI
    SERENA --> WEB
    SERENA --> MCP
    SERENA --> TOOLS
    SERENA --> LSP
    SERENA --> MEMORY
    SERENA --> CONFIG

    CLI --> TOOLS
    CLI --> CONFIG
    CLI --> CORE

    WEB --> TOOLS
    WEB --> CONFIG
    WEB --> CORE

    MCP --> TOOLS
    MCP --> CONFIG
    MCP --> CORE

    TOOLS --> LSP
    TOOLS --> MEMORY
    TOOLS --> CONFIG
    TOOLS --> CORE

    LSP --> CORE
    MEMORY --> CORE
    CONFIG --> CORE

    style CORE fill:#bfb,stroke:#333,stroke-width:3px
```

## Tool Execution Flow

```mermaid
sequenceDiagram
    participant AI as AI Agent
    participant MCP as MCP Server
    participant Agent as Serena Agent
    participant Tools as Tool Registry
    participant LSP as LSP Manager
    participant LS as Language Server

    AI->>MCP: tools/call: find_symbol
    MCP->>Agent: execute_tool("find_symbol", params)
    Agent->>Tools: get("find_symbol")
    Tools-->>Agent: FindSymbolTool
    Agent->>Tools: tool.execute(params)

    Tools->>LSP: get_server(Language::Python)
    LSP-->>Tools: PythonLSP
    Tools->>LS: document_symbols(uri)
    LS-->>Tools: Vec<SymbolInfo>

    Tools->>Tools: filter by name_path
    Tools-->>Agent: ToolResult
    Agent-->>MCP: ToolResult
    MCP-->>AI: JSON Response
```

## LSP Client Architecture

```mermaid
graph TB
    subgraph "Language Server Manager"
        MGR[Manager]
        CACHE[Response Cache]
    end

    subgraph "Language Servers"
        PY[Python LSP]
        RS[Rust LSP]
        TS[TypeScript LSP]
        GO[Go LSP]
    end

    subgraph "LSP Client"
        CLIENT[Generic Client]
        STDIO[Stdio Transport]
        PARSER[JSON-RPC Parser]
    end

    subgraph "External Processes"
        PYLS[pyright<br/>process]
        RSLS[rust-analyzer<br/>process]
        TSLS[typescript-language-server<br/>process]
        GOLS[gopls<br/>process]
    end

    MGR --> PY
    MGR --> RS
    MGR --> TS
    MGR --> GO
    MGR --> CACHE

    PY --> CLIENT
    RS --> CLIENT
    TS --> CLIENT
    GO --> CLIENT

    CLIENT --> STDIO
    STDIO --> PARSER

    PARSER <-->|stdin/stdout| PYLS
    PARSER <-->|stdin/stdout| RSLS
    PARSER <-->|stdin/stdout| TSLS
    PARSER <-->|stdin/stdout| GOLS

    style MGR fill:#f9f,stroke:#333,stroke-width:2px
    style CLIENT fill:#bbf,stroke:#333,stroke-width:2px
```

## Data Flow: File Tool Execution

```mermaid
graph TD
    START[AI Request: read_file]
    VALIDATE[Validate Parameters]
    CHECK_PROJECT[Check Active Project]
    RESOLVE[Resolve Absolute Path]
    CHECK_FILE[File Exists?]
    READ[Read File Content]
    SLICE[Apply Line Range]
    CACHE[Cache Result]
    RETURN[Return ToolResult]

    START --> VALIDATE
    VALIDATE -->|Valid| CHECK_PROJECT
    VALIDATE -->|Invalid| ERROR1[Error: Invalid Params]

    CHECK_PROJECT -->|Project Active| RESOLVE
    CHECK_PROJECT -->|No Project| ERROR2[Error: No Active Project]

    RESOLVE --> CHECK_FILE
    CHECK_FILE -->|Exists| READ
    CHECK_FILE -->|Not Found| ERROR3[Error: File Not Found]

    READ --> SLICE
    SLICE --> CACHE
    CACHE --> RETURN

    style START fill:#bfb,stroke:#333,stroke-width:2px
    style RETURN fill:#bfb,stroke:#333,stroke-width:2px
    style ERROR1 fill:#fbb,stroke:#333,stroke-width:2px
    style ERROR2 fill:#fbb,stroke:#333,stroke-width:2px
    style ERROR3 fill:#fbb,stroke:#333,stroke-width:2px
```

## Memory System Architecture

```mermaid
graph TB
    subgraph "Memory Interface"
        WRITE[Write Memory]
        READ[Read Memory]
        SEARCH[Search Memory]
        LIST[List Memories]
    end

    subgraph "Storage Backend"
        TRAIT[MemoryStorage Trait]
        SQLITE[SQLite Implementation]
        SLED[Sled Implementation<br/>Alternative]
    end

    subgraph "SQLite Storage"
        TABLE[memories table]
        FTS[memories_fts<br/>Full-Text Search]
    end

    subgraph "Export/Import"
        MD[Markdown Files]
    end

    WRITE --> TRAIT
    READ --> TRAIT
    SEARCH --> TRAIT
    LIST --> TRAIT

    TRAIT --> SQLITE
    TRAIT -.->|Optional| SLED

    SQLITE --> TABLE
    SQLITE --> FTS

    TABLE <--> MD
    FTS --> TABLE

    style TRAIT fill:#bbf,stroke:#333,stroke-width:2px
    style SQLITE fill:#f9f,stroke:#333,stroke-width:2px
```

## Configuration Loading Hierarchy

```mermaid
graph TD
    START[Application Start]
    ENV[Environment Variables]
    CLI_ARGS[CLI Arguments]
    USER_CONFIG[~/.config/serena/serena.yaml]
    PROJECT_CONFIG[.serena/project.yaml]
    DEFAULT[Default Configuration]

    START --> ENV
    ENV --> CLI_ARGS
    CLI_ARGS --> USER_CONFIG
    USER_CONFIG --> PROJECT_CONFIG
    PROJECT_CONFIG --> DEFAULT

    DEFAULT --> MERGED[Merged Configuration]

    MERGED --> VALIDATE[Validate Config]
    VALIDATE -->|Valid| READY[Configuration Ready]
    VALIDATE -->|Invalid| ERROR[Configuration Error]

    style START fill:#bfb,stroke:#333,stroke-width:2px
    style MERGED fill:#f9f,stroke:#333,stroke-width:2px
    style READY fill:#bfb,stroke:#333,stroke-width:2px
    style ERROR fill:#fbb,stroke:#333,stroke-width:2px
```

## Async Runtime Architecture

```mermaid
graph TB
    subgraph "Tokio Runtime"
        RUNTIME[Multi-threaded Runtime]
        WORK_STEAL[Work-Stealing Scheduler]
    end

    subgraph "Async Tasks"
        MCP_TASK[MCP Server Task]
        WEB_TASK[Web Server Task]
        LSP_TASKS[LSP Client Tasks]
        TOOL_TASKS[Tool Execution Tasks]
    end

    subgraph "Synchronous Work"
        RAYON[Rayon Thread Pool]
        FILE_OPS[File Operations]
        SEARCH[Parallel Search]
    end

    RUNTIME --> WORK_STEAL
    WORK_STEAL --> MCP_TASK
    WORK_STEAL --> WEB_TASK
    WORK_STEAL --> LSP_TASKS
    WORK_STEAL --> TOOL_TASKS

    TOOL_TASKS -->|CPU-bound| RAYON
    RAYON --> FILE_OPS
    RAYON --> SEARCH

    style RUNTIME fill:#f9f,stroke:#333,stroke-width:3px
    style RAYON fill:#bbf,stroke:#333,stroke-width:2px
```

## Build and Release Pipeline

```mermaid
graph LR
    subgraph "Development"
        CODE[Write Code]
        FMT[cargo fmt]
        CLIPPY[cargo clippy]
        TEST[cargo test]
    end

    subgraph "Build"
        BUILD_LINUX[Build Linux x64]
        BUILD_WIN[Build Windows x64]
        BUILD_MAC[Build macOS x64/ARM]
    end

    subgraph "Package"
        TAR[tar.gz Archives]
        ZIP[Windows ZIP]
        DEB[Debian Package]
        BREW[Homebrew Formula]
    end

    subgraph "Release"
        GH_RELEASE[GitHub Release]
        CRATES_IO[crates.io]
        DOCS[docs.rs]
    end

    CODE --> FMT
    FMT --> CLIPPY
    CLIPPY --> TEST

    TEST --> BUILD_LINUX
    TEST --> BUILD_WIN
    TEST --> BUILD_MAC

    BUILD_LINUX --> TAR
    BUILD_LINUX --> DEB
    BUILD_WIN --> ZIP
    BUILD_MAC --> TAR
    BUILD_MAC --> BREW

    TAR --> GH_RELEASE
    ZIP --> GH_RELEASE
    DEB --> GH_RELEASE
    BREW --> GH_RELEASE

    GH_RELEASE --> CRATES_IO
    CRATES_IO --> DOCS

    style TEST fill:#bfb,stroke:#333,stroke-width:2px
    style GH_RELEASE fill:#f9f,stroke:#333,stroke-width:2px
```

## Migration Timeline

```mermaid
gantt
    title Serena Rust Migration Timeline
    dateFormat  YYYY-MM-DD
    section Foundation
    Workspace Setup           :2025-01-01, 1w
    Core Types & Traits      :2025-01-08, 1w
    Config System            :2025-01-15, 1w

    section LSP
    Generic LSP Client       :2025-01-22, 2w
    Python/Rust/TS Servers   :2025-02-05, 1w

    section Tools
    File Tools               :2025-02-12, 1w
    Symbol Tools             :2025-02-19, 2w

    section Protocol
    MCP Server               :2025-03-05, 1w

    section Storage
    Memory System            :2025-03-12, 1w

    section Optional
    Web Dashboard            :2025-03-19, 2w

    section Scale
    Multi-Language Support   :2025-04-02, 4w

    section Polish
    Optimization & Docs      :2025-04-30, 2w
```

## Performance Comparison: Python vs Rust

```mermaid
graph LR
    subgraph "Startup Time"
        PY_START[Python: 2-3s]
        RS_START[Rust: 0.2-0.3s]
    end

    subgraph "Memory Usage"
        PY_MEM[Python: 150-300MB]
        RS_MEM[Rust: 20-50MB]
    end

    subgraph "File Search (10k files)"
        PY_SEARCH[Python: 800ms]
        RS_SEARCH[Rust: 80ms]
    end

    subgraph "Binary Size"
        PY_SIZE[Python: 50MB<br/>+ runtime]
        RS_SIZE[Rust: 15-25MB<br/>self-contained]
    end

    style RS_START fill:#bfb,stroke:#333,stroke-width:2px
    style RS_MEM fill:#bfb,stroke:#333,stroke-width:2px
    style RS_SEARCH fill:#bfb,stroke:#333,stroke-width:2px
    style RS_SIZE fill:#bfb,stroke:#333,stroke-width:2px
```

## Deployment Options

```mermaid
graph TB
    BIN[serena Binary]

    subgraph "Standalone"
        SINGLE[Single Binary<br/>15-25MB]
        CONFIG_DIR[~/.config/serena/]
        DATA_DIR[~/.local/share/serena/]
    end

    subgraph "Container"
        DOCKER[Docker Image<br/>scratch-based]
        SMALL[25-30MB total]
    end

    subgraph "System Integration"
        SYSTEMD[systemd Service]
        LAUNCHD[macOS launchd]
        WINDOWS_SVC[Windows Service]
    end

    BIN --> SINGLE
    BIN --> DOCKER
    BIN --> SYSTEMD
    BIN --> LAUNCHD
    BIN --> WINDOWS_SVC

    SINGLE --> CONFIG_DIR
    SINGLE --> DATA_DIR

    style BIN fill:#f9f,stroke:#333,stroke-width:3px
    style SINGLE fill:#bfb,stroke:#333,stroke-width:2px
    style DOCKER fill:#bbf,stroke:#333,stroke-width:2px
```
