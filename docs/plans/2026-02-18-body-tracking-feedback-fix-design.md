# Fix Body Tracking Feedback Loop

## Problem

`track_body_tracking` computes `look_at_space` from the current `head_gtf`, which includes body tracking rotations from the previous frame. This creates a feedback loop:

1. Body tracking rotates spine/chest/neck (~60% of smoothed_yaw feeds back)
2. Next frame: `look_at_space` uses rotated `head_gtf`
3. `calc_yaw_pitch` computes raw angles in this rotated frame
4. When cursor crosses sides (e.g., right to left), raw_yaw can exceed ±180° in the rotated frame
5. `smooth_angle` takes the shortest arc, which goes the wrong direction (further away from target)

Result: the model gets stuck or turns the wrong way when the cursor moves to the opposite side.

## Root Cause

The look_at_space orientation is: `head_gtf.rotation * head_tf.rotation.inverse()` = parent chain global rotation (includes neck/chest/spine body tracking from last frame).

With default weights (spine=0.15, chest=0.20, neck=0.25), 60% of the smoothed_yaw rotates the look_at_space reference frame, creating positive feedback.

## Solution: Root-Aligned Rest-Pose Orientation

Replace the look_at_space orientation with a stable reference that:
- Uses the head's rest-pose parent orientation (no body tracking)
- Re-aligns by the VRM root entity's current rotation (follows root movement)

### Formula

```
rest_parent_rot = head_rest_gtf.rotation * head_rest_tf.rotation.inverse()
relative_to_root = root_rest_rotation.inverse() * rest_parent_rot
stable_rotation = root_current_gtf.rotation * relative_to_root
```

When root hasn't rotated: `stable_rotation = rest_parent_rot` (pure rest pose).
When root rotates: rest orientation follows the root.
Body tracking rotations never appear in the reference frame.

## Changes

### File: `src/vrm/body_tracking.rs`

#### Query changes

| Query | Change |
|-------|--------|
| `vrms` | Add `Entity` as first element |
| `transforms` | Add `Without<Vrm>` to filter |
| NEW `root_gtfs` | `Query<&GlobalTransform, With<Vrm>>` |
| NEW `root_rest_rots` | `Local<HashMap<Entity, Quat>>` |

Adding `Without<Vrm>` to `transforms` makes it provably disjoint from `root_gtfs` (bone entities never have `Vrm`).

#### Look_at_space computation (replaces lines 246-250)

```rust
let root_gtf = root_gtfs.get(root_entity)?;
let root_rest_rot = *root_rest_rots.entry(root_entity).or_insert(root_gtf.rotation());

let Ok((rest_tf, rest_gtf)) = rests.get(head.0) else { continue; };
let rest_parent_rot = rest_gtf.rotation() * rest_tf.rotation.inverse();
let relative_to_root = root_rest_rot.inverse() * rest_parent_rot;
let stable_rotation = root_gtf.rotation() * relative_to_root;

let offset = stable_rotation * Vec3::from(properties.offset_from_head_bone);
let look_at_space = GlobalTransform::from(Transform {
    translation: head_gtf.translation() + offset,
    rotation: stable_rotation,
    scale: Vec3::ONE,
});
```

### What stays the same

- `calc_yaw_pitch` (shared with eye LookAt)
- `smooth_angle`, `bone_rotation`, `compute_additive_rotation`
- Per-bone weight/clamp logic
- Output smoothing and chain propagation

## Testing

- Existing unit tests pass unchanged
- Manual verification: `cargo run --example body_tracking`, cursor right-to-left transition is smooth

## Limitation

Rest-pose orientation does not follow animation torso rotations (e.g., dance animations that twist the upper body). For typical cursor tracking use, this is acceptable. Can be enhanced with Approach C (reconstruct animation-only chain) if needed.
