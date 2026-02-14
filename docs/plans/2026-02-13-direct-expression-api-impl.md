# Direct Expression API Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `SetExpressions` and `ClearExpressions` events to directly control VRM expression weights from user code.

**Architecture:** An `ExpressionEntityMap` cache is built at initialization time for O(1) lookups. `SetExpressions` inserts `ExpressionOverride` components on expression entities. The existing `bind_expressions` system is modified to prefer overrides over VRMA animation values. `ClearExpressions` removes overrides.

**Tech Stack:** Bevy 0.18, bevy_vrm1 crate (Rust edition 2024)

---

### Task 1: Add ExpressionEntityMap component and build it during initialization

**Files:**
- Modify: `src/vrm/expressions.rs:13-17` (add new component types)
- Modify: `src/vrm/expressions.rs:92-123` (build map during initialization)
- Modify: `src/vrm/expressions.rs:65-77` (register new types)
- Test: `src/vrm/expressions.rs` (existing test module)

**Step 1: Write the failing test**

Add this test to the `#[cfg(test)] mod tests` block in `src/vrm/expressions.rs`:

```rust
#[test]
fn test_expression_entity_map_built_on_init() -> TestResult {
    let mut app = test_app();
    app.add_plugins(VrmExpressionPlugin);

    let vrm_entity = app
        .world_mut()
        .spawn((VrmExpressionRegistry(
            [(
                VrmExpression::from("happy"),
                vec![ExpressionNode {
                    name: Name::new("Test"),
                    morph_target_index: 0,
                }],
            )]
            .into_iter()
            .collect(),
        ),))
        .with_children(|c| {
            c.spawn(Name::new("Test"));
        })
        .id();

    app.world_mut()
        .commands()
        .entity(vrm_entity)
        .trigger(RequestInitializeExpressions);
    app.update();

    let map = app
        .world()
        .get::<ExpressionEntityMap>(vrm_entity)
        .expect("ExpressionEntityMap not found");

    assert!(map.0.contains_key(&VrmExpression::from("happy")));
    Ok(())
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_expression_entity_map_built_on_init -- --nocapture`
Expected: FAIL — `ExpressionEntityMap` type doesn't exist yet.

**Step 3: Write minimal implementation**

Add these types after line 17 in `src/vrm/expressions.rs`:

```rust
/// Cached mapping from expression name to expression entity.
/// Built during VRM initialization. Use this to query available expressions.
#[derive(Component, Deref, Reflect)]
pub struct ExpressionEntityMap(pub HashMap<VrmExpression, Entity>);

/// Override weight for a single expression entity.
/// Inserted by `SetExpressions`, removed by `ClearExpressions`.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct ExpressionOverride(pub f32);
```

In `apply_initialize_expressions` (line 92-123), build the map. After the `for` loop, insert `ExpressionEntityMap` on the VRM entity. Replace the function with:

```rust
fn apply_initialize_expressions(
    trigger: On<RequestInitializeExpressions>,
    mut commands: Commands,
    expressions: Query<&VrmExpressionRegistry>,
    searcher: ChildSearcher,
) {
    let vrm_entity = trigger.event_target();
    let expressions_root = commands.spawn(Name::new(Vrm::EXPRESSIONS_ROOT)).id();
    commands.entity(vrm_entity).add_child(expressions_root);

    let Ok(registry) = expressions.get(vrm_entity) else {
        commands.entity(vrm_entity).insert(ExpressionEntityMap(HashMap::default()));
        return;
    };

    let mut entity_map = HashMap::default();

    for (expression, nodes) in registry.iter() {
        let expression_entity = commands
            .spawn((
                Name::new(expression.to_string()),
                RetargetSource,
                Transform::default(),
                AnimationPlayer::default(),
                RetargetExpressionNodes(obtain_expression_nodes(vrm_entity, &searcher, nodes)),
            ))
            .id();
        commands.entity(expression_entity).insert((
            AnimationTargetId::from_name(&Name::new(expression.to_string())),
            AnimatedBy(expression_entity),
        ));
        commands
            .entity(expressions_root)
            .add_child(expression_entity);
        entity_map.insert(expression.clone(), expression_entity);
    }

    commands.entity(vrm_entity).insert(ExpressionEntityMap(entity_map));
}
```

