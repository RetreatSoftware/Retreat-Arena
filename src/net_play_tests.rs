//! The game's gameplay state over the network: a real server and two clients in one process, with
//! latency, jitter and loss (the engine's harness). The server decides; clients must end up with the
//! same health, scores and phase, and predict the timers without further corrections.

use crate::arena_game::add_arena_plugins;
use bevy::prelude::*;
use retreat_engine::net::commands::NetCommandsPlugin;
use retreat_engine::net::harness::{HarnessConfig, NetHarness};
use retreat_engine::net::link::LinkConditions;
use retreat_engine::{PhysicsPlugin, RetreatCommandsPlugin, StateHashPlugin};
use retreat_engine_api::run_command_line;
use retreat_gameplay_api::{Health, MatchPhase, MatchState, PlayerId, Scoreboard};

fn game_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransformPlugin, AssetPlugin::default()))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins(RetreatCommandsPlugin::default())
        .add_plugins((PhysicsPlugin, StateHashPlugin, NetCommandsPlugin));
    add_arena_plugins(&mut app);
    app
}

fn harness() -> NetHarness {
    let link = LinkConditions {
        latency_ticks: 4,
        jitter_ticks: 1,
        loss: 0.05,
    };
    let config = HarnessConfig {
        margin_ticks: 4,
        state_interval_ticks: 2,
        up: link,
        down: link,
        seed: 3,
    };
    let mut harness = NetHarness::new(config, &[], 2, game_app);
    harness.run_idle(100);
    assert!(harness.all_joined());
    harness
}

fn on_server(harness: &mut NetHarness, line: &str) {
    run_command_line(harness.server_app.world_mut(), line)
        .unwrap_or_else(|error| panic!("`{line}` failed: {error}"));
}

/// Health by player, the scoreboard and whether the match is finished (not the countdown: a client
/// runs ahead of the server).
fn gameplay(app: &mut App) -> (Vec<(u32, u32)>, Vec<Scoreboard>, Vec<bool>) {
    let world = app.world_mut();
    let mut health: Vec<(u32, u32)> = world
        .query::<(&PlayerId, &Health)>()
        .iter(world)
        .map(|(player, health)| (player.0, health.current()))
        .collect();
    health.sort();
    let boards = world.query::<&Scoreboard>().iter(world).cloned().collect();
    let finished = world
        .query::<&MatchState>()
        .iter(world)
        .map(|state| matches!(state.phase, MatchPhase::Finished { .. }))
        .collect();
    (health, boards, finished)
}

fn assert_clients_agree(harness: &mut NetHarness, moment: &str) {
    let server = gameplay(&mut harness.server_app);
    for (index, client) in harness.client_apps.iter_mut().enumerate() {
        assert_eq!(gameplay(client), server, "client {index} {moment}");
    }
}

fn corrections(harness: &NetHarness) -> Vec<u64> {
    (0..harness.client_apps.len())
        .map(|index| harness.client(index).stats.corrections)
        .collect()
}

#[test]
fn clients_follow_damage_death_respawn_score_and_restart() {
    let mut harness = harness();
    on_server(&mut harness, "health.add_dummy 1");
    on_server(&mut harness, "health.add_dummy 2");
    harness.run_idle(100);
    on_server(&mut harness, "health.damage 1 60 2");
    harness.run_idle(60);
    assert_eq!(gameplay(&mut harness.server_app).0, [(1, 40), (2, 100)]);
    assert_clients_agree(&mut harness, "after damage");

    on_server(&mut harness, "health.damage 1 60 2");
    harness.run_idle(60);
    on_server(&mut harness, "assert.health_dead 1");
    on_server(&mut harness, "assert.match_score 2 1");
    assert_clients_agree(&mut harness, "after the death");

    // The respawn is a timer every machine runs itself.
    harness.run_idle(200);
    on_server(&mut harness, "assert.health 1 100");
    assert_clients_agree(&mut harness, "after the respawn");

    on_server(&mut harness, "match.end");
    harness.run_idle(60);
    assert_eq!(gameplay(&mut harness.server_app).2, [true]);
    assert_clients_agree(&mut harness, "after the end");

    harness.run_idle(700);
    on_server(&mut harness, "assert.match_running");
    on_server(&mut harness, "assert.match_score 2 0");
    assert_clients_agree(&mut harness, "after the restart");
}

#[test]
fn predicted_timers_need_no_corrections() {
    let mut harness = harness();
    on_server(&mut harness, "health.add_dummy 1");
    harness.run_idle(100);
    on_server(&mut harness, "health.kill 1");
    harness.run_idle(60);
    // From here on nothing happens that a client cannot predict: the respawn, the new protection
    // and the match clock.
    let before = corrections(&harness);
    harness.run_idle(400);
    on_server(&mut harness, "assert.health 1 100");
    assert_eq!(corrections(&harness), before);
    assert_clients_agree(&mut harness, "after predicting");
}
