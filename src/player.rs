use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{CameraPosition, GameLayers, PlayerCount};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        //app.add_systems(Startup, spawn_players);
        app.add_systems(
            Update,
            (
                get_player_inputs,
                apply_spring_force,
                apply_movement,
                apply_ground_friction,
                apply_turning,
                apply_stability,
                apply_anti_slip,
                camera_lerp_to,
                camera_look_at,
                reset_dino,
            ),
        );
    }
}

pub fn spawn_players(
    commands: &mut Commands,
    asset_server: &AssetServer,
    player_count: &PlayerCount,
) {
    //// Spawn splitscreen cameras
    for index in 0..player_count.0 {
        //Spawn player
        let player = commands.spawn(player_bundle(index, asset_server)).id();

        // Spawn camera root
        let camera_root = commands.spawn(camera_root_bundle(index, player)).id();

        // Spawn camera
        let camera = commands
            .spawn(camera_bundle(index, camera_root, player_count.0))
            .id();

        // Spawn ui for each player
        commands.spawn(ui_bundle(index, camera));
    }
}


#[derive(Component)]
pub struct PlayerRelatedComponent;


#[derive(Component, Default)]
pub struct PlayerInput {
    pub index: usize,
    pub steer: f32,
    pub throttle: f32,
}

#[derive(Component, Default)]
pub struct Floating {
    pub height: f32,
    pub force: f32,
    pub damping: f32,
}

#[derive(Component)]
#[relationship(relationship_target = LookAtTarget)]
struct LookAt(Entity);

#[derive(Component)]
#[relationship_target(relationship = LookAt)]
struct LookAtTarget(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = LerpTarget)]
struct LerpTo(Entity);

#[derive(Component)]
#[relationship_target(relationship = LerpTo)]
struct LerpTarget(Vec<Entity>);

pub fn player_bundle(player_index: usize, asset_server: &AssetServer) -> impl Bundle {
    // Variables for the bundle
    let bundle_name: String = format!("Player{}", player_index);

    // The actuall bundle
    (
        Name::new(bundle_name),
        (
            PlayerRelatedComponent,
            NoAutoMass,
            NoAutoCenterOfMass,
            TransformInterpolation,
        ),
        InheritedVisibility::VISIBLE,
        PlayerInput {
            index: player_index,
            steer: 0.0,
            throttle: 0.0,
        },
        Transform::from_xyz(0.0, 5.0, 0.0),
        Floating {
            height: 0.75,
            force: 300.0,
            damping: 15.0,
        },
        RigidBody::Dynamic,
        RayCaster::new(Vec3::ZERO, Dir3::NEG_Y)
            .with_max_distance(1.0)
            .with_max_hits(1)
            .with_query_filter(SpatialQueryFilter::from_mask(GameLayers::Default)),
        Mass(1.0),
        CenterOfMass::new(0.0, -1.0, 0.0),
        LinearDamping(1.0),
        AngularDamping(4.0),
        Friction::new(0.0).with_combine_rule(CoefficientCombine::Min),
        children![
            (
                Name::new("Dino mesh"),
                Transform::from_xyz(0.0, -0.5, 0.0),
                SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("dino_scooter_01_sk.glb"))),
                //SceneRoot(
                //    asset_server
                //        .load(GltfAssetLabel::Scene(0).from_asset("dino_scooter_01_sk.glb"))
                //)
            ),
            (
                Name::new("Capsule Collider"),
                //Transform::from_rotation(Quat::from_rotation_x(1.0)),
                Transform::IDENTITY.aligned_by(Vec3::Y, Dir3::NEG_Z, Vec3::X, Dir3::X),
                Collider::capsule(0.5, 1.0),
                CollisionLayers::new(GameLayers::Player, LayerMask::ALL),
            ),
        ],
    )
}

fn get_player_inputs(mut query: Query<&mut PlayerInput>, keys: Res<ButtonInput<KeyCode>>) {
    for mut input in &mut query {
        input.steer = 0.0;
        input.throttle = 0.0;

        if keys.pressed(KeyCode::KeyA) {
            input.steer += 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            input.steer -= 1.0;
        }

        if keys.pressed(KeyCode::KeyW) {
            input.throttle += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            input.throttle -= 1.0;
        }
    }
}

fn apply_spring_force(mut query: Query<(&Floating, &RayHits, Forces)>) {
    for (floating, hits, mut forces) in &mut query {
        for hit in hits.iter_sorted() {
            let distance = hit.distance;
            let normal = hit.normal;
            let normal_vel = Vec3::dot(normal, forces.linear_velocity());
            let displacement = floating.height - distance;
            let force = (floating.force * displacement) - (normal_vel * floating.damping);
            forces.apply_force(normal * force);
        }
    }
}

fn apply_ground_friction(mut query: Query<(&RayHits, &mut LinearDamping)>) {
    for (hits, mut damping) in &mut query {
        if hits.0.is_empty() {
            damping.0 = 0.0
        } else {
            damping.0 = 2.0;
        }
    }
}

fn apply_movement(mut query: Query<(Forces, &RayHits, &Transform, &PlayerInput)>) {
    for (mut forces, ray_hits, transform, input) in &mut query {
        // Only apply movement when the player is "on" the ground
        if ray_hits.0.is_empty() {
            continue;
        }

        forces.apply_force(transform.forward() * input.throttle * 50.0);
    }
}

