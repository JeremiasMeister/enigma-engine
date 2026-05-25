use egui::Ui;
use enigma_3d::AppState;
use rfd::FileDialog;

use crate::editor::actions;
use crate::editor::state::{EditorRoot, Modal, ProjectState};
use crate::project;

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.menu_button("File", |ui| {
            if ui.button("New Project").clicked() {
                if let Some(path) = pick_folder() {
                    if let Err(e) = project::try_new_project(&path, app_state) {
                        eprintln!("new project failed: {e}");
                    }
                }
                ui.close_menu();
            }
            if ui.button("Open Project").clicked() {
                if let Some(path) = pick_file("json") {
                    project::start_open_project(&path, app_state);
                }
                ui.close_menu();
            }
            if ui.button("Save Project").clicked() {
                project::start_save_project_only(app_state);
                ui.close_menu();
            }
            if ui.button("Save Scene").clicked() {
                project::start_save_scene_and_project(app_state);
                ui.close_menu();
            }
        });

        scene_menu(ui, app_state);

        let project_loaded = current_project_clone(app_state).is_some();
        let busy = actions::is_busy(app_state);
        let enabled = project_loaded && !busy;

        ui.add_enabled_ui(enabled, |ui| {
            if ui.button("Play").clicked() {
                actions::run_project(app_state);
            }
            if ui.button("Debug Build").clicked() {
                actions::build_project(app_state, false);
            }
            if ui.button("Release Build").clicked() {
                actions::build_project(app_state, true);
            }
            if ui.button("Update Dependencies").clicked() {
                actions::update_dependencies(app_state);
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") {
                if let Some(job) = root.editor.job.as_ref() {
                    let elapsed = job.started_at.elapsed().as_secs_f32();
                    ui.add(egui::Spinner::new());
                    ui.label(format!("{} — {:.1}s", job.label, elapsed));
                } else if let Some(last) = root.editor.last_job.as_ref() {
                    let marker = if last.success { "✓" } else { "✗" };
                    let dur = last.duration.as_secs_f32();
                    ui.label(format!("{} {} ({:.1}s)", marker, last.label, dur));
                }
                if let Some(p) = &root.project {
                    if root.editor.dirty {
                        ui.label("•");
                    }
                    if let Some(scene) = p.scenes.get(p.active_scene_index) {
                        ui.label(format!("scene: {}", scene.name));
                    }
                    ui.label(format!("project: {}", p.name));
                } else {
                    ui.label("no project");
                }
            }
        });
    });
}

fn scene_menu(ui: &mut Ui, app_state: &mut AppState) {
    let project_clone = current_project_clone(app_state);
    let Some(project) = project_clone else { return; };

    ui.menu_button("Scene", |ui| {
        if ui.button("Save Scene").clicked() {
            project::start_save_scene_and_project(app_state);
            ui.close_menu();
        }
        if ui.button("New Scene…").clicked() {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                root.editor.modal = Some(Modal::NewSceneName(String::new()));
            }
            ui.close_menu();
        }
        let mut switch_to: Option<usize> = None;
        ui.menu_button("Switch Scene", |ui| {
            for (idx, s) in project.scenes.iter().enumerate() {
                let active = idx == project.active_scene_index;
                let mut label = s.name.clone();
                if active { label.push_str(" (active)"); }
                if ui.button(&label).clicked() {
                    switch_to = Some(idx);
                    ui.close_menu();
                }
            }
        });
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
        if ui.button("Set Current as Startup").clicked() {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(proj) = root.project.as_mut() {
                    proj.startup_scene_index = proj.active_scene_index;
                }
            }
            ui.close_menu();
        }
    });
}

fn current_project_clone(app_state: &mut AppState) -> Option<ProjectState> {
    app_state.get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.clone())
}


fn pick_folder() -> Option<String> {
    FileDialog::new()
        .pick_folder()
        .map(|p| p.to_string_lossy().into_owned())
}

fn pick_file(filter: &str) -> Option<String> {
    FileDialog::new()
        .add_filter(filter, &[filter])
        .pick_file()
        .map(|p| p.to_string_lossy().into_owned())
}
