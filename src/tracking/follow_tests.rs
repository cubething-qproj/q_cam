use super::*;

#[derive(Resource, Default)]
struct FrustumPose(Option<GlobalTransform>);

struct Fixture {
    app: App,
    camera: Entity,
    target: Entity,
}

impl Fixture {
    fn new() -> Self {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, SpringArmCameraPlugin));
        app.finish();
        app.cleanup();
        let target = app.world_mut().spawn(Transform::from_xyz(1., 2., 3.)).id();
        let camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                SpringArm::new(target),
                Transform::default(),
            ))
            .id();
        Self {
            app,
            camera,
            target,
        }
    }
}

#[test]
fn inactive_uncaptured_camera_follows_propagated_target_before_frusta() {
    let mut f = Fixture::new();
    f.app.init_resource::<FrustumPose>().add_systems(
        PostUpdate,
        (|camera: Single<&GlobalTransform, With<SpringArm>>, mut pose: ResMut<FrustumPose>| {
            pose.0 = Some(**camera);
        })
        .in_set(VisibilitySystems::UpdateFrusta),
    );
    let world = f.app.world_mut();
    world.get_mut::<Camera>(f.camera).unwrap().is_active = false;
    assert!(world.get::<ChildOf>(f.camera).is_none());
    assert!(world.get::<Children>(f.camera).is_none());
    let parent = world.spawn(Transform::default()).id();
    world.entity_mut(f.target).insert(ChildOf(parent));
    let offset = Vec3::new(0.5, 1., -0.5);
    world.get_mut::<SpringArm>(f.camera).unwrap().target_offset = offset;
    let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    world.get_mut::<Transform>(f.camera).unwrap().rotation = rotation;
    let length = world.get::<SpringArm>(f.camera).unwrap().length;

    for position in [Vec3::new(4., 5., 6.), Vec3::new(-2., 3., 1.)] {
        world.get_mut::<Transform>(parent).unwrap().translation = position;
        world.run_schedule(PostUpdate);
        let expected = position + Vec3::new(1., 2., 3.) + offset + Vec3::X * length;
        let local = world.get::<Transform>(f.camera).unwrap();
        let global = world.get::<GlobalTransform>(f.camera).unwrap();
        let at_frusta = world.resource::<FrustumPose>().0.unwrap();
        assert!(local.translation.abs_diff_eq(expected, 1e-5));
        assert_eq!(local.rotation, rotation);
        assert!(global.translation().abs_diff_eq(expected, 1e-5));
        assert!(at_frusta.to_matrix().abs_diff_eq(global.to_matrix(), 1e-5));
    }
}

#[test]
fn follow_skips_missing_targets_and_handles_zero_length_without_rotating() {
    let mut f = Fixture::new();
    let world = f.app.world_mut();
    let start = Transform::from_xyz(7., 8., 9.).with_rotation(Quat::from_euler(
        EulerRot::YXZ,
        0.4,
        0.2,
        0.,
    ));
    world.entity_mut(f.camera).insert(start);
    world.despawn(f.target);
    world.run_schedule(PostUpdate);
    assert_eq!(*world.get::<Transform>(f.camera).unwrap(), start);

    let target = world.spawn(Transform::from_xyz(-1., 2., 4.)).id();
    let offset = Vec3::Y;
    world.entity_mut(f.camera).insert(SpringArm {
        target,
        target_offset: offset,
        length: 0.,
        ..SpringArm::new(target)
    });
    world.run_schedule(PostUpdate);
    let local = world.get::<Transform>(f.camera).unwrap();
    assert_eq!(local.translation, Vec3::new(-1., 2., 4.) + offset);
    assert_eq!(local.rotation, start.rotation);
    assert_eq!(
        world
            .get::<GlobalTransform>(f.camera)
            .unwrap()
            .translation(),
        local.translation
    );
}

#[test]
fn follow_excludes_parented_cameras_and_cameras_with_transform_children() {
    for parented in [true, false] {
        let mut f = Fixture::new();
        let world = f.app.world_mut();
        let relative = world.spawn(Transform::default()).id();
        if parented {
            world.entity_mut(f.camera).insert(ChildOf(relative));
        } else {
            world.entity_mut(relative).insert(ChildOf(f.camera));
        }
        world.run_schedule(PostUpdate);
        assert_eq!(
            *world.get::<Transform>(f.camera).unwrap(),
            Transform::default()
        );
        assert_eq!(
            world
                .get::<GlobalTransform>(f.camera)
                .unwrap()
                .translation(),
            Vec3::ZERO
        );
    }
}
