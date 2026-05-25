use egui::Ui;
use enigma_3d::AppState;

use crate::editor::actions::{self, LightTemplate, ObjectTemplate};
use crate::editor::state::{EditorRoot, Selection};

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let project_loaded = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.project.is_some())
        .unwrap_or(false);

    if !project_loaded {
        ui.label("No project loaded");
        return;
    }

    ui.heading("Hierarchy");
    ui.separator();

    let object_rows: Vec<(uuid::Uuid, String)> = app_state.objects.iter()
        .map(|o| (o.get_unique_id(), o.name.clone()))
        .collect();
    let light_rows: Vec<(usize, String)> = app_state.light.iter().enumerate()
        .map(|(i, l)| (i, format!("Light {} @ ({:.1}, {:.1}, {:.1})", i, l.position[0], l.position[1], l.position[2])))
        .collect();
    let has_camera = app_state.camera.is_some();

    let current_selection = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.selection.clone())
        .unwrap_or(Selection::None);

    let mut new_selection: Option<Selection> = None;

    let mut spawn_object: Option<ObjectTemplate> = None;
    egui::CollapsingHeader::new(format!("Objects ({})", object_rows.len()))
        .default_open(true)
        .show(ui, |ui| {
            ui.menu_button("+ Add", |ui| {
                if ui.button("Empty").clicked() {
                    spawn_object = Some(ObjectTemplate::Empty);
                    ui.close_menu();
                }
                if ui.button("Cube").clicked() {
                    spawn_object = Some(ObjectTemplate::Cube);
                    ui.close_menu();
                }
                if ui.button("Sphere").clicked() {
                    spawn_object = Some(ObjectTemplate::Sphere);
                    ui.close_menu();
                }
            });
            for (uuid, name) in &object_rows {
                let selected = matches!(current_selection, Selection::SceneObject(s) if s == *uuid);
                if ui.selectable_label(selected, name).clicked() {
                    new_selection = Some(Selection::SceneObject(*uuid));
                }
            }
        });

    let mut spawn_light: Option<LightTemplate> = None;
    egui::CollapsingHeader::new(format!("Lights ({})", light_rows.len()))
        .default_open(true)
        .show(ui, |ui| {
            ui.menu_button("+ Add", |ui| {
                if ui.button("Directional").clicked() {
                    spawn_light = Some(LightTemplate::Directional);
                    ui.close_menu();
                }
                if ui.button("Point").clicked() {
                    spawn_light = Some(LightTemplate::Point);
                    ui.close_menu();
                }
                if ui.button("Ambient").clicked() {
                    spawn_light = Some(LightTemplate::Ambient);
                    ui.close_menu();
                }
            });
            for (idx, name) in &light_rows {
                let selected = matches!(current_selection, Selection::Light(s) if s == *idx);
                if ui.selectable_label(selected, name).clicked() {
                    new_selection = Some(Selection::Light(*idx));
                }
            }
        });

    egui::CollapsingHeader::new("Camera")
        .default_open(true)
        .show(ui, |ui| {
            if has_camera {
                let selected = matches!(current_selection, Selection::Camera);
                if ui.selectable_label(selected, "Camera").clicked() {
                    new_selection = Some(Selection::Camera);
                }
            } else {
                ui.label("(no camera)");
            }
        });

    if let Some(t) = spawn_object {
        actions::add_object(app_state, t);
    }
    if let Some(t) = spawn_light {
        actions::add_light(app_state, t);
    }

    if let Some(sel) = new_selection {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.selection = sel;
        }
    }
}
