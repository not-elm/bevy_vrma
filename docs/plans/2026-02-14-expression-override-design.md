# Design: VRM 1.0 Expression Override System

## Context

`ModifyExpressions` (additive partial update) was added but revealed deeper spec-compliance issues in `bind_expressions`:

1. `MorphTargetBind.weight` is parsed but ignored (should be multiplied)
2. Direct assignment (`weights[i] = weight`) instead of additive accumulation (VRM spec requires reset + sum)
3. Override system (`overrideBlink/LookAt/Mouth`) is completely unimplemented despite fields being parsed
4. `isBinary` threshold behavior is unimplemented

Team investigation (bug-investigator, bevy-researcher, vrm-spec-researcher, reviewer) confirmed these findings against UniVRM and three-vrm reference implementations.

## Decision

- Remove `ModifyExpressions` entirely
- Fix `bind_expressions` to be VRM 1.0 spec-compliant
- Implement the full override algorithm

## VRM 1.0 Override Algorithm

Verified against UniVRM (`DefaultExpressionValidator.cs`) and three-vrm (`VRMExpressionManager.ts`).

### Expression Categories

| Category | Expressions |
|----------|------------|
| Mouth | aa, ih, ou, ee, oh |
| Blink | blink, blinkLeft, blinkRight |
| LookAt | lookUp, lookDown, lookLeft, lookRight |
| Other | happy, angry, sad, relaxed, surprised, neutral, custom |

### Override Types

- `"none"` — no effect
- `"block"` — if source weight > 0, contribute 1.0 to override rate
- `"blend"` — contribute source weight directly to override rate

### Algorithm (3-pass)

```
Pass 1: Collect weights + accumulate override rates
  for each expression:
    outputWeight = isBinary ? (raw > 0.5 ? 1.0 : 0.0) : clamp(raw, 0, 1)
    mouthRate  += overrideMouth.rate(outputWeight)
    blinkRate  += overrideBlink.rate(outputWeight)
    lookAtRate += overrideLookAt.rate(outputWeight)

Pass 2: Compute multipliers
  mouthMul  = 1.0 - clamp(mouthRate, 0, 1)
  blinkMul  = 1.0 - clamp(blinkRate, 0, 1)
  lookAtMul = 1.0 - clamp(lookAtRate, 0, 1)

Pass 3: Reset morph weights to 0, then accumulate
  for each expression:
    multiplier = category match { Mouth => mouthMul, Blink => blinkMul, LookAt => lookAtMul, Other => 1.0 }
    finalWeight = if isBinary && multiplier < 1.0 { 0.0 } else { outputWeight * multiplier }
    for each bind:
      morphWeights[bind.index] += finalWeight * bind.weight
```

## Data Model Changes

### New Types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum ExpressionCategory { Mouth, Blink, LookAt, Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ExpressionOverrideType { None, Block, Blend }

#[derive(Component, Reflect)]
pub(crate) struct ExpressionOverrideSettings {
    pub override_blink: ExpressionOverrideType,
    pub override_look_at: ExpressionOverrideType,
    pub override_mouth: ExpressionOverrideType,
}

#[derive(Component, Reflect)]
pub(crate) struct ExpressionCategoryTag(pub ExpressionCategory);

#[derive(Component, Reflect)]
pub(crate) struct BinaryExpression;

pub(crate) struct ExpressionMetadata {
    pub nodes: Vec<ExpressionNode>,
    pub category: ExpressionCategory,
    pub override_settings: ExpressionOverrideSettings,
    pub is_binary: bool,
}
```

### Modified Types

- `ExpressionNode`: add `weight: f32` field
- `BindExpressionNode`: add `weight: f32` field
- `VrmExpressionRegistry`: value type changes from `Vec<ExpressionNode>` to `ExpressionMetadata`

### Removed Types

- `ModifyExpressions` (struct + impl + observer)

## Initialization Changes

- `VrmExpressionRegistry::new()` extracts override settings, category, is_binary from `VrmPreset`
- `apply_initialize_expressions` inserts `ExpressionCategoryTag`, `ExpressionOverrideSettings`, `BinaryExpression` on expression entities
- `convert_to_node` passes through `bind.weight`

## Files Affected

| File | Change |
|------|--------|
| `src/vrm/expressions.rs` | Core rewrite: new types, init changes, bind_expressions rewrite, remove ModifyExpressions |
| `src/vrm/gltf/extensions/vrmc_vrm.rs` | No change (fields already parsed) |
| `src/vrm.rs` | Remove ModifyExpressions from prelude |
| `examples/expressions.rs` | Remove keys 5-8, simplify to SetExpressions only |
| `CHANGELOG.md` | Update entries |

## Test Plan

| Test | Validates |
|------|-----------|
| `test_bind_weight_applied` | bind.weight multiplication |
| `test_additive_accumulation` | Reset + sum for shared morph targets |
| `test_override_block` | overrideMouth="block" suppresses mouth expressions |
| `test_override_blend` | overrideMouth="blend" attenuates by weight |
| `test_is_binary` | Threshold at 0.5 |
| `test_is_binary_override_suppression` | Binary + override = full suppression |
| `test_set_expressions_replaces_previous` | Existing (maintained) |
| `test_clear_expressions` | Existing (maintained) |

## References

- [VRM Expression Spec](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/expressions.md)
- [VRM Update Order](https://vrm.dev/api/api_update/)
- [UniVRM DefaultExpressionValidator.cs](https://github.com/vrm-c/UniVRM/blob/59776bf2bc9b131b6b0877777142d2a9402b3563/Assets/VRM10/Runtime/Components/Expression/DefaultExpressionValidator.cs)
- [three-vrm VRMExpressionManager.ts](https://github.com/pixiv/three-vrm/blob/dev/packages/three-vrm-core/src/expressions/VRMExpressionManager.ts)
