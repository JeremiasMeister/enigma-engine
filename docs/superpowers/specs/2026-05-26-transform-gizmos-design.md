# Transform Gizmos — Design

**Status:** approved
**Date:** 2026-05-26
**Scope:** Editor-side translate / rotate / scale gizmos drawn in egui screen-space on top of the 3D viewport, plus a small viewport toolbar that exposes modes and toggles.

## Goal

Let the user move, rotate, and scale the currently selected scene entity directly in the viewport using mouse-dragged handles, with hotkeys (`W`/`E`/`R`/`Q`) and toolbar buttons to switch modes.

## Non-goals

- No undo/redo. The editor has none today; this work doesn't introduce it.
- No multi-select gizmo. Selection is a single-target enum today.
- No icon-based toolbar. Text labels are fine for v1.
- No real 3D handles rendered through `enigma-3d`. All gizmo geometry is drawn with the egui painter.
- No keyboard shortcut for the world/local or snap toggle yet (toolbar buttons only).
- No gimbal-lock-proof rotation storage. We accept Euler roundtrip limitations near ±90° pitch.

## What already exists (no work needed)

- `EguiGlium` paints after the 3D scene render, so anything drawn with `egui::Painter` is naturally on top (`enigma-3d/src/lib.rs:1432–1437`).
- `editor::panels::viewport::draw` owns the viewport rect, camera input, and click-to-select via the existing `unproject` helper and `enigma_3d::collision_world::RayCast`.
- `editor::state::Selection` enum already exists with variants `SceneObject(Uuid)`, `Light(usize)`, `ParticleInstance(Uuid)`, `None`.
- `Transform` on `Object` is `{ position: Vec3, rotation: Vec3 (Euler radians), scale: Vec3 }`. Lights expose `position: [f32; 3]`. Particle instances expose `position: [f32; 3]`.
- `editor.dirty` flag pattern is used by all inspector edits to mark project state changed.
- The existing F-to-frame hotkey demonstrates the "pointer-in-rect + no RMB" guard for viewport hotkeys.

## Approach

DIY in egui screen-space — projection-and-painter, no shader changes, no engine changes.

Rejected alternatives:
- **Drop-in crate** (`transform-gizmo`, `egui-gizmo`): most current versions require egui ≥ 0.25; the editor is on 0.23.
- **3D handles inside `enigma-3d`**: requires an always-on-top render pass, constant-screen-size scaling, scene-side picking — out of scope for an editor-only feature.

## Architecture

New module `src/editor/gizmo/`:

- `mod.rs` — public surface, `GizmoState`, `GizmoMode`, `Space`, the `Drag` enum, the `handle_input` and `draw` entry points, and the `gizmo_target` accessor that maps the active `Selection` to a `GizmoTarget`.
- `math.rs` — pure helpers, no egui or app_state dependency, fully unit-testable.
- `translate.rs`, `rotate.rs`, `scale.rs` — one file per mode, each owning that mode's draw + hit-test + drag-update logic.
- `toolbar.rs` — the viewport-overlay toolbar.

### State

Added to `editor::state::EditorState`:

```rust
pub gizmo: GizmoState,
```

```rust
pub struct GizmoState {
    pub mode: GizmoMode,         // None | Translate | Rotate | Scale
    pub space: Space,            // World | Local
    pub snap_enabled: bool,
    pub drag: Option<Drag>,
    pub hovered_handle: Option<Handle>, // for highlight rendering
    pub consumed_click_this_frame: bool, // suppresses the viewport pick on gizmo drag end
}

pub enum GizmoMode { None, Translate, Rotate, Scale }
pub enum Space { World, Local }
pub enum Axis { X, Y, Z }
pub enum Handle { Axis(Axis), Center } // Center used by scale's uniform handle

pub enum Drag {
    Translate { axis: Axis, start_pos: Vector3<f32>, start_on_axis: Vector3<f32> },
    Rotate    { axis: Axis, start_quat: UnitQuaternion<f32>, start_dir: Vector3<f32> },
    Scale     { handle: Handle, start_scale: Vector3<f32>, start_pivot_screen: Pos2, start_cursor: Pos2, start_distance: f32 },
}
```

