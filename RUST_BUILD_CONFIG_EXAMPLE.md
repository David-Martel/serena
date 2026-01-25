# Rust Build Configuration Examples

## Makefile.toml (cargo-make)

```toml
[config]
default_to_workspace = false
reduce_output = false

[env]
CARGO_MAKE_EXTEND_WORKSPACE_MAKEFILE = true
RUST_BACKTRACE = "1"

# Development tasks
[tasks.dev]
description = "Quick development build"
command = "cargo"
args = ["build", "--workspace"]

[tasks.dev-watch]
description = "Watch and rebuild on changes"
install_crate = "cargo-watch"
command = "cargo"
args = ["watch", "-x", "build"]

# Testing tasks
[tasks.test]
description = "Run all tests"
command = "cargo"
args = ["test", "--workspace"]

[tasks.test-fast]
description = "Run tests without LSP downloads"
env = { SERENA_SKIP_LSP_DOWNLOADS = "1" }
command = "cargo"
args = ["test", "--workspace", "--", "--test-threads=4"]

[tasks.test-integration]
description = "Run integration tests only"
command = "cargo"
args = ["test", "--test", "integration", "--", "--nocapture"]

[tasks.test-coverage]
description = "Generate test coverage report"
install_crate = "cargo-tarpaulin"
command = "cargo"
args = [
    "tarpaulin",
    "--workspace",
    "--out", "Html",
    "--output-dir", "target/coverage"
]

# Code quality tasks
[tasks.fmt]
description = "Format code"
command = "cargo"
args = ["fmt", "--all"]

[tasks.fmt-check]
description = "Check code formatting"
command = "cargo"
args = ["fmt", "--all", "--", "--check"]

[tasks.clippy]
description = "Run clippy lints"
command = "cargo"
args = [
    "clippy",
    "--workspace",
    "--all-targets",
    "--all-features",
    "--",
    "-D", "warnings",
    "-W", "clippy::pedantic",
    "-A", "clippy::module_name_repetitions",
    "-A", "clippy::missing_errors_doc",
    "-A", "clippy::missing_panics_doc"
]

[tasks.clippy-fix]
description = "Auto-fix clippy warnings"
command = "cargo"
args = [
    "clippy",
    "--workspace",
    "--all-targets",
    "--fix",
    "--allow-dirty",
    "--allow-staged"
]

[tasks.check-all]
description = "Run all checks (fmt, clippy, test)"
dependencies = ["fmt-check", "clippy", "test"]

# Build tasks
[tasks.build-release]
description = "Build optimized release"
command = "cargo"
args = ["build", "--workspace", "--release"]

[tasks.build-release-optimized]
description = "Build with maximum optimization"
command = "cargo"
args = ["build", "--workspace", "--profile", "release-optimized"]

# Cross-compilation tasks
[tasks.cross-windows]
description = "Cross-compile for Windows x64"
install_crate = "cross"
command = "cross"
args = [
    "build",
    "--target", "x86_64-pc-windows-msvc",
    "--release",
    "-p", "serena"
]

[tasks.cross-linux]
description = "Cross-compile for Linux x64"
install_crate = "cross"
command = "cross"
args = [
    "build",
    "--target", "x86_64-unknown-linux-gnu",
    "--release",
    "-p", "serena"
]

[tasks.cross-macos-x64]
description = "Cross-compile for macOS x64"
install_crate = "cross"
command = "cross"
args = [
    "build",
    "--target", "x86_64-apple-darwin",
    "--release",
    "-p", "serena"
]

[tasks.cross-macos-arm]
description = "Cross-compile for macOS ARM"
install_crate = "cross"
command = "cross"
args = [
    "build",
    "--target", "aarch64-apple-darwin",
    "--release",
    "-p", "serena"
]

[tasks.cross-all]
description = "Cross-compile for all platforms"
dependencies = [
    "cross-windows",
    "cross-linux",
    "cross-macos-x64",
    "cross-macos-arm"
]

# Documentation tasks
[tasks.doc]
description = "Generate documentation"
command = "cargo"
args = ["doc", "--workspace", "--no-deps"]

[tasks.doc-open]
description = "Generate and open documentation"
command = "cargo"
args = ["doc", "--workspace", "--no-deps", "--open"]

# Benchmarking tasks
[tasks.bench]
description = "Run benchmarks"
command = "cargo"
args = ["bench", "--workspace"]

[tasks.bench-baseline]
description = "Save benchmark baseline"
command = "cargo"
args = ["bench", "--workspace", "--", "--save-baseline", "main"]

[tasks.bench-compare]
description = "Compare against baseline"
command = "cargo"
args = ["bench", "--workspace", "--", "--baseline", "main"]

# Profiling tasks
[tasks.profile-cpu]
description = "Profile CPU usage (Linux only)"
script = '''
#!/bin/bash
cargo build --release -p serena
perf record -F 99 -g target/release/serena start --project test
perf script | stackcollapse-perf.pl | flamegraph.pl > flamegraph.svg
echo "Flame graph saved to flamegraph.svg"
'''

[tasks.profile-memory]
description = "Profile memory usage"
install_crate = "cargo-instruments"
command = "cargo"
args = [
    "instruments",
    "-t", "Allocations",
    "--release",
    "-p", "serena",
    "--",
    "start", "--project", "test"
]

# Package and release tasks
[tasks.package-prep]
description = "Prepare for packaging"
script = '''
#!/bin/bash
mkdir -p dist
rm -rf dist/*
'''

[tasks.package-linux]
description = "Create Linux distribution package"
dependencies = ["cross-linux"]
script = '''
#!/bin/bash
VERSION=$(cargo metadata --format-version=1 | jq -r '.packages[] | select(.name == "serena") | .version')
mkdir -p dist/serena-${VERSION}-linux-x64

cp target/x86_64-unknown-linux-gnu/release/serena dist/serena-${VERSION}-linux-x64/
cp LICENSE README.md dist/serena-${VERSION}-linux-x64/
mkdir -p dist/serena-${VERSION}-linux-x64/config
cp config/serena.yaml.example dist/serena-${VERSION}-linux-x64/config/

cd dist
tar czf serena-${VERSION}-linux-x64.tar.gz serena-${VERSION}-linux-x64/
sha256sum serena-${VERSION}-linux-x64.tar.gz > serena-${VERSION}-linux-x64.tar.gz.sha256
rm -rf serena-${VERSION}-linux-x64
cd ..

echo "Created dist/serena-${VERSION}-linux-x64.tar.gz"
'''

[tasks.package-windows]
description = "Create Windows distribution package"
dependencies = ["cross-windows"]
script = '''
#!/bin/bash
VERSION=$(cargo metadata --format-version=1 | jq -r '.packages[] | select(.name == "serena") | .version')
mkdir -p dist/serena-${VERSION}-windows-x64

cp target/x86_64-pc-windows-msvc/release/serena.exe dist/serena-${VERSION}-windows-x64/
cp LICENSE README.md dist/serena-${VERSION}-windows-x64/
mkdir -p dist/serena-${VERSION}-windows-x64/config
cp config/serena.yaml.example dist/serena-${VERSION}-windows-x64/config/

cd dist
zip -r serena-${VERSION}-windows-x64.zip serena-${VERSION}-windows-x64/
sha256sum serena-${VERSION}-windows-x64.zip > serena-${VERSION}-windows-x64.zip.sha256
rm -rf serena-${VERSION}-windows-x64
cd ..

echo "Created dist/serena-${VERSION}-windows-x64.zip"
'''

[tasks.package-macos]
description = "Create macOS distribution packages"
dependencies = ["cross-macos-x64", "cross-macos-arm"]
script = '''
#!/bin/bash
VERSION=$(cargo metadata --format-version=1 | jq -r '.packages[] | select(.name == "serena") | .version')

# x64 package
mkdir -p dist/serena-${VERSION}-macos-x64
cp target/x86_64-apple-darwin/release/serena dist/serena-${VERSION}-macos-x64/
cp LICENSE README.md dist/serena-${VERSION}-macos-x64/
mkdir -p dist/serena-${VERSION}-macos-x64/config
cp config/serena.yaml.example dist/serena-${VERSION}-macos-x64/config/

cd dist
tar czf serena-${VERSION}-macos-x64.tar.gz serena-${VERSION}-macos-x64/
sha256sum serena-${VERSION}-macos-x64.tar.gz > serena-${VERSION}-macos-x64.tar.gz.sha256
rm -rf serena-${VERSION}-macos-x64
cd ..

# ARM package
mkdir -p dist/serena-${VERSION}-macos-arm64
cp target/aarch64-apple-darwin/release/serena dist/serena-${VERSION}-macos-arm64/
cp LICENSE README.md dist/serena-${VERSION}-macos-arm64/
mkdir -p dist/serena-${VERSION}-macos-arm64/config
cp config/serena.yaml.example dist/serena-${VERSION}-macos-arm64/config/

cd dist
tar czf serena-${VERSION}-macos-arm64.tar.gz serena-${VERSION}-macos-arm64/
sha256sum serena-${VERSION}-macos-arm64.tar.gz > serena-${VERSION}-macos-arm64.tar.gz.sha256
rm -rf serena-${VERSION}-macos-arm64
cd ..

echo "Created macOS packages in dist/"
'''

[tasks.package-all]
description = "Create all distribution packages"
dependencies = ["package-prep"]
run_task = { name = ["package-linux", "package-windows", "package-macos"], parallel = true }

# Docker tasks
[tasks.docker-build]
description = "Build Docker image"
script = '''
#!/bin/bash
VERSION=$(cargo metadata --format-version=1 | jq -r '.packages[] | select(.name == "serena") | .version')
docker build -t serena:${VERSION} -t serena:latest .
'''

[tasks.docker-run]
description = "Run Docker container"
script = '''
#!/bin/bash
docker run -it --rm -v $(pwd):/workspace serena:latest start --project /workspace
'''

# CI tasks
[tasks.ci-prepare]
description = "Prepare CI environment"
script = '''
#!/bin/bash
rustup component add clippy rustfmt
cargo install cargo-tarpaulin
'''

[tasks.ci-check]
description = "CI check pipeline"
dependencies = ["fmt-check", "clippy", "test", "doc"]

[tasks.ci-build]
description = "CI build pipeline"
dependencies = ["ci-check", "build-release"]

# Utility tasks
[tasks.clean]
description = "Clean build artifacts"
command = "cargo"
args = ["clean"]

[tasks.clean-all]
description = "Clean all build artifacts and caches"
script = '''
#!/bin/bash
cargo clean
rm -rf target/
rm -rf dist/
rm -rf .cache/
'''

[tasks.size-check]
description = "Check binary sizes"
script = '''
#!/bin/bash
cargo build --release -p serena
ls -lh target/release/serena
strip target/release/serena
ls -lh target/release/serena
'''

[tasks.deps-update]
description = "Update dependencies"
command = "cargo"
args = ["update"]

[tasks.deps-audit]
description = "Audit dependencies for security issues"
install_crate = "cargo-audit"
command = "cargo"
args = ["audit"]

[tasks.deps-tree]
description = "Show dependency tree"
command = "cargo"
args = ["tree", "--workspace"]

[tasks.deps-outdated]
description = "Check for outdated dependencies"
install_crate = "cargo-outdated"
command = "cargo"
args = ["outdated", "--workspace"]

# Default task
[tasks.default]
alias = "check-all"
```

