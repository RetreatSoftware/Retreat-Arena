//! What `projectile_launcher` adds to the checks.

use super::*;
use crate::arena_game::assert_sound_exists;
use crate::net_play_tests::*;
use retreat_engine::EffectLog;
use retreat_engine::net::harness::NetHarness;
use retreat_engine::net::link::LinkConditions;
use retreat_gameplay_api::{Inventory, PlayerId, SpawnPoints};
use retreat_module_projectile_launcher::{Rocket, blast_look};

pub const TEST: ModuleTest = ModuleTest {
    full_match,
    quiet: |_| {},
};

/// A rocket kill by player 1, with the launcher it carries: from the start, or found on the map by
/// an earlier module's part.
fn full_match(harness: &mut NetHarness) -> MatchPart {
    face_off(harness);
    on_client(harness, 1, "movement.look 0 -0.05");
    harness.run_idle(30);
    // Two direct hits, three if the first part left the target wounded.
    for left in [20, 0] {
        harness.step(&[(1, vec!["launcher.fire 0 -0.05".to_string()])]);
        run_until_health(harness, 0, left, 200);
        harness.run_idle(60);
    }
    run_until_health(harness, 0, 100, 300);
    let own = sounds_asked(&harness.client_apps[1]);
    assert!(own.iter().any(|id| id.ends_with("/boom")), "{own:?}");
    MatchPart {
        kills: 1,
        sounds: vec!["rocket/1/"],
    }
}

/// Gives a server player the launcher and rockets where players carry items (a game without an
/// inventory gives everyone the launcher), and lets its client learn of it.
fn arm_with_launcher(harness: &mut NetHarness, id: u32) {
    let world = harness.server_app.world_mut();
    let mut carriers = world.query::<(&PlayerId, &Inventory)>();
    if carriers.iter(world).any(|(player, _)| player.0 == id) {
        on_server(harness, &format!("inventory.grant {id} launcher 1"));
        on_server(harness, &format!("inventory.grant {id} rockets 8"));
    }
    // Long enough that the client knows, and its input pacing has settled again.
    harness.run_idle(100);
}

#[test]
fn its_sounds_are_in_the_assets() {
    let config = config();
    for cue in [
        config.launch_sound,
        config.explosion_sound,
        config.reload_sound,
    ] {
        assert_sound_exists(cue);
    }
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
