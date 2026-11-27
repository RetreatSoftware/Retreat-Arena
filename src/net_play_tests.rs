//! The game's gameplay state over the network: a real server and two clients in one process, with
//! latency, jitter and loss (the engine's harness). The server decides; clients must end up with the
//! same health, scores and phase, and predict the timers without further corrections.

use crate::arena_game::add_arena_plugins;
use bevy::prelude::*;
use retreat_engine::net::commands::NetCommandsPlugin;
use retreat_engine::net::harness::{HarnessConfig, NetHarness};
use retreat_engine::net::link::LinkConditions;
use retreat_engine::{
    EffectLog, InputActionsPlugin, NetPosePlugin, PhysicsPlugin, RetreatCommandsPlugin,
    StateHashPlugin, VisualEffectsPlugin,
};
use retreat_engine_api::{
    AnalogInput, NetViewTick, RayCast, SoundEvents, cast_ray, cast_ray_at, run_command_line,
};
use retreat_gameplay_api::{
    CharacterSize, Health, Inventory, ItemId, KillLog, LookAngles, MatchPhase, MatchState,
    PlayerId, Scoreboard, SpawnPoints, WeaponKind, WeaponStatus, Weapons, view_direction,
};
use retreat_module_character_movement::{ADVANCE_AXIS, MovementState, STRAFE_AXIS};
use retreat_module_loot::LootConfig;
use retreat_module_projectile_launcher::{Rocket, blast_look};
use std::collections::BTreeSet;

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
        NetPosePlugin,
        VisualEffectsPlugin,
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
    // Its items stay with it: a respawned dummy picking up its own drop is the server's to say.
    harness
        .server_app
        .world_mut()
        .resource_mut::<LootConfig>()
        .drop_seconds = 0.0;
    // Its items stay with it: a respawned dummy picking up its own drop is the server's to say.
    harness
        .server_app
        .world_mut()
        .resource_mut::<LootConfig>()
        .drop_seconds = 0.0;
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
    let size = *world.resource::<CharacterSize>();
    let mut query = world.query::<(&PlayerId, &Transform, &MovementState, &LookAngles)>();
    query
        .iter(world)
        .find(|(player, ..)| player.0 == id)
        .map(|(_, transform, state, look)| (size.feet(transform.translation), *state, *look))
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

/// Open ground: the shooter's feet, and the target's 8 m north of them.
const SHOOTER_FEET: Vec3 = Vec3::new(-8.0, 0.0, 26.0);
const TARGET_FEET: Vec3 = Vec3::new(-8.0, 0.0, 18.0);

/// Puts client 1's character (the shooter) and client 0's (the target) in place; the spawn
/// protection is over afterwards.
fn face_off(harness: &mut NetHarness) {
    for (name, feet) in [("player_1", SHOOTER_FEET), ("player_0", TARGET_FEET)] {
        on_server(
            harness,
            &format!("physics.teleport {name} {} 0.6 {}", feet.x, feet.z),
        );
    }
    harness.run_idle(150);
}

fn weapon(app: &mut App, id: u32) -> WeaponStatus {
    carried(app, id, WeaponKind::Rifle)
}

fn carried(app: &mut App, id: u32, kind: WeaponKind) -> WeaponStatus {
    let world = app.world_mut();
    let mut query = world.query::<(&PlayerId, &Weapons)>();
    query
        .iter(world)
        .find(|(player, _)| player.0 == id)
        .and_then(|(_, weapons)| weapons.of(kind).copied())
        .unwrap_or_else(|| panic!("no weapon {id}"))
}

fn health(app: &mut App, id: u32) -> u32 {
    gameplay(app)
        .0
        .iter()
        .find(|(player, _)| *player == id)
        .map(|(_, health)| *health)
        .unwrap_or_else(|| panic!("no player {id}"))
}

fn entity_named(app: &mut App, name: &str) -> Entity {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &Name)>();
    query
        .iter(world)
        .find(|(_, found)| found.as_str() == name)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("no entity {name}"))
}