## .cargo/config.toml

```toml
[build]
# Use all available CPU cores
jobs = 8

# Shared target directory (optional)
# target-dir = "~/.cargo/shared-target"

[target.x86_64-pc-windows-msvc]
# Increase stack size on Windows
rustflags = ["-C", "link-arg=/STACK:8388608"]

[target.x86_64-unknown-linux-gnu]
# Use LLD linker for faster builds on Linux
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.x86_64-apple-darwin]
# macOS-specific flags
rustflags = ["-C", "link-arg=-Wl,-dead_strip"]

# Environment-specific compilation settings
[profile.dev]
# Faster compile times for development
opt-level = 0
debug = true
incremental = true

[profile.release]
# Production builds
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true
panic = "abort"

[profile.release-optimized]
# Maximum optimization (slower build, smaller/faster binary)
inherits = "release"
lto = "fat"
codegen-units = 1
opt-level = "z"  # Optimize for size
strip = true
panic = "abort"

[profile.bench]
# Benchmark-specific settings
inherits = "release"
lto = "thin"

# Alias commands
[alias]
xtask = "run --package xtask --"
```

## Cross.toml (for cross-compilation)

```toml
[build.env]
passthrough = [
    "RUST_BACKTRACE",
    "SERENA_SKIP_LSP_DOWNLOADS",
]

[target.x86_64-pc-windows-msvc]
pre-build = [
    "apt-get update",
    "apt-get install -y mingw-w64"
]

[target.x86_64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/x86_64-unknown-linux-gnu:latest"

[target.x86_64-apple-darwin]
image = "ghcr.io/cross-rs/x86_64-apple-darwin:latest"

[target.aarch64-apple-darwin]
image = "ghcr.io/cross-rs/aarch64-apple-darwin:latest"
```

