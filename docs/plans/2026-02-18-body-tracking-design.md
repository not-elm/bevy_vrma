# Design: Body Tracking for LookAt System

## Context

The current LookAt system only rotates eye bones (leftEye, rightEye) toward a gaze target. Desktop companion applications like MateEngine also rotate the head, neck, chest, and spine toward the target for more natural-looking tracking. This design adds an optional `BodyTracking` component that enables this behavior.

## Decision

Add body tracking as an opt-in feature via a new `BodyTracking` component, running within the existing `VrmSystemSets::GazeControl` before the eye LookAt system. Uses fractional weight distribution with per-bone clamping, global yaw/pitch smoothing, and manual chain propagation (following the SpringBone precedent).

## Design (Validated by Expert Debate)

This design was validated by a 5-expert debate team (VRM 1.0 Spec, Bevy ECS, MateEngine UX, Performance, Skeletal Animation) across 2 rounds of cross-review, plus an independent Codex review that identified 5 additional issues (all addressed below).

### Component API

```rust
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct BodyTracking {
    // Fractional weights: fraction of total gaze angle applied to each bone
    pub head_weight: f32,    // default: 0.4
    pub neck_weight: f32,    // default: 0.25
    pub chest_weight: f32,   // default: 0.2
    pub spine_weight: f32,   // default: 0.15

    // Per-bone clamp limits (degrees)
    pub head_yaw_max: f32,    // default: 40.0
    pub head_pitch_max: f32,  // default: 30.0
    pub neck_yaw_max: f32,    // default: 25.0
    pub neck_pitch_max: f32,  // default: 20.0
    pub chest_yaw_max: f32,   // default: 20.0
    pub chest_pitch_max: f32, // default: 0.0 (yaw-only)
    pub spine_yaw_max: f32,   // default: 15.0
    pub spine_pitch_max: f32, // default: 0.0 (yaw-only)

    // Smoothing speed (higher = faster response)
    pub smoothing: f32,       // default: 10.0
}
```

Per-bone smoothing state stored as a separate component on the VRM root entity:

```rust
#[derive(Component)]
pub struct SmoothedGaze {
    pub yaw: f32,
    pub pitch: f32,
}
```

Note: State is stored as yaw/pitch scalars (not Quat) to avoid angle-wrap artifacts near +/-180 degrees and to match the smoothing domain (yaw/pitch level, pre-distribution).

### Algorithm

1. Calculate raw yaw/pitch from LookAt space to target (same as existing eye code)
2. Apply frame-rate-independent smoothing at yaw/pitch level (before bone distribution):
   `smoothed += shortest_arc_delta(raw, smoothed) * (1.0 - (-speed * dt).exp())`
   Uses shortest-arc delta to avoid long-way interpolation near +/-180 degrees.
3. Distribute smoothed yaw/pitch to each bone: `bone_yaw = smoothed_yaw * weight`
4. Clamp per bone: `bone_yaw.clamp(-yaw_max, yaw_max)`
5. Apply rotation using existing formula: `rest_tf * rest_gtf.inverse() * euler * rest_gtf`
6. Apply bottom-up: Spine -> Chest -> (UpperChest) -> Neck -> Head
7. Manual chain propagation for GlobalTransform (see below)
8. Eye system then recalculates remaining gaze from updated head GlobalTransform

### Weight Model: Fractional + Clamp

Each bone receives `weight * total_angle`, clamped to its max. This models human biomechanics where proximal joints (spine) engage less than distal joints (head) for small gaze deviations.

Default weights sum to 1.0 (head=0.4 + neck=0.25 + chest=0.2 + spine=0.15). Eyes handle any residual naturally through the existing LookAt recalculation.

Rationale (from debate): Independent per-bone targets (MateEngine style) cause overshoot for small angles -- all bones independently rotate the full gaze angle, producing 3-4x total body rotation. Fractional weights prevent this while remaining configurable (set weight=1.0 to approximate independent behavior).

### Spine/Chest Pitch: Yaw-Only by Default

Spine and chest pitch defaults are 0.0 (yaw-only). For desktop companions viewed from a fixed angle, spine pitch looks like bowing/leaning and fights with idle breathing animations. Head and neck handle vertical tracking sufficiently. Users can enable spine pitch by setting `spine_pitch_max > 0.0`.

### Smoothing

Global pre-distribution smoothing using frame-rate-independent exponential decay. Applied to yaw/pitch before distributing to bones, ensuring all bones move in lockstep as a coordinated unit (avoids per-bone convergence rate mismatch).

