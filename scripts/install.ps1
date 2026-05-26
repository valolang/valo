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

$OsArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
$Arch = switch ($OsArch) {
    "Arm64" { "arm64" }
    "X64"   { "x64" }
    "X86"   { "x86" }
    Default { "x64" }
}
$Releases = Invoke-RestMethod -Uri "https://api.github.com/repos/valolang/valo/releases"
$LatestTag = $Releases[0].tag_name
$Url = "https://github.com/valolang/valo/releases/download/$LatestTag/valo-windows-$Arch.zip"

Write-Host "[Valo] Downloading Valo from $Url..." -ForegroundColor Cyan
try {
    $ZipFile = Join-Path $ValoDir "valo.zip"
    Invoke-WebRequest -Uri $Url -OutFile $ZipFile
    Expand-Archive -Path $ZipFile -DestinationPath $BinDir -Force
    Remove-Item $ZipFile
} catch {
    Write-Error "[Valo] Failed to download or extract Valo binary."
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
