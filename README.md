# Retreat Arena

A third-person arena shooter built with the [Retreat engine](https://github.com/RetreatSoftware/Retreat-Engine-3D)
and [Retreat modules](https://github.com/RetreatSoftware/Retreat-Modules). Early stage: foxes in their
own colors walk around the arena and shoot each other with a rifle and rockets, alone or over the local network. Asset
credits: [assets/CREDITS.md](assets/CREDITS.md).

All commands are PowerShell, run from this folder.

## Play alone

```powershell
cargo run
```

Click to turn with the mouse, Escape to release it. W A S D move, Shift sprints, Space jumps, Ctrl
dashes, Q swaps shoulder. Hold the left mouse button to shoot the rifle, the right one to fire
rockets; R reloads both. Five rifle hits kill, or one direct rocket and one rifle hit; you come
back after 3 seconds. Rockets fly, so aim ahead of someone running.

A match lasts three minutes; the clock is at the top. Hold Tab for the scoreboard. When the time is
up the winner is shown for eight seconds, then the next match starts. Alone there is nobody to shoot at.

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

## Play with a server and two clients

On Windows, double-click `play.bat` (or run `.\play.bat`): it builds, starts the server and two
client windows, and stops the server when both windows are closed. `-Port 5001` changes the port.

By hand:

1. Server (leave it running). It prints `listening on UDP port 5000 certificate-hash <hash>`; the hash
   changes every start.

```powershell
cargo run -- --headless --no-render --server 5000
```

2. First player, in a second terminal (replace `HASH`):

```powershell
cargo run -- --connect https://127.0.0.1:5000 --cert-hash HASH
```

3. Second player: the same command in a third terminal (a browser player uses `play-lan.bat`).

## Checks

Pictures are written to `target/snapshots/` (browser: `target/snapshots/web/`).

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
