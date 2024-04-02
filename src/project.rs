use std::fs;
use std::fs::File;
use std::path::Path;
use std::io::Write;
use enigma::AppState;
use crate::{Engine, resources};


pub fn try_new_project(path: &str, app_state: &mut AppState) -> bool {
    // create a new rust project
    if !check_empty_directory(path) {
        return false;
    }
    let mut new_engine = Engine::new();
    new_engine.current_project = path.replace("\\", "/").to_string();
    create_folder_struct(path, &new_engine);
    app_state.add_state_data("engine", Box::new(new_engine));
    true
}

pub fn try_load_project(path: &str, app_state: &mut AppState) -> bool {
    let path = &path.replace("\\", "/");
    // load an existing rust project
    if !is_valid_project(path) {
        return false;
    }
    let project_data = fs::read_to_string(path).expect("Failed to read project file");
    let engine: Engine = serde_json::from_str(&project_data).expect("Failed to deserialize project file");
    app_state.add_state_data("engine", Box::new(engine));
    true
}

pub fn try_save_project(app_state: &mut AppState) -> bool {
    // save the current project
    let engine = app_state.get_state_data_value_mut::<Engine>("engine");
    let success = match engine {
        Some(engine) => {
            let project_file = format!("{}/enigma_project.json", engine.current_project);
            if is_valid_project(project_file.as_str()) {
                let serialized_engine = serde_json::to_string_pretty(&engine).unwrap();
                fs::write(&project_file, serialized_engine).unwrap();
                true
            } else {
                false
            }
        }
        None => {
            println!("could not find engine");
            false
        }
    };
    success
}

pub fn is_valid_project(path: &str) -> bool {
    let file = Path::new(path);
    if file.is_file() && file.file_name().unwrap() == "enigma_project.json" {
        return true;
    }
    false
}

fn check_empty_directory(path: &str) -> bool {
    // check if the directory is empty
    let dir = Path::new(path);
    if dir.is_file() {
        return false;
    }
    let mut entries = fs::read_dir(dir).expect("Failed to read directory");
    entries.next().is_none()
}


fn create_folder_struct(path: &str, engine: &Engine) {
    // create a folder struct for the project
    // Step 1: Create the project directory
    let project_dir = Path::new(path);
    let project_name = project_dir.file_name().unwrap().to_str().unwrap();
    fs::create_dir_all(project_dir.join("src/resources/textures")).expect("Failed to create textures folder");
    fs::create_dir_all(project_dir.join("src/resources/models")).expect("Failed to create models folder");
    fs::create_dir_all(project_dir.join("src/resources/shader")).expect("Failed to create shader folder");
    fs::create_dir_all(project_dir.join("src/resources/scenes")).expect("Failed to create scripts folder");
    fs::create_dir_all(project_dir.join("src/resources/audio")).expect("Failed to create audio folder");
    fs::create_dir_all(project_dir.join("src/resources/other")).expect("Failed to create scripts folder");

    // Step 2: Generate Cargo.toml
    let mut cargo_toml = File::create(project_dir.join("Cargo.toml")).expect("Failed to create Cargo.toml");
    let toml_template = resources::CARGO_TOML.to_string().replace("ENIGMA_PROJECT_NAME", project_name);
    write!(cargo_toml, "{}", toml_template).expect("Failed to write to Cargo.toml");


    // Step 3: Create src/main.rs with a simple program
    let mut main_rs = File::create(project_dir.join("src/main.rs")).expect("Failed to create main.rs");
    let main_template = resources::MAIN_RS.to_string().replace("ENIGMA_PROJECT_NAME", project_name);
    writeln!(main_rs, "{}", main_template).expect("Failed to write to main.rs");

    // Step 4: Create Empty Scene File
    let mut scene_file = File::create(project_dir.join("src/resources/scenes/enigma_main_scene.json")).expect("Failed to create scene file");
    writeln!(scene_file, "{}", "{}").expect("Failed to write to scene file");

    // Step 5: Create project file
    let mut project_file = File::create(project_dir.join("enigma_project.json")).expect("Failed to create project file");

    let serialized = serde_json::to_string(&engine).expect("Failed to serialize engine");
    writeln!(project_file, "{}", serialized).expect("Failed to write to project file");

    println!("Cargo project '{}' created successfully.", path);
}