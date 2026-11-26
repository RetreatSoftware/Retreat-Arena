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
  per player, the character size, the weapon, health and HUD numbers, camera, sky and lights. A game started with no
  script and no network spawns the solo player (id 0). One directional light only: the browser draws
  no second one.
- `src/net_play_tests.rs`: server and clients in one process (this binary crate has no `tests/`).
  **Arena preset check** (decided 2026-11-26): `arena_preset_full_match` (a whole 180 s match over a
  lossy link, deterministic) together with the real-process network check is what proves the arena
  module set playable.
- `scripts/sounds/generate.mjs` makes `assets/sounds/*.wav`; CI runs it with `--check`.
- `scripts/verify/*.cmds`: one script per check that writes pictures. Run from the repository root and
  open every PNG in `target/snapshots/`:
  `cargo run -- --headless --commands scripts/verify/arena_map.cmds`
- `scripts/verify/network/`: `node scripts/verify/network/run.mjs [--browser | --served-page] [--software-renderer]`
  after `cargo build` (and `node scripts/web/build.mjs` for the browser; CI uses `--served-page`). Client A moves, client B
  renders, watches, hits A with a rocket, kills it with a rifle shot and sees the respawn. Then client C joins the running match and everyone sees the result and the restart. Scripts set the match clock with `match.set_clock`. B turns its view when it is ready and A waits
  for that; the server puts B's character beside A's path and, once B aims (the yaw is the signal),
  puts A on the spot B aims at; B's script holds that spot (`assert.movement_near`).
  The network runner starts the executable itself and sets `BEVY_ASSET_ROOT`.
- `play.bat` (`scripts/play.ps1`): a local server and two client windows. `play-lan.bat`
  (`scripts/play-lan.ps1`, decided 2026-11-26): web build, then a server that serves it to the local
  network over HTTPS (`--serve-web`, engine capability).
- `scripts/web/`: `build.mjs` (browser build into `target/web/` by the engine's `build_web.mjs`)
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
| Rockets | 80 on a direct hit, 80 to 20 over a 3 m blast, half to the shooter, 25 m/s, 4 s flight, one every 0.9 s, 4 rockets, 2 s reload |
| Match (decided 2026-11-26) | 180 s, result shown 8 s; 1 point a kill, -1 for a self-kill or a fall |
| Players | named by color: Red, Blue, Green, Yellow, Purple, Orange, Cyan, White |
| Scoreboard and kill feed | text size 22; a death stays 4 s, at most 4 |
| HUD looks (decided 2026-11-26) | black panel at 0.85 behind text; player lines in the player's color |
| Health | 100, respawn after 3 s, 1 s spawn protection (ends on firing), die below -10 m |
| Character size (decided 2026-11-26) | the fox: radius 0.35 m, height 1.2 m, eyes 1.0 m |
| HUD | hit marker 0.2 s, text size 28, crosshair 24 px |
| Sound volumes (decided 2026-11-26, not judged by ear) | shot 0.3, reload 0.35, rocket 0.45, explosion 0.65, hit 0.35, death 0.45, respawn 0.3, jump 0.2, dash 0.3, match start and end 0.45 |

## Status

Arena steps 1 to 7, the planned game is complete: animated, colored foxes walk the `graybox` arena and
fight timed free-for-all matches with a rifle and rockets, alone, on one computer or from browsers on
the local network (`play-lan.bat`). The server decides hits, deaths, scores and the match; the HUD
shows health, ammo, clock, kill feed, scoreboard and winner; weapons, hits, deaths, movement and the
match are heard. The server needs `--headless --no-render`. Not tried by hand or ear, or from a
second device.
