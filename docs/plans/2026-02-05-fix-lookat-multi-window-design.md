# Fix LookAt Cursor Tracking in Multi-Window

## Problem

When a second window is opened, the VRM stops tracking the cursor entirely.
`Window::cursor_position()` returns `None` when the cursor is outside that window's bounds.
The current code always resolves to the primary window's camera, so when the cursor moves to a secondary window, the system early-returns without updating eye rotations.

## Solution

### API Change

Simplify `LookAt::Cursor` from:

```rust
pub enum LookAt {
    Cursor { camera: Option<Entity> },
    Target(Entity),
}
```

to:

```rust
pub enum LookAt {
    Cursor,
    Target(Entity),
}
```

The `camera` field is removed. The system automatically discovers which window has the cursor and which `Camera3d` renders to it.

### Cursor Resolution Algorithm (window-first)

1. Iterate all windows, find the one where `cursor_position()` returns `Some(pos)` (short-circuits, usually 1-2 windows)
2. For that window, find the `Camera3d` whose `RenderTarget` matches:
   - `WindowRef::Primary` -> window has `PrimaryWindow` marker
   - `WindowRef::Entity(e)` -> `e == window_entity`
3. Ray cast from that camera through the cursor position
4. Intersect with a plane halfway between camera and VRM head
5. Return the intersection point as the gaze target

### Query Changes

- Camera query: `Query<(Entity, &Camera, &RenderTarget, &GlobalTransform), With<Camera3d>>`
- Window query: `Query<(Entity, &Window, Has<PrimaryWindow>)>`

### Files Changed

- `src/vrm/look_at.rs`: Simplify enum, rewrite cursor resolution, update docstring
- `examples/look_at_cursor.rs`: Remove camera entity capture, use `LookAt::Cursor`