## Dockerfile (Multi-stage build)

```dockerfile
# Stage 1: Build
FROM rust:1.75-slim as builder

WORKDIR /build

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build release binary
RUN cargo build --release -p serena

# Stage 2: Runtime
FROM scratch

# Copy binary
COPY --from=builder /build/target/release/serena /serena

# Copy CA certificates for HTTPS
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Metadata
LABEL org.opencontainers.image.title="Serena"
LABEL org.opencontainers.image.description="AI Code Agent"
LABEL org.opencontainers.image.version="0.2.0"

ENTRYPOINT ["/serena"]
CMD ["start", "--transport", "stdio"]
```

## GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  RUST_BACKTRACE: 1
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-features

  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings

  test:
    name: Test
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --all-features

  coverage:
    name: Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      - name: Run coverage
        run: cargo tarpaulin --workspace --out Xml
      - name: Upload to codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml

  build:
    name: Build
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release --target ${{ matrix.target }} -p serena
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: serena-${{ matrix.target }}
          path: |
            target/${{ matrix.target }}/release/serena
            target/${{ matrix.target }}/release/serena.exe

# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  create-release:
    name: Create Release
    runs-on: ubuntu-latest
    outputs:
      upload_url: ${{ steps.create_release.outputs.upload_url }}
    steps:
      - id: create_release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          draft: false
          prerelease: false

  build-release:
    name: Build Release
    needs: create-release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            asset_name: serena-linux-x64.tar.gz
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            asset_name: serena-windows-x64.zip
          - os: macos-latest
            target: x86_64-apple-darwin
            asset_name: serena-macos-x64.tar.gz
          - os: macos-latest
            target: aarch64-apple-darwin
            asset_name: serena-macos-arm64.tar.gz
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2

      - name: Build
        run: cargo build --release --target ${{ matrix.target }} -p serena

      - name: Package (Linux/macOS)
        if: matrix.os != 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../${{ matrix.asset_name }} serena
          cd ../../..
          sha256sum ${{ matrix.asset_name }} > ${{ matrix.asset_name }}.sha256

      - name: Package (Windows)
        if: matrix.os == 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          7z a ../../../${{ matrix.asset_name }} serena.exe
          cd ../../..
          certutil -hashfile ${{ matrix.asset_name }} SHA256 > ${{ matrix.asset_name }}.sha256

      - name: Upload Release Asset
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ needs.create-release.outputs.upload_url }}
          asset_path: ./${{ matrix.asset_name }}
          asset_name: ${{ matrix.asset_name }}
          asset_content_type: application/octet-stream

      - name: Upload SHA256
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ needs.create-release.outputs.upload_url }}
          asset_path: ./${{ matrix.asset_name }}.sha256
          asset_name: ${{ matrix.asset_name }}.sha256
          asset_content_type: text/plain

  publish-crates-io:
    name: Publish to crates.io
    needs: build-release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo publish --token ${{ secrets.CARGO_TOKEN }}
