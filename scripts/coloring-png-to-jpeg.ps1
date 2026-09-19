# Convert coloring PNG masters to 300 DPI JPEG for KDP wrap + interior.
# Usage: powershell -File coloring-png-to-jpeg.ps1 -ArtDir ... -OutDir ...
param(
  [Parameter(Mandatory = $true)][string]$ArtDir,
  [Parameter(Mandatory = $true)][string]$OutDir,
  [int]$CoverW = 2550,
  [int]$CoverH = 3375,
  [int]$PlateW = 1800,
  [int]$PlateH = 2400,
  [int]$Quality = 72,
  [int]$Pages = 130,
  [double]$SpineIn = 0.30511,
  [string]$SpineText = 'CLASSIC AMERICAN IRON'
)

Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() |
  Where-Object { $_.MimeType -eq 'image/jpeg' }
$enc = [System.Drawing.Imaging.Encoder]::Quality
$ep = New-Object System.Drawing.Imaging.EncoderParameters(1)
$ep.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter($enc, [int64]$Quality)

function Convert-Jpeg([string]$Src, [string]$Dst, [int]$W, [int]$H, [bool]$Fill, [string]$PadHex) {
  if (-not (Test-Path $Src)) { throw "missing $Src" }
  $srcTime = (Get-Item $Src).LastWriteTimeUtc
  if ((Test-Path $Dst) -and (Get-Item $Dst).LastWriteTimeUtc -ge $srcTime) {
    $exist = [System.Drawing.Image]::FromFile($Dst)
    $same = ($exist.Width -eq $W -and $exist.Height -eq $H)
    $exist.Dispose()
    if ($same) {
      Write-Host "skip $(Split-Path $Dst -Leaf)"
      return
    }
  }
  $img = [System.Drawing.Image]::FromFile($Src)
  try {
    $bmp = New-Object System.Drawing.Bitmap $W, $H
    $bmp.SetResolution(300, 300)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try {
      $pad = [System.Drawing.ColorTranslator]::FromHtml($PadHex)
      $g.Clear($pad)
      $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
      $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
      $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
      $scale = if ($Fill) {
        [Math]::Max($W / [double]$img.Width, $H / [double]$img.Height)
      } else {
        [Math]::Min($W / [double]$img.Width, $H / [double]$img.Height)
      }
      $dw = [int][Math]::Round($img.Width * $scale)
      $dh = [int][Math]::Round($img.Height * $scale)
      $dx = [int][Math]::Round(($W - $dw) / 2.0)
      $dy = [int][Math]::Round(($H - $dh) / 2.0)
      $g.DrawImage($img, $dx, $dy, $dw, $dh)
    } finally { $g.Dispose() }
    $tmp = "$Dst.__tmp.jpg"
    $bmp.Save($tmp, $codec, $ep)
    $bmp.Dispose()
    Move-Item -Force $tmp $Dst
    Write-Host "ok $(Split-Path $Dst -Leaf)"
  } finally { $img.Dispose() }
}

# Cover panels: fill/crop so 3:4 art does not grow white pillars on an 8.5x11 sheet.
Convert-Jpeg (Join-Path $ArtDir 'cover-front.png') (Join-Path $OutDir 'cover-front.jpg') $CoverW $CoverH $true '#1A120C'
Convert-Jpeg (Join-Path $ArtDir 'cover-back.png') (Join-Path $OutDir 'cover-back.jpg') $CoverW $CoverH $true '#1A120C'

