# 分析源图：背景/前景亮度分布、黑色方块边界、圆角半径
# Analyze the source image: luminance histogram, dark bbox, corner radius.
param([string]$Src = "docs\icon-source.png")

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$bmp = [System.Drawing.Bitmap]::FromFile($Src)
$w = $bmp.Width
$h = $bmp.Height
"size: ${w}x${h}  format: $($bmp.PixelFormat)"

$rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
$data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$stride = $data.Stride
$buf = New-Object byte[] ($stride * $h)
[System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $buf, 0, $buf.Length)
$bmp.UnlockBits($data)

# 亮度统计
$hist = New-Object int[] 256
$minL = 255; $maxL = 0
for ($y = 0; $y -lt $h; $y++) {
  $row = $y * $stride
  for ($x = 0; $x -lt $w; $x++) {
    $i = $row + $x * 4
    $b = $buf[$i]; $g = $buf[$i+1]; $r = $buf[$i+2]
    $l = [int](0.299*$r + 0.587*$g + 0.114*$b)
    $hist[$l]++
    if ($l -lt $minL) { $minL = $l }
    if ($l -gt $maxL) { $maxL = $l }
  }
}
"luminance min=$minL max=$maxL"
"pixels <16  : $($hist[0..15] | Measure-Object -Sum | Select-Object -ExpandProperty Sum)"
"pixels 16-60: $(($hist[16..60] | Measure-Object -Sum).Sum)"
"pixels 61-200: $(($hist[61..200] | Measure-Object -Sum).Sum)"
"pixels 201-250: $(($hist[201..250] | Measure-Object -Sum).Sum)"
"pixels >250: $((($hist[251..255] | Measure-Object -Sum).Sum))"

# 深色像素包围盒（阈值 100）
$minX = $w; $minY = $h; $maxX = -1; $maxY = -1
for ($y = 0; $y -lt $h; $y++) {
  $row = $y * $stride
  for ($x = 0; $x -lt $w; $x++) {
    $i = $row + $x * 4
    $l = 0.299*$buf[$i+2] + 0.587*$buf[$i+1] + 0.114*$buf[$i]
    if ($l -lt 100) {
      if ($x -lt $minX) { $minX = $x }
      if ($x -gt $maxX) { $maxX = $x }
      if ($y -lt $minY) { $minY = $y }
      if ($y -gt $maxY) { $maxY = $y }
    }
  }
}
"dark bbox: ($minX,$minY) - ($maxX,$maxY)  size $($maxX-$minX+1)x$($maxY-$minY+1)"

# 圆角半径：沿上边一行找左右边界，再逐行看左边界内缩情况
$top = $minY
$left = $minX
$rowTop = $top * $stride
# 顶边第一个深色像素的 x 范围（说明圆角起点）
$first = -1; $last = -1
for ($x = $minX; $x -le $maxX; $x++) {
  $i = $rowTop + $x*4
  $l = 0.299*$buf[$i+2] + 0.587*$buf[$i+1] + 0.114*$buf[$i]
  if ($l -lt 100) { if ($first -lt 0) { $first = $x }; $last = $x }
}
"top row dark span: x=$first..$last"

# 找到左边界到达 minX 的行（圆角结束的 y）
$cornerEndY = -1
for ($y = $top; $y -lt $top + 400; $y++) {
  $row = $y * $stride
  $lx = -1
  for ($x = $minX; $x -le $minX + 400; $x++) {
    $i = $row + $x*4
    $l = 0.299*$buf[$i+2] + 0.587*$buf[$i+1] + 0.114*$buf[$i]
    if ($l -lt 100) { $lx = $x; break }
  }
  if ($lx -eq $minX) { $cornerEndY = $y; break }
}
"left edge reaches minX at y=$cornerEndY  => radius ≈ $($cornerEndY - $top) px"

# 检查背景是否为纯白（四角区域）
$corners = @(
  @(5,5), @($w-6,5), @(5,$h-6), @($w-6,$h-6)
)
foreach ($c in $corners) {
  $i = $c[1]*$stride + $c[0]*4
  "corner ($($c[0]),$($c[1])) rgb = $($buf[$i+2]),$($buf[$i+1]),$($buf[$i])"
}
# 紧贴方块外侧的像素（判断有没白边/阴影）
$x2 = [int](($minX + $maxX)/2)
$i2 = ($minY - 3)*$stride + $x2*4
"above block rgb = $($buf[$i2+2]),$($buf[$i2+1]),$($buf[$i2])"
$i3 = $minY*$stride + $x2*4
"at block top edge rgb = $($buf[$i3+2]),$($buf[$i3+1]),$($buf[$i3])"

$bmp.Dispose()