In `VrmExpressionPlugin::build` (line 67-77), register the new types:

```rust
impl Plugin for VrmExpressionPlugin {
    fn build(
        &self,
        app: &mut App,
    ) {
        app.register_type::<BindExpressionNode>()
            .register_type::<RetargetExpressionNodes>()
            .register_type::<VrmExpressionRegistry>()
            .register_type::<ExpressionEntityMap>()
            .register_type::<ExpressionOverride>()
            .add_observer(apply_initialize_expressions);
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_expression_entity_map_built_on_init -- --nocapture`
Expected: PASS

**Step 5: Run all existing tests to check for regressions**

Run: `cargo test`
Expected: All tests PASS (including `test_obtain_expression_nodes`)

**Step 6: Commit**

```bash
git add src/vrm/expressions.rs
git commit -m "feat(expressions): add ExpressionEntityMap built at initialization"
```

---

### Task 2: Add SetExpressions event and observer

**Files:**
- Modify: `src/vrm/expressions.rs` (add event type, observer, convenience constructors)
- Test: `src/vrm/expressions.rs` (test module)

**Step 1: Write the failing test**

Add to `#[cfg(test)] mod tests` in `src/vrm/expressions.rs`:

```rust
#[test]
fn test_set_expressions() -> TestResult {
    let mut app = test_app();
    app.add_plugins(VrmExpressionPlugin);

    let vrm_entity = app
        .world_mut()
        .spawn((VrmExpressionRegistry(
            [(
                VrmExpression::from("happy"),
                vec![ExpressionNode {
                    name: Name::new("Test"),
                    morph_target_index: 0,
                }],
            )]
            .into_iter()
            .collect(),
        ),))
        .with_children(|c| {
            c.spawn(Name::new("Test"));
        })
        .id();

    // Initialize expressions
    app.world_mut()
        .commands()
        .entity(vrm_entity)
        .trigger(RequestInitializeExpressions);
    app.update();

    // Set expression
    app.world_mut()
        .commands()
        .trigger(SetExpressions::single(vrm_entity, "happy", 0.8));
    app.update();

    // Find the expression entity via the map
    let map = app.world().get::<ExpressionEntityMap>(vrm_entity).unwrap();
    let expr_entity = *map.0.get(&VrmExpression::from("happy")).unwrap();

    let override_val = app
        .world()
        .get::<ExpressionOverride>(expr_entity)
        .expect("ExpressionOverride not found");
    assert!((override_val.0 - 0.8).abs() < f32::EPSILON);
    Ok(())
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_set_expressions -- --nocapture`
Expected: FAIL — `SetExpressions` type doesn't exist yet.

**Step 3: Write minimal implementation**

Add after the `ExpressionOverride` definition in `src/vrm/expressions.rs`:

```rust
/// Sets expression weights on a VRM model.
///
/// Trigger this event to directly control facial expressions.
/// Expression weights are clamped to `0.0..=1.0`.
/// When both VRMA animation and `SetExpressions` control the same expression,
/// `SetExpressions` takes priority until [`ClearExpressions`] is triggered.
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_vrm1::prelude::*;
///
/// fn set_happy(mut commands: Commands, vrms: Query<Entity, With<Vrm>>) {
///     for vrm in vrms.iter() {
///         commands.trigger(SetExpressions::single(vrm, "happy", 1.0));
///     }
/// }
/// ```
#[derive(EntityEvent, Debug)]
pub struct SetExpressions {
    #[event_target]
    pub entity: Entity,
    pub weights: HashMap<VrmExpression, f32>,
}

impl SetExpressions {
    /// Creates a [`SetExpressions`] event for a single expression.
    pub fn single(entity: Entity, expression: impl Into<VrmExpression>, weight: f32) -> Self {
        Self {
            entity,
            weights: [(expression.into(), weight)].into_iter().collect(),
        }
    }

