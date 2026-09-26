//! Run from the organization root with `cargo run -p q_cam --example camera_test --features q_cam/free_camera`.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_8};

use avian3d::prelude::*;
use bevy::{
    audio::AudioPlugin,
    camera_controller::free_camera::{FreeCamera, FreeCameraState},
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use q_cam::{
    free::{FreeCameraInput, FreeCameraInputPlugin},
    tracking::{SpringArm, SpringArmCameraPlugin, SpringArmCameraSettings},
};

const FREE_HELP: &str = "Camera: Free | Tab: Tracking\nRMB: hold / M: toggle capture | Esc: release\nWASD: fly | Q/E: vertical | Shift: faster | Wheel: speed";
const TRACKING_HELP: &str = "Camera: Tracking | Tab: Free\nClick: capture | Esc: release\nMouse: orbit | Wheel: FOV zoom | Arrow keys: move target";

#[derive(Component)]
struct Target;

#[derive(Component)]
struct Help;

#[derive(Resource, Default)]
struct TrackingCapture {
    armed: bool,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<AudioPlugin>(),
            PhysicsPlugins::default(),
            SpringArmCameraPlugin,
            FreeCameraInputPlugin,
        ))
        .insert_resource(Gravity(Vec3::ZERO))
        .init_resource::<TrackingCapture>()
        .add_systems(Startup, setup)
        .add_systems(Update, (controls, move_target))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let target = commands
        .spawn((
            Target,
            Mesh3d(meshes.add(Sphere::new(0.65))),
            MeshMaterial3d(materials.add(Color::srgb(0.9, 0.65, 0.15))),
            Transform::from_xyz(0., 1., 0.),
        ))
        .id();
    let mut free_state = FreeCameraState::default();
    free_state.enabled = false;
    commands.spawn((
        Camera3d::default(),
        FreeCamera::default(),
        free_state,
        FreeCameraInput::default(),
        Transform::from_xyz(-2.5, 4.5, 9.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Camera {
            is_active: false,
            ..default()
        },
        SpringArm {
            target_offset: Vec3::Y,
            ..SpringArm::new(target)
        },
        Transform::default(),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50., 50.))),
        MeshMaterial3d(materials.add(Color::srgb(0.25, 0.35, 0.3))),
        Transform::default(),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
    ));
    let wall = meshes.add(Cuboid::new(4., 3., 0.5));
    let wall_material = materials.add(Color::srgb(0.35, 0.55, 0.85));
    for position in [Vec3::new(0., 1.5, 5.), Vec3::new(5., 1.5, -2.)] {
        commands.spawn((
            Mesh3d(wall.clone()),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(position),
            RigidBody::Static,
            Collider::cuboid(4., 3., 0.5),
        ));
    }
    commands.spawn((PointLight::default(), Transform::from_xyz(4., 8., 4.)));
    let ui_camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
        ))
        .id();
    commands.spawn((
        UiTargetCamera(ui_camera),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            padding: UiRect::all(px(12)),
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
        children![(
            Help,
            Text::new(FREE_HELP),
            TextFont::default().with_font_size(16.)
        )],
    ));
}

fn release(cursor: &mut CursorOptions, state: &mut FreeCameraState, input: &mut FreeCameraInput) {
    cursor.grab_mode = CursorGrabMode::None;
    cursor.visible = true;
    input.reset(state);
}

fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    settings: Res<SpringArmCameraSettings>,
    mut capture: ResMut<TrackingCapture>,
    mut window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut free: Single<
        (&mut Camera, &mut FreeCameraState, &mut FreeCameraInput),
        (With<FreeCamera>, Without<SpringArm>),
    >,
    mut tracking: Single<
        (&mut Camera, &mut Transform, &mut Projection),
        (With<SpringArm>, Without<FreeCamera>),
    >,
    mut help: Single<&mut Text, With<Help>>,
) {
    let (window, cursor) = &mut *window;
    let (free_camera, state, input) = &mut *free;
    let (tracking_camera, tracking_transform, projection) = &mut *tracking;
    if !window.focused {
        capture.armed = false;
        release(cursor, state, input);
        return;
    }
    if keys.just_pressed(KeyCode::Tab) {
        release(cursor, state, input);
        capture.armed = false;
        free_camera.is_active = !free_camera.is_active;
        tracking_camera.is_active = !tracking_camera.is_active;
        help.0 = if free_camera.is_active {
            FREE_HELP
        } else {
            TRACKING_HELP
        }
        .into();
    }
    if keys.just_pressed(KeyCode::Escape) && cursor.grab_mode != CursorGrabMode::None {
        release(cursor, state, input);
        capture.armed = false;
    }
    if !tracking_camera.is_active {
        return;
    }
    if !mouse.pressed(MouseButton::Left) {
        capture.armed = true;
    } else if mouse.just_pressed(MouseButton::Left) && capture.armed {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
        capture.armed = false;
    }
    if cursor.grab_mode != CursorGrabMode::Locked {
        return;
    }
    let (yaw, pitch, _) = tracking_transform.rotation.to_euler(EulerRot::YXZ);
    tracking_transform.rotation = Quat::from_euler(
        EulerRot::YXZ,
        yaw - motion.delta.x * 0.01,
        (pitch - motion.delta.y * 0.01).clamp(-FRAC_PI_8, FRAC_PI_8),
        0.,
    );
    if let Projection::Perspective(projection) = &mut **projection {
        projection.fov = (projection.fov + scroll.delta.y * 0.1 * settings.zoom_speed)
            .clamp(FRAC_PI_8, FRAC_PI_2);
    }
}

fn move_target(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut target: Single<&mut Transform, With<Target>>,
) {
    let direction = Vec3::new(
        keys.pressed(KeyCode::ArrowRight) as i32 as f32
            - keys.pressed(KeyCode::ArrowLeft) as i32 as f32,
        0.,
        keys.pressed(KeyCode::ArrowDown) as i32 as f32
            - keys.pressed(KeyCode::ArrowUp) as i32 as f32,
    );
    target.translation += direction.normalize_or_zero() * 4. * time.delta_secs();
}
