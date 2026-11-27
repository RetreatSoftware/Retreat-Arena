//! The optional modules in this build, in the order they declare and are added. This file is the
//! only list of them: a game without one has no line for it here (`template.json`).

use crate::optional_module::OptionalModule;
mod inventory;
mod loot;
mod projectile_launcher;

#[rustfmt::skip]
pub const OPTIONAL_MODULES: &[OptionalModule] = &[
    inventory::MODULE,
    loot::MODULE,
    projectile_launcher::MODULE,
];
