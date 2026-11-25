# Retreat Arena

A third-person arena shooter built with the [Retreat engine](https://github.com/RetreatSoftware/Retreat-Engine-3D)
and [Retreat modules](https://github.com/RetreatSoftware/Retreat-Modules). Early stage: foxes in their
own colors walk around the arena, alone or over the local network. There are no weapons yet. Asset
credits: [assets/CREDITS.md](assets/CREDITS.md).

All commands are PowerShell, run from this folder.

## Play alone

```powershell
cargo run
```

Click to turn with the mouse, Escape to release it. W A S D move, Shift sprints, Space jumps, Ctrl
dashes, Q swaps shoulder.

## Play with a server and two clients

1. Server (leave it running). It prints `listening on UDP port 5000 certificate-hash <hash>`; the hash
   changes every start.

```powershell
cargo run -- --headless --no-render --server 5000
```

2. First player, in a second terminal (replace `HASH`):

```powershell
cargo run -- --connect https://127.0.0.1:5000 --cert-hash HASH
```

3. Second player: the same command in a third terminal, or in the browser (Chrome or Edge). Build once
   (needs `rustup target add wasm32-unknown-unknown` and `wasm-bindgen-cli`):

```powershell
node scripts/web/build.mjs
```

```powershell
python -m http.server 8090 --directory target/web
```

Then open `http://localhost:8090/?connect=https://127.0.0.1:5000&cert_hash=HASH`. Without the part
after `?`, the page plays alone.

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
cargo run -- --headless --no-render --commands scripts/verify/simulation/health_match.cmds
```

```powershell
node scripts/verify/network/run.mjs
```

```powershell
node scripts/web/run_in_browser.mjs scripts/verify/walk.cmds
```
