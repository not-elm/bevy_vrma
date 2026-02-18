# Body Tracking Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add optional body tracking (head, neck, chest, spine rotation toward gaze target) to the LookAt system.

**Architecture:** New `BodyTracking` component on VRM root entity, processed by `track_body_tracking` system in `VrmSystemSets::GazeControl` before existing eye LookAt. Uses fractional weight distribution with per-bone clamping, frame-rate-independent exponential smoothing at yaw/pitch level, and manual chain propagation (SpringBone pattern).

**Tech Stack:** Bevy 0.18, Rust 2024 edition, bevy_test_helper for testing.

**Design Doc:** `docs/plans/2026-02-18-body-tracking-design.md`

---

### Task 1: Add BodyTracking and SmoothedGaze Components

**Files:**
- Create: `src/vrm/body_tracking.rs`
- Modify: `src/vrm.rs` (add module declaration + prelude export)

**Step 1: Create component file with BodyTracking and SmoothedGaze**

Create `src/vrm/body_tracking.rs`:

```rust
use crate::prelude::*;
use crate::system_set::VrmSystemSets;
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use bevy::window::Window;

/// Optional body tracking that makes head, neck, chest, and spine bones
/// follow the LookAt target. Insert alongside [`LookAt`] to enable.
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
pub struct SmoothedGaze {
    pub yaw: f32,
    pub pitch: f32,
}
```

**Step 2: Add module declaration and prelude export in `src/vrm.rs`**

In `src/vrm.rs`, add after the existing `pub mod look_at;` line:

```rust
pub mod body_tracking;
```

In the `pub mod prelude` block, add:

```rust
body_tracking::BodyTracking,
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles without errors.

**Step 4: Commit**

```bash
git add src/vrm/body_tracking.rs src/vrm.rs
git commit -m "feat(body_tracking): add BodyTracking and SmoothedGaze components"
```

---

### Task 2: Add BodyTrackingPlugin with System Registration

**Files:**
- Modify: `src/vrm/body_tracking.rs` (add plugin + system stub)
- Modify: `src/vrm.rs` (register plugin)

**Step 1: Add plugin struct and system stub to `body_tracking.rs`**

Add after the `SmoothedGaze` struct:

```rust
pub(super) struct BodyTrackingPlugin;

