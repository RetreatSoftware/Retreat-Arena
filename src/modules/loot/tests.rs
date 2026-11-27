//! What `loot` adds to the checks.

use super::*;
use crate::arena_game::{assert_sound_exists, declared_setup};
use crate::net_play_tests::*;
use retreat_engine::net::harness::NetHarness;
use retreat_engine::net::link::LinkConditions;
use retreat_gameplay_api::PickupSpots;
use retreat_module_arena::graybox_map;

pub const TEST: ModuleTest = ModuleTest {
    full_match,
    // A respawned dummy picking up its own drop is the server's to say.
    quiet: |server| server.world_mut().resource_mut::<LootConfig>().drop_seconds = 0.0,
};

/// Player 1 takes what its victim dropped, a health kit, and all equipment found on the map.
fn full_match(harness: &mut NetHarness) -> MatchPart {
    on_server(harness, "assert.loot_drops 1");
    on_server(
        harness,
        &format!(
            "physics.teleport player_1 {} 0.6 {}",
            TARGET_FEET.x, TARGET_FEET.z
        ),
    );
    harness.run_idle(40);
    on_server(harness, "assert.loot_drops 0");
    harness.client_apps[1].world_mut().run_schedule(Update);
    on_client(harness, 1, "assert.hud_shown hud_pickup0 true");
    walk_onto(harness, "health");
    let setup = declared_setup();
    // The spot that holds all of the equipment, when some module declared any.
    let complete = setup.pickups.iter().find(|setting| {
        !setup.found.is_empty()
            && setup
                .found
                .iter()
                .all(|(found, _)| setting.contents.iter().any(|(item, _)| item == found))
    });
    if let Some(setting) = complete {
        walk_onto(harness, setting.kind);
        for (item, count) in &setting.contents {
            on_server(harness, &format!("assert.inventory 1 {item} {count}"));
        }
    }
    MatchPart {
        kills: 0,
        sounds: vec!["loot/1/"],
    }
}

/// Puts player 1 on the first spot of `kind` and checks the pickup was taken.
fn walk_onto(harness: &mut NetHarness, kind: &str) {
    let spot = harness
        .server_app
        .world()
        .resource::<PickupSpots>()
        .0
        .iter()
        .find(|spot| spot.kind == kind)
        .unwrap_or_else(|| panic!("the map has no spot of kind {kind}"))
        .clone();
    let (x, y, z) = (spot.feet.x, spot.feet.y + 0.6, spot.feet.z);
    on_server(harness, &format!("physics.teleport player_1 {x} {y} {z}"));
    harness.run_idle(100);
    let taken = format!("assert.loot_available {} false", spot.name);
    on_server(harness, &taken);
}

#[test]
fn every_declared_pickup_has_a_spot_on_the_map_and_its_sounds() {
    let spots = graybox_map().pickup_spots;
    let config = config(&declared_setup());
    for kind in &config.kinds {
        assert!(
            spots.iter().any(|spot| spot.kind == kind.kind),
            "no spot holds {}",
            kind.kind
        );
    }
    assert_sound_exists(config.drop_sound);
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
            "assert.loot_available ammo_northwest true",
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
