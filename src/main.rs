//! Retreat Arena. Launch options are the engine's (`--commands <script>`, `--headless`,
//! `--software-renderer`, `--seed`, `--server`, `--connect`).

mod arena_game;
#[cfg(test)]
mod net_play_tests;

use bevy::prelude::*;

const WINDOW_TITLE: &str = "Retreat Arena";

/// Returns the app's exit status, so a failing command script fails the process (native).
fn main() -> AppExit {
    retreat_engine::launch(WINDOW_TITLE, arena_game::add_arena_plugins)
}