/// Where client or server `app` has the eyes of player `id`.
fn eye(app: &mut App, id: u32) -> Vec3 {
    let feet = character(app, id).0;
    feet + Vec3::Y * app.world().resource::<CharacterSize>().eye_height
}

/// The yaw (level shots) at which client 1 sees the target at `view_tick`: the middle of the
/// directions that hit it.
fn yaw_to_what_client_1_sees(harness: &mut NetHarness, view_tick: f64) -> (f32, f32) {
    let app = &mut harness.client_apps[1];
    let own = entity_named(app, "player_1");
    let eye = eye(app, 1);
    let pitch = -0.08;
    let hits: Vec<f32> = (-900..=900)
        .map(|step| step as f32 * 0.001)
        .filter(|&yaw| {
            let cast = RayCast {
                origin: eye,
                direction: view_direction(LookAngles { yaw, pitch }),
                max_distance: 100.0,
                exclude: vec![own],
            };
            cast_ray_at(app.world_mut(), &cast, view_tick).is_some_and(|hit| hit.name == "player_0")
        })
        .collect();
    assert!(!hits.is_empty(), "client 1 does not see the target");
    (hits[hits.len() / 2], pitch)
}

fn server_hits_now(harness: &mut NetHarness, yaw: f32, pitch: f32) -> bool {
    let app = &mut harness.server_app;
    let own = entity_named(app, "player_1");
    let eye = eye(app, 1);
    let cast = RayCast {
        origin: eye,
        direction: view_direction(LookAngles { yaw, pitch }),
        max_distance: 100.0,
        exclude: vec![own],
    };
    cast_ray(app.world_mut(), &cast).is_some_and(|hit| hit.name == "player_0")
}

#[test]
fn a_shot_at_a_running_target_hits_what_the_shooter_saw_over_a_lossy_link() {
    let mut harness = harness_over(
        LinkConditions {
            latency_ticks: 8,
            jitter_ticks: 2,
            loss: 0.08,
        },
        4,
    );
    face_off(&mut harness);
    // The target sprints sideways.
    press(&mut harness, 0, KeyCode::ShiftLeft, true);
    hold(&mut harness, 0, STRAFE_AXIS, 1.0);
    harness.run_idle(30);
    let shots = 3;
    let mut would_have_missed = 0;
    for _ in 0..shots {
        let view_tick = harness.client_apps[1]
            .world()
            .resource::<NetViewTick>()
            .for_input();
        assert!(view_tick > 0.0, "the client draws from the server's past");
        let (yaw, pitch) = yaw_to_what_client_1_sees(&mut harness, view_tick);
        would_have_missed += u32::from(!server_hits_now(&mut harness, yaw, pitch));
        harness.step(&[(1, vec![format!("rifle.fire {yaw} {pitch} {view_tick}")])]);
        harness.run_idle(14);
    }
    hold(&mut harness, 0, STRAFE_AXIS, 0.0);
    press(&mut harness, 0, KeyCode::ShiftLeft, false);
    harness.run_idle(30);
    assert_eq!(
        would_have_missed, shots,
        "without lag compensation every shot would miss"
    );
    let hits = weapon(&mut harness.server_app, 1).hits_landed;
    // An input may be lost; every one that arrived must hit.
    let fired = 30 - weapon(&mut harness.server_app, 1).ammo;
    assert!(fired >= 2, "only {fired} shots arrived");
    assert_eq!(hits, fired, "every shot that arrived hit");
    assert_eq!(health(&mut harness.server_app, 0), 100 - 20 * hits);
    harness.run_idle(100);
    assert_clients_agree(&mut harness, "after the shots");
    for client in &mut harness.client_apps {
        assert_eq!(weapon(client, 1).hits_landed, hits);
    }
}

fn hold_fire(harness: &mut NetHarness, down: bool) {
    // The cursor is taken before the first shot (the click that takes it does not shoot).
    run_command_line(harness.client_apps[1].world_mut(), "input.capture_cursor").expect("a client");
    harness.run_idle(2);
    let mut buttons = harness.client_apps[1]
        .world_mut()
        .resource_mut::<ButtonInput<MouseButton>>();
    if down {
        buttons.press(MouseButton::Left);
    } else {
        buttons.release(MouseButton::Left);
    }
}

