//! What an optional module of this game is. Each one lives in `src/modules/<module id>/` and is
//! listed in `src/modules/mod.rs`; leaving it out of the game means deleting its folder, its list
//! entry and its dependency in `Cargo.toml` (the choices: `template.json`).
//!
//! Modules never name each other. What one needs to say to another goes through [`Setup`]: first
//! every module declares into it, then every module adds itself reading it.

use bevy::prelude::*;
use retreat_gameplay_api::{ItemDefinition, ItemId, SoundCue};

pub struct OptionalModule {
    /// Adds what it brings to the game to [`Setup`].
    pub declare: fn(&mut Setup),
    /// Inserts its settings and its plugin.
    pub add: fn(&mut App, &Setup),
    #[cfg(test)]
    pub test: crate::net_play_tests::ModuleTest,
}

/// What the selected modules bring, collected before any of them is added.
#[derive(Debug, Default)]
// A selection without a module that reads a field leaves that field unread.
#[allow(dead_code)]
pub struct Setup {
    /// The game's items, in the order they are listed on screen.
    pub items: Vec<ItemDefinition>,
    /// What every new life carries.
    pub start: Vec<(ItemId, u32)>,
    /// Equipment that is found on the map; carried from the start when nothing places pickups.
    pub found: Vec<(ItemId, u32)>,
    /// What may lie at the map's pickup spots.
    pub pickups: Vec<PickupSetting>,
    /// A selected module places pickups on the map.
    pub places_pickups: bool,
}

/// What one kind of pickup spot holds and how it looks and sounds (recorded in CLAUDE.md).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PickupSetting {
    /// The map's name for the kind of spot.
    pub kind: &'static str,
    pub contents: Vec<(ItemId, u32)>,
    pub respawn_seconds: f32,
    pub color: Color,
    /// Width, height and depth of the box, metres.
    pub size: Vec3,
    pub sound: Option<SoundCue>,
}
