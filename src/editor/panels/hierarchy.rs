use egui::Ui;
use enigma_3d::AppState;

use crate::editor::state::{EditorRoot, Selection};

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let project_loaded = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.project.is_some())
        .unwrap_or(false);

    if !project_loaded {
        ui.label("No project loaded");
        return;
    }

    ui.heading("Hierarchy");
    ui.separator();

    let object_rows: Vec<(uuid::Uuid, String)> = app_state.objects.iter()
        .map(|o| (o.get_unique_id(), o.name.clone()))
        .collect();
    let light_rows: Vec<(usize, String)> = app_state.light.iter().enumerate()
        .map(|(i, l)| (i, format!("Light {} @ ({:.1}, {:.1}, {:.1})", i, l.position[0], l.position[1], l.position[2])))
        .collect();
    let has_camera = app_state.camera.is_some();

    let current_selection = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.selection.clone())
        .unwrap_or(Selection::None);

    let mut new_selection: Option<Selection> = None;

    egui::CollapsingHeader::new(format!("Objects ({})", object_rows.len()))
        .default_open(true)
        .show(ui, |ui| {
            for (uuid, name) in &object_rows {
                let selected = matches!(current_selection, Selection::SceneObject(s) if s == *uuid);
                if ui.selectable_label(selected, name).clicked() {
                    new_selection = Some(Selection::SceneObject(*uuid));
                }
            }
        });

    egui::CollapsingHeader::new(format!("Lights ({})", light_rows.len()))
        .default_open(true)
        .show(ui, |ui| {
            for (idx, name) in &light_rows {
                let selected = matches!(current_selection, Selection::Light(s) if s == *idx);
                if ui.selectable_label(selected, name).clicked() {
                    new_selection = Some(Selection::Light(*idx));
                }
            }
        });

    egui::CollapsingHeader::new("Camera")
        .default_open(true)
        .show(ui, |ui| {
            if has_camera {
                let selected = matches!(current_selection, Selection::Camera);
                if ui.selectable_label(selected, "Camera").clicked() {
                    new_selection = Some(Selection::Camera);
                }
            } else {
                ui.label("(no camera)");
            }
        });

    if let Some(sel) = new_selection {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.selection = sel;
        }
    }
}
