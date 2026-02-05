# Multi-Window Cursor Tracking for LookAt

**Date:** 2026-02-05
**Status:** Approved
**Files:** `src/vrm/look_at.rs`

## Problem

When using `LookAt::Cursor` with multiple windows across monitors, the gaze flips direction when the cursor crosses window boundaries. This happens because each window normalizes cursor position to [-1, 1] independently.

Example:
- Window A on left monitor, Window B on right monitor
- Cursor at right edge of Window A → normalized x ≈ +1.0
- Cursor enters Window B from left → normalized x suddenly ≈ -1.0
- Gaze flips direction instead of continuing smoothly

## Solution

Calculate cursor position in global screen space, then normalize relative to the bounding box of all app windows. This gives continuous coordinates across all windows.

### Fallback Behavior

When windows don't have explicit positions (`WindowPosition::At`), fall back to single-window behavior (current implementation).

## Calculation

### Step 1: Collect Window Information

For each window with explicit position, gather:
- Screen position (`WindowPosition::At(x, y)`)
- Window size
- Cursor position (if present)

### Step 2: Calculate Bounding Box

```
Window A: position (0, 0), size (1920, 1080)
Window B: position (1920, 0), size (1920, 1080)

Bounding box:
  min: (0, 0)
  max: (3840, 1080)
  center: (1920, 540)
  half_size: (1920, 540)
```

### Step 3: Calculate Global Cursor Position

```
Cursor in Window B at local (100, 540)
Global cursor = window_position + local = (1920 + 100, 0 + 540) = (2020, 540)
```

### Step 4: Normalize Relative to Bounding Box Center

```
normalized = (global_cursor - center) / half_size
          = ((2020, 540) - (1920, 540)) / (1920, 540)
          = (100, 0) / (1920, 540)
          = (0.052, 0.0)
```

Cursor just right of center → small positive x → slight rightward gaze.

## Implementation

### New Function: `find_cursor_position_normalized_multi_window`

```rust
fn find_cursor_position_normalized_multi_window(
    windows: &Query<(Entity, &Window, Has<PrimaryWindow>)>,
) -> Option<Vec2> {
    // Collect windows with explicit positions
    let mut window_bounds: Vec<(Vec2, Vec2)> = Vec::new(); // (position, size)
    let mut cursor_global: Option<Vec2> = None;

    for (_, window, _) in windows.iter() {
        let WindowPosition::At(pos) = window.position else {
            continue; // Skip windows without explicit position
        };
        let pos = Vec2::new(pos.x as f32, pos.y as f32);
        let size = window.size();
        window_bounds.push((pos, size));

        if let Some(cursor_local) = window.cursor_position() {
            cursor_global = Some(pos + cursor_local);
        }
    }

    // Fallback: not enough positioned windows
    if window_bounds.len() < 2 {
        return None;
    }

    let cursor_global = cursor_global?;

    // Calculate bounding box
    let min = window_bounds.iter()
        .map(|(pos, _)| *pos)
        .reduce(|a, b| a.min(b))?;
    let max = window_bounds.iter()
        .map(|(pos, size)| *pos + *size)
        .reduce(|a, b| a.max(b))?;

    let center = (min + max) / 2.0;
    let half_size = (max - min) / 2.0;

    // Normalize relative to center
    Some((cursor_global - center) / half_size)
}
```

### Modified `find_cursor_position_normalized`

```rust
fn find_cursor_position_normalized(
    windows: &Query<(Entity, &Window, Has<PrimaryWindow>)>,
) -> Option<Vec2> {
    // Try multi-window first
    if let Some(normalized) = find_cursor_position_normalized_multi_window(windows) {
        return Some(normalized);
    }

    // Fallback to single-window behavior
    windows.iter().find_map(|(_, window, _)| {
        let cursor = window.cursor_position()?;
        let size = window.size();
        Some(Vec2::new(
            (cursor.x / size.x - 0.5) * 2.0,
            (cursor.y / size.y - 0.5) * 2.0,
        ))
    })
}
```

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Single window | Falls back to single-window normalization |
| Windows without `WindowPosition::At` | Ignored; falls back if < 2 positioned windows |
| Cursor outside all windows | Returns `None`, gaze unchanged |
| Windows moved at runtime | Recalculated each frame (uses current positions) |

## What Changes

- `find_cursor_position_normalized` tries multi-window calculation first
- New `find_cursor_position_normalized_multi_window` function

## What Stays the Same

- `calc_yaw_pitch_from_screen` (unchanged)
- `LookAt::Target` mode (unchanged)
- All other LookAt behavior
- `LookAtProperties` and range maps
