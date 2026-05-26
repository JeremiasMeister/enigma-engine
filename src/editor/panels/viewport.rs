use egui::Ui;
use enigma_3d::AppState;
use enigma_3d::collision_world::RayCast;
use enigma_3d::camera::Camera;
use nalgebra::Vector3;

use crate::editor::gizmo::math;
use crate::editor::state::{EditorRoot, Selection};

const FLY_SPEED: f32 = 4.0;          // world units per second
const FAST_MULT: f32 = 4.0;          // shift-held multiplier
const LOOK_SENSITIVITY: f32 = 0.005; // radians per pixel
const PAN_SENSITIVITY: f32 = 0.01;
const WHEEL_DOLLY: f32 = 0.5;        // units per scroll-line

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

    // Camera controls only when the pointer started in the viewport.
    // Use any-drag to keep moving even if cursor leaves the rect.
    if pointer_in_rect || any_drag {
        update_camera(ctx, app_state, pointer_in_rect);
        if any_drag {
            ctx.request_repaint();
        }
    }

    // Click-to-select via ray pick. Skip when releasing a drag — primary release
    // from a deselect drag shouldn't reselect.
    let primary_released = ctx.input(|i| i.pointer.primary_released());
    if primary_released && pointer_in_rect && !any_drag {
        let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) else { return; };

        let drag_active = app_state
            .get_state_data_value::<EditorRoot>("editor")
            .map(|r| r.editor.drag.is_some())
            .unwrap_or(false);
        if drag_active { return; }

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
}

fn update_camera(ctx: &egui::Context, app_state: &mut AppState, pointer_in_rect: bool) {
    let dt = ctx.input(|i| i.unstable_dt);
    let rmb_down = ctx.input(|i| i.pointer.secondary_down());
    let mmb_down = ctx.input(|i| i.pointer.middle_down());
    let mouse_delta = ctx.input(|i| i.pointer.delta());
    let wheel = ctx.input(|i| i.scroll_delta.y);
    let shift = ctx.input(|i| i.modifiers.shift);

    // Press F to frame selected — requires pointer in viewport.
    let frame_pressed = pointer_in_rect && ctx.input(|i| i.key_pressed(egui::Key::F));

    let Some(cam) = app_state.camera.as_mut() else { return; };

    // Look: RMB drag rotates camera in place.
    if rmb_down && (mouse_delta.x.abs() > 0.0 || mouse_delta.y.abs() > 0.0) {
        // Camera rotation is stored as radian Euler (X, Y, Z) where Y is yaw
        // and X is pitch. calculate_direction_vector uses [pitch, yaw, _].
        let yaw_delta = -mouse_delta.x * LOOK_SENSITIVITY;
        let pitch_delta = -mouse_delta.y * LOOK_SENSITIVITY;
        cam.transform.rotation.y += yaw_delta;
        cam.transform.rotation.x = (cam.transform.rotation.x + pitch_delta)
            .clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);
        cam.update_matrices();
    }

    // Fly: WASDQE while RMB held.
    let mut move_vec = Vector3::<f32>::new(0.0, 0.0, 0.0);
    if rmb_down {
        let forward = Vector3::from(cam.calculate_direction_vector());
        let world_up = Vector3::new(0.0, 1.0, 0.0);
        let right = forward.cross(&world_up).normalize();

        if ctx.input(|i| i.key_down(egui::Key::W)) { move_vec += forward; }
        if ctx.input(|i| i.key_down(egui::Key::S)) { move_vec -= forward; }
        if ctx.input(|i| i.key_down(egui::Key::D)) { move_vec += right; }
        if ctx.input(|i| i.key_down(egui::Key::A)) { move_vec -= right; }
        if ctx.input(|i| i.key_down(egui::Key::E)) { move_vec += world_up; }
        if ctx.input(|i| i.key_down(egui::Key::Q)) { move_vec -= world_up; }
    }

    if move_vec.norm() > 0.0 {
        let speed = if shift { FLY_SPEED * FAST_MULT } else { FLY_SPEED };
        let step = move_vec.normalize() * speed * dt;
        cam.transform.position += step;
        cam.update_matrices();
    }

    // Pan: MMB drag moves camera in screen plane.
    if mmb_down && (mouse_delta.x.abs() > 0.0 || mouse_delta.y.abs() > 0.0) {
        let forward = Vector3::from(cam.calculate_direction_vector());
        let world_up = Vector3::new(0.0, 1.0, 0.0);
        let right = forward.cross(&world_up).normalize();
        let up = right.cross(&forward).normalize();
        let pan_speed = PAN_SENSITIVITY * if shift { FAST_MULT } else { 1.0 };
        let step = -right * (mouse_delta.x * pan_speed) + up * (mouse_delta.y * pan_speed);
        cam.transform.position += step;
        cam.update_matrices();
    }

    // Wheel: dolly forward/back along view direction.
    if wheel.abs() > 0.0 && pointer_in_rect {
        let forward = Vector3::from(cam.calculate_direction_vector());
        let step_mul = if shift { FAST_MULT } else { 1.0 };
        cam.transform.position += forward * (wheel * WHEEL_DOLLY * step_mul * 0.01);
        cam.update_matrices();
    }

    if frame_pressed {
        if let Some(target) = current_selection_position(app_state) {
            if let Some(cam) = app_state.camera.as_mut() {
                frame_target(cam, target);
            }
        }
    }
}

fn current_selection_position(app_state: &AppState) -> Option<Vector3<f32>> {
    let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return None; };
    match &root.editor.selection {
        Selection::SceneObject(uuid) => {
            app_state.objects.iter()
                .find(|o| o.get_unique_id() == *uuid)
                .map(|o| o.transform.position)
        }
        Selection::Light(idx) => {
            app_state.light.get(*idx).map(|l| Vector3::from(l.position))
        }
        Selection::ParticleInstance(uuid) => {
            root.project.as_ref()
                .and_then(|p| p.scenes.get(p.active_scene_index))
                .and_then(|s| s.particle_instances.iter().find(|i| i.uuid == *uuid))
                .map(|i| Vector3::from(i.position))
        }
        _ => None,
    }
}

fn frame_target(cam: &mut Camera, target: Vector3<f32>) {
    // Position camera 5 units back along its current forward, looking at target.
    let current_forward = Vector3::from(cam.calculate_direction_vector());
    let distance = 5.0;
    let new_pos = target - current_forward * distance;
    cam.transform.position = new_pos;
    // Aim at target — compute pitch/yaw from (target - new_pos).
    let look = (target - new_pos).normalize();
    let yaw = (-look.x).atan2(-look.z);
    let pitch = look.y.asin();
    cam.transform.rotation.x = pitch;
    cam.transform.rotation.y = yaw;
    cam.transform.rotation.z = 0.0;
    cam.update_matrices();
}
