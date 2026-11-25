//! The game's gameplay state over the network: a real server and two clients in one process, with
//! latency, jitter and loss (the engine's harness). The server decides; clients must end up with the
//! same health, scores and phase, and predict the timers without further corrections.

use crate::arena_game::add_arena_plugins;
use bevy::prelude::*;
use retreat_engine::net::commands::NetCommandsPlugin;
use retreat_engine::net::harness::{HarnessConfig, NetHarness};
use retreat_engine::net::link::LinkConditions;
use retreat_engine::{InputActionsPlugin, PhysicsPlugin, RetreatCommandsPlugin, StateHashPlugin};
use retreat_engine_api::{AnalogInput, run_command_line};
use retreat_gameplay_api::{
    Health, LookAngles, MatchPhase, MatchState, PlayerId, Scoreboard, SpawnPoints, character_feet,
};
use retreat_module_character_movement::{ADVANCE_AXIS, MovementState, STRAFE_AXIS};

fn game_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        bevy::input::InputPlugin,
    ))
    .init_asset::<Mesh>()
    .init_asset::<StandardMaterial>()
    .add_plugins(RetreatCommandsPlugin::default())
    .add_plugins((
        PhysicsPlugin,
        StateHashPlugin,
        NetCommandsPlugin,
        InputActionsPlugin,
    ));
    add_arena_plugins(&mut app);
    app
}

/// A slow, jittery link that loses packets.
const BAD_LINK: LinkConditions = LinkConditions {
    latency_ticks: 4,
    jitter_ticks: 1,
    loss: 0.05,
};

fn harness() -> NetHarness {
    harness_over(BAD_LINK, 4)
}

