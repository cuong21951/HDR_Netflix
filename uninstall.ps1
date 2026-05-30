$ErrorActionPreference = "Stop"

$startup = [Environment]::GetFolderPath("Startup")
$shortcutPath = Join-Path $startup "HDR Netflix.lnk"
if (Test-Path $shortcutPath) {
    Remove-Item -Force $shortcutPath
}
Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "HDR Netflix" -ErrorAction SilentlyContinue

$installDir = Join-Path $env:LOCALAPPDATA "HDRNetflix"
$exeNames = @("HDRNetflix.exe", "hdr-netflix.exe")
foreach ($exeName in $exeNames) {
    $exe = Join-Path $installDir $exeName
    if (Test-Path $exe) {
        Remove-Item -Force $exe
    }
}

Write-Host "Removed startup setting and installed executable."
Write-Host "Config is kept at: $(Join-Path $installDir 'config.toml')"
