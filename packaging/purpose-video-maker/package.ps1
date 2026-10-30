param(
    [string]$BinaryDirectory,
    [string]$FfmpegArchive,
    [string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$version = '0.1.9'
$ffmpegName = 'ffmpeg-n8.1.3-24-g92611900bb-win64-lgpl-8.1.zip'
$ffmpegUrl = "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-10-27-18-38/$ffmpegName"
$ffmpegSha256 = '4b999c5b938ee20dc1bd7be68cb09381ac425520e363a58b67e787402cb96022'
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repoRoot 'dist/purpose-video-maker' }
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
$stage = Join-Path $OutputDirectory "Purpose-video-maker-$version-windows-x64"
if (Test-Path -LiteralPath $stage) { throw "Package staging folder already exists: $stage" }

if (-not $BinaryDirectory) {
    Push-Location $repoRoot
    try {
        cargo build --release --locked --target x86_64-pc-windows-msvc -p figcraft --bin purpose-video-maker --features purpose-branding
        if ($LASTEXITCODE -ne 0) { throw 'Windows application build failed' }
    } finally { Pop-Location }
    $BinaryDirectory = Join-Path $repoRoot 'target/x86_64-pc-windows-msvc/release'
}
$binary = Join-Path (Resolve-Path -LiteralPath $BinaryDirectory).Path 'purpose-video-maker.exe'
if (-not (Test-Path -LiteralPath $binary)) { throw "Missing application binary: $binary" }
$bytes = [IO.File]::ReadAllBytes($binary)
$pe = [BitConverter]::ToInt32($bytes, 0x3C)
if ([BitConverter]::ToUInt16($bytes, $pe + 4) -ne 0x8664) { throw 'Application binary is not Windows x64' }
if ([BitConverter]::ToUInt16($bytes, $pe + 0x5C) -ne 2) { throw 'Application binary is not a Windows GUI executable' }
& $binary --version
if ($LASTEXITCODE -ne 0) { throw 'Application version smoke test failed' }

$temporary = Join-Path $OutputDirectory '.ffmpeg-download'
New-Item -ItemType Directory -Path $temporary -Force | Out-Null
if (-not $FfmpegArchive) {
    $FfmpegArchive = Join-Path $temporary $ffmpegName
    Invoke-WebRequest -Uri $ffmpegUrl -OutFile $FfmpegArchive
}
$actualSha256 = (Get-FileHash -LiteralPath $FfmpegArchive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualSha256 -ne $ffmpegSha256) { throw "FFmpeg archive checksum mismatch: $actualSha256" }
$expanded = Join-Path $temporary 'expanded'
Expand-Archive -LiteralPath $FfmpegArchive -DestinationPath $expanded
$ffmpegBinary = Get-ChildItem -LiteralPath $expanded -Recurse -File -Filter ffmpeg.exe | Select-Object -First 1
if (-not $ffmpegBinary) { throw 'FFmpeg archive has no ffmpeg.exe' }
$ffmpegRoot = $ffmpegBinary.Directory.Parent.FullName
$ffmpegLicense = Join-Path $ffmpegRoot 'LICENSE.txt'
if (-not (Test-Path -LiteralPath $ffmpegLicense)) { throw 'FFmpeg archive has no license' }

New-Item -ItemType Directory -Path (Join-Path $stage 'ffmpeg/bin') -Force | Out-Null
Copy-Item -LiteralPath $binary -Destination $stage
Copy-Item -LiteralPath (Join-Path $repoRoot 'assets/purpose/purpose.ico') -Destination (Join-Path $stage "purpose-icon-$version.ico")
foreach ($name in @('ffmpeg.exe', 'ffprobe.exe', 'ffplay.exe')) {
    $source = Join-Path $ffmpegBinary.Directory.FullName $name
    if (-not (Test-Path -LiteralPath $source)) { throw "FFmpeg archive has no $name" }
    Copy-Item -LiteralPath $source -Destination (Join-Path $stage 'ffmpeg/bin')
}
Copy-Item -LiteralPath $ffmpegLicense -Destination (Join-Path $stage 'ffmpeg/LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE-MIT') -Destination (Join-Path $stage 'LICENSE-MIT.txt')
Copy-Item -LiteralPath (Join-Path $repoRoot 'LICENSE-APACHE') -Destination (Join-Path $stage 'LICENSE-APACHE.txt')
Copy-Item -LiteralPath (Join-Path $repoRoot 'NOTICE') -Destination (Join-Path $stage 'NOTICE.txt')
$filmcraftNotices = Join-Path $stage 'licenses/filmcraft'
New-Item -ItemType Directory -Path $filmcraftNotices -Force | Out-Null
foreach ($name in @('LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'UPSTREAM.md')) {
    Copy-Item -LiteralPath (Join-Path $repoRoot "third-party/filmcraft-ui/$name") -Destination $filmcraftNotices
}
Copy-Item -LiteralPath (Join-Path $repoRoot 'third-party/filmcraft-ui/licenses') -Destination $filmcraftNotices -Recurse
$iconNotices = Join-Path $stage 'licenses/lucide'
New-Item -ItemType Directory -Path $iconNotices -Force | Out-Null
foreach ($name in @('LICENSE.txt', 'SOURCE.json')) {
    Copy-Item -LiteralPath (Join-Path $repoRoot "assets/ui/lucide/$name") -Destination $iconNotices
}
$markdownNotices = Join-Path $stage 'licenses/pulldown-cmark'
New-Item -ItemType Directory -Path $markdownNotices -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $repoRoot 'third-party/pulldown-cmark/LICENSE') -Destination $markdownNotices
Copy-Item -LiteralPath (Join-Path $repoRoot 'third-party/pulldown-cmark/UPSTREAM.md') -Destination $markdownNotices
Copy-Item -LiteralPath (Join-Path $repoRoot 'docs/video-maker.md') -Destination (Join-Path $stage 'VIDEO-MAKER.md')
@'
Purpose video maker for Windows x64

The setup EXE installs to your user profile and creates a Start menu shortcut.
Your projects are saved in the FigCraft configuration folder, outside the install directory.

FFmpeg, ffprobe and ffplay are included in ffmpeg/bin. FFmpeg comes from the
BtbN/FFmpeg-Builds LGPL 3.0 build dated 2026-10-27. Its license is in
ffmpeg/LICENSE.txt. Source and build instructions:
https://github.com/BtbN/FFmpeg-Builds/tree/autobuild-2026-10-27-18-38
https://ffmpeg.org/download.html

OpenAI Codex CLI is included in codex/. Sign in with ChatGPT in the app sidebar.
See VIDEO-MAKER.md for features and current limits.
'@ | Set-Content -LiteralPath (Join-Path $stage 'README.txt') -Encoding utf8

$codexVersion = 'rust-v0.165.0'
$codexDirectory = Join-Path $stage 'codex'
New-Item -ItemType Directory -Path $codexDirectory -Force | Out-Null
$codexFiles = @(
    @{ Source = 'codex-x86_64-pc-windows-msvc.exe'; Name = 'codex.exe'; Sha256 = 'd4b2e6a054ea1dfe53217f9d8942c5f8a9f4b76171a1e0bb76ced5759915300a' }
    @{ Source = 'codex-command-runner-x86_64-pc-windows-msvc.exe'; Name = 'codex-command-runner.exe'; Sha256 = '8f1a9d4217a34d6b44df81c25999ff9b98bb8d0e3c179aa070fcd84b3ce7a9d3' }
    @{ Source = 'codex-windows-sandbox-setup-x86_64-pc-windows-msvc.exe'; Name = 'codex-windows-sandbox-setup.exe'; Sha256 = 'a39f42dc697919e650cd24f82daaecd6121e41e3f4d56373fef4f4177f8eee02' }
    @{ Source = 'codex-windows-sandbox-service-x86_64-pc-windows-msvc.exe'; Name = 'codex-windows-sandbox-service.exe'; Sha256 = '48b13eee521adada28083167f187b2df0fb6c2eab2f7e59ef36e0c12beae77b0' }
 )
foreach ($item in $codexFiles) {
    $destination = Join-Path $codexDirectory $item.Name
    Invoke-WebRequest -Uri "https://github.com/openai/codex/releases/download/$codexVersion/$($item.Source)" -OutFile $destination
    if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant() -ne $item.Sha256) { throw 'Codex checksum mismatch' }
}
Copy-Item -LiteralPath (Join-Path $repoRoot 'third-party/codex/LICENSE') -Destination (Join-Path $codexDirectory 'LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $repoRoot 'third-party/codex/NOTICE') -Destination (Join-Path $codexDirectory 'NOTICE.txt')
"OpenAI Codex $codexVersion; Apache-2.0; https://github.com/openai/codex/tree/$codexVersion" |
    Set-Content -LiteralPath (Join-Path $codexDirectory 'SOURCE.txt') -Encoding utf8
& (Join-Path $codexDirectory 'codex.exe') --version
if ($LASTEXITCODE -ne 0) { throw 'Bundled Codex failed' }

$revision = (git -C $repoRoot rev-parse HEAD).Trim()
"Commit: $revision`nVersion: $version`nFFmpeg: $ffmpegName`nFFmpeg SHA256: $ffmpegSha256`n" |
    Set-Content -LiteralPath (Join-Path $stage 'BUILD-INFO.txt') -Encoding ascii

$iscc = Get-Command ISCC.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1
if (-not $iscc) { $iscc = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe' }
if (-not (Test-Path -LiteralPath $iscc)) { throw 'Inno Setup 6 ISCC.exe is required' }
& $iscc "/DStageDir=$stage" "/DOutDir=$OutputDirectory" (Join-Path $PSScriptRoot 'installer.iss')
if ($LASTEXITCODE -ne 0) { throw 'Installer build failed' }
$setup = Join-Path $OutputDirectory "Purpose-video-maker-$version-windows-x64-setup.exe"
if (-not (Test-Path -LiteralPath $setup)) { throw 'Installer was not created' }
$sums = foreach ($file in @($setup)) {
    "$( (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() )  $([IO.Path]::GetFileName($file))"
}
$sums | Set-Content -LiteralPath (Join-Path $OutputDirectory 'SHA256SUMS.txt') -Encoding ascii
Remove-Item -LiteralPath $temporary -Recurse -Force
Write-Output $setup
