# Body Tracking Feedback Loop Fix — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate the feedback loop in `track_body_tracking` by computing look_at_space from rest-pose orientation re-aligned by the VRM root's current rotation.

**Architecture:** Replace the look_at_space computation (currently derived from the head's current GlobalTransform, which includes body tracking from the previous frame) with a stable reference using `RestGlobalTransform`/`RestTransform` for orientation and a lazily-captured root rest rotation for re-alignment.

**Tech Stack:** Bevy 0.18, bevy_vrm1 (Rust)

---

### Task 1: Update system queries

**Files:**
- Modify: `src/vrm/body_tracking.rs:219-240`

**Step 1: Add Entity to vrms query, add root_gtfs query, update transforms filter, add root_rest_rots Local**

Change the `track_body_tracking` function signature from:

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
    mut transforms: Query<(&mut Transform, &mut GlobalTransform), Without<Camera>>,
    child_ofs: Query<&ChildOf>,
    rests: Query<(&RestTransform, &RestGlobalTransform)>,
    windows: Query<(Entity, &Window)>,
    cameras: Cameras,
    time: Res<Time>,
    mut bone_states: Local<HashMap<Entity, BoneState>>,
) {
    let dt = time.delta_secs();

    for (look_at, properties, tracking, head, neck, chest, spine, mut smoothed) in vrms.iter_mut() {
```

To:

```rust
fn track_body_tracking(
    mut vrms: Query<(
        Entity,
        &LookAt,
        &LookAtProperties,
        &BodyTracking,
        &HeadBoneEntity,
        Option<&NeckBoneEntity>,
        Option<&ChestBoneEntity>,
        Option<&SpineBoneEntity>,
        &mut SmoothedGaze,
    )>,
    mut transforms: Query<(&mut Transform, &mut GlobalTransform), (Without<Camera>, Without<Vrm>)>,
    root_gtfs: Query<&GlobalTransform, With<Vrm>>,
    child_ofs: Query<&ChildOf>,
    rests: Query<(&RestTransform, &RestGlobalTransform)>,
    windows: Query<(Entity, &Window)>,
    cameras: Cameras,
    time: Res<Time>,
    mut bone_states: Local<HashMap<Entity, BoneState>>,
    mut root_rest_rots: Local<HashMap<Entity, Quat>>,
) {
    let dt = time.delta_secs();

    for (root_entity, look_at, properties, tracking, head, neck, chest, spine, mut smoothed) in
        vrms.iter_mut()
    {
```

**Step 2: Run `cargo check`**

Run: `cargo check`
Expected: PASS (queries are valid, no conflicts)

**Step 3: Commit**

```bash
git add src/vrm/body_tracking.rs
git commit -m "refactor(body_tracking): update queries for feedback loop fix"
```

---

### Task 2: Replace look_at_space computation

**Files:**
- Modify: `src/vrm/body_tracking.rs:241-250`

**Step 1: Replace the look_at_space block**

Replace this block (the old feedback-prone computation):

```rust
        // 1. Get head GlobalTransform and build LookAt space.
        let Ok((&head_tf, &head_gtf)) = transforms.get(head.0) else {
            continue;
        };

        let look_at_space = GlobalTransform::default();
        let mut look_at_space_tf = look_at_space.reparented_to(&head_gtf);
        look_at_space_tf.translation = Vec3::from(properties.offset_from_head_bone);
        look_at_space_tf.rotation = head_tf.rotation.inverse();
        let look_at_space = head_gtf.mul_transform(look_at_space_tf);
```

With the stable computation:

```rust
        // 1. Build stable LookAt space using rest-pose orientation + root delta.
        let Ok((&head_tf, &head_gtf)) = transforms.get(head.0) else {
            continue;
        };
        let Ok((rest_tf, rest_gtf)) = rests.get(head.0) else {
            continue;
        };
        let Ok(root_gtf) = root_gtfs.get(root_entity) else {
            continue;
        };

        let root_rest_rot = *root_rest_rots
            .entry(root_entity)
            .or_insert(root_gtf.rotation());
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

**Step 2: Run `cargo check`**

Run: `cargo check`
Expected: PASS

**Step 3: Run existing tests**

Run: `cargo test`
Expected: All 30 unit tests + 8 doc-tests pass

**Step 4: Commit**

```bash
git add src/vrm/body_tracking.rs
git commit -m "fix(body_tracking): use rest-pose orientation for look_at_space

Eliminates the feedback loop where body tracking rotations from the
previous frame leaked into the look_at_space reference frame, causing
the model to get stuck or turn the wrong direction when the cursor
crossed sides."
```

---

### Task 3: Verify manually

**Step 1: Run the body_tracking example**

Run: `cargo run --example body_tracking`

**Step 2: Manual test procedure**

1. Move cursor to the right side of the screen — model should look right
2. Move cursor from right, through center, to the left — model should smoothly turn left
3. Move cursor to the top of the screen — model should tilt upward
4. Rapidly move cursor between left and right — model should smoothly follow without getting stuck

**Step 3: Run the look_at_cursor example to verify eye tracking is unaffected**

Run: `cargo run --example look_at_cursor`

Eye tracking should work as before (it uses a separate system and is not affected by this change).
