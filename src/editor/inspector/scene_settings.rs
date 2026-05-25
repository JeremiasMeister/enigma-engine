use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::{EditorRoot, ResourceKind};

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    ui.heading("Scene Settings");
    ui.separator();

    let (textures, current_skybox) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = root.project.as_ref() else { return; };
        let textures: Vec<(Uuid, String)> = project.manifest.iter()
            .filter(|e| e.kind == ResourceKind::Texture)
            .map(|e| (e.uuid, e.name.clone()))
            .collect();
        (textures, project.skybox)
    };

    let current_label = current_skybox
        .and_then(|u| textures.iter().find(|(uu, _)| *uu == u).map(|(_, n)| n.clone()))
        .unwrap_or_else(|| "(default)".to_string());

    let mut new_choice: Option<Option<Uuid>> = None;
    ui.horizontal(|ui| {
        ui.label("Skybox texture:");
        egui::ComboBox::from_id_source("skybox_picker")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                if ui.selectable_label(current_skybox.is_none(), "(default)").clicked() {
                    new_choice = Some(None);
                }
                for (uuid, name) in &textures {
                    let selected = current_skybox == Some(*uuid);
                    if ui.selectable_label(selected, name).clicked() {
                        new_choice = Some(Some(*uuid));
                    }
                }
            });
    });

    if let Some(value) = new_choice {
        if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = r.project.as_mut() {
                project.skybox = value;
                r.editor.dirty = true;
            }
        }
    }
}