#[test]
fn holding_fire_kills_and_the_victim_respawns_on_every_machine() {
    let mut harness = harness();
    face_off(&mut harness);
    // No camera on this machine: the shots go where the player looks.
    let camera = entity_named(&mut harness.client_apps[1], "arena_camera");
    harness.client_apps[1].world_mut().despawn(camera);
    // From the eyes down to the middle of a body 8 m ahead.
    run_command_line(harness.client_apps[1].world_mut(), "movement.look 0 -0.087")
        .expect("a player");
    harness.run_idle(30);
    hold_fire(&mut harness, true);
    for _ in 0..200 {
        harness.run_idle(1);
        if health(&mut harness.server_app, 0) == 0 {
            break;
        }
    }
    hold_fire(&mut harness, false);
    on_server(&mut harness, "assert.health_dead 0");
    harness.run_idle(40);
    on_server(&mut harness, "assert.rifle_hits 1 5");
    on_server(&mut harness, "assert.match_score 1 1");
    assert_clients_agree(&mut harness, "after the kill");
    let shooter = weapon(&mut harness.server_app, 1);
    for client in &mut harness.client_apps {
        assert_eq!(weapon(client, 1), shooter);
    }
    harness.run_idle(250);
    on_server(&mut harness, "assert.health 0 100");
    on_server(&mut harness, "assert.rifle_ammo 0 30");
    let back = character(&mut harness.server_app, 0).0;
    assert!(back.distance(TARGET_FEET) > 5.0, "at a spawn point: {back}");
    for client in &mut harness.client_apps {
        assert!(character(client, 0).0.distance(back) < 0.05);
    }
    assert_clients_agree(&mut harness, "after the respawn");
    assert_eq!(harness.server().stats.refusals(), 0);
}

/// Every rocket on one machine: `(entity, name, exploded)`.
fn rockets(app: &mut App) -> Vec<(Entity, String, bool)> {
    let world = app.world_mut();
    let mut query = world
        .query_filtered::<(Entity, &Name, &Rocket), Without<retreat_engine_api::RetiredEntity>>();
    query
        .iter(world)
        .map(|(entity, name, rocket)| (entity, name.to_string(), rocket.exploded > 0))
        .collect()
}

fn effects_played(app: &App) -> u64 {
    app.world().resource::<EffectLog>().played
}

#[test]
fn a_rocket_is_predicted_once_and_the_server_decides_its_blast_over_a_lossy_link() {
    let mut harness = harness_over(
        LinkConditions {
            latency_ticks: 8,
            jitter_ticks: 2,
            loss: 0.08,
        },
        4,
    );
    face_off(&mut harness);
    arm_with_launcher(&mut harness, 1);
    // Level with the target's middle, 8 m ahead.
    harness.step(&[(1, vec!["launcher.fire 0 -0.05".to_string()])]);
    let predicted = rockets(&mut harness.client_apps[1]);
    assert_eq!(predicted.len(), 1, "the shooter sees its rocket at once");
    assert!(rockets(&mut harness.server_app).is_empty());
    let (entity, name, _) = predicted[0].clone();
    let mut server_had_it = false;
    for tick in 0..120 {
        harness.run_idle(1);
        for (found, found_name, _) in rockets(&mut harness.client_apps[1]) {
            assert_eq!(
                (found, found_name),
                (entity, name.clone()),
                "tick {tick}: the shooter keeps one rocket"
            );
        }
        for app in [&mut harness.server_app, &mut harness.client_apps[0]] {
            let seen = rockets(app);
            assert!(seen.len() <= 1, "tick {tick}: {seen:?}");
            server_had_it |= !seen.is_empty();
        }
    }
    assert!(server_had_it);
    for app in [&mut harness.server_app]
        .into_iter()
        .chain(&mut harness.client_apps)
    {
        assert!(rockets(app).is_empty(), "the exploded rocket is removed");
        let parts = blast_look(3.0).len() as u64;
        assert_eq!(
            effects_played(app),
            parts,
            "one explosion, replays included"
        );
        assert_eq!(carried(app, 1, WeaponKind::Launcher).ammo, 3);
        assert_eq!(carried(app, 1, WeaponKind::Launcher).hits_landed, 1);
    }
    assert_eq!(health(&mut harness.server_app, 0), 20);
    assert_clients_agree(&mut harness, "after the rocket");
}

