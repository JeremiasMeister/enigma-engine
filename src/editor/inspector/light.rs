use egui::Ui;
use enigma_3d::AppState;

use crate::editor::{state::EditorRoot, widgets};

pub fn draw(ui: &mut Ui, app_state: &mut AppState, index: usize) {
    let Some(light) = app_state.light.get_mut(index) else {
        return;
    };
    if draw_props(ui, light, false) {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.dirty = true;
        }
    }
}

pub fn draw_ambient(ui: &mut Ui, app_state: &mut AppState) {
    let Some(light) = app_state.ambient_light.as_mut() else {
        ui.label("No ambient light in this scene.");
        return;
    };
    if draw_props(ui, light, true) {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.dirty = true;
        }
    }
}

fn draw_props(ui: &mut Ui, light: &mut enigma_3d::light::Light, ambient: bool) -> bool {
    let mut changed = false;
    let label = if ambient { "Ambient light" } else { "Light" };
    egui::CollapsingHeader::new(label)
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Color");
                changed |= ui.color_edit_button_rgb(&mut light.color).changed();
            });
            changed |=
                widgets::slider_with(ui, "Intensity", &mut light.intensity, 0.0..=20.0, "", true)
                    .changed();

            if !ambient {
                changed |= ui
                    .checkbox(&mut light.cast_shadow, "Cast shadows")
                    .changed();
                let mut directional = light.is_directional();
                if ui.checkbox(&mut directional, "Directional").changed() {
                    light.direction = if directional {
                        [0.0, -1.0, 0.0]
                    } else {
                        [0.0, 0.0, 0.0]
                    };
                    changed = true;
                }
                if directional {
                    changed |= widgets::vector3(ui, "Direction", &mut light.direction, 0.01);
                }
            }
        });
    changed
}
