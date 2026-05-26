use egui::{Color32, Pos2, Rect, Stroke, Ui};
use nalgebra::Vector3;

use crate::editor::gizmo::math;
use crate::editor::gizmo::translate::{axis_color, axis_dir};
use crate::editor::state::{Axis, Drag, Handle, Space};

const HIT_TOLERANCE: f32 = 8.0;

fn cube_screen_radius() -> f32 {
    // Fixed pixel radius for axis-tip and center handles. The actual world-space
    // box at the axis tip projects to roughly this size at typical fovs/distances.
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
    let mut best: Option<(Handle, f32)> = None;
    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let dir = axis_dir(axis, Space::Local, rotation);
        let tip = pivot + dir * size;
        let Some(tip_s) = math::world_to_screen(camera, rect, tip) else { continue };
        let r = cube_screen_radius();
        let d = (cursor - tip_s).length();
        if d <= r + HIT_TOLERANCE {
            best = match best {
                Some((_, prev)) if prev <= d => best,
                _ => Some((Handle::Axis(axis), d)),
            };
        }
    }
    if let Some(center_s) = math::world_to_screen(camera, rect, pivot) {
        let r = cube_screen_radius() * 0.8;
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
        let r = cube_screen_radius();
        painter.rect_filled(
            Rect::from_center_size(b, egui::vec2(r * 2.0, r * 2.0)),
            0.0,
            color,
        );
    }
    if let Some(c) = math::world_to_screen(camera, rect, pivot) {
        let r = cube_screen_radius() * 0.8;
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
