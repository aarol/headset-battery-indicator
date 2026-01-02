# Requires ImageMagick installed and 'magick' available in PATH
# Assumes the presence of directories arctis1x, arctis2x, arctis4x with PNG files containing the icons at different resolutions
$iconOutputDir = $PSScriptRoot

# Create output directory if it doesn't exist
if (!(Test-Path $iconOutputDir)) {
  New-Item -ItemType Directory -Path $iconOutputDir | Out-Null
}

$allPngs = Get-ChildItem .\arctis1x\ | Select-Object -ExpandProperty Name

foreach ($name in $allPngs) {
  # Remove .png from the name
  $basename = [System.IO.Path]::GetFileNameWithoutExtension($name)
  $iconPath = Join-Path $iconOutputDir "$basename.ico"
  # Combine PNGs into a single ICO file
  magick "arctis1x/$name" "arctis2x/$name" "arctis4x/$name" "$iconPath"
}