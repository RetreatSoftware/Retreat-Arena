//! Optional module `loot`: map pickups and what the dead drop. Requires `inventory`.

#[cfg(test)]
mod tests;

use crate::arena_game::sound;
use crate::optional_module::{OptionalModule, PickupSetting, Setup};
use bevy::prelude::*;
use retreat_gameplay_api::ItemId;
use retreat_module_loot::{LootConfig, LootModulePlugin, PickupKind, PickupLook};

pub const MODULE: OptionalModule = OptionalModule {
    declare,
    add,
    #[cfg(test)]
    test: tests::TEST,
};

/// Health and rifle rounds; other modules declare what else may be found.
fn declare(setup: &mut Setup) {
    setup.places_pickups = true;
    setup.pickups.extend([
        PickupSetting {
            kind: "health",
            contents: vec![(ItemId::new("health_kit"), 1)],
            respawn_seconds: 15.0,
            color: Color::srgb(0.1, 0.9, 0.2),
            size: Vec3::splat(0.45),
            sound: sound("pickup_health", 0.35),
        },
        PickupSetting {
            kind: "ammo",
            contents: vec![(ItemId::new("rifle_ammo"), 30)],
            respawn_seconds: 15.0,
            color: Color::srgb(1.0, 0.85, 0.1),
            size: Vec3::new(0.55, 0.25, 0.4),
            sound: sound("pickup_ammo", 0.35),
        },
    ]);
}

/// What lies at the map's pickup spots and what the dead leave (recorded in CLAUDE.md). A spot
/// of a kind nothing declared stays empty.
fn config(setup: &Setup) -> LootConfig {
    let kinds = setup
        .pickups
        .iter()
        .map(|setting| PickupKind {
            kind: setting.kind.to_string(),
            contents: setting.contents.clone(),
            respawn_seconds: setting.respawn_seconds,
            look: PickupLook {
                color: setting.color,
                size: setting.size,
            },
            sound: setting.sound.clone(),
        })
        .collect();
    LootConfig {
        kinds,
        reach: 0.9,
        drop_seconds: 20.0,
        drop_look: PickupLook {
            color: Color::srgb(0.95, 0.95, 0.95),
            size: Vec3::splat(0.3),
        },
        drop_sound: sound("pickup_ammo", 0.3),
        ..default()
    }
}

fn add(app: &mut App, setup: &Setup) {
    app.insert_resource(config(setup))
        .add_plugins(LootModulePlugin);
}
