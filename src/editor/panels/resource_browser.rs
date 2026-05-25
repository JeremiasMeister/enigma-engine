use egui::Ui;
use enigma_3d::AppState;
use rfd::AsyncFileDialog;

use crate::editor::state::{EditorRoot, MaterialDef, Modal, ResourceKind, ResourceTab, Selection};
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
            ResourceTab::Materials, ResourceTab::Scenes, ResourceTab::Audio, ResourceTab::Other,
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

    match current_tab {
        ResourceTab::Models => list_kind(ui, app_state, ResourceKind::Model),
        ResourceTab::Textures => list_kind(ui, app_state, ResourceKind::Texture),
        ResourceTab::Shaders => list_kind(ui, app_state, ResourceKind::Shader),
        ResourceTab::Audio => list_kind(ui, app_state, ResourceKind::Audio),
        ResourceTab::Other => list_kind(ui, app_state, ResourceKind::Other),
        ResourceTab::Materials => list_materials(ui, app_state),
        ResourceTab::Scenes => list_scenes(ui, app_state),
    }
}

fn list_kind(ui: &mut Ui, app_state: &mut AppState, kind: ResourceKind) {
    let mut import_clicked = false;
    if ui.button("+ Import").clicked() { import_clicked = true; }

    let (rows, current_sel): (Vec<(uuid::Uuid, String)>, Selection) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else {
            return;
        };
        let rows = root.project.as_ref().map(|p| p.manifest.iter()
            .filter(|e| e.kind == kind)
            .map(|e| (e.uuid, e.name.clone()))
            .collect()).unwrap_or_default();
        (rows, root.editor.selection.clone())
    };

    let mut new_sel: Option<Selection> = None;
    for (uuid, name) in &rows {
        let selected = matches!(&current_sel, Selection::Resource(u) if u == uuid);
        if ui.selectable_label(selected, name).clicked() {
            new_sel = Some(Selection::Resource(*uuid));
        }
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

    if let Some(s) = new_sel {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.selection = s;
        }
    }
}

fn list_materials(ui: &mut Ui, app_state: &mut AppState) {
    let mut create_clicked = false;
    if ui.button("+ New").clicked() { create_clicked = true; }

    let (rows, current_sel) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else {
            return;
        };
        let rows: Vec<(uuid::Uuid, String)> = root.project.as_ref()
            .map(|p| p.materials.iter().map(|m| (m.uuid, m.name.clone())).collect())
            .unwrap_or_default();
        (rows, root.editor.selection.clone())
    };

    let mut new_sel: Option<Selection> = None;
    for (uuid, name) in &rows {
        let selected = matches!(&current_sel, Selection::Material(u) if u == uuid);
        if ui.selectable_label(selected, name).clicked() {
            new_sel = Some(Selection::Material(*uuid));
        }
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
    } else if let Some(s) = new_sel {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.selection = s;
        }
    }
}

fn list_scenes(ui: &mut Ui, app_state: &mut AppState) {
    let mut new_clicked = false;
    if ui.button("+ New Scene").clicked() { new_clicked = true; }

    let (scenes, active_index, startup_index) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(p) = root.project.as_ref() else { return; };
        let scenes: Vec<String> = p.scenes.iter().map(|s| s.name.clone()).collect();
        (scenes, p.active_scene_index, p.startup_scene_index)
    };

    let mut switch_to: Option<usize> = None;
    for (idx, name) in scenes.iter().enumerate() {
        let mut label = name.clone();
        if idx == startup_index { label.push_str(" (startup)"); }
        if idx == active_index { label.push_str(" — active"); }
        if ui.selectable_label(idx == active_index, &label).double_clicked() {
            switch_to = Some(idx);
        }
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
    let (tx, rx) = std::sync::mpsc::channel();
    let owned: Vec<String> = exts.iter().map(|s| s.to_string()).collect();
    async_std::task::spawn(async move {
        let mut dialog = AsyncFileDialog::new();
        if !owned.is_empty() {
            let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
            dialog = dialog.add_filter("file", &refs);
        }
        if let Some(p) = dialog.pick_file().await {
            let _ = tx.send(p.path().to_string_lossy().into_owned());
        } else {
            let _ = tx.send(String::new());
        }
    });
    match rx.recv() {
        Ok(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}
