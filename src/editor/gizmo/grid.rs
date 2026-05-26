use egui::{Color32, Rect, Stroke, Ui};
use nalgebra::Vector3;

use crate::editor::gizmo::math;

/// Half-extent of the grid in world units, measured from the camera's
/// XZ-projected gaze point. The grid spans `2 * EXTENT` units on each axis.
const EXTENT: i32 = 50;
/// Spacing between adjacent grid lines, in world units.
const MINOR_STEP: i32 = 1;
/// Lines whose coordinate is a multiple of this step render brighter.
const MAJOR_STEP: i32 = 10;

const MINOR_COLOR: Color32 = Color32::from_rgba_premultiplied(80, 80, 80, 110);
const MAJOR_COLOR: Color32 = Color32::from_rgba_premultiplied(140, 140, 140, 170);
const X_AXIS_COLOR: Color32 = Color32::from_rgba_premultiplied(200, 70, 70, 220);
const Z_AXIS_COLOR: Color32 = Color32::from_rgba_premultiplied(70, 100, 200, 220);

pub fn draw(
    ui: &mut Ui,
    rect: Rect,
    camera: &enigma_3d::camera::Camera,
) {
    let painter = ui.painter_at(rect);

    let center = grid_center(camera);
    let min_x = center.0 - EXTENT;
    let max_x = center.0 + EXTENT;
    let min_z = center.1 - EXTENT;
    let max_z = center.1 + EXTENT;

    // Lines parallel to Z (varying X).
    for x in min_x..=max_x {
        let color = line_color_x(x);
        let a = Vector3::new(x as f32, 0.0, min_z as f32);
        let b = Vector3::new(x as f32, 0.0, max_z as f32);
        draw_world_line(&painter, camera, rect, a, b, color);
    }

    // Lines parallel to X (varying Z).
    for z in min_z..=max_z {
        let color = line_color_z(z);
        let a = Vector3::new(min_x as f32, 0.0, z as f32);
        let b = Vector3::new(max_x as f32, 0.0, z as f32);
        draw_world_line(&painter, camera, rect, a, b, color);
    }
}

fn line_color_x(x: i32) -> Color32 {
    if x == 0 { Z_AXIS_COLOR }
    else if x % MAJOR_STEP == 0 { MAJOR_COLOR }
    else { MINOR_COLOR }
}

fn line_color_z(z: i32) -> Color32 {
    if z == 0 { X_AXIS_COLOR }
    else if z % MAJOR_STEP == 0 { MAJOR_COLOR }
    else { MINOR_COLOR }
}

/// Center the grid on the camera's XZ position, snapped to MINOR_STEP so the
/// grid follows the camera but stays aligned to integer world coordinates.
/// Centering on the camera (rather than its gaze landing point on Y=0) keeps
/// the area directly under the camera covered no matter where it's looking.
fn grid_center(camera: &enigma_3d::camera::Camera) -> (i32, i32) {
    let pos = Vector3::from(camera.get_position());
    let snap = MINOR_STEP as f32;
    let cx = (pos.x / snap).round() as i32 * MINOR_STEP;
    let cz = (pos.z / snap).round() as i32 * MINOR_STEP;
    (cx, cz)
}

fn draw_world_line(
    painter: &egui::Painter,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
    a_world: Vector3<f32>,
    b_world: Vector3<f32>,
    color: Color32,
) {
    let Some(a) = math::world_to_screen(camera, rect, a_world) else { return; };
    let Some(b) = math::world_to_screen(camera, rect, b_world) else { return; };
    painter.line_segment([a, b], Stroke::new(1.0, color));
}
