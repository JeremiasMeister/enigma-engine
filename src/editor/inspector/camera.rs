use egui::{DragValue, Ui};
use enigma_3d::AppState;

use crate::editor::{state::EditorRoot, widgets};

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let Some(cam) = app_state.camera.as_mut() else {
        ui.label("No camera in this scene.");
        return;
    };
    let mut changed = false;
    egui::CollapsingHeader::new("Transform")
        .default_open(true)
        .show(ui, |ui| {
            let mut position = cam.get_position();
            if widgets::vector3(ui, "Position", &mut position, 0.05) {
                cam.set_position(position);
                changed = true;
            }
            let mut rotation = cam.get_rotation();
            if widgets::vector3(ui, "Rotation (degrees)", &mut rotation, 1.0) {
                cam.set_rotation(rotation);
                changed = true;
            }
        });

    egui::CollapsingHeader::new("Lens")
        .default_open(true)
        .show(ui, |ui| {
            changed |= widgets::angle_slider(ui, "Field of view", &mut cam.fov, 5.0..=150.0);
            ui.label("Clipping distance");
            ui.horizontal(|ui| {
                changed |= ui
                    .add(DragValue::new(&mut cam.near).speed(0.01).prefix("Near "))
                    .changed();
                changed |= ui
                    .add(DragValue::new(&mut cam.far).speed(1.0).prefix("Far "))
                    .changed();
            });
        });
    if changed {
        cam.update_matrices();
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.dirty = true;
        }
    }
}
