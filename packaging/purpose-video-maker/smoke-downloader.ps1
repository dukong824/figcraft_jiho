param([Parameter(Mandatory)][string]$Installed, [Parameter(Mandatory)][string]$Sample)
$ErrorActionPreference = 'Stop'
$Installed = (Resolve-Path -LiteralPath $Installed).Path
$python = Join-Path $Installed 'downloader/python/python.exe'
$deno = Join-Path $Installed 'downloader/deno.exe'
$ffmpeg = Join-Path $Installed 'ffmpeg/bin/ffmpeg.exe'
$ffprobe = Join-Path $Installed 'ffmpeg/bin/ffprobe.exe'
foreach ($file in @($python, $deno, (Join-Path $Installed 'codex/codex-code-mode-host.exe'), (Join-Path $Installed 'licenses/downloader/SOURCE.txt'))) {
    if (-not (Test-Path -LiteralPath $file)) { throw "Installed runtime missing: $file" }
}
function Invoke-Bounded([string]$File, [string[]]$Arguments) {
    $info = [Diagnostics.ProcessStartInfo]::new($File)
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($info)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit(90000)) { $process.Kill($true); throw "Runtime timed out: $File" }
    $text = $stdout.GetAwaiter().GetResult()
    $errorText = $stderr.GetAwaiter().GetResult()
    if ($process.ExitCode -ne 0) { throw "Runtime failed: $File`n$text`n$errorText" }
    return $text
}
$null = Invoke-Bounded (Join-Path $Installed 'codex/codex-code-mode-host.exe') @('--help')
$root = Join-Path $env:RUNNER_TEMP 'purpose-downloader-smoke'
New-Item -ItemType Directory -Path $root -Force | Out-Null
Copy-Item -LiteralPath $Sample -Destination (Join-Path $root 'sample.mp4')
$listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
$listener.Start()
$port = $listener.LocalEndpoint.Port
$listener.Stop()
$info = [Diagnostics.ProcessStartInfo]::new($python)
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
foreach ($argument in @('-I', '-m', 'http.server', "$port", '--bind', '127.0.0.1', '--directory', $root)) { $info.ArgumentList.Add($argument) }
$server = [Diagnostics.Process]::Start($info)
try {
    $ready = $false
    for ($i = 0; $i -lt 40; $i++) {
        try { $client = [Net.Sockets.TcpClient]::new('127.0.0.1', $port); $client.Dispose(); $ready = $true; break } catch { Start-Sleep -Milliseconds 100 }
    }
    if (-not $ready) { throw 'Local downloader fixture did not start' }
    foreach ($format in @('mp4', 'mp3')) {
        $out = Join-Path $root "download-$format"
        New-Item -ItemType Directory -Path $out -Force | Out-Null
        $arguments = @('-I', '-X', 'utf8', '-m', 'yt_dlp', '--ignore-config', '--no-plugin-dirs', '--no-update', '--no-playlist', '--playlist-items', '1', '--no-simulate', '--windows-filenames', '--ffmpeg-location', $ffmpeg, '--js-runtimes', "deno:$deno", '--print', 'after_move:purpose-result:{"path":%(filepath)j,"title":%(title)j}', '-o', (Join-Path $out '%(title).160B [%(id)s].%(ext)s'))
        if ($format -eq 'mp3') { $arguments += @('-f', 'bestaudio/best', '-x', '--audio-format', 'mp3', '--audio-quality', '0') }
        else { $arguments += @('-f', 'bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b', '--merge-output-format', 'mp4', '--remux-video', 'mp4') }
        $arguments += @('--', "http://127.0.0.1:$port/sample.mp4")
        $text = Invoke-Bounded $python $arguments
        $resultLine = @($text -split "`n" | Where-Object { $_.StartsWith('purpose-result:') })
        if ($resultLine.Count -ne 1) { throw "Downloader result missing: $text" }
        $result = $resultLine[0].Substring('purpose-result:'.Length) | ConvertFrom-Json
        if (-not (Test-Path -LiteralPath $result.path) -or [IO.Path]::GetExtension($result.path) -ne ".$format" -or (Get-Item -LiteralPath $result.path).Length -le 0) { throw 'Downloaded file invalid' }
        $codec = Invoke-Bounded $ffprobe @('-v', 'error', '-show_entries', 'stream=codec_name', '-of', 'default=noprint_wrappers=1:nokey=1', $result.path)
        if (($format -eq 'mp3' -and $codec.Trim() -ne 'mp3') -or ($format -eq 'mp4' -and $codec -notmatch 'h264')) { throw "Wrong downloaded codec: $codec" }
    }
    Write-Output 'Installed code-mode host and local HTTP video / MP3 download passed'
} finally { if (-not $server.HasExited) { $server.Kill($true); $server.WaitForExit() } }
