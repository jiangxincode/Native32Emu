# Smoke test script for Native32Emu
# Runs each game for N frames, takes a screenshot, and validates the result.
# Usage: .\smoke_test.ps1 [-Frames 150] [-BuildFirst] [-OutputDir smoke_results]

param(
    [int]$Frames = 150,
    [switch]$BuildFirst,
    [string]$OutputDir = "smoke_results"
)

$ErrorActionPreference = "Continue"
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Definition
$gameRoot = Join-Path $projectRoot "native32_game"
$screenshotDir = Join-Path (Join-Path $projectRoot $OutputDir) "screenshots"
$reportPath = Join-Path (Join-Path $projectRoot $OutputDir) "report.txt"

# Create output directories
if (-not (Test-Path $screenshotDir)) {
    New-Item -ItemType Directory -Force -Path $screenshotDir | Out-Null
}

# Build first if requested
if ($BuildFirst) {
    Write-Host "Building emulator..." -ForegroundColor Cyan
    & cargo build --release 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "BUILD FAILED" -ForegroundColor Red
        exit 1
    }
    Write-Host "Build OK" -ForegroundColor Green
}

# Locate the release binary
$binary = Join-Path $projectRoot "target\release\native32-emu.exe"
if (-not (Test-Path $binary)) {
    Write-Host "Binary not found at $binary - building..." -ForegroundColor Yellow
    & cargo build --release 2>&1 | Out-Null
    if (-not (Test-Path $binary)) {
        Write-Host "BUILD FAILED" -ForegroundColor Red
        exit 1
    }
}

# Find all Native32 game files (.smf and .ssl, exclude NESGAME)
$smfGames = Get-ChildItem -Path $gameRoot -Filter "*.smf" -Recurse -ErrorAction SilentlyContinue | Where-Object { $_.FullName -notmatch "NESGAME" }
$sslGames = Get-ChildItem -Path $gameRoot -Filter "*.ssl" -Recurse -ErrorAction SilentlyContinue | Where-Object { $_.FullName -notmatch "NESGAME" -and $_.FullName -notmatch "\.ssl_sav$" }
$games = @($smfGames) + @($sslGames)
$games = $games | Sort-Object FullName

$total = $games.Count
$passed = 0
$failed = 0
$warned = 0
$results = @()

Write-Host ""
Write-Host "=== Native32Emu Smoke Test ===" -ForegroundColor Cyan
Write-Host "Games found: $total"
Write-Host "Frames per game: $Frames"
Write-Host "Output: $screenshotDir"
Write-Host ""

