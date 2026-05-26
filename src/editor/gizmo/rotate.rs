use egui::{Pos2, Rect, Stroke, Ui};
use nalgebra::{Unit, UnitQuaternion, Vector3};

use crate::editor::gizmo::math;
use crate::editor::gizmo::translate::{axis_color, axis_dir};
use crate::editor::state::{Axis, Drag, Space};

const RING_SAMPLES: usize = 64;
const HIT_TOLERANCE: f32 = 8.0;

/// Build the two basis vectors that span the ring plane for `axis_dir`.
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
    // Anchor the ring direction in the START rotation so it stays put across the drag.
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
