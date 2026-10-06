use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::{state::EditorRoot, widgets};

pub fn draw_for_object(ui: &mut Ui, app_state: &mut AppState, uuid: Uuid) {
    let Some(obj) = app_state
        .objects
        .iter_mut()
        .find(|o| o.get_unique_id() == uuid)
    else {
        return;
    };
    let mut changed = false;
    egui::CollapsingHeader::new("Transform")
        .default_open(true)
        .show(ui, |ui| {
            let mut position: [f32; 3] = obj.transform.position.into();
            if widgets::vector3(ui, "Position", &mut position, 0.05) {
                obj.transform.set_position(position);
                changed = true;
            }

            let mut rotation: [f32; 3] = obj.transform.get_rotation().into();
            if widgets::vector3(ui, "Rotation (degrees)", &mut rotation, 1.0) {
                obj.transform.set_rotation(rotation);
                changed = true;
            }

            let mut scale: [f32; 3] = obj.transform.scale.into();
            if widgets::vector3(ui, "Scale", &mut scale, 0.05) {
                obj.transform.set_scale(scale);
                changed = true;
            }
        });
    if changed {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.dirty = true;
        }
    }
}

pub fn draw_for_light(ui: &mut Ui, app_state: &mut AppState, index: usize) {
    let Some(light) = app_state.light.get_mut(index) else {
        return;
    };
    let mut changed = false;
    egui::CollapsingHeader::new("Transform")
        .default_open(true)
        .show(ui, |ui| {
            changed |= widgets::vector3(ui, "Position", &mut light.position, 0.05);
        });
    if changed {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.dirty = true;
        }
    }
}
