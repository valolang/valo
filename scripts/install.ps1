# Valo Runtime Installer
$ErrorActionPreference = "Stop"

$ValoDir = Join-Path $env:USERPROFILE ".valo"
$BinDir = Join-Path $ValoDir "bin"

$Dirs = @(
    $BinDir,
    (Join-Path $ValoDir "cache"),
    (Join-Path $ValoDir "packages"),
    (Join-Path $ValoDir "toolchains"),
    (Join-Path $ValoDir "tmp")
)

Write-Host "[Valo] Creating runtime structure..." -ForegroundColor Cyan
foreach ($Dir in $Dirs) {
    if (-not (Test-Path $Dir)) {
        New-Item -ItemType Directory -Path $Dir -Force | Out-Null
    }
}

$OsArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
$Arch = switch ($OsArch) {
    "Arm64" { "x64" } # release.yml ainda não publica valo-windows-arm64.zip
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
    $ExtractDir = Join-Path $ValoDir "extract"

    if (Test-Path $ExtractDir) {
        Remove-Item $ExtractDir -Recurse -Force
    }

    Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $ZipFile
    Expand-Archive -Path $ZipFile -DestinationPath $ExtractDir -Force
    Remove-Item $ZipFile

    $ExtractedValoDir = Join-Path $ExtractDir "valo"
    $ExtractedExe = Join-Path $ExtractedValoDir "valo.exe"

    if (-not (Test-Path $ExtractedExe)) {
        Write-Error "[Valo] valo.exe not found in downloaded archive."
        exit 1
    }

    Copy-Item -Path (Join-Path $ExtractedValoDir "*") -Destination $BinDir -Recurse -Force
    Remove-Item $ExtractDir -Recurse -Force
} catch {
    Write-Error "[Valo] Failed to download or extract Valo binary. $_"
    exit 1
}

$UserPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
if (-not $UserPath) {
    $UserPath = ""
}

if (-not ($UserPath.Split(";") -contains $BinDir)) {
    Write-Host "[Valo] Adding $BinDir to PATH..." -ForegroundColor Cyan

    if ($UserPath.Length -eq 0) {
        $NewPath = $BinDir
    } else {
        $NewPath = "$UserPath;$BinDir"
    }

    [System.Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    $env:PATH = "$env:PATH;$BinDir"
} else {
    Write-Host "[Valo] $BinDir is already in PATH." -ForegroundColor Cyan
}

Write-Host "[Valo] Validating installation..." -ForegroundColor Cyan

try {
    $ValoExe = Join-Path $BinDir "valo.exe"
    $version = & $ValoExe version
    Write-Host "[Valo] Success! Installed $version" -ForegroundColor Green
} catch {
    Write-Error "[Valo] Installation failed validation. $_"
    exit 1
}
