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
  are `retreat_engine` (`launch`, `dispatch_command_line`; tests also `RetreatCommandsPlugin`) and
  `retreat_engine_api`.
- A module is selected by adding its dependency in `Cargo.toml` and its plugin in
  `src/arena_game.rs::add_arena_plugins`. Its numbers are overridden there by inserting its config
  resource before the plugin.
- Do not modify sibling repositories from this one without being asked.

## Layout

- `src/main.rs` calls `retreat_engine::launch`.
- `src/arena_game.rs`: module selection, `MAP_ID`, camera and lights.
- `scripts/verify/*.cmds`: one script per check. Run from the repository root and open every PNG in
  `target/snapshots/`:
  `cargo run -- --headless --commands scripts/verify/arena_map.cmds`
- Per change: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
  `cargo check --target wasm32-unknown-unknown`, the verify scripts.

## Status

Loads the `graybox` arena (`retreat_module_arena`) with an overview camera. No players yet. No
browser build page yet (the engine's `web/` page and scripts are not reusable from another repository;
needed for plan step 6).
