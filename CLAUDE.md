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
- `src/arena_game.rs`: module selection, `MAP_ID`, camera and lights. A game started with no
  script and no network spawns the solo player (id 0). One directional light only: the browser draws
  no second one.
- `src/net_play_tests.rs`: server and clients in one process (this binary crate has no `tests/`).
- `scripts/verify/*.cmds`: one script per check that writes pictures. Run from the repository root and
  open every PNG in `target/snapshots/`:
  `cargo run -- --headless --commands scripts/verify/arena_map.cmds`
- `scripts/verify/network/`: `node scripts/verify/network/run.mjs [--browser] [--software-renderer]`
  after `cargo build` (and `node scripts/web/build.mjs` for `--browser`). Client A moves, client B
  renders and watches; B's script holds client A's end position.
- `scripts/web/`: `build.mjs` (browser build into `target/web/`, page taken from the engine checkout)
  and `run_in_browser.mjs <script>`. CI runs the picture scripts and the network check in the browser;
  `walk.cmds` asserts the same `state.hash` natively and in the browser.
- `scripts/verify/simulation/*.cmds`: checks with nothing to see (rules and timers). Run with
  `--headless --no-render`: a software renderer would take minutes per script.
- Per change: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo check --target wasm32-unknown-unknown`, the verify scripts.

## Status

Arena step 1: characters walk the `graybox` arena (spawning, movement, third-person camera), alone or
with a server and clients; `health` and `match` are selected with their default numbers. No weapons
yet (`health.add_dummy` and `health.damage` stand in). A dead player respawns with full health where
it is: the engine cannot teleport bodies yet. The server needs `--headless --no-render` and uses a
whole CPU core (the engine runs headless frames back to back).
