---
name: q-cam
description: Deep dive on q_cam (lib/q_cam), qproj's camera utilities — SpringArm follow camera with avian3d sphere-sweep collision (SpringArmCameraPlugin, SpringArmCollisionFilter, SpringArmCameraSettings) and FreeCameraInput gating for Bevy's free camera (feature free_camera). Use when working on quell's camera, camera collision, cursor grab behavior, or the camera_test example/screen.
---

# q_cam

Source: `lib/q_cam/src/{tracking.rs, free.rs}` with tests in
`src/tracking/{follow,collision}_tests.rs` and `src/free/tests.rs`. Example:
`examples/camera_test.rs` (needs `--features free_camera`). Design:
`.docs/designs/quell-camera-design.md`. Deps: bevy 0.19, avian3d 0.7.

## SpringArm (always available)

```rust
app.add_plugins(SpringArmCameraPlugin);
commands.spawn((Camera3d::default(), Transform::default(),
                SpringArm { target_offset: Vec3::Y, ..SpringArm::new(player) }));
```

- Each frame in `PostUpdate`, after `TransformSystems::Propagate` and before
  `VisibilitySystems::UpdateFrusta`, places the camera at
  `target + target_offset + back() * distance`. **Orientation is yours**: set
  the camera's `Transform.rotation` (orbit input) elsewhere; the arm only
  positions along `back()`.
- Camera must be **unparented with no children** (its `GlobalTransform` is
  written directly) and have `Camera`.
- `length` finite ≥ 0, `probe_radius` finite > 0; invalid values skip the
  camera with `warn_once!` (fields are reflect-editable live).
- Collision: sphere cast via avian `SpatialQuery` if present; otherwise
  follows unobstructed. Ignores the camera, the target, colliders of the
  target's body, sensors, and memberships in
  `SpringArmCollisionFilter.excluded_memberships`. Games should exclude their
  player layer there. Probe clearance ≠ near-plane clearance.
- `SpringArmCameraSettings { zoom_speed }` is a tuning resource for
  integrations (q_cam doesn't read it).

## FreeCameraInput (feature `free_camera`)

`FreeCameraInputPlugin` adds Bevy's `FreeCameraPlugin` and a gate system in
`RunFixedMainLoop` (`BeforeFixedMainLoop`, before
`run_freecamera_controller`). Attach `FreeCameraInput` next to `FreeCamera`.

- Needs an armed tick first: a fresh/reset controller stays disabled until
  grab keys/buttons and Escape are released.
- Inactive camera or unfocused window → releases cursor and resets.
- `FreeCameraInput::reset(&mut state)` when releasing the cursor externally.
- Removing an active `FreeCamera` restores the cursor.

Tests use `q_test_harness`'s `InputTestPlugin` + `AppExt::step`; follow
`src/free/tests.rs`.
