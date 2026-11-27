# Retreat Arena

A third-person arena shooter built with the [Retreat engine](https://github.com/RetreatSoftware/Retreat-Engine-3D)
and [Retreat modules](https://github.com/RetreatSoftware/Retreat-Modules): two to eight foxes in their
own colors fight timed free-for-all matches with a rifle and rockets, with sound, on one computer or
over the local network. Asset credits: [assets/CREDITS.md](assets/CREDITS.md).

All commands are PowerShell, run from this folder.

## How to play

| | |
|---|---|
| Mouse | look (the first click takes the mouse, Escape releases it) |
| W A S D | move; Shift sprint, Space jump, Ctrl dash |
| Left mouse (hold) | rifle |
| Right mouse (hold) | rockets (they fly, so aim ahead of someone running) |
| R | reload (from the reserve shown as `+n`) |
| E | use a health kit (restores 50) |
| Q | swap shoulder |
| Tab (hold) | scoreboard |

Five rifle hits kill, or one direct rocket and one rifle hit; you come back after 3 seconds. A match
lasts three minutes; the winner is shown for eight seconds, then the next match starts. Every life
starts with the rifle, 60 spare rounds and one health kit. Walk over the glowing balls on the map to
pick things up: green a health kit, yellow rifle rounds, orange rockets, violet (on the high
platform) the rocket launcher with rockets. They come back after a while. The dead drop what they
carried (white) for 20 seconds. Nothing is kept after a death or a match.

## Play alone

```powershell
cargo run
```

There is nobody to shoot at: it is for looking around.

## Play on this computer (a server and two windows)

```powershell
.\play.bat
```

Closing both windows stops the server.

## Play on the local network (browsers)

```powershell
.\play-lan.bat
```

It builds the web version (the first time takes several minutes; needs Node,
`rustup target add wasm32-unknown-unknown` and `wasm-bindgen-cli`), then runs the server, which
serves the game's page. Then:

1. If Windows asks about the firewall, allow the app on **Private networks** (other devices need TCP
   port 8443 and UDP port 5000).
2. On each device on the same network, open the address printed after `players on this network open`
   (for example `https://192.168.1.20:8443/`) in Chrome or Edge. On this computer
   `https://localhost:8443/` also works.
3. The browser warns that the certificate is not trusted (the server made it): choose **Advanced**,
   then **Proceed**. Nothing is installed. The warning comes back after every server restart.

The page joins by itself. Ctrl+C stops the server. After changing the game, run `play-lan.bat` again.
If the printed address is wrong (a VPN), pass the computer's address:

```powershell
.\play-lan.bat -LanAddress 192.168.1.20
```

## Start the parts by hand

1. Server (leave it running). It prints `listening on UDP port 5000 certificate-hash <hash>`; the
   hash changes every start.

```powershell
cargo run -- --headless --no-render --server 5000
```

2. Each player, in another terminal (replace `HASH`):

```powershell
cargo run -- --connect https://127.0.0.1:5000 --cert-hash HASH
```

## Checks

Pictures are written to `target/snapshots/` (browser: `target/snapshots/web/`). `cargo test`
includes the preset check `arena_preset_full_match`: a whole match over a lossy link.

```powershell
cargo run -- --headless --commands scripts/verify/walk.cmds
```

```powershell
cargo run -- --headless --commands scripts/verify/arena_map.cmds
```

```powershell
cargo run -- --headless --commands scripts/verify/visuals.cmds
```

```powershell
cargo run -- --headless --commands scripts/verify/colors.cmds
```

```powershell
cargo run -- --headless --commands scripts/verify/combat.cmds
```

```powershell
cargo run -- --headless --commands scripts/verify/wall.cmds
```

```powershell
cargo run -- --headless --commands scripts/verify/rockets.cmds
```

```powershell
cargo run -- --headless --commands scripts/verify/match.cmds
```

```powershell
cargo run -- --headless --commands scripts/verify/inventory.cmds
```

```powershell
cargo run -- --headless --commands scripts/verify/loot.cmds
```

```powershell
cargo run -- --headless --no-render --commands scripts/verify/simulation/sounds.cmds
```

```powershell
cargo run -- --headless --no-render --commands scripts/verify/simulation/health_match.cmds
```

```powershell
node scripts/verify/network/run.mjs
```

```powershell
node scripts/web/build.mjs
```

```powershell
node scripts/web/run_in_browser.mjs scripts/verify/walk.cmds
```

```powershell
node scripts/verify/network/run.mjs --served-page
```