### Propagation: Manual Chain (Not Full Pass)

Instead of adding a 3rd full `propagate_parent_transforms` pass (O(N) over all entities), manually compute GlobalTransform for just the bone chain (O(6-8) matrix multiplications):

```rust
// Walk the actual ChildOf chain from each modified bone down to head,
// not a fixed humanoid bone list (glTF hierarchy may have intermediate nodes).
fn propagate_chain(from: Entity, to: Entity, transforms: &mut Query<...>) {
    // Collect actual parent chain from `to` up to `from` via ChildOf
    let chain = collect_ancestors(to, from);
    for entity in chain {
        let parent_gtf = global_transforms.get(child_of.parent());
        *global_transform = parent_gtf.mul_transform(*transform);
    }
}
```

This pattern is already used by SpringBone (`src/vrm/spring_bone/update.rs:83-89`) and is proven safe in Bevy 0.18.

**Important**: The chain must be built from actual `ChildOf` relationships, not assumed humanoid bone ordering. VRM humanoid bone mappings do not guarantee direct parent-child adjacency in the glTF node hierarchy -- intermediate nodes may exist between e.g. Spine and Chest. The `PropagateAfterExpressions` pass correctly propagates non-chain branches (shoulders, arms) before SpringBone, because `Changed<Transform>` on modified bones triggers Bevy's dirty tree marking.

### System Ordering

```
PropagateAfterConstraints (existing)
    -> track_body_tracking (new: rotation + smoothing + manual chain propagation)
    -> track_looking_target (existing eye system, unchanged)
    -> Expressions (existing)
    -> PropagateAfterExpressions (existing)
    -> SpringBone (existing)
```

Explicit ordering constraints:
- `track_body_tracking.before(track_looking_target)` -- must be explicit, not rely on set membership alone
- `track_body_tracking.after(VrmSystemSets::PropagateAfterConstraints)`
- `track_body_tracking.in_set(VrmSystemSets::GazeControl)`

Run condition: `.run_if(any_with_component::<BodyTracking>)`

### Input Validation

The `BodyTracking` component should validate or clamp inputs:
- `smoothing >= 0.0` (negative smoothing is nonsensical)
- All `*_max` fields `>= 0.0`
- All `*_weight` fields `>= 0.0` (negative weights would reverse tracking direction)

### Bone Hierarchy

Tracked bones (all optional except Head):
- Spine (`SpineBoneEntity`) - optional
- Chest (`ChestBoneEntity`) - optional
- UpperChest (`UpperChestBoneEntity`) - optional
- Neck (`NeckBoneEntity`) - optional
- Head (`HeadBoneEntity`) - required (already used by eye LookAt)

Missing bones are gracefully skipped. Their weight is not redistributed by default, which may result in reduced tracking responsiveness on rigs lacking spine/chest. Users can compensate by increasing remaining bone weights.

## Known Limitations (v1)

1. **Expression `override_look_at` not respected**: Body tracking does not attenuate when expressions override LookAt. This matches the existing bone-type eye system behavior (which also ignores `override_look_at`). Will be addressed when expression-type LookAt is implemented.

2. **Constraint conflict**: If Node Constraints target the same bones as BodyTracking (spine/chest/head), body tracking overwrites constraint results. Document this interaction.

3. **Return to rest**: When cursor leaves the window, body stays at last tracked position. Smoothing infrastructure makes adding rest-return trivial in v2.

## Usage Example

```rust
commands.spawn((
    VrmHandle(asset_server.load("model.vrm")),
    LookAt::Cursor,
    BodyTracking {
        head_weight: 0.4,
        neck_weight: 0.25,
        chest_weight: 0.2,
        spine_weight: 0.15,
        smoothing: 10.0,
        ..default()
    },
));
```

## Expert Review Summary

| Expert | Verdict |
|--------|---------|
| VRM 1.0 Spec | Approved: No spec violations. Body bones outside LookAt spec scope. |
| Bevy ECS | Approved: Manual chain propagation safe (SpringBone precedent). Run conditions correct. |
| MateEngine UX | Approved with smoothing. Spine yaw-only confirmed by MateEngine source analysis. |
| Performance | Approved: Manual chain O(6-8) vs full pass O(N). Smoothing cost negligible (~350 FLOPs). |
| Skeletal Animation | Approved: YXZ Euler correct. Fractional weights anatomically superior. Formula correct for body bones. |
