//! What makes this game: the selected modules, the map choice and the view and light settings.
//! No gameplay rules here: they belong in Retreat-Modules.

use bevy::prelude::*;
use retreat_engine::net::client::NetClient;
use retreat_engine::net::server::NetServer;
use retreat_engine::{CommandScript, dispatch_command_line};
use retreat_engine_api::GameCamera;
use retreat_gameplay_api::{
    CharacterSize, ItemCatalog, ItemDefinition, ItemId, ItemKind, MAX_PLAYERS, PlayerRoster,
    RosterEntry, SoundCue, WeaponKind,
};
use retreat_module_arena::ArenaModulePlugin;
use retreat_module_character_movement::{CharacterMovementModulePlugin, MovementConfig};
use retreat_module_character_visuals::{
    CharacterModel, CharacterVisualsConfig, CharacterVisualsModulePlugin, Clip,
};
use retreat_module_combat_hud::{CombatHudConfig, CombatHudModulePlugin};
use retreat_module_health::{HealthConfig, HealthModulePlugin};
use retreat_module_inventory::{InventoryConfig, InventoryModulePlugin};
use retreat_module_loot::{LootConfig, LootModulePlugin, PickupKind, PickupLook};
use retreat_module_match::{MatchConfig, MatchModulePlugin};
use retreat_module_player_spawn::PlayerSpawnModulePlugin;
use retreat_module_projectile_launcher::{LauncherConfig, ProjectileLauncherModulePlugin};
use retreat_module_rifle::{RifleConfig, RifleModulePlugin};
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

/// The size of every character: the fox's, so a shot that clearly passes above it misses.
fn character_size() -> CharacterSize {
    CharacterSize {
        radius: 0.35,
        height: 1.2,
        eye_height: 1.0,
    }
}

/// Name and color of every player, in player order. The colors replace the fox's texture
/// (`flat_colors`), so they show as written; the HUD calls and colors players the same way.
const PLAYERS: [(&str, (f32, f32, f32)); MAX_PLAYERS] = [
    ("Red", (0.85, 0.12, 0.1)),
    ("Blue", (0.12, 0.35, 0.95)),
    ("Green", (0.15, 0.7, 0.2)),
    ("Yellow", (0.95, 0.8, 0.1)),
    ("Purple", (0.55, 0.2, 0.85)),
    ("Orange", (0.95, 0.45, 0.05)),
    ("Cyan", (0.1, 0.8, 0.85)),
    ("White", (0.92, 0.92, 0.92)),
];

fn player_roster() -> PlayerRoster {
    PlayerRoster(
        PLAYERS
            .map(|(name, (red, green, blue))| RosterEntry::new(name, Color::srgb(red, green, blue)))
            .to_vec(),
    )
}

/// The match (recorded in CLAUDE.md): three minutes, then the result for eight seconds.
fn match_rules() -> MatchConfig {
    MatchConfig {
        length_seconds: 180.0,
        result_seconds: 8.0,
        points_per_kill: 1,
        points_per_self_kill: -1,
        start_sound: sound("match_start", 0.45),
        end_sound: sound("match_end", 0.45),
    }
}

/// A sound of `assets/sounds/` (made by `scripts/sounds/generate.mjs`) at `volume`. The volumes
/// (recorded in CLAUDE.md) keep what repeats quiet and what matters louder.
fn sound(name: &str, volume: f32) -> Option<SoundCue> {
    Some(SoundCue::new(&format!("sounds/{name}.wav"), volume))
}

fn movement() -> MovementConfig {
    MovementConfig {
        jump_sound: sound("jump", 0.2),
        dash_sound: sound("dash", 0.3),
        ..default()
    }
}

/// The rifle's numbers (recorded in CLAUDE.md): 5 hits kill, about 7 shots a second.
fn rifle() -> RifleConfig {
    RifleConfig {
        damage: 20,
        fire_interval_seconds: 0.15,
        magazine: 30,
        reload_seconds: 1.5,
        range: 100.0,
        shot_sound: sound("rifle_shot", 0.3),
        reload_sound: sound("rifle_reload", 0.35),
    }
}