fn apply_turning(mut query: Query<(Forces, &Transform, &RayHits, &PlayerInput)>) {
    for (mut forces, transform, hits, input) in &mut query {
        let mut dir_mult: f32 = 1.0;
        // Reduce turn speed while airborne
        if hits.0.is_empty() {
            dir_mult = 0.25;
        }

        let mut turn_dir = input.steer * dir_mult;
        if input.throttle < 0.0 {
            turn_dir = turn_dir * -1.0;
        }

        forces.apply_local_torque(transform.up() * turn_dir * 2.0);
    }
}

fn apply_stability(mut query: Query<(&Transform, &RayHits, Forces), With<Floating>>) {
    for (transform, ray_hits, mut forces) in &mut query {
        for hit in ray_hits.iter_sorted() {
            let normal = hit.normal;
            let forward_dot = Vec3::dot(*transform.back(), normal);
            let right_dot = Vec3::dot(*transform.right(), normal);
            let stabilize_vector =
                (*transform.right() * forward_dot) + (*transform.forward() * right_dot);
            forces.apply_torque(stabilize_vector * 10.0);
        }
    }
}

fn apply_anti_slip(mut query: Query<(Forces, &Transform, &RayHits)>, time: Res<Time>) {
    for (mut forces, transform, hits) in &mut query {
        if hits.0.is_empty() {
            continue;
        }

        let slip_dir = *transform.right();
        let vel_in_slip_dir = Vec3::dot(forces.linear_velocity(), slip_dir);
        if !vel_in_slip_dir.is_nan() && vel_in_slip_dir != 0.0 {
            let anti_slip_force = -(vel_in_slip_dir * 0.125) / time.delta_secs();
            forces.apply_force(transform.right() * anti_slip_force);
        }
    }
}

fn reset_dino(mut query: Query<(&mut Transform, &mut LinearVelocity), With<Floating>>) {
    for (mut transform, mut velocity) in &mut query {
        if transform.translation.y < -10.0 {
            transform.translation = Vec3::new(0.0, 2.0, 0.0);
            velocity.0 = Vec3::ZERO;
        }
    }
}

//

fn camera_root_bundle(player_index: usize, player: Entity) -> impl Bundle {
    let bundle_name: String = format!("Player{}CameraRoot", player_index);

    (
        Name::new(bundle_name),
        PlayerRelatedComponent,
        Transform::default(),
        // Make the camera root follow the player
        LerpTo(player),
        LookAt(player),
        InheritedVisibility::VISIBLE,
    )
}

fn camera_bundle(player_index: usize, camera_root: Entity, player_count: usize) -> impl Bundle {
    let bundle_name: String = format!("Player{}Camera", player_index);

    let columns_count = (player_count as f32).sqrt().ceil() as usize;

    let x_pos = (player_index % columns_count) as u32;
    let y_pos = (player_index / columns_count) as u32;

    (
        Name::new(bundle_name),
        PlayerRelatedComponent,
        // Attach the camera to the camera root
        ChildOf(camera_root),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera3d::default(),
        Camera {
            order: player_index as isize,
            ..default()
        },
        CameraPosition {
            pos: UVec2::new(x_pos, y_pos),
        },
    )
}

fn camera_look_at(
    mut look_at_query: Query<(&mut Transform, &LookAt), Without<Floating>>,
    look_at_target_query: Query<&Transform, (With<LookAtTarget>, Without<LookAt>)>,
    timer: Res<Time>,
) {
    for (mut transform, look_at) in &mut look_at_query {
        if let Ok(target_transform) = look_at_target_query.get(look_at.0) {
            let look_at_position =
                target_transform.translation + (target_transform.forward() * 3.0);
            let new_rotaion = transform.looking_at(look_at_position, Vec3::Y);
            //transform.rotation = look_at_rot.rotation;
            transform.rotation = transform
                .rotation
                .lerp(new_rotaion.rotation, timer.delta_secs() * 5.0);
        }
    }
}

fn camera_lerp_to(
    mut lerp_to_query: Query<(&mut Transform, &LerpTo), Without<Floating>>,
    lerp_target_query: Query<&Transform, (With<LerpTarget>, Without<LerpTo>)>,
    timer: Res<Time>,
) {
    for (mut transform, lerp_to) in &mut lerp_to_query {
        if let Ok(target_transform) = lerp_target_query.get(lerp_to.0) {
            // Move the lerp to entity towards the lerp_target entity
            transform.translation = transform
                .translation
                .lerp(target_transform.translation, timer.delta_secs() * 10.0);
        }
    }
}

//

fn ui_bundle(player_index: usize, camera: Entity) -> impl Bundle {
    let bundle_name: String = format!("Player{}Ui", player_index);

    (
        Name::new(bundle_name),
        PlayerRelatedComponent,
        UiTargetCamera(camera),
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        children![(
            Text::new(format!("Player {}", player_index + 1)),
            Node {
                position_type: PositionType::Absolute,
                top: percent(12),
                left: percent(12),
                ..default()
            },
        ),],
    )
}