# One-piece wrap JPEG: bleed + back + spine + front + bleed. Spine text is
# painted into the bitmap so the upload PDF has zero fonts.
function Build-WrapJpeg {
  $dpi = 300.0
  $bleedIn = 0.125
  $trimWIn = 8.5
  $wrapHIn = 11.25
  $wrapWIn = (2.0 * $bleedIn) + (2.0 * $trimWIn) + $SpineIn
  $W = [int][Math]::Round($wrapWIn * $dpi)
  $H = [int][Math]::Round($wrapHIn * $dpi)
  $bleedPx = [int][Math]::Round($bleedIn * $dpi)
  $trimWpx = [int][Math]::Round($trimWIn * $dpi)
  $spinePx = [int][Math]::Round($SpineIn * $dpi)
  $dst = Join-Path $OutDir 'cover-wrap.jpg'
  $front = Join-Path $OutDir 'cover-front.jpg'
  $back = Join-Path $OutDir 'cover-back.jpg'
  $need = $true
  if ((Test-Path $dst) -and (Test-Path $front) -and (Test-Path $back)) {
    $exist = [System.Drawing.Image]::FromFile($dst)
    $same = ($exist.Width -eq $W -and $exist.Height -eq $H)
    $exist.Dispose()
    $dstT = (Get-Item $dst).LastWriteTimeUtc
    $need = -not ($same -and $dstT -ge (Get-Item $front).LastWriteTimeUtc -and $dstT -ge (Get-Item $back).LastWriteTimeUtc)
  }
  if (-not $need) {
    Write-Host "skip cover-wrap.jpg"
    return
  }
  $backImg = [System.Drawing.Image]::FromFile($back)
  $frontImg = [System.Drawing.Image]::FromFile($front)
  try {
    $bmp = New-Object System.Drawing.Bitmap $W, $H
    $bmp.SetResolution(300, 300)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try {
      $g.Clear([System.Drawing.ColorTranslator]::FromHtml('#1A120C'))
      $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
      $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
      $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
      $g.DrawImage($backImg, 0, 0, $bleedPx + $trimWpx, $H)
      $spineX = $bleedPx + $trimWpx
      $g.FillRectangle((New-Object System.Drawing.SolidBrush ([System.Drawing.ColorTranslator]::FromHtml('#120C08'))), $spineX, 0, $spinePx, $H)
      $g.DrawImage($frontImg, $spineX + $spinePx, 0, $trimWpx + $bleedPx, $H)
      if ($SpineText) {
        $state = $g.Save()
        $g.TranslateTransform(($spineX + $spinePx / 2.0), ($H / 2.0))
        $g.RotateTransform(-90)
        $fs = [Math]::Max(18, [int]($spinePx * 0.42))
        $font = New-Object System.Drawing.Font 'Segoe UI', $fs, ([System.Drawing.FontStyle]::Bold), ([System.Drawing.GraphicsUnit]::Pixel)
        $brush = New-Object System.Drawing.SolidBrush ([System.Drawing.ColorTranslator]::FromHtml('#E8DCC8'))
        $sf = New-Object System.Drawing.StringFormat
        $sf.Alignment = [System.Drawing.StringAlignment]::Center
        $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
        $g.DrawString($SpineText, $font, $brush, 0, 0, $sf)
        $font.Dispose()
        $brush.Dispose()
        $sf.Dispose()
        $g.Restore($state)
      }
    } finally { $g.Dispose() }
    $wrapEp = New-Object System.Drawing.Imaging.EncoderParameters(1)
    $wrapEp.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter($enc, [int64]88)
    $tmp = "$dst.__tmp.jpg"
    $bmp.Save($tmp, $codec, $wrapEp)
    $bmp.Dispose()
    Move-Item -Force $tmp $dst
    Write-Host "ok cover-wrap.jpg ${W}x${H}"
  } finally {
    $backImg.Dispose()
    $frontImg.Dispose()
  }
}

Build-WrapJpeg

Get-ChildItem $ArtDir -Filter '*.png' | Where-Object {
  $_.Name -match '-(color|p[0-3])\.png$'
} | ForEach-Object {
  $dst = Join-Path $OutDir ($_.BaseName + '.jpg')
  Convert-Jpeg $_.FullName $dst $PlateW $PlateH $false '#FFFFFF'
}
