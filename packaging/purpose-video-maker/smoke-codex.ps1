param([Parameter(Mandatory)][string]$Installed)
$ErrorActionPreference = 'Stop'
$cli = Join-Path (Resolve-Path -LiteralPath $Installed).Path 'codex/codex.exe'
$root = Join-Path ([IO.Path]::GetTempPath()) "purpose-codex-protocol-$([Guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $root | Out-Null
$process = $null
try {
    $info = [Diagnostics.ProcessStartInfo]::new($cli)
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.Environment['CODEX_HOME'] = $root
    $null = $info.Environment.Remove('OPENAI_API_KEY')
    $null = $info.Environment.Remove('CODEX_API_KEY')
    foreach ($argument in @('-c', 'forced_login_method="chatgpt"', '-c', 'model_provider="openai"', '-c', 'features.request_permissions_tool=true', '-c', 'features.default_mode_request_user_input=true', 'app-server', '--listen', 'stdio://')) {
        $info.ArgumentList.Add($argument)
    }
    $process = [Diagnostics.Process]::Start($info)
    $stderr = $process.StandardError.ReadToEndAsync()
    function Send-Request($Request) {
        $process.StandardInput.WriteLine(($Request | ConvertTo-Json -Depth 20 -Compress))
        $process.StandardInput.Flush()
    }
    function Read-Response([int]$RequestId) {
        $deadline = [DateTime]::UtcNow.AddSeconds(30)
        while ([DateTime]::UtcNow -lt $deadline) {
            $line = $process.StandardOutput.ReadLineAsync()
            if (-not $line.Wait([TimeSpan]::FromSeconds(30))) { throw 'Codex protocol response timed out' }
            $text = $line.GetAwaiter().GetResult()
            if ($null -eq $text) { throw 'Codex protocol closed before replying' }
            $value = $text | ConvertFrom-Json
            if ($null -ne $value.id -and $value.id -eq $RequestId) {
                if ($value.error) { throw "Codex protocol rejected request: $text" }
                return $value.result
            }
        }
        throw 'Codex protocol did not reply to the requested ID'
    }
    Send-Request @{ id = 1; method = 'initialize'; params = @{ clientInfo = @{ name = 'purpose_installed_smoke'; version = '1' }; capabilities = @{ experimentalApi = $true } } }
    $null = Read-Response 1
    Send-Request @{ method = 'initialized' }
    Send-Request @{ id = 2; method = 'thread/start'; params = @{ cwd = $root; modelProvider = 'openai'; sandbox = 'read-only'; ephemeral = $true; approvalsReviewer = 'user'; approvalPolicy = @{ granular = @{ rules = $true; sandbox_approval = $false; mcp_elicitations = $true; request_permissions = $true } } } }
    $thread = Read-Response 2
    if (-not $thread.thread.id) { throw 'Codex app-server did not create a thread' }
    # No turn/start: this checks the installed transport without login or model usage.
    Write-Output 'Installed Codex app-server: initialize, read-only thread and user approval policy passed'
} finally {
    if ($process) {
        $process.StandardInput.Close()
        if (-not $process.WaitForExit(3000)) { $process.Kill($true); $process.WaitForExit() }
        $process.Dispose()
    }
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
