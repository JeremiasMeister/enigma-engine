use egui::{Pos2, Rect, Ui};
use enigma_3d::AppState;
use enigma_3d::collision_world::RayCast;
use enigma_3d::camera::Camera;
use nalgebra::Vector3;

use crate::editor::state::{EditorRoot, Selection};

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let rect = ui.max_rect();
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.viewport_rect = Some(rect);
    }

    let ctx = ui.ctx();
    let pressed = ctx.input(|i| i.pointer.primary_released());
    if !pressed { return; }
    let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) else { return; };
    if !rect.contains(pos) { return; }

    let drag_active = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.drag.is_some())
        .unwrap_or(false);
    if drag_active { return; }

    let Some(camera) = app_state.camera.as_ref() else { return; };
    let (origin, dir) = unproject(camera, pos, rect);
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

fn unproject(camera: &Camera, screen_pos: Pos2, rect: Rect) -> (Vector3<f32>, Vector3<f32>) {
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
