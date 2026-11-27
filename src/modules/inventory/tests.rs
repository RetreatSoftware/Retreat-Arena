//! What `inventory` adds to the checks.

use super::*;
use crate::arena_game::{assert_sound_exists, declared_setup};
use crate::net_play_tests::*;
use retreat_engine::net::link::LinkConditions;

pub const TEST: ModuleTest = ModuleTest {
    full_match,
    quiet: |_| {},
};

/// Player 1, wounded, uses the kit it started with.
fn full_match(harness: &mut retreat_engine::net::harness::NetHarness) -> MatchPart {
    on_server(harness, "health.damage 1 60 0");
    harness.run_idle(60);
    // The input may be lost on the way: sent again until it arrived.
    for _ in 0..5 {
        if health(&mut harness.server_app, 1) == 40 {
            harness.step(&[(1, vec!["inventory.use health_kit".to_string()])]);
            harness.run_idle(60);
        }
    }
    on_server(harness, "assert.health 1 90");
    on_server(harness, "assert.inventory 1 health_kit 0");
    MatchPart {
        kills: 0,
        sounds: vec!["item/1/1"],
    }
}

#[test]
fn its_sound_is_in_the_assets() {
    assert_sound_exists(config(&declared_setup()).use_sound);
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
