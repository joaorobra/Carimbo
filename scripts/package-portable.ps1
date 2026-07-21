# Produces the portable Carimbo distribution: the standalone exe plus a
# `portable` marker file, zipped. Run after `cargo tauri build`.
#
# The marker file tells Carimbo to store its data next to the exe (see
# src-tauri/src/paths.rs) so it can run from a USB stick or locked-down machine
# without writing to %APPDATA%.

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$releaseDir = Join-Path $root "src-tauri\target\release"
$exe = Join-Path $releaseDir "carimbo.exe"

if (-not (Test-Path $exe)) {
  Write-Error "carimbo.exe not found at $exe. Run 'cargo tauri build' first."
}

# Read version from tauri.conf.json for the zip name.
$conf = Get-Content (Join-Path $root "src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json
$version = $conf.version

$stageDir = Join-Path $releaseDir "carimbo-portable"
$zipPath = Join-Path $releaseDir "Carimbo_${version}_portable_x64.zip"

# Stage: exe + marker.
if (Test-Path $stageDir) { Remove-Item $stageDir -Recurse -Force }
New-Item -ItemType Directory -Path $stageDir | Out-Null
Copy-Item $exe (Join-Path $stageDir "carimbo.exe")
# The marker (empty file) switches the app into portable mode.
New-Item -ItemType File -Path (Join-Path $stageDir "portable") | Out-Null
# A short readme.
@"
Carimbo (portable) $version

Run carimbo.exe directly — no installation needed. All data (snippets,
clipboard history, settings) is stored in the 'data' folder next to the exe,
so you can carry it on a USB drive.

Requires the Microsoft Edge WebView2 runtime, which is preinstalled on
Windows 11 and most updated Windows 10 systems. If Carimbo reports it's
missing, install it from https://developer.microsoft.com/microsoft-edge/webview2/
"@ | Set-Content (Join-Path $stageDir "LEIA-ME.txt") -Encoding UTF8

if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path (Join-Path $stageDir "*") -DestinationPath $zipPath

Write-Output "Portable package created: $zipPath"
