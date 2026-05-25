use egui::Ui;
use enigma_3d::AppState;
use rfd::FileDialog;
use uuid::Uuid;

use crate::editor::state::{
    EditorRoot, MaterialDef, Modal, ParticleSystemDef, PendingDelete, RenameTarget, ResourceKind, ResourceTab,
    Selection,
};
use crate::project;

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let (project_loaded, current_tab) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        (root.project.is_some(), root.editor.resource_browser_tab)
    };

    if !project_loaded {
        ui.label("No project loaded");
        return;
    }

    let mut new_tab: Option<ResourceTab> = None;
    ui.horizontal(|ui| {
        for tab in [
            ResourceTab::Models, ResourceTab::Textures, ResourceTab::Shaders,
            ResourceTab::Materials, ResourceTab::Particles, ResourceTab::Scenes,
            ResourceTab::Audio, ResourceTab::Other,
        ] {
            if ui.selectable_label(current_tab == tab, format!("{tab:?}")).clicked() {
                new_tab = Some(tab);
            }
        }
    });
    ui.separator();

    if let Some(t) = new_tab {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.resource_browser_tab = t;
        }
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            match current_tab {
                ResourceTab::Models => list_kind(ui, app_state, ResourceKind::Model),
                ResourceTab::Textures => list_kind(ui, app_state, ResourceKind::Texture),
                ResourceTab::Shaders => list_kind(ui, app_state, ResourceKind::Shader),
                ResourceTab::Audio => list_kind(ui, app_state, ResourceKind::Audio),
                ResourceTab::Other => list_kind(ui, app_state, ResourceKind::Other),
                ResourceTab::Materials => list_materials(ui, app_state),
                ResourceTab::Particles => list_particles(ui, app_state),
                ResourceTab::Scenes => list_scenes(ui, app_state),
            }
        });
}

fn list_kind(ui: &mut Ui, app_state: &mut AppState, kind: ResourceKind) {
    let mut import_clicked = false;
    if ui.button("+ Import").clicked() { import_clicked = true; }

    let (rows, current_sel, renaming) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let rows: Vec<(Uuid, String)> = root.project.as_ref().map(|p| p.manifest.iter()
            .filter(|e| e.kind == kind)
            .map(|e| (e.uuid, e.name.clone()))
            .collect()).unwrap_or_default();
        (rows, root.editor.selection.clone(), root.editor.renaming.clone())
    };

    let mut new_sel: Option<Selection> = None;
    let mut delete: Option<PendingDelete> = None;
    let mut rename_start: Option<RenameTarget> = None;
    let mut rename_commit: Option<RenameTarget> = None;
    let mut rename_cancel = false;

    for (uuid, name) in &rows {
        let selected = matches!(&current_sel, Selection::Resource(u) if u == uuid);
        let renaming_this = matches!(&renaming, Some(RenameTarget::Resource { uuid: u, .. }) if u == uuid);
        ui.horizontal(|ui| {
            if renaming_this {
                if let Some(RenameTarget::Resource { uuid, draft }) = &renaming {
                    let mut d = draft.clone();
                    let response = ui.text_edit_singleline(&mut d);
                    response.request_focus();
                    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    let commit_btn = ui.small_button("✓").on_hover_text("Apply").clicked();
                    let cancel_btn = ui.small_button("✗").on_hover_text("Cancel").clicked();
                    if escape || cancel_btn {
                        rename_cancel = true;
                    } else if enter || commit_btn {
                        rename_commit = Some(RenameTarget::Resource { uuid: *uuid, draft: d });
                    } else {
                        rename_start = Some(RenameTarget::Resource { uuid: *uuid, draft: d });
                    }
                }
            } else {
                let resp = ui.selectable_label(selected, name);
                if resp.clicked() { new_sel = Some(Selection::Resource(*uuid)); }
                if resp.double_clicked() {
                    rename_start = Some(RenameTarget::Resource { uuid: *uuid, draft: name.clone() });
                }
                if ui.small_button("×").on_hover_text("Delete").clicked() {
                    delete = Some(PendingDelete::Resource(*uuid));
                }
            }
        });
    }

    if import_clicked {
        let exts = filter_for(kind);
        if let Some(src) = pick_file(&exts) {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(project) = root.project.as_mut() {
                    if let Err(e) = project::resource::import(std::path::Path::new(&src), kind, project) {
                        eprintln!("import failed: {e:?}");
                    } else {
                        root.editor.dirty = true;
                    }
                }
            }
        }
    }

    finalize(app_state, new_sel, delete, "resource", rename_start, rename_commit, rename_cancel);
}

