# Direct Expression API Design

## Problem

VRM expressions (facial morph targets like "happy", "blink") can only be triggered through VRMA animation files. There is no public API to set expression weights directly from user code.

## Solution

Add `SetExpressions` and `ClearExpressions` entity events that users trigger on VRM entities to set or release expression weight overrides.

## API

```rust
/// Sets expression weights on a VRM model.
///
/// Trigger this event on the VRM root entity (the entity with the `Vrm` component).
/// Expression weights are clamped to `0.0..=1.0`.
/// When both VRMA animation and `SetExpressions` control the same expression,
/// `SetExpressions` takes priority until `ClearExpressions` is triggered.
#[derive(EntityEvent)]
pub struct SetExpressions {
    #[event_target]
    pub entity: Entity,
    pub weights: HashMap<VrmExpression, f32>,
}

impl SetExpressions {
    /// Creates a `SetExpressions` event for a single expression.
    pub fn single(entity: Entity, expression: impl Into<VrmExpression>, weight: f32) -> Self {
        Self {
            entity,
            weights: [(expression.into(), weight)].into_iter().collect(),
        }
    }

    /// Creates a `SetExpressions` event from an iterator of expression-weight pairs.
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

/// Clears expression overrides, returning control to VRMA animation.
/// If no expression names are specified, clears all overrides.
#[derive(EntityEvent)]
pub struct ClearExpressions {
    #[event_target]
    pub entity: Entity,
}
```

Usage:

```rust
// Single expression
commands.trigger(SetExpressions::single(vrm_entity, "happy", 0.8));

// Multiple expressions
commands.trigger(SetExpressions::from_iter(vrm_entity, [
    ("happy", 0.8),
    ("blink", 1.0),
]));

// Release overrides back to VRMA
commands.trigger(ClearExpressions { entity: vrm_entity });
```

## Architecture

### Override Layer (key design change from v1)

The original design wrote directly to `Transform.translation.x`, but this is overwritten by Bevy's animation system in `PostUpdate::AnimationSystems` before `bind_expressions` runs. The revised design introduces an explicit override layer:

1. **`ExpressionOverride(f32)`** component — attached to individual expression entities when the user sets a weight via `SetExpressions`
2. **`ExpressionEntityMap`** component — a cached `HashMap<VrmExpression, Entity>` built at initialization time on the VRM entity, enabling O(1) lookups instead of per-trigger hierarchy traversal

### Data Flow

```
SetExpressions triggered
    ↓
Observer: ExpressionEntityMap lookup (O(1))
    ↓
Insert ExpressionOverride(weight) on expression entity
    ↓
bind_expressions (VrmSystemSets::Expressions, after AnimationSystems):
    - If ExpressionOverride exists → use override value
    - Else → use Transform.translation.x (VRMA value)
    ↓
Write to MorphWeights
```

### ClearExpressions Flow

```
ClearExpressions triggered
    ↓
Observer: remove ExpressionOverride from all expression entities under VRM
    ↓
bind_expressions: no override → falls back to Transform.translation.x (VRMA)
```

## New Components

```rust
/// Cached mapping from expression name to expression entity.
/// Built at initialization, public for user introspection.
#[derive(Component)]
pub struct ExpressionEntityMap(pub HashMap<VrmExpression, Entity>);

/// Override weight for a single expression entity.
/// Inserted by SetExpressions, removed by ClearExpressions.
#[derive(Component)]
pub(crate) struct ExpressionOverride(pub f32);
```

## Changes

### `src/vrm/expressions.rs`
- Add `SetExpressions` entity event (public)
- Add `ClearExpressions` entity event (public)
- Add `ExpressionEntityMap` component (public)
- Add `ExpressionOverride` component (pub(crate))
- Add `apply_set_expressions` observer — reads `ExpressionEntityMap`, inserts `ExpressionOverride`
- Add `apply_clear_expressions` observer — removes `ExpressionOverride`
- Build `ExpressionEntityMap` in `apply_initialize_expressions`
- Add `#[cfg(feature = "log")]` warnings for missing expressions / uninitialized VRM
- Register new observer and types in `VrmExpressionPlugin::build`
- Add unit tests

### `src/vrma/animation/expressions.rs`
- Modify `bind_expressions` to check `ExpressionOverride` before `Transform.translation.x`

### `src/vrm.rs` (prelude)
- Export `SetExpressions`, `ClearExpressions`, `ExpressionEntityMap` from prelude

### `examples/expressions.rs`
- New example demonstrating keyboard-driven expression control
- Show querying available expressions via `ExpressionEntityMap`
- Show setting and clearing expressions

## Testing

1. **test_set_expressions**: Trigger `SetExpressions` → verify `ExpressionOverride` is inserted with correct weight
2. **test_clear_expressions**: Trigger `SetExpressions` then `ClearExpressions` → verify `ExpressionOverride` is removed
3. **test_bind_expressions_with_override**: Verify `bind_expressions` prefers `ExpressionOverride` over `Transform.translation.x`
4. **test_invalid_expression_name**: Trigger with nonexistent expression name → verify no panic, silent skip

## Review Findings Addressed

| Issue | Severity | Resolution |
|-------|----------|------------|
| VRMA overwrites observer-set Transform values | CRITICAL | ExpressionOverride component layer (not Transform) |
| EntityEvent requires Entity field | CRITICAL | Named struct with `#[event_target]` |
| Entity name collision in hierarchy search | CRITICAL | ExpressionEntityMap cache eliminates hierarchy search |
| ChildSearcher per-trigger traversal cost | HIGH | ExpressionEntityMap O(1) lookup |
| Silent failure before initialization | HIGH | `#[cfg(feature = "log")]` warnings |
| No mechanism to release control to VRMA | HIGH | ClearExpressions event |
| Cannot query available expressions | MEDIUM | ExpressionEntityMap is public |
| No convenience constructors | MEDIUM | single(), from_iter() |
| Tuple struct not extensible | MEDIUM | Named struct with fields |
