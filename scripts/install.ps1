# Valo Runtime Installer
$ErrorActionPreference = "Stop"

$ValoDir = Join-Path $env:USERPROFILE ".valo"
$BinDir = Join-Path $ValoDir "bin"

Write-Host "[Valo] Creating runtime structure..." -ForegroundColor Cyan
New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $ValoDir "cache") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $ValoDir "packages") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $ValoDir "toolchains") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $ValoDir "tmp") -Force | Out-Null

$Arch = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
$Url = "https://github.com/valolang/valo/releases/latest/download/valo-windows-$Arch.exe"

Write-Host "[Valo] Downloading Valo from $Url..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $Url -OutFile (Join-Path $BinDir "valo.exe")

$Path = [System.Environment]::GetEnvironmentVariable("PATH", "User")
if (-not $Path.Contains($BinDir)) {
    Write-Host "[Valo] Adding $BinDir to PATH..." -ForegroundColor Cyan
    [System.Environment]::SetEnvironmentVariable("PATH", "$Path;$BinDir", "User")
    $env:PATH = "$env:PATH;$BinDir"
}

Write-Host "[Valo] Validating installation..." -ForegroundColor Cyan
try {
    $version = & (Join-Path $BinDir "valo.exe") version
    Write-Host "[Valo] Success! Installed $version" -ForegroundColor Green
} catch {
    Write-Error "[Valo] Installation failed."
}