fn list_materials(ui: &mut Ui, app_state: &mut AppState) {
    let mut create_clicked = false;
    if ui.button("+ New").clicked() { create_clicked = true; }

    let (rows, current_sel, renaming) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let rows: Vec<(Uuid, String)> = root.project.as_ref()
            .map(|p| p.materials.iter().map(|m| (m.uuid, m.name.clone())).collect())
            .unwrap_or_default();
        (rows, root.editor.selection.clone(), root.editor.renaming.clone())
    };

    let mut new_sel: Option<Selection> = None;
    let mut delete: Option<PendingDelete> = None;
    let mut rename_start: Option<RenameTarget> = None;
    let mut rename_commit: Option<RenameTarget> = None;
    let mut rename_cancel = false;

    for (uuid, name) in &rows {
        let selected = matches!(&current_sel, Selection::Material(u) if u == uuid);
        let renaming_this = matches!(&renaming, Some(RenameTarget::Material { uuid: u, .. }) if u == uuid);
        ui.horizontal(|ui| {
            if renaming_this {
                if let Some(RenameTarget::Material { uuid, draft }) = &renaming {
                    let mut d = draft.clone();
                    let response = ui.text_edit_singleline(&mut d);
                    response.request_focus();
                    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    let commit_btn = ui.small_button("✓").on_hover_text("Apply").clicked();
                    let cancel_btn = ui.small_button("✗").on_hover_text("Cancel").clicked();
                    if escape || cancel_btn {
                        rename_cancel = true;
                    } else if enter || commit_btn {
                        rename_commit = Some(RenameTarget::Material { uuid: *uuid, draft: d });
                    } else {
                        rename_start = Some(RenameTarget::Material { uuid: *uuid, draft: d });
                    }
                }
            } else {
                let resp = ui.selectable_label(selected, name);
                if resp.clicked() { new_sel = Some(Selection::Material(*uuid)); }
                if resp.double_clicked() {
                    rename_start = Some(RenameTarget::Material { uuid: *uuid, draft: name.clone() });
                }
                if ui.small_button("×").on_hover_text("Delete").clicked() {
                    delete = Some(PendingDelete::Material(*uuid));
                }
            }
        });
    }

    if create_clicked {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = root.project.as_mut() {
                let name = format!("Material {}", project.materials.len() + 1);
                let mat = MaterialDef::default_pbr(name);
                let uuid = mat.uuid;
                project.materials.push(mat);
                root.editor.selection = Selection::Material(uuid);
                root.editor.dirty = true;
            }
        }
    }

    finalize(app_state, new_sel, delete, "material", rename_start, rename_commit, rename_cancel);
}

