use std::fs;
use egui::{Context, Style};
use rfd::AsyncFileDialog;
use enigma::AppState;
use crate::{Engine, project};

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
            ui.label("Project");
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        println!("Creating new project");
                        let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                        let (tx, rx) = std::sync::mpsc::channel();
                        if let Some(engine) = engine {
                            println!("closing Current project: {}", &engine.get_current_project());
                            let dialog = AsyncFileDialog::new().pick_folder();
                            async_std::task::spawn(async move {
                                if let Some(folder) = dialog.await {
                                    let path = folder.path().to_str().expect("Invalid path").to_owned();
                                    tx.send(path.replace("\\", "/")).expect("Failed to send path");
                                }
                            });
                            if let Ok(path) = rx.recv() {
                                if project::try_new_project(&path) {
                                    println!("New project path: {:?}", path);
                                    engine.new_project(&path);
                                } else {
                                    println!("Failed to create new project. Most likely, since the folder is not empty");
                                }
                            }
                        } else {
                            println!("No engine found. Creating one");
                            let mut engine = Engine::new();
                            let dialog = AsyncFileDialog::new().pick_folder();
                            async_std::task::spawn(async move {
                                if let Some(folder) = dialog.await {
                                    let path = folder.path().to_str().expect("Invalid path").to_owned();
                                    tx.send(path.replace("\\", "/")).expect("Failed to send path");
                                }
                            });
                            if let Ok(path) = rx.recv() {
                                if project::try_new_project(&path) {
                                    println!("New project path: {:?}", path);
                                    engine.new_project(&path);
                                } else {
                                    println!("Failed to create new project. Most likely, since the folder is not empty");
                                }
                            }
                            app_state.add_state_data("engine", Box::new(engine));
                        }
                        ui.close_menu();
                    }
                    if ui.button("Open Project").clicked() {
                        println!("Opening project");
                        let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                        if let Some(engine) = engine {
                            let (tx, rx) = std::sync::mpsc::channel();
                            let dialog = AsyncFileDialog::new().pick_folder();
                            async_std::task::spawn(async move {
                                if let Some(folder) = dialog.await {
                                    let path = folder.path().to_str().expect("Invalid path").to_owned();
                                    tx.send(path.replace("\\", "/")).expect("Failed to send path");
                                }
                            });
                            if let Ok(path) = rx.recv() {
                                if project::is_valid_project(&path) {
                                    println!("Opened project path: {:?}", path);
                                    engine.open_project(&path);
                                } else {
                                    println!("Failed to open project. Most likely, since the folder is empty");
                                }
                            }
                        } else {
                            println!("No engine found. Creating one");
                            let mut engine = Engine::new();
                            let (tx, rx) = std::sync::mpsc::channel();
                            let dialog = AsyncFileDialog::new().pick_folder();
                            async_std::task::spawn(async move {
                                if let Some(folder) = dialog.await {
                                    let path = folder.path().to_str().expect("Invalid path").to_owned();
                                    tx.send(path.replace("\\", "/")).expect("Failed to send path");
                                }
                            });
                            if let Ok(path) = rx.recv() {
                                if project::is_valid_project(&path) {
                                    println!("Opened project path: {:?}", path);
                                    engine.open_project(&path);
                                } else {
                                    println!("Failed to open project. Most likely, since the folder is empty");
                                }
                            }
                            app_state.add_state_data("engine", Box::new(engine));
                        }
                        ui.close_menu();
                    }
                    if ui.button("Save Project").clicked() {
                        let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                        match engine {
                            Some(engine) => {
                                let project = engine.get_current_project();
                                if project::is_valid_project(project){
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
                ui.menu_button("Cargo Config", |ui| {
                    if ui.button("Run Current Project").clicked() {
                        let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                        if let Some(engine) = engine {
                            println!("Running project: {}", &engine.current_project);
                            engine.run_project();
                        } else {
                            println!("No project to run");
                        }
                        ui.close_menu();
                    }
                    if ui.button("Debug Build Current Project").clicked() {
                        let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                        if let Some(engine) = engine {
                            println!("Building project: {}", &engine.current_project);
                            engine.build_project(false);
                        } else {
                            println!("No project to build");
                        }
                        ui.close_menu();
                    }
                    if ui.button("Release Build Current Project").clicked() {
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

pub fn scene_entities_window(context: &Context, app_state: &mut AppState) {
    context.set_style(get_ui_style(context));
    // This is a placeholder for the actual function
    egui::Window::new("Scene Entities")
        .default_width(200.0)
        .default_height(200.0)
        .show(context, |ui| {
            ui.label("Scene Entities");
            ui.horizontal(|ui| {
                ui.menu_button("Add Object", add_object_to_scene_context_menu);
                ui.menu_button("Delete Selected Objects", |ui| {
                    if ui.button("Delete?").clicked() {
                        println!("Deleting objects");
                        app_state.object_selection.clear();
                        ui.close_menu();
                    }
                });
            });
        });
}

fn add_object_to_scene_context_menu(ui: &mut egui::Ui) {
    ui.menu_button("Lights", |ui| {
        if ui.button("Point Light").clicked() {
            ui.close_menu();
        }
    });
    ui.menu_button("Objects", |ui| {
        if ui.button("Load from Resource").clicked() {
            ui.close_menu();
        }
    });
    ui.menu_button("Camera", |ui| {
        if ui.button("Create Camera").clicked() {
            ui.close_menu();
        }
    });
}

pub fn transform_window(context: &Context, app_state: &mut AppState) {
    context.set_style(get_ui_style(context));
    let screen_size = context.available_rect().max;

    // This is a placeholder for the actual function
    egui::Window::new("Transform Edit")
        .default_width(200.0)
        .default_height(200.0)
        .default_pos(egui::Pos2::new(screen_size.x - 200.0, 20.0))
        .show(context, |ui| {
            if let Some(selected_object) = app_state.get_selected_objects_mut().first_mut() {
                ui.label("Transform");

                let mut position = selected_object.transform.get_position();
                ui.label("Position");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut position.x));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut position.y));
                    ui.label("Z");
                    ui.add(egui::DragValue::new(&mut position.z));
                });
                selected_object.transform.set_position(position.into());

                let mut rotation = app_state.get_selected_objects_mut()[0].transform.get_rotation();
                ui.label("Rotation");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut rotation.x));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut rotation.y));
                    ui.label("Z");
                    ui.add(egui::DragValue::new(&mut rotation.z));
                });
                app_state.get_selected_objects_mut()[0].transform.set_rotation(rotation.into());

                let mut scale = app_state.get_selected_objects_mut()[0].transform.get_scale();
                ui.label("Scale");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut scale.x));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut scale.y));
                    ui.label("Z");
                    ui.add(egui::DragValue::new(&mut scale.z));
                });
                app_state.get_selected_objects_mut()[0].transform.set_scale(scale.into());
            } else {
                ui.label("No object selected");
            }
        });
}

