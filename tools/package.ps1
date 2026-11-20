# 打包脚本：构建 release 并输出到 dist/（相对路径，便于整包迁移）
# Package script: build release and emit dist/llamaui.exe
[CmdletBinding()]
param(
  [switch]$NoBuild
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $NoBuild) {
  $env:CARGO_HOME = Join-Path $root '.toolchain\cargo'
  $env:RUSTUP_HOME = Join-Path $root '.toolchain\rustup'
  $mingw = Join-Path $root '.toolchain\w64devkit\bin'
  $cargoBin = Join-Path $root '.toolchain\cargo\bin'
  $prefix = @()
  if (Test-Path $mingw) { $prefix += $mingw }
  if (Test-Path $cargoBin) { $prefix += $cargoBin }
  if ($prefix.Count -gt 0) { $env:PATH = (($prefix -join ';') + ';' + $env:PATH) }
  & cargo build --release
  if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

$exe = if ($IsWindows -or $env:OS -eq 'Windows_NT') { 'llamaui.exe' } else { 'llamaui' }
$built = Join-Path $root "target\release\$exe"
if (-not (Test-Path $built)) { throw "binary not found: $built" }

$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item $built (Join-Path $dist $exe) -Force

$size = [math]::Round((Get-Item (Join-Path $dist $exe)).Length / 1KB, 1)
Write-Output "packaged: dist/$exe  ${size}KB"
Write-Output "提示：dist 内的可执行文件会自动在上级目录查找 llama-server；"
Write-Output "      若要使用工程内的模型与数据，请把 exe 放在工程根目录运行。"
