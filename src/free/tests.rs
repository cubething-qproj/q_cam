use std::time::Duration;

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraState},
    ecs::schedule::ScheduleLabel,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use q_test_harness::prelude::{AppExt as _, InputTestPlugin};

use super::{FreeCameraInput, FreeCameraInputPlugin};

struct Fixture {
    app: App,
    camera: Entity,
    window: Entity,
}

impl Fixture {
    fn new() -> Self {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputTestPlugin, FreeCameraInputPlugin));
        app.finish();
        app.cleanup();
        let window = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<PrimaryWindow>>()
                .single(world)
                .unwrap()
        };
        let camera = app.world_mut().spawn(Self::camera_bundle()).id();
        Self {
            app,
            camera,
            window,
        }
    }

    fn camera_bundle() -> impl Bundle {
        let mut state = FreeCameraState::default();
        state.enabled = false;
        (
            Camera::default(),
            FreeCamera::default(),
            state,
            FreeCameraInput::default(),
            Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        )
    }

    fn tick(&mut self) {
        self.app.step(
            Duration::from_millis(16),
            [PreUpdate.intern(), RunFixedMainLoop.intern()],
        );
    }

    fn key(&mut self, key: KeyCode, pressed: bool) {
        self.app.key(key, pressed);
    }

    fn mouse(&mut self, button: MouseButton, pressed: bool) {
        self.app.mouse(button, pressed);
    }

    fn motion(&mut self) {
        self.app.motion(Vec2::new(12.0, 6.0));
    }

    fn transform(&self) -> Transform {
        *self.app.world().get::<Transform>(self.camera).unwrap()
    }

    fn state(&self) -> &FreeCameraState {
        self.app
            .world()
            .get::<FreeCameraState>(self.camera)
            .unwrap()
    }

    fn assert_cursor(&self, captured: bool) {
        let cursor = self.app.world().get::<CursorOptions>(self.window).unwrap();
        assert_eq!(cursor.visible, !captured);
        assert_eq!(
            cursor.grab_mode,
            if captured {
                CursorGrabMode::Locked
            } else {
                CursorGrabMode::None
            }
        );
    }

    fn assert_captured(&self) {
        assert!(self.state().enabled);
        self.assert_cursor(true);
    }

    fn assert_stopped(&self) {
        assert!(!self.state().enabled);
        assert_eq!(self.state().velocity, Vec3::ZERO);
        assert!(self.state().rotation_curve.is_none());
        self.assert_cursor(false);
    }
}

#[test]
fn rmb_release_stops_translation_and_pending_snap_in_the_same_frame() {
    let mut f = Fixture::new();
    f.tick();
    f.key(KeyCode::KeyW, true);
    f.key(KeyCode::Numpad7, true);
    f.mouse(MouseButton::Right, true);
    f.tick();
    f.assert_captured();
    assert!(f.state().rotation_curve.is_some());
    assert_ne!(f.state().velocity, Vec3::ZERO);
    let stopped_at = f.transform();

    f.mouse(MouseButton::Right, false);
    f.key(KeyCode::Numpad3, true);
    f.motion();
    f.tick();
    f.assert_stopped();
    assert_eq!(f.transform(), stopped_at);
    f.tick();
    assert_eq!(f.transform(), stopped_at);

    f.mouse(MouseButton::Right, true);
    f.tick();
    f.assert_captured();
    assert_ne!(f.transform().translation, stopped_at.translation);
    assert_eq!(f.transform().rotation, stopped_at.rotation);
    assert!(f.state().rotation_curve.is_none());
}

#[test]
fn m_toggle_and_rmb_are_or_combined_without_repeating_held_toggles() {
    let mut f = Fixture::new();
    f.tick();
    f.key(KeyCode::KeyW, true);
    f.key(KeyCode::KeyM, true);
    f.tick();
    f.assert_captured();
    let moving = f.transform();
    f.tick();
    f.assert_captured();
    assert_ne!(f.transform().translation, moving.translation);
    f.key(KeyCode::KeyM, false);
    f.tick();
    f.assert_captured();

    f.mouse(MouseButton::Right, true);
    f.tick();
    f.mouse(MouseButton::Right, false);
    f.tick();
    f.assert_captured();

    f.mouse(MouseButton::Right, true);
    f.key(KeyCode::KeyM, true);
    f.tick();
    f.assert_captured();
    let stopped_at = f.transform();
    f.key(KeyCode::KeyM, false);
    f.mouse(MouseButton::Right, false);
    f.tick();
    f.assert_stopped();
    assert_eq!(f.transform(), stopped_at);

    f.key(KeyCode::KeyM, true);
    f.tick();
    f.assert_captured();
    f.key(KeyCode::KeyM, false);
    f.tick();
    let stopped_at = f.transform();
    f.key(KeyCode::KeyM, true);
    f.tick();
    f.assert_stopped();
    assert_eq!(f.transform(), stopped_at);
}