fn list_particles(ui: &mut Ui, app_state: &mut AppState) {
    let mut create_clicked = false;
    if ui.button("+ New").clicked() { create_clicked = true; }

    let (rows, current_sel, renaming) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let rows: Vec<(Uuid, String)> = root.project.as_ref()
            .map(|p| p.particle_systems.iter().map(|m| (m.uuid, m.config.name.clone())).collect())
            .unwrap_or_default();
        (rows, root.editor.selection.clone(), root.editor.renaming.clone())
    };

    let mut new_sel: Option<Selection> = None;
    let mut delete: Option<PendingDelete> = None;
    let mut rename_start: Option<RenameTarget> = None;
    let mut rename_commit: Option<RenameTarget> = None;
    let mut rename_cancel = false;

    for (uuid, name) in &rows {
        let selected = matches!(&current_sel, Selection::Particle(u) if u == uuid);
        let renaming_this = matches!(&renaming, Some(RenameTarget::Particle { uuid: u, .. }) if u == uuid);
        ui.horizontal(|ui| {
            if renaming_this {
                if let Some(RenameTarget::Particle { uuid, draft }) = &renaming {
                    let mut d = draft.clone();
                    let response = ui.text_edit_singleline(&mut d);
                    response.request_focus();
                    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    let commit_btn = ui.small_button("✓").on_hover_text("Apply").clicked();
                    let cancel_btn = ui.small_button("✗").on_hover_text("Cancel").clicked();
                    if escape || cancel_btn { rename_cancel = true; }
                    else if enter || commit_btn {
                        rename_commit = Some(RenameTarget::Particle { uuid: *uuid, draft: d });
                    } else {
                        rename_start = Some(RenameTarget::Particle { uuid: *uuid, draft: d });
                    }
                }
            } else {
                let resp = ui.selectable_label(selected, name);
                if resp.clicked() { new_sel = Some(Selection::Particle(*uuid)); }
                if resp.double_clicked() {
                    rename_start = Some(RenameTarget::Particle { uuid: *uuid, draft: name.clone() });
                }
                if ui.small_button("×").on_hover_text("Delete").clicked() {
                    delete = Some(PendingDelete::Particle(*uuid));
                }
            }
        });
    }

    if create_clicked {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = root.project.as_mut() {
                let name = format!("Particles {}", project.particle_systems.len() + 1);
                let default_mat = ensure_particle_default_material(project);
                let mut def = ParticleSystemDef::new_default(name);
                def.material = default_mat;
                let uuid = def.uuid;
                project.particle_systems.push(def);
                root.editor.selection = Selection::Particle(uuid);
                root.editor.dirty = true;
            }
        }
    }

    finalize(app_state, new_sel, delete, "particle system", rename_start, rename_commit, rename_cancel);
}

/// Returns the uuid of a project material suitable for particle rendering.
/// Reuses an existing "Particles Default" material if present, otherwise creates one.
fn ensure_particle_default_material(project: &mut crate::editor::state::ProjectState) -> Option<Uuid> {
    if let Some(m) = project.materials.iter().find(|m| m.name == "Particles Default") {
        return Some(m.uuid);
    }
    let mut def = MaterialDef::default_pbr("Particles Default".to_string());
    // additive-friendly: white emissive default, transparent enabled
    def.color = [1.0, 1.0, 1.0];
    def.emissive_strength = 1.0;
    def.transparent = true;
    def.transparency_strength = 1.0;
    let uuid = def.uuid;
    project.materials.push(def);
    Some(uuid)
}

fn list_scenes(ui: &mut Ui, app_state: &mut AppState) {
    let mut new_clicked = false;
    if ui.button("+ New Scene").clicked() { new_clicked = true; }

    let (scenes, active_index, startup_index, renaming) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(p) = root.project.as_ref() else { return; };
        let scenes: Vec<String> = p.scenes.iter().map(|s| s.name.clone()).collect();
        (scenes, p.active_scene_index, p.startup_scene_index, root.editor.renaming.clone())
    };

    let mut switch_to: Option<usize> = None;
    let mut delete: Option<PendingDelete> = None;
    let mut rename_start: Option<RenameTarget> = None;
    let mut rename_commit: Option<RenameTarget> = None;
    let mut rename_cancel = false;

    for (idx, name) in scenes.iter().enumerate() {
        let renaming_this = matches!(&renaming, Some(RenameTarget::Scene { index: i, .. }) if *i == idx);
        ui.horizontal(|ui| {
            if renaming_this {
                if let Some(RenameTarget::Scene { index, draft }) = &renaming {
                    let mut d = draft.clone();
                    let response = ui.text_edit_singleline(&mut d);
                    response.request_focus();
                    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    let commit_btn = ui.small_button("✓").on_hover_text("Apply").clicked();
                    let cancel_btn = ui.small_button("✗").on_hover_text("Cancel").clicked();
                    if escape || cancel_btn {
                        rename_cancel = true;
                    } else if enter || commit_btn {
                        rename_commit = Some(RenameTarget::Scene { index: *index, draft: d });
                    } else {
                        rename_start = Some(RenameTarget::Scene { index: *index, draft: d });
                    }
                }
            } else {
                let mut label = name.clone();
                if idx == startup_index { label.push_str(" (startup)"); }
                if idx == active_index { label.push_str(" — active"); }
                let resp = ui.selectable_label(idx == active_index, &label);
                if resp.double_clicked() { switch_to = Some(idx); }
                if ui.small_button("⟳").on_hover_text("Rename").clicked() {
                    rename_start = Some(RenameTarget::Scene { index: idx, draft: name.clone() });
                }
                if ui.small_button("×").on_hover_text("Delete").clicked() {
                    delete = Some(PendingDelete::Scene(idx));
                }
            }
        });
    }

    if new_clicked {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.modal = Some(Modal::NewSceneName(String::new()));
        }
    }

    if let Some(idx) = switch_to {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(proj) = root.project.as_mut() {
                let mut proj_clone = proj.clone();
                if let Err(e) = crate::project::scene::switch(&mut proj_clone, app_state, idx) {
                    eprintln!("scene switch failed: {e:?}");
                } else if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                    if let Some(p) = root.project.as_mut() {
                        *p = proj_clone;
                    }
                }
            }
        }
    }

    finalize(app_state, None, delete, "scene", rename_start, rename_commit, rename_cancel);
}

