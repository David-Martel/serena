<#
.SYNOPSIS
    Removes Windows reserved 'nul' device files from the repository.

.DESCRIPTION
    On Windows, 'nul' is a reserved device name (like /dev/null on Unix).
    When code accidentally creates files named 'nul', they become problematic
    because git cannot track them and they cause errors during 'git add'.

    This script finds and removes all 'nul' files recursively.

.PARAMETER Path
    The root path to search. Defaults to the repository root.

.PARAMETER DryRun
    If specified, only shows what would be deleted without actually deleting.

.PARAMETER Quiet
    Suppress output except for errors.

.EXAMPLE
    .\clean-nul.ps1
    Removes all nul files from the repository.

.EXAMPLE
    .\clean-nul.ps1 -DryRun
    Shows what nul files would be removed without deleting them.
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Path = (git rev-parse --show-toplevel 2>$null),

    [switch]$DryRun,

    [switch]$Quiet
)

$ErrorActionPreference = 'Stop'

# Fallback if not in a git repo
if (-not $Path) {
    $Path = $PSScriptRoot | Split-Path -Parent
}

function Write-Status {
    param([string]$Message, [string]$Color = 'White')
    if (-not $Quiet) {
        Write-Host $Message -ForegroundColor $Color
    }
}

# Find all 'nul' files (case-insensitive on Windows)
$nulFiles = @()

try {
    # Use Get-ChildItem with -Force to find hidden files too
    # Filter for exact name 'nul' (no extension)
    $nulFiles = Get-ChildItem -Path $Path -Recurse -Force -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -eq 'nul' }
} catch {
    Write-Warning "Error scanning directory: $_"
}

if ($nulFiles.Count -eq 0) {
    Write-Status "No 'nul' files found." -Color Green
    exit 0
}

Write-Status "Found $($nulFiles.Count) 'nul' file(s):" -Color Yellow

foreach ($file in $nulFiles) {
    $relativePath = $file.FullName.Substring($Path.Length + 1)

    if ($DryRun) {
        Write-Status "  [DRY RUN] Would delete: $relativePath" -Color Cyan
    } else {
        $deleted = $false

        # Method 1: Try .NET method
        try {
            [System.IO.File]::Delete($file.FullName)
            $deleted = $true
        } catch {}

        # Method 2: Try UNC path prefix (bypasses reserved name restrictions)
        if (-not $deleted) {
            try {
                $uncPath = "\\?\$($file.FullName)"
                [System.IO.File]::Delete($uncPath)
                $deleted = $true
            } catch {}
        }

        # Method 3: Try cmd.exe with UNC path
        if (-not $deleted) {
            try {
                $uncPath = "\\?\$($file.FullName)"
                $null = cmd /c "del /f /q `"$uncPath`"" 2>&1
                if (-not (Test-Path $file.FullName -ErrorAction SilentlyContinue)) {
                    $deleted = $true
                }
            } catch {}
        }

        # Method 4: Try renaming first, then deleting
        if (-not $deleted) {
            try {
                $tempName = [System.IO.Path]::Combine($file.DirectoryName, [System.Guid]::NewGuid().ToString())
                [System.IO.File]::Move("\\?\$($file.FullName)", $tempName)
                [System.IO.File]::Delete($tempName)
                $deleted = $true
            } catch {}
        }

        if ($deleted) {
            Write-Status "  Deleted: $relativePath" -Color Green
        } else {
            Write-Warning "  Could not delete: $relativePath (may need manual removal or WSL)"
            Write-Status "    Hint: In WSL, run: rm '$relativePath'" -Color Gray
        }
    }
}

if ($DryRun) {
    Write-Status "`nRun without -DryRun to delete these files." -Color Yellow
} else {
    Write-Status "`nCleanup complete." -Color Green
}
