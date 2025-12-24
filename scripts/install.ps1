# Cross-platform installation script for hal (PowerShell)
# Downloads and installs the pre-built binary from GitHub releases

$ErrorActionPreference = "Stop"

# GitHub repository (update if forked)
$GithubRepo = if ($env:GITHUB_REPO) { $env:GITHUB_REPO } else { "scottdkey/halvor" }
$GithubApi = "https://api.github.com/repos/$GithubRepo"

Write-Host "Installing halvor (Homelab Automation Layer)..." -ForegroundColor Cyan

# Detect OS and architecture
function Detect-Platform {
    $os = "windows"
    $arch = "amd64"
    
    if ($IsLinux -or ($PSVersionTable.Platform -eq "Unix" -and (uname -s) -like "Linux*")) {
        $os = "linux"
    } elseif ($IsMacOS -or ($PSVersionTable.Platform -eq "Unix" -and (uname -s) -like "Darwin*")) {
        $os = "darwin"
    }
    
    $machine = if ($env:PROCESSOR_ARCHITECTURE) { $env:PROCESSOR_ARCHITECTURE } else { "x86_64" }
    if ($machine -like "*ARM64*" -or $machine -like "*aarch64*") {
        $arch = "arm64"
    } elseif ($machine -like "*AMD64*" -or $machine -like "*x86_64*") {
        $arch = "amd64"
    }
    
    "$os-$arch"
}

# Get latest release version
function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "$GithubApi/releases/latest"
        $response.tag_name
    } catch {
        Write-Host "Warning: Could not fetch latest version, using 'latest'" -ForegroundColor Yellow
        "latest"
    }
}

# Download binary from GitHub releases
function Download-Binary {
    param(
        [string]$Version,
        [string]$Platform
    )
    
    $downloadUrl = $null
    
    if ($Version -eq "latest") {
        try {
            $release = Invoke-RestMethod -Uri "$GithubApi/releases/latest"
            $downloadUrl = ($release.assets | Where-Object { $_.name -like "halvor-*-$Platform.tar.gz" }).browser_download_url | Select-Object -First 1
        } catch {
            Write-Host "Error: Could not fetch latest release" -ForegroundColor Red
            exit 1
        }
    } else {
        try {
            $release = Invoke-RestMethod -Uri "$GithubApi/releases/tags/$Version"
            $downloadUrl = ($release.assets | Where-Object { $_.name -like "halvor-*-$Platform.tar.gz" }).browser_download_url | Select-Object -First 1
        } catch {
            Write-Host "Error: Could not fetch release $Version" -ForegroundColor Red
            exit 1
        }
    }
    
    if (-not $downloadUrl) {
        Write-Host "Error: Could not find download URL for platform $Platform" -ForegroundColor Red
        Write-Host "Available releases: https://github.com/$GithubRepo/releases" -ForegroundColor Yellow
        exit 1
    }
    
    Write-Host "Downloading from: $downloadUrl" -ForegroundColor Green
    
    $tempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_.FullName }
    $archivePath = Join-Path $tempDir "hal.tar.gz"
    
    try {
        Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
    } catch {
        Write-Host "Error: Failed to download binary" -ForegroundColor Red
        Remove-Item -Recurse -Force $tempDir
        exit 1
    }
    
    # Extract tar.gz (requires tar command, available in Windows 10+ and PowerShell 7+)
    Push-Location $tempDir
    tar -xzf $archivePath
    Pop-Location
    
    $binaryPath = Join-Path $tempDir "halvor.exe"
    if (-not (Test-Path $binaryPath)) {
        $binaryPath = Join-Path $tempDir "halvor"
    }
    
    $binaryPath
}

$Platform = Detect-Platform
$Version = if ($args[0]) { $args[0] } else { "latest" }

if ($Version -ne "latest" -and $Version -notlike "v*") {
    $Version = "v$Version"
}

if ($Version -eq "latest") {
    $Version = Get-LatestVersion
}

Write-Host "Platform: $Platform" -ForegroundColor Green
Write-Host "Version: $Version" -ForegroundColor Green

# Determine installation directory
$InstallDir = "$env:USERPROFILE\.local\bin"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$InstallPath = Join-Path $InstallDir "halvor.exe"

# Check if halvor already exists
if (Test-Path $InstallPath) {
    $response = Read-Host "halvor already exists at $InstallPath. Overwrite? (y/N)"
    if ($response -ne "y" -and $response -ne "Y") {
        Write-Host "Installation cancelled." -ForegroundColor Yellow
        exit 0
    }
    Remove-Item $InstallPath -Force
}

# Download and install
Write-Host "Downloading halvor binary..." -ForegroundColor Cyan
$BinaryPath = Download-Binary -Version $Version -Platform $Platform

# Move binary to install location
Move-Item -Path $BinaryPath -Destination $InstallPath -Force

# Cleanup
Remove-Item -Recurse -Force (Split-Path $BinaryPath)

Write-Host "✓ halvor installed to $InstallPath" -ForegroundColor Green

# Check if install directory is in PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host ""
    Write-Host "Warning: $InstallDir is not in your PATH" -ForegroundColor Yellow
    Write-Host "Add this directory to your PATH environment variable" -ForegroundColor Yellow
    Write-Host "Or run: `$env:Path += `";$InstallDir`"" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Installation complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Configure halvor: halvor config init" -ForegroundColor Yellow
Write-Host "     (This sets up the path to your .env file)" -ForegroundColor Gray
Write-Host "  2. Setup cluster: halvor k3s setup --primary <node> --nodes <node1>,<node2>" -ForegroundColor Yellow
Write-Host "  3. See all commands: halvor --help" -ForegroundColor Yellow
