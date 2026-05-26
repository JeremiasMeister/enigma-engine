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

/// Render the three axis lines.
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
