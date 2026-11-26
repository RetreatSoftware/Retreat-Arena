# Local play: builds the game, starts a server in the background and two client windows.
# Closing both client windows (or Ctrl+C here) stops the server. Run through play.bat.
# -Port changes the server's UDP port.
param([int]$Port = 5000)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root 'target\debug\retreat_arena.exe'
$logs = Join-Path $root 'target\play'
$serverLog = Join-Path $logs 'server.log'
$serverErrors = Join-Path $logs 'server.errors.log'

Push-Location $root
try {
    cargo build
    if ($LASTEXITCODE -ne 0) { throw "play: 'cargo build' failed; see the errors above." }
} finally {
    Pop-Location
}

New-Item -ItemType Directory -Force $logs | Out-Null
# The executable is started directly, so it is told where the game's `assets` folder is.
$env:BEVY_ASSET_ROOT = $root

$processes = @()
try {
    $server = Start-Process $exe -ArgumentList '--headless', '--no-render', '--server', $Port `
        -WorkingDirectory $root -NoNewWindow -PassThru `
        -RedirectStandardOutput $serverLog -RedirectStandardError $serverErrors
    $processes += $server

    # The certificate hash changes every start; the server prints it when it listens.
    $hash = $null
    $deadline = (Get-Date).AddSeconds(60)
    while (-not $hash) {
        if ($server.HasExited -or (Get-Date) -gt $deadline) {
            throw "play: the server did not start listening on port $Port (is an old server still running?). Logs: $serverLog and $serverErrors"
        }
        $line = Select-String -Path $serverLog, $serverErrors -Pattern 'certificate-hash ([0-9a-f]{64})' | Select-Object -First 1
        if ($line) { $hash = $line.Matches[0].Groups[1].Value } else { Start-Sleep -Milliseconds 200 }
    }
    Write-Host "play: server running on port $Port (log: $serverLog)"

    $clients = foreach ($number in 1, 2) {
        Start-Process $exe -ArgumentList '--connect', "https://127.0.0.1:$Port", '--cert-hash', $hash `
            -WorkingDirectory $root -NoNewWindow -PassThru
    }
    $processes += $clients
    Write-Host 'play: two clients started. Close both windows to stop the server.'
    $clients | Wait-Process
} finally {
    $processes | Where-Object { -not $_.HasExited } | Stop-Process -Force
}
