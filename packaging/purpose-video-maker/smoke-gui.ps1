param([Parameter(Mandatory=$true)][string]$Executable, [Parameter(Mandatory=$true)][string]$ProjectDirectory)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class PurposeWindowCheck {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct MonitorInfo { public int Size; public Rect Monitor, Work; public uint Flags; }
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
    [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr monitor, ref MonitorInfo info);
    public static Rect Bounds(IntPtr window) { Rect rect; if (!GetWindowRect(window, out rect)) throw new Exception("GetWindowRect failed"); return rect; }
    public static Rect Screen(IntPtr window) {
        MonitorInfo info = new MonitorInfo(); info.Size = Marshal.SizeOf(typeof(MonitorInfo));
        if (!GetMonitorInfo(MonitorFromWindow(window, 2), ref info)) throw new Exception("GetMonitorInfo failed"); return info.Monitor;
    }
    public static void F11(IntPtr window) {
        PostMessage(window, 0x100, new IntPtr(0x7A), new IntPtr(0x00570001));
        PostMessage(window, 0x101, new IntPtr(0x7A), new IntPtr(0xC0570001L));
    }
}
'@
function Test-NearBounds($Actual, $Expected) {
    return [Math]::Abs($Actual.Left-$Expected.Left) -le 4 -and [Math]::Abs($Actual.Top-$Expected.Top) -le 4 -and [Math]::Abs($Actual.Right-$Expected.Right) -le 4 -and [Math]::Abs($Actual.Bottom-$Expected.Bottom) -le 4
}
function Wait-Bounds($Window, $Expected) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    while ($timer.Elapsed.TotalSeconds -lt 15) {
        if (Test-NearBounds ([PurposeWindowCheck]::Bounds($Window)) $Expected) { return }
        Start-Sleep -Milliseconds 200
    }
    $actual = [PurposeWindowCheck]::Bounds($Window)
    throw "Unexpected window bounds: $($actual.Left),$($actual.Top),$($actual.Right),$($actual.Bottom); expected $($Expected.Left),$($Expected.Top),$($Expected.Right),$($Expected.Bottom)"
}
$env:PURPOSE_PROJECTS_DIR = $ProjectDirectory
$app = Start-Process -FilePath $Executable -PassThru
try {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    do {
        Start-Sleep -Milliseconds 200
        $app.Refresh()
        if ($app.HasExited) { throw "Installed GUI exited early: $($app.ExitCode)" }
    } while ($app.MainWindowHandle -eq 0 -and $timer.Elapsed.TotalSeconds -lt 30)
    $window = $app.MainWindowHandle
    if ($window -eq 0) { throw 'Installed GUI has no main window' }
    $timer.Restart()
    do {
        $bigIcon = [PurposeWindowCheck]::SendMessage($window, 0x7F, [IntPtr]1, [IntPtr]0)
        if ($bigIcon -ne [IntPtr]::Zero) { break }
        $app.Refresh()
        if ($app.HasExited) { throw "Installed GUI exited before icon setup: $($app.ExitCode)" }
        Start-Sleep -Milliseconds 200
    } while ($timer.Elapsed.TotalSeconds -lt 30)
    if ($bigIcon -eq [IntPtr]::Zero) { throw 'Windows taskbar has no explicit large application icon' }
    $normal = [PurposeWindowCheck]::Bounds($window)
    $screen = [PurposeWindowCheck]::Screen($window)
    [PurposeWindowCheck]::F11($window)
    Wait-Bounds $window $screen
    [PurposeWindowCheck]::F11($window)
    Wait-Bounds $window $normal
    $null = $app.CloseMainWindow()
    if (-not $app.WaitForExit(15000)) { throw 'Installed GUI did not close cleanly' }
    if ($app.ExitCode -ne 0) { throw "Installed GUI failed: $($app.ExitCode)" }
    Write-Output 'Installed GUI: main window, taskbar icon, fullscreen, restore and clean exit passed'
} finally {
    if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force }
    Remove-Item Env:PURPOSE_PROJECTS_DIR
}