```

## Justfile (Alternative to cargo-make)

```justfile
# Default recipe to display help
default:
    @just --list

# Development build
dev:
    cargo build --workspace

# Run tests
test:
    cargo test --workspace

# Fast tests (skip LSP downloads)
test-fast:
    SERENA_SKIP_LSP_DOWNLOADS=1 cargo test --workspace

# Format code
fmt:
    cargo fmt --all

# Run clippy
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run all checks
check: fmt clippy test

# Build release
release:
    cargo build --workspace --release

# Build optimized release
release-optimized:
    cargo build --workspace --profile release-optimized

# Cross-compile for all platforms
cross-all: cross-linux cross-windows cross-macos-x64 cross-macos-arm

# Cross-compile for Linux
cross-linux:
    cross build --target x86_64-unknown-linux-gnu --release -p serena

# Cross-compile for Windows
cross-windows:
    cross build --target x86_64-pc-windows-msvc --release -p serena

# Cross-compile for macOS x64
cross-macos-x64:
    cross build --target x86_64-apple-darwin --release -p serena

# Cross-compile for macOS ARM
cross-macos-arm:
    cross build --target aarch64-apple-darwin --release -p serena

# Package all platforms
package: package-linux package-windows package-macos

# Package Linux
package-linux: cross-linux
    #!/usr/bin/env bash
    VERSION=$(cargo metadata --format-version=1 | jq -r '.packages[] | select(.name == "serena") | .version')
    mkdir -p dist/serena-${VERSION}-linux-x64
    cp target/x86_64-unknown-linux-gnu/release/serena dist/serena-${VERSION}-linux-x64/
    cd dist && tar czf serena-${VERSION}-linux-x64.tar.gz serena-${VERSION}-linux-x64/
    sha256sum serena-${VERSION}-linux-x64.tar.gz > serena-${VERSION}-linux-x64.tar.gz.sha256

