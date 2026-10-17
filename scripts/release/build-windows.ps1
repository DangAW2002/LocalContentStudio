<#
.SYNOPSIS
    Builds Local Content Studio Windows Release Installer (NSIS).
.DESCRIPTION
    Runs design-system build, desktop-ui build, and Tauri release build.
    Ensures release mode packages frontend assets from frontendDist without dev server.
    Does not code sign; does not bundle Node, FFmpeg, or Chrome.
#>

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $RepoRoot

Write-Host "=== 1. Building @studio/design-system ===" -ForegroundColor Cyan
pnpm --filter @studio/design-system build
if ($LASTEXITCODE -ne 0) { throw "Build design-system failed with exit code $LASTEXITCODE" }

Write-Host "=== 2. Building @studio/desktop-ui ===" -ForegroundColor Cyan
pnpm --filter @studio/desktop-ui build
if ($LASTEXITCODE -ne 0) { throw "Build desktop-ui failed with exit code $LASTEXITCODE" }

Write-Host "=== 3. Building Tauri Windows Release Bundle ===" -ForegroundColor Cyan
$TauriCli = Join-Path $RepoRoot "packages\desktop-ui\node_modules\@tauri-apps\cli\tauri.js"
if (Test-Path $TauriCli) {
    node $TauriCli build --bundles nsis --no-sign
} else {
    pnpm --filter @studio/desktop-ui exec tauri build --bundles nsis --no-sign
}
if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE" }

Write-Host "=== 4. Locating NSIS Installer ===" -ForegroundColor Cyan
$NsisDir = Join-Path $RepoRoot "target\release\bundle\nsis"
if (-not (Test-Path $NsisDir)) {
    throw "NSIS output directory not found at $NsisDir"
}

$Installers = Get-ChildItem -Path $NsisDir -Filter "*.exe" | Sort-Object LastWriteTime -Descending
if ($Installers.Count -eq 0) {
    throw "No NSIS installer .exe found in $NsisDir"
}

$TargetInstaller = $Installers[0]
$SizeMb = [math]::Round($TargetInstaller.Length / 1MB, 2)

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Green
Write-Host " Local Content Studio NSIS Installer Built Successfully! " -ForegroundColor Green
Write-Host " Path: $($TargetInstaller.FullName)" -ForegroundColor Green
Write-Host " Size: $SizeMb MB ($($TargetInstaller.Length) bytes)" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green

[PSCustomObject]@{
    Name = $TargetInstaller.Name
    Path = $TargetInstaller.FullName
    SizeBytes = $TargetInstaller.Length
    SizeMB = $SizeMb
}
