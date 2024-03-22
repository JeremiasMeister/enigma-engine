use std::fs;
use std::fs::File;
use std::path::Path;
use std::io::Write;
use crate::resources;


pub fn try_new_project(path: &str) -> bool {
    // create a new rust project
    if !check_empty_directory(path) {
        return false;
    }
    create_folder_struct(path);
    true
}

pub fn is_valid_project(path: &str) -> bool {
    // check if a Cargo.toml file exists in the directory
    let dir = Path::new(path);
    if dir.is_file() {
        return false;
    }
    let cargo_toml = dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return false;
    }
    true
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


fn create_folder_struct(path: &str) {
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

    println!("Cargo project '{}' created successfully.", path);
}