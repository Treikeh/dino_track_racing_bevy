use avian3d::prelude::*;
use bevy::{
    camera::{Viewport, visibility::RenderLayers},
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::{WindowResized, WindowResolution},
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

use crate::player::PlayerPlugin;


mod player;


#[derive(PhysicsLayer, Default)]
enum GameLayers {
    #[default]
    Default,
    Player,
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
struct PlayerCount(usize);

#[derive(Resource)]
struct DinoCount(usize);

#[derive(Component)]
struct FpsLabel;

#[derive(Event)]
struct UpdateViewports;


fn main() {
    App::new()
        .insert_resource(PlayerCount(1))
        .insert_resource(DinoCount(0))
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            PhysicsDebugPlugin::default(),
            PlayerPlugin,
            FrameTimeDiagnosticsPlugin::default(),
            EguiPlugin::default(),
            WorldInspectorPlugin::new(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (set_camera_viewports, log_fps))
        .add_observer(update_camera_viewports)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Spawn platform
    const BASE_RADIUS: f32 = 10.0;
    commands.spawn((
        Name::new("Platform"),
        Mesh3d(meshes.add(Cylinder::new(BASE_RADIUS, 0.1))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        RigidBody::Static,
        Collider::cylinder(BASE_RADIUS, 0.1),
        CollisionLayers::new(GameLayers::Default, LayerMask::ALL),
    ));

    // Spawn skylight
    commands.spawn((
        DirectionalLight {
            illuminance: 3_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::default().looking_at(Vec3::new(-1.0, -0.7, -1.0), Vec3::X),
    ));

    // Spawn track
    commands.spawn((
        Name::new("Track"),
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("track_01_sm.glb"))),
        RigidBody::Static,
        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMeshWithConfig(
            TrimeshFlags::MERGE_DUPLICATE_VERTICES,
        ))
        .with_default_layers(CollisionLayers::new(GameLayers::Default, LayerMask::ALL)),
    ));

    // Spawn a sensor
    commands
        .spawn((
            Name::new("Sensor"),
            Collider::sphere(1.0),
            Sensor,
            CollisionEventsEnabled,
            CollisionLayers::new(GameLayers::Player, LayerMask::ALL),
        ))
        .observe(detect_collision);

    let ui_camera = commands.spawn((
        Name::new("UiCamera"),
        Camera2d,
        RenderLayers::layer(1),
        Camera {
            order: 100,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    )).id();

    // Spawn ui
    commands
        .spawn((
            Name::new("FpsCounter"),
            FpsLabel,
            UiTargetCamera(ui_camera),
            Node {
                top: percent(50),
                left: percent(50),
                ..default()
            },
            Text::new("Click me"),
        ))
        // Make text red when hovering over with the mouse pointer
        .observe(text_hover_start)
        // Make text white when stoping the mouse pointer hover
        .observe(text_hover_end)
        .observe(spawn_new_players);
        //.observe(spawn_dino);

    commands.spawn((
        Name::new("PlayerCountIncreaseButton"),
        UiTargetCamera(ui_camera),
        Node {
            top: percent(50),
            left: percent(25),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Text::new("Increase player count")
    ))
    .observe(text_hover_start)
    .observe(text_hover_end)
    .observe(increase_player_count);


    commands.spawn((
        Name::new("PlayerCountDecreaseButton"),
        UiTargetCamera(ui_camera),
        Node {
            top: percent(50),
            left: percent(75),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Text::new("Decrease player count"),
    ))
    .observe(text_hover_start)
    .observe(text_hover_end)
    .observe(decrease_player_count);
}

#[derive(Component)]
struct CameraPosition {
    pos: UVec2,
}

fn set_camera_viewports(
    mut commands: Commands,
    mut window_resized_reader: MessageReader<WindowResized>,
) {
    for _window_resized in window_resized_reader.read() {
        commands.trigger(UpdateViewports);
    }
}


fn update_camera_viewports(
    _viewports_updated: On<UpdateViewports>,
    window: Single<&Window>,
    mut query: Query<(&CameraPosition, &mut Camera)>,
    player_count: Res<PlayerCount>,
) {
    let columns_count = (player_count.0 as f32).sqrt().ceil() as u32;
    let row_count = (player_count.0 as f32 / columns_count as f32).ceil() as u32;

    info!("Columns {}, Rows {}", columns_count, row_count);

    //window.resolution.set(900.0, 540.0);

    let size_x = window.physical_size().x / columns_count;
    let size_y = window.physical_size().y / row_count;
    let size = UVec2::new(size_x, size_y);

    for (camera_position, mut camera) in &mut query {
        camera.viewport = Some(Viewport {
            physical_position: camera_position.pos * size,
            physical_size: size,
            ..default()
        });
    }
}


fn log_fps(
    mut query: Query<&mut Text, With<FpsLabel>>,
    diagnostics: Res<DiagnosticsStore>,
    dino_count: Res<DinoCount>,
    player_count: Res<PlayerCount>,
) {
    for mut text in &mut query {
        if let Some(value) = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.smoothed())
        {
            text.0 = format!("SpawnPlayer\nFPS: {}\nDinoCount {}\nPlayerCount {}", value, dino_count.0, player_count.0);
        }
    }
}

fn text_hover_start(
    _over: On<Pointer<Over>>,
    mut texts: Query<&mut TextColor>,
) {
    let mut text_color = texts.get_mut(_over.entity).unwrap();
    text_color.0 = Color::srgb(1.0, 0.0, 0.0);
}


fn text_hover_end(
    _out: On<Pointer<Out>>,
    mut texts: Query<&mut TextColor>,
) {
    let mut text_color = texts.get_mut(_out.entity).unwrap();
    text_color.0 = Color::srgb(1.0, 1.0, 1.0);
}


fn spawn_new_players(
    _click: On<Pointer<Click>>,
    mut commands: Commands,
    query: Query<Entity, With<player::PlayerRelatedComponent>>,
    player_count: Res<PlayerCount>,
    asset_server: Res<AssetServer>,
) {
    // Despawn all players
    for entity in &query {
        commands.entity(entity).try_despawn();
    }


    // Spawn new players
    player::spawn_players(&mut commands, &asset_server, &player_count);
    commands.trigger(UpdateViewports);
}


fn increase_player_count(
    _click: On<Pointer<Click>>,
    mut player_count: ResMut<PlayerCount>,
) {
    player_count.0 += 1;
}


fn decrease_player_count(
    _click: On<Pointer<Click>>,
    mut player_count: ResMut<PlayerCount>,
) {
    player_count.0 -= 1;
    if player_count.0 <= 1 
    {
        player_count.0 = 1;
    }
}


fn spawn_dino(
    _click: On<Pointer<Click>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut dino_count: ResMut<DinoCount>,
) {
    for _i in 0..10
    {
        commands.spawn(player::player_bundle(0, &asset_server));
        dino_count.0 += 1;
    }
}

fn detect_collision(event: On<CollisionStart>, name_query: Query<&Name>) {
    //let sensor = event.collider1;
    let other_entity = event.collider2;

    // Print out the name of the entity that is colliding with the sensor
    if let Ok(name) = name_query.get(other_entity) {
        info!("{} is colliding with a sensor", name);
    }
}