#[test]
fn new_camera_waits_for_all_held_capture_controls_to_be_released() {
    let mut f = Fixture::new();
    let start = f.transform();
    f.key(KeyCode::KeyW, true);
    f.key(KeyCode::KeyM, true);
    f.mouse(MouseButton::Right, true);
    for _ in 0..2 {
        f.tick();
        f.assert_stopped();
        assert_eq!(f.transform(), start);
    }
    f.mouse(MouseButton::Right, false);
    f.tick();
    f.assert_stopped();
    assert_eq!(f.transform(), start);
    f.key(KeyCode::KeyM, false);
    f.tick();
    f.assert_stopped();
    assert_eq!(f.transform(), start);

    f.key(KeyCode::KeyM, true);
    f.tick();
    f.assert_captured();
    assert_ne!(f.transform().translation, start.translation);
}

#[test]
fn inactive_or_unfocused_camera_stops_and_does_not_replay_held_capture() {
    for lose_focus in [false, true] {
        let mut f = Fixture::new();
        f.tick();
        f.key(KeyCode::KeyW, true);
        f.key(KeyCode::KeyM, true);
        f.key(KeyCode::Numpad7, true);
        f.mouse(MouseButton::Right, true);
        f.tick();
        f.assert_captured();
        assert!(f.state().rotation_curve.is_some());
        let stopped_at = f.transform();

        if lose_focus {
            f.app
                .world_mut()
                .get_mut::<Window>(f.window)
                .unwrap()
                .focused = false;
        } else {
            f.app
                .world_mut()
                .get_mut::<Camera>(f.camera)
                .unwrap()
                .is_active = false;
        }
        f.motion();
        f.tick();
        f.assert_stopped();
        assert_eq!(f.transform(), stopped_at);

        f.app
            .world_mut()
            .get_mut::<Window>(f.window)
            .unwrap()
            .focused = true;
        f.app
            .world_mut()
            .get_mut::<Camera>(f.camera)
            .unwrap()
            .is_active = true;
        f.tick();
        f.assert_stopped();
        assert_eq!(f.transform(), stopped_at);
        f.key(KeyCode::KeyM, false);
        f.mouse(MouseButton::Right, false);
        f.tick();
        f.assert_stopped();
        assert_eq!(f.transform(), stopped_at);

        f.mouse(MouseButton::Right, true);
        f.tick();
        f.assert_captured();
        assert_ne!(f.transform().translation, stopped_at.translation);
        assert_eq!(f.transform().rotation, stopped_at.rotation);
        f.mouse(MouseButton::Right, false);
        f.tick();
        f.assert_stopped();
    }
}

#[test]
fn despawn_unlocks_immediately_and_reentry_clears_upstream_capture_flags() {
    let mut f = Fixture::new();
    f.tick();
    f.key(KeyCode::KeyW, true);
    f.key(KeyCode::KeyM, true);
    f.mouse(MouseButton::Right, true);
    f.tick();
    f.assert_captured();

    f.app.world_mut().despawn(f.camera);
    f.assert_cursor(false);
    f.camera = f.app.world_mut().spawn(Fixture::camera_bundle()).id();
    let start = f.transform();
    f.tick();
    f.assert_stopped();
    assert_eq!(f.transform(), start);
    f.key(KeyCode::KeyM, false);
    f.mouse(MouseButton::Right, false);
    f.tick();
    f.assert_stopped();
    assert_eq!(f.transform(), start);

    f.key(KeyCode::KeyM, true);
    f.tick();
    f.assert_captured();
    assert_ne!(f.transform().translation, start.translation);
    f.key(KeyCode::KeyM, false);
    f.tick();
    f.key(KeyCode::KeyM, true);
    f.tick();
    f.assert_stopped();
}