    /// Creates a [`SetExpressions`] event from an iterator of expression-weight pairs.
    pub fn from_iter(
        entity: Entity,
        iter: impl IntoIterator<Item = (impl Into<VrmExpression>, f32)>,
    ) -> Self {
        Self {
            entity,
            weights: iter.into_iter().map(|(e, w)| (e.into(), w)).collect(),
        }
    }
}
```

Add the observer function:

```rust
fn apply_set_expressions(
    trigger: On<SetExpressions>,
    cache: Query<&ExpressionEntityMap>,
    mut commands: Commands,
) {
    let vrm_entity = trigger.event_target();
    let Ok(map) = cache.get(vrm_entity) else {
        #[cfg(feature = "log")]
        warn!("SetExpressions: ExpressionEntityMap not found for entity {:?}. VRM may not be initialized yet.", vrm_entity);
        return;
    };
    for (expression, weight) in trigger.event().weights.iter() {
        let Some(&expr_entity) = map.0.get(expression) else {
            #[cfg(feature = "log")]
            warn!("SetExpressions: expression '{}' not found", expression);
            continue;
        };
        commands.entity(expr_entity).insert(ExpressionOverride(weight.clamp(0.0, 1.0)));
    }
}
```

Register the observer in `VrmExpressionPlugin::build`:

```rust
.add_observer(apply_initialize_expressions)
.add_observer(apply_set_expressions);
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_set_expressions -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/vrm/expressions.rs
git commit -m "feat(expressions): add SetExpressions event with observer"
```

---

### Task 3: Add ClearExpressions event and observer

**Files:**
- Modify: `src/vrm/expressions.rs` (add event, observer)
- Test: `src/vrm/expressions.rs` (test module)

**Step 1: Write the failing test**

Add to `#[cfg(test)] mod tests`:

```rust
#[test]
fn test_clear_expressions() -> TestResult {
    let mut app = test_app();
    app.add_plugins(VrmExpressionPlugin);

    let vrm_entity = app
        .world_mut()
        .spawn((VrmExpressionRegistry(
            [(
                VrmExpression::from("happy"),
                vec![ExpressionNode {
                    name: Name::new("Test"),
                    morph_target_index: 0,
                }],
            )]
            .into_iter()
            .collect(),
        ),))
        .with_children(|c| {
            c.spawn(Name::new("Test"));
        })
        .id();

    // Initialize
    app.world_mut()
        .commands()
        .entity(vrm_entity)
        .trigger(RequestInitializeExpressions);
    app.update();

    // Set expression
    app.world_mut()
        .commands()
        .trigger(SetExpressions::single(vrm_entity, "happy", 0.8));
    app.update();

    // Verify override exists
    let map = app.world().get::<ExpressionEntityMap>(vrm_entity).unwrap();
    let expr_entity = *map.0.get(&VrmExpression::from("happy")).unwrap();
    assert!(app.world().get::<ExpressionOverride>(expr_entity).is_some());

    // Clear expressions
    app.world_mut()
        .commands()
        .trigger(ClearExpressions { entity: vrm_entity });
    app.update();

    // Verify override removed
    assert!(app.world().get::<ExpressionOverride>(expr_entity).is_none());
    Ok(())
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_clear_expressions -- --nocapture`
Expected: FAIL — `ClearExpressions` type doesn't exist yet.

**Step 3: Write minimal implementation**

Add after `SetExpressions` impl block in `src/vrm/expressions.rs`:

```rust
/// Clears expression overrides, returning control to VRMA animation.
///
/// After triggering this event, expressions previously set by [`SetExpressions`]
/// will be controlled by VRMA animation again.
#[derive(EntityEvent, Debug)]
pub struct ClearExpressions {
    #[event_target]
    pub entity: Entity,
}
```

Add the observer function:

```rust
fn apply_clear_expressions(
    trigger: On<ClearExpressions>,
    cache: Query<&ExpressionEntityMap>,
    mut commands: Commands,
) {
    let vrm_entity = trigger.event_target();
    let Ok(map) = cache.get(vrm_entity) else {
        return;
    };
    for &expr_entity in map.0.values() {
        commands.entity(expr_entity).remove::<ExpressionOverride>();
    }
}
```

Register in `VrmExpressionPlugin::build`:

```rust
.add_observer(apply_initialize_expressions)
.add_observer(apply_set_expressions)
.add_observer(apply_clear_expressions);
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_clear_expressions -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/vrm/expressions.rs
git commit -m "feat(expressions): add ClearExpressions event with observer"
```

