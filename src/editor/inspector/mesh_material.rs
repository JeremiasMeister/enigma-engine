use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::EditorRoot;

pub fn draw(ui: &mut Ui, app_state: &mut AppState, object_uuid: Uuid) {
    egui::CollapsingHeader::new("Mesh & Material").default_open(true).show(ui, |ui| {
        ui.label("Mesh: (from spawn)");

        let (materials, scene_uuid, current): (Vec<(Uuid, String)>, Option<Uuid>, Option<Uuid>) = {
            let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else {
                return;
            };
            let Some(project) = root.project.as_ref() else { return; };
            let mats: Vec<(Uuid, String)> = project.materials.iter()
                .map(|m| (m.uuid, m.name.clone()))
                .collect();
            let scene_uuid = project.scenes.get(project.active_scene_index).map(|s| s.uuid);
            let current = scene_uuid.and_then(|s| project.get_assignment(s, object_uuid));
            (mats, scene_uuid, current)
        };

        let Some(scene_uuid) = scene_uuid else { return; };

        let current_label = current
            .and_then(|u| materials.iter().find(|(uu, _)| *uu == u).map(|(_, n)| n.clone()))
            .unwrap_or_else(|| "(none)".to_string());

        let mut new_assignment: Option<Option<Uuid>> = None;
        ui.horizontal(|ui| {
            ui.label("Material:");
            egui::ComboBox::from_id_source("material_assign")
                .selected_text(current_label)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(current.is_none(), "(none)").clicked() {
                        new_assignment = Some(None);
                    }
                    for (uuid, name) in &materials {
                        let selected = current == Some(*uuid);
                        if ui.selectable_label(selected, name).clicked() {
                            new_assignment = Some(Some(*uuid));
                        }
                    }
                });
        });

        if let Some(value) = new_assignment {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(project) = root.project.as_mut() {
                    match value {
                        Some(mat_uuid) => project.set_assignment(scene_uuid, object_uuid, mat_uuid),
                        None => project.clear_assignment(scene_uuid, object_uuid),
                    }
                    root.editor.dirty = true;
                }
            }
        }
    });
}