pub fn resource_inspector_window(context: &Context, app_state: &mut AppState) {
    context.set_style(get_ui_style(context));
    let screen_size = context.available_rect().max;

    // This is a placeholder for the actual function
    egui::Window::new("Resource Inspector")
        .default_width(200.0)
        .default_height(600.0)
        .default_pos(egui::Pos2::new(screen_size.x - 200.0, screen_size.y - 280.0))
        .show(context, |ui| {
            ui.label("Resource Inspector");
            ui.horizontal(|ui| {
                ui.label("Inspector");
            });
        });
}

pub fn resources_window(context: &egui::Context, app_state: &mut AppState) {
    context.set_style(get_ui_style(context));
    let screen_size = context.available_rect().max;


    // This is a placeholder for the actual function
    egui::Window::new("Resources")
        .default_width(screen_size.x)
        .resizable(true)
        .default_width(screen_size.x - 280.0)
        .default_height(600.0)
        .default_pos(egui::Pos2::new(20.0, screen_size.y - 280.0))
        .show(context, |ui| {
            ui.label("Resources");
            ui.horizontal(|ui| {
                ui.menu_button("Import Binary", |ui| {
                    let mut project_path: Option<String> = None;
                    let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                    if let Some(engine) = engine {
                        project_path = Some(engine.get_current_project().to_string());
                    }
                    if let Some(project_path) = project_path {
                        // Clone project_path here to capture it by the async block below
                        if ui.button("Import Texture").clicked() {
                            // Clone project_path for use in async block
                            let project_path = project_path.clone();
                            let dialog = AsyncFileDialog::new().pick_files();
                            async_std::task::spawn(async move {
                                if let Some(files) = dialog.await {
                                    for file in files {
                                        let path = file.path().to_str().expect("Invalid path").to_owned();
                                        // Use cloned project_path here
                                        let destination = format!("{0}/src/resources/textures/{1}", project_path, file.file_name().as_str());
                                        async_std::fs::copy(path.clone(), destination.clone()).await.expect("Failed to copy file");
                                        let name = file.file_name().as_str().to_owned();
                                        let resource = crate::resources::import_resource_binary(&name, &destination);
                                        //TODO: Add resource to engine
                                        println!("Importing resource: {:?}", path);
                                    }
                                }
                            });
                            ui.close_menu();
                        }
                        if ui.button("Import Model").clicked() {
                            // Clone project_path for use in async block
                            let project_path = project_path.clone();
                            let dialog = AsyncFileDialog::new().pick_files();
                            async_std::task::spawn(async move {
                                if let Some(files) = dialog.await {
                                    for file in files {
                                        let path = file.path().to_str().expect("Invalid path").to_owned();
                                        // Use cloned project_path here
                                        let destination = format!("{0}/src/resources/models/{1}", project_path, file.file_name().as_str());
                                        async_std::fs::copy(path.clone(), destination.clone()).await.expect("Failed to copy file");
                                        let resource = crate::resources::import_resource_binary(file.file_name().as_str(), &destination);
                                        //TODO: Add resource to engine
                                        println!("Importing resource: {:?}", path);
                                    }
                                }
                            });
                            ui.close_menu();
                        }
                        if ui.button("Import Audio").clicked() {
                            // Clone project_path for use in async block
                            let project_path = project_path.clone();
                            let dialog = AsyncFileDialog::new().pick_files();
                            async_std::task::spawn(async move {
                                if let Some(files) = dialog.await {
                                    for file in files {
                                        let path = file.path().to_str().expect("Invalid path").to_owned();
                                        // Use cloned project_path here
                                        let destination = format!("{0}/src/resources/audio/{1}", project_path, file.file_name().as_str());
                                        async_std::fs::copy(path.clone(), destination).await.expect("Failed to copy file");
                                        println!("Importing resource: {:?}", path);
                                    }
                                }
                            });
                            ui.close_menu();
                        }
                        if ui.button("Import Other").clicked() {
                            // Clone project_path for use in async block
                            let project_path = project_path.clone();
                            let dialog = AsyncFileDialog::new().pick_files();
                            async_std::task::spawn(async move {
                                if let Some(files) = dialog.await {
                                    for file in files {
                                        let path = file.path().to_str().expect("Invalid path").to_owned();
                                        // Use cloned project_path here
                                        let destination = format!("{0}/src/resources/other/{1}", project_path, file.file_name().as_str());
                                        async_std::fs::copy(path.clone(), destination).await.expect("Failed to copy file");
                                        println!("Importing resource: {:?}", path);
                                    }
                                }
                            });
                            ui.close_menu();
                        }
                    } else {
                        println!("No engine found");
                    }
                });
                ui.menu_button("Import Text", |ui| {
                    let mut project_path: Option<String> = None;
                    let engine = app_state.get_state_data_value_mut::<Engine>("engine");
                    if let Some(engine) = engine {
                        project_path = Some(engine.get_current_project().to_string());
                    }
                    if let Some(project_path) = project_path {
                        // Clone project_path here to capture it by the async block below
                        if ui.button("Import Shader").clicked() {
                            // Clone project_path for use in async block
                            let project_path = project_path.clone();
                            let dialog = AsyncFileDialog::new().pick_files();
                            async_std::task::spawn(async move {
                                if let Some(files) = dialog.await {
                                    for file in files {
                                        let path = file.path().to_str().expect("Invalid path").to_owned();
                                        // Use cloned project_path here
                                        let destination = format!("{0}/src/resources/shader/{1}", project_path, file.file_name().as_str());
                                        async_std::fs::copy(path.clone(), destination.clone()).await.expect("Failed to copy file");
                                        let resource = crate::resources::import_resource_text(file.file_name().as_str(), &destination);
                                        //TODO: Add resource to engine
                                        println!("Importing resource: {:?}", path);
                                    }
                                }
                            });
                            ui.close_menu();
                        }
                        if ui.button("Import Other").clicked() {
                            // Clone project_path for use in async block
                            let project_path = project_path.clone();
                            let dialog = AsyncFileDialog::new().pick_files();
                            async_std::task::spawn(async move {
                                if let Some(files) = dialog.await {
                                    for file in files {
                                        let path = file.path().to_str().expect("Invalid path").to_owned();
                                        // Use cloned project_path here
                                        let destination = format!("{0}/src/resources/other/{1}", project_path, file.file_name().as_str());
                                        async_std::fs::copy(path.clone(), destination).await.expect("Failed to copy file");
                                        println!("Importing resource: {:?}", path);
                                    }
                                }
                            });
                            ui.close_menu();
                        }
                    } else {
                        println!("No engine found");
                    }
                });
            });
            ui.separator();
            ui.label("Textures");
            ui.separator();
            let mut project_path: Option<String> = None;
            let engine = app_state.get_state_data_value_mut::<Engine>("engine");
            if let Some(engine) = engine {
                project_path = Some(engine.get_current_project().to_string());
            }
            if let Some(project_path) = project_path {
                let texture_path = format!("{0}/src/resources/textures", project_path);
                let entries = fs::read_dir(texture_path).expect("Failed to read directory");
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        let name = path.file_name().expect("Failed to get file name").to_str().expect("Failed to convert to string");
                        ui.label(name);
                    }
                }
            }
            ui.separator();
            ui.label("Models");
            ui.separator();
            let mut project_path: Option<String> = None;
            let engine = app_state.get_state_data_value_mut::<Engine>("engine");
            if let Some(engine) = engine {
                project_path = Some(engine.get_current_project().to_string());
            }
            if let Some(project_path) = project_path {
                let texture_path = format!("{0}/src/resources/models", project_path);
                let entries = fs::read_dir(texture_path).expect("Failed to read directory");
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        let name = path.file_name().expect("Failed to get file name").to_str().expect("Failed to convert to string");
                        ui.label(name);
                    }
                }
            }
            ui.separator();
            ui.label("Audio");
            ui.separator();
            let mut project_path: Option<String> = None;
            let engine = app_state.get_state_data_value_mut::<Engine>("engine");
            if let Some(engine) = engine {
                project_path = Some(engine.get_current_project().to_string());
            }
            if let Some(project_path) = project_path {
                let texture_path = format!("{0}/src/resources/audio", project_path);
                let entries = fs::read_dir(texture_path).expect("Failed to read directory");
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        let name = path.file_name().expect("Failed to get file name").to_str().expect("Failed to convert to string");
                        ui.label(format!("{:?} - NOT IN APPSTATE", name));
                    }
                }
            }
            ui.separator();
            ui.label("Shader");
            ui.separator();
            let mut project_path: Option<String> = None;
            let engine = app_state.get_state_data_value_mut::<Engine>("engine");
            if let Some(engine) = engine {
                project_path = Some(engine.get_current_project().to_string());
            }
            if let Some(project_path) = project_path {
                let texture_path = format!("{0}/src/resources/shader", project_path);
                let entries = fs::read_dir(texture_path).expect("Failed to read directory");
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        let name = path.file_name().expect("Failed to get file name").to_str().expect("Failed to convert to string");
                        ui.label(name);
                    }
                }
            }
            ui.separator();
            ui.label("Other");
            ui.separator();
            let mut project_path: Option<String> = None;
            let engine = app_state.get_state_data_value_mut::<Engine>("engine");
            if let Some(engine) = engine {
                project_path = Some(engine.get_current_project().to_string());
            }
            if let Some(project_path) = project_path {
                let texture_path = format!("{0}/src/resources/other", project_path);
                let entries = fs::read_dir(texture_path).expect("Failed to read directory");
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        let name = path.file_name().expect("Failed to get file name").to_str().expect("Failed to convert to string");
                        ui.label(format!("{:?} - NOT IN APPSTATE", name));
                    }
                }
            }
        });
}
