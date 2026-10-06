use egui::Ui;
use crate::editor::widgets;
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::EditorRoot;

pub fn draw(ui: &mut Ui, app_state: &mut AppState, instance_uuid: Uuid) {
    let (mut instance_clone, defs) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = root.project.as_ref() else { return; };
        let active = project.active_scene_index;
        let Some(scene) = project.scenes.get(active) else { return; };
        let Some(inst) = scene.particle_instances.iter().find(|i| i.uuid == instance_uuid) else {
            ui.label("(particle instance not found)");
            return;
        };
        let defs: Vec<(Uuid, String)> = project.particle_systems.iter()
            .map(|d| (d.uuid, d.config.name.clone()))
            .collect();
        (inst.clone(), defs)
    };

    let mut changed = false;
    ui.heading("Particle Instance");
    ui.horizontal(|ui| {
        ui.label("Name");
        changed |= ui.text_edit_singleline(&mut instance_clone.name).changed();
    });

    egui::CollapsingHeader::new("Transform").default_open(true).show(ui, |ui| {
        changed |= widgets::vector3(ui, "Position", &mut instance_clone.position, 0.1);
    });

    ui.label("Particle system");
    changed |= widgets::searchable_combo(ui, "particle_inst_def", &mut instance_clone.def_uuid, &defs);

    if changed {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = root.project.as_mut() {
                let active = project.active_scene_index;
                if let Some(scene) = project.scenes.get_mut(active) {
                    if let Some(inst) = scene.particle_instances.iter_mut().find(|i| i.uuid == instance_uuid) {
                        *inst = instance_clone;
                    }
                }
                root.editor.dirty = true;
            }
        }
    }
}
