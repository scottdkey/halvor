# Install halvor CLI on Windows
# Equivalent to 'make install-cli' on Unix systems

param(
    [switch]$SkipService
)

$ErrorActionPreference = "Stop"

Write-Host "Building and installing CLI to system..."

# Stop agent service if running
$TaskName = "HalvorAgent"
$Task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
if ($Task) {
    $TaskInfo = Get-ScheduledTaskInfo -TaskName $TaskName -ErrorAction SilentlyContinue
    if ($TaskInfo -and $TaskInfo.State -eq "Running") {
        Write-Host "Stopping halvor agent service..."
        Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 1
    }
}

# Build the CLI
Write-Host "Building halvor CLI..."
$CargoPath = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $CargoPath) {
    Write-Error "Cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
}

Push-Location $PSScriptRoot\..
try {
    cargo build --release --bin halvor --manifest-path crates/halvor-cli/Cargo.toml
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Build failed"
        exit 1
    }
} finally {
    Pop-Location
}

# Install to cargo bin directory
$CargoBin = "$env:USERPROFILE\.cargo\bin"
New-Item -ItemType Directory -Force -Path $CargoBin | Out-Null

$Source = "target\release\halvor.exe"
$Dest = "$CargoBin\halvor.exe"

if (Test-Path $Source) {
    Copy-Item -Path $Source -Destination $Dest -Force
    Write-Host "✓ CLI installed to $Dest (available as 'halvor')"
} else {
    Write-Error "Build artifact not found: $Source"
    exit 1
}

# Restart agent service if it exists
if (-not $SkipService) {
    if ($Task) {
        Write-Host "Restarting halvor agent service..."
        $ScriptPath = Join-Path $PSScriptRoot "setup-agent-service.ps1"
        & $ScriptPath -HalvorPath $Dest
        Write-Host "✓ Agent service restarted"
    } else {
        Write-Host "No agent service found. Set up with:"
        Write-Host "  .\scripts\setup-agent-service.ps1"
    }
}

Write-Host ""
Write-Host "✓ Installation complete!"
Write-Host "  Add $CargoBin to your PATH if not already there"