Defaults: `mode = None`, `space = World`, `snap_enabled = false`, `drag = None`, `hovered_handle = None`, `consumed_click_this_frame = false`.

### Selection wiring

```rust
enum GizmoTarget<'a> {
    Full(&'a mut Transform),            // SceneObject — translate / rotate / scale
    PositionOnly(&'a mut [f32; 3]),     // Light / ParticleInstance — translate only
}

fn gizmo_target<'a>(selection: &Selection, app_state: &'a mut AppState) -> Option<GizmoTarget<'a>>
```

Render rules:

- `Selection::None` → render nothing.
- `GizmoTarget::PositionOnly` with `mode = Rotate | Scale` → render the translate gizmo (silent downgrade). Toolbar continues to show the user's selected mode; only the gizmo geometry adapts.
- `GizmoTarget::Full` → render whichever mode is active.

### Drag state machine

Per frame, in order:

1. **`drag.is_some()`** → call the active mode's drag-update; on mouse-up, mutate complete, set `editor.dirty = true`, clear `drag`.
2. **`drag.is_none()` and pointer over a handle** → set `hovered_handle`; on mouse-down, snapshot start state and enter drag.
3. **Otherwise** → clear `hovered_handle`, do nothing.

Two interaction guards:

- **Gizmo input wins over viewport pick.** When the gizmo enters drag on mouse-down, `viewport::draw` must skip its existing click-to-select. Implementation: `viewport::draw`'s primary-released branch already checks `editor.drag`; add `gizmo.drag.is_some()` to that guard.
- **RMB held suppresses gizmo input.** Camera fly mode is exclusive with gizmo interaction; the camera owns the RMB drag.

## Drawing — constant screen size

Every frame, for the active gizmo:

```
distance = (object_pos - camera_pos).norm()
gizmo_world_size = distance * (camera.fov / 2.0).tan() * SCREEN_FRACTION
```

`SCREEN_FRACTION` ≈ 0.15 (handles span ~15% of the viewport vertical extent). All handle lengths, ring radii, and cube sizes scale off this single factor.

## Translate

Geometry: three world-space line segments from `pivot` to `pivot + axis_dir * gizmo_world_size`, projected to 2D and stroked. Colors: X = red, Y = green, Z = blue. Hovered axis renders brighter (yellow when hovered, white when actively dragged).

Hit-test: 2D distance from cursor to each axis segment; ~8 px threshold; closest axis wins on overlap.

Drag math (closest point on axis line):

1. On mouse-down — unproject cursor → ray; find closest point on axis line to ray → `start_on_axis`. Store `start_pos = transform.position`.
2. On mouse-move — repeat, get `current_on_axis`. `delta_along_axis = (current_on_axis - start_on_axis).dot(axis_dir)`.
3. Apply `transform.position = start_pos + axis_dir * delta_along_axis`.

`axis_dir`:
- World space: `Vector3::x()` / `y()` / `z()`.
- Local space: `transform_quat * axis_basis` (using the start-of-drag rotation).

Snap: when active, `delta_along_axis = (delta_along_axis / 1.0).round() * 1.0`.

## Rotate

Geometry: three colored rings around `pivot`, each in the plane perpendicular to its axis, radius = `gizmo_world_size`. Each ring sampled at 64 points; consecutive samples projected to 2D and stroked. v1 draws the full ring (no back-half culling).

Hit-test: 2D distance from cursor to each ring's 64 segments; ~8 px threshold.

Drag math (ray-into-plane):

1. On mouse-down — define plane through `pivot` with normal = `axis_dir`. Unproject cursor → ray. Intersect ray with plane → world point `p0`. Set `start_dir = (p0 - pivot).normalize()`. Snapshot `start_quat = UnitQuaternion::from_euler_angles(rot.x, rot.y, rot.z)`.
2. On mouse-move — repeat to get `current_dir`. Compute signed angle around `axis_dir`:
   ```
   cos_a = start_dir.dot(current_dir).clamp(-1.0, 1.0)
   sin_a = axis_dir.dot(start_dir.cross(&current_dir))
   delta_angle = sin_a.atan2(cos_a)
   ```
