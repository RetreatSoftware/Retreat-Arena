//! Optional module `inventory`: reserve ammunition and health kits. Requires nothing.

#[cfg(test)]
mod tests;

use crate::arena_game::sound;
use crate::optional_module::{OptionalModule, Setup};
use bevy::prelude::*;
use retreat_gameplay_api::{ItemDefinition, ItemId, ItemKind, WeaponKind};
use retreat_module_inventory::{InventoryConfig, InventoryModulePlugin};

pub const MODULE: OptionalModule = OptionalModule {
    declare,
    add,
    #[cfg(test)]
    test: tests::TEST,
};

/// The reserve ammunition and the health kit (recorded in CLAUDE.md). Item ids are stable:
/// commands, scripts and one day saved games name them.
fn declare(setup: &mut Setup) {
    let rifle = ItemKind::Ammo(WeaponKind::Rifle);
    setup.items.extend([
        ItemDefinition::new("rifle_ammo", "Rifle rounds", rifle, 120),
        ItemDefinition::new("health_kit", "Health kit", ItemKind::Heal(50), 3),
    ]);
    setup.start.extend([
        (ItemId::new("rifle_ammo"), 60),
        (ItemId::new("health_kit"), 1),
    ]);
}

/// What every new life starts with, and what using a kit sounds like.
fn config(setup: &Setup) -> InventoryConfig {
    let mut loadout = setup.start.clone();
    if !setup.places_pickups {
        loadout.extend(setup.found.clone());
    }
    InventoryConfig {
        // One slot per item the game has: nothing is ever turned away for lack of room.
        slots: setup.items.len() as u32,
        loadout,
        use_sound: sound("heal", 0.35),
    }
}

fn add(app: &mut App, setup: &Setup) {
    app.insert_resource(config(setup))
        .add_plugins(InventoryModulePlugin);
}
