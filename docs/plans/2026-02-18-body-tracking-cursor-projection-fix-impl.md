# Body Tracking Cursor Projection Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix body tracking to produce proportional yaw when the camera is directly in front of the VRM model, instead of snapping to ~90 degrees.

**Architecture:** Add a `reference_depth` parameter to `BodyTracking` and a `calc_yaw_pitch_clamped` function that clamps the local Z component to `reference_depth` before computing `atan2`. This prevents the z~0 singularity when the cursor projection plane is at the same depth as the head.

**Tech Stack:** Rust, Bevy 0.18, bevy_vrm1

---

### Task 1: Add `reference_depth` field to `BodyTracking`

**Files:**
- Modify: `src/vrm/body_tracking.rs:29-63` (struct fields)
- Modify: `src/vrm/body_tracking.rs:65-84` (Default impl)

**Step 1: Add `reference_depth` field to the struct**

In `src/vrm/body_tracking.rs`, add a new field after `output_smoothing` (line 62):

```rust
    /// Minimum forward depth (meters) for yaw/pitch calculation.
    /// Prevents extreme yaw when the camera is directly in front of the model.
    /// Higher values reduce sensitivity. Default: 1.0.
    #[cfg_attr(feature = "serde", serde(default = "default_reference_depth"))]
    pub reference_depth: f32,
```

**Step 2: Add default value in `Default` impl**

In the `Default` impl (line 67-83), add after `output_smoothing: 25.0`:

```rust
            reference_depth: 1.0,
```

**Step 3: Add default function for serde**

After `default_output_smoothing` (line 86-88), add:

```rust
fn default_reference_depth() -> f32 {
    1.0
}
```

**Step 4: Run `cargo check` to verify compilation**

Run: `cargo check`
Expected: compiles successfully

**Step 5: Commit**

```bash
git add src/vrm/body_tracking.rs
git commit -m "feat(body_tracking): add reference_depth parameter to BodyTracking"
```

---

### Task 2: Add `calc_yaw_pitch_clamped` function with tests

**Files:**
- Modify: `src/vrm/body_tracking.rs` (add function after `bone_rotation`, add tests)

**Step 1: Write the failing tests**

Add these tests at the end of the `mod tests` block (before the closing `}`):

```rust
    #[test]
    fn test_calc_yaw_pitch_clamped_zero_depth_target() {
        // When target is at the same Z-depth as the look_at_space origin,
        // the clamped version should produce a small, proportional yaw
        // instead of ~90 degrees.
        let look_at_space = GlobalTransform::from(Transform {
            translation: Vec3::new(0.0, 1.36, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        });
        // Target slightly to the right, at the same Z as origin
        let target = Vec3::new(0.1, 1.3, 0.0);
        let min_depth = 1.0;

        let (yaw, _pitch) = calc_yaw_pitch_clamped(&look_at_space, target, min_depth);
        // With min_depth=1.0, yaw should be atan2(0.1, 1.0) ≈ 5.7 degrees
        assert!(
            yaw.abs() < 10.0,
            "Yaw should be small for slight offset: {yaw}"
        );
        assert!(yaw > 0.0, "Yaw should be positive for right offset: {yaw}");
    }

    #[test]
    fn test_calc_yaw_pitch_clamped_preserves_valid_depth() {
        // When target has meaningful Z depth (> min_depth), the clamp
        // should not activate and the result should match calc_yaw_pitch.
        let look_at_space = GlobalTransform::from(Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        });
        // Target at (1, 0, 3) — well in front, Z=3 > min_depth=1
        let target = Vec3::new(1.0, 0.0, 3.0);
        let min_depth = 1.0;

        let (clamped_yaw, clamped_pitch) =
            calc_yaw_pitch_clamped(&look_at_space, target, min_depth);
        let (original_yaw, original_pitch) = calc_yaw_pitch(&look_at_space, target);

        assert!(
            (clamped_yaw - original_yaw).abs() < 0.01,
            "Should match original when Z > min_depth: clamped={clamped_yaw}, original={original_yaw}"
        );
        assert!(
            (clamped_pitch - original_pitch).abs() < 0.01,
            "Pitch should match: clamped={clamped_pitch}, original={original_pitch}"
        );
    }

    #[test]
    fn test_calc_yaw_pitch_clamped_proportional() {
        // Yaw should be proportional to horizontal offset for small values.
        let look_at_space = GlobalTransform::from(Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        });
        let min_depth = 1.0;

        let (yaw1, _) =
            calc_yaw_pitch_clamped(&look_at_space, Vec3::new(0.1, 0.0, 0.0), min_depth);
        let (yaw2, _) =
            calc_yaw_pitch_clamped(&look_at_space, Vec3::new(0.2, 0.0, 0.0), min_depth);

        // yaw2 should be roughly 2x yaw1 for small angles
        let ratio = yaw2 / yaw1;
        assert!(
            (ratio - 2.0).abs() < 0.2,
            "Yaw should be roughly proportional: ratio={ratio}"
        );
    }

    #[test]
    fn test_calc_yaw_pitch_clamped_center_is_zero() {
        // When target is directly in front (along +Z or at origin), yaw should be ~0.
        let look_at_space = GlobalTransform::from(Transform {
            translation: Vec3::new(0.0, 1.3, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        });
        // Target at same position as origin (cursor at face center)
        let target = Vec3::new(0.0, 1.3, 0.0);
        let min_depth = 1.0;

        let (yaw, _pitch) = calc_yaw_pitch_clamped(&look_at_space, target, min_depth);
        assert!(
            yaw.abs() < 0.01,
            "Yaw should be ~0 when target is at origin: {yaw}"
        );
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib -- test_calc_yaw_pitch_clamped`
Expected: FAIL — `calc_yaw_pitch_clamped` not defined

