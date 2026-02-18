use crate::prelude::*;
use crate::system_set::VrmSystemSets;
use crate::vrm::look_at::{calc_yaw_pitch, find_cursor_world_position, track_looking_target};
use crate::vrm::{RestGlobalTransform, RestTransform};
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use bevy::window::Window;

/// Optional body tracking that makes head, neck, chest, and spine bones
/// follow the `LookAt` target. Insert alongside [`LookAt`] to enable.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_vrm1::prelude::*;
///
/// fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
///     commands.spawn((
///         VrmHandle(asset_server.load("model.vrm")),
///         LookAt::Cursor,
///         BodyTracking::default(),
///     ));
/// }
/// ```
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct BodyTracking {
    /// Fraction of total gaze angle applied to head bone (0.0-1.0).
    pub head_weight: f32,
    /// Fraction of total gaze angle applied to neck bone (0.0-1.0).
    pub neck_weight: f32,
    /// Fraction of total gaze angle applied to chest bone (0.0-1.0).
    pub chest_weight: f32,
    /// Fraction of total gaze angle applied to spine bone (0.0-1.0).
    pub spine_weight: f32,

    /// Maximum head yaw in degrees.
    pub head_yaw_max: f32,
    /// Maximum head pitch in degrees.
    pub head_pitch_max: f32,
    /// Maximum neck yaw in degrees.
    pub neck_yaw_max: f32,
    /// Maximum neck pitch in degrees.
    pub neck_pitch_max: f32,
    /// Maximum chest yaw in degrees.
    pub chest_yaw_max: f32,
    /// Maximum chest pitch in degrees. Set to 0.0 for yaw-only.
    pub chest_pitch_max: f32,
    /// Maximum spine yaw in degrees.
    pub spine_yaw_max: f32,
    /// Maximum spine pitch in degrees. Set to 0.0 for yaw-only.
    pub spine_pitch_max: f32,

    /// Smoothing speed. Higher values = faster response. 0.0 = instant (no smoothing).
    pub smoothing: f32,
}

impl Default for BodyTracking {
    fn default() -> Self {
        Self {
            head_weight: 0.4,
            neck_weight: 0.25,
            chest_weight: 0.2,
            spine_weight: 0.15,
            head_yaw_max: 40.0,
            head_pitch_max: 30.0,
            neck_yaw_max: 25.0,
            neck_pitch_max: 20.0,
            chest_yaw_max: 20.0,
            chest_pitch_max: 0.0,
            spine_yaw_max: 15.0,
            spine_pitch_max: 0.0,
            smoothing: 10.0,
        }
    }
}

/// Smoothed gaze state stored on the VRM root entity.
/// Inserted automatically when `BodyTracking` is present.
#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct SmoothedGaze {
    pub yaw: f32,
    pub pitch: f32,
}

pub(super) struct BodyTrackingPlugin;

impl Plugin for BodyTrackingPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<BodyTracking>()
            .register_type::<SmoothedGaze>()
            .add_observer(auto_insert_smoothed_gaze)
            .add_systems(
                PostUpdate,
                track_body_tracking
                    .in_set(VrmSystemSets::GazeControl)
                    .after(VrmSystemSets::PropagateAfterConstraints)
                    .before(track_looking_target)
                    .run_if(any_with_component::<BodyTracking>),
            );
    }
}

/// Automatically insert [`SmoothedGaze`] when [`BodyTracking`] is added.
fn auto_insert_smoothed_gaze(
    trigger: On<Insert, BodyTracking>,
    mut commands: Commands,
) {
    commands
        .entity(trigger.event_target())
        .insert(SmoothedGaze::default());
}

/// Exponential-decay smoothing with shortest-arc delta for angles.
fn smooth_angle(
    current: f32,
    target: f32,
    speed: f32,
    dt: f32,
) -> f32 {
    if speed <= 0.0 {
        return target;
    }
    let mut delta = target - current;
    while delta > 180.0 {
        delta -= 360.0;
    }
    while delta < -180.0 {
        delta += 360.0;
    }
    current + delta * (1.0 - (-speed * dt).exp())
}