---

### Task 4: Modify bind_expressions to check ExpressionOverride

**Files:**
- Modify: `src/vrma/animation/expressions.rs:48-65` (modify bind_expressions system)
- Test: `src/vrma/animation/expressions.rs` (add test)

**Step 1: Write the failing test**

Add a `#[cfg(test)] mod tests` block at the end of `src/vrma/animation/expressions.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::tests::{TestResult, test_app};
    use crate::vrm::expressions::ExpressionOverride;
    use bevy::prelude::*;
    use bevy::render::mesh::morph::MorphWeights;

    use super::*;

    #[test]
    fn test_bind_expressions_prefers_override() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmaRetargetExpressionsPlugin);

        // Create a mesh entity with morph weights
        let mesh_entity = app
            .world_mut()
            .spawn(MorphWeights::new(vec![0.0], None))
            .id();

        // Create an expression entity with VRMA value (Transform) and override
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.3, 0.0, 0.0)),
            RetargetExpressionNodes(vec![BindExpressionNode {
                expression_entity: mesh_entity,
                index: 0,
            }]),
            ExpressionOverride(0.9),
        ));
        app.update();

        let morph = app.world().get::<MorphWeights>(mesh_entity).unwrap();
        assert!(
            (morph.weights()[0] - 0.9).abs() < f32::EPSILON,
            "Expected override value 0.9, got {}",
            morph.weights()[0]
        );
        Ok(())
    }

    #[test]
    fn test_bind_expressions_falls_back_to_transform() -> TestResult {
        let mut app = test_app();
        app.add_plugins(VrmaRetargetExpressionsPlugin);

        let mesh_entity = app
            .world_mut()
            .spawn(MorphWeights::new(vec![0.0], None))
            .id();

        // No ExpressionOverride — should use Transform.translation.x
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.5, 0.0, 0.0)),
            RetargetExpressionNodes(vec![BindExpressionNode {
                expression_entity: mesh_entity,
                index: 0,
            }]),
        ));
        app.update();

        let morph = app.world().get::<MorphWeights>(mesh_entity).unwrap();
        assert!(
            (morph.weights()[0] - 0.5).abs() < f32::EPSILON,
            "Expected VRMA value 0.5, got {}",
            morph.weights()[0]
        );
        Ok(())
    }
}
```

**Step 2: Run tests to verify the override test fails**

Run: `cargo test test_bind_expressions_prefers_override -- --nocapture`
Expected: FAIL — `bind_expressions` ignores `ExpressionOverride`, writes 0.3 instead of 0.9.

**Step 3: Modify bind_expressions**

In `src/vrma/animation/expressions.rs`, update imports and modify `bind_expressions`:

Add import at top:
```rust
use crate::vrm::expressions::ExpressionOverride;
```

Replace `bind_expressions` function (lines 48-65):

```rust
fn bind_expressions(
    mut expressions: Query<&mut MorphWeights>,
    rig_expressions: Query<
        (&Transform, &RetargetExpressionNodes, Option<&ExpressionOverride>),
        Or<(Changed<Transform>, Changed<ExpressionOverride>)>,
    >,
) {
    for (tf, RetargetExpressionNodes(binds), maybe_override) in rig_expressions.iter() {
        let weight = match maybe_override {
            Some(ExpressionOverride(w)) => *w,
            None => tf.translation.x,
        };
        for BindExpressionNode {
            expression_entity,
            index,
        } in binds.iter()
        {
            if let Ok(mut morph_weights) = expressions.get_mut(*expression_entity) {
                morph_weights.weights_mut()[*index] = weight;
            }
        }
    }
}
```

Key changes:
- Added `Option<&ExpressionOverride>` to the query
- Changed filter from `Changed<Transform>` to `Or<(Changed<Transform>, Changed<ExpressionOverride>)>`
- Override value takes priority over `Transform.translation.x`

**Step 4: Run tests to verify they pass**

Run: `cargo test test_bind_expressions -- --nocapture`
Expected: Both tests PASS

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests PASS

**Step 6: Commit**

