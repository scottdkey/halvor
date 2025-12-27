# Setup halvor agent service on Windows
# Uses Windows Task Scheduler to run as a user task (no admin required)

param(
    [string]$HalvorPath = "",
    [int]$Port = 13500,
    [string]$WebPort = ""
)

# Get halvor binary path
if ([string]::IsNullOrEmpty($HalvorPath)) {
    $HalvorPath = Get-Command halvor -ErrorAction SilentlyContinue
    if (-not $HalvorPath) {
        $HalvorPath = "$env:USERPROFILE\.cargo\bin\halvor.exe"
    } else {
        $HalvorPath = $HalvorPath.Source
    }
}

# Verify halvor exists
if (-not (Test-Path $HalvorPath)) {
    Write-Error "Halvor binary not found at: $HalvorPath"
    exit 1
}

# Get user directories
$HomeDir = $env:USERPROFILE
$ConfigDir = "$HomeDir\.config\halvor"
$PidFile = "$ConfigDir\halvor-agent.pid"
$WebDir = if ($env:HALVOR_WEB_DIR) { $env:HALVOR_WEB_DIR } else { "C:\opt\halvor\projects\web" }

# Create config directory
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

# Build command arguments
$Arguments = "agent start --port $Port"
if ($WebPort) {
    $Arguments += " --web-port $WebPort"
}
$Arguments += " --daemon"

# Task name
$TaskName = "HalvorAgent"

# Check if task already exists
$ExistingTask = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue

if ($ExistingTask) {
    Write-Host "Halvor agent task already exists. Updating..."
    
    # Stop the task if running
    Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    
    # Remove existing task
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
}

# Create the scheduled task action
$Action = New-ScheduledTaskAction -Execute $HalvorPath -Argument $Arguments -WorkingDirectory $HomeDir

# Create task settings
$Settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -RestartCount 3 `
    -RestartInterval (New-TimeSpan -Minutes 1)

# Create task principal (run as current user)
$Principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited

# Create trigger (at logon)
$Trigger = New-ScheduledTaskTrigger -AtLogOn

# Set environment variables
$TaskEnv = @{
    "HOME" = $HomeDir
    "HALVOR_DB_DIR" = $ConfigDir
    "HALVOR_WEB_DIR" = $WebDir
}

# Register the task
Register-ScheduledTask `
    -TaskName $TaskName `
    -Action $Action `
    -Settings $Settings `
    -Principal $Principal `
    -Trigger $Trigger `
    -Description "Halvor Agent - Secure cluster management service" `
    -Force | Out-Null

# Set environment variables for the task
$Task = Get-ScheduledTask -TaskName $TaskName
$Task.Actions[0].EnvironmentVariables = $TaskEnv
$Task | Set-ScheduledTask | Out-Null

# Start the task
Start-ScheduledTask -TaskName $TaskName

Write-Host "✓ Halvor agent service set up as scheduled task"
Write-Host "✓ Task name: $TaskName"
Write-Host "  Use 'Get-ScheduledTask -TaskName $TaskName' to check status"
Write-Host "  Use 'Get-ScheduledTaskInfo -TaskName $TaskName' to view details"

