use egui::Ui;
use enigma_3d::AppState;

use crate::editor::state::EditorRoot;

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let rect = ui.max_rect();
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.viewport_rect = Some(rect);
    }
}
