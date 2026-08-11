[CmdletBinding()]
param(
  [string]$Source,
  [string]$Output,
  [string]$UiOutput
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if ([string]::IsNullOrWhiteSpace($Source)) {
  $Source = Join-Path $scriptRoot "..\src\assets\providers\claude-code.png"
}
if ([string]::IsNullOrWhiteSpace($Output)) {
  $Output = Join-Path $scriptRoot "..\src\assets\brand\ccsm-app-icon.png"
}
if ([string]::IsNullOrWhiteSpace($UiOutput)) {
  $UiOutput = Join-Path $scriptRoot "..\src\assets\brand\ccsm-app-icon-ui.png"
}

function New-RoundedRectanglePath {
  param(
    [Parameter(Mandatory)]
    [System.Drawing.RectangleF]$Rectangle,
    [Parameter(Mandatory)]
    [float]$Radius
  )

  $diameter = $Radius * 2
  $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
  $path.AddArc($Rectangle.X, $Rectangle.Y, $diameter, $diameter, 180, 90)
  $path.AddArc($Rectangle.Right - $diameter, $Rectangle.Y, $diameter, $diameter, 270, 90)
  $path.AddArc(
    $Rectangle.Right - $diameter,
    $Rectangle.Bottom - $diameter,
    $diameter,
    $diameter,
    0,
    90
  )
  $path.AddArc($Rectangle.X, $Rectangle.Bottom - $diameter, $diameter, $diameter, 90, 90)
  $path.CloseFigure()
  return $path
}

$sourcePath = (Resolve-Path -LiteralPath $Source).Path
$outputPath = [System.IO.Path]::GetFullPath($Output)
$uiOutputPath = [System.IO.Path]::GetFullPath($UiOutput)
$outputDirectory = [System.IO.Path]::GetDirectoryName($outputPath)
$uiOutputDirectory = [System.IO.Path]::GetDirectoryName($uiOutputPath)
if ([string]::IsNullOrWhiteSpace($outputDirectory)) {
  throw "Unable to resolve the output directory."
}
if ([string]::IsNullOrWhiteSpace($uiOutputDirectory)) {
  throw "Unable to resolve the UI output directory."
}
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $uiOutputDirectory -Force | Out-Null

$canvasSize = 1024
$sourceBitmap = $null
$outputBitmap = $null
$graphics = $null
$uiBitmap = $null
$uiGraphics = $null
$shadowPath = $null
$badgePath = $null
$shadowBrush = $null
$badgeBrush = $null
$badgeBorder = $null
$promptPen = $null

try {
  $sourceBitmap = [System.Drawing.Bitmap]::FromFile($sourcePath)
  $outputBitmap = [System.Drawing.Bitmap]::new(
    $canvasSize,
    $canvasSize,
    [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
  )
  $graphics = [System.Drawing.Graphics]::FromImage($outputBitmap)
  $graphics.Clear([System.Drawing.Color]::Transparent)
  $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
  $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $graphics.DrawImage(
    $sourceBitmap,
    [System.Drawing.Rectangle]::new(0, 0, $canvasSize, $canvasSize)
  )

  $shadowRectangle = [System.Drawing.RectangleF]::new(654, 662, 320, 320)
  $shadowPath = New-RoundedRectanglePath -Rectangle $shadowRectangle -Radius 82
  $shadowBrush = [System.Drawing.SolidBrush]::new(
    [System.Drawing.Color]::FromArgb(72, 0, 0, 0)
  )
  $graphics.FillPath($shadowBrush, $shadowPath)

  $badgeRectangle = [System.Drawing.RectangleF]::new(634, 634, 320, 320)
  $badgePath = New-RoundedRectanglePath -Rectangle $badgeRectangle -Radius 82
  $badgeBrush = [System.Drawing.SolidBrush]::new(
    [System.Drawing.ColorTranslator]::FromHtml("#172026")
  )
  $badgeBorder = [System.Drawing.Pen]::new(
    [System.Drawing.ColorTranslator]::FromHtml("#FEFCFB"),
    18
  )
  $badgeBorder.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
  $graphics.FillPath($badgeBrush, $badgePath)
  $graphics.DrawPath($badgeBorder, $badgePath)

  $promptPen = [System.Drawing.Pen]::new(
    [System.Drawing.ColorTranslator]::FromHtml("#FEFCFB"),
    27
  )
  $promptPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
  $promptPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
  $promptPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
  $graphics.DrawLines(
    $promptPen,
    [System.Drawing.PointF[]]@(
      [System.Drawing.PointF]::new(714, 728),
      [System.Drawing.PointF]::new(782, 795),
      [System.Drawing.PointF]::new(714, 862)
    )
  )
  $graphics.DrawLine($promptPen, 814, 862, 894, 862)

  $outputBitmap.Save($outputPath, [System.Drawing.Imaging.ImageFormat]::Png)

  $uiBitmap = [System.Drawing.Bitmap]::new(
    64,
    64,
    [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
  )
  $uiGraphics = [System.Drawing.Graphics]::FromImage($uiBitmap)
  $uiGraphics.Clear([System.Drawing.Color]::Transparent)
  $uiGraphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
  $uiGraphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $uiGraphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $uiGraphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $uiGraphics.DrawImage($outputBitmap, [System.Drawing.Rectangle]::new(0, 0, 64, 64))
  $uiBitmap.Save($uiOutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
  foreach ($resource in @(
    $uiGraphics,
    $uiBitmap,
    $promptPen,
    $badgeBorder,
    $badgeBrush,
    $shadowBrush,
    $badgePath,
    $shadowPath,
    $graphics,
    $outputBitmap,
    $sourceBitmap
  )) {
    if ($null -ne $resource) {
      $resource.Dispose()
    }
  }
}

Write-Output $outputPath
Write-Output $uiOutputPath
