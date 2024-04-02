use std::fs;
use egui::{Context, Style};
use rfd::AsyncFileDialog;
use enigma::AppState;
use crate::{Engine, project};
use crate::resources::ResourceType;

pub fn get_ui_style(context: &Context) -> Style {
    let mut style = (*context.style()).clone();

    style.visuals.window_shadow.extrusion = 0.0;
    style.visuals.window_shadow.color = egui::Color32::TRANSPARENT;

    style.visuals.window_fill = egui::Color32::from_rgba_premultiplied(0, 0, 0, 200);
    style.visuals.override_text_color = Some(egui::Color32::WHITE);
    style.visuals.window_stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);

    style
}

pub fn project_window(context: &Context, app_state: &mut AppState) {
    context.set_style(get_ui_style(context));
    // This is a placeholder for the actual function
    egui::Window::new("Project")
        .default_width(200.0)
        .default_height(200.0)
        .show(context, |ui| {
            let engine = app_state.get_state_data_value::<Engine>("engine");
            let project_name = match engine {
                Some(engine) => {
                    let current_project = &engine.current_project;
                    if current_project.is_empty() {
                        stringify!("No Project")
                    } else {
                        current_project.split("/").last().unwrap_or(stringify!("Could not get project name"))
                    }
                }
                None => stringify!("No Project"),
            };

            ui.label(format!("Project: {}", project_name));
            ui.horizontal(|ui| {
                ui.menu_button("Project", |ui| {
                    if ui.button("New Project").clicked() {
                        let (tx, rx) = std::sync::mpsc::channel();
                        let dialog = AsyncFileDialog::new().pick_folder();
                        async_std::task::spawn(async move {
                            if let Some(path) = dialog.await {
                                tx.send(path).expect("Failed to send path");
                            }
                        });
                        if let Ok(path) = rx.recv() {
                            let path = path.path().to_str().expect("Invalid path");
                            let success = project::try_new_project(path, app_state);
                            if success {
                                println!("New project created at: {:?}", path);
                            } else {
                                println!("Failed to create project at: {:?}", path);
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Open Project").clicked() {
                        println!("Opening project");
                        let (tx, rx) = std::sync::mpsc::channel();
                        let dialog = AsyncFileDialog::new().add_filter(".json", &["json"]).pick_file();
                        async_std::task::spawn(async move {
                            if let Some(path) = dialog.await {
                                tx.send(path).expect("Failed to send path");
                            }
                        });
                        if let Ok(path) = rx.recv() {
                            let path = path.path().to_str().expect("Invalid path");
                            let success = project::try_load_project(path, app_state);
                            if success {
                                println!("Project loaded from: {:?}", path.replace("\\", "/"));
                            } else {
                                println!("Failed to load project from: {:?}", path.replace("\\", "/"));
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save Project").clicked() {
                        let success = project::try_save_project(app_state);
                        if success {
                            println!("Project saved");
                        } else {
                            println!("Failed to save project");
                        }
                    }
                    if ui.button("Save Scene").clicked() {
                        let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                        match engine {
                            Some(engine) => {
                                let project = &engine.current_project;
                                if project::is_valid_project(project) {
                                    let start_folder = format!("{}/src/resources/scenes", project);
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    let dialog = AsyncFileDialog::new().add_filter(".json", &["json"]).set_directory(start_folder).pick_file();
                                    async_std::task::spawn(async move {
                                        if let Some(scene) = dialog.await {
                                            let path = scene.path().to_str().expect("Invalid path").to_owned();
                                            tx.send(path.replace("\\", "/")).expect("Failed to send path");
                                        }
                                    });
                                    if let Ok(path) = rx.recv() {
                                        if !path.ends_with(".json") {
                                            println!("Invalid file type. Must be a .json file");
                                        } else {
                                            println!("Saving project to: {:?}", path);
                                            let serialize_app_state = app_state.to_serializer();
                                            let serialized = serde_json::to_string_pretty(&serialize_app_state).unwrap();
                                            std::fs::write(path, serialized).unwrap();
                                        }
                                    }
                                }
                            }
                            None => {
                                println!("could not find engine");
                            }
                        }
                    }
                });
                ui.menu_button("Play/Build", |ui| {
                    if ui.button("Play").clicked() {
                        let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                        if let Some(engine) = engine {
                            println!("Running project: {}", &engine.current_project);
                            engine.run_project();
                        } else {
                            println!("No project to run");
                        }
                        ui.close_menu();
                    }
                    if ui.button("Debug Build").clicked() {
                        let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                        if let Some(engine) = engine {
                            println!("Building project: {}", &engine.current_project);
                            engine.build_project(false);
                        } else {
                            println!("No project to build");
                        }
                        ui.close_menu();
                    }
                    if ui.button("Release Build").clicked() {
                        let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                        if let Some(engine) = engine {
                            println!("Building project: {}", &engine.current_project);
                            engine.build_project(true);
                        } else {
                            println!("No project to build");
                        }
                        ui.close_menu();
                    }
                });
            });
        });
}
