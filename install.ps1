# install.ps1 -- Timeline DSL CLI installer for Windows
# Usage: irm https://raw.githubusercontent.com/keroway/timeline-dsl/main/install.ps1 | iex
# Set $env:TDSL_VERSION to install a specific version (default: latest)
#
# Requires PowerShell 5.1 or later.

$ErrorActionPreference = "Stop"

$REPO = "keroway/timeline-dsl"
$BIN_NAME = "tdsl.exe"
$ARCHIVE_NAME = "tdsl-windows-x86_64.zip"
$INSTALL_DIR = "$env:USERPROFILE\.local\bin"

function Resolve-Version {
    if ($env:TDSL_VERSION) {
        return $env:TDSL_VERSION
    }

    $apiUrl = "https://api.github.com/repos/$REPO/releases/latest"
    try {
        $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "tdsl-installer" }
        return $release.tag_name
    } catch {
        Write-Error "Failed to fetch latest release version from GitHub: $_"
        exit 1
    }
}

function Main {
    $version = Resolve-Version
    $downloadUrl = "https://github.com/$REPO/releases/download/$version/$ARCHIVE_NAME"

    Write-Host "Installing tdsl $version ($ARCHIVE_NAME)..."

    # Create install directory
    if (-not (Test-Path $INSTALL_DIR)) {
        New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null
    }

    # Download archive to temp file
    $tmpZip = [System.IO.Path]::GetTempFileName() + ".zip"
    $tmpDir = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        Write-Host "Downloading from $downloadUrl ..."
        Invoke-WebRequest -Uri $downloadUrl -OutFile $tmpZip -UseBasicParsing

        Expand-Archive -Path $tmpZip -DestinationPath $tmpDir -Force

        $srcBin = Join-Path $tmpDir $BIN_NAME
        $destBin = Join-Path $INSTALL_DIR $BIN_NAME
        Copy-Item -Path $srcBin -Destination $destBin -Force

        Write-Host "Installed tdsl to $destBin"
    } finally {
        Remove-Item -Path $tmpZip -ErrorAction SilentlyContinue
        Remove-Item -Path $tmpDir -Recurse -ErrorAction SilentlyContinue
    }

    # Warn if INSTALL_DIR is not in PATH
    $currentPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
    if ($currentPath -notlike "*$INSTALL_DIR*") {
        Write-Host ""
        Write-Host "NOTE: $INSTALL_DIR is not in your PATH."
        Write-Host "Run the following command to add it permanently:"
        Write-Host ""
        Write-Host "  `$env:PATH += `";$INSTALL_DIR`""
        Write-Host "  [System.Environment]::SetEnvironmentVariable('PATH', `$env:PATH, 'User')"
        Write-Host ""
    }
}

Main
