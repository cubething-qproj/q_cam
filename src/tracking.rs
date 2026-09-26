use avian3d::prelude::*;
use bevy::{camera::visibility::VisibilitySystems, prelude::*, transform::TransformSystems};

/// An unparented, childless camera following a target in world space.
/// Orbit orientation is owned by the camera's `Transform.rotation`.
#[derive(Component, Debug, Reflect)]
pub struct SpringArm {
    pub target: Entity,
    pub target_offset: Vec3,
    /// Desired sphere-center distance; finite and nonnegative.
    pub length: f32,
    /// Positive, finite probe radius. This does not guarantee near-plane clearance.
    pub probe_radius: f32,
}

impl SpringArm {
    pub fn new(target: Entity) -> Self {
        Self {
            target,
            target_offset: Vec3::ZERO,
            length: 10.,
            probe_radius: 0.3,
        }
    }
}

/// Live camera tuning shared by integrations using a spring arm.
#[derive(Resource, Reflect, Debug)]
#[reflect(Resource)]
pub struct SpringArmCameraSettings {
    pub zoom_speed: f32,
}

impl Default for SpringArmCameraSettings {
    fn default() -> Self {
        Self { zoom_speed: -5. }
    }
}

/// Which collision memberships can obstruct the arm's sphere sweep.
///
/// By default only Avian's default layer blocks the camera. Set
/// `excluded_memberships` to the player layer when integrating with a game:
/// a mixed default/player collider must not obstruct the camera.
#[derive(Resource, Debug, Clone, Copy, Reflect)]
pub struct SpringArmCollisionFilter {
    pub included: LayerMask,
    pub excluded_memberships: LayerMask,
}

impl Default for SpringArmCollisionFilter {
    fn default() -> Self {
        Self {
            included: LayerMask::DEFAULT,
            excluded_memberships: LayerMask::NONE,
        }
    }
}

fn apply(
    targets: Query<&GlobalTransform, Without<SpringArm>>,
    owners: Query<&ColliderOf>,
    solids: Query<&CollisionLayers, Without<Sensor>>,
    // Physics-disabled applications still follow their target without obstruction queries.
    spatial_query: Option<SpatialQuery>,
    collision_filter: Res<SpringArmCollisionFilter>,
    mut cameras: Query<
        (Entity, &SpringArm, &mut Transform, &mut GlobalTransform),
        (With<Camera>, Without<ChildOf>, Without<Children>),
    >,
) {
    let filter = SpatialQueryFilter::from_mask(collision_filter.included);
    for (camera, arm, mut transform, mut global_transform) in &mut cameras {
        let Ok(target) = targets.get(arm.target) else {
            continue;
        };
        // Fields are editable through reflection, so reject invalid live configuration too.
        if !arm.length.is_finite()
            || arm.length < 0.
            || !arm.probe_radius.is_finite()
            || arm.probe_radius <= 0.
        {
            warn_once!(
                "SpringArm requires a finite nonnegative length and finite positive probe radius"
            );
            continue;
        }
        let pivot = target.translation() + arm.target_offset;
        let direction = transform.back();
        let mut distance = 0.;
        if arm.length > f32::EPSILON {
            distance = arm.length;
            if let Some(spatial_query) = &spatial_query {
                let body = owners
                    .get(arm.target)
                    .map_or(arm.target, |owner| owner.body);
                let hit = spatial_query.cast_shape_predicate(
                    &Collider::sphere(arm.probe_radius),
                    pivot,
                    Quat::IDENTITY,
                    direction,
                    &ShapeCastConfig::from_max_distance(arm.length).with_target_distance(0.02),
                    &filter,
                    &|entity| {
                        entity != camera
                            && entity != arm.target
                            && owners.get(entity).map_or(entity, |owner| owner.body) != body
                            && solids.get(entity).is_ok_and(|layers| {
                                layers.memberships.0 & collision_filter.excluded_memberships.0 == 0
                            })
                    },
                );
                if let Some(hit) = hit {
                    // Center travel already includes probe clearance. A zero hit collapses
                    // to the pivot; it cannot depenetrate an embedded pivot.
                    distance = hit.distance.clamp(0., arm.length);
                }
            }
        }
        transform.translation = pivot + direction * distance;
        // Target globals are current because this runs after transform propagation.
        // Moving the camera now leaves its global stale. It has no parent or children,
        // so copying its local transform updates world space without another tree pass.
        *global_transform = GlobalTransform::from(*transform);
    }
}

/// Install spring-arm follow after transform propagation and before frustum updates.
/// Requires Avian's spatial query plugin to collide; without it the camera follows unobstructed.
/// Cameras must be unparented and have no transform children for same-frame global sync.
/// The sphere sweep clears the probe radius, not necessarily the camera near plane.
/// Quell should set the filter to include its default layer and exclude its player membership.
pub struct SpringArmCameraPlugin;

impl Plugin for SpringArmCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpringArmCollisionFilter>()
            .init_resource::<SpringArmCameraSettings>()
            .register_type::<SpringArmCameraSettings>()
            .register_type::<SpringArm>()
            .register_type::<SpringArmCollisionFilter>()
            .add_systems(
                PostUpdate,
                apply
                    .after(TransformSystems::Propagate)
                    .before(VisibilitySystems::UpdateFrusta),
            );
    }
}

#[cfg(test)]
#[path = "tracking/collision_tests.rs"]
mod collision_tests;
#[cfg(test)]
#[path = "tracking/follow_tests.rs"]
mod follow_tests;
