# 工程备份脚本 / Project backup helper
# 用法 / Usage:
#   pwsh -NoProfile -File tools/backup.ps1 -Note "即将进行的操作说明"
# 生成 / Creates:
#   llamaUI工程备份/llamaUI-<yyyyMMdd-HHmmss>.zip   源码快照
#   llamaUI工程备份/备份记录.txt                     追加一条操作记录
[CmdletBinding()]
param(
  [string]$Note = "未说明 / unspecified"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root 'llamaUI工程备份'
New-Item -ItemType Directory -Force -Path $dest | Out-Null

# 只备份源码与配置，排除工具链、模型与历史备份（体积控制）
$include = @('Cargo.toml', 'Cargo.lock', 'README.md', 'LICENSE', 'src', 'assets', 'locales', 'tools', 'docs', 'data', 'build.rs')
$staging = Join-Path $dest '.staging'
if (Test-Path $staging) { Remove-Item $staging -Recurse -Force }
New-Item -ItemType Directory -Force -Path $staging | Out-Null

foreach ($item in $include) {
  $p = Join-Path $root $item
  if (-not (Test-Path $p)) { continue }
  $target = Join-Path $staging $item
  if ((Get-Item $p).PSIsContainer) {
    Copy-Item $p $target -Recurse -Force
  } else {
    Copy-Item $p $target -Force
  }
}

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$zip = Join-Path $dest "llamaUI-$stamp.zip"
Compress-Archive -Path (Join-Path $staging '*') -DestinationPath $zip -CompressionLevel Optimal
Remove-Item $staging -Recurse -Force

$size = [math]::Round((Get-Item $zip).Length / 1KB, 1)
$line = "[{0}] {1}  |  zip: {2} ({3} KB)" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'), $Note, (Split-Path -Leaf $zip), $size
Add-Content -Path (Join-Path $dest '备份记录.txt') -Value $line -Encoding utf8

# 备份文件保留最近 40 个，避免无限增长
$old = Get-ChildItem $dest -Filter 'llamaUI-*.zip' | Sort-Object LastWriteTime -Descending | Select-Object -Skip 40
foreach ($f in $old) { Remove-Item $f.FullName -Force }

Write-Output "backup ok: $(Split-Path -Leaf $zip) ${size}KB"