```bash
git add src/vrma/animation/expressions.rs
git commit -m "feat(expressions): bind_expressions prefers ExpressionOverride over VRMA"
```

---

### Task 5: Export new types in prelude

**Files:**
- Modify: `src/vrm/expressions.rs:1` (change module visibility for new types)
- Modify: `src/vrm.rs:28-39` (add to prelude)

**Step 1: Verify current prelude exports compile**

Run: `cargo check`
Expected: PASS

**Step 2: Add exports to prelude**

In `src/vrm.rs`, modify the prelude (lines 28-39). Add `expressions::SetExpressions`, `expressions::ClearExpressions`, and `expressions::ExpressionEntityMap`:

```rust
pub mod prelude {
    pub use crate::vrm::{
        Initialized, RestGlobalTransform, RestTransform, Vrm, VrmBone, VrmExpression, VrmPath,
        VrmPlugin,
        expressions::{ClearExpressions, ExpressionEntityMap, SetExpressions},
        gltf::prelude::*,
        humanoid_bone::prelude::*,
        loader::{VrmAsset, VrmHandle},
        look_at::LookAt,
        mtoon::prelude::*,
        spring_bone::{SpringJointProps, SpringJoints, SpringRoot},
    };
}
```

**Step 3: Check compilation**

Run: `cargo check`
Expected: PASS — if visibility errors occur, ensure `SetExpressions`, `ClearExpressions`, and `ExpressionEntityMap` are `pub` (not `pub(crate)`) in `expressions.rs`.

**Step 4: Run all tests**

Run: `cargo test`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/vrm.rs src/vrm/expressions.rs
git commit -m "feat(expressions): export SetExpressions, ClearExpressions, ExpressionEntityMap in prelude"
```

---

### Task 6: Add expressions example

**Files:**
- Create: `examples/expressions.rs`

**Step 1: Write the example**

Create `examples/expressions.rs`:

```rust
//! This example shows how to directly control VRM expressions from code.
//!
//! Press number keys to trigger expressions:
//! - 1: happy
//! - 2: angry
//! - 3: sad
//! - 4: blink
//! - 0: clear all expressions (return to VRMA control)

use bevy::prelude::*;
use bevy_vrm1::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, VrmPlugin))
        .add_systems(Startup, (spawn_light, spawn_camera, spawn_vrm))
        .add_systems(Update, control_expressions)
        .run();
}

fn spawn_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(3.0, 3.0, 0.3).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(0., 0.8, 2.5)));
}

fn spawn_vrm(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(VrmHandle(asset_server.load("vrm/AliciaSolid.vrm")));
}

fn control_expressions(
    mut commands: Commands,
    vrms: Query<Entity, With<Vrm>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for vrm in vrms.iter() {
        if input.just_pressed(KeyCode::Digit1) {
            commands.trigger(SetExpressions::single(vrm, "happy", 1.0));
        }
        if input.just_pressed(KeyCode::Digit2) {
            commands.trigger(SetExpressions::single(vrm, "angry", 1.0));
        }
        if input.just_pressed(KeyCode::Digit3) {
            commands.trigger(SetExpressions::single(vrm, "sad", 1.0));
        }
        if input.just_pressed(KeyCode::Digit4) {
            commands.trigger(SetExpressions::single(vrm, "blink", 1.0));
        }
        if input.just_pressed(KeyCode::Digit0) {
            commands.trigger(ClearExpressions { entity: vrm });
        }
    }
}
```

**Step 2: Verify compilation**

Run: `cargo build --example expressions`
Expected: PASS

**Step 3: Commit**

```bash
git add examples/expressions.rs
git commit -m "feat(expressions): add expressions example for direct expression control"
```

---

### Task 7: Final verification

**Step 1: Run clippy**

Run: `cargo clippy`
Expected: No warnings

**Step 2: Run all tests**

Run: `cargo test`
Expected: All tests PASS

**Step 3: Run the example manually (optional)**

Run: `cargo run --example expressions`
Expected: VRM model loads. Press 1-4 to trigger expressions, 0 to clear.

**Step 4: Final commit if clippy fixes needed**

```bash
git add -A
git commit -m "chore: address clippy warnings"
```
