# Multi-Window Cursor Tracking Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable smooth gaze tracking across multiple windows by using global screen-space coordinates.

**Architecture:** Add `find_cursor_position_normalized_multi_window` function that calculates cursor position relative to the bounding box of all positioned windows. Falls back to single-window behavior when < 2 windows have explicit positions.

**Tech Stack:** Bevy 0.18, `Window::position` (`WindowPosition::At`), `Window::cursor_position()`

---

### Task 1: Add Multi-Window Cursor Position Function

**Files:**
- Modify: `src/vrm/look_at.rs:168-179` (after current `find_cursor_position_normalized`)

**Step 1: Add the import for WindowPosition**

Add `WindowPosition` to the imports at line 8.

```rust
use bevy::window::{PrimaryWindow, WindowPosition};
```

**Step 2: Add `find_cursor_position_normalized_multi_window` function**

Add this function after the existing `find_cursor_position_normalized` function (after line 179):

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
    let min = window_bounds
        .iter()
        .map(|(pos, _)| *pos)
        .reduce(|a, b| a.min(b))?;
    let max = window_bounds
        .iter()
        .map(|(pos, size)| *pos + *size)
        .reduce(|a, b| a.max(b))?;

    let center = (min + max) / 2.0;
    let half_size = (max - min) / 2.0;

    // Normalize relative to center
    Some((cursor_global - center) / half_size)
}
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors (function is unused for now)

**Step 4: Commit**

```bash
git add src/vrm/look_at.rs
git commit -m "feat(look_at): add multi-window cursor position calculation"
```

---

### Task 2: Integrate Multi-Window Function

**Files:**
- Modify: `src/vrm/look_at.rs:168-179` (replace `find_cursor_position_normalized` body)

**Step 1: Update `find_cursor_position_normalized` to try multi-window first**

Replace the function body at lines 168-179:

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

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/vrm/look_at.rs
git commit -m "feat(look_at): integrate multi-window cursor tracking"
```

---

### Task 3: Manual Testing

**Files:**
- Test with: `examples/look_at_cursor.rs` (may need modification for multi-window)

**Step 1: Create a multi-window test setup**

Modify the example or create a test that spawns two windows with explicit positions:

```rust
// Example window setup for testing
commands.spawn(Window {
    title: "Left Window".to_string(),
    position: WindowPosition::At(IVec2::new(0, 0)),
    resolution: WindowResolution::new(800.0, 600.0),
    ..default()
});

commands.spawn(Window {
    title: "Right Window".to_string(),
    position: WindowPosition::At(IVec2::new(800, 0)),
    resolution: WindowResolution::new(800.0, 600.0),
    ..default()
});
```

**Step 2: Run and verify behavior**

Run: `cargo run --example look_at_cursor`

Expected behavior:
- Cursor in left window, left side → gaze looks left
- Cursor crosses to right window → gaze continues smoothly rightward (no flip)
- Cursor in right window, right side → gaze looks right

**Step 3: Verify single-window fallback**

Remove explicit positions from windows and verify original behavior works.

**Step 4: Final commit with any adjustments**

```bash
git add -A
git commit -m "test(look_at): verify multi-window cursor tracking"
```

---

## Summary

| Task | Description | Estimated Steps |
|------|-------------|-----------------|
| 1 | Add multi-window function | 4 |
| 2 | Integrate into existing function | 3 |
| 3 | Manual testing | 4 |

**Total:** 3 tasks, ~11 steps
