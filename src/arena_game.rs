//! What makes this game: the selected modules, the map choice and the view and light settings.
//! No gameplay rules here: they belong in Retreat-Modules.

use bevy::prelude::*;
use retreat_engine::net::client::NetClient;
use retreat_engine::net::server::NetServer;
use retreat_engine::{CommandScript, dispatch_command_line};
use retreat_engine_api::GameCamera;
use retreat_module_arena::ArenaModulePlugin;
use retreat_module_character_movement::CharacterMovementModulePlugin;
use retreat_module_health::HealthModulePlugin;
use retreat_module_match::MatchModulePlugin;
use retreat_module_player_spawn::PlayerSpawnModulePlugin;
use retreat_module_third_person_camera::ThirdPersonCameraModulePlugin;

/// The map every machine loads at startup.
pub const MAP_ID: &str = "graybox";

/// The player of a game played alone.
pub const SOLO_PLAYER_ID: u32 = 0;

/// The camera: it follows this machine's player, and shows the whole arena until there is one.
const CAMERA_NAME: &str = "arena_camera";
const CAMERA_POSITION: Vec3 = Vec3::new(0.0, 48.0, 56.0);
const CAMERA_TARGET: Vec3 = Vec3::new(0.0, 0.0, 4.0);

const SUN_ILLUMINANCE: f32 = 8_000.0;
/// Light from everywhere, so faces turned away from the sun stay readable. Not a second directional
/// light: the browser (WebGL2) draws only one.
const AMBIENT_BRIGHTNESS: f32 = 600.0;

/// The module selection plus this game's own setup.
pub fn add_arena_plugins(app: &mut App) {
    // Each module's numbers are its config defaults.
    app.add_plugins((
        ArenaModulePlugin,
        PlayerSpawnModulePlugin,
        CharacterMovementModulePlugin,
        ThirdPersonCameraModulePlugin,
        HealthModulePlugin,
        MatchModulePlugin,
    ))
    .add_systems(
        Startup,
        (load_chosen_map, spawn_view, start_playing).chain(),
    );
}

/// Loads the map through its command, like any other entry point.
fn load_chosen_map(world: &mut World) {
    dispatch_command_line(world, "arena startup", &format!("arena.load {MAP_ID}"));
}

/// Someone who just starts the game plays alone; a server, a client and a script decide for
/// themselves who plays.
fn plays_alone(scripted: bool, server: bool, client: bool) -> bool {
    !(scripted || server || client)
}

fn start_playing(world: &mut World) {
    dispatch_command_line(
        world,
        "arena startup",
        &format!("shoulder.follow {CAMERA_NAME}"),
    );
    if plays_alone(
        world.contains_resource::<CommandScript>(),
        world.contains_resource::<NetServer>(),
        world.contains_resource::<NetClient>(),
    ) {
        dispatch_command_line(
            world,
            "arena startup",
            &format!("spawn.solo {SOLO_PLAYER_ID}"),
        );
    }
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
    commands.insert_resource(GlobalAmbientLight {
        brightness: AMBIENT_BRIGHTNESS,
        ..default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use retreat_engine::{InputActionsPlugin, RetreatCommandsPlugin};
    use retreat_gameplay_api::{PlayerId, SoloPlayer};
    use retreat_module_arena::LoadedArena;

    #[test]
    fn startup_loads_the_map_and_a_lone_player() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::input::InputPlugin,
        ))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins((RetreatCommandsPlugin::default(), InputActionsPlugin));
        add_arena_plugins(&mut app);
        app.update();
        let world = app.world_mut();
        assert_eq!(world.resource::<LoadedArena>().0, MAP_ID);
        assert_eq!(world.query::<&GameCamera>().iter(world).count(), 1);
        assert_eq!(
            world.resource::<SoloPlayer>().0,
            Some(PlayerId(SOLO_PLAYER_ID))
        );
    }

    #[test]
    fn only_an_unscripted_game_without_a_network_gets_a_lone_player() {
        assert!(plays_alone(false, false, false));
        assert!(!plays_alone(true, false, false));
        assert!(!plays_alone(false, true, false));
        assert!(!plays_alone(false, false, true));
    }
}