impl Plugin for BodyTrackingPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BodyTracking>()
            .register_type::<SmoothedGaze>()
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
    mut transforms: Query<(&mut Transform, &mut GlobalTransform, &ChildOf)>,
    global_transforms: Query<&GlobalTransform>,
    rests: Query<(&RestTransform, &RestGlobalTransform)>,
    windows: Query<(Entity, &Window)>,
    cameras: Cameras,
    time: Res<Time>,
) {
    // Implementation in next task
}
```

Note: `track_looking_target` needs to be made `pub(crate)` in `src/vrm/look_at.rs` so it can be referenced in `.before()`.

**Step 2: Make `track_looking_target` pub(crate) in `src/vrm/look_at.rs`**

Change line 63 from:
```rust
fn track_looking_target(
```
to:
```rust
pub(crate) fn track_looking_target(
```

**Step 3: Register BodyTrackingPlugin in `src/vrm.rs`**

In `VrmPlugin::build()`, add `BodyTrackingPlugin` to the plugin tuple (after `LookAtPlugin`):

```rust
app.init_asset::<VrmAsset>().add_plugins((
    VrmLoaderPlugin,
    VrmInitializePlugin,
    VrmSpringBonePlugin,
    VrmHumanoidBonePlugin,
    VrmExpressionPlugin,
    VrmNodeConstraintPlugin,
    MtoonMaterialPlugin,
    LookAtPlugin,
    BodyTrackingPlugin,  // <-- add this
));
```

Import at top of `src/vrm.rs`:
```rust
use body_tracking::BodyTrackingPlugin;
```

**Step 4: Add auto-insertion of SmoothedGaze via observer**

In `BodyTrackingPlugin::build()`, add an observer that inserts `SmoothedGaze` when `BodyTracking` is added:

```rust
app.add_observer(|trigger: Trigger<OnAdd, BodyTracking>, mut commands: Commands| {
    commands.entity(trigger.target()).insert(SmoothedGaze::default());
});
```

**Step 5: Verify compilation**

Run: `cargo check`
Expected: Compiles without errors (system body is empty).

**Step 6: Commit**

```bash
git add src/vrm/body_tracking.rs src/vrm/look_at.rs src/vrm.rs
git commit -m "feat(body_tracking): add BodyTrackingPlugin with system registration"
```

---

### Task 3: Implement Core Body Tracking Algorithm

**Files:**
- Modify: `src/vrm/body_tracking.rs` (implement `track_body_tracking`)

**Step 1: Add helper functions**

Add before `track_body_tracking`:

```rust
/// Apply exponential smoothing with shortest-arc delta for yaw.
fn smooth_angle(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
    if speed <= 0.0 {
        return target;
    }
    let mut delta = target - current;
    // Shortest-arc: wrap delta to [-180, 180]
    while delta > 180.0 {
        delta -= 360.0;
    }
    while delta < -180.0 {
        delta += 360.0;
    }
    current + delta * (1.0 - (-speed * dt).exp())
}

/// Compute bone rotation from world-space yaw/pitch using rest transforms.
/// Same formula as eye rotation in look_at.rs.
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

/// Propagate GlobalTransform from a bone up through its ancestors
/// down to the bone itself. Walks the ChildOf chain.
fn propagate_chain_to(
    entity: Entity,
    transforms: &mut Query<(&mut Transform, &mut GlobalTransform, &ChildOf)>,
    global_transforms: &Query<&GlobalTransform>,
) {
    let Ok((tf, mut gtf, child_of)) = transforms.get_mut(entity) else {
        return;
    };
    let parent = child_of.parent();
    // Try to get parent from mutable query first, fall back to read-only
    let parent_gtf = if let Ok((_, parent_gtf, _)) = transforms.get(parent) {
        *parent_gtf
    } else if let Ok(parent_gtf) = global_transforms.get(parent) {
        *parent_gtf
    } else {
        return;
    };
    *gtf = parent_gtf.mul_transform(*tf);
}
```

**Step 2: Implement track_body_tracking**

Replace the empty `track_body_tracking` body:

```rust
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
    mut transforms: Query<(&mut Transform, &mut GlobalTransform, &ChildOf)>,
    global_transforms: Query<&GlobalTransform>,
    rests: Query<(&RestTransform, &RestGlobalTransform)>,
    windows: Query<(Entity, &Window)>,
    cameras: Cameras,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (look_at, properties, tracking, head, neck, chest, spine, mut smoothed) in
        vrms.iter_mut()
    {
        // 1. Get head global transform for LookAt space calculation
        let Ok(head_gtf) = global_transforms.get(head.0) else {
            continue;
        };
        let Ok(head_tf) = transforms.get(head.0).map(|(tf, _, _)| *tf) else {
            continue;
        };

        // 2. Build LookAt space (same as look_at.rs)
        let look_at_space = GlobalTransform::default();
        let mut look_at_space_tf = look_at_space.reparented_to(head_gtf);
        look_at_space_tf.translation = Vec3::from(properties.offset_from_head_bone);
        look_at_space_tf.rotation = head_tf.rotation.inverse();
        let look_at_space = head_gtf.mul_transform(look_at_space_tf);

        // 3. Calculate raw yaw/pitch to target
        let (raw_yaw, raw_pitch) = match look_at {
            LookAt::Cursor => {
                let Some(target_pos) =
                    find_cursor_world_position(&windows, &cameras, head_gtf)
                else {
                    continue;
                };
                calc_yaw_pitch(&look_at_space, target_pos)
            }
            LookAt::Target(target_entity) => {
                let Ok(target_gtf) = global_transforms.get(*target_entity) else {
                    continue;
                };
                calc_yaw_pitch(&look_at_space, target_gtf.translation())
            }
        };

        // 4. Apply smoothing at yaw/pitch level
        smoothed.yaw = smooth_angle(smoothed.yaw, raw_yaw, tracking.smoothing, dt);
        smoothed.pitch = smooth_angle(smoothed.pitch, raw_pitch, tracking.smoothing, dt);

        // 5. Apply rotation to each bone bottom-up, with manual chain propagation
        let bones: [(Option<Entity>, f32, f32, f32, f32); 4] = [
            (
                spine.map(|s| s.0),
                tracking.spine_weight,
                tracking.spine_yaw_max,
                tracking.spine_weight, // reuse for pitch weight
                tracking.spine_pitch_max,
            ),
            (
                chest.map(|c| c.0),
                tracking.chest_weight,
                tracking.chest_yaw_max,
                tracking.chest_weight,
                tracking.chest_pitch_max,
            ),
            (
                neck.map(|n| n.0),
                tracking.neck_weight,
                tracking.neck_yaw_max,
                tracking.neck_weight,
                tracking.neck_pitch_max,
            ),
            (
                Some(head.0),
                tracking.head_weight,
                tracking.head_yaw_max,
                tracking.head_weight,
                tracking.head_pitch_max,
            ),
        ];

        for (bone_entity, weight, yaw_max, _weight_pitch, pitch_max) in bones {
            let Some(entity) = bone_entity else {
                continue;
            };
            let Ok((rest_tf, rest_gtf)) = rests.get(entity) else {
                continue;
            };

            let bone_yaw = (smoothed.yaw * weight).clamp(-yaw_max, yaw_max);
            let bone_pitch = (smoothed.pitch * weight).clamp(-pitch_max, pitch_max);

            let rotation = bone_rotation(bone_yaw, bone_pitch, rest_tf, rest_gtf);

            let Ok((mut tf, mut gtf, child_of)) = transforms.get_mut(entity) else {
                continue;
            };
            tf.rotation = rotation;

            // Manual chain propagation (SpringBone pattern)
            let parent = child_of.parent();
            let parent_gtf = if let Ok(p) = global_transforms.get(parent) {
                *p
            } else if let Ok((_, p, _)) = transforms.get(parent) {
                *p
            } else {
                continue;
            };
            *gtf = parent_gtf.mul_transform(*tf);
        }
    }
}
```

Note: `find_cursor_world_position` and `calc_yaw_pitch` need to be made `pub(crate)` in `look_at.rs`.

**Step 3: Make helper functions pub(crate) in look_at.rs**

In `src/vrm/look_at.rs`, change visibility of:
```rust
pub(crate) fn find_cursor_world_position(...)
pub(crate) fn calc_yaw_pitch(...)
```

**Step 4: Add necessary use statements to body_tracking.rs**

```rust
use crate::vrm::look_at::{calc_yaw_pitch, find_cursor_world_position, track_looking_target};
```

**Step 5: Verify compilation**

Run: `cargo check`
Expected: Compiles without errors.

**Step 6: Commit**

```bash
git add src/vrm/body_tracking.rs src/vrm/look_at.rs
git commit -m "feat(body_tracking): implement core body tracking algorithm"
```

---

### Task 4: Write Tests

**Files:**
- Modify: `src/vrm/body_tracking.rs` (add test module)

**Step 1: Add unit tests for smooth_angle**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smooth_angle_converges() {
        let mut current = 0.0;
        for _ in 0..100 {
            current = smooth_angle(current, 45.0, 10.0, 1.0 / 60.0);
        }
        assert!((current - 45.0).abs() < 0.1, "Should converge to target: {current}");
    }

    #[test]
    fn test_smooth_angle_instant_when_speed_zero() {
        let result = smooth_angle(0.0, 45.0, 0.0, 1.0 / 60.0);
        assert!((result - 45.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_smooth_angle_shortest_arc() {
        // From 170 to -170 should go through 180, not through 0
        let result = smooth_angle(170.0, -170.0, 100.0, 1.0);
        // With high speed and dt=1.0, should be close to -170
        // The delta should be 20 degrees (through 180), not 340 degrees (through 0)
        assert!(result > 170.0 || result < -160.0, "Should take shortest arc: {result}");
    }

    #[test]
    fn test_smooth_angle_no_change_at_target() {
        let result = smooth_angle(45.0, 45.0, 10.0, 1.0 / 60.0);
        assert!((result - 45.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_body_tracking_default() {
        let bt = BodyTracking::default();
        let total_weight = bt.head_weight + bt.neck_weight + bt.chest_weight + bt.spine_weight;
        assert!(
            (total_weight - 1.0).abs() < f32::EPSILON,
            "Default weights should sum to 1.0: {total_weight}"
        );
        assert!(bt.smoothing > 0.0);
        assert_eq!(bt.spine_pitch_max, 0.0, "Spine pitch should default to yaw-only");
        assert_eq!(bt.chest_pitch_max, 0.0, "Chest pitch should default to yaw-only");
    }

    #[test]
    fn test_bone_rotation_identity_at_zero() {
        let rest_tf = RestTransform(Transform::IDENTITY);
        let rest_gtf = RestGlobalTransform(GlobalTransform::IDENTITY);
        let result = bone_rotation(0.0, 0.0, &rest_tf, &rest_gtf);
        let diff = result.angle_between(Quat::IDENTITY);
        assert!(diff < 0.001, "Zero yaw/pitch should produce identity rotation: {diff}");
    }
}
```

**Step 2: Run tests**

Run: `cargo test --lib body_tracking`
Expected: All tests pass.

**Step 3: Commit**

```bash
git add src/vrm/body_tracking.rs
git commit -m "test(body_tracking): add unit tests for smoothing and rotation"
```

---

### Task 5: Add Body Tracking Example

**Files:**
- Create: `examples/body_tracking.rs`

**Step 1: Create example**

```rust
use bevy::prelude::*;
use bevy_vrm1::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, VrmPlugin))
        .add_systems(Startup, (spawn_camera_and_vrm, spawn_directional_light))
        .run();
}

fn spawn_camera_and_vrm(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 1.3, 1.0)));
    commands.spawn((
        VrmHandle(asset_server.load("vrm/AliciaSolid.vrm")),
        LookAt::Cursor,
        BodyTracking::default(),
    ));
}

fn spawn_directional_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 1.0, 0.0)),
    ));
}
```

**Step 2: Verify compilation**

Run: `cargo build --example body_tracking`
Expected: Compiles without errors.

**Step 3: Manual test**

Run: `cargo run --example body_tracking`
Expected: VRM model's head, neck, and spine follow the cursor with smooth motion.

**Step 4: Commit**

```bash
git add examples/body_tracking.rs
git commit -m "feat(body_tracking): add body_tracking example"
```

---

### Task 6: Final Cleanup and Documentation

**Files:**
- Modify: `src/vrm/body_tracking.rs` (add serde support)
- Modify: `src/vrm.rs` (verify prelude export)

**Step 1: Add serde support to BodyTracking**

Match the pattern used by other components (e.g., `LookAt`):

```rust
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct BodyTracking { ... }
```

Same for `SmoothedGaze`.

**Step 2: Verify all features compile**

Run: `cargo check --features serde,log`
Expected: Compiles without errors.

**Step 3: Run clippy**

Run: `cargo clippy`
Expected: No warnings.

**Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add src/vrm/body_tracking.rs
git commit -m "chore(body_tracking): add serde support and cleanup"
```
