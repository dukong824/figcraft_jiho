param(
    [Parameter(Mandatory)][ValidatePattern('^(jiho|preview)-v[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$')][string]$Tag,
    [string]$BinaryDirectory,
    [string]$BuildProfile = 'release'
)
$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$dist = Join-Path $repoRoot 'dist/jiho'
$stage = Join-Path $dist $Tag
if (Test-Path -LiteralPath $stage) { throw "Package staging folder already exists: $stage" }
if (-not $BinaryDirectory) {
    Push-Location $repoRoot
    try {
        cargo build --release --locked --target x86_64-pc-windows-msvc -p figcraft -p figcraft-cli
        if ($LASTEXITCODE -ne 0) { throw 'Application build failed' }
        cargo build --release --locked --target x86_64-pc-windows-msvc -p figcraft-render --example paper_metal_gif
        if ($LASTEXITCODE -ne 0) { throw 'GIF exporter build failed' }
    } finally { Pop-Location }
    $BinaryDirectory = Join-Path $repoRoot 'target/x86_64-pc-windows-msvc/release'
}
$BinaryDirectory = (Resolve-Path -LiteralPath $BinaryDirectory).Path
$files = @{
    'figcraft.exe' = 'figcraft_jiho.exe'
    'figcraft-cli.exe' = 'figcraft-cli.exe'
    'examples/paper_metal_gif.exe' = 'Paper-to-GIF.exe'
}
foreach ($source in $files.Keys) {
    if (-not (Test-Path -LiteralPath (Join-Path $BinaryDirectory $source))) { throw "Missing binary: $source" }
    $bytes = [IO.File]::ReadAllBytes((Join-Path $BinaryDirectory $source))
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    if ([BitConverter]::ToUInt16($bytes, $pe + 4) -ne 0x8664) { throw "Not an x64 executable: $source" }
    $expectedSubsystem = if ($source -eq 'figcraft.exe') { 2 } else { 3 }
    if ([BitConverter]::ToUInt16($bytes, $pe + 0x5C) -ne $expectedSubsystem) { throw "Unexpected executable subsystem: $source" }
}
New-Item -ItemType Directory -Path $stage -Force | Out-Null
foreach ($source in $files.Keys) {
    Copy-Item -LiteralPath (Join-Path $BinaryDirectory $source) -Destination (Join-Path $stage $files[$source])
}
Copy-Item -LiteralPath (Join-Path $repoRoot 'examples/jiho/Demo.figcraft') -Destination $stage
Copy-Item -LiteralPath (Join-Path $repoRoot 'VALIDATION.md') -Destination $stage
Copy-Item -LiteralPath (Join-Path $repoRoot 'docs/paper-liquid-metal.md') -Destination (Join-Path $stage 'PAPER-LIQUID-METAL.md')
$licenses = Join-Path $stage 'licenses'
New-Item -ItemType Directory -Path $licenses | Out-Null
foreach ($file in @('LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE')) {
    Copy-Item -LiteralPath (Join-Path $repoRoot $file) -Destination $licenses
}
foreach ($file in @('LICENSE', 'NOTICE', 'UPSTREAM.md')) {
    Copy-Item -LiteralPath (Join-Path $repoRoot "crates/render/src/paper-shaders/$file") -Destination (Join-Path $licenses "Paper-$file.txt")
}
if ($env:CRAFT_FONTS_DIR) {
    $fontRoot = Join-Path $env:CRAFT_FONTS_DIR 'fonts'
    foreach ($family in Get-ChildItem -LiteralPath $fontRoot -Directory -ErrorAction SilentlyContinue) {
        $ofl = Join-Path $family.FullName 'OFL.txt'
        if (Test-Path -LiteralPath $ofl) { Copy-Item -LiteralPath $ofl -Destination (Join-Path $licenses "OFL-$($family.Name).txt") }
    }
}
@'
@echo off
start "" "%~dp0figcraft_jiho.exe" "%~dp0Demo.figcraft"
'@ | Set-Content -LiteralPath (Join-Path $stage 'Open Demo.cmd') -Encoding ascii
@'
@echo off
if "%~1"=="" (
  echo Drop an image onto this script to make a 3-second Liquid Metal GIF.
  pause
  exit /b 1
)
"%~dp0Paper-to-GIF.exe" "%~1" "%~dpn1-liquid-metal.gif"
pause
'@ | Set-Content -LiteralPath (Join-Path $stage 'Image to GIF.cmd') -Encoding ascii
@'
figcraft_jiho — personal FigCraft build

Run figcraft_jiho.exe or Open Demo.cmd.
Select an image, add an Effect, choose Paper Liquid Metal, and open its settings.
Transparent PNGs and vectors expose their own silhouette; opaque photographs become rectangular metal.
Drag an image onto Image to GIF.cmd to export a 3-second, 24fps GIF.

Updates: https://github.com/dukong824/figcraft_jiho/releases
Close the application, extract the new portable package, and replace the files at the same location.
The application does not install official upstream binaries or automatically replace itself.
Use Windows Open With / Always to associate .figcraft files with this executable.

Native effect only; live rendering depends on hardware. Read VALIDATION.md for actual checks and limits.
Powered by Paper Shaders. License and attribution files are included.
'@ | Set-Content -LiteralPath (Join-Path $stage 'README.txt') -Encoding utf8
$revision = (git -C $repoRoot rev-parse HEAD).Trim()
"Repository: https://github.com/dukong824/figcraft_jiho`nTag: $Tag`nCommit: $revision`nProfile: $BuildProfile`n" |
    Set-Content -LiteralPath (Join-Path $stage 'BUILD-INFO.txt') -Encoding utf8
$version = $Tag -replace '^(jiho|preview)-v', ''
$archive = Join-Path $dist "figcraft_jiho-$version-windows-x64-portable.zip"
if (Test-Path -LiteralPath $archive) { throw "Archive already exists: $archive" }
Compress-Archive -LiteralPath $stage -DestinationPath $archive -CompressionLevel Optimal
$digest = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$digest  $([IO.Path]::GetFileName($archive))" | Set-Content -LiteralPath (Join-Path $dist 'SHA256SUMS.txt') -Encoding ascii
Write-Output $archive
