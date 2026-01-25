#!/bin/bash
# Serena Dependency Health - Immediate Fixes
# Generated: 2025-12-25
# Run this script to apply immediate dependency improvements

set -e

echo "==================================="
echo "Serena Dependency Health Fixes"
echo "==================================="
echo ""

# Navigate to project root
cd "$(dirname "$0")/.." || exit 1

echo "Step 1: Updating cargo-audit to support CVSS 4.0..."
cargo install cargo-audit --force
echo "✓ cargo-audit updated"
echo ""

echo "Step 2: Running security audit..."
if cargo audit; then
    echo "✓ No security vulnerabilities found"
else
    echo "⚠ Security vulnerabilities detected - review output above"
fi
echo ""

echo "Step 3: Updating patch versions (rustix, tempfile)..."
cargo update rustix tempfile
echo "✓ Patch versions updated"
echo ""

echo "Step 4: Checking for duplicates after update..."
echo "Duplicate dependencies:"
cargo tree --duplicates --workspace | head -50
echo ""

echo "Step 5: Testing build after updates..."
if cargo check --workspace; then
    echo "✓ Workspace builds successfully"
else
    echo "✗ Build failed - review errors above"
    exit 1
fi
echo ""

echo "==================================="
echo "Immediate fixes completed!"
echo "==================================="
echo ""
echo "Next steps (manual):"
echo "1. Review DEPENDENCY_HEALTH_REPORT.md"
echo "2. Move dashboard profile to workspace root"
echo "3. Plan Leptos 0.8 migration"
echo "4. Replace serde_yaml with serde_yml"
echo ""
echo "Run 'cargo test --workspace' to verify all tests pass."