/// The rockets' numbers (recorded in CLAUDE.md): a direct hit and one rifle shot kill.
fn launcher() -> LauncherConfig {
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

/// The reserve ammunition and what else players carry (recorded in CLAUDE.md). Item ids are
/// stable: commands, scripts and one day saved games name them.
fn items() -> ItemCatalog {
    let ammo = ItemKind::Ammo;
    ItemCatalog(vec![
        ItemDefinition::new("rifle_ammo", "Rifle rounds", ammo(WeaponKind::Rifle), 120),
        ItemDefinition::new("rockets", "Rockets", ammo(WeaponKind::Launcher), 8),
        ItemDefinition::new("health_kit", "Health kit", ItemKind::Heal(50), 3),
        ItemDefinition::new(
            "launcher",
            "Rocket launcher",
            ItemKind::Weapon(WeaponKind::Launcher),
            1,
        ),
    ])
}

/// What every new life starts with. The launcher and its rockets are found on the map.
fn inventory() -> InventoryConfig {
    let start = |item: &str, count| (ItemId::new(item), count);
    InventoryConfig {
        // One slot per item the game has: nothing is ever turned away for lack of room.
        slots: items().0.len() as u32,
        loadout: vec![start("rifle_ammo", 60), start("health_kit", 1)],
        use_sound: sound("heal", 0.35),
    }
}

/// What lies at the map's pickup spots and what the dead leave (recorded in CLAUDE.md).
fn loot() -> LootConfig {
    let kind = |kind: &str,
                contents: &[(&str, u32)],
                respawn_seconds,
                (color, size),
                (sound_name, volume): (&str, f32)| PickupKind {
        kind: kind.to_string(),
        contents: contents
            .iter()
            .map(|&(item, count)| (ItemId::new(item), count))
            .collect(),
        respawn_seconds,
        look: PickupLook { color, size },
        sound: sound(sound_name, volume),
    };
    let health_sound = ("pickup_health", 0.35);
    let ammo_sound = ("pickup_ammo", 0.35);
    let weapon_sound = ("pickup_weapon", 0.45);
    let green = Color::srgb(0.1, 0.9, 0.2);
    let yellow = Color::srgb(1.0, 0.85, 0.1);
    let orange = Color::srgb(1.0, 0.4, 0.05);
    let violet = Color::srgb(0.75, 0.2, 1.0);
    // The shapes tell them apart too: a cube, a flat crate, a short and a long bar.
    let cube = Vec3::splat(0.45);
    let crate_shape = Vec3::new(0.55, 0.25, 0.4);
    let short_bar = Vec3::new(0.65, 0.22, 0.22);
    let long_bar = Vec3::new(1.1, 0.28, 0.28);
    let launcher: &[(&str, u32)] = &[("launcher", 1), ("rockets", 4)];
    LootConfig {
        kinds: vec![
            kind(
                "health",
                &[("health_kit", 1)],
                15.0,
                (green, cube),
                health_sound,
            ),
            kind(
                "ammo",
                &[("rifle_ammo", 30)],
                15.0,
                (yellow, crate_shape),
                ammo_sound,
            ),
            kind(
                "rockets",
                &[("rockets", 4)],
                20.0,
                (orange, short_bar),
                ammo_sound,
            ),
            kind("launcher", launcher, 30.0, (violet, long_bar), weapon_sound),
        ],
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

fn health() -> HealthConfig {
    HealthConfig {
        max_health: 100,
        respawn_delay_seconds: 3.0,
        spawn_protection_seconds: 1.0,
        fall_out_height: -10.0,
        death_sound: sound("death", 0.45),
        respawn_sound: sound("respawn", 0.3),
    }
}

fn combat_hud() -> CombatHudConfig {
    CombatHudConfig {
        hit_marker_seconds: 0.2,
        hit_marker_text: "HIT".to_string(),
        font_size: 28.0,
        crosshair_size: 24.0,
        scoreboard_font_size: 22.0,
        kill_feed_seconds: 4.0,
        kill_feed_lines: 4,
        pickup_seconds: 2.0,
        panel: Color::srgba(0.0, 0.0, 0.0, 0.85),
        hit_sound: sound("hit", 0.35),
    }
}

/// How characters look: the fox, one color per player.
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
        flat_colors: true,
        ..default()
    }
}

/// The module selection plus this game's own setup.
pub fn add_arena_plugins(app: &mut App) {
    // Modules not configured here use their config defaults.
    app.insert_resource(character_size())
        .insert_resource(character_visuals())
        .insert_resource(rifle())
        .insert_resource(launcher())
        .insert_resource(health())
        .insert_resource(combat_hud())
        .insert_resource(match_rules())
        .insert_resource(player_roster())
        .insert_resource(items())
        .insert_resource(inventory())
        .insert_resource(loot())
        .insert_resource(movement())
        .insert_resource(ClearColor(SKY_COLOR));
    app.add_plugins((
        ArenaModulePlugin,
        PlayerSpawnModulePlugin,
        CharacterMovementModulePlugin,
        ThirdPersonCameraModulePlugin,
        CharacterVisualsModulePlugin,
        HealthModulePlugin,
        InventoryModulePlugin,
        LootModulePlugin,
        MatchModulePlugin,
        RifleModulePlugin,
        ProjectileLauncherModulePlugin,
        CombatHudModulePlugin,
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
    use retreat_engine::{InputActionsPlugin, PhysicsPlugin, RetreatCommandsPlugin};
    use retreat_gameplay_api::{PlayerId, SoloPlayer};
    use retreat_module_arena::LoadedArena;

    #[test]
    fn startup_loads_the_map_and_a_lone_player() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            TransformPlugin,
            bevy::input::InputPlugin,
        ))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins((
            RetreatCommandsPlugin::default(),
            PhysicsPlugin,
            InputActionsPlugin,
        ));
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
    fn every_pickup_spot_of_the_map_holds_something() {
        let config = loot();
        for spot in retreat_module_arena::graybox_map().pickup_spots {
            assert!(config.kind(&spot.kind).is_some(), "{}", spot.name);
        }
    }

    #[test]
    fn every_sound_the_game_names_is_in_its_assets() {
        let (rifle, launcher, health) = (rifle(), launcher(), health());
        let (hud, rules, movement) = (combat_hud(), match_rules(), movement());
        let loot = loot();
        let pickups = loot
            .kinds
            .into_iter()
            .map(|kind| kind.sound)
            .chain([loot.drop_sound, inventory().use_sound]);
        let cues = [
            rifle.shot_sound,
            rifle.reload_sound,
            launcher.launch_sound,
            launcher.explosion_sound,
            launcher.reload_sound,
            health.death_sound,
            health.respawn_sound,
            hud.hit_sound,
            rules.start_sound,
            rules.end_sound,
            movement.jump_sound,
            movement.dash_sound,
        ];
        for cue in cues.into_iter().chain(pickups) {
            let cue = cue.expect("the game gives every sound a cue");
            let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(&cue.path);
            assert!(file.is_file(), "{} does not exist", file.display());
            assert_eq!(cue.validate(), Ok(()));
        }
    }

    #[test]
    fn only_an_unscripted_game_without_a_network_gets_a_lone_player() {
        assert!(plays_alone(false, false, false));
        assert!(!plays_alone(true, false, false));
        assert!(!plays_alone(false, true, false));
        assert!(!plays_alone(false, false, true));
    }
}
