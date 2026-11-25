# Retreat-Arena

Third-person arena shooter, the first real Retreat game; later Forge's project template. Plan:
`../ARENA_PLAN.md`.

This file contains project facts and rules only. Personal preferences belong in `CLAUDE.local.md`.
The root `../CLAUDE.md` also applies here.

## Rules

- This repository only **selects modules, chooses the map and holds configuration** (including view and
  light setup until a module owns them). Gameplay mechanics go in `../Retreat-Modules`; capabilities in
  `../Retreat-Engine-3D`. Never add gameplay rules or `cfg(target_arch = "wasm32")` code here.
- Engine and modules are git dependencies; `Cargo.lock` pins the commits. The only engine crates used
  are `retreat_engine` (`launch`, `dispatch_command_line`, and `CommandScript` / `NetServer` /
  `NetClient` to know whether someone is just playing; tests also the simulation plugins and the
  network harness) and `retreat_engine_api`.
- A module is selected by adding its dependency in `Cargo.toml` and its plugin in
  `src/arena_game.rs::add_arena_plugins`. Its numbers are overridden there by inserting its config
  resource before the plugin.
- Do not modify sibling repositories from this one without being asked.

## Layout

- `src/main.rs` calls `retreat_engine::launch`.
- `assets/`: the game's files, each third-party one credited in `assets/CREDITS.md`. The fox is the
  character until real models exist.
- `src/arena_game.rs`: module selection, `MAP_ID`, the character model and its clips, one flat color
  per player, the rifle, health and HUD numbers, camera, sky and lights. A game started with no
  script and no network spawns the solo player (id 0). One directional light only: the browser draws
  no second one.
- `src/net_play_tests.rs`: server and clients in one process (this binary crate has no `tests/`).
- `scripts/verify/*.cmds`: one script per check that writes pictures. Run from the repository root and
  open every PNG in `target/snapshots/`:
  `cargo run -- --headless --commands scripts/verify/arena_map.cmds`
- `scripts/verify/network/`: `node scripts/verify/network/run.mjs [--browser] [--software-renderer]`
  after `cargo build` (and `node scripts/web/build.mjs` for `--browser`). Client A moves, client B
  renders, watches, shoots A dead and sees the respawn. B turns its view when it is ready and A waits
  for that; the server puts B's character beside A's path and, once B aims (the yaw is the signal),
  puts A on the spot B aims at; B's script holds that spot (`assert.movement_near`).
  The network runner starts the executable itself and sets `BEVY_ASSET_ROOT`.
- `scripts/web/`: `build.mjs` (browser build into `target/web/`, page taken from the engine checkout)
  and `run_in_browser.mjs <script>`. CI runs the picture scripts and the network check in the browser;
  `walk.cmds` asserts the same `state.hash` natively and in the browser.
- `scripts/verify/simulation/*.cmds`: checks with nothing to see (rules and timers). Run with
  `--headless --no-render`: a software renderer would take minutes per script.
- Per change: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo check --target wasm32-unknown-unknown`, the verify scripts.

## Game numbers (decided 2026-11-25, set in `src/arena_game.rs`)

| What | Value |
|---|---|
| Rifle | 20 damage, a shot every 0.15 s, 30 rounds, 1.5 s reload (automatic when empty), 100 m |
| Health | 100, respawn after 3 s, 1 s spawn protection (ends on firing), die below -10 m |
| HUD | hit marker 0.2 s, text size 28, crosshair 24 px |

## Status

Arena steps 1 to 3: animated, colored foxes walk the `graybox` arena (stairs included) and fight with a
rifle, alone or with a server and clients; the server decides hits as the shooter saw the others.
Dead players are hidden, cannot move, shoot or be hit, and respawn at a spawn point. No rockets, match
display or scoreboard yet. The server needs `--headless --no-render`. The hit shape is the 1.8 m
character capsule, taller than the fox. A dead player's invisible body still blocks walking.
