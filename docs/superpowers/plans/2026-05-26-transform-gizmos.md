# Transform Gizmos Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add translate / rotate / scale gizmos for the currently selected scene entity, drawn in egui screen-space on top of the 3D viewport, plus a small viewport toolbar exposing modes and toggles.

**Architecture:** New module `src/editor/gizmo/` with one file per mode plus shared math + toolbar. Math is the only file with projection logic and the only file with tests; per-mode files compose math helpers and mutate state through `viewport.rs` hooks. `EguiGlium` paints after the 3D render, so screen-space drawing naturally lands on top.

**Tech Stack:** Rust 2021, `egui` 0.23, `nalgebra` 0.32, `enigma-3d` (editor's renderer), `glium` 0.33.

**Spec:** `docs/superpowers/specs/2026-05-26-transform-gizmos-design.md`

---

## File Structure

**New:**
- `src/editor/gizmo/mod.rs` — public surface, `GizmoState` types, the `Drag` enum, `handle_input`, `draw`, `gizmo_target` accessor.
- `src/editor/gizmo/math.rs` — pure math helpers + unit tests. The only file with projection logic.
- `src/editor/gizmo/translate.rs` — translate gizmo (draw + hit-test + drag update).
- `src/editor/gizmo/rotate.rs` — rotate gizmo.
- `src/editor/gizmo/scale.rs` — scale gizmo.
- `src/editor/gizmo/toolbar.rs` — viewport toolbar.

**Modified:**
- `src/editor/mod.rs` — register `pub mod gizmo;`.
- `src/editor/state.rs` — add `gizmo: GizmoState` field on `EditorState`.
- `src/editor/panels/viewport.rs` — call new hooks; delete the local `unproject` (now in `math.rs`).

---

## Task 1: Math helpers — `world_to_screen`, `unproject` (TDD)

**Files:**
- Create: `src/editor/gizmo/mod.rs`
- Create: `src/editor/gizmo/math.rs`
- Modify: `src/editor/mod.rs:4`

- [ ] **Step 1: Register the new module**

Edit `src/editor/mod.rs`, after line 4 (`pub mod inspector;`):

```rust
pub mod gizmo;
```

- [ ] **Step 2: Create the gizmo module surface**

Create `src/editor/gizmo/mod.rs`:

```rust
pub mod math;
```

- [ ] **Step 3: Write `math.rs` with failing tests**

Create `src/editor/gizmo/math.rs`:

```rust
use egui::{Pos2, Rect};
use enigma_3d::camera::Camera;
use nalgebra::Vector3;

/// Project a world-space point to screen-space coordinates inside `rect`.
/// Returns `None` if the point is behind the camera (view_z <= 0).
pub fn world_to_screen(camera: &Camera, rect: Rect, world: Vector3<f32>) -> Option<Pos2> {
    let cam_pos = Vector3::from(camera.get_position());
    let forward = Vector3::from(camera.calculate_direction_vector());
    let world_up = Vector3::new(0.0, 1.0, 0.0);
    let right = forward.cross(&world_up).normalize();
    let up = right.cross(&forward).normalize();

    let rel = world - cam_pos;
    let view_z = rel.dot(&forward);
    if view_z <= 0.0 {
        return None;
    }
    let view_x = rel.dot(&right);
    let view_y = rel.dot(&up);

    let aspect = camera.width / camera.height;
    let half_h = (camera.fov / 2.0).tan() * view_z;
    let half_w = half_h * aspect;

    let ndc_x = view_x / half_w;
    let ndc_y = view_y / half_h;

    let screen_x = rect.min.x + (ndc_x + 1.0) * 0.5 * rect.width();
    let screen_y = rect.min.y + (1.0 - ndc_y) * 0.5 * rect.height();
    Some(Pos2::new(screen_x, screen_y))
}

/// Convert a screen-space cursor position to a world-space ray.
/// Returns `(origin, direction)` with `direction` normalized.
pub fn unproject(camera: &Camera, screen_pos: Pos2, rect: Rect) -> (Vector3<f32>, Vector3<f32>) {
    let ndc_x = (screen_pos.x - rect.min.x) / rect.width() * 2.0 - 1.0;
    let ndc_y = -((screen_pos.y - rect.min.y) / rect.height() * 2.0 - 1.0);

    let aspect = camera.width / camera.height;
    let half_h = (camera.fov / 2.0).tan();
    let half_w = half_h * aspect;

    let forward = Vector3::from(camera.calculate_direction_vector());
    let world_up = Vector3::new(0.0, 1.0, 0.0);
    let right = forward.cross(&world_up).normalize();
    let up = right.cross(&forward).normalize();

    let dir = (forward + right * (ndc_x * half_w) + up * (ndc_y * half_h)).normalize();
    let origin = Vector3::from(camera.get_position());
    (origin, dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_camera() -> Camera {
        // Camera at origin looking down -Z is the default in enigma-3d.
        // (calculate_direction_vector with rotation = 0,0,0 → forward = -Z.)
        // To make the math match a "looking along +Z" intuition we instead
        // place the camera at (0,0,-5) looking at origin (forward = +Z).
        let mut cam = Camera::default();
        cam.set_position([0.0, 0.0, -5.0]);
        cam.set_rotation([0.0, 0.0, 0.0]);
        cam.width = 800.0;
        cam.height = 600.0;
        cam.fov = std::f32::consts::FRAC_PI_2; // 90°
        cam.near = 0.1;
        cam.far = 1000.0;
        cam.update_matrices();
        cam
    }

    #[test]
    fn world_to_screen_point_in_front_lands_inside_rect() {
        let cam = test_camera();
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::Vec2::new(800.0, 600.0));
        // Confirm the camera actually faces +Z by sampling its direction.
        let forward = Vector3::from(cam.calculate_direction_vector());
        // A point along the camera's forward direction should land near rect center.
        let target = Vector3::from(cam.get_position()) + forward * 5.0;
        let screen = world_to_screen(&cam, rect, target).expect("point in front");
        assert!((screen.x - 400.0).abs() < 1.0, "screen.x = {}", screen.x);
        assert!((screen.y - 300.0).abs() < 1.0, "screen.y = {}", screen.y);
    }

    #[test]
    fn world_to_screen_point_behind_camera_is_none() {
        let cam = test_camera();
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::Vec2::new(800.0, 600.0));
        let backward = -Vector3::from(cam.calculate_direction_vector());
        let behind = Vector3::from(cam.get_position()) + backward * 5.0;
        assert!(world_to_screen(&cam, rect, behind).is_none());
    }

    #[test]
    fn unproject_center_returns_forward_ray() {
        let cam = test_camera();
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::Vec2::new(800.0, 600.0));
        let center = Pos2::new(400.0, 300.0);
        let (origin, dir) = unproject(&cam, center, rect);
        let expected_forward = Vector3::from(cam.calculate_direction_vector());
        let expected_origin = Vector3::from(cam.get_position());
        assert!((origin - expected_origin).norm() < 1e-5);
        assert!((dir - expected_forward).norm() < 1e-5, "dir = {:?}", dir);
    }
}
```

- [ ] **Step 4: Run the tests — expect FAIL (compile error or assertion)**

Run: `cargo test gizmo::math -- --nocapture`

Expected: build may succeed but tests may need iteration on the camera setup. If everything passes, proceed. If a test fails on the screen coordinate, inspect what the camera's forward vector actually is and adjust the test expectations.

- [ ] **Step 5: Commit**

```bash
git add src/editor/mod.rs src/editor/gizmo/mod.rs src/editor/gizmo/math.rs
git commit -m "feat(editor/gizmo): add math::world_to_screen and math::unproject with tests"
```

---

## Task 2: Math helpers — geometric primitives (TDD)

**Files:**
- Modify: `src/editor/gizmo/math.rs`

- [ ] **Step 1: Add the helper function signatures + tests at the top of the tests module**

Append to `math.rs` **above** the existing `#[cfg(test)] mod tests` block:

```rust
/// Closest point on the infinite line `line_origin + t * line_dir` to the ray
/// `ray_origin + s * ray_dir`. Both directions must be unit length.
pub fn closest_point_on_line_to_ray(
    line_origin: Vector3<f32>, line_dir: Vector3<f32>,
    ray_origin: Vector3<f32>, ray_dir: Vector3<f32>,
) -> Vector3<f32> {
    let w0 = line_origin - ray_origin;
    let a = line_dir.dot(&line_dir);
    let b = line_dir.dot(&ray_dir);
    let c = ray_dir.dot(&ray_dir);
    let d = line_dir.dot(&w0);
    let e = ray_dir.dot(&w0);
    let denom = a * c - b * b;
    let t = if denom.abs() < 1e-6 {
        // Lines are parallel: fall back to projecting onto line at the closest scalar.
        0.0
    } else {
        (b * e - c * d) / denom
    };
    line_origin + line_dir * t
}

/// Intersect a ray with a plane. Returns `None` if the ray is parallel to the plane.
pub fn ray_plane_intersect(
    ray_origin: Vector3<f32>, ray_dir: Vector3<f32>,
    plane_point: Vector3<f32>, plane_normal: Vector3<f32>,
) -> Option<Vector3<f32>> {
    let denom = plane_normal.dot(&ray_dir);
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = (plane_point - ray_origin).dot(&plane_normal) / denom;
    Some(ray_origin + ray_dir * t)
}

/// 2D distance from point `p` to the segment between `a` and `b`.
pub fn distance_point_to_segment_2d(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.x * ab.x + ab.y * ab.y;
    if len_sq < 1e-6 {
        return (p - a).length();
    }
    let t = (((p - a).x * ab.x) + ((p - a).y * ab.y)) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let foot = a + ab * t;
    (p - foot).length()
}

/// Round `value` to the nearest multiple of `step`.
pub fn snap(value: f32, step: f32) -> f32 {
    if step <= 0.0 { return value; }
    (value / step).round() * step
}
```

Add inside the existing `mod tests` block:

```rust
    #[test]
    fn closest_point_perpendicular_ray_hits_line_origin() {
        // Line along +X through origin. Ray comes straight down from (0, 1, 0).
        let p = closest_point_on_line_to_ray(
            Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0), Vector3::new(0.0, -1.0, 0.0),
        );
        assert!((p - Vector3::zeros()).norm() < 1e-4, "p = {:?}", p);
    }

    #[test]
    fn ray_plane_intersect_hits_xy_plane() {
        let p = ray_plane_intersect(
            Vector3::new(0.0, 0.0, 1.0), Vector3::new(0.0, 0.0, -1.0),
            Vector3::zeros(), Vector3::new(0.0, 0.0, 1.0),
        ).expect("ray hits plane");
        assert!((p - Vector3::zeros()).norm() < 1e-4);
    }

    #[test]
    fn ray_plane_intersect_parallel_returns_none() {
        let r = ray_plane_intersect(
            Vector3::new(0.0, 0.0, 1.0), Vector3::new(1.0, 0.0, 0.0),
            Vector3::zeros(), Vector3::new(0.0, 0.0, 1.0),
        );
        assert!(r.is_none());
    }

    #[test]
    fn distance_point_to_segment_perpendicular_foot_inside() {
        let d = distance_point_to_segment_2d(
            Pos2::new(5.0, 3.0),
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
        );
        assert!((d - 3.0).abs() < 1e-4);
    }

    #[test]
    fn distance_point_to_segment_before_a() {
        let d = distance_point_to_segment_2d(
            Pos2::new(-3.0, 0.0),
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
        );
        assert!((d - 3.0).abs() < 1e-4);
    }

    #[test]
    fn distance_point_to_segment_after_b() {
        let d = distance_point_to_segment_2d(
            Pos2::new(13.0, 0.0),
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
        );
        assert!((d - 3.0).abs() < 1e-4);
    }

    #[test]
    fn snap_translate_step_one() {
        assert!((snap(2.3, 1.0) - 2.0).abs() < 1e-6);
        assert!((snap(-2.7, 1.0) - -3.0).abs() < 1e-6);
    }

    #[test]
    fn snap_angle_step_fifteen_degrees() {
        let step = std::f32::consts::PI / 12.0;
        let result = snap(0.5, step);
        // round(0.5 / step) * step — let the test compute the expected.
        let expected = (0.5 / step).round() * step;
        assert!((result - expected).abs() < 1e-6);
    }
```

- [ ] **Step 2: Run tests — expect all to PASS**

Run: `cargo test gizmo::math`

Expected: all 9 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/editor/gizmo/math.rs
git commit -m "feat(editor/gizmo): add closest-point, ray-plane, segment-distance, and snap helpers"
```

---

## Task 3: Refactor `viewport.rs` to use `gizmo::math::unproject`

**Files:**
- Modify: `src/editor/panels/viewport.rs:181-197` (the existing `unproject` fn — delete it)
- Modify: `src/editor/panels/viewport.rs:1-7` (imports)
- Modify: `src/editor/panels/viewport.rs:52` (caller of unproject)

- [ ] **Step 1: Replace the import block at the top of `viewport.rs`**

Edit lines 1-7 of `src/editor/panels/viewport.rs`:

```rust
use egui::Ui;
use enigma_3d::AppState;
use enigma_3d::collision_world::RayCast;
use enigma_3d::camera::Camera;
use nalgebra::Vector3;

use crate::editor::gizmo::math;
use crate::editor::state::{EditorRoot, Selection};
```

(Removed the unused `Pos2, Rect` imports — they were only used by the local `unproject`.)

- [ ] **Step 2: Update the call site at line 52**

Replace the line `let (origin, dir) = unproject(camera, pos, rect);` with:

```rust
        let (origin, dir) = math::unproject(camera, pos, rect);
```

- [ ] **Step 3: Delete the local `unproject` function**

Delete lines 181-197 inclusive — the whole `fn unproject(camera: &Camera, screen_pos: Pos2, rect: Rect) -> (Vector3<f32>, Vector3<f32>) { ... }`.

- [ ] **Step 4: Build**

Run: `cargo build`
Expected: builds cleanly. If `Camera` import becomes unused, remove it; if `Vector3` remains used elsewhere in the file, keep it. (Camera is still used by `frame_target`; Vector3 by movement vectors.)

- [ ] **Step 5: Commit**

```bash
git add src/editor/panels/viewport.rs
git commit -m "refactor(editor): move viewport unproject into editor::gizmo::math"
```

---

## Task 4: `GizmoState` types in `state.rs`

**Files:**
- Modify: `src/editor/state.rs:282-304` (add field) + new types

- [ ] **Step 1: Add the gizmo types to `state.rs`**

Append to the bottom of `src/editor/state.rs`:

```rust
use nalgebra::{UnitQuaternion, Vector3};

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GizmoMode {
    #[default]
    None,
    Translate,
    Rotate,
    Scale,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Space {
    #[default]
    World,
    Local,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis { X, Y, Z }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Handle {
    Axis(Axis),
    Center,
}

pub enum Drag {
    Translate {
        axis: Axis,
        start_pos: Vector3<f32>,
        start_on_axis: Vector3<f32>,
    },
    Rotate {
        axis: Axis,
        start_quat: UnitQuaternion<f32>,
        start_dir: Vector3<f32>,
    },
    Scale {
        handle: Handle,
        start_scale: Vector3<f32>,
        start_pivot_screen: egui::Pos2,
        start_cursor: egui::Pos2,
        start_distance: f32,
    },
}

#[derive(Default)]
pub struct GizmoState {
    pub mode: GizmoMode,
    pub space: Space,
    pub snap_enabled: bool,
    pub drag: Option<Drag>,
    pub hovered_handle: Option<Handle>,
    pub consumed_click_this_frame: bool,
}
```

(The `nalgebra` import goes at the bottom of the existing imports if there's already a `use nalgebra::` line elsewhere in `state.rs`; otherwise leave it grouped with this block.)

- [ ] **Step 2: Add the field to `EditorState`**

Edit `state.rs` around line 303 (inside `pub struct EditorState`). Add as the last field, before the closing brace at line 304:

```rust
    pub gizmo: GizmoState,
```

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: builds cleanly. `GizmoState` has `#[derive(Default)]` and all its fields default, so `EditorState`'s own `#[derive(Default)]` keeps working.

- [ ] **Step 4: Commit**

```bash
git add src/editor/state.rs
git commit -m "feat(editor/gizmo): add GizmoState, GizmoMode, Space, Axis, Handle, Drag types"
```

---

## Task 5: Empty `handle_input` and `draw` stubs + viewport wiring

**Files:**
- Modify: `src/editor/gizmo/mod.rs`
- Modify: `src/editor/panels/viewport.rs`

- [ ] **Step 1: Add stubs to `gizmo/mod.rs`**

Replace `src/editor/gizmo/mod.rs` with:

```rust
pub mod math;

use egui::{Context, Rect, Ui};
use enigma_3d::AppState;
use nalgebra::Vector3;

use crate::editor::state::{EditorRoot, GizmoMode, Selection};

/// Mode-switch hotkeys, hit-tests handles, begins/ends/updates drag.
/// Called from viewport::draw after camera input.
pub fn handle_input(_ctx: &Context, _rect: Rect, _app_state: &mut AppState) {
    // Implemented in later tasks.
}

/// Paints gizmo handles using the viewport rect's painter.
/// Called from viewport::draw after the click-to-select branch.
pub fn draw(_ui: &mut Ui, _rect: Rect, _app_state: &mut AppState) {
    // Implemented in later tasks.
}

/// Resolve the current selection to a draggable target (or None).
pub(crate) fn selection_pivot(app_state: &AppState) -> Option<Vector3<f32>> {
    let root = app_state.get_state_data_value::<EditorRoot>("editor")?;
    match &root.editor.selection {
        Selection::SceneObject(uuid) => app_state
            .objects
            .iter()
            .find(|o| o.get_unique_id() == *uuid)
            .map(|o| o.transform.position),
        Selection::Light(idx) => app_state
            .light
            .get(*idx)
            .map(|l| Vector3::from(l.position)),
        Selection::ParticleInstance(uuid) => root
            .project
            .as_ref()
            .and_then(|p| p.scenes.get(p.active_scene_index))
            .and_then(|s| s.particle_instances.iter().find(|i| i.uuid == *uuid))
            .map(|i| Vector3::from(i.position)),
        Selection::None | _ => None,
    }
}

/// Suppress reads-from-warnings for now — the toolbar module is added later.
#[allow(dead_code)]
fn _gizmo_mode_keep_alive(_m: GizmoMode) {}
```

Note: keep the `#[allow(dead_code)]` helper only if `cargo build` complains about unused `GizmoMode`; if not, drop it.

- [ ] **Step 2: Wire the stubs into `viewport::draw`**

Edit `src/editor/panels/viewport.rs`. After the camera-input branch (current line ~37, just before the primary-released block), add the `handle_input` call. Also update the primary-released branch to honor `consumed_click_this_frame`, and call `draw` at the very end of the function.

Replace the existing `pub fn draw` body. The full new body of the `pub fn draw` function (was lines 15-66):

```rust
pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let rect = ui.max_rect();
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.viewport_rect = Some(rect);
    }

    let ctx = ui.ctx();
    let pointer_in_rect = ctx.input(|i| i.pointer.interact_pos())
        .map(|p| rect.contains(p))
        .unwrap_or(false);

    let rmb_down = ctx.input(|i| i.pointer.secondary_down());
    let mmb_down = ctx.input(|i| i.pointer.middle_down());
    let any_drag = rmb_down || mmb_down;

    if pointer_in_rect || any_drag {
        update_camera(ctx, app_state, pointer_in_rect);
        if any_drag {
            ctx.request_repaint();
        }
    }

    // Gizmo input runs before click-to-select so it can claim mouse-down/up.
    crate::editor::gizmo::handle_input(ctx, rect, app_state);

    let primary_released = ctx.input(|i| i.pointer.primary_released());
    if primary_released && pointer_in_rect && !any_drag {
        let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) else { return; };

        let (drag_active, gizmo_consumed) = app_state
            .get_state_data_value_mut::<EditorRoot>("editor")
            .map(|r| {
                (
                    r.editor.drag.is_some(),
                    std::mem::replace(&mut r.editor.gizmo.consumed_click_this_frame, false),
                )
            })
            .unwrap_or((false, false));
        if drag_active || gizmo_consumed { return; }

        let Some(camera) = app_state.camera.as_ref() else { return; };
        let (origin, dir) = math::unproject(camera, pos, rect);
        let length = camera.far;

        let mut ray = RayCast::new(origin, dir, length);
        ray.cast(app_state);

        let new_selection = ray.get_intersection_uuids().first().copied()
            .map(Selection::SceneObject)
            .unwrap_or(Selection::None);

        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.selection = new_selection;
        }
    }

    crate::editor::gizmo::draw(ui, rect, app_state);
}
```

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: builds cleanly with no warnings.

- [ ] **Step 4: Run the existing app to confirm no regression**

Run: `cargo run`
Manually check: viewport renders, click-to-select still works, RMB-fly still works, F-to-frame still works. Close the app.

- [ ] **Step 5: Commit**

```bash
git add src/editor/gizmo/mod.rs src/editor/panels/viewport.rs
git commit -m "feat(editor/gizmo): wire handle_input/draw stubs + consumed-click guard into viewport"
```

---

## Task 6: Toolbar with mode buttons and toggles

**Files:**
- Create: `src/editor/gizmo/toolbar.rs`
- Modify: `src/editor/gizmo/mod.rs` (add `pub mod toolbar;` + call from `draw`)

- [ ] **Step 1: Create the toolbar module**

Create `src/editor/gizmo/toolbar.rs`:

```rust
use egui::{Area, Context, Order, Rect};
use enigma_3d::AppState;

use crate::editor::state::{EditorRoot, GizmoMode, Space};

pub fn draw(ctx: &Context, rect: Rect, app_state: &mut AppState) {
    let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") else { return; };
    let g = &mut root.editor.gizmo;

    Area::new("gizmo_toolbar")
        .order(Order::Foreground)
        .fixed_pos(rect.min + egui::vec2(8.0, 8.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut mode = g.mode;
                    ui.selectable_value(&mut mode, GizmoMode::None, "Select");
                    ui.selectable_value(&mut mode, GizmoMode::Translate, "Move");
                    ui.selectable_value(&mut mode, GizmoMode::Rotate, "Rotate");
                    ui.selectable_value(&mut mode, GizmoMode::Scale, "Scale");
                    g.mode = mode;

                    ui.separator();

                    let label_space = match g.space {
                        Space::World => "World",
                        Space::Local => "Local",
                    };
                    if ui.button(label_space).clicked() {
                        g.space = match g.space {
                            Space::World => Space::Local,
                            Space::Local => Space::World,
                        };
                    }

                    let snap_label = if g.snap_enabled { "Snap: On" } else { "Snap: Off" };
                    if ui.button(snap_label).clicked() {
                        g.snap_enabled = !g.snap_enabled;
                    }
                });
            });
        });
}
```

- [ ] **Step 2: Hook the toolbar into `gizmo/mod.rs`**

Edit `src/editor/gizmo/mod.rs`. After `pub mod math;`, add:

```rust
pub mod toolbar;
```

And at the end of the existing `pub fn draw(ui, rect, app_state)`:

```rust
pub fn draw(_ui: &mut Ui, rect: Rect, app_state: &mut AppState) {
    toolbar::draw(_ui.ctx(), rect, app_state);
}
```

- [ ] **Step 3: Build + run**

Run: `cargo run`
Manual: open the editor. A toolbar appears in the top-left of the viewport with five buttons: Select, Move, Rotate, Scale, World, Snap. Clicking each toggles state visually. Close.

- [ ] **Step 4: Commit**

```bash
git add src/editor/gizmo/mod.rs src/editor/gizmo/toolbar.rs
git commit -m "feat(editor/gizmo): viewport toolbar with mode buttons and world/snap toggles"
```

---

## Task 7: Translate gizmo — draw, hit-test, drag

**Files:**
- Create: `src/editor/gizmo/translate.rs`
- Modify: `src/editor/gizmo/mod.rs`

- [ ] **Step 1: Create `translate.rs`**

Create `src/editor/gizmo/translate.rs`:

```rust
use egui::{Color32, Pos2, Rect, Stroke, Ui};
use nalgebra::{UnitQuaternion, Vector3};

use crate::editor::gizmo::math;
use crate::editor::state::{Axis, Drag, Space};

/// Fraction of viewport vertical extent that the gizmo handles span.
const SCREEN_FRACTION: f32 = 0.15;
/// 2D pixel tolerance for cursor-on-axis hit-test.
const HIT_TOLERANCE: f32 = 8.0;

pub fn axis_color(axis: Axis, hovered: bool, dragging: bool) -> Color32 {
    if dragging { return Color32::WHITE; }
    let (base, hot) = match axis {
        Axis::X => (Color32::from_rgb(220, 60, 60), Color32::from_rgb(255, 200, 60)),
        Axis::Y => (Color32::from_rgb(60, 200, 60), Color32::from_rgb(255, 200, 60)),
        Axis::Z => (Color32::from_rgb(60, 100, 220), Color32::from_rgb(255, 200, 60)),
    };
    if hovered { hot } else { base }
}

fn axis_basis(axis: Axis) -> Vector3<f32> {
    match axis {
        Axis::X => Vector3::new(1.0, 0.0, 0.0),
        Axis::Y => Vector3::new(0.0, 1.0, 0.0),
        Axis::Z => Vector3::new(0.0, 0.0, 1.0),
    }
}

pub fn axis_dir(axis: Axis, space: Space, rotation: Vector3<f32>) -> Vector3<f32> {
    let basis = axis_basis(axis);
    match space {
        Space::World => basis,
        Space::Local => {
            let q = UnitQuaternion::from_euler_angles(rotation.x, rotation.y, rotation.z);
            (q * basis).normalize()
        }
    }
}

/// World-space handle length, recomputed each frame so the gizmo stays
/// roughly the same size on screen regardless of camera distance.
pub fn handle_world_size(camera_pos: Vector3<f32>, pivot: Vector3<f32>, fov: f32) -> f32 {
    let distance = (pivot - camera_pos).norm().max(0.001);
    distance * (fov / 2.0).tan() * SCREEN_FRACTION
}

/// Hit-test cursor against the three axis segments. Returns the axis whose
/// projected segment is closest to the cursor within HIT_TOLERANCE.
pub fn hit_test(
    cursor: Pos2,
    pivot: Vector3<f32>,
    size: f32,
    space: Space,
    rotation: Vector3<f32>,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
) -> Option<Axis> {
    let mut best: Option<(Axis, f32)> = None;
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let dir = axis_dir(axis, space, rotation);
        let a_world = pivot;
        let b_world = pivot + dir * size;
        let Some(a) = math::world_to_screen(camera, rect, a_world) else { continue };
        let Some(b) = math::world_to_screen(camera, rect, b_world) else { continue };
        let d = math::distance_point_to_segment_2d(cursor, a, b);
        if d <= HIT_TOLERANCE {
            best = match best {
                Some((_, prev)) if prev <= d => best,
                _ => Some((axis, d)),
            };
        }
    }
    best.map(|(a, _)| a)
}

/// Render the three axis lines. Highlights `hovered` and the axis currently
/// being dragged.
pub fn draw(
    ui: &mut Ui,
    rect: Rect,
    pivot: Vector3<f32>,
    size: f32,
    space: Space,
    rotation: Vector3<f32>,
    camera: &enigma_3d::camera::Camera,
    hovered: Option<Axis>,
    dragging: Option<Axis>,
) {
    let painter = ui.painter_at(rect);
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let dir = axis_dir(axis, space, rotation);
        let Some(a) = math::world_to_screen(camera, rect, pivot) else { continue };
        let Some(b) = math::world_to_screen(camera, rect, pivot + dir * size) else { continue };
        let color = axis_color(axis, hovered == Some(axis), dragging == Some(axis));
        painter.line_segment([a, b], Stroke::new(3.0, color));
        // small arrowhead — solid circle at the tip
        painter.circle_filled(b, 5.0, color);
    }
}

/// Start a translate drag on the given axis.
pub fn begin_drag(
    axis: Axis,
    cursor: Pos2,
    pivot: Vector3<f32>,
    space: Space,
    rotation: Vector3<f32>,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
) -> Drag {
    let dir = axis_dir(axis, space, rotation);
    let (ray_o, ray_d) = math::unproject(camera, cursor, rect);
    let start_on_axis = math::closest_point_on_line_to_ray(pivot, dir, ray_o, ray_d);
    Drag::Translate {
        axis,
        start_pos: pivot,
        start_on_axis,
    }
}

/// Update an in-progress translate drag. Returns the new position.
pub fn update_drag(
    axis: Axis,
    start_pos: Vector3<f32>,
    start_on_axis: Vector3<f32>,
    cursor: Pos2,
    space: Space,
    rotation: Vector3<f32>,
    snap: bool,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
) -> Vector3<f32> {
    let dir = axis_dir(axis, space, rotation);
    let (ray_o, ray_d) = math::unproject(camera, cursor, rect);
    let current_on_axis = math::closest_point_on_line_to_ray(start_pos, dir, ray_o, ray_d);
    let mut delta = (current_on_axis - start_on_axis).dot(&dir);
    if snap {
        delta = math::snap(delta, 1.0);
    }
    start_pos + dir * delta
}
```

- [ ] **Step 2: Hook translate into `gizmo/mod.rs` — handle_input + draw**

Replace the `handle_input` and `draw` bodies in `src/editor/gizmo/mod.rs`. Full new content (math/toolbar/translate stay):

```rust
pub mod math;
pub mod toolbar;
pub mod translate;

use egui::{Context, Pos2, Rect, Ui};
use enigma_3d::AppState;
use nalgebra::Vector3;

use crate::editor::state::{Axis, Drag, EditorRoot, GizmoMode, Selection, Space};

pub fn handle_input(ctx: &Context, rect: Rect, app_state: &mut AppState) {
    // Reset the per-frame consumed flag (cleared by viewport, but be safe).
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.gizmo.consumed_click_this_frame = false;
    }

    let rmb = ctx.input(|i| i.pointer.secondary_down());
    if rmb { return; } // RMB-fly suppresses gizmo input.

    let Some(cursor) = ctx.input(|i| i.pointer.interact_pos()) else { return; };
    if !rect.contains(cursor) { return; }

    let Some(pivot) = selection_pivot(app_state) else { return; };
    let Some(camera) = app_state.camera.as_ref() else { return; };
    let camera = camera.clone();

    let (mode, space, snap_enabled, drag_some) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        (
            root.editor.gizmo.mode,
            root.editor.gizmo.space,
            root.editor.gizmo.snap_enabled,
            root.editor.gizmo.drag.is_some(),
        )
    };

    // Effective snap: toolbar XOR Ctrl-held.
    let ctrl = ctx.input(|i| i.modifiers.ctrl);
    let snap = snap_enabled ^ ctrl;

    let rotation = selection_rotation(app_state);

    // Drag in progress: update and possibly end.
    if drag_some {
        let released = ctx.input(|i| i.pointer.primary_released());
        update_active_drag(app_state, cursor, space, rotation, snap, &camera, rect);
        if released {
            end_drag(app_state);
        }
        return;
    }

    // No drag: hit-test and possibly begin.
    let camera_pos = Vector3::from(camera.get_position());
    let size = translate::handle_world_size(camera_pos, pivot, camera.fov);

    let hovered = match mode {
        GizmoMode::Translate => translate::hit_test(cursor, pivot, size, space, rotation, &camera, rect)
            .map(crate::editor::state::Handle::Axis),
        _ => None,
    };

    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.gizmo.hovered_handle = hovered;
    }

    let pressed = ctx.input(|i| i.pointer.primary_pressed());
    if pressed {
        if let Some(crate::editor::state::Handle::Axis(axis)) = hovered {
            let drag = translate::begin_drag(axis, cursor, pivot, space, rotation, &camera, rect);
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                root.editor.gizmo.drag = Some(drag);
                root.editor.gizmo.consumed_click_this_frame = true;
            }
        }
    }
}

pub fn draw(ui: &mut Ui, rect: Rect, app_state: &mut AppState) {
    // Draw handles first, then toolbar over the top.
    if let Some(pivot) = selection_pivot(app_state) {
        if let Some(camera) = app_state.camera.as_ref() {
            let camera = camera.clone();
            let (mode, space, hovered, dragging) = {
                let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else {
                    toolbar::draw(ui.ctx(), rect, app_state);
                    return;
                };
                let drag_axis = match &root.editor.gizmo.drag {
                    Some(Drag::Translate { axis, .. }) => Some(*axis),
                    _ => None,
                };
                let hover_axis = match root.editor.gizmo.hovered_handle {
                    Some(crate::editor::state::Handle::Axis(a)) => Some(a),
                    _ => None,
                };
                (
                    root.editor.gizmo.mode,
                    root.editor.gizmo.space,
                    hover_axis,
                    drag_axis,
                )
            };

            let rotation = selection_rotation(app_state);
            let camera_pos = Vector3::from(camera.get_position());
            let size = translate::handle_world_size(camera_pos, pivot, camera.fov);

            // PositionOnly targets always show the translate gizmo regardless of mode.
            let target_full = matches!(
                app_state.get_state_data_value::<EditorRoot>("editor")
                    .map(|r| &r.editor.selection),
                Some(Selection::SceneObject(_))
            );
            let show_translate = matches!(mode, GizmoMode::Translate)
                || (!target_full && !matches!(mode, GizmoMode::None));

            if show_translate {
                translate::draw(ui, rect, pivot, size, space, rotation, &camera, hovered, dragging);
            }
        }
    }
    toolbar::draw(ui.ctx(), rect, app_state);
}

pub(crate) fn selection_pivot(app_state: &AppState) -> Option<Vector3<f32>> {
    let root = app_state.get_state_data_value::<EditorRoot>("editor")?;
    match &root.editor.selection {
        Selection::SceneObject(uuid) => app_state
            .objects
            .iter()
            .find(|o| o.get_unique_id() == *uuid)
            .map(|o| o.transform.position),
        Selection::Light(idx) => app_state
            .light
            .get(*idx)
            .map(|l| Vector3::from(l.position)),
        Selection::ParticleInstance(uuid) => root
            .project
            .as_ref()
            .and_then(|p| p.scenes.get(p.active_scene_index))
            .and_then(|s| s.particle_instances.iter().find(|i| i.uuid == *uuid))
            .map(|i| Vector3::from(i.position)),
        Selection::None => None,
    }
}

pub(crate) fn selection_rotation(app_state: &AppState) -> Vector3<f32> {
    let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else {
        return Vector3::zeros();
    };
    if let Selection::SceneObject(uuid) = &root.editor.selection {
        if let Some(o) = app_state.objects.iter().find(|o| o.get_unique_id() == *uuid) {
            return o.transform.rotation;
        }
    }
    Vector3::zeros()
}

fn update_active_drag(
    app_state: &mut AppState,
    cursor: Pos2,
    space: Space,
    rotation: Vector3<f32>,
    snap: bool,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
) {
    let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
    let Some(drag) = root.editor.gizmo.drag.as_ref() else { return; };
    match drag {
        Drag::Translate { axis, start_pos, start_on_axis } => {
            let new_pos = translate::update_drag(
                *axis, *start_pos, *start_on_axis, cursor, space, rotation, snap, camera, rect,
            );
            apply_position(app_state, new_pos);
        }
        _ => {}
    }
}

fn apply_position(app_state: &mut AppState, new_pos: Vector3<f32>) {
    let selection = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.selection.clone());
    let Some(selection) = selection else { return; };
    match selection {
        Selection::SceneObject(uuid) => {
            if let Some(o) = app_state.objects.iter_mut().find(|o| o.get_unique_id() == uuid) {
                o.transform.position = new_pos;
            }
        }
        Selection::Light(idx) => {
            if let Some(l) = app_state.light.get_mut(idx) {
                l.position = [new_pos.x, new_pos.y, new_pos.z];
            }
        }
        Selection::ParticleInstance(uuid) => {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(project) = root.project.as_mut() {
                    if let Some(scene) = project.scenes.get_mut(project.active_scene_index) {
                        if let Some(inst) = scene.particle_instances.iter_mut().find(|i| i.uuid == uuid) {
                            inst.position = [new_pos.x, new_pos.y, new_pos.z];
                        }
                    }
                }
            }
        }
        _ => {}
    }
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.dirty = true;
    }
}

fn end_drag(app_state: &mut AppState) {
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.gizmo.drag = None;
        root.editor.gizmo.consumed_click_this_frame = true;
    }
}
```

(If `Selection` doesn't implement `Clone`, that's a separate small fix in `state.rs`. Check first: `grep "impl.*Clone.*for Selection\|#\[derive.*Clone.*\].*\nenum Selection" src/editor/state.rs`. If not derived, add `#[derive(Clone)]` to `Selection`.)

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: builds cleanly. If `Selection` doesn't derive `Clone`, add the derive. If borrow-checker complains about overlapping borrows in `apply_position`, restructure: read the selection out into a local first, then operate.

- [ ] **Step 4: Manual viewport test**

Run: `cargo run`
Manual: 
1. Add a cube. Click it to select. Click "Move" in the toolbar.
2. Three colored axis lines appear at the cube. Hover X — turns yellow. Drag — cube moves along world X. Repeat Y, Z.
3. Toggle World → Local. Rotate the cube via the inspector first to confirm local axes follow the object's rotation when you drag again.
4. Toggle Snap. Drag X — value snaps to integer units.
5. Hold Ctrl during a drag with Snap off — it snaps. Hold Ctrl with Snap on — it unsnaps.
6. Select a light. Click "Move". Translate gizmo appears on the light, drags it.
7. Hold RMB. Cursor in viewport. Press W — no mode change (RMB suppresses). Release RMB, click Move via toolbar instead. (W hotkey isn't wired yet — that's Task 9.)

- [ ] **Step 5: Commit**

```bash
git add src/editor/gizmo/mod.rs src/editor/gizmo/translate.rs src/editor/state.rs
git commit -m "feat(editor/gizmo): translate gizmo — draw, hit-test, drag, world/local, snap, dirty"
```

---

## Task 8: Rotate gizmo

**Files:**
- Create: `src/editor/gizmo/rotate.rs`
- Modify: `src/editor/gizmo/mod.rs`

- [ ] **Step 1: Create `rotate.rs`**

Create `src/editor/gizmo/rotate.rs`:

```rust
use egui::{Pos2, Rect, Stroke, Ui};
use enigma_3d::AppState;
use nalgebra::{Unit, UnitQuaternion, Vector3};

use crate::editor::gizmo::math;
use crate::editor::gizmo::translate::{axis_color, axis_dir};
use crate::editor::state::{Axis, Drag, Space};

const RING_SAMPLES: usize = 64;
const HIT_TOLERANCE: f32 = 8.0;

/// Build the two basis vectors that span the ring plane for `axis_dir`.
/// Used to walk around the ring in world space.
fn ring_basis(axis_dir: Vector3<f32>) -> (Vector3<f32>, Vector3<f32>) {
    let world_up = Vector3::new(0.0, 1.0, 0.0);
    let pick = if axis_dir.dot(&world_up).abs() > 0.9 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        world_up
    };
    let u = pick.cross(&axis_dir).normalize();
    let v = axis_dir.cross(&u).normalize();
    (u, v)
}

/// Sample `RING_SAMPLES` world-space points around the ring.
fn sample_ring(pivot: Vector3<f32>, axis_dir: Vector3<f32>, radius: f32) -> Vec<Vector3<f32>> {
    let (u, v) = ring_basis(axis_dir);
    (0..RING_SAMPLES)
        .map(|i| {
            let t = i as f32 / RING_SAMPLES as f32 * std::f32::consts::TAU;
            pivot + (u * t.cos() + v * t.sin()) * radius
        })
        .collect()
}

pub fn hit_test(
    cursor: Pos2,
    pivot: Vector3<f32>,
    radius: f32,
    space: Space,
    rotation: Vector3<f32>,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
) -> Option<Axis> {
    let mut best: Option<(Axis, f32)> = None;
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let dir = axis_dir(axis, space, rotation);
        let pts = sample_ring(pivot, dir, radius);
        let mut screen_pts: Vec<Pos2> = Vec::with_capacity(RING_SAMPLES);
        for p in &pts {
            if let Some(s) = math::world_to_screen(camera, rect, *p) {
                screen_pts.push(s);
            }
        }
        if screen_pts.len() < 2 { continue; }
        let mut min_d = f32::INFINITY;
        for i in 0..screen_pts.len() {
            let a = screen_pts[i];
            let b = screen_pts[(i + 1) % screen_pts.len()];
            let d = math::distance_point_to_segment_2d(cursor, a, b);
            if d < min_d { min_d = d; }
        }
        if min_d <= HIT_TOLERANCE {
            best = match best {
                Some((_, prev)) if prev <= min_d => best,
                _ => Some((axis, min_d)),
            };
        }
    }
    best.map(|(a, _)| a)
}

pub fn draw(
    ui: &mut Ui,
    rect: Rect,
    pivot: Vector3<f32>,
    radius: f32,
    space: Space,
    rotation: Vector3<f32>,
    camera: &enigma_3d::camera::Camera,
    hovered: Option<Axis>,
    dragging: Option<Axis>,
) {
    let painter = ui.painter_at(rect);
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let dir = axis_dir(axis, space, rotation);
        let pts = sample_ring(pivot, dir, radius);
        let mut screen_pts: Vec<Pos2> = Vec::with_capacity(RING_SAMPLES);
        for p in &pts {
            if let Some(s) = math::world_to_screen(camera, rect, *p) {
                screen_pts.push(s);
            }
        }
        if screen_pts.len() < 2 { continue; }
        let color = axis_color(axis, hovered == Some(axis), dragging == Some(axis));
        for i in 0..screen_pts.len() {
            let a = screen_pts[i];
            let b = screen_pts[(i + 1) % screen_pts.len()];
            painter.line_segment([a, b], Stroke::new(2.0, color));
        }
    }
}

pub fn begin_drag(
    axis: Axis,
    cursor: Pos2,
    pivot: Vector3<f32>,
    space: Space,
    rotation: Vector3<f32>,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
) -> Option<Drag> {
    let dir = axis_dir(axis, space, rotation);
    let (ray_o, ray_d) = math::unproject(camera, cursor, rect);
    let p0 = math::ray_plane_intersect(ray_o, ray_d, pivot, dir)?;
    let start_dir = (p0 - pivot).normalize();
    let start_quat = UnitQuaternion::from_euler_angles(rotation.x, rotation.y, rotation.z);
    Some(Drag::Rotate { axis, start_quat, start_dir })
}

/// Returns the new Euler rotation to write into `transform.rotation`.
/// `pivot` is the object's current position — used to define the rotation plane.
pub fn update_drag(
    axis: Axis,
    start_quat: UnitQuaternion<f32>,
    start_dir: Vector3<f32>,
    pivot: Vector3<f32>,
    cursor: Pos2,
    space: Space,
    snap: bool,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
) -> Vector3<f32> {
    // The ring direction is computed from the START rotation so it stays
    // anchored throughout the drag, even as `transform.rotation` changes.
    let start_rotation = {
        let (rx, ry, rz) = start_quat.euler_angles();
        Vector3::new(rx, ry, rz)
    };
    let dir = axis_dir(axis, space, start_rotation);

    let (ray_o, ray_d) = math::unproject(camera, cursor, rect);
    let Some(p) = math::ray_plane_intersect(ray_o, ray_d, pivot, dir) else {
        return start_rotation;
    };
    let current_dir = (p - pivot).normalize();

    let cos_a = start_dir.dot(&current_dir).clamp(-1.0, 1.0);
    let sin_a = dir.dot(&start_dir.cross(&current_dir));
    let mut delta_angle = sin_a.atan2(cos_a);
    if snap {
        delta_angle = math::snap(delta_angle, std::f32::consts::PI / 12.0);
    }

    let axis_unit = Unit::new_normalize(dir);
    let new_quat = UnitQuaternion::from_axis_angle(&axis_unit, delta_angle) * start_quat;
    let (rx, ry, rz) = new_quat.euler_angles();
    Vector3::new(rx, ry, rz)
}
```

- [ ] **Step 2: Wire rotate into `gizmo/mod.rs`**

Edit `src/editor/gizmo/mod.rs`:

1. Add `pub mod rotate;` near the top.
2. Extend `handle_input` so that when `mode == GizmoMode::Rotate` and `target_full == true`, we also hit-test and possibly begin-drag a rotate ring. Modify the `hovered` calculation:

```rust
    let target_full = matches!(
        app_state.get_state_data_value::<EditorRoot>("editor")
            .map(|r| &r.editor.selection),
        Some(Selection::SceneObject(_))
    );

    let hovered = match mode {
        GizmoMode::Translate => translate::hit_test(cursor, pivot, size, space, rotation, &camera, rect)
            .map(crate::editor::state::Handle::Axis),
        GizmoMode::Rotate if target_full => rotate::hit_test(cursor, pivot, size, space, rotation, &camera, rect)
            .map(crate::editor::state::Handle::Axis),
        _ => None,
    };
```

3. In the `pressed` branch, dispatch by mode:

```rust
    if pressed {
        if let Some(crate::editor::state::Handle::Axis(axis)) = hovered {
            let drag = match mode {
                GizmoMode::Translate => Some(translate::begin_drag(axis, cursor, pivot, space, rotation, &camera, rect)),
                GizmoMode::Rotate => rotate::begin_drag(axis, cursor, pivot, space, rotation, &camera, rect),
                _ => None,
            };
            if let Some(drag) = drag {
                if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                    root.editor.gizmo.drag = Some(drag);
                    root.editor.gizmo.consumed_click_this_frame = true;
                }
            }
        }
    }
```

4. Extend `update_active_drag` to handle the Rotate variant. Add a `pivot: Vector3<f32>` parameter and pass it down. The match becomes:

```rust
    match drag {
        Drag::Translate { axis, start_pos, start_on_axis } => {
            let new_pos = translate::update_drag(
                *axis, *start_pos, *start_on_axis, cursor, space, rotation, snap, camera, rect,
            );
            apply_position(app_state, new_pos);
        }
        Drag::Rotate { axis, start_quat, start_dir } => {
            let new_rot = rotate::update_drag(
                *axis, *start_quat, *start_dir, pivot, cursor, space, snap, camera, rect,
            );
            apply_rotation(app_state, new_rot);
        }
        _ => {}
    }
```

(Add `pivot: Vector3<f32>` to `update_active_drag`'s signature and pass it from `handle_input`.)

5. Add `apply_rotation`:

```rust
fn apply_rotation(app_state: &mut AppState, new_rot: Vector3<f32>) {
    let selection = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.selection.clone());
    let Some(selection) = selection else { return; };
    if let Selection::SceneObject(uuid) = selection {
        if let Some(o) = app_state.objects.iter_mut().find(|o| o.get_unique_id() == uuid) {
            o.transform.rotation = new_rot;
        }
    }
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.dirty = true;
    }
}
```

6. Extend the `drag_axis` extraction in `draw` to also handle `Drag::Rotate`, so `dragging: Option<Axis>` is populated for both translate and rotate:

```rust
                let drag_axis = match &root.editor.gizmo.drag {
                    Some(Drag::Translate { axis, .. }) => Some(*axis),
                    Some(Drag::Rotate { axis, .. }) => Some(*axis),
                    _ => None,
                };
```

Then after the `if show_translate { ... }` block, add the rotate render branch:

```rust
            let show_rotate = matches!(mode, GizmoMode::Rotate) && target_full;
            if show_rotate {
                rotate::draw(ui, rect, pivot, size, space, rotation, &camera, hovered, dragging);
            }
```

`hovered` and `dragging` are both `Option<Axis>` — same shape as translate's signature.

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: clean. Resolve any borrow/lifetime issues by reading values out into locals first.

- [ ] **Step 4: Manual viewport test**

Run: `cargo run`
Manual:
1. Select a cube. Click "Rotate". Three colored rings appear at the cube.
2. Drag the X ring (red, in YZ plane). Cube rotates around world X.
3. Repeat Y, Z.
4. Toggle World → Local. Pre-rotate the cube via the inspector. Drag a ring in Local — it rotates around the cube's own axis.
5. Toggle Snap. Drag a ring — angle snaps to 15°.
6. Hold Ctrl during drag — snap inverts.

- [ ] **Step 5: Commit**

```bash
git add src/editor/gizmo/mod.rs src/editor/gizmo/rotate.rs
git commit -m "feat(editor/gizmo): rotate gizmo — rings, ray-plane drag, quaternion roundtrip, snap"
```

---

## Task 9: Scale gizmo

**Files:**
- Create: `src/editor/gizmo/scale.rs`
- Modify: `src/editor/gizmo/mod.rs`

- [ ] **Step 1: Create `scale.rs`**

Create `src/editor/gizmo/scale.rs`:

```rust
use egui::{Color32, Pos2, Rect, Stroke, Ui};
use nalgebra::Vector3;

use crate::editor::gizmo::math;
use crate::editor::gizmo::translate::{axis_color, axis_dir};
use crate::editor::state::{Axis, Drag, Handle, Space};

const HIT_TOLERANCE: f32 = 8.0;

fn cube_screen_radius(size: f32, camera: &enigma_3d::camera::Camera) -> f32 {
    // The world-space size is `size`. A tip cube spans ~10% of that on each side.
    // Approximate the projected half-extent as a small constant pixel radius.
    let _ = size; let _ = camera;
    8.0
}

pub fn hit_test(
    cursor: Pos2,
    pivot: Vector3<f32>,
    size: f32,
    rotation: Vector3<f32>,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
) -> Option<Handle> {
    // Scale is always object-local: use Local space for the axes.
    let mut best: Option<(Handle, f32)> = None;
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let dir = axis_dir(axis, Space::Local, rotation);
        let tip = pivot + dir * size;
        let Some(tip_s) = math::world_to_screen(camera, rect, tip) else { continue };
        let r = cube_screen_radius(size, camera);
        let d = (cursor - tip_s).length();
        if d <= r + HIT_TOLERANCE {
            best = match best {
                Some((_, prev)) if prev <= d => best,
                _ => Some((Handle::Axis(axis), d)),
            };
        }
    }
    if let Some(center_s) = math::world_to_screen(camera, rect, pivot) {
        let r = cube_screen_radius(size, camera) * 0.8;
        let d = (cursor - center_s).length();
        if d <= r + HIT_TOLERANCE {
            best = match best {
                Some((_, prev)) if prev <= d => best,
                _ => Some((Handle::Center, d)),
            };
        }
    }
    best.map(|(h, _)| h)
}

pub fn draw(
    ui: &mut Ui,
    rect: Rect,
    pivot: Vector3<f32>,
    size: f32,
    rotation: Vector3<f32>,
    camera: &enigma_3d::camera::Camera,
    hovered: Option<Handle>,
    dragging: Option<Handle>,
) {
    let painter = ui.painter_at(rect);
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let dir = axis_dir(axis, Space::Local, rotation);
        let Some(a) = math::world_to_screen(camera, rect, pivot) else { continue };
        let Some(b) = math::world_to_screen(camera, rect, pivot + dir * size) else { continue };
        let hov = matches!(hovered, Some(Handle::Axis(x)) if x == axis);
        let drg = matches!(dragging, Some(Handle::Axis(x)) if x == axis);
        let color = axis_color(axis, hov, drg);
        painter.line_segment([a, b], Stroke::new(2.0, color));
        // tip cube as a filled rect centered on `b`.
        let r = cube_screen_radius(size, camera);
        painter.rect_filled(
            Rect::from_center_size(b, egui::vec2(r * 2.0, r * 2.0)),
            0.0,
            color,
        );
    }
    if let Some(c) = math::world_to_screen(camera, rect, pivot) {
        let r = cube_screen_radius(0.0, camera) * 0.8;
        let hov = matches!(hovered, Some(Handle::Center));
        let drg = matches!(dragging, Some(Handle::Center));
        let color = if drg { Color32::WHITE }
            else if hov { Color32::from_rgb(255, 200, 60) }
            else { Color32::from_gray(220) };
        painter.rect_filled(
            Rect::from_center_size(c, egui::vec2(r * 2.0, r * 2.0)),
            0.0,
            color,
        );
    }
}

pub fn begin_drag(
    handle: Handle,
    cursor: Pos2,
    pivot: Vector3<f32>,
    start_scale: Vector3<f32>,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
) -> Option<Drag> {
    let start_pivot_screen = math::world_to_screen(camera, rect, pivot)?;
    let start_distance = (cursor - start_pivot_screen).length();
    if start_distance < 1.0 { return None; } // guard against div-by-zero
    Some(Drag::Scale {
        handle,
        start_scale,
        start_pivot_screen,
        start_cursor: cursor,
        start_distance,
    })
}

/// Returns the new scale vector to write into `transform.scale`.
pub fn update_drag(
    handle: Handle,
    start_scale: Vector3<f32>,
    start_pivot_screen: Pos2,
    start_distance: f32,
    cursor: Pos2,
    snap: bool,
) -> Vector3<f32> {
    let current_distance = (cursor - start_pivot_screen).length();
    let mut factor = current_distance / start_distance.max(1e-3);
    if snap {
        factor = (math::snap(factor, 0.1)).max(0.1);
    }
    match handle {
        Handle::Axis(Axis::X) => Vector3::new(start_scale.x * factor, start_scale.y, start_scale.z),
        Handle::Axis(Axis::Y) => Vector3::new(start_scale.x, start_scale.y * factor, start_scale.z),
        Handle::Axis(Axis::Z) => Vector3::new(start_scale.x, start_scale.y, start_scale.z * factor),
        Handle::Center => start_scale * factor,
    }
}
```

- [ ] **Step 2: Wire scale into `gizmo/mod.rs`**

1. Add `pub mod scale;`.
2. Extend the `hovered` branch in `handle_input`:

```rust
        GizmoMode::Scale if target_full => scale::hit_test(cursor, pivot, size, rotation, &camera, rect),
```

(Returns `Option<Handle>` directly — no `.map(Handle::Axis)` wrapping like translate/rotate.)

3. In the `pressed` branch, add a scale dispatch. Read the selection's `start_scale` first:

```rust
                GizmoMode::Scale => {
                    let start_scale = selection_scale(app_state).unwrap_or(Vector3::new(1.0, 1.0, 1.0));
                    if let Some(crate::editor::state::Handle::Axis(_) | crate::editor::state::Handle::Center) = hovered {
                        scale::begin_drag(hovered.unwrap(), cursor, pivot, start_scale, &camera, rect)
                    } else { None }
                }
```

(Adjust the surrounding `if let Some(Handle::Axis(axis)) = hovered` pattern so it accepts either axis or center for scale. Cleanest: change to `if let Some(handle) = hovered { ... match by mode ... }`.)

4. Extend `update_active_drag`'s match arm:

```rust
        Drag::Scale { handle, start_scale, start_pivot_screen, start_distance, .. } => {
            let new_scale = scale::update_drag(
                *handle, *start_scale, *start_pivot_screen, *start_distance, cursor, snap,
            );
            apply_scale(app_state, new_scale);
        }
```

5. Add `apply_scale` and `selection_scale`:

```rust
fn selection_scale(app_state: &AppState) -> Option<Vector3<f32>> {
    let root = app_state.get_state_data_value::<EditorRoot>("editor")?;
    if let Selection::SceneObject(uuid) = &root.editor.selection {
        return app_state.objects.iter()
            .find(|o| o.get_unique_id() == *uuid)
            .map(|o| o.transform.scale);
    }
    None
}

fn apply_scale(app_state: &mut AppState, new_scale: Vector3<f32>) {
    let selection = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.selection.clone());
    let Some(selection) = selection else { return; };
    if let Selection::SceneObject(uuid) = selection {
        if let Some(o) = app_state.objects.iter_mut().find(|o| o.get_unique_id() == uuid) {
            o.transform.scale = new_scale;
        }
    }
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.dirty = true;
    }
}
```

6. Extend `draw`:

```rust
            let show_scale = matches!(mode, GizmoMode::Scale) && target_full;
            if show_scale {
                let dragging_handle = match &app_state.get_state_data_value::<EditorRoot>("editor")
                    .and_then(|r| r.editor.gizmo.drag.as_ref())
                {
                    Some(Drag::Scale { handle, .. }) => Some(*handle),
                    _ => None,
                };
                scale::draw(ui, rect, pivot, size, rotation, &camera, hovered, dragging_handle);
            }
```

- [ ] **Step 3: Build**

Run: `cargo build`
Expected: clean.

- [ ] **Step 4: Manual viewport test**

Run: `cargo run`
Manual:
1. Select a cube. Click "Scale". Three colored axes ending in cubes appear, plus a small white cube at the origin.
2. Drag the X cube — cube scales along X only.
3. Drag the center cube — uniform scale on all three axes.
4. Toggle Snap. Drag X — factor snaps to 0.1 increments. Verify scale can't go below 0.1× the start.
5. Verify the world/local toggle has no effect on scale (it's always local).

- [ ] **Step 5: Commit**

```bash
git add src/editor/gizmo/mod.rs src/editor/gizmo/scale.rs
git commit -m "feat(editor/gizmo): scale gizmo — axis cubes + uniform center, screen-distance factor"
```

---

## Task 10: Hotkeys W/E/R/Q

**Files:**
- Modify: `src/editor/gizmo/mod.rs`

- [ ] **Step 1: Add hotkey detection at the top of `handle_input`**

Edit `src/editor/gizmo/mod.rs`. At the start of `handle_input`, after the RMB guard and before reading the cursor, add:

```rust
    // Hotkeys — only when the pointer is in the viewport and RMB isn't held.
    let pointer = ctx.input(|i| i.pointer.interact_pos());
    let in_rect = pointer.map(|p| rect.contains(p)).unwrap_or(false);
    if in_rect && !rmb {
        let pressed = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Q),
                i.key_pressed(egui::Key::W),
                i.key_pressed(egui::Key::E),
                i.key_pressed(egui::Key::R),
            )
        });
        if pressed.0 || pressed.1 || pressed.2 || pressed.3 {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if pressed.0 { root.editor.gizmo.mode = GizmoMode::None; }
                if pressed.1 { root.editor.gizmo.mode = GizmoMode::Translate; }
                if pressed.2 { root.editor.gizmo.mode = GizmoMode::Rotate; }
                if pressed.3 { root.editor.gizmo.mode = GizmoMode::Scale; }
            }
        }
    }
```

- [ ] **Step 2: Build**

Run: `cargo build`
Expected: clean.

- [ ] **Step 3: Manual viewport test**

Run: `cargo run`
Manual:
1. Select a cube. Hover the viewport.
2. Press Q — gizmo disappears (mode = None).
3. Press W — translate gizmo. E — rotate. R — scale.
4. Hold RMB and press W — no mode change (camera fly takes precedence).
5. Hover the toolbar (not the viewport) and press W — no mode change (only viewport hotkey).

- [ ] **Step 4: Commit**

```bash
git add src/editor/gizmo/mod.rs
git commit -m "feat(editor/gizmo): Q/W/E/R hotkeys for select/translate/rotate/scale"
```

---

## Task 11: Manual verification + spec sign-off

**Files:** none (verification only)

- [ ] **Step 1: Walk the full manual checklist from the spec**

Run `cargo run` and verify each:

1. Select a cube. Press W → translate gizmo appears.
2. Drag X — moves along world X. Y, Z — same.
3. Press E. Drag a ring — rotates. Switch to Local, rotate Y 45° via inspector, then translate in Local — moves along the cube's tilted X.
4. Press R. Drag axis cube — only that axis scales. Drag center cube — uniform.
5. Toggle Snap. Drag X — snaps to 1.0. Hold Ctrl mid-drag — snap inverts.
6. Press Q. Gizmo gone. Click-to-select still works.
7. Select a directional light. Press W — translate gizmo on the light. Press E — still the translate gizmo (silent downgrade); toolbar shows Rotate active.
8. Hold RMB and press W — no mode change. Press F — camera frames selection (existing hotkey unchanged).
9. Open a cube's inspector while a gizmo is active. Drag in the viewport — inspector's numeric fields update live.

- [ ] **Step 2: Run `cargo test` once more**

Run: `cargo test`
Expected: all gizmo math tests pass, no regressions elsewhere.

- [ ] **Step 3: Final commit (only if any polish fix was needed)**

If any check failed and was patched, commit. Otherwise no commit.

---

## Notes for the engineer

- The codebase pattern is to mutate `AppState` through `app_state.get_state_data_value_mut::<EditorRoot>("editor")`. Borrow-checker fights are common: read what you need into locals first, then operate.
- The `Selection` enum may need `#[derive(Clone)]` added; check `state.rs` and add it if necessary.
- nalgebra's `UnitQuaternion::euler_angles()` returns `(roll, pitch, yaw) = (X, Y, Z)`, matching what the codebase already uses on line 915 of `enigma-3d/src/object.rs`.
- `egui::Area` paints on top of the central panel naturally because of `Order::Foreground`.
- When you're unsure whether `gizmo::handle_input` should consume a click vs let it through, lean toward consume — false-positive consumption is harmless (a frame skipped), false-negative consumption double-fires the ray-pick.
