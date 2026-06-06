#!/usr/bin/env pwsh
# Build the Native32Emu libretro core and stage it for RetroArch.
#
# RetroArch requires the core file name to end in `_libretro` and the matching
# info file to share the same base name. Cargo names the cdylib after the lib
# target (`native32emu`), so this script renames the artifact to
# `native32emu_libretro.<ext>` and copies the info file alongside it into dist/.
#
# Usage:
#   ./scripts/build-libretro.ps1                 # release build into ./dist
#   ./scripts/build-libretro.ps1 -DebugBuild     # debug build

[CmdletBinding()]
param(
    [switch]$DebugBuild,
    [string]$OutDir = "dist"
)

$ErrorActionPreference = "Stop"

# Resolve repo root (parent of this script's directory)
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

$BuildProfile = if ($DebugBuild) { "debug" } else { "release" }
$CoreName = "native32emu_libretro"

Write-Host "Building libretro core ($BuildProfile)..."
$buildArgs = @("rustc", "--lib", "--features", "libretro", "--no-default-features", "--crate-type", "cdylib")
if (-not $DebugBuild) { $buildArgs += "--release" }
& cargo @buildArgs
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# Locate the produced dynamic library across platforms
$artifactDir = Join-Path "target" $BuildProfile
$candidates = @(
    "native32emu.dll",        # Windows
    "libnative32emu.so",      # Linux
    "libnative32emu.dylib"    # macOS
)
$src = $null
foreach ($c in $candidates) {
    $p = Join-Path $artifactDir $c
    if (Test-Path $p) { $src = $p; break }
}
if ($null -eq $src) { throw "Could not find built dynamic library in $artifactDir" }

# Target file name keeps the platform extension but uses the _libretro base name
$ext = [System.IO.Path]::GetExtension($src)
$destName = "$CoreName$ext"

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Copy-Item -Force $src (Join-Path $OutDir $destName)
Copy-Item -Force "native32emu_libretro.info" (Join-Path $OutDir "native32emu_libretro.info")

Write-Host "Staged libretro core into $OutDir/:"
Write-Host "  - $destName"
Write-Host "  - native32emu_libretro.info"
Write-Host ""
Write-Host "Install: copy '$destName' to RetroArch's cores/ directory and"
Write-Host "'native32emu_libretro.info' to RetroArch's info/ directory."
