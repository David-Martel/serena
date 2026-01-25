# Serena Build System
# ===================
# Justfile for building Serena components including:
# - Native Rust MCP server binary
# - WASM Dashboard frontend
# - Python package
#
# Install just: cargo install just
# Install trunk: cargo install trunk
# Install wasm-pack: cargo install wasm-pack
#
# Windows Users: Use .\build.ps1 for enhanced diagnostics
#   .\build.ps1 diag  - Pre-build diagnostics
#   .\build.ps1 ci    - Full CI pipeline

# Set shell for Windows compatibility
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Default recipe - show available commands
default:
    @just --list

# =============================================================================
# Pre-build Diagnostics (RECOMMENDED FIRST)
# =============================================================================

# Run pre-build diagnostics (Windows PowerShell)
[windows]
diag:
    @powershell -ExecutionPolicy Bypass -File build.ps1 diag

# Run pre-build diagnostics (Unix)
[unix]
diag:
    @echo "=== Pre-build Diagnostics ==="
    @echo "Rust version:" && rustc --version
    @echo "Cargo version:" && cargo --version
    @echo "Targets:" && rustup target list --installed
    @echo "Checking workspace..." && cargo check --workspace 2>&1 | head -20

# Show environment information
[windows]
env:
    @powershell -ExecutionPolicy Bypass -File build.ps1 env

[unix]
env:
    @echo "Rust:" && rustc --version && cargo --version
    @echo "Targets:" && rustup target list --installed

# =============================================================================
# Native Rust Binary Builds
# =============================================================================

# Build the MCP server in release mode
build-release:
    cargo build --release --package serena_core --bin serena-mcp-server

# Build the MCP server in debug mode
build-debug:
    cargo build --package serena_core --bin serena-mcp-server

# Build all Rust crates
build-all:
    cargo build --workspace

# Build all in release mode
build-all-release:
    cargo build --release --workspace

# Build with optimizations for size
build-size-optimized:
    RUSTFLAGS="-C opt-level=z -C lto=fat" cargo build --release --package serena_core --bin serena-mcp-server

# =============================================================================
# WASM Dashboard Builds
# =============================================================================

# Build WASM dashboard with trunk (development)
wasm-dev:
    cd crates/serena-dashboard && trunk build

# Build WASM dashboard with trunk (release, optimized)
wasm-release:
    cd crates/serena-dashboard && trunk build --release

# Serve WASM dashboard locally for development
wasm-serve:
    cd crates/serena-dashboard && trunk serve --open

# Build WASM using wasm-pack (alternative to trunk)
wasm-pack:
    cd crates/serena-dashboard && wasm-pack build --target web --out-dir pkg

# Clean WASM build artifacts
wasm-clean:
    rm -rf crates/serena-dashboard/dist crates/serena-dashboard/pkg

# =============================================================================
# Combined Builds
# =============================================================================

# Build everything: native binary + WASM dashboard
build: build-release wasm-release
    @echo "Built native binary and WASM dashboard"

# Full release build with all optimizations
release: build-all-release wasm-release
    @echo "Full release build complete"

# =============================================================================
# Testing
# =============================================================================

# Run all Rust tests
test:
    cargo test --workspace

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Run specific crate tests
test-core:
    cargo test --package serena_core

# Run WASM tests in browser
test-wasm:
    cd crates/serena-dashboard && wasm-pack test --headless --chrome

# =============================================================================
# Python Integration
# =============================================================================

# Format Python code
py-format:
    uv run poe format

# Type check Python
py-typecheck:
    uv run poe type-check

# Run Python tests
py-test:
    uv run poe test

# Full Python check
py-check: py-format py-typecheck py-test

# =============================================================================
# Code Quality
# =============================================================================

# Run clippy lints
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Format Rust code
fmt:
    cargo fmt --all

# Check formatting without modifying
fmt-check:
    cargo fmt --all -- --check

# Run all quality checks
check: fmt-check lint test
    @echo "All quality checks passed"

# Auto-fix all issues (format + clippy + cargo fix)
[windows]
fix:
    @powershell -ExecutionPolicy Bypass -File build.ps1 fix

[unix]
fix:
    cargo fmt --all
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged
    cargo fix --workspace --all-targets --allow-dirty --allow-staged

# =============================================================================
# Benchmarks
# =============================================================================

# Run criterion benchmarks
bench:
    cargo bench --package serena_core

# Run benchmarks and save baseline
bench-baseline name:
    cargo bench --package serena_core -- --save-baseline {{name}}

# Compare against baseline
bench-compare name:
    cargo bench --package serena_core -- --baseline {{name}}

# =============================================================================
# Documentation
# =============================================================================

# Build documentation
docs:
    cargo doc --workspace --no-deps --open

# Build docs with private items
docs-private:
    cargo doc --workspace --no-deps --document-private-items --open

# =============================================================================
# Installation
# =============================================================================

# Install build tools
install-tools:
    cargo install trunk wasm-pack just
    rustup target add wasm32-unknown-unknown

# Install the MCP server locally
install:
    cargo install --path serena_core

# =============================================================================
# Clean
# =============================================================================

# Clean all build artifacts
clean:
    cargo clean
    just wasm-clean
    rm -rf dist __pycache__ .pytest_cache

# Clean and rebuild
rebuild: clean build

# =============================================================================
# Development Helpers
# =============================================================================

# Watch and rebuild on changes
watch:
    cargo watch -x check -x test

# Watch WASM dashboard with hot reload
watch-wasm:
    cd crates/serena-dashboard && trunk watch

# Start the MCP server
run:
    cargo run --release --package serena_core --bin serena-mcp-server

# Start with debug logging
run-debug:
    RUST_LOG=debug cargo run --package serena_core --bin serena-mcp-server

# =============================================================================
# E2E Tests (Playwright)
# =============================================================================

# Install E2E test dependencies
e2e-install:
    cd e2e && npm ci && npx playwright install chromium

# Run all E2E tests
e2e:
    cd e2e && npm test

# Run E2E tests with UI
e2e-ui:
    cd e2e && npm run test:ui

# Run frontend tests only
e2e-frontend:
    cd e2e && npm run test:frontend

# Run integration tests only
e2e-integration:
    cd e2e && npm run test:integration

# Run performance tests only
e2e-performance:
    cd e2e && npm run test:performance

# Run stability tests only
e2e-stability:
    cd e2e && npm run test:stability

# Show E2E test report
e2e-report:
    cd e2e && npm run test:report

# =============================================================================
# CI/CD
# =============================================================================

# Full CI check (what runs in GitHub Actions)
ci: check py-check build-all-release wasm-release
    @echo "CI checks passed"

# Pre-commit hook
pre-commit: fmt lint test
    @echo "Pre-commit checks passed"

# =============================================================================
# Deployment
# =============================================================================

# Package for distribution
package: release
    mkdir -p dist
    cp target/release/serena-mcp-server dist/ || cp target/release/serena-mcp-server.exe dist/
    cp -r crates/serena-dashboard/dist dist/dashboard
    @echo "Package created in dist/"

# Create GitHub release assets
release-assets: package
    cd dist && tar -czvf serena-$(git describe --tags).tar.gz *
    @echo "Release assets created"