$index = 0
foreach ($game in $games) {
    $index++
    $gameRel = $game.FullName.Substring($gameRoot.Length + 1)
    # Create a safe filename from the relative path
    $safeName = ($gameRel -replace '[\\\/\.\s]', '_') + ".png"
    $screenshotFile = Join-Path $screenshotDir $safeName

    # Remove old screenshot if exists
    if (Test-Path $screenshotFile) { Remove-Item $screenshotFile -Force }

    Write-Host "[$index/$total] $gameRel ... " -NoNewline

    # Run emulator with screenshot
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $stderrFile = Join-Path $env:TEMP "n32smoke_stderr.txt"
    $stdoutFile = Join-Path $env:TEMP "n32smoke_stdout.txt"

    try {
        $process = Start-Process -FilePath $binary `
            -ArgumentList "`"$($game.FullName)`"", "--screenshot", "`"$screenshotFile`"", "--screenshot-frames", "$Frames" `
            -NoNewWindow -Wait -PassThru `
            -RedirectStandardError $stderrFile `
            -RedirectStandardOutput $stdoutFile
        $exitCode = $process.ExitCode
    }
    catch {
        $exitCode = -1
        $stderr = $_.Exception.Message
    }

    $sw.Stop()
    $elapsed = $sw.ElapsedMilliseconds

    if (Test-Path $stderrFile) {
        $stderr = Get-Content $stderrFile -Raw -ErrorAction SilentlyContinue
    }

    # Validate result
    $status = "PASS"
    $reason = ""

    if ($exitCode -ne 0) {
        $status = "FAIL"
        $reason = "exit code $exitCode"
        if ($stderr -match "panicked") {
            $reason += " (panic detected)"
        }
    }
    elseif (-not (Test-Path $screenshotFile)) {
        $status = "FAIL"
        $reason = "no screenshot generated"
    }
    else {
        $fileSize = (Get-Item $screenshotFile).Length
        if ($fileSize -lt 100) {
            $status = "FAIL"
            $reason = "screenshot too small ($fileSize bytes)"
        }
        else {
            # Check if image is all-black or all-white
            try {
                Add-Type -AssemblyName System.Drawing -ErrorAction Stop
                $bmp = [System.Drawing.Bitmap]::FromFile($screenshotFile)
                $w = [Math]::Max(1, $bmp.Width)
                $h = [Math]::Max(1, $bmp.Height)

                # Sample pixels at key positions
                $samples = @()
                $positions = @(
                    [System.Drawing.Point]::new(0, 0),
                    [System.Drawing.Point]::new([int]($w/2), [int]($h/2)),
                    [System.Drawing.Point]::new($w-1, $h-1),
                    [System.Drawing.Point]::new([int]($w/4), [int]($h/4)),
                    [System.Drawing.Point]::new([int](3*$w/4), [int](3*$h/4))
                )
                foreach ($p in $positions) {
                    $px = $bmp.GetPixel($p.X, $p.Y)
                    $samples += "$($px.R),$($px.G),$($px.B)"
                }
                $bmp.Dispose()

                # Check if all sampled pixels are identical
                $unique = $samples | Select-Object -Unique
                if ($unique.Count -eq 1) {
                    $firstColor = $unique[0]
                    if ($firstColor -eq "0,0,0") {
                        $status = "WARN"
                        $reason = "screenshot appears all-black (${w}x${h})"
                    }
                    elseif ($firstColor -eq "255,255,255") {
                        $status = "WARN"
                        $reason = "screenshot appears all-white (${w}x${h})"
                    }
                }
            }
            catch {
                # If System.Drawing not available, skip pixel check
            }
        }
    }

    # Report result
    $color = switch ($status) {
        "PASS" { "Green" }
        "FAIL" { "Red" }
        "WARN" { "Yellow" }
    }
    $msg = "$status (${elapsed}ms)"
    if ($reason) { $msg += " - $reason" }
    Write-Host $msg -ForegroundColor $color

    if ($status -eq "PASS") { $passed++ }
    elseif ($status -eq "WARN") { $warned++ }
    else { $failed++ }

    $results += [PSCustomObject]@{
        Game   = $gameRel
        Status = $status
        Time   = "${elapsed}ms"
        Reason = $reason
    }
}

# Summary
Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "Total: $total | " -NoNewline
Write-Host "Passed: $passed" -ForegroundColor Green -NoNewline
Write-Host " | " -NoNewline
if ($warned -gt 0) {
    Write-Host "Warned: $warned" -ForegroundColor Yellow -NoNewline
    Write-Host " | " -NoNewline
}
Write-Host "Failed: $failed" -ForegroundColor $(if ($failed -gt 0) { "Red" } else { "Green" })

# Write report file
$reportContent = "Native32Emu Smoke Test Report`n"
$reportContent += "Date: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')`n"
$reportContent += "Frames: $Frames`n"
$reportContent += "Total: $total | Passed: $passed | Warned: $warned | Failed: $failed`n"
$reportContent += ("=" * 80) + "`n`n"

foreach ($r in $results) {
    $line = "[$($r.Status.PadRight(4))] $($r.Game.PadRight(40)) $($r.Time)"
    if ($r.Reason) { $line += "  # $($r.Reason)" }
    $reportContent += "$line`n"
}

$reportContent | Out-File -FilePath $reportPath -Encoding utf8
Write-Host ""
Write-Host "Report saved to: $reportPath" -ForegroundColor Cyan
Write-Host "Screenshots saved to: $screenshotDir" -ForegroundColor Cyan

# Exit with failure code if any game failed
if ($failed -gt 0) { exit 1 }
