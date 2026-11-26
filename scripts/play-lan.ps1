# LAN play: builds the browser version and the game, then runs a server that serves the web page over
# HTTPS to the local network. Players open the printed address in Chrome or Edge. Ctrl+C stops the
# server. Run through play-lan.bat.
# -Port changes the game's UDP port, -WebPort the page's TCP port, -LanAddress the printed address.
param([int]$Port = 5000, [int]$WebPort = 8443, [string]$LanAddress = '')

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root 'target\debug\retreat_arena.exe'

Push-Location $root
try {
    Write-Host 'play-lan: building the browser version (the first time takes several minutes)...'
    node scripts/web/build.mjs
    if ($LASTEXITCODE -ne 0) { throw "play-lan: the web build failed; see the errors above (it needs Node, 'rustup target add wasm32-unknown-unknown' and wasm-bindgen-cli)." }
    cargo build
    if ($LASTEXITCODE -ne 0) { throw "play-lan: 'cargo build' failed; see the errors above." }

    Write-Host ''
    Write-Host 'play-lan: starting the server. When it prints "players on this network open https://...":'
    Write-Host '  1. If Windows asks about the firewall, allow the app on Private networks.'
    Write-Host '  2. On each device, open that address in Chrome or Edge.'
    Write-Host '  3. The browser warns about the certificate (this server made it): Advanced, then Proceed.'
    Write-Host '     The warning comes back after every server restart.'
    Write-Host '  Ctrl+C here stops the server.'
    Write-Host ''

    $arguments = @('--headless', '--no-render', '--server', $Port, '--serve-web', 'target/web', '--web-port', $WebPort)
    if ($LanAddress) { $arguments += @('--lan-address', $LanAddress) }
    # The executable is started directly, so it is told where the game's `assets` folder is.
    $env:BEVY_ASSET_ROOT = $root
    & $exe @arguments
    if ($LASTEXITCODE -ne 0) { throw "play-lan: the server stopped with an error (is port $Port or $WebPort in use?)." }
} finally {
    Pop-Location
}
