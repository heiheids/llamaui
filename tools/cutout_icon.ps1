# 抠图工具：把白色背景去掉，保留黑色圆角方块 + 白色 AI
# Cut out the white background, keep the black rounded square with white "AI".
#
# 尝试三种方法（结果分别输出，便于对比）：
#   A 亮度阈值：直接把接近白色的像素设为透明
#   B 反相 alpha：alpha = 1 - 亮度（白底变透明，但白字也会一起变透明）
#   C 解析圆角 + 字形遮罩（最终采用）：拟合圆角矩形，用解析形状生成抗锯齿 alpha，
#     字形用内部亮度做遮罩，边缘永不带白边
#
# 输出：assets/icon-<size>.png、docs/icon-methodA.png、docs/icon-methodC.png 等
param(
  [string]$Src = "docs\icon-source.png",
  [int[]]$Sizes = @(16, 24, 32, 48, 64, 128, 256, 512),
  [double]$Margin = 0.0
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

function Lum([byte]$r, [byte]$g, [byte]$b) { return 0.299*$r + 0.587*$g + 0.114*$b }

# ---------------- 读入源图 ----------------
$bmp = [System.Drawing.Bitmap]::FromFile($Src)
$W = $bmp.Width
$H = $bmp.Height
$rect = New-Object System.Drawing.Rectangle 0, 0, $W, $H
$data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$stride = $data.Stride
$px = New-Object byte[] ($stride * $H)
[System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $px, 0, $px.Length)
$bmp.UnlockBits($data)
$bmp.Dispose()

# 亮度表
$lum = New-Object byte[] ($W * $H)
for ($y = 0; $y -lt $H; $y++) {
  $row = $y * $stride
  $orow = $y * $W
  for ($x = 0; $x -lt $W; $x++) {
    $i = $row + $x * 4
    $lum[$orow + $x] = [byte][math]::Round((0.299*$px[$i+2] + 0.587*$px[$i+1] + 0.114*$px[$i]))
  }
}
"loaded $($W)x$($H)"

# ---------------- 形状包围盒（阈值 128，取中点） ----------------
$thr = 128
$minX = $W; $minY = $H; $maxX = -1; $maxY = -1
for ($y = 0; $y -lt $H; $y++) {
  $orow = $y * $W
  for ($x = 0; $x -lt $W; $x++) {
    if ($lum[$orow + $x] -lt $thr) {
      if ($x -lt $minX) { $minX = $x }
      if ($x -gt $maxX) { $maxX = $x }
      if ($y -lt $minY) { $minY = $y }
      if ($y -gt $maxY) { $maxY = $y }
    }
  }
}
$x0 = $minX; $y0 = $minY
$sw = $maxX - $minX + 1
$sh = $maxY - $minY + 1
"shape bbox: ($x0,$y0) ${sw}x${sh}"

# ---------------- 拟合圆角半径 ----------------
# 左上角：对每条扫描行找最左深色像素，按圆角公式拟合 r
$samples = New-Object System.Collections.ArrayList
for ($y = $y0; $y -lt $y0 + [int]($sh/2); $y++) {
  $orow = $y * $W
  for ($x = $x0; $x -lt $x0 + [int]($sw/2); $x++) {
    if ($lum[$orow + $x] -lt $thr) {
      [void]$samples.Add(@($x, $y))
      break
    }
  }
}
$bestR = 0; $bestErr = [double]::MaxValue
for ($r = 60; $r -le 300; $r++) {
  $err = 0.0; $n = 0
  foreach ($s in $samples) {
    $x = $s[0]; $y = $s[1]
    $dy = $y0 + $r - $y
    if ($dy -lt 0) { continue }
    if ($y -ge $y0 + $r) { continue }
    $dx = [math]::Sqrt([math]::Max(0.0, $r*$r - $dy*$dy))
    $predX = $x0 + $r - $dx
    $err += [math]::Abs($predX - $x)
    $n++
  }
  if ($n -gt 50) {
    $err = $err / $n
    if ($err -lt $bestErr) { $bestErr = $err; $bestR = $r }
  }
}
$radius = $bestR
"fit radius = $radius (mean err $([math]::Round($bestErr,2)) px)"

# ---------------- 字形遮罩：形状内部、且亮（白字） ----------------
# 先算出“至少离边界 3px”的安全区，避免边缘抗锯齿被当成字形
$glyph = New-Object byte[] ($W * $H)
$cxr = $x0 + $sw / 2.0
$cyr = $y0 + $sh / 2.0
$inset = 3.0
for ($y = $y0; $y -lt $y0 + $sh; $y++) {
  $orow = $y * $W
  for ($x = $x0; $x -lt $x0 + $sw; $x++) {
    if ($lum[$orow + $x] -le 128) { continue }
    # 判断点是否在“内缩 3px”的圆角矩形内
    $lx = $x + 0.5; $ly = $y + 0.5
    $ix0 = $x0 + $inset; $iy0 = $y0 + $inset
    $ix1 = $x0 + $sw - $inset; $iy1 = $y0 + $sh - $inset
    $ir = [math]::Max(0.0, $radius - $inset)
    $insideX = ($lx -ge $ix0) -and ($lx -le $ix1)
    $insideY = ($ly -ge $iy0) -and ($ly -le $iy1)
    if (-not ($insideX -and $insideY)) { continue }
    $ok = $true
    # 只在四个角区域做圆角判断
    if ($lx -lt $ix0 + $ir -and $ly -lt $iy0 + $ir) {
      $dx = $lx - ($ix0 + $ir); $dy = $ly - ($iy0 + $ir)
      $ok = ($dx*$dx + $dy*$dy) -le $ir*$ir
    } elseif ($lx -gt $ix1 - $ir -and $ly -lt $iy0 + $ir) {
      $dx = $lx - ($ix1 - $ir); $dy = $ly - ($iy0 + $ir)
      $ok = ($dx*$dx + $dy*$dy) -le $ir*$ir
    } elseif ($lx -lt $ix0 + $ir -and $ly -gt $iy1 - $ir) {
      $dx = $lx - ($ix0 + $ir); $dy = $ly - ($iy1 - $ir)
      $ok = ($dx*$dx + $dy*$dy) -le $ir*$ir
    } elseif ($lx -gt $ix1 - $ir -and $ly -gt $iy1 - $ir) {
      $dx = $lx - ($ix1 - $ir); $dy = $ly - ($iy1 - $ir)
      $ok = ($dx*$dx + $dy*$dy) -le $ir*$ir
    }
    if ($ok) {
      # 用亮度做柔边（0..1），保留字母的抗锯齿
      $v = ($lum[$orow + $x] - 140.0) / (240.0 - 140.0)
      if ($v -lt 0) { $v = 0 }; if ($v -gt 1) { $v = 1 }
      $glyph[$orow + $x] = [byte][math]::Round($v * 255)
    }
  }
}

# ---------------- 渲染：方法 C ----------------
# 形状用解析圆角做 4x4 超采样（边缘干净）；
# 字形按像素覆盖面积做盒式平均（小尺寸不会发灰、也不会点采样丢笔画）。
function Render-C([int]$size) {
  $SS = 4
  $out = New-Object byte[] ($size * $size * 4)
  $marginPx = $Margin * $size
  $avail = $size - 2 * $marginPx
  $side = [math]::Min($sw, $sh)
  $scale = $avail / $side
  $drawW = $sw * $scale
  $drawH = $sh * $scale
  $offX = ($size - $drawW) / 2.0
  $offY = ($size - $drawH) / 2.0
  $rr = $radius

  # 字形盒式平均的采样步长（每像素最多 8x8 次）
  $foot = $scale   # 一个目标像素覆盖多少源像素
  $step = [math]::Max(1, [int][math]::Floor($foot / 8))

  for ($oy = 0; $oy -lt $size; $oy++) {
    for ($ox = 0; $ox -lt $size; $ox++) {
      # ---- 形状覆盖率 ----
      $cov = 0.0
      for ($sy = 0; $sy -lt $SS; $sy++) {
        for ($sx = 0; $sx -lt $SS; $sx++) {
          $pxf = $ox + ($sx + 0.5) / $SS
          $pyf = $oy + ($sy + 0.5) / $SS
          $lx = ($pxf - $offX) / $scale
          $ly = ($pyf - $offY) / $scale
          $inside = ($lx -ge 0) -and ($lx -le $sw) -and ($ly -ge 0) -and ($ly -le $sh)
          if ($inside -and $rr -gt 0) {
            if ($lx -lt $rr -and $ly -lt $rr) {
              $dx = $lx - $rr; $dy = $ly - $rr
              $inside = ($dx*$dx + $dy*$dy) -le $rr*$rr
            } elseif ($lx -gt $sw - $rr -and $ly -lt $rr) {
              $dx = $lx - ($sw - $rr); $dy = $ly - $rr
              $inside = ($dx*$dx + $dy*$dy) -le $rr*$rr
            } elseif ($lx -lt $rr -and $ly -gt $sh - $rr) {
              $dx = $lx - $rr; $dy = $ly - ($sh - $rr)
              $inside = ($dx*$dx + $dy*$dy) -le $rr*$rr
            } elseif ($lx -gt $sw - $rr -and $ly -gt $sh - $rr) {
              $dx = $lx - ($sw - $rr); $dy = $ly - ($sh - $rr)
              $inside = ($dx*$dx + $dy*$dy) -le $rr*$rr
            }
          }
          if ($inside) { $cov += 1.0 }
        }
      }
      $cov = $cov / ($SS * $SS)
      $o = ($oy * $size + $ox) * 4
      if ($cov -le 0) {
        $out[$o] = 0; $out[$o+1] = 0; $out[$o+2] = 0; $out[$o+3] = 0
        continue
      }

      # ---- 字形覆盖率：对该像素覆盖的源区域做盒式平均 ----
      $gx0 = [int][math]::Floor($x0 + ($ox - $offX) / $scale)
      $gy0 = [int][math]::Floor($y0 + ($oy - $offY) / $scale)
      $gx1 = [int][math]::Ceiling($x0 + ($ox + 1 - $offX) / $scale)
      $gy1 = [int][math]::Ceiling($y0 + ($oy + 1 - $offY) / $scale)
      if ($gx0 -lt $x0) { $gx0 = $x0 }
      if ($gy0 -lt $y0) { $gy0 = $y0 }
      if ($gx1 -gt $x0 + $sw) { $gx1 = $x0 + $sw }
      if ($gy1 -gt $y0 + $sh) { $gy1 = $y0 + $sh }
      $sum = 0.0; $cnt = 0
      for ($yy = $gy0; $yy -lt $gy1; $yy += $step) {
        $orow = $yy * $W
        for ($xx = $gx0; $xx -lt $gx1; $xx += $step) {
          $sum += $glyph[$orow + $xx]
          $cnt++
        }
      }
      $gfrac = if ($cnt -gt 0) { ($sum / $cnt) / 255.0 } else { 0.0 }
      # 小尺寸稍作对比度提升，避免字母被平均成灰
      if ($size -le 32) {
        $gfrac = ($gfrac - 0.12) / 0.72
        if ($gfrac -lt 0) { $gfrac = 0 }
        if ($gfrac -gt 1) { $gfrac = 1 }
      }
      $v = [int][math]::Round(255 * $gfrac)
      $out[$o] = [byte]$v; $out[$o+1] = [byte]$v; $out[$o+2] = [byte]$v
      $out[$o+3] = [byte][math]::Round(255 * $cov)
    }
  }
  return $out
}

function Save-Png([byte[]]$rgba, [int]$size, [string]$path) {
  $b = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $r = New-Object System.Drawing.Rectangle 0, 0, $size, $size
  $d = $b.LockBits($r, [System.Drawing.Imaging.ImageLockMode]::WriteOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  [System.Runtime.InteropServices.Marshal]::Copy($rgba, 0, $d.Scan0, $rgba.Length)
  $b.UnlockBits($d)
  $b.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $b.Dispose()
}

# 方法 A：亮度阈值抠白底
function Render-A([int]$size) {
  $out = New-Object byte[] ($size * $size * 4)
  $scale = $size / [double]([math]::Max($sw, $sh))
  for ($oy = 0; $oy -lt $size; $oy++) {
    for ($ox = 0; $ox -lt $size; $ox++) {
      $sx = [int]($x0 + $ox / $scale); $sy = [int]($y0 + $oy / $scale)
      $o = ($oy * $size + $ox) * 4
      if ($sx -lt 0 -or $sx -ge $W -or $sy -lt 0 -or $sy -ge $H) { continue }
      $i = $sy * $W + $sx
      $l = $lum[$i]
      $out[$o] = $l; $out[$o+1] = $l; $out[$o+2] = $l
      $out[$o+3] = if ($l -ge 250) { [byte]0 } else { [byte]255 }
    }
  }
  return $out
}

# 方法 B：alpha 反相
function Render-B([int]$size) {
  $out = New-Object byte[] ($size * $size * 4)
  $scale = $size / [double]([math]::Max($sw, $sh))
  for ($oy = 0; $oy -lt $size; $oy++) {
    for ($ox = 0; $ox -lt $size; $ox++) {
      $sx = [int]($x0 + $ox / $scale); $sy = [int]($y0 + $oy / $scale)
      $o = ($oy * $size + $ox) * 4
      if ($sx -lt 0 -or $sx -ge $W -or $sy -lt 0 -or $sy -ge $H) { continue }
      $l = $lum[$sy * $W + $sx]
      $a = 255 - $l
      $out[$o] = 0; $out[$o+1] = 0; $out[$o+2] = 0
      $out[$o+3] = [byte]$a
    }
  }
  return $out
}

foreach ($s in $Sizes) {
  $rgba = Render-C $s
  Save-Png $rgba $s ("assets\icon-$s.png")
  # 供打包 ico 用的原始 RGBA
  [System.IO.File]::WriteAllBytes("assets\icon-$s.rgba", $rgba)
  "rendered $s"
}

# 对比图（512 缩放便于看圆角）
Save-Png (Render-A 512) 512 "docs\icon-methodA.png"
Save-Png (Render-B 512) 512 "docs\icon-methodB.png"
Save-Png (Render-C 512) 512 "docs\icon-methodC.png"
"radius=$radius bbox=($x0,$y0,$sw,$sh)"