/// Compute bone rotation from yaw/pitch using the same formula as eye rotation.
fn bone_rotation(
    yaw_degrees: f32,
    pitch_degrees: f32,
    rest_tf: &RestTransform,
    rest_gtf: &RestGlobalTransform,
) -> Quat {
    (rest_tf.rotation * rest_gtf.rotation().inverse())
        * Quat::from_euler(
            EulerRot::YXZ,
            yaw_degrees.to_radians(),
            pitch_degrees.to_radians(),
            0.0,
        )
        * rest_gtf.rotation()
}

/// Per-bone state for tracking animation changes between frames.
#[derive(Debug, Clone)]
struct BoneState {
    /// The bone rotation before body tracking was applied (from animation or rest).
    base: Quat,
    /// The gaze delta applied last frame.
    last_delta: Quat,
    /// Whether state has been initialized.
    initialized: bool,
}

impl Default for BoneState {
    fn default() -> Self {
        Self {
            base: Quat::IDENTITY,
            last_delta: Quat::IDENTITY,
            initialized: false,
        }
    }
}

/// Compute additive rotation: apply the gaze delta (relative to rest) on top of
/// the base (animated) rotation.
///
/// `base` — current bone rotation from animation (or rest if no animation).
/// `rest` — the bone's rest pose local rotation.
/// `gaze` — the target rotation computed by `bone_rotation()`.
///
/// Returns `base * (rest⁻¹ * gaze)`.
/// When `base == rest`, this simplifies to `gaze` (identical to overwrite mode).
fn compute_additive_rotation(base: Quat, rest: Quat, gaze: Quat) -> Quat {
    let delta = rest.inverse() * gaze;
    base * delta
}

/// Bone descriptor used when iterating through the chain.
struct BoneEntry {
    entity: Entity,
    weight: f32,
    yaw_max: f32,
    pitch_max: f32,
}

