## Unreleased

- change visibility of Spring structs to public


## v0.5.1 

[Release Notes](https://github.com/not-elm/bevy_vrm1/releases/tag/v0.5.1)

### Bug Fixes

- Fixed MToon outline rendering pipeline after Bevy 0.17+ migration
    - Outline pass now correctly uses MToonMaterial vertex/fragment shaders instead of default PBR
    - Added MToon material bind group layout at index 3 with MATERIAL_BIND_GROUP=3

## v0.5.0

- Migrated Bevy dependency from v0.17 to v0.18.

## v0.4.0

### Breaking Changes
- Migrated Bevy dependency from v0.16 to v0.17.
- Outline rendering now uses a strict depth compare to avoid full-surface outline fill on thin meshes.

### Bug Fixes

- Fixed system execution order for VRM constraints and expressions to comply with VRM specification
    - Added manual transform propagation after constraints and expressions
    - Ensures `GlobalTransform` updates propagate correctly between systems
    - Fixes rendering and physics issues caused by stale transform data

## v0.3.0

[Release Notes](https://github.com/not-elm/bevy_vrm1/releases/tag/v0.3.0)

### Features

- Added support for Node Constraints (VRMC_node_constraint-1.0)
    - Rotation Constraint: Transfers entire local rotation from source to destination nodes
    - Roll Constraint: Transfers rotation around a specific axis (X, Y, or Z)
    - Aim Constraint: Rotates a node to face a target node
    - All constraint types support weight-based interpolation using spherical linear interpolation (slerp)

### Breaking Changes

- `AnimationTransitions` are now used internally;This enables smooth animation transitions.
    - Changed fields of `PlayVrma`
- added `log` feature flag to enable logging.
    - Error logs are now not output by default.
- The update timing for SpringBone and LookAt has been changed to `PostUpdate`.
- Rust edition has been changed to 2024.
- Renamed some of the methods defined on SystemParams in this crate.
    - Doesn't affect most users

### Bug Fixes

- Fixed collision detection for the SpringBone sphere collider.
- Fixed logic to determine redraw
- Fixed look at bone rotation
- Fixed `ColliderGroup::name` types from `String` to `Option<String>` to match the spec.

## v0.2.2

[Release Notes](https://github.com/not-elm/bevy_vrm1/releases/tag/v0.2.2)

### Bug Fixes

- Fixed SpringBone colliders.
- Changed the spring bone calculation to use the center space if a center node is set.
- Fixed an issue that caused a crash during MToon shader processing.
    - This occurred in Bevy v0.16.1 and later versions.

## v0.2.1

[Release Notes](https://github.com/not-elm/bevy_vrm1/releases/tag/v0.2.1)
I was going to add this in v0.2.0 but forgot.

### Improvements

- Added `VrmSystemSets` to define the system order of `Retarget`, `LookAt`, and `SpringBone`.
- Export several VRM(A) components that were not being exported correctly via `prelude` module.

## v0.2.0

[Release Notes](https://github.com/not-elm/bevy_vrm1/releases/tag/v0.2.0)

### Breaking Changes

- `MToonOutline` is no longer a component; it has become part of the `MToonMaterial` fields.
- `OutlineWidthMode` has been added as part of the field of `MToonOutline`.
    - Currently only supports `OutlineWidthMode::WorldCoordinates` and `OutlineWidthMode::None`, and if
      `screenCoordinates` is passed, the outline will not be rendered.
- Fixed the rendering order of the outline to match the spec.
    - refer
      to [here](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_materials_mtoon-1.0/README.md#rendering)
      for more details.
- Removed `reflect` feature flag, and `serde` has been added instead.
    - `Reflect` is now applied to most structs by default.

### Bug Fixes

- Fixed outline rendering

## v0.1.2

[Release Notes](https://github.com/not-elm/bevy_vrm1/releases/tag/v0.1.2)

### Bug Fixes

- Fixed so that retargeting bone works correctly between models with different initial poses.
- Fixed a bug that only one animation could be played.

## v0.1.1

[Release Notes](https://github.com/not-elm/bevy_vrm1/releases/tag/v0.1.1)

### Bug Fixes

- Fixed `VrmcMaterialsExtensitions::outline_width_factor` type from `f32` to `Option<f32>` to match the spec.
- Fixed shadow casting for directional lights.

### Features

- Supported multiple directional lights

## v0.1.0

First Release!
