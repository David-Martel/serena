@echo off
REM Serena Dependency Health - Immediate Fixes (Windows)
REM Generated: 2025-12-25
REM Run this script to apply immediate dependency improvements

echo ===================================
echo Serena Dependency Health Fixes
echo ===================================
echo.

cd /d "%~dp0\.."

echo Step 1: Updating cargo-audit to support CVSS 4.0...
cargo install cargo-audit --force
if %errorlevel% neq 0 (
    echo X Failed to update cargo-audit
    exit /b 1
)
echo √ cargo-audit updated
echo.

echo Step 2: Running security audit...
cargo audit
if %errorlevel% neq 0 (
    echo ! Security vulnerabilities detected - review output above
) else (
    echo √ No security vulnerabilities found
)
echo.

echo Step 3: Updating patch versions (rustix, tempfile)...
cargo update rustix tempfile
if %errorlevel% neq 0 (
    echo X Failed to update dependencies
    exit /b 1
)
echo √ Patch versions updated
echo.

echo Step 4: Testing build after updates...
cargo check --workspace
if %errorlevel% neq 0 (
    echo X Build failed - review errors above
    exit /b 1
)
echo √ Workspace builds successfully
echo.

echo ===================================
echo Immediate fixes completed!
echo ===================================
echo.
echo Next steps (manual):
echo 1. Review DEPENDENCY_HEALTH_REPORT.md
echo 2. Move dashboard profile to workspace root
echo 3. Plan Leptos 0.8 migration
echo 4. Replace serde_yaml with serde_yml
echo.
echo Run 'cargo test --workspace' to verify all tests pass.
pause
