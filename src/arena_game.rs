//! What makes this game: the selected modules, the map choice and the view and light settings.
//! No gameplay rules here: they belong in Retreat-Modules.

use bevy::prelude::*;
use retreat_engine::dispatch_command_line;
use retreat_engine_api::GameCamera;
use retreat_module_arena::ArenaModulePlugin;
use retreat_module_health::HealthModulePlugin;
use retreat_module_match::MatchModulePlugin;

/// The map every machine loads at startup.
pub const MAP_ID: &str = "graybox";

/// The camera name scripts use. An overview until the third-person camera module exists.
const CAMERA_NAME: &str = "arena_camera";
const CAMERA_POSITION: Vec3 = Vec3::new(0.0, 48.0, 56.0);
const CAMERA_TARGET: Vec3 = Vec3::new(0.0, 0.0, 4.0);

const SUN_ILLUMINANCE: f32 = 8_000.0;
/// A weaker light from the opposite side, so faces turned away from the sun stay readable.
const FILL_ILLUMINANCE: f32 = 2_000.0;

/// The module selection plus this game's own setup.
pub fn add_arena_plugins(app: &mut App) {
    // Each module's numbers are its config defaults.
    app.add_plugins((ArenaModulePlugin, HealthModulePlugin, MatchModulePlugin))
        .add_systems(Startup, (load_chosen_map, spawn_view));
}

/// Loads the map through its command, like any other entry point.
fn load_chosen_map(world: &mut World) {
    dispatch_command_line(world, "arena startup", &format!("arena.load {MAP_ID}"));
}

fn spawn_view(mut commands: Commands) {
    commands.spawn((
        Name::new(CAMERA_NAME),
        GameCamera::looking_at(CAMERA_POSITION, CAMERA_TARGET),
    ));
    commands.spawn((
        Name::new("arena_sun"),
        DirectionalLight {
            illuminance: SUN_ILLUMINANCE,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(25.0, 40.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("arena_fill_light"),
        DirectionalLight {
            illuminance: FILL_ILLUMINANCE,
            ..default()
        },
        Transform::from_xyz(-25.0, 20.0, -15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use retreat_engine::RetreatCommandsPlugin;
    use retreat_module_arena::LoadedArena;

    #[test]
    fn startup_loads_the_chosen_map_and_one_camera() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .add_plugins(RetreatCommandsPlugin::default());
        add_arena_plugins(&mut app);
        app.update();
        let world = app.world_mut();
        assert_eq!(world.resource::<LoadedArena>().0, MAP_ID);
        assert_eq!(world.query::<&GameCamera>().iter(world).count(), 1);
    }
}
