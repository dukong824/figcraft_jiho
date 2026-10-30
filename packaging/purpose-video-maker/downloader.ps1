param([Parameter(Mandatory)][string]$Stage, [Parameter(Mandatory)][string]$Temporary)
$ErrorActionPreference = 'Stop'
# Official PyPI wheels contain only Unlicense code. The upstream PyInstaller EXE includes
# optional GPL dependencies and is deliberately not redistributed by this application.
$tools = Join-Path $Stage 'downloader'
$python = Join-Path $tools 'python'
$packages = Join-Path $python 'Lib/site-packages'
New-Item -ItemType Directory -Path $packages -Force | Out-Null
$downloads = @(
    @{ Name='python'; Url='https://www.python.org/ftp/python/3.14.10/python-3.14.10-embed-amd64.zip'; Sha='90905e64a840557d8c1bc4f5b74db3d7ee85e3ffb217611cac1c7813acea4159'; Target=$python }
    @{ Name='yt-dlp'; Url='https://files.pythonhosted.org/packages/7f/3d/16ea7d5638cf585991f2c5705e8083688dde633d32d0871f2404e0f4d827/yt_dlp-2026.10.26-py3-none-any.whl'; Sha='a05c5b5d8562f7e8a62f5334348b406af4930d56d3a65f8021d14af0e22339ae'; Target=$packages }
    @{ Name='yt-dlp-ejs'; Url='https://files.pythonhosted.org/packages/e3/bd/520769863744b669440a924271a6159ddd82ad5ae26b4ac4d4b69e9f8d44/yt_dlp_ejs-0.8.0-py3-none-any.whl'; Sha='79300e5fca7f937a1eeede11f0456862c1b41107ce1d726871e0207424f4bdb4'; Target=$packages }
    @{ Name='deno'; Url='https://github.com/denoland/deno/releases/download/v2.9.8/deno-x86_64-pc-windows-msvc.zip'; Sha='cba84b26e0df5ae7f74fc66b86a95b866872a86292ebd84fa1e959927edd93bf'; Target=$tools }
)
foreach ($item in $downloads) {
    $archive = Join-Path $Temporary "$($item.Name).zip"
    Invoke-WebRequest -Uri $item.Url -OutFile $archive
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $item.Sha) { throw "Downloader checksum mismatch: $($item.Name)" }
    Expand-Archive -LiteralPath $archive -DestinationPath $item.Target -Force
}
@'
python314.zip
.
Lib/site-packages
'@ | Set-Content -LiteralPath (Join-Path $python 'python314._pth') -Encoding ascii
$notices = Join-Path $Stage 'licenses/downloader'
New-Item -ItemType Directory -Path $notices -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $python 'LICENSE.txt') -Destination (Join-Path $notices 'Python-LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $packages 'yt_dlp-2026.10.26.dist-info/licenses/LICENSE') -Destination (Join-Path $notices 'yt-dlp-LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $packages 'yt_dlp_ejs-0.8.0.dist-info/licenses/LICENSE') -Destination (Join-Path $notices 'yt-dlp-ejs-LICENSE.txt')
# Deno 2.9 has no license subcommand/flag. Distribute the unchanged upstream
# notices pinned to the runtime and its V8 / TypeScript versions instead.
$noticeSource = Join-Path $PSScriptRoot '../../third-party/downloader'
foreach ($name in @('Deno-LICENSE.txt', 'Rusty-V8-LICENSE.txt', 'V8-LICENSE.txt', 'TypeScript-LICENSE.txt', 'UPSTREAM.md')) {
    Copy-Item -LiteralPath (Join-Path $noticeSource $name) -Destination $notices
}
$downloads | ForEach-Object { "$($_.Name)`n$($_.Url)`nSHA256: $($_.Sha)`n" } | Set-Content -LiteralPath (Join-Path $notices 'SOURCE.txt') -Encoding utf8
& (Join-Path $python 'python.exe') -I -m yt_dlp --version
if ($LASTEXITCODE -ne 0) { throw 'Bundled yt-dlp failed' }
& (Join-Path $tools 'deno.exe') --version
if ($LASTEXITCODE -ne 0) { throw 'Bundled Deno failed' }
