Add-Type -AssemblyName System.Drawing

$dir = "src-tauri\icons"
if (-not (Test-Path $dir)) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
}

function Create-AppIcon {
    param (
        [int]$size,
        [string]$outputPath
    )

    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)

    $rect = New-Object System.Drawing.Rectangle(2, 2, ($size - 4), ($size - 4))
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, [System.Drawing.Color]::FromArgb(255, 0, 240, 255), [System.Drawing.Color]::FromArgb(255, 16, 185, 129), 45)
    $g.FillEllipse($brush, $rect)

    $innerMargin = [int]($size * 0.15)
    $innerSize = [int]($size * 0.7)
    $innerRect = New-Object System.Drawing.Rectangle($innerMargin, $innerMargin, $innerSize, $innerSize)
    $darkBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 11, 15, 23))
    $g.FillEllipse($darkBrush, $innerRect)

    $pen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 0, 240, 255), [float]($size * 0.08))
    $g.DrawLine($pen, [float]($size * 0.3), [float]($size * 0.5), [float]($size * 0.45), [float]($size * 0.35))
    $g.DrawLine($pen, [float]($size * 0.45), [float]($size * 0.35), [float]($size * 0.55), [float]($size * 0.65))
    $g.DrawLine($pen, [float]($size * 0.55), [float]($size * 0.65), [float]($size * 0.7), [float]($size * 0.5))

    $bmp.Save($outputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose()
    $bmp.Dispose()
}

Create-AppIcon -size 32 -outputPath "src-tauri\icons\32x32.png"
Create-AppIcon -size 128 -outputPath "src-tauri\icons\128x128.png"
Create-AppIcon -size 256 -outputPath "src-tauri\icons\128x128@2x.png"

# Save as .ico
$bmp256 = [System.Drawing.Bitmap]::FromFile((Resolve-Path "src-tauri\icons\128x128@2x.png"))
$hIcon = $bmp256.GetHicon()
$ico = [System.Drawing.Icon]::FromHandle($hIcon)
$fs = New-Object System.IO.FileStream("src-tauri\icons\icon.ico", [System.IO.FileMode]::Create)
$ico.Save($fs)
$fs.Close()
$ico.Dispose()
$bmp256.Dispose()

# Create public dir if missing and copy logo
if (-not (Test-Path "public")) { New-Item -ItemType Directory -Force -Path "public" | Out-Null }
Copy-Item "src-tauri\icons\32x32.png" "public\logo.svg" -Force

Write-Host "Icons successfully generated!"
Get-ChildItem "src-tauri\icons"
