//! Input gating for Bevy's free camera controller.
//!
//! Enable this module with `q_cam/free_camera`, add [`FreeCameraInputPlugin`] to the app, and
//! attach [`FreeCameraInput`] alongside Bevy's `FreeCamera` to the camera.

use bevy::{
    app::RunFixedMainLoopSystems,
    camera_controller::free_camera::{
        FreeCamera, FreeCameraPlugin as BevyFreeCameraPlugin, FreeCameraState,
        run_freecamera_controller,
    },
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

/// Predicts capture before upstream flight runs, including the release frame.
#[derive(Component, Default)]
pub struct FreeCameraInput {
    armed: bool,
    toggled: bool,
}

impl FreeCameraInput {
    /// Clears the capture latch and stops movement; call when releasing the cursor externally.
    pub fn reset(&mut self, state: &mut FreeCameraState) {
        self.armed = false;
        self.toggled = false;
        Self::stop(state);
    }

    fn stop(state: &mut FreeCameraState) {
        state.enabled = false;
        state.velocity = Vec3::ZERO;
        state.rotation_curve = None;
    }
}

fn gate_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut camera: Single<(
        &Camera,
        &FreeCamera,
        &mut FreeCameraState,
        &mut FreeCameraInput,
    )>,
) {
    let (window, cursor) = &mut *window;
    let (camera, config, state, input) = &mut *camera;
    let held = mouse.pressed(config.mouse_key_cursor_grab);
    let toggle_held = keys.pressed(config.keyboard_key_toggle_cursor_grab);

    if !camera.is_active || !window.focused {
        if state.enabled {
            cursor.grab_mode = CursorGrabMode::None;
            cursor.visible = true;
        }
        input.reset(state);
        return;
    }

    // Always give a newly spawned/reset controller a disabled tick to clear its
    // private capture flags, and wait for held capture controls to be released.
    if !input.armed {
        input.armed = !held && !toggle_held && !keys.pressed(KeyCode::Escape);
        FreeCameraInput::stop(state);
        return;
    }

    if state.enabled && cursor.grab_mode != CursorGrabMode::Locked {
        input.reset(state);
        return;
    }

    if keys.just_pressed(config.keyboard_key_toggle_cursor_grab) {
        input.toggled = !input.toggled;
    }
    state.enabled = input.toggled || held;
    if !state.enabled {
        // Upstream applies movement before releasing its cursor. Predicting the
        // release here also prevents its subsequent snap-rotation system running.
        FreeCameraInput::stop(state);
    }
}

fn on_remove(
    event: On<Remove, FreeCamera>,
    cameras: Query<&Camera>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if cameras
        .get(event.entity)
        .is_ok_and(|camera| camera.is_active)
    {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Registers Bevy's free camera controller and the input gate before it.
pub struct FreeCameraInputPlugin;

impl Plugin for FreeCameraInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BevyFreeCameraPlugin)
            .add_systems(
                RunFixedMainLoop,
                gate_input
                    .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop)
                    .before(run_freecamera_controller),
            )
            .add_observer(on_remove);
    }
}

#[cfg(test)]
mod tests;
