$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $MyInvocation.MyCommand.Path
$dist = Join-Path $repo "dist"
$zip = Join-Path $repo "HDRNetflix-windows.zip"

Push-Location $repo
try {
    cargo build --release --locked
} finally {
    Pop-Location
}

if (Test-Path $dist) {
    Remove-Item -Recurse -Force $dist
}
New-Item -ItemType Directory -Force -Path $dist | Out-Null

Copy-Item -Force (Join-Path $repo "target\release\HDRNetflix.exe") (Join-Path $dist "HDRNetflix.exe")
Copy-Item -Force (Join-Path $repo "README.md") (Join-Path $dist "README.md")
Copy-Item -Force (Join-Path $repo "uninstall.ps1") (Join-Path $dist "uninstall.ps1")

if (Test-Path $zip) {
    Remove-Item -Force $zip
}
Compress-Archive -Path (Join-Path $dist "*") -DestinationPath $zip

Write-Host "Created:"
Write-Host "  $dist"
Write-Host "  $zip"
