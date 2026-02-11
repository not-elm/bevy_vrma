//! - [`look at specification(en)`](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/lookAt.md)
//! - [`look at specification(ja)`](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/lookAt.ja.md)

use crate::prelude::*;
use crate::system_set::VrmSystemSets;
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowPosition};

/// Controls what the VRM model looks at.
/// This component should be inserted into the root entity of the VRM.
///
/// [`LookAt::Cursor`] tracks the mouse cursor across all windows.
/// [`LookAt::Target`] looks at a specified entity.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_vrm1::prelude::*;
///
/// fn spawn_camera_and_vrm(
///     mut commands: Commands,
///     asset_server: Res<AssetServer>,
/// ) {
///     commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 1.3, 1.0)));
///     commands.spawn((
///         VrmHandle(asset_server.load("model.vrm")),
///         LookAt::Cursor,
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub enum LookAt {
    /// Look at the mouse cursor. Automatically finds the window with the cursor
    /// and the `Camera3d` rendering to it.
    Cursor,

    /// Look at a specific target entity.
    Target(Entity),
}

pub(super) struct LookAtPlugin;

impl Plugin for LookAtPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<LookAt>()
            .register_type::<LookAtProperties>()
            .register_type::<LookAtType>()
            .add_systems(
                PostUpdate,
                track_looking_target
                    .in_set(VrmSystemSets::GazeControl)
                    .after(VrmSystemSets::PropagateAfterConstraints),
            );
    }
}

fn track_looking_target(
    mut commands: Commands,
    vrms: Query<(
        &LookAt,
        &LookAtProperties,
        &HeadBoneEntity,
        &LeftEyeBoneEntity,
        &RightEyeBoneEntity,
    )>,
    transforms: Query<&Transform>,
    global_transforms: Query<&GlobalTransform>,
    rests: Query<(&RestTransform, &RestGlobalTransform)>,
    windows: Query<(Entity, &Window, Has<PrimaryWindow>)>,
) {
    vrms.iter()
        .for_each(|(look_at, properties, head, left_eye, right_eye)| {
            let Ok(head_gtf) = global_transforms.get(head.0) else {
                return;
            };
            let Ok(head_tf) = transforms.get(head.0) else {
                return;
            };

            let (yaw, pitch) = match look_at {
                LookAt::Cursor => {
                    let Some(normalized) = find_cursor_position_normalized(&windows) else {
                        return;
                    };
                    calc_yaw_pitch_from_screen(normalized, properties)
                }
                LookAt::Target(target_entity) => {
                    let Ok(target_gtf) = global_transforms.get(*target_entity) else {
                        return;
                    };
                    let look_at_space = GlobalTransform::default();
                    let mut look_at_space_tf = look_at_space.reparented_to(head_gtf);
                    look_at_space_tf.translation = Vec3::from(properties.offset_from_head_bone);
                    look_at_space_tf.rotation = head_tf.rotation.inverse();
                    let look_at_space = head_gtf.mul_transform(look_at_space_tf);
                    calc_yaw_pitch(&look_at_space, target_gtf.translation())
                }
            };

            match properties.r#type {
                LookAtType::Bone => {
                    apply_bone(
                        &mut commands,
                        &transforms,
                        &rests,
                        left_eye,
                        right_eye,
                        properties,
                        yaw,
                        pitch,
                    );
                }
                LookAtType::Expression => {
                    todo!("Expression look at is not supported yet");
                }
            }
        });
}

fn apply_bone(
    commands: &mut Commands,
    transforms: &Query<&Transform>,
    rests: &Query<(&RestTransform, &RestGlobalTransform)>,
    left_eye: &LeftEyeBoneEntity,
    right_eye: &RightEyeBoneEntity,
    properties: &LookAtProperties,
    yaw: f32,
    pitch: f32,
) {
    let Ok(left_eye_tf) = transforms.get(left_eye.0) else {
        return;
    };
    let Ok(right_eye_tf) = transforms.get(right_eye.0) else {
        return;
    };
    let Ok((left_eye_rest_tf, left_eye_gtf)) = rests.get(left_eye.0) else {
        return;
    };
    let Ok((right_eye_rest_tf, right_eye_gtf)) = rests.get(right_eye.0) else {
        return;
    };
    let applied_left_eye_tf = apply_left_eye_bone(
        left_eye_tf,
        left_eye_rest_tf,
        left_eye_gtf,
        properties,
        yaw,
        pitch,
    );
    let applied_right_eye_tf = apply_right_eye_bone(
        right_eye_tf,
        right_eye_rest_tf,
        right_eye_gtf,
        properties,
        yaw,
        pitch,
    );
    commands.entity(left_eye.0).insert(applied_left_eye_tf);
    commands.entity(right_eye.0).insert(applied_right_eye_tf);
}

fn find_cursor_position_normalized(
    windows: &Query<(Entity, &Window, Has<PrimaryWindow>)>
) -> Option<Vec2> {
    // Try multi-window first
    if let Some(normalized) = find_cursor_position_normalized_multi_window(windows) {
        return Some(normalized);
    }

    // Fallback to single-window behavior
    windows.iter().find_map(|(_, window, _)| {
        let cursor = window.cursor_position()?;
        let size = window.size();
        Some(Vec2::new(
            (cursor.x / size.x - 0.5) * 2.0,
            (cursor.y / size.y - 0.5) * 2.0,
        ))
    })
}

