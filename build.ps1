# Serena Build System - PowerShell
# ================================
# Comprehensive build automation with diagnostics for Windows
#
# Usage:
#   .\build.ps1                     # Show help
#   .\build.ps1 diag                # Run pre-build diagnostics
#   .\build.ps1 build               # Build workspace
#   .\build.ps1 build -Release      # Release build
#   .\build.ps1 test                # Run all tests
#   .\build.ps1 test -Package serena-core  # Test specific crate
#   .\build.ps1 ci                  # Full CI pipeline
#   .\build.ps1 fix                 # Auto-fix all issues

param(
    [Parameter(Position=0)]
    [ValidateSet('help', 'diag', 'env', 'build', 'test', 'check', 'lint', 'fix', 'fmt', 'doc', 'clean', 'ci', 'mcp', 'mcp-test', 'watch')]
    [string]$Command = 'help',

    [switch]$Release,
    [switch]$ShowOutput,
    [string]$Package = '',
    [switch]$AllTargets,
    [switch]$NoDeps
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Colors for output
function Write-Header { param([string]$msg) Write-Host "`n=== $msg ===" -ForegroundColor Cyan }
function Write-Success { param([string]$msg) Write-Host "[OK] $msg" -ForegroundColor Green }
function Write-Warning { param([string]$msg) Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Error { param([string]$msg) Write-Host "[ERROR] $msg" -ForegroundColor Red }
function Write-Info { param([string]$msg) Write-Host "[INFO] $msg" -ForegroundColor Blue }

# ============================================================================
# Pre-build Diagnostics
# ============================================================================

function Show-Environment {
    Write-Header "Environment Information"

    # Rust toolchain
    Write-Info "Rust Toolchain:"
    rustc --version
    cargo --version

    # Check for required targets
    Write-Info "`nInstalled Targets:"
    rustup target list --installed

    # MSVC info
    Write-Info "`nVisual Studio / MSVC:"
    if (Test-Path 'C:\Program Files\Microsoft Visual Studio') {
        Get-ChildItem 'C:\Program Files\Microsoft Visual Studio' -Directory | ForEach-Object {
            Write-Host "  Found: $($_.Name)"
        }
    }

    # Linker info
    Write-Info "`nLinker Path (link.exe):"
    $linkPath = (Get-Command link.exe -ErrorAction SilentlyContinue).Source
    if ($linkPath) {
        Write-Host "  $linkPath"
        # Get version
        & link.exe 2>&1 | Select-Object -First 2
    } else {
        Write-Warning "link.exe not found in PATH"
    }

    # Environment variables
    Write-Info "`nRelevant Environment Variables:"
    @('RUSTFLAGS', 'CARGO_BUILD_TARGET', 'CC', 'CXX', 'LIB', 'LIBPATH', 'INCLUDE') | ForEach-Object {
        $val = [Environment]::GetEnvironmentVariable($_)
        if ($val) { Write-Host "  $_=$val" }
    }
}

function Show-Diagnostics {
    Write-Header "Pre-build Diagnostics"

    # Check Rust version
    Write-Info "Checking Rust toolchain..."
    $rustVersion = rustc --version
    if ($rustVersion -match '(\d+\.\d+\.\d+)') {
        $version = [version]$Matches[1]
        $minVersion = [version]'1.75.0'
        if ($version -ge $minVersion) {
            Write-Success "Rust $($Matches[1]) >= 1.75.0 required"
        } else {
            Write-Warning "Rust $($Matches[1]) < 1.75.0 - may have issues"
        }
    }

    # Check cargo-make
    Write-Info "Checking cargo-make..."
    $cargoMake = Get-Command cargo-make -ErrorAction SilentlyContinue
    if ($cargoMake) {
        Write-Success "cargo-make is installed"
    } else {
        Write-Warning "cargo-make not found - install with: cargo install cargo-make"
    }

    # Check just
    Write-Info "Checking just..."
    $just = Get-Command just -ErrorAction SilentlyContinue
    if ($just) {
        Write-Success "just is installed"
    } else {
        Write-Warning "just not found - install with: cargo install just"
    }

    # Check MSVC toolchain
    Write-Info "Checking MSVC toolchain..."
    try {
        $clOutput = & cl.exe 2>&1
        if ($clOutput -match 'Version (\d+\.\d+)') {
            Write-Success "MSVC cl.exe version $($Matches[1])"
        }
    } catch {
        Write-Warning "cl.exe not accessible - ensure VS Developer environment is active"
    }

    # Check for C library issues (common with bundled SQLite)
    Write-Info "Checking for problematic dependencies..."
    $cargoTree = cargo tree --workspace -f "{p}" 2>&1
    $problematic = @('libsqlite3-sys', 'zstd-sys', 'bzip2-sys')
    foreach ($dep in $problematic) {
        if ($cargoTree -match $dep) {
            Write-Warning "$dep found - may have C compilation issues on some MSVC versions"
        }
    }

    # Check workspace structure
    Write-Info "Workspace members:"
    try {
        $cargoToml = Get-Content .\Cargo.toml -Raw
        # Extract members array more carefully
        $members = @()
        foreach ($line in (Get-Content .\Cargo.toml)) {
            if ($line -match '^\s*"([^"]+)"') {
                $member = $Matches[1]
                if ($member -match '^crates/') {
                    $members += $member
                }
            }
        }
        foreach ($member in $members) {
            if (Test-Path $member) {
                Write-Success "  $member"
            } else {
                Write-Host "  [MISSING] $member" -ForegroundColor Red
            }
        }
    } catch {
        Write-Warning "Could not parse Cargo.toml: $_"
    }

    # Check for common Windows issues
    Write-Info "Checking for Windows-specific issues..."
    $targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { ".\target" }
    if (Test-Path $targetDir) {
        $pathLength = (Resolve-Path $targetDir).Path.Length
        if ($pathLength -gt 200) {
            Write-Warning "Target dir path length ($pathLength) may cause issues on Windows"
        } else {
            Write-Success "Target dir path length OK ($pathLength chars)"
        }
    }
}

# ============================================================================
# Build Commands
# ============================================================================

function Invoke-Build {
    Write-Header "Building Workspace"

    $args = @('build', '--workspace')

    if ($Release) { $args += '--release' }
    if ($Package) { $args = @('build', '--package', $Package); if ($Release) { $args += '--release' } }
    if ($AllTargets) { $args += '--all-targets' }

    Write-Info "cargo $($args -join ' ')"
    & cargo @args

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Build completed successfully"
    } else {
        Write-Error "Build failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }
}

function Invoke-Test {
    Write-Header "Running Tests"

    $args = @('test', '--workspace')

    if ($Release) { $args += '--release' }
    if ($Package) { $args = @('test', '--package', $Package); if ($Release) { $args += '--release' } }
    if ($ShowOutput) { $args += '--', '--nocapture' }

    Write-Info "cargo $($args -join ' ')"
    & cargo @args

    if ($LASTEXITCODE -eq 0) {
        Write-Success "All tests passed"
    } else {
        Write-Error "Tests failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }
}

function Invoke-Check {
    Write-Header "Checking Code"

    $args = @('check', '--workspace', '--all-targets')

    Write-Info "cargo $($args -join ' ')"
    & cargo @args

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Check passed"
    }
}

function Invoke-Lint {
    Write-Header "Running Clippy"

    $args = @('clippy', '--workspace', '--all-targets', '--', '-D', 'warnings')

    Write-Info "cargo $($args -join ' ')"
    & cargo @args

    if ($LASTEXITCODE -eq 0) {
        Write-Success "Clippy passed"
    }
}

function Invoke-Fix {
    Write-Header "Auto-fixing Issues"

    # Format first
    Write-Info "Formatting code..."
    & cargo fmt --all

    # Then clippy fix
    Write-Info "Applying clippy fixes..."
    & cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

    # Then cargo fix
    Write-Info "Applying cargo fixes..."
    & cargo fix --workspace --all-targets --allow-dirty --allow-staged

    Write-Success "Auto-fix completed"
}

function Invoke-Format {
    Write-Header "Formatting Code"
    & cargo fmt --all
    Write-Success "Formatting completed"
}

function Invoke-Doc {
    Write-Header "Generating Documentation"

    $args = @('doc', '--workspace')
    if ($NoDeps) { $args += '--no-deps' }
    $args += '--open'

    & cargo @args
}

function Invoke-Clean {
    Write-Header "Cleaning Build Artifacts"
    & cargo clean

    # Clean additional artifacts
    if (Test-Path '.\dist') { Remove-Item -Recurse -Force '.\dist' }
    if (Test-Path '.\.pytest_cache') { Remove-Item -Recurse -Force '.\.pytest_cache' }

    Write-Success "Clean completed"
}

function Invoke-CI {
    Write-Header "Full CI Pipeline"

    $steps = @(
        @{ Name = "Diagnostics"; Cmd = { Show-Diagnostics } },
        @{ Name = "Format Check"; Cmd = { & cargo fmt --all --check } },
        @{ Name = "Clippy"; Cmd = { & cargo clippy --workspace --all-targets -- -D warnings } },
        @{ Name = "Build"; Cmd = { & cargo build --workspace } },
        @{ Name = "Test"; Cmd = { & cargo test --workspace } },
        @{ Name = "Doc"; Cmd = { & cargo doc --workspace --no-deps } }
    )

    $failed = @()

    foreach ($step in $steps) {
        Write-Info "Running: $($step.Name)"
        try {
            & $step.Cmd
            if ($LASTEXITCODE -ne 0) {
                throw "Exit code: $LASTEXITCODE"
            }
            Write-Success "$($step.Name) passed"
        } catch {
            Write-Error "$($step.Name) failed: $_"
            $failed += $step.Name
        }
    }

    Write-Header "CI Summary"
    if ($failed.Count -eq 0) {
        Write-Success "All CI steps passed!"
    } else {
        Write-Error "Failed steps: $($failed -join ', ')"
        exit 1
    }
}

function Invoke-MCP {
    Write-Header "Starting MCP Server"

    $buildMode = if ($Release) { '--release' } else { '' }
    $args = @('run', '--package', 'serena', '--', 'start', '--transport', 'stdio')
    if ($Release) { $args = @('run', '--release', '--package', 'serena', '--', 'start', '--transport', 'stdio') }

    Write-Info "cargo $($args -join ' ')"
    & cargo @args
}

function Invoke-MCPTest {
    Write-Header "Testing MCP Server"

    # Build first
    Write-Info "Building MCP server..."
    & cargo build --package serena

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Build failed"
        exit 1
    }

    # Run test script
    if (Test-Path '.\test_mcp.ps1') {
        Write-Info "Running MCP test script..."
        & .\test_mcp.ps1
    } else {
        Write-Warning "test_mcp.ps1 not found"
    }
}

function Invoke-Watch {
    Write-Header "Watching for Changes"

    $cargoWatch = Get-Command cargo-watch -ErrorAction SilentlyContinue
    if (-not $cargoWatch) {
        Write-Warning "cargo-watch not installed. Installing..."
        & cargo install cargo-watch
    }

    & cargo watch -x check -x test
}

function Show-Help {
    @"
Serena Build System - PowerShell
================================

USAGE:
    .\build.ps1 <command> [options]

COMMANDS:
    help        Show this help message
    diag        Run pre-build diagnostics (RECOMMENDED FIRST)
    env         Show environment information
    build       Build the workspace
    test        Run all tests
    check       Quick check without building
    lint        Run clippy lints
    fix         Auto-fix all issues (format + clippy + cargo fix)
    fmt         Format code
    doc         Generate documentation
    clean       Clean build artifacts
    ci          Full CI pipeline
    mcp         Start MCP server
    mcp-test    Test MCP server
    watch       Watch for changes and rebuild

OPTIONS:
    -Release     Build in release mode
    -ShowOutput  Show test output (--nocapture)
    -Package     Target specific package (e.g., -Package serena-core)
    -AllTargets  Include all targets (tests, examples, benches)
    -NoDeps      Exclude dependencies (for doc command)

EXAMPLES:
    .\build.ps1 diag                    # Run diagnostics first
    .\build.ps1 build -Release          # Release build
    .\build.ps1 test -Package serena-core -Verbose
    .\build.ps1 fix                     # Auto-fix all issues
    .\build.ps1 ci                      # Full CI pipeline

WORKFLOW (Recommended):
    1. .\build.ps1 diag          # Check environment
    2. .\build.ps1 fix           # Auto-fix issues
    3. .\build.ps1 build         # Build
    4. .\build.ps1 test          # Test
    5. .\build.ps1 ci            # Full CI before commit

"@
}

# ============================================================================
# Main Entry Point
# ============================================================================

switch ($Command) {
    'help'      { Show-Help }
    'diag'      { Show-Diagnostics; Show-Environment }
    'env'       { Show-Environment }
    'build'     { Invoke-Build }
    'test'      { Invoke-Test }
    'check'     { Invoke-Check }
    'lint'      { Invoke-Lint }
    'fix'       { Invoke-Fix }
    'fmt'       { Invoke-Format }
    'doc'       { Invoke-Doc }
    'clean'     { Invoke-Clean }
    'ci'        { Invoke-CI }
    'mcp'       { Invoke-MCP }
    'mcp-test'  { Invoke-MCPTest }
    'watch'     { Invoke-Watch }
    default     { Show-Help }
}
