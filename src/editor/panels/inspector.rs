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
            inspector::scene_settings::draw(ui, app_state);
            ui.separator();
            ui.label("Select something in the hierarchy or resource browser to inspect it.");
        }
        Selection::SceneObject(uuid) => {
            inspector::transform::draw_for_object(ui, app_state, uuid);
            inspector::mesh_material::draw(ui, app_state, uuid);
            inspector::animation::draw(ui, app_state, uuid);
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
        Selection::Particle(uuid) => {
            inspector::particle_editor::draw(ui, app_state, uuid);
        }
        Selection::ParticleInstance(uuid) => {
            inspector::particle_instance::draw(ui, app_state, uuid);
        }
        Selection::Terrain => {
            inspector::terrain_editor::draw(ui, app_state);
        }
    }
}