#[test]
fn a_dead_player_is_walked_through_on_every_machine() {
    let mut harness = harness();
    face_off(&mut harness);
    on_server(&mut harness, "health.kill 0");
    harness.run_idle(30);
    // The shooter walks north through the body, before it respawns.
    hold(&mut harness, 1, ADVANCE_AXIS, 1.0);
    harness.run_idle(100);
    hold(&mut harness, 1, ADVANCE_AXIS, 0.0);
    harness.run_idle(40);
    on_server(&mut harness, "assert.health_dead 0");
    let walked = character(&mut harness.server_app, 1).0;
    assert!(walked.z < TARGET_FEET.z - 1.0, "stopped at {walked}");
    assert!((walked.x - TARGET_FEET.x).abs() < 0.1, "pushed to {walked}");
    for client in &mut harness.client_apps {
        assert!(character(client, 1).0.distance(walked) < 0.05);
    }
}

#[test]
fn a_shot_clearly_above_a_fox_misses() {
    let mut harness = harness();
    face_off(&mut harness);
    let app = &mut harness.server_app;
    let own = entity_named(app, "player_1");
    let mut hits_at = |height: f32| {
        let cast = RayCast {
            origin: SHOOTER_FEET + Vec3::Y * height,
            direction: Vec3::NEG_Z,
            max_distance: 20.0,
            exclude: vec![own],
        };
        cast_ray(app.world_mut(), &cast).is_some_and(|hit| hit.name == "player_0")
    };
    assert!(hits_at(1.0));
    assert!(!hits_at(1.35));
}

fn kill_log(app: &mut App) -> KillLog {
    let world = app.world_mut();
    let mut logs = world.query::<&KillLog>();
    logs.single(world).expect("one match").clone()
}

#[test]
fn a_match_ends_with_its_winner_and_restarts_by_itself_on_every_machine() {
    let mut harness = harness();
    face_off(&mut harness);
    on_server(&mut harness, "health.damage 0 100 1");
    harness.run_idle(40);
    on_server(&mut harness, "match.set_clock 1");
    harness.run_idle(100);
    on_server(&mut harness, "assert.match_winner 1");
    assert_clients_agree(&mut harness, "at the end");
    let log = kill_log(&mut harness.server_app);
    assert_eq!(log.entries().len(), 1);
    for client in &mut harness.client_apps {
        assert_eq!(kill_log(client), log);
        run_command_line(client.world_mut(), "assert.match_winner 1").expect("the winner");
    }
    // The pause is 8 s.
    harness.run_idle(520);
    on_server(&mut harness, "assert.match_running");
    on_server(&mut harness, "assert.match_score 1 0");
    assert_clients_agree(&mut harness, "after the restart");
    for client in &mut harness.client_apps {
        assert!(kill_log(client).entries().is_empty());
    }
}

#[test]
fn a_restart_with_a_rocket_in_flight_resets_everything_over_a_lossy_link() {
    let mut harness = harness_over(
        LinkConditions {
            latency_ticks: 8,
            jitter_ticks: 2,
            loss: 0.08,
        },
        4,
    );
    let spawns = harness
        .server_app
        .world()
        .resource::<SpawnPoints>()
        .0
        .clone();
    face_off(&mut harness);
    arm_with_launcher(&mut harness, 1);
    on_server(&mut harness, "health.damage 0 40 1");
    harness.step(&[(1, vec!["rifle.fire 0 0.5".to_string()])]);
    harness.run_idle(30);
    // Straight up: it would fly for four seconds.
    harness.step(&[(1, vec!["launcher.fire 0 1.4".to_string()])]);
    harness.run_idle(40);
    assert_eq!(rockets(&mut harness.server_app).len(), 1, "in flight");
    on_server(&mut harness, "match.restart");
    harness.run_idle(100);
    for app in [&mut harness.server_app]
        .into_iter()
        .chain(&mut harness.client_apps)
    {
        assert!(rockets(app).is_empty(), "the old match's rocket is gone");
        for id in [0, 1] {
            assert_eq!(health(app, id), 100);
            assert_eq!(carried(app, id, WeaponKind::Rifle).ammo, 30);
            assert_eq!(carried(app, id, WeaponKind::Launcher).ammo, 4);
            let feet = character(app, id).0;
            assert!(
                spawns.iter().any(|spawn| spawn.feet.distance(feet) < 0.1),
                "player {id} is at {feet}, not at a spawn point"
            );
        }
    }
    assert_clients_agree(&mut harness, "after the restart");
}

