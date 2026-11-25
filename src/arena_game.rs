//! What makes this game: the selected modules, the map choice and the view and light settings.
//! No gameplay rules here: they belong in Retreat-Modules.

use bevy::prelude::*;
use retreat_engine::net::client::NetClient;
use retreat_engine::net::server::NetServer;
use retreat_engine::{CommandScript, dispatch_command_line};
use retreat_engine_api::GameCamera;
use retreat_gameplay_api::MAX_PLAYERS;
use retreat_module_arena::ArenaModulePlugin;
use retreat_module_character_movement::CharacterMovementModulePlugin;
use retreat_module_character_visuals::{
    CharacterModel, CharacterVisualsConfig, CharacterVisualsModulePlugin, Clip,
};
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

/// The sky behind the arena.
const SKY_COLOR: Color = Color::srgb(0.53, 0.7, 0.86);

/// The character everyone is shown as, until real models exist: the fox (credits in
/// `assets/CREDITS.md`). The model is about 1.2 m tall at this scale and faces +Z.
const CHARACTER_MODEL_PATH: &str = "models/Fox.glb";
const CHARACTER_MODEL_SCALE: f32 = 0.012;

/// One tint per player, `(red, green, blue)` in linear light. The engine multiplies the fox's orange
/// texture by it, and that texture holds almost no blue, so the blue-ish tints need factors far above
/// 1. Order: red, blue, green, natural, purple, charcoal, cyan, white.
const PLAYER_TINTS: [(f32, f32, f32); MAX_PLAYERS] = [
    (0.9, 0.12, 0.1),
    (0.08, 0.6, 14.0),
    (0.12, 1.8, 1.0),
    (1.0, 1.0, 1.0),
    (0.35, 0.15, 12.0),
    (0.04, 0.09, 0.5),
    (0.08, 2.6, 18.0),
    (1.3, 3.5, 25.0),
];

/// How characters look: the fox, one tint per player.
fn character_visuals() -> CharacterVisualsConfig {
    let clip = |name: &str, rate: f32, matched_speed: Option<f32>| Clip {
        name: name.to_string(),
        rate,
        matched_speed,
    };
    CharacterVisualsConfig {
        model: Some(CharacterModel {
            path: CHARACTER_MODEL_PATH.to_string(),
            scale: CHARACTER_MODEL_SCALE,
            yaw: std::f32::consts::PI,
            // The fox has no idle or jump clip: it looks around when standing and holds a slow
            // running stride in the air.
            idle: clip("Survey", 1.0, None),
            walk: clip("Walk", 1.0, Some(3.0)),
            run: clip("Run", 1.0, Some(7.0)),
            air: clip("Run", 0.4, None),
        }),
        colors: PLAYER_TINTS
            .map(|(red, green, blue)| Color::linear_rgb(red, green, blue))
            .to_vec(),
        ..default()
    }
}

/// The module selection plus this game's own setup.
pub fn add_arena_plugins(app: &mut App) {
    // Modules not configured here use their config defaults.
    app.insert_resource(character_visuals())
        .insert_resource(ClearColor(SKY_COLOR));
    app.add_plugins((
        ArenaModulePlugin,
        PlayerSpawnModulePlugin,
        CharacterMovementModulePlugin,
        ThirdPersonCameraModulePlugin,
        CharacterVisualsModulePlugin,
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
