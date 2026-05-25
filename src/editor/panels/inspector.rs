use egui::Ui;
use enigma_3d::AppState;

use crate::editor::inspector;
use crate::editor::state::{EditorRoot, Selection};

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let selection = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        root.editor.selection.clone()
    };

    ui.heading("Inspector");
    ui.separator();

    match selection {
        Selection::None => {
            ui.label("Select something in the hierarchy or resource browser.");
        }
        Selection::SceneObject(uuid) => {
            inspector::transform::draw_for_object(ui, app_state, uuid);
            inspector::mesh_material::draw(ui, app_state, uuid);
        }
        Selection::Light(index) => {
            inspector::transform::draw_for_light(ui, app_state, index);
            inspector::light::draw(ui, app_state, index);
        }
        Selection::AmbientLight => {
            inspector::light::draw_ambient(ui, app_state);
        }
        Selection::Camera => {
            inspector::camera::draw(ui, app_state);
        }
        Selection::Material(uuid) => {
            inspector::material_editor::draw(ui, app_state, uuid);
        }
        Selection::Resource(uuid) => {
            inspector::resource_meta::draw(ui, app_state, uuid);
        }
    }
}
