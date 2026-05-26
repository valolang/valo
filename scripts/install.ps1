# Valo Runtime Installer
$ErrorActionPreference = "Stop"

$ValoDir = Join-Path $env:USERPROFILE ".valo"
$BinDir = Join-Path $ValoDir "bin"
$Dirs = @(
    $BinDir,
    Join-Path $ValoDir "cache",
    Join-Path $ValoDir "packages",
    Join-Path $ValoDir "toolchains",
    Join-Path $ValoDir "tmp"
)

Write-Host "[Valo] Creating runtime structure..." -ForegroundColor Cyan
foreach ($Dir in $Dirs) {
    if (-not (Test-Path $Dir)) {
        New-Item -ItemType Directory -Path $Dir -Force | Out-Null
    }
}

$Arch = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }
$Url = "https://github.com/valolang/valo/releases/latest/download/valo-windows-$Arch.exe"

Write-Host "[Valo] Downloading Valo from $Url..." -ForegroundColor Cyan
try {
    Invoke-WebRequest -Uri $Url -OutFile (Join-Path $BinDir "valo.exe")
} catch {
    Write-Error "[Valo] Failed to download Valo binary."
    exit 1
}

$UserPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
if (-not $UserPath.Contains($BinDir)) {
    Write-Host "[Valo] Adding $BinDir to PATH..." -ForegroundColor Cyan
    $NewPath = "$UserPath;$BinDir"
    [System.Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    $env:PATH = "$env:PATH;$BinDir"
} else {
    Write-Host "[Valo] $BinDir is already in PATH." -ForegroundColor Cyan
}

Write-Host "[Valo] Validating installation..." -ForegroundColor Cyan
try {
    $version = & (Join-Path $BinDir "valo.exe") version
    Write-Host "[Valo] Success! Installed $version" -ForegroundColor Green
} catch {
    Write-Error "[Valo] Installation failed validation."
    exit 1
}