/// A server and two joined clients; each client has a character.
fn harness_over(link: LinkConditions, margin_ticks: i64) -> NetHarness {
    let config = HarnessConfig {
        margin_ticks,
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
    on_server(&mut harness, "health.add_dummy 11");
    on_server(&mut harness, "health.add_dummy 12");
    harness.run_idle(100);
    on_server(&mut harness, "health.damage 11 60 12");
    harness.run_idle(60);
    assert_eq!(
        gameplay(&mut harness.server_app).0,
        [(0, 100), (1, 100), (11, 40), (12, 100)]
    );
    assert_clients_agree(&mut harness, "after damage");

    on_server(&mut harness, "health.damage 11 60 12");
    harness.run_idle(60);
    on_server(&mut harness, "assert.health_dead 11");
    on_server(&mut harness, "assert.match_score 12 1");
    assert_clients_agree(&mut harness, "after the death");

    // The respawn is a timer every machine runs itself.
    harness.run_idle(200);
    on_server(&mut harness, "assert.health 11 100");
    assert_clients_agree(&mut harness, "after the respawn");

    on_server(&mut harness, "match.end");
    harness.run_idle(60);
    assert_eq!(gameplay(&mut harness.server_app).2, [true]);
    assert_clients_agree(&mut harness, "after the end");

    harness.run_idle(700);
    on_server(&mut harness, "assert.match_running");
    on_server(&mut harness, "assert.match_score 12 0");
    assert_clients_agree(&mut harness, "after the restart");
}

#[test]
fn predicted_timers_need_no_corrections() {
    let mut harness = harness();
    on_server(&mut harness, "health.add_dummy 11");
    harness.run_idle(100);
    on_server(&mut harness, "health.kill 11");
    harness.run_idle(60);
    // From here on nothing happens that a client cannot predict: the respawn, the new protection
    // and the match clock.
    let before = corrections(&harness);
    harness.run_idle(400);
    on_server(&mut harness, "assert.health 11 100");
    assert_eq!(corrections(&harness), before);
    assert_clients_agree(&mut harness, "after predicting");
}

/// `(feet position, movement state, view)` of a player on one machine.
fn character(app: &mut App, id: u32) -> (Vec3, MovementState, LookAngles) {
    let world = app.world_mut();
    let mut query = world.query::<(&PlayerId, &Transform, &MovementState, &LookAngles)>();
    query
        .iter(world)
        .find(|(player, ..)| player.0 == id)
        .map(|(_, transform, state, look)| (character_feet(transform.translation), *state, *look))
        .unwrap_or_else(|| panic!("no character {id}"))
}

fn hold(harness: &mut NetHarness, client: usize, axis: &str, value: f32) {
    harness.client_apps[client]
        .world_mut()
        .resource_mut::<AnalogInput>()
        .force_axis(axis, Some(value))
        .expect("a declared axis");
}

fn press(harness: &mut NetHarness, client: usize, key: KeyCode, down: bool) {
    let mut keys = harness.client_apps[client]
        .world_mut()
        .resource_mut::<ButtonInput<KeyCode>>();
    if down {
        keys.press(key);
    } else {
        keys.release(key);
    }
}

/// Client 0 walks forward, turns with the mouse, jumps and dashes; client 1 strafes.
fn walk_around(harness: &mut NetHarness) {
    hold(harness, 0, ADVANCE_AXIS, 1.0);
    hold(harness, 1, STRAFE_AXIS, 1.0);
    harness.run_idle(40);
    harness.client_apps[0]
        .world_mut()
        .resource_mut::<AnalogInput>()
        .add_look(Vec2::new(-150.0, 20.0));
    harness.run_idle(20);
    press(harness, 0, KeyCode::Space, true);
    harness.run_idle(10);
    press(harness, 0, KeyCode::Space, false);
    harness.run_idle(60);
    press(harness, 0, KeyCode::ControlLeft, true);
    harness.run_idle(20);
    press(harness, 0, KeyCode::ControlLeft, false);
    hold(harness, 0, ADVANCE_AXIS, 0.0);
    hold(harness, 1, STRAFE_AXIS, 0.0);
    harness.run_idle(200);
}

#[test]
fn characters_walk_and_every_machine_agrees_over_a_bad_link() {
    let mut harness = harness();
    let start = [0, 1].map(|id| character(&mut harness.server_app, id).0);
    walk_around(&mut harness);
    for id in [0, 1] {
        let (position, state, look) = character(&mut harness.server_app, id);
        assert!(
            position.distance(start[id as usize]) > 3.0,
            "player {id} walked from {} to {position}",
            start[id as usize]
        );
        assert!(state.grounded, "player {id}: {state:?}");
        for (index, client) in harness.client_apps.iter_mut().enumerate() {
            let (seen, seen_state, seen_look) = character(client, id);
            assert!(
                seen.distance(position) < 0.05,
                "client {index} sees player {id} at {seen}, the server at {position}"
            );
            assert_eq!((seen_state, seen_look), (state, look), "client {index}");
        }
    }
    assert!(character(&mut harness.server_app, 0).2.yaw != 0.0);
    assert_eq!(harness.server().stats.refusals(), 0);
}

#[test]
fn a_players_own_movement_is_predicted_before_the_server_answers() {
    let mut harness = harness_over(
        LinkConditions {
            latency_ticks: 10,
            jitter_ticks: 0,
            loss: 0.0,
        },
        4,
    );
    let start = character(&mut harness.client_apps[0], 0).0;
    hold(&mut harness, 0, ADVANCE_AXIS, 1.0);
    // Less than one trip to the server.
    harness.run_idle(8);
    let mine = character(&mut harness.client_apps[0], 0).0;
    let server = character(&mut harness.server_app, 0).0;
    assert!(mine.distance(start) > 0.2, "predicted: {mine}");
    assert!(
        mine.distance(start) > server.distance(start),
        "ahead of the server: {mine} vs {server}"
    );
}

#[test]
fn walking_alone_on_a_good_link_needs_no_corrections() {
    let mut harness = harness_over(LinkConditions::PERFECT, 4);
    let before = corrections(&harness)[0];
    hold(&mut harness, 0, ADVANCE_AXIS, 1.0);
    harness.run_idle(150);
    hold(&mut harness, 0, ADVANCE_AXIS, 0.0);
    harness.run_idle(100);
    assert_eq!(corrections(&harness)[0], before);
}

#[test]
fn a_dead_character_respawns_at_a_spawn_point_and_walks_again_on_every_machine() {
    let mut harness = harness();
    let spawns = harness
        .server_app
        .world()
        .resource::<SpawnPoints>()
        .0
        .clone();
    hold(&mut harness, 0, ADVANCE_AXIS, 1.0);
    harness.run_idle(150);
    let away = character(&mut harness.server_app, 0).0;
    hold(&mut harness, 0, ADVANCE_AXIS, 0.0);
    harness.run_idle(100);
    on_server(&mut harness, "health.kill 0");
    harness.run_idle(300);
    on_server(&mut harness, "assert.health 0 100");
    let (back, state, _) = character(&mut harness.server_app, 0);
    assert!(back.distance(away) > 5.0, "moved from {away} to {back}");
    assert!(
        spawns.iter().any(|spawn| spawn.feet.distance(back) < 0.1),
        "{back} is not a spawn point"
    );
    assert!(state.grounded, "{state:?}");
    for (index, client) in harness.client_apps.iter_mut().enumerate() {
        let seen = character(client, 0).0;
        assert!(seen.distance(back) < 0.05, "client {index} sees {seen}");
    }
    // It moves again, and everyone still agrees.
    hold(&mut harness, 0, ADVANCE_AXIS, 1.0);
    harness.run_idle(60);
    hold(&mut harness, 0, ADVANCE_AXIS, 0.0);
    harness.run_idle(200);
    let walked = character(&mut harness.server_app, 0).0;
    assert!(walked.distance(back) > 3.0, "from {back} to {walked}");
    for (index, client) in harness.client_apps.iter_mut().enumerate() {
        let seen = character(client, 0).0;
        assert!(seen.distance(walked) < 0.05, "client {index} sees {seen}");
    }
}