fn track_body_tracking(
    mut vrms: Query<(
        &LookAt,
        &LookAtProperties,
        &BodyTracking,
        &HeadBoneEntity,
        Option<&NeckBoneEntity>,
        Option<&ChestBoneEntity>,
        Option<&SpineBoneEntity>,
        &mut SmoothedGaze,
    )>,
    mut transforms: Query<(&mut Transform, &mut GlobalTransform), Without<Camera>>,
    child_ofs: Query<&ChildOf>,
    rests: Query<(&RestTransform, &RestGlobalTransform)>,
    windows: Query<(Entity, &Window)>,
    cameras: Cameras,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (look_at, properties, tracking, head, neck, chest, spine, mut smoothed) in vrms.iter_mut() {
        // 1. Get head GlobalTransform and build LookAt space.
        let Ok((&head_tf, &head_gtf)) = transforms.get(head.0) else {
            continue;
        };

        let look_at_space = GlobalTransform::default();
        let mut look_at_space_tf = look_at_space.reparented_to(&head_gtf);
        look_at_space_tf.translation = Vec3::from(properties.offset_from_head_bone);
        look_at_space_tf.rotation = head_tf.rotation.inverse();
        let look_at_space = head_gtf.mul_transform(look_at_space_tf);

        // 2. Calculate raw yaw/pitch.
        let (raw_yaw, raw_pitch) = match look_at {
            LookAt::Cursor => {
                let Some(target_pos) = find_cursor_world_position(&windows, &cameras, &head_gtf)
                else {
                    continue;
                };
                calc_yaw_pitch(&look_at_space, target_pos)
            }
            LookAt::Target(target_entity) => {
                let Ok((_, &target_gtf)) = transforms.get(*target_entity) else {
                    continue;
                };
                calc_yaw_pitch(&look_at_space, target_gtf.translation())
            }
        };

        // 3. Smooth yaw/pitch.
        smoothed.yaw = smooth_angle(smoothed.yaw, raw_yaw, tracking.smoothing, dt);
        smoothed.pitch = smooth_angle(smoothed.pitch, raw_pitch, tracking.smoothing, dt);

        // 4. Build bone chain bottom-up: spine -> chest -> neck -> head.
        let mut chain: Vec<BoneEntry> = Vec::with_capacity(4);
        if let Some(spine) = spine {
            chain.push(BoneEntry {
                entity: spine.0,
                weight: tracking.spine_weight,
                yaw_max: tracking.spine_yaw_max,
                pitch_max: tracking.spine_pitch_max,
            });
        }
        if let Some(chest) = chest {
            chain.push(BoneEntry {
                entity: chest.0,
                weight: tracking.chest_weight,
                yaw_max: tracking.chest_yaw_max,
                pitch_max: tracking.chest_pitch_max,
            });
        }
        if let Some(neck) = neck {
            chain.push(BoneEntry {
                entity: neck.0,
                weight: tracking.neck_weight,
                yaw_max: tracking.neck_yaw_max,
                pitch_max: tracking.neck_pitch_max,
            });
        }
        chain.push(BoneEntry {
            entity: head.0,
            weight: tracking.head_weight,
            yaw_max: tracking.head_yaw_max,
            pitch_max: tracking.head_pitch_max,
        });

        // 5. Apply rotations bottom-up, propagating GlobalTransform along the chain.
        //    We track which entities we've already computed so we can use local
        //    cached values for parent lookups when the parent is also in the chain.
        let mut computed_gtfs: Vec<(Entity, GlobalTransform)> = Vec::with_capacity(4);

        for bone in &chain {
            let Ok((rest_tf, rest_gtf)) = rests.get(bone.entity) else {
                continue;
            };

            let bone_yaw = (smoothed.yaw * bone.weight).clamp(-bone.yaw_max, bone.yaw_max);
            let bone_pitch = (smoothed.pitch * bone.weight).clamp(-bone.pitch_max, bone.pitch_max);
            let rotation = bone_rotation(bone_yaw, bone_pitch, rest_tf, rest_gtf);

            // Get parent entity for chain propagation.
            let Ok(child_of) = child_ofs.get(bone.entity) else {
                continue;
            };
            let parent_entity = child_of.parent();

            // Look up parent GlobalTransform: first check our local cache (for chain
            // parents like spine->chest), then fall back to the read-only query.
            let parent_gtf = computed_gtfs
                .iter()
                .rev()
                .find(|(e, _)| *e == parent_entity)
                .map(|(_, gtf)| *gtf)
                .or_else(|| transforms.get(parent_entity).map(|(_, gtf)| *gtf).ok());

            let Some(parent_gtf) = parent_gtf else {
                continue;
            };

            // Write Transform.rotation and manually propagate GlobalTransform.
            let Ok((mut tf, mut gtf)) = transforms.get_mut(bone.entity) else {
                continue;
            };
            tf.rotation = rotation;
            *gtf = parent_gtf.mul_transform(*tf);

            computed_gtfs.push((bone.entity, *gtf));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smooth_angle_converges() {
        let mut current = 0.0;
        for _ in 0..100 {
            current = smooth_angle(current, 45.0, 10.0, 1.0 / 60.0);
        }
        assert!(
            (current - 45.0).abs() < 0.1,
            "Should converge to target: {current}"
        );
    }

    #[test]
    fn test_smooth_angle_instant_when_speed_zero() {
        let result = smooth_angle(0.0, 45.0, 0.0, 1.0 / 60.0);
        assert!((result - 45.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_smooth_angle_shortest_arc() {
        let result = smooth_angle(170.0, -170.0, 100.0, 1.0);
        // Should go through 180 (20 degrees), not through 0 (340 degrees)
        assert!(
            result > 170.0 || result < -160.0,
            "Should take shortest arc: {result}"
        );
    }

    #[test]
    fn test_smooth_angle_no_change_at_target() {
        let result = smooth_angle(45.0, 45.0, 10.0, 1.0 / 60.0);
        assert!((result - 45.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_body_tracking_default_weights_sum_to_one() {
        let bt = BodyTracking::default();
        let total = bt.head_weight + bt.neck_weight + bt.chest_weight + bt.spine_weight;
        assert!(
            (total - 1.0).abs() < f32::EPSILON,
            "Default weights should sum to 1.0: {total}"
        );
    }

    #[test]
    fn test_body_tracking_default_spine_pitch_is_yaw_only() {
        let bt = BodyTracking::default();
        assert_eq!(bt.spine_pitch_max, 0.0);
        assert_eq!(bt.chest_pitch_max, 0.0);
    }

    #[test]
    fn test_bone_rotation_identity_at_zero() {
        let rest_tf = RestTransform(Transform::IDENTITY);
        let rest_gtf = RestGlobalTransform(GlobalTransform::IDENTITY);
        let result = bone_rotation(0.0, 0.0, &rest_tf, &rest_gtf);
        let diff = result.angle_between(Quat::IDENTITY);
        assert!(
            diff < 0.001,
            "Zero yaw/pitch should produce near-identity: {diff}"
        );
    }

    #[test]
    fn test_compute_additive_rotation_no_animation() {
        // When no animation is playing (base == rest), result should equal
        // the full gaze rotation (same as current overwrite behavior).
        let rest = Quat::from_rotation_y(0.3);
        let gaze = bone_rotation(
            15.0,
            10.0,
            &RestTransform(Transform::from_rotation(rest)),
            &RestGlobalTransform(GlobalTransform::from(Transform::from_rotation(rest))),
        );
        let result = compute_additive_rotation(rest, rest, gaze);
        let diff = result.angle_between(gaze);
        assert!(
            diff < 0.001,
            "With no animation (base==rest), should equal gaze rotation: diff={diff}"
        );
    }

    #[test]
    fn test_compute_additive_rotation_with_animation() {
        // When animation sets a different rotation, the gaze delta should be
        // applied on top of the animated rotation, NOT replace it.
        let rest = Quat::IDENTITY;
        let animated = Quat::from_rotation_x(0.2); // Animation tilts spine forward
        let gaze = bone_rotation(
            10.0,
            0.0,
            &RestTransform(Transform::IDENTITY),
            &RestGlobalTransform(GlobalTransform::IDENTITY),
        );
        let result = compute_additive_rotation(animated, rest, gaze);
        // Result should NOT equal gaze (which ignores animation)
        let diff_from_gaze = result.angle_between(gaze);
        assert!(
            diff_from_gaze > 0.01,
            "With animation, result should differ from pure gaze: diff={diff_from_gaze}"
        );
        // Result should incorporate the animation's forward tilt
        let diff_from_animated = result.angle_between(animated);
        assert!(
            diff_from_animated > 0.01,
            "Result should also differ from pure animation: diff={diff_from_animated}"
        );
    }

    #[test]
    fn test_compute_additive_rotation_zero_gaze_preserves_animation() {
        // When gaze is zero (looking straight ahead), the animation rotation
        // should be fully preserved.
        let rest = Quat::from_rotation_y(0.5);
        let animated = Quat::from_rotation_x(0.3) * Quat::from_rotation_y(0.5);
        let gaze_at_zero = bone_rotation(
            0.0,
            0.0,
            &RestTransform(Transform::from_rotation(rest)),
            &RestGlobalTransform(GlobalTransform::from(Transform::from_rotation(rest))),
        );
        // gaze_at_zero == rest, so delta == identity, result == animated
        let result = compute_additive_rotation(animated, rest, gaze_at_zero);
        let diff = result.angle_between(animated);
        assert!(
            diff < 0.001,
            "Zero gaze should preserve animated rotation: diff={diff}"
        );
    }
}
