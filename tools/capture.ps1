# 截取 llamaUI 窗口截图：置前 + 屏幕拷贝（含 PrintWindow 兜底）
# Capture the llamaUI window: bring to front then screen-copy, PrintWindow as fallback.
param(
  [string]$TitleLike = '*模型管理器*',
  [string]$Out = 'docs/shot-gui.png'
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class Cap2 {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
}
"@

$p = Get-Process llamaui -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $p) { throw "llamaui 未运行" }
$pidTarget = $p.Id

$found = New-Object System.Collections.ArrayList
$cb = [Cap2+EnumProc]{
  param($h, $l)
  $pp = 0
  [void][Cap2]::GetWindowThreadProcessId($h, [ref]$pp)
  if ($pp -eq $pidTarget -and [Cap2]::IsWindowVisible($h)) {
    $sb = New-Object System.Text.StringBuilder 300
    [void][Cap2]::GetWindowText($h, $sb, 300)
    $t = $sb.ToString()
    if ($t.Length -gt 0 -and $t -like $TitleLike) {
      [void]$found.Add([pscustomobject]@{ H = [IntPtr]$h; T = $t })
    }
  }
  return $true
}
[void][Cap2]::EnumWindows($cb, [IntPtr]::Zero)
if ($found.Count -eq 0) { throw "未找到匹配窗口：$TitleLike" }

$win = $found | Select-Object -First 1
# 置前：恢复 → 置顶 → 取消置顶 → 切前台
[void][Cap2]::ShowWindow($win.H, 9)              # SW_RESTORE
[void][Cap2]::SetWindowPos($win.H, [IntPtr](-1), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040)  # TOPMOST|NOSIZE|NOMOVE
[void][Cap2]::SetWindowPos($win.H, [IntPtr](-2), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040)  # NOTOPMOST 回退
[void][Cap2]::SetForegroundWindow($win.H)
[void][Cap2]::BringWindowToTop($win.H)
Start-Sleep -Milliseconds 700

$r = New-Object Cap2+RECT
[void][Cap2]::GetWindowRect($win.H, [ref]$r)
$w = $r.R - $r.L
$hgt = $r.B - $r.T
if ($w -le 0 -or $hgt -le 0) { throw "窗口尺寸无效" }

$path = Join-Path (Get-Location) $Out
$bmp = New-Object System.Drawing.Bitmap $w, $hgt
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $hgt))
$g.Dispose()
$bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

Write-Output ("captured '{0}' {1}x{2} -> {3}" -f $win.T, $w, $hgt, $Out)
