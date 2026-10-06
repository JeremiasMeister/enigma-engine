use egui::Ui;
use enigma_3d::AppState;

use crate::editor::inspector;
use crate::editor::state::{EditorRoot, Selection};

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let selection = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else {
            return;
        };
        root.editor.selection.clone()
    };

    let project = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|root| root.project.as_ref());
    let subject = match &selection {
        Selection::None => "Scene settings".to_owned(),
        Selection::SceneObject(uuid) => app_state
            .objects
            .iter()
            .find(|object| object.get_unique_id() == *uuid)
            .map(|object| object.name.clone())
            .unwrap_or_else(|| "Scene object".into()),
        Selection::Light(index) => format!("Light {index}"),
        Selection::AmbientLight => "Ambient light".into(),
        Selection::Camera => "Scene camera".into(),
        Selection::Material(uuid) => project
            .and_then(|p| p.materials.iter().find(|item| item.uuid == *uuid))
            .map(|item| item.name.clone())
            .unwrap_or_else(|| "Material".into()),
        Selection::Resource(uuid) => project
            .and_then(|p| p.manifest.iter().find(|item| item.uuid == *uuid))
            .map(|item| item.name.clone())
            .unwrap_or_else(|| "Asset".into()),
        Selection::Particle(uuid) => project
            .and_then(|p| p.particle_systems.iter().find(|item| item.uuid == *uuid))
            .map(|item| item.config.name.clone())
            .unwrap_or_else(|| "Particle system".into()),
        Selection::ParticleInstance(_) => "Particle instance".into(),
        Selection::Terrain => "Terrain".into(),
    };
    crate::editor::theme::panel_heading(ui, "Inspector", &subject);
    ui.spacing_mut().item_spacing.y = 7.0;
    ui.spacing_mut().text_edit_width = (ui.available_width() - 60.0).max(100.0);

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
