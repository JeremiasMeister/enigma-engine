use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::actions::{self, LightTemplate, ObjectTemplate};
use crate::editor::state::{EditorRoot, Modal, PendingDelete, RenameTarget, ResourceKind, Selection};

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

    let object_rows: Vec<(Uuid, String)> = app_state.objects.iter()
        .map(|o| (o.get_unique_id(), o.name.clone()))
        .collect();
    let light_rows: Vec<(usize, String)> = app_state.light.iter().enumerate()
        .map(|(i, l)| (i, format!("Light {} @ ({:.1}, {:.1}, {:.1})", i, l.position[0], l.position[1], l.position[2])))
        .collect();
    let has_ambient = app_state.ambient_light.is_some();
    let has_camera = app_state.camera.is_some();

    let (current_selection, renaming) = match app_state.get_state_data_value::<EditorRoot>("editor") {
        Some(r) => (r.editor.selection.clone(), r.editor.renaming.clone()),
        None => (Selection::None, None),
    };

    let mut new_selection: Option<Selection> = None;
    let mut delete_request: Option<PendingDelete> = None;
    let mut rename_start: Option<RenameTarget> = None;
    let mut rename_commit: Option<RenameTarget> = None;
    let mut rename_cancel = false;

    let model_rows: Vec<(Uuid, String)> = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| p.manifest.iter()
            .filter(|e| e.kind == ResourceKind::Model)
            .map(|e| (e.uuid, e.name.clone()))
            .collect())
        .unwrap_or_default();

    let mut spawn_object: Option<ObjectTemplate> = None;
    let mut spawn_model: Option<Uuid> = None;
    egui::CollapsingHeader::new(format!("Objects ({})", object_rows.len()))
        .default_open(true)
        .show(ui, |ui| {
            ui.menu_button("+ Add", |ui| {
                if ui.button("Empty").clicked() { spawn_object = Some(ObjectTemplate::Empty); ui.close_menu(); }
                if ui.button("Cube").clicked() { spawn_object = Some(ObjectTemplate::Cube); ui.close_menu(); }
                if ui.button("Sphere").clicked() { spawn_object = Some(ObjectTemplate::Sphere); ui.close_menu(); }
                ui.menu_button("From Model…", |ui| {
                    if model_rows.is_empty() {
                        ui.label("(import a model first)");
                    } else {
                        for (uuid, name) in &model_rows {
                            if ui.button(name).clicked() {
                                spawn_model = Some(*uuid);
                                ui.close_menu();
                            }
                        }
                    }
                });
            });
            for (uuid, name) in &object_rows {
                let selected = matches!(current_selection, Selection::SceneObject(s) if s == *uuid);
                let renaming_this = matches!(&renaming, Some(RenameTarget::SceneObject { uuid: u, .. }) if u == uuid);
                ui.horizontal(|ui| {
                    if renaming_this {
                        if let Some(RenameTarget::SceneObject { uuid, draft }) = &renaming {
                            let mut draft = draft.clone();
                            let response = ui.text_edit_singleline(&mut draft);
                            response.request_focus();
                            if response.lost_focus() {
                                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    rename_commit = Some(RenameTarget::SceneObject { uuid: *uuid, draft: draft.clone() });
                                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                    rename_cancel = true;
                                } else {
                                    rename_commit = Some(RenameTarget::SceneObject { uuid: *uuid, draft: draft.clone() });
                                }
                            } else {
                                rename_start = Some(RenameTarget::SceneObject { uuid: *uuid, draft });
                            }
                        }
                    } else {
                        let resp = ui.selectable_label(selected, name);
                        if resp.clicked() {
                            new_selection = Some(Selection::SceneObject(*uuid));
                        }
                        if resp.double_clicked() {
                            rename_start = Some(RenameTarget::SceneObject { uuid: *uuid, draft: name.clone() });
                        }
                    }
                    if ui.small_button("×").on_hover_text("Delete").clicked() {
                        delete_request = Some(PendingDelete::SceneObject(*uuid));
                    }
                });
            }
        });

    let mut spawn_light: Option<LightTemplate> = None;
    egui::CollapsingHeader::new(format!("Lights ({})", light_rows.len() + has_ambient as usize))
        .default_open(true)
        .show(ui, |ui| {
            ui.menu_button("+ Add", |ui| {
                if ui.button("Directional").clicked() { spawn_light = Some(LightTemplate::Directional); ui.close_menu(); }
                if ui.button("Point").clicked() { spawn_light = Some(LightTemplate::Point); ui.close_menu(); }
                if !has_ambient && ui.button("Ambient").clicked() {
                    spawn_light = Some(LightTemplate::Ambient);
                    ui.close_menu();
                }
            });
            for (idx, name) in &light_rows {
                let selected = matches!(current_selection, Selection::Light(s) if s == *idx);
                ui.horizontal(|ui| {
                    if ui.selectable_label(selected, name).clicked() {
                        new_selection = Some(Selection::Light(*idx));
                    }
                    if ui.small_button("×").on_hover_text("Delete").clicked() {
                        delete_request = Some(PendingDelete::Light(*idx));
                    }
                });
            }
            // Always render the ambient slot — present if Some, "+ Add" inline if None.
            // This avoids a one-frame delay before users see the row after spawning.
            ui.horizontal(|ui| {
                if has_ambient {
                    let selected = matches!(current_selection, Selection::AmbientLight);
                    if ui.selectable_label(selected, "Ambient").clicked() {
                        new_selection = Some(Selection::AmbientLight);
                    }
                    if ui.small_button("×").on_hover_text("Delete ambient light").clicked() {
                        delete_request = Some(PendingDelete::AmbientLight);
                    }
                } else {
                    ui.weak("Ambient (none)");
                    if ui.small_button("+ Add").clicked() {
                        spawn_light = Some(LightTemplate::Ambient);
                    }
                }
            });
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

    let delete_key = ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));
    if delete_key && renaming.is_none() {
        match &current_selection {
            Selection::SceneObject(u) => delete_request = Some(PendingDelete::SceneObject(*u)),
            Selection::Light(i) => delete_request = Some(PendingDelete::Light(*i)),
            Selection::AmbientLight => delete_request = Some(PendingDelete::AmbientLight),
            _ => {}
        }
    }

    if let Some(t) = spawn_object { actions::add_object(app_state, t); }
    if let Some(uuid) = spawn_model { actions::spawn_from_model(app_state, uuid); }
    if let Some(t) = spawn_light { actions::add_light(app_state, t); }

    if let Some(sel) = new_selection {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.selection = sel;
        }
    }

    if let Some(req) = delete_request {
        let label = match &req {
            PendingDelete::SceneObject(_) => "object",
            PendingDelete::Light(_) => "light",
            PendingDelete::AmbientLight => "ambient light",
            _ => "item",
        };
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.modal = Some(Modal::ConfirmDelete { label: label.to_string(), pending: req });
        }
    }

    if rename_cancel {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.renaming = None;
        }
    } else if let Some(rc) = rename_commit {
        commit_rename(app_state, rc);
    } else if let Some(rs) = rename_start {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.renaming = Some(rs);
        }
    }
}

fn commit_rename(app_state: &mut AppState, target: RenameTarget) {
    match target {
        RenameTarget::SceneObject { uuid, draft } => {
            if let Some(obj) = app_state.objects.iter_mut().find(|o| o.get_unique_id() == uuid) {
                obj.name = draft;
            }
        }
        _ => {}
    }
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.renaming = None;
        root.editor.dirty = true;
    }
}