/// The ids of every sound a client asked for, with the ticks' repeats folded together.
fn sounds_asked(app: &App) -> BTreeSet<String> {
    let events = app.world().resource::<SoundEvents>();
    events
        .requests
        .iter()
        .map(|event| event.id.clone())
        .collect()
}

fn on_client(harness: &mut NetHarness, client: usize, line: &str) {
    run_command_line(harness.client_apps[client].world_mut(), line)
        .unwrap_or_else(|error| panic!("client {client}: `{line}` failed: {error}"));
}

/// Runs until the server's player `id` has `health`, at most `most` ticks.
fn run_until_health(harness: &mut NetHarness, id: u32, wanted: u32, most: u64) {
    for _ in 0..most {
        if health(&mut harness.server_app, id) == wanted {
            return;
        }
        harness.run_idle(1);
    }
    panic!(
        "player {id} has {} health after {most} ticks, not {wanted}",
        health(&mut harness.server_app, id)
    );
}

/// The arena preset check: a whole match of the game's own length, played over a lossy link, from
/// joining to the next match. Deterministic: the harness steps server and clients tick by tick.
#[test]
fn arena_preset_full_match() {
    let mut harness = harness();
    on_server(&mut harness, "assert.match_running");
    // Both walk, jump and dash.
    walk_around(&mut harness);
    face_off(&mut harness);
    let camera = entity_named(&mut harness.client_apps[1], "arena_camera");
    harness.client_apps[1].world_mut().despawn(camera);
    // A rifle kill by holding the button.
    on_client(&mut harness, 1, "movement.look 0 -0.05");
    harness.run_idle(30);
    hold_fire(&mut harness, true);
    run_until_health(&mut harness, 0, 0, 300);
    hold_fire(&mut harness, false);
    run_until_health(&mut harness, 0, 100, 300);
    // The dead player's items lie where it fell.
    on_server(&mut harness, "assert.loot_drops 1");
    // The killer takes them, finds a health kit and uses one.
    on_server(&mut harness, "physics.teleport player_1 -8 0.6 18");
    harness.run_idle(40);
    on_server(&mut harness, "assert.loot_drops 0");
    harness.client_apps[1].world_mut().run_schedule(Update);
    on_client(&mut harness, 1, "assert.hud_shown hud_pickup0 true");
    on_server(&mut harness, "physics.teleport player_1 20 0.6 6");
    harness.run_idle(60);
    on_server(&mut harness, "assert.loot_available health_east false");
    on_server(&mut harness, "assert.inventory 1 health_kit 3");
    on_server(&mut harness, "health.damage 1 60 0");
    harness.run_idle(60);
    harness.step(&[(1, vec!["inventory.use health_kit".to_string()])]);
    harness.run_idle(60);
    on_server(&mut harness, "assert.health 1 90");
    on_server(&mut harness, "assert.inventory 1 health_kit 2");
    // Player 1 fetches the launcher from the high platform.
    on_server(&mut harness, "physics.teleport player_1 -3.5 2.6 -21.5");
    harness.run_idle(100);
    on_server(
        &mut harness,
        "assert.loot_available launcher_platform false",
    );
    on_server(&mut harness, "assert.inventory 1 launcher 1");
    on_server(&mut harness, "assert.inventory 1 rockets 4");
    // A rocket kill: two direct hits.
    face_off(&mut harness);
    on_client(&mut harness, 1, "movement.look 0 -0.05");
    harness.run_idle(30);
    for left in [20, 0] {
        harness.step(&[(1, vec!["launcher.fire 0 -0.05".to_string()])]);
        run_until_health(&mut harness, 0, left, 200);
        harness.run_idle(60);
    }
    run_until_health(&mut harness, 0, 100, 300);
    assert_clients_agree(&mut harness, "after both kills");
    on_server(&mut harness, "assert.match_score 1 2");
    // The rest of the match, on its own clock.
    let mut ticks = 0;
    while !gameplay(&mut harness.server_app).2[0] {
        harness.run_idle(64);
        ticks += 64;
        assert!(ticks <= 180 * 64, "the match did not end after its length");
    }
    harness.run_idle(100);
    on_server(&mut harness, "assert.match_winner 1");
    assert_clients_agree(&mut harness, "at the end");
    for client in 0..2 {
        // The result and the scoreboard are on screen.
        harness.client_apps[client].world_mut().run_schedule(Update);
        for line in [
            "assert.match_winner 1",
            "assert.hud_text hud_match_banner Blue wins",
            "assert.hud_shown hud_match_score_row1 true",
        ] {
            on_client(&mut harness, client, line);
        }
    }
    harness.run_idle(8 * 64);
    on_server(&mut harness, "assert.match_running");
    on_server(&mut harness, "assert.match_score 1 0");
    on_server(&mut harness, "assert.health 0 100");
    assert_clients_agree(&mut harness, "in the next match");
    assert_eq!(harness.server().stats.refusals(), 0);
    // Everything that happened was heard on a client, the other player's sounds included.
    let heard = sounds_asked(&harness.client_apps[0]);
    for start in [
        "rifle/1/1",
        "rocket/1/",
        "death/0/0",
        "respawn/0/1",
        "move/0/jump/1",
        "move/0/dash/1",
        "match/0/end",
        "match/1/start",
        "loot/1/",
        "item/1/1",
    ] {
        assert!(
            heard.iter().any(|id| id.starts_with(start)),
            "no sound `{start}..` among {heard:?}"
        );
    }
    let own = sounds_asked(&harness.client_apps[1]);
    assert!(own.iter().any(|id| id.starts_with("hud/hit/")), "{own:?}");
    assert!(own.iter().any(|id| id.ends_with("/boom")), "{own:?}");
}