# Package Windows
package-windows: cross-windows
    #!/usr/bin/env bash
    VERSION=$(cargo metadata --format-version=1 | jq -r '.packages[] | select(.name == "serena") | .version')
    mkdir -p dist/serena-${VERSION}-windows-x64
    cp target/x86_64-pc-windows-msvc/release/serena.exe dist/serena-${VERSION}-windows-x64/
    cd dist && zip -r serena-${VERSION}-windows-x64.zip serena-${VERSION}-windows-x64/

# Package macOS
package-macos: cross-macos-x64 cross-macos-arm
    #!/usr/bin/env bash
    VERSION=$(cargo metadata --format-version=1 | jq -r '.packages[] | select(.name == "serena") | .version')
    # x64
    mkdir -p dist/serena-${VERSION}-macos-x64
    cp target/x86_64-apple-darwin/release/serena dist/serena-${VERSION}-macos-x64/
    cd dist && tar czf serena-${VERSION}-macos-x64.tar.gz serena-${VERSION}-macos-x64/
    # ARM
    mkdir -p dist/serena-${VERSION}-macos-arm64
    cp target/aarch64-apple-darwin/release/serena dist/serena-${VERSION}-macos-arm64/
    cd dist && tar czf serena-${VERSION}-macos-arm64.tar.gz serena-${VERSION}-macos-arm64/

# Run benchmarks
bench:
    cargo bench --workspace

# Generate documentation
doc:
    cargo doc --workspace --no-deps --open

# Clean build artifacts
clean:
    cargo clean
    rm -rf dist/

# Update dependencies
update:
    cargo update

# Audit dependencies
audit:
    cargo audit

# Check binary size
size:
    cargo build --release -p serena
    ls -lh target/release/serena
    strip target/release/serena
    ls -lh target/release/serena
```

These build configurations provide comprehensive automation for development, testing, cross-compilation, and distribution of the Rust implementation of Serena.
