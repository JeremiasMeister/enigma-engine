use egui::{DragValue, Ui};
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
        ui.label("Position");
        ui.horizontal(|ui| {
            changed |= ui.add(DragValue::new(&mut instance_clone.position[0]).speed(0.1).prefix("x ")).changed();
            changed |= ui.add(DragValue::new(&mut instance_clone.position[1]).speed(0.1).prefix("y ")).changed();
            changed |= ui.add(DragValue::new(&mut instance_clone.position[2]).speed(0.1).prefix("z ")).changed();
        });
    });

    ui.horizontal(|ui| {
        ui.label("Def");
        let current = defs.iter().find(|(u, _)| *u == instance_clone.def_uuid)
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| "(missing)".into());
        egui::ComboBox::from_id_source("particle_inst_def").selected_text(current).show_ui(ui, |ui| {
            for (uuid, name) in &defs {
                let sel = instance_clone.def_uuid == *uuid;
                if ui.selectable_label(sel, name).clicked() {
                    instance_clone.def_uuid = *uuid;
                    changed = true;
                }
            }
        });
    });

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
