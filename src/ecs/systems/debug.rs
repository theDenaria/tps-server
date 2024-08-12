use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use iyes_perf_ui::prelude::PerfUiCompleteBundle;

use crate::constants::DEBUG_CAMERA_SENSITIVITY;

#[derive(Component)]
pub struct MyCameraMarker;

#[derive(Component)]
pub struct IsWindowLocked(bool);

pub fn set_debug_metrics(mut commands: Commands) {
    commands.spawn(PerfUiCompleteBundle::default());
}

pub fn set_debug_metrics_cam(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

pub fn set_debug_3d_render_camera(
    mut commands: Commands,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut primary_window = q_windows.single_mut();
    // let window = windows.primary_mut();
    primary_window.cursor.grab_mode = CursorGrabMode::Locked;

    // also hide the cursor
    primary_window.cursor.visible = false;
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-3.0, 100.0, 10.0)
                .looking_at(Vec3::ZERO, Vec3::Y)
                .with_scale(Vec3::new(1.0, -1.0, 1.0)),

            ..Default::default()
        },
        MyCameraMarker,
        IsWindowLocked(false),
    ));
}

pub fn move_debug_camera(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut IsWindowLocked), With<MyCameraMarker>>,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok((mut transform, mut is_locked)) = query.get_single_mut() {
        if !is_locked.0 {
            let direction = transform.rotation.clone();
            if keys.pressed(KeyCode::Space) {
                transform.translation += Vec3::new(0.0, 1.0, 0.0);
            }

            if keys.pressed(KeyCode::KeyW) {
                transform.translation += direction * Vec3::new(0.0, 0.0, -1.0);
            }
            // we can check multiple at once with `.any_*`
            if keys.pressed(KeyCode::KeyA) {
                transform.translation += direction * Vec3::new(-1.0, 0.0, 0.0);
            }
            if keys.pressed(KeyCode::KeyS) {
                transform.translation += direction * Vec3::new(0.0, 0.0, 1.0);
            }
            if keys.pressed(KeyCode::KeyD) {
                transform.translation += direction * Vec3::new(1.0, 0.0, 0.0);
            }
        }

        if keys.just_pressed(KeyCode::Escape) {
            let mut primary_window = q_windows.single_mut();
            is_locked.0 = !is_locked.0;
            if is_locked.0 {
                primary_window.cursor.grab_mode = CursorGrabMode::None;
                primary_window.cursor.visible = true;
            } else {
                primary_window.cursor.grab_mode = CursorGrabMode::Locked;
                primary_window.cursor.visible = false;
            }
        }
    }
}

pub fn look_debug_camera(
    mut evr_motion: EventReader<MouseMotion>,
    mut query: Query<(&mut Transform, &mut IsWindowLocked), With<MyCameraMarker>>,
) {
    if let Ok((mut transform, is_locked)) = query.get_single_mut() {
        if !is_locked.0 {
            for ev in evr_motion.read() {
                // Rotate around the Y axis for horizontal movement (yaw)
                // Rotate around the Y axis for horizontal movement (yaw)
                transform.rotate_y(ev.delta.x * DEBUG_CAMERA_SENSITIVITY);

                // Invert the pitch rotation since the camera is mirrored
                transform.rotate_local_x(ev.delta.y * DEBUG_CAMERA_SENSITIVITY);
            }
        }
    }
}