3. Apply `new_quat = UnitQuaternion::from_axis_angle(&axis_dir.into(), delta_angle) * start_quat`. Convert: `let (rx, ry, rz) = new_quat.euler_angles(); transform.rotation = Vector3::new(rx, ry, rz);`.

`axis_dir`:
- World space: `Vector3::x()` / `y()` / `z()`.
- Local space: `start_quat * axis_basis`.

Snap: when active, `delta_angle = (delta_angle / (PI/12.0)).round() * (PI/12.0)`.

Known limitation: Euler roundtrip near ±90° pitch can flip stored components. The rotation displayed and applied stays correct during the drag (we use quaternions internally); only the persisted form may differ from what the user typed. Acceptable for v1.

## Scale

Geometry: three colored axis segments like translate, but ending in small cubes (~10% of `gizmo_world_size` per side) instead of plain endpoints. Plus a small white cube at the origin (~8% per side) for uniform scale. Axes drawn dimmer than translate so the two modes read differently at a glance.

Hit-test: each axis-tip cube and the center cube projected to 2D, point-in-rect test on their bounding rectangles. Axis line bodies are not hittable.

Drag math (screen-space distance ratio):

1. On mouse-down — project pivot to screen → `start_pivot_screen`. Capture `start_scale = transform.scale`, `start_cursor`, `start_distance = (start_cursor - start_pivot_screen).length()`.
2. On mouse-move — `factor = (cursor - start_pivot_screen).length() / start_distance`.
3. Apply:
   - X handle: `transform.scale.x = start_scale.x * factor`.
   - Y handle: `transform.scale.y = start_scale.y * factor`.
   - Z handle: `transform.scale.z = start_scale.z * factor`.
   - Center handle: all three components multiplied by `factor`.

Guard: if `start_distance < 1.0` (cursor started essentially on top of the pivot), abort the drag rather than dividing by a near-zero number.

Scale ignores the world/local toggle. Scale is always object-local by convention.

Snap: when active, `factor = ((factor / 0.1).round() * 0.1).max(0.1)`. The `.max(0.1)` floor prevents snapping the scale to zero.

## Toolbar

A small `egui::Area` anchored to the viewport rect's top-left with an 8 px inset margin. Single horizontal row of buttons:

| Label | Hotkey | Action |
|-------|--------|--------|
| Select | Q | `mode = None` |
| Move | W | `mode = Translate` |
| Rotate | E | `mode = Rotate` |
| Scale | R | `mode = Scale` |
| World/Local | — | toggle `space` |
| Snap | — | toggle `snap_enabled` |

Active mode button is highlighted with egui's selected-fill. Toggles render with their on/off state visible.

Hotkey guard: only fire when pointer is in the viewport rect and RMB is not held — same condition as the existing F-to-frame hotkey in `viewport.rs`.

Snap-during-drag rule: effective snap state = `snap_enabled XOR Ctrl-held`. So a held Ctrl temporarily inverts the toolbar's setting.

## Wiring into the viewport

`editor::panels::viewport::draw` runs these in order each frame:

1. (existing) Update camera.
2. (new) `gizmo::handle_input(ctx, rect, app_state)` — hotkey detection, hit-test, drag begin/update/end. Sets `consumed_click_this_frame = true` on any frame that begins or ends a drag.
3. (existing) Primary-released click-to-select — must not fire when the release belongs to the gizmo. The guard becomes:

   ```rust
   let consumed = app_state.get_state_data_value_mut::<EditorRoot>("editor")
       .map(|r| std::mem::replace(&mut r.editor.gizmo.consumed_click_this_frame, false))
       .unwrap_or(false);
   if drag_active || consumed { return; }
   ```

4. (new) `gizmo::draw(ui, rect, app_state)` — paints gizmo handles using `ui.painter_at(rect)`.
5. (new) `gizmo::toolbar::draw(ctx, rect, app_state)` — paints the toolbar in its own `egui::Area`.