fn finalize(
    app_state: &mut AppState,
    new_sel: Option<Selection>,
    delete: Option<PendingDelete>,
    kind_label: &str,
    rename_start: Option<RenameTarget>,
    rename_commit: Option<RenameTarget>,
    rename_cancel: bool,
) {
    if let Some(req) = delete {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.modal = Some(Modal::ConfirmDelete { label: kind_label.to_string(), pending: req });
        }
    }
    if let Some(s) = new_sel {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.selection = s;
        }
    }
    if rename_cancel {
        if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            r.editor.renaming = None;
        }
    } else if let Some(rc) = rename_commit {
        commit_rename(app_state, rc);
    } else if let Some(rs) = rename_start {
        if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            r.editor.renaming = Some(rs);
        }
    }
}

fn commit_rename(app_state: &mut AppState, target: RenameTarget) {
    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        if let Some(project) = r.project.as_mut() {
            match &target {
                RenameTarget::Resource { uuid, draft } => {
                    let name = draft.trim();
                    if !name.is_empty() {
                        if let Some(e) = project.manifest.iter_mut().find(|e| &e.uuid == uuid) {
                            e.name = name.to_string();
                        }
                    }
                }
                RenameTarget::Material { uuid, draft } => {
                    let name = draft.trim();
                    if !name.is_empty() {
                        if let Some(m) = project.materials.iter_mut().find(|m| &m.uuid == uuid) {
                            m.name = name.to_string();
                        }
                    }
                }
                RenameTarget::Scene { index, draft } => {
                    let name = draft.trim();
                    if !name.is_empty() {
                        if let Some(s) = project.scenes.get_mut(*index) {
                            s.name = name.to_string();
                        }
                    }
                }
                RenameTarget::Particle { uuid, draft } => {
                    let name = draft.trim();
                    if !name.is_empty() {
                        if let Some(p) = project.particle_systems.iter_mut().find(|p| &p.uuid == uuid) {
                            p.config.name = name.to_string();
                        }
                    }
                }
                _ => {}
            }
            r.editor.dirty = true;
        }
        r.editor.renaming = None;
    }
}

fn filter_for(kind: ResourceKind) -> Vec<&'static str> {
    match kind {
        ResourceKind::Model => vec!["gltf", "glb"],
        ResourceKind::Texture => vec!["png", "jpg", "jpeg", "tga", "dds", "bmp"],
        ResourceKind::Shader => vec!["glsl", "vert", "frag", "vs", "fs"],
        ResourceKind::Audio => vec!["wav", "ogg", "mp3", "flac"],
        ResourceKind::Other => vec![],
    }
}

fn pick_file(exts: &[&str]) -> Option<String> {
    let mut dialog = FileDialog::new();
    if !exts.is_empty() {
        dialog = dialog.add_filter("file", exts);
    }
    dialog.pick_file().map(|p| p.to_string_lossy().into_owned())
}
