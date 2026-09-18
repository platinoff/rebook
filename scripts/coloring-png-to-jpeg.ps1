# Convert coloring PNG masters to 300 DPI JPEG for KDP wrap + interior.
# Usage: powershell -File coloring-png-to-jpeg.ps1 -ArtDir ... -OutDir ...
param(
  [Parameter(Mandatory = $true)][string]$ArtDir,
  [Parameter(Mandatory = $true)][string]$OutDir,
  [int]$CoverW = 2550,
  [int]$CoverH = 3375,
  [int]$PlateW = 2400,
  [int]$PlateH = 3200,
  [int]$Quality = 92
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
    Write-Host "skip $(Split-Path $Dst -Leaf)"
    return
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

Get-ChildItem $ArtDir -Filter '*.png' | Where-Object {
  $_.Name -match '-(color|p[0-3])\.png$'
} | ForEach-Object {
  $dst = Join-Path $OutDir ($_.BaseName + '.jpg')
  Convert-Jpeg $_.FullName $dst $PlateW $PlateH $false '#FFFFFF'
}