## Math helpers (`math.rs`)

Pure, no `AppState` / no `egui` types beyond `Pos2` for screen coords:

```rust
pub fn world_to_screen(camera: &Camera, rect: Rect, world: Vector3<f32>) -> Option<Pos2>;
// None if behind camera

pub fn unproject(camera: &Camera, screen_pos: Pos2, rect: Rect) -> (Vector3<f32>, Vector3<f32>);
// Returns (origin, dir); same logic as the existing helper in viewport.rs — we move it here

pub fn closest_point_on_line_to_ray(
    line_origin: Vector3<f32>, line_dir: Vector3<f32>,
    ray_origin: Vector3<f32>, ray_dir: Vector3<f32>,
) -> Vector3<f32>;

pub fn ray_plane_intersect(
    ray_origin: Vector3<f32>, ray_dir: Vector3<f32>,
    plane_point: Vector3<f32>, plane_normal: Vector3<f32>,
) -> Option<Vector3<f32>>;
// None if ray is parallel to plane

pub fn distance_point_to_segment_2d(p: Pos2, a: Pos2, b: Pos2) -> f32;

pub fn snap(value: f32, step: f32) -> f32;
```

The existing `unproject` in `viewport.rs` is moved into `math.rs` and re-used by both viewport pick and gizmo.

## Testing

**Unit tests in `math.rs`** (a fresh `#[cfg(test)] mod tests` block at the bottom):

- `world_to_screen` — known camera looking down -Z, point at origin → expected screen center; point behind camera → `None`.
- `closest_point_on_line_to_ray` — perpendicular ray + axis at origin → exact intersection.
- `ray_plane_intersect` — known ray + plane → known point; parallel ray → `None`.
- `distance_point_to_segment_2d` — three cases: perpendicular foot inside segment, before A endpoint, after B endpoint.
- `snap` — round to step (translate 1.0, rotate π/12, scale 0.1).

No unit tests for the per-mode files. They compose math.rs helpers and mutate state; the meaningful coverage either lives in math.rs already or can only come from manual viewport use.

**Manual viewport checks** (the spec author runs these after implementation):

1. Select a cube. Press W → translate gizmo appears in last-used mode.
2. Drag X axis — cube moves along world X. Drag Y, then Z — same in each direction.
3. Press E. Drag a rotate ring — cube rotates around that axis. Switch to Local, rotate Y by 45°, then translate in Local — moves along the cube's tilted X.
4. Press R. Drag an axis cube — only that axis scales. Drag the center cube — uniform scale.
5. Toggle Snap. Drag X — value snaps to integer units. Hold Ctrl during the drag — snap inverts.
6. Press Q. Gizmo disappears. Click-to-select still works.
7. Select a directional light. Press W → translate gizmo on the light. Press E → still the translate gizmo (silent downgrade); toolbar shows Rotate active.
8. Hold RMB and press W. No mode change (camera fly takes precedence). Press F. Camera frames selection (existing hotkey unchanged).
9. Open a tilted object's inspector while a gizmo is active. Drag in the viewport — the inspector's numeric fields update live.

## Files touched

New:
- `src/editor/gizmo/mod.rs`
- `src/editor/gizmo/math.rs`
- `src/editor/gizmo/translate.rs`
- `src/editor/gizmo/rotate.rs`
- `src/editor/gizmo/scale.rs`
- `src/editor/gizmo/toolbar.rs`

Modified:
- `src/editor/mod.rs` — add `pub mod gizmo;`.
- `src/editor/state.rs` — add `gizmo: GizmoState` field to `EditorState`, plus the `GizmoState` / `GizmoMode` / `Space` / `Axis` / `Handle` / `Drag` types.
- `src/editor/panels/viewport.rs` — call the four new hooks (input + draw + toolbar + guard); move `unproject` into `gizmo::math`.

## Open questions (none blocking)

- Should the gizmo render when a particle-instance position is dragged through the inspector while a drag is mid-flight? Not blocking; default behavior is fine (the gizmo redraws each frame from the live transform).
- Icon font for the toolbar — deferred.