fn find_cursor_position_normalized_multi_window(
    windows: &Query<(Entity, &Window, Has<PrimaryWindow>)>
) -> Option<Vec2> {
    // Collect windows with explicit positions
    let mut window_bounds: Vec<(Vec2, Vec2)> = Vec::new(); // (position, size)
    let mut cursor_global: Option<Vec2> = None;

    for (_, window, _) in windows.iter() {
        let WindowPosition::At(pos) = window.position else {
            continue; // Skip windows without explicit position
        };
        let pos = Vec2::new(pos.x as f32, pos.y as f32);
        let size = window.size();
        window_bounds.push((pos, size));

        // Last window with cursor wins; typically only one window reports cursor per frame
        if let Some(cursor_local) = window.cursor_position() {
            cursor_global = Some(pos + cursor_local);
        }
    }

    // Fallback: not enough positioned windows
    if window_bounds.len() < 2 {
        return None;
    }

    let cursor_global = cursor_global?;

    // Calculate bounding box
    let min = window_bounds
        .iter()
        .map(|(pos, _)| *pos)
        .reduce(|a, b| a.min(b))?;
    let max = window_bounds
        .iter()
        .map(|(pos, size)| *pos + *size)
        .reduce(|a, b| a.max(b))?;

    let center = (min + max) / 2.0;
    let half_size = (max - min) / 2.0;

    if half_size.x == 0.0 || half_size.y == 0.0 {
        return None;
    }

    // Normalize relative to center
    Some((cursor_global - center) / half_size)
}

fn calc_yaw_pitch_from_screen(
    normalized: Vec2,
    properties: &LookAtProperties,
) -> (f32, f32) {
    let horizontal_max = properties
        .range_map_horizontal_outer
        .input_max_value
        .max(properties.range_map_horizontal_inner.input_max_value);
    let vertical_max = properties
        .range_map_vertical_down
        .input_max_value
        .max(properties.range_map_vertical_up.input_max_value);

    (normalized.x * horizontal_max, normalized.y * vertical_max)
}

fn calc_yaw_pitch(
    look_at_space: &GlobalTransform,
    target: Vec3,
) -> (f32, f32) {
    let local_target = look_at_space.to_matrix().inverse().transform_point3(target);

    let z = local_target.dot(Vec3::Z);
    let x = local_target.dot(Vec3::X);
    let yaw = (x.atan2(z)).to_degrees();

    let xz = (x * x + z * z).sqrt();
    let y = local_target.dot(Vec3::Y);
    let pitch = (-y.atan2(xz)).to_degrees();

    (yaw, pitch)
}

fn apply_left_eye_bone(
    left_eye: &Transform,
    rest_tf: &RestTransform,
    rest_gtf: &RestGlobalTransform,
    properties: &LookAtProperties,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> Transform {
    let range_map_horizontal_outer = properties.range_map_horizontal_outer;
    let range_map_horizontal_inner = properties.range_map_horizontal_inner;
    let range_map_vertical_down = properties.range_map_vertical_down;
    let range_map_vertical_up = properties.range_map_vertical_up;
    let yaw = if yaw_degrees > 0.0 {
        yaw_degrees.min(range_map_horizontal_outer.input_max_value)
            / range_map_horizontal_outer.input_max_value
            * range_map_horizontal_outer.output_scale
    } else {
        -(yaw_degrees
            .abs()
            .min(range_map_horizontal_inner.input_max_value)
            / range_map_horizontal_inner.input_max_value
            * range_map_horizontal_inner.output_scale)
    };

    let pitch = if pitch_degrees > 0.0 {
        pitch_degrees.min(range_map_vertical_down.input_max_value)
            / range_map_vertical_down.input_max_value
            * range_map_vertical_down.output_scale
    } else {
        -(pitch_degrees
            .abs()
            .min(range_map_vertical_up.input_max_value)
            / range_map_vertical_up.input_max_value
            * range_map_vertical_up.output_scale)
    };
    left_eye.with_rotation(to_eye_rotation(yaw, pitch, rest_tf, rest_gtf))
}

fn apply_right_eye_bone(
    right_eye: &Transform,
    rest_tf: &RestTransform,
    rest_gtf: &RestGlobalTransform,
    properties: &LookAtProperties,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> Transform {
    let range_map_horizontal_outer = properties.range_map_horizontal_outer;
    let range_map_horizontal_inner = properties.range_map_horizontal_inner;
    let range_map_vertical_down = properties.range_map_vertical_down;
    let range_map_vertical_up = properties.range_map_vertical_up;

    let yaw = if yaw_degrees > 0.0 {
        yaw_degrees.min(range_map_horizontal_inner.input_max_value)
            / range_map_horizontal_inner.input_max_value
            * range_map_horizontal_inner.output_scale
    } else {
        -(yaw_degrees
            .abs()
            .min(range_map_horizontal_outer.input_max_value)
            / range_map_horizontal_outer.input_max_value
            * range_map_horizontal_outer.output_scale)
    };

    let pitch = if pitch_degrees > 0.0 {
        pitch_degrees.min(range_map_vertical_down.input_max_value)
            / range_map_vertical_down.input_max_value
            * range_map_vertical_down.output_scale
    } else {
        -(pitch_degrees
            .abs()
            .min(range_map_vertical_up.input_max_value)
            / range_map_vertical_up.input_max_value
            * range_map_vertical_up.output_scale)
    };

    right_eye.with_rotation(to_eye_rotation(yaw, pitch, rest_tf, rest_gtf))
}

#[inline]
fn to_eye_rotation(
    yaw: f32,
    pitch: f32,
    rest_tf: &RestTransform,
    rest_gtf: &RestGlobalTransform,
) -> Quat {
    (rest_tf.rotation * rest_gtf.rotation().inverse())
        * Quat::from_euler(EulerRot::YXZ, yaw.to_radians(), pitch.to_radians(), 0.0)
        * rest_gtf.rotation()
}