**Step 3: Implement `calc_yaw_pitch_clamped`**

Add this function after `bone_rotation` (after line 167) in `src/vrm/body_tracking.rs`:

```rust
/// Like [`calc_yaw_pitch`] but clamps the local Z component to `min_depth`
/// before computing angles. This prevents extreme yaw values when the
/// cursor projection places the target at the same depth as the head
/// (e.g., camera directly in front of the model).
fn calc_yaw_pitch_clamped(
    look_at_space: &GlobalTransform,
    target: Vec3,
    min_depth: f32,
) -> (f32, f32) {
    let local_target = look_at_space.to_matrix().inverse().transform_point3(target);

    let x = local_target.dot(Vec3::X);
    let y = local_target.dot(Vec3::Y);
    let z = local_target.dot(Vec3::Z).max(min_depth);

    let yaw = (x.atan2(z)).to_degrees();
    let xz = (x * x + z * z).sqrt();
    let pitch = (-y.atan2(xz)).to_degrees();

    (yaw, pitch)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --lib -- test_calc_yaw_pitch_clamped`
Expected: all 4 tests PASS

**Step 5: Commit**

```bash
git add src/vrm/body_tracking.rs
git commit -m "feat(body_tracking): add calc_yaw_pitch_clamped with z-depth clamping"
```

---

### Task 3: Wire up `calc_yaw_pitch_clamped` in `track_body_tracking`

**Files:**
- Modify: `src/vrm/body_tracking.rs:272-286` (replace calc_yaw_pitch calls)

**Step 1: Replace `calc_yaw_pitch` with `calc_yaw_pitch_clamped` in `track_body_tracking`**

Change lines 272-286 from:

```rust
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
```

To:

```rust
        // 2. Calculate raw yaw/pitch with z-depth clamping.
        let (raw_yaw, raw_pitch) = match look_at {
            LookAt::Cursor => {
                let Some(target_pos) = find_cursor_world_position(&windows, &cameras, &head_gtf)
                else {
                    continue;
                };
                calc_yaw_pitch_clamped(&look_at_space, target_pos, tracking.reference_depth)
            }
            LookAt::Target(target_entity) => {
                let Ok((_, &target_gtf)) = transforms.get(*target_entity) else {
                    continue;
                };
                calc_yaw_pitch_clamped(
                    &look_at_space,
                    target_gtf.translation(),
                    tracking.reference_depth,
                )
            }
        };
```

**Step 2: Run all tests**

Run: `cargo test`
Expected: all tests PASS

**Step 3: Run clippy**

Run: `cargo clippy`
Expected: no warnings

**Step 4: Commit**

```bash
git add src/vrm/body_tracking.rs
git commit -m "fix(body_tracking): use z-depth clamped yaw calculation to prevent extreme rotation"
```

---

### Task 4: Manual verification with example

**Step 1: Run the body_tracking example**

Run: `cargo run --example body_tracking`

**Step 2: Verify behavior**

- Move cursor to the center of the face: body should face forward (no rotation)
- Move cursor slightly right: body should turn slightly right (proportional)
- Move cursor to the far right edge: body should turn further right (still within reasonable range)
- Move cursor slightly left: body should turn slightly left
- Verify smooth transitions between positions

**Step 3: Test edge case — cursor at screen edge**

Move cursor to the very edge of the window. Body should be rotated but not at an extreme 90-degree angle.
