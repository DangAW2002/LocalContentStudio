<#
.SYNOPSIS
    Builds Local Content Studio Windows Release Installer (NSIS).
.DESCRIPTION
    Runs design-system build, desktop-ui build, and Tauri release build.
    Ensures release mode packages frontend assets from frontendDist without dev server.
    Does not code sign; does not bundle Node, FFmpeg, or Chrome.
    Copies installer and generates SHA256SUMS.txt into .local/release/.
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

# Always verify frontend UI build exists before starting cargo / tauri release build
$UiIndex = Join-Path $RepoRoot "packages\desktop-ui\dist\index.html"
if (-not (Test-Path $UiIndex)) {
    throw "Frontend UI build did not produce $UiIndex. Cannot proceed with cargo/tauri build."
}

Write-Host "=== 3. Building Tauri Windows Release Bundle ===" -ForegroundColor Cyan
$TauriCli = Join-Path $RepoRoot "packages\desktop-ui\node_modules\@tauri-apps\cli\tauri.js"
if (Test-Path $TauriCli) {
    node $TauriCli build --bundles nsis --no-sign
} else {
    pnpm --filter @studio/desktop-ui exec tauri build --bundles nsis --no-sign
}
if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE" }

Write-Host "=== 4. Locating NSIS Installer ===" -ForegroundColor Cyan
$NsisDir = if ($env:CARGO_TARGET_DIR) { Join-Path $env:CARGO_TARGET_DIR "release\bundle\nsis" } else { Join-Path $RepoRoot "target\release\bundle\nsis" }
if (-not (Test-Path $NsisDir)) {
    $NsisDir = Join-Path $RepoRoot "target\release\bundle\nsis"
}
if (-not (Test-Path $NsisDir)) {
    throw "NSIS output directory not found at $NsisDir"
}

$Installers = Get-ChildItem -Path $NsisDir -Filter "*.exe" | Sort-Object LastWriteTime -Descending
if ($Installers.Count -eq 0) {
    throw "No NSIS installer .exe found in $NsisDir"
}

$TargetInstaller = $Installers[0]
$SizeMb = [math]::Round($TargetInstaller.Length / 1MB, 2)

Write-Host "=== 5. Copying to .local/release and Computing SHA-256 ===" -ForegroundColor Cyan
$LocalReleaseDir = Join-Path $RepoRoot ".local\release"
if (-not (Test-Path $LocalReleaseDir)) {
    New-Item -ItemType Directory -Force -Path $LocalReleaseDir | Out-Null
}

$DestInstaller = Join-Path $LocalReleaseDir $TargetInstaller.Name
Copy-Item -Path $TargetInstaller.FullName -Destination $DestInstaller -Force

$Hash = (Get-FileHash -Algorithm SHA256 -Path $DestInstaller).Hash.ToLower()
$ShaFile = Join-Path $LocalReleaseDir "SHA256SUMS.txt"
Set-Content -Path $ShaFile -Value "$Hash *$($TargetInstaller.Name)" -Encoding utf8

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Green
Write-Host " Local Content Studio NSIS Installer Built Successfully! " -ForegroundColor Green
Write-Host " Path: $DestInstaller" -ForegroundColor Green
Write-Host " Size: $SizeMb MB ($($TargetInstaller.Length) bytes)" -ForegroundColor Green
Write-Host " SHA-256: $Hash" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green

[PSCustomObject]@{
    Name = $TargetInstaller.Name
    Path = $DestInstaller
    SizeBytes = $TargetInstaller.Length
    SizeMB = $SizeMb
    SHA256 = $Hash
}
