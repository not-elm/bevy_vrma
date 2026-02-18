# Body Tracking Cursor Projection Fix Design

## Problem

When a camera is placed directly in front of a VRM model with both `LookAt::Cursor` and `BodyTracking` active, any horizontal cursor movement causes the body to turn nearly 90 degrees sideways instead of proportionally tracking the cursor.

### Root Cause

`to_world_by_viewport` projects the cursor ray onto a camera-perpendicular plane passing through the head position. When the camera faces the model directly, this plane is at the same Z-depth as the head in the model's local frame.

In `calc_yaw_pitch`, `yaw = atan2(x, z)` where `z` is the forward component. Since all projected targets have `z ≈ 0`, any horizontal offset produces `atan2(nonzero, ~0) ≈ ±90°`.

The eye LookAt system hides this via VRM range maps (90 degrees input mapped to ~20 degrees output). Body tracking has no such compression — per-bone clamping allows up to 100 degrees total body rotation.

## Solution: Local-Z Clamping

Clamp the local Z component to a minimum positive value before computing `atan2`. This ensures a meaningful forward depth exists for the yaw calculation.

```rust
let z = local_target.dot(Vec3::Z).max(min_depth);
let yaw = (x.atan2(z)).to_degrees();
```

### Why This Approach

An expert team evaluated 5 approaches:

| Approach | Verdict |
|----------|---------|
| Forward-plane projection | Good but discards valid Z in non-degenerate cases |
| **Local-Z clamping** | **Optimal**: identical to forward-plane when needed, preserves valid Z otherwise |
| Yaw range mapping | Broken: cannot recover lost information from atan2 |
| Direction normalization | Broken: z=0 in direction too |
| Closest point on ray | Broken: still gives z≈0 near head depth |

Z-clamping is mathematically equivalent to forward-plane projection in the degenerate case (`z < min_depth`), but strictly superior because it preserves the natural Z depth when it is valid (off-axis camera, angled views).

### Mathematical Properties

- Continuous (no discontinuities)
- Monotonic (yaw increases with horizontal cursor offset)
- Proportional for small offsets (`atan(x/d) ≈ x/d`)
- Bounded to (-90, +90) degrees
- No singularities (`min_depth > 0` guarantees valid atan2)

### Sensitivity Table

| min_depth | Sensitivity at center | Max yaw for 1m offset |
|-----------|-----------------------|-----------------------|
| 0.5 | 114.6 deg/m | ~63.4 deg |
| 1.0 | 57.3 deg/m | ~45.0 deg |
| 2.0 | 28.6 deg/m | ~26.6 deg |

Default `min_depth = 1.0` provides ~57 deg/m sensitivity, matching typical camera distance.

## Changes

### `BodyTracking` component

Add a `reference_depth: f32` field (default 1.0) that controls the minimum Z depth used in yaw/pitch calculation.

### `body_tracking.rs`

Add a `calc_yaw_pitch_clamped` function that clamps `local_target.z` to `reference_depth` before computing atan2. Use this instead of `calc_yaw_pitch` in `track_body_tracking`.

### Scope

- Only body tracking is modified
- Eye LookAt continues using existing `calc_yaw_pitch` (range maps handle extreme values)
- `to_world_by_viewport` is unchanged
- `LookAt::Target` path is unaffected (targets naturally have depth)
