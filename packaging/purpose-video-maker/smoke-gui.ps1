param([Parameter(Mandatory=$true)][string]$Executable, [Parameter(Mandatory=$true)][string]$ProjectDirectory)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class PurposeWindowCheck {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct MonitorInfo { public int Size; public Rect Monitor, Work; public uint Flags; }
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr window, ref Point point);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
    [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr monitor, ref MonitorInfo info);
    public delegate bool EnumWindow(IntPtr window, IntPtr data);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindow callback, IntPtr data);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr window, StringBuilder title, int count);
    public static string Title(IntPtr window) { var title = new StringBuilder(512); GetWindowText(window, title, title.Capacity); return title.ToString(); }
    public static IntPtr Find(int process) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((window, data) => {
            uint owner; GetWindowThreadProcessId(window, out owner);
            if (owner == (uint)process && IsWindowVisible(window) && Title(window).Contains("Purpose video maker")) { found = window; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static Rect Bounds(IntPtr window) { Rect rect; if (!GetWindowRect(window, out rect)) throw new Exception("GetWindowRect failed"); return rect; }
    public static Rect Screen(IntPtr window) {
        MonitorInfo info = new MonitorInfo(); info.Size = Marshal.SizeOf(typeof(MonitorInfo));
        if (!GetMonitorInfo(MonitorFromWindow(window, 2), ref info)) throw new Exception("GetMonitorInfo failed"); return info.Monitor;
    }
    public static void F11(IntPtr window) {
        PostMessage(window, 0x100, new IntPtr(0x7A), new IntPtr(0x00570001));
        PostMessage(window, 0x101, new IntPtr(0x7A), new IntPtr(0xC0570001L));
    }
    public static void Escape(IntPtr window) {
        PostMessage(window, 0x100, new IntPtr(0x1B), new IntPtr(0x00010001));
        PostMessage(window, 0x101, new IntPtr(0x1B), new IntPtr(0xC0010001L));
    }
    public static void Click(IntPtr window, int x, int y) {
        Point screen = new Point { X = x, Y = y };
        if (!ClientToScreen(window, ref screen)) throw new Exception("ClientToScreen failed");
        SetCursorPos(screen.X, screen.Y);
        IntPtr position = new IntPtr((y << 16) | (x & 0xFFFF));
        PostMessage(window, 0x200, IntPtr.Zero, position);
        PostMessage(window, 0x201, new IntPtr(1), position);
        PostMessage(window, 0x202, IntPtr.Zero, position);
    }
    public static void OpenEditor(IntPtr window) {
        double scale = GetDpiForWindow(window) / 96.0;
        Click(window, (int)(90 * scale), (int)(423 * scale));
    }
    public static void EditorFullscreen(IntPtr window, bool fullscreen) {
        Rect client; if (!GetClientRect(window, out client)) throw new Exception("GetClientRect failed");
        double scale = GetDpiForWindow(window) / 96.0;
        Click(window, client.Right - (int)(28 * scale), (int)((fullscreen ? 20 : 111) * scale));
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
$previousLogLevel = $env:RUST_LOG
$env:RUST_LOG = 'info'
New-Item -ItemType Directory -Path $ProjectDirectory -Force | Out-Null
$startupLog = Join-Path $ProjectDirectory 'startup.log'
$app = Start-Process -FilePath $Executable -PassThru -RedirectStandardError $startupLog
try {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    do {
        Start-Sleep -Milliseconds 200
        $app.Refresh()
        if ($app.HasExited) { throw "Installed GUI exited early: $($app.ExitCode)" }
        $window = [PurposeWindowCheck]::Find($app.Id)
    } while ($window -eq 0 -and $timer.Elapsed.TotalSeconds -lt 30)
    if ($window -eq 0) { throw 'Installed GUI has no main window' }
    Write-Output "Testing window: $([PurposeWindowCheck]::Title($window)); handle=$window"
    $timer.Restart()
    do {
        $bigIcon = [PurposeWindowCheck]::SendMessage($window, 0x7F, [IntPtr]1, [IntPtr]0)
        if ($bigIcon -ne [IntPtr]::Zero) { break }
        $app.Refresh()
        if ($app.HasExited) { throw "Installed GUI exited before icon setup: $($app.ExitCode)" }
        Start-Sleep -Milliseconds 200
    } while ($timer.Elapsed.TotalSeconds -lt 30)
    if ($bigIcon -eq [IntPtr]::Zero) {
        $smallIcon = [PurposeWindowCheck]::SendMessage($window, 0x7F, [IntPtr]0, [IntPtr]0)
        throw "Windows taskbar has no explicit large application icon; window=$window; small=$smallIcon"
    }
    $normal = [PurposeWindowCheck]::Bounds($window)
    $screen = [PurposeWindowCheck]::Screen($window)
    [PurposeWindowCheck]::OpenEditor($window)
    Start-Sleep -Seconds 1
    [PurposeWindowCheck]::EditorFullscreen($window, $false)
    Wait-Bounds $window $screen
    Start-Sleep -Milliseconds 300
    [PurposeWindowCheck]::EditorFullscreen($window, $true)
    Wait-Bounds $window $normal
    [PurposeWindowCheck]::F11($window)
    Wait-Bounds $window $screen
    [PurposeWindowCheck]::Escape($window)
    Wait-Bounds $window $normal
    $null = $app.CloseMainWindow()
    if (-not $app.WaitForExit(15000)) { throw 'Installed GUI did not close cleanly' }
    if ($app.ExitCode -ne 0) { throw "Installed GUI failed: $($app.ExitCode)" }
    Write-Output 'Installed GUI: main window, taskbar icon, FilmCraft fullscreen button, restore, F11/Esc and clean exit passed'
} finally {
    if (Test-Path -LiteralPath $startupLog) { Get-Content -LiteralPath $startupLog | Write-Output }
    if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force }
    Remove-Item Env:PURPOSE_PROJECTS_DIR
    $env:RUST_LOG = $previousLogLevel
}