/// How many of `item` player `id` carries on one machine.
fn items_of(app: &mut App, id: u32, item: &str) -> u32 {
    let world = app.world_mut();
    let mut query = world.query::<(&PlayerId, &Inventory)>();
    query
        .iter(world)
        .find(|(player, _)| player.0 == id)
        .map(|(_, inventory)| inventory.count(&ItemId::new(item)))
        .unwrap_or_else(|| panic!("no inventory {id}"))
}

#[test]
fn reserves_and_health_kits_agree_on_every_machine_over_a_lossy_link() {
    let mut harness = harness_over(
        LinkConditions {
            latency_ticks: 8,
            jitter_ticks: 2,
            loss: 0.08,
        },
        4,
    );
    face_off(&mut harness);
    // Client 1 shoots into the sky twice and reloads: the reserve pays.
    for line in ["rifle.fire 0 1.2", "rifle.fire 0 1.2", "rifle.reload"] {
        harness.step(&[(1, vec![line.to_string()])]);
        harness.run_idle(20);
    }
    harness.run_idle(150);
    let fired = 60 - items_of(&mut harness.server_app, 1, "rifle_ammo");
    assert!((1..=2).contains(&fired), "{fired} rounds left the reserve");
    assert_eq!(weapon(&mut harness.server_app, 1).ammo, 30);
    // Wounded, it uses its kit: at once on its own machine, then everywhere.
    on_server(&mut harness, "health.damage 1 70 0");
    harness.run_idle(40);
    // The input may be lost on the way: then the prediction is taken back and it is sent again.
    let mut attempts = 0;
    while health(&mut harness.server_app, 1) == 30 {
        attempts += 1;
        assert!(attempts <= 5, "no use of the kit ever arrived");
        harness.step(&[(1, vec!["inventory.use health_kit".to_string()])]);
        harness.run_idle(2);
        assert_eq!(health(&mut harness.client_apps[1], 1), 80, "predicted");
        assert_eq!(health(&mut harness.server_app, 1), 30, "not there yet");
        harness.run_idle(100);
    }
    assert_eq!(health(&mut harness.server_app, 1), 80);
    for app in [&mut harness.server_app]
        .into_iter()
        .chain(&mut harness.client_apps)
    {
        assert_eq!(items_of(app, 1, "health_kit"), 0);
        assert_eq!(items_of(app, 1, "rifle_ammo"), 60 - fired);
        assert_eq!(items_of(app, 0, "health_kit"), 1);
    }
    assert_clients_agree(&mut harness, "after the kit");
    // A death empties it, the new life starts with the loadout.
    on_server(&mut harness, "health.kill 1");
    harness.run_idle(40);
    for client in &mut harness.client_apps {
        assert_eq!(items_of(client, 1, "rifle_ammo"), 0);
    }
    harness.run_idle(250);
    for app in [&mut harness.server_app]
        .into_iter()
        .chain(&mut harness.client_apps)
    {
        assert_eq!(items_of(app, 1, "rifle_ammo"), 60);
        assert_eq!(items_of(app, 1, "health_kit"), 1);
    }
    assert_eq!(harness.server().stats.refusals(), 0);
}

