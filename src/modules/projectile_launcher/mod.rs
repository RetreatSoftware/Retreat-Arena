//! Optional module `projectile_launcher`: the rockets. Requires nothing. Its items count only in a
//! game with an inventory, its pickups only in a game that places them.

#[cfg(test)]
mod tests;

use crate::arena_game::sound;
use crate::optional_module::{OptionalModule, PickupSetting, Setup};
use bevy::prelude::*;
use retreat_gameplay_api::{ItemDefinition, ItemId, ItemKind, WeaponKind};
use retreat_module_projectile_launcher::{LauncherConfig, ProjectileLauncherModulePlugin};

pub const MODULE: OptionalModule = OptionalModule {
    declare,
    add,
    #[cfg(test)]
    test: tests::TEST,
};

/// The launcher and its rockets as items: found on the map, or carried from the start when the
/// game places no pickups.
fn declare(setup: &mut Setup) {
    let (rockets, launcher) = (ItemId::new("rockets"), ItemId::new("launcher"));
    setup.items.extend([
        ItemDefinition::new(
            "rockets",
            "Rockets",
            ItemKind::Ammo(WeaponKind::Launcher),
            8,
        ),
        ItemDefinition::new(
            "launcher",
            "Rocket launcher",
            ItemKind::Weapon(WeaponKind::Launcher),
            1,
        ),
    ]);
    setup
        .found
        .extend([(launcher.clone(), 1), (rockets.clone(), 8)]);
    setup.pickups.extend([
        PickupSetting {
            kind: "rockets",
            contents: vec![(rockets.clone(), 4)],
            respawn_seconds: 20.0,
            color: Color::srgb(1.0, 0.4, 0.05),
            size: Vec3::new(0.65, 0.22, 0.22),
            sound: sound("pickup_ammo", 0.35),
        },
        PickupSetting {
            kind: "launcher",
            contents: vec![(launcher, 1), (rockets, 4)],
            respawn_seconds: 30.0,
            color: Color::srgb(0.75, 0.2, 1.0),
            size: Vec3::new(1.1, 0.28, 0.28),
            sound: sound("pickup_weapon", 0.45),
        },
    ]);
}

/// The rockets' numbers (recorded in CLAUDE.md): a direct hit and one rifle shot kill.
fn config() -> LauncherConfig {
    LauncherConfig {
        damage: 80,
        edge_damage: 20,
        blast_radius: 3.0,
        self_damage_percent: 50,
        speed: 25.0,
        rocket_radius: 0.15,
        lifetime_seconds: 4.0,
        fire_interval_seconds: 0.9,
        magazine: 4,
        reload_seconds: 2.0,
        aim_range: 100.0,
        launch_sound: sound("rocket_launch", 0.45),
        explosion_sound: sound("explosion", 0.65),
        reload_sound: sound("rifle_reload", 0.35),
    }
}

fn add(app: &mut App, _setup: &Setup) {
    app.insert_resource(config())
        .add_plugins(ProjectileLauncherModulePlugin);
}
