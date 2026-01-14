# bevy_vrm1

[![Crates.io](https://img.shields.io/crates/v/bevy_vrm1.svg)](https://crates.io/crates/bevy_vrm1)
[![Docs](https://docs.rs/bevy_vrm1/badge.svg)](https://docs.rs/bevy_vrm1/latest/bevy_vrm1/)

> [!CAUTION]
> This crate is in an early stage of development and may undergo breaking changes.

> [!NOTE]
> This crate only supports VRM 1.0.

This crate allows you to use [VRM1.0](https://vrm.dev/en/vrm/vrm_about/) and [VRMA](https://vrm.dev/en/vrma/).

## Usage

| Name            | currently supported |
|-----------------|---------------------|
| Spring Bone     | ✅                   |
| Look At         | ✅                   |
| Animation(vrma) | ✅                   |
| Node Constraint | ✅                   |
| First Person    | ❌                   |

### Spring Bone

![SpringBone](./docs/spring_bone.gif)

This is a feature for expressing the sway of a character's hair and other parts.

- [spring bone specification(en)](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_springBone-1.0/README.md)
- [spring bone specification(ja)](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_springBone-1.0/README.ja.md)

#### examples

- [spring_bone.rs](./examples/spring_bone.rs)

### Look At

![LookAt](./docs/look_at.gif)

- [look at specification(en)](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/lookAt.md)
- [look at specification(ja)](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm-1.0/lookAt.ja.md)

LookAt is a component for animating the line of sight into a VRM model.
You can use the `LookAt` component to track a specific target or the mouse cursor.

#### examples

- [look_at_cursor.rs](./examples/look_at_cursor.rs)
- [look_at_target.rs](./examples/look_at_target.rs)

### Animation(vrma)

![VRMA](./docs/vrma.gif)

You can play animations using VRMA.

- [vrma specification(en)](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm_animation-1.0/README.md)
- [vrma specification(ja)](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_vrm_animation-1.0/README.ja.md)

### examples

- [vrma.rs](./examples/vrma.rs)

### Node Constraint

Node Constraint is a feature for constraining node transformations in real-time, primarily designed for Humanoid bones. This library supports all three constraint types defined in the VRMC_node_constraint-1.0 specification:

- [node constraint specification(en)](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_node_constraint-1.0/README.md)
- [node constraint specification(ja)](https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_node_constraint-1.0/README.ja.md)

#### Constraint Types

**Rotation Constraint**
- Transfers the entire local rotation from a source node to destination nodes
- Typical use case: Sub-arms and auxiliary bones
- Supports weight parameter for interpolation (0.0 - 1.0)

**Roll Constraint**
- Transfers rotation around a specific axis (X, Y, or Z)
- Typical use case: Twist bones for arms and legs
- Supports weight parameter and configurable roll axis

**Aim Constraint**
- Rotates a node to face a target node
- Typical use case: Clothing sleeves and accessories
- Supports weight parameter and configurable aim axis (PositiveX, NegativeX, PositiveY, NegativeY, PositiveZ, NegativeZ)

All constraint types use spherical linear interpolation (slerp) based on the weight parameter to blend between the rest rotation and the constrained rotation.

### Features

| Feature | Description                                         | default |
|---------|-----------------------------------------------------|---------|
| serde   | derive `Serialize` and `Deserialize` for components | no      |
| log     | enable log for debugging                            | no      |

## Versions

| bevy_vrm1 | bevy |
|-----------|------|
| 0.5.0 ~   | 0.18 |
| 0.4.0 ~   | 0.17 |
| 0.1.0 ~   | 0.16 |

## Credits

Using [bevy_game_template](https://github.com/NiklasEi/bevy_game_template) to CI.

### Sample Models

- **AliciaSolid** by **© DWANGO Co., Ltd.**