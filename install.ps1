param(
    [switch]$NoStartup
)

$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $repo
try {
    cargo build --release
} finally {
    Pop-Location
}

$installDir = Join-Path $env:LOCALAPPDATA "HDRNetflix"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null

$exeSource = Join-Path $repo "target\release\HDRNetflix.exe"
$exeDest = Join-Path $installDir "HDRNetflix.exe"
Copy-Item -Force $exeSource $exeDest

$oldExe = Join-Path $installDir "hdr-netflix.exe"
Remove-Item -Force $oldExe -ErrorAction SilentlyContinue

if (-not $NoStartup) {
    $runKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    Set-ItemProperty -Path $runKey -Name "HDR Netflix" -Value "`"$exeDest`""
} else {
    Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "HDR Netflix" -ErrorAction SilentlyContinue
}

$startup = [Environment]::GetFolderPath("Startup")
$shortcutPath = Join-Path $startup "HDR Netflix.lnk"
Remove-Item -Force $shortcutPath -ErrorAction SilentlyContinue

Write-Host "Installed: $exeDest"
if (-not $NoStartup) {
    Write-Host "Start with Windows enabled."
} else {
    Write-Host "Start with Windows disabled."
}
Write-Host "Run now: $exeDest"
