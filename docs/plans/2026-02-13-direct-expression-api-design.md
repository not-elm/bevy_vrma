# Direct Expression API Design

## Problem

VRM expressions (facial morph targets like "happy", "blink") can only be triggered through VRMA animation files. There is no public API to set expression weights directly from user code.

## Solution

Add a `SetExpressions` entity event that users trigger on VRM entities to set expression weights directly.

## API

```rust
#[derive(EntityEvent)]
pub struct SetExpressions(pub HashMap<VrmExpression, f32>);
```

Usage:

```rust
commands.entity(vrm_entity).trigger(SetExpressions(
    [(VrmExpression::from("happy"), 0.8),
     (VrmExpression::from("blink"), 1.0)].into()
));
```

Weights are clamped to `0.0..=1.0`.

## Implementation

An observer registered in `VrmExpressionPlugin` handles `SetExpressions`:

1. Receives the VRM entity and expression weight map
2. Uses `ChildSearcher` to find expression entities by name under the VRM hierarchy
3. Writes the weight to `Transform.translation.x` on each matching expression entity
4. The existing `bind_expressions` system (which uses `Changed<Transform>`) picks up the change and applies morph weights to meshes

### VRMA Override Behavior

When both VRMA animation and the direct API write to the same expression, the direct API wins because `SetExpressions` sets `Transform.translation.x` after VRMA's animation pass. The `bind_expressions` system reads the final `Transform` value.

## Changes

### `src/vrm/expressions.rs`
- Add `SetExpressions` entity event (public)
- Add `apply_set_expressions` observer
- Register observer in `VrmExpressionPlugin::build`
- Add unit test

### `src/vrm.rs` (prelude)
- Export `SetExpressions` from `expressions` module
- Add `SetExpressions` to prelude

### `examples/expressions.rs`
- New example demonstrating keyboard-driven expression control
- Press keys to trigger expressions on a VRM model

## Testing

Unit test verifying that triggering `SetExpressions` on a VRM entity correctly updates the expression entity's `Transform.translation.x` to the specified weight value.
