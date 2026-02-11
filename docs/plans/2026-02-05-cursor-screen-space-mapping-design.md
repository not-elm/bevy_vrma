# Cursor Screen-Space Mapping for LookAt

**Date:** 2026-02-05
**Status:** Approved
**Files:** `src/vrm/look_at.rs`

## Problem

The current `find_cursor_target` function uses world-space ray casting to determine where the VRM model should look when tracking the cursor. This approach produces incorrect results with `OrthographicProjection` cameras because the world-space displacement from cursor movement is very small (only ~2-3 world units across the entire screen), resulting in minimal gaze angle changes.

The expected behavior is: cursor at screen edge = maximum gaze angle, regardless of camera projection settings.

## Solution

Replace world-space ray casting with direct screen-space to angle mapping for cursor mode.

### Current Flow
```
cursor position → viewport_to_world → plane intersection → world Vec3 → calc_yaw_pitch → angles
```

### New Flow
```
cursor position → normalize to [-1, 1] → apply range maps → angles
```

## Implementation

### 1. New Function: `find_cursor_position_normalized`

Replaces `find_cursor_target` for cursor mode. Returns normalized screen coordinates.

```rust
fn find_cursor_position_normalized(
    windows: &Query<(Entity, &Window, Has<PrimaryWindow>)>,
) -> Option<Vec2> {
    windows.iter().find_map(|(_, window, _)| {
        let cursor = window.cursor_position()?;
        let size = window.size();
        Some(Vec2::new(
            (cursor.x / size.x - 0.5) * 2.0,  // [-1, 1], left to right
            (cursor.y / size.y - 0.5) * 2.0,  // [-1, 1], top to bottom
        ))
    })
}
```

### 2. New Function: `calc_yaw_pitch_from_screen`

Maps normalized screen position directly to yaw/pitch angles using `LookAtProperties` range maps.

```rust
fn calc_yaw_pitch_from_screen(normalized: Vec2, properties: &LookAtProperties) -> (f32, f32) {
    // Yaw (horizontal)
    let max_yaw = if normalized.x > 0.0 {
        properties.range_map_horizontal_outer.input_max_value
    } else {
        properties.range_map_horizontal_inner.input_max_value
    };
    let yaw = normalized.x * max_yaw;

    // Pitch (vertical)
    let max_pitch = if normalized.y > 0.0 {
        properties.range_map_vertical_down.input_max_value
    } else {
        properties.range_map_vertical_up.input_max_value
    };
    let pitch = normalized.y * max_pitch;

    (yaw, pitch)
}
```

### 3. Modified System Flow

```rust
fn look_at_system(...) {
    let (yaw, pitch): Option<(f32, f32)> = match mode {
        LookAtMode::Cursor => {
            let normalized = find_cursor_position_normalized(...)?;
            Some(calc_yaw_pitch_from_screen(normalized, properties))
        }
        LookAtMode::Target(entity) => {
            let target = get_entity_position(...)?;
            Some(calc_yaw_pitch(look_at_space, target))
        }
    };

    if let Some((yaw, pitch)) = (yaw, pitch) {
        apply_to_bones(yaw, pitch, ...);
    }
}
```

## What Changes

- `find_cursor_target` → `find_cursor_position_normalized` (returns `Option<Vec2>`)
- New `calc_yaw_pitch_from_screen` function for cursor mode
- Cursor mode bypasses world-space ray casting entirely
- Target mode remains unchanged

## What Stays the Same

- `LookAtProperties` and range maps
- `apply_*_eye_bone` functions
- `calc_yaw_pitch` (still used for Target mode)
- All other LookAt behavior

## Benefits

- Cursor at screen edge = max gaze angle (as configured in range maps)
- Works identically regardless of camera projection type (perspective or orthographic)
- Simpler code path for cursor mode
- Multi-monitor works correctly (each window tracks its own cursor)
