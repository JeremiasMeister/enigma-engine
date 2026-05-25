use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::{EditorRoot, ResourceKind};

pub fn draw(ui: &mut Ui, app_state: &mut AppState, uuid: Uuid) {
    let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
    let Some(project) = root.project.as_ref() else { return; };
    let Some(entry) = project.manifest.iter().find(|e| e.uuid == uuid) else {
        ui.label("(resource not found)");
        return;
    };

    ui.label(format!("Name: {}", entry.name));
    ui.label(format!("Kind: {:?}", entry.kind));
    ui.label(format!("Path: {}", entry.relative_path));

    if let ResourceKind::Texture = entry.kind {
        let path = std::path::Path::new(&project.root_path)
            .join("src/resources")
            .join(&entry.relative_path);
        if let Ok(meta) = std::fs::metadata(&path) {
            ui.label(format!("Size: {} bytes", meta.len()));
        }
    }
}
