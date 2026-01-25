<#
.SYNOPSIS
    Installs custom git hooks for the repository.

.DESCRIPTION
    Configures git to use the .githooks directory for hooks instead of .git/hooks.
    This allows hooks to be version-controlled and shared with the team.

    Hooks installed:
    - pre-commit: Removes 'nul' files before commits
    - post-checkout: Removes 'nul' files after branch switches
    - post-merge: Removes 'nul' files after merges

.PARAMETER Uninstall
    Reverts git to use the default .git/hooks directory.

.EXAMPLE
    .\install-hooks.ps1
    Installs the custom git hooks.

.EXAMPLE
    .\install-hooks.ps1 -Uninstall
    Uninstalls the custom git hooks.
#>

[CmdletBinding()]
param(
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'

# Get repository root
$repoRoot = git rev-parse --show-toplevel 2>$null
if (-not $repoRoot) {
    Write-Error "Not in a git repository"
    exit 1
}

$hooksDir = Join-Path $repoRoot '.githooks'

if ($Uninstall) {
    Write-Host "Uninstalling custom git hooks..." -ForegroundColor Yellow

    # Reset to default hooks path
    git config --local --unset core.hooksPath 2>$null

    Write-Host "Git hooks reset to default (.git/hooks)" -ForegroundColor Green
    exit 0
}

# Verify hooks directory exists
if (-not (Test-Path $hooksDir)) {
    Write-Error "Hooks directory not found: $hooksDir"
    exit 1
}

Write-Host "Installing custom git hooks..." -ForegroundColor Cyan

# Configure git to use .githooks directory
git config --local core.hooksPath .githooks

# Make hooks executable (important for Unix/WSL)
$hooks = Get-ChildItem -Path $hooksDir -File
foreach ($hook in $hooks) {
    Write-Host "  Configured: $($hook.Name)" -ForegroundColor Green
}

Write-Host "`nGit hooks installed successfully!" -ForegroundColor Green
Write-Host "Hooks directory: .githooks/" -ForegroundColor Gray

# Run initial cleanup
Write-Host "`nRunning initial 'nul' file cleanup..." -ForegroundColor Cyan
$cleanScript = Join-Path $repoRoot 'scripts\clean-nul.ps1'
if (Test-Path $cleanScript) {
    & $cleanScript
}

Write-Host "`nSetup complete. Hooks will run automatically on:" -ForegroundColor Green
Write-Host "  - git commit (pre-commit)" -ForegroundColor Gray
Write-Host "  - git checkout (post-checkout)" -ForegroundColor Gray
Write-Host "  - git merge/pull (post-merge)" -ForegroundColor Gray
