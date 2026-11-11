use egui::Ui;
use enigma_3d::AppState;
use rfd::FileDialog;

use crate::editor::state::{EditorRoot, Modal, ProjectState};
use crate::editor::{actions, theme, widgets};
use crate::project;

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Enigma")
                .size(19.0)
                .strong()
                .color(theme::ACCENT),
        );
        ui.separator();
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

        ui.separator();
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            ui.selectable_value(&mut root.editor.workspace, crate::editor::state::Workspace::Scene, "3D scene");
            ui.selectable_value(&mut root.editor.workspace, crate::editor::state::Workspace::Ui, "UI");
        }

        let project_loaded = current_project_clone(app_state).is_some();
        let busy = app_state
            .get_state_data_value::<EditorRoot>("editor")
            .map(|root| {
                root.editor.job.is_some()
                    || root.editor.save_job.is_some()
                    || root.editor.project_load.is_some()
            })
            .unwrap_or(false);
        let enabled = project_loaded && !busy;

        if enabled
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
        {
            project::start_save_scene_and_project(app_state);
        }

        ui.add_enabled_ui(enabled, |ui| {
            ui.menu_button("Build", |ui| {
                if ui.button("Debug build").clicked() {
                    actions::build_project(app_state, false);
                    ui.close_menu();
                }
                if ui.button("Release build").clicked() {
                    actions::build_project(app_state, true);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Update dependencies").clicked() {
                    actions::update_dependencies(app_state);
                    ui.close_menu();
                }
            });
            ui.separator();
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("▶  Play")
                            .strong()
                            .color(theme::BACKGROUND),
                    )
                    .fill(theme::ACCENT)
                    .min_size(egui::vec2(80.0, 27.0)),
                )
                .on_hover_text("Save the scene and launch the game")
                .clicked()
            {
                actions::run_project(app_state);
            }
            if ui
                .button("Save scene")
                .on_hover_text("Save the active scene and project (Ctrl/Cmd+S)")
                .clicked()
            {
                project::start_save_scene_and_project(app_state);
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") {
                if let Some(p) = &root.project {
                    if root.editor.dirty {
                        ui.label(egui::RichText::new("Unsaved").small().color(theme::AMBER))
                            .on_hover_text("Unsaved changes");
                    }
                    if let Some(scene) = p.scenes.get(p.active_scene_index) {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&scene.name).strong())
                                .truncate(true),
                        );
                    }
                } else {
                    ui.weak("Create or open a project to begin");
                }
            }
        });
    });
}

pub fn status(ui: &mut Ui, app_state: &AppState) {
    ui.horizontal(|ui| {
        if let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") {
            let label = root
                .project
                .as_ref()
                .map(|p| p.name.as_str())
                .unwrap_or("No project open");
            ui.add(
                egui::Label::new(egui::RichText::new(label).small().color(theme::MUTED))
                    .truncate(true),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(job) = root.editor.job.as_ref() {
                    let elapsed = job.started_at.elapsed().as_secs_f32();
                    ui.add(egui::Spinner::new());
                    ui.small(format!("{} ({:.1}s)", job.label, elapsed));
                } else if let Some(last) = root.editor.last_job.as_ref() {
                    let marker = if last.success { "✓" } else { "✗" };
                    let dur = last.duration.as_secs_f32();
                    ui.small(format!("{} {} ({:.1}s)", marker, last.label, dur))
                        .on_hover_text(&last.message);
                } else {
                    ui.small(if root.editor.workspace == crate::editor::state::Workspace::Ui {
                        "Drag to move · Corner to resize · Ctrl/Cmd+Z Undo"
                    } else { "Q Select   W Move   E Rotate   R Scale   F Focus" });
                }
            });
        }
    });
}

fn scene_menu(ui: &mut Ui, app_state: &mut AppState) {
    let project_clone = current_project_clone(app_state);
    let Some(project) = project_clone else {
        return;
    };

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
        let scenes: Vec<_> = project
            .scenes
            .iter()
            .enumerate()
            .map(|(index, scene)| {
                let label = if index == project.active_scene_index {
                    format!("{} (active)", scene.name)
                } else {
                    scene.name.clone()
                };
                (index, label)
            })
            .collect();
        let switch_to = widgets::search_menu(ui, "Switch scene", &scenes);

        if let Some(idx) = switch_to {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(proj) = root.project.as_mut() {
                    let mut proj_clone = proj.clone();
                    if let Err(e) = crate::project::scene::switch(&mut proj_clone, app_state, idx) {
                        eprintln!("scene switch failed: {e:?}");
                    } else if let Some(root) =
                        app_state.get_state_data_value_mut::<EditorRoot>("editor")
                    {
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
    app_state
        .get_state_data_value::<EditorRoot>("editor")
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
