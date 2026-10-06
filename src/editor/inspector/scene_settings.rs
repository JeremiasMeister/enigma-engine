use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::{EditorRoot, ResourceKind};
use crate::editor::widgets;

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

    let mut value = current_skybox;
    if widgets::asset_picker(ui, "Skybox texture", &mut value, &textures, "Default skybox") {
        if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = r.project.as_mut() {
                project.skybox = value;
                r.editor.dirty = true;
            }
        }
    }
}
