# 界面自动化小工具：按窗口类查找主窗口，取控件、点按钮、切语言
# UI automation helper: find the main window by class, get controls, click, switch language.
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class Ui {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr h, int id);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr p);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
"@

function Get-LlamaWindows {
  $proc = Get-Process llamaui -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $proc) { return @() }
  $target = $proc.Id
  $rows = New-Object System.Collections.ArrayList
  $cb = [Ui+EnumProc]{
    param($w, $l)
    $p = 0
    [void][Ui]::GetWindowThreadProcessId($w, [ref]$p)
    if ($p -eq $target) {
      $cls = New-Object System.Text.StringBuilder 128
      [void][Ui]::GetClassName($w, $cls, 128)
      $txt = New-Object System.Text.StringBuilder 256
      [void][Ui]::GetWindowText($w, $txt, 256)
      $r = New-Object Ui+RECT
      [void][Ui]::GetWindowRect($w, [ref]$r)
      [void]$rows.Add([pscustomobject]@{
        H = [IntPtr]$w; Cls = $cls.ToString(); Txt = $txt.ToString()
        Vis = [Ui]::IsWindowVisible($w); Rect = "$($r.L),$($r.T),$($r.R),$($r.B)"
      })
    }
    return $true
  }
  [void][Ui]::EnumWindows($cb, [IntPtr]::Zero)
  return $rows
}

function Get-MainWindow {
  # 主窗口标题里带 “— 副标题”，高级设置窗口标题以“高级设置 / Advanced settings”开头，需排除
  $w = Get-LlamaWindows | Where-Object {
    $_.Cls -eq 'LlamaUiMain' -and $_.Txt.Length -gt 0 -and $_.Vis -and
    $_.Txt -notlike '高级设置*' -and $_.Txt -notlike 'Advanced settings*'
  }
  return ($w | Select-Object -First 1).H
}

function Show-Front([IntPtr]$h) {
  $fg = [Ui]::GetForegroundWindow()
  $fgT = [Ui]::GetWindowThreadProcessId($fg, [IntPtr]::Zero)
  $myT = [Ui]::GetCurrentThreadId()
  [void][Ui]::AttachThreadInput($myT, $fgT, $true)
  [void][Ui]::ShowWindow($h, 9)
  [void][Ui]::BringWindowToTop($h)
  [void][Ui]::SetForegroundWindow($h)
  [void][Ui]::AttachThreadInput($myT, $fgT, $false)
  Start-Sleep -Milliseconds 400
}

function Click-Id([IntPtr]$main, [int]$id) {
  $ctl = [Ui]::GetDlgItem($main, $id)
  [void][Ui]::SendMessage($main, 0x0111, [IntPtr]$id, $ctl)
  Start-Sleep -Milliseconds 400
}
