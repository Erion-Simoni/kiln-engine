param([int]$Left, [int]$Top, [int]$Width, [int]$Height, [string]$Output)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
if ($Width -lt 100 -or $Height -lt 100) { throw 'Window client area is too small' }
$bitmap = New-Object System.Drawing.Bitmap($Width, $Height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
    $graphics.CopyFromScreen($Left, $Top, 0, 0, $bitmap.Size)
    $colors = New-Object 'System.Collections.Generic.HashSet[int]'
    for ($x = 10; $x -lt $Width - 10; $x += 8) {
        for ($y = 10; $y -lt $Height - 10; $y += 8) {
            [void]$colors.Add($bitmap.GetPixel($x, $y).ToArgb())
        }
    }
    $bitmap.Save($Output, [System.Drawing.Imaging.ImageFormat]::Png)
    if ($colors.Count -lt 12) { throw 'Window client capture appears blank' }
    Write-Output "Captured client area with $($colors.Count) sampled colors"
} finally {
    $graphics.Dispose()
    $bitmap.Dispose()
}