/// Gives a server player the launcher and rockets, and lets its client learn of it.
fn arm_with_launcher(harness: &mut NetHarness, id: u32) {
    on_server(harness, &format!("inventory.grant {id} launcher 1"));
    on_server(harness, &format!("inventory.grant {id} rockets 8"));
    // Long enough that the client knows, and its input pacing has settled again.
    harness.run_idle(100);
}

#[test]
fn pickups_and_death_drops_agree_on_every_machine_over_a_lossy_link() {
    let mut harness = harness_over(
        LinkConditions {
            latency_ticks: 8,
            jitter_ticks: 2,
            loss: 0.08,
        },
        4,
    );
    face_off(&mut harness);
    on_server(&mut harness, "physics.teleport player_1 20 0.6 6");
    harness.run_idle(100);
    for client in 0..2 {
        on_client(
            &mut harness,
            client,
            "assert.loot_available health_east false",
        );
        on_client(
            &mut harness,
            client,
            "assert.loot_available launcher_platform true",
        );
    }
    for app in [&mut harness.server_app]
        .into_iter()
        .chain(&mut harness.client_apps)
    {
        assert_eq!(items_of(app, 1, "health_kit"), 2, "taken once");
    }
    // It comes back after fifteen seconds, and is not taken by a player that is full.
    on_server(&mut harness, "inventory.grant 1 health_kit 1");
    harness.run_idle(15 * 64);
    for client in 0..2 {
        on_client(
            &mut harness,
            client,
            "assert.loot_available health_east true",
        );
    }
    // A death leaves a drop everyone sees; the other player takes it.
    on_server(&mut harness, "health.kill 1");
    harness.run_idle(60);
    for client in 0..2 {
        on_client(&mut harness, client, "assert.loot_drops 1");
    }
    on_server(&mut harness, "physics.teleport player_0 20 0.6 6");
    harness.run_idle(100);
    for app in [&mut harness.server_app]
        .into_iter()
        .chain(&mut harness.client_apps)
    {
        assert_eq!(items_of(app, 0, "health_kit"), 3);
        assert_eq!(items_of(app, 0, "rifle_ammo"), 120);
    }
    // What did not fit (two kits) still lies there, until the new match.
    for client in 0..2 {
        on_client(&mut harness, client, "assert.loot_drops 1");
    }
    on_server(&mut harness, "match.restart");
    harness.run_idle(100);
    for client in 0..2 {
        on_client(&mut harness, client, "assert.loot_drops 0");
    }
    assert_clients_agree(&mut harness, "after the restart");
    assert_eq!(harness.server().stats.refusals(), 0);
}
