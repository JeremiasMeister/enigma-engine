mod ui;
mod project;
mod resources;
mod serialization;

use std::sync::Arc;
use enigma::AppState;
use enigma::camera::Camera;
use enigma::object::Object;
use crate::resources::{BinaryResource, TextResource};

struct Engine {
    pub current_project: String,
    pub selected_resource: String,
    pub object_builder_open: bool,
    pub model_resources: Vec<BinaryResource>,
    pub texture_resources: Vec<BinaryResource>,
    pub shader_resources: Vec<TextResource>,
    pub object_resources: Vec<Object>,
}

impl Engine {
    pub fn new() -> Self {
        Engine {
            current_project: String::new(),
            selected_resource: String::new(),
            object_builder_open: false,
            model_resources: Vec::new(),
            texture_resources: Vec::new(),
            shader_resources: Vec::new(),
            object_resources: Vec::new(),
        }
    }

    pub fn set_current_project(&mut self, path: &str) {
        self.current_project = path.to_string();
    }

    pub fn new_project(&mut self, path: &str) {
        self.current_project = path.to_string();
        project::try_new_project(path);
    }

    pub fn get_current_project(&self) -> &str {
        &self.current_project
    }

    pub fn open_project(&mut self, path: &str) {
        self.current_project = path.to_string();
    }

    pub fn run_project(&self) {
        let output = std::process::Command::new("cargo")
            .arg("run")
            .current_dir(&self.current_project)
            .output()
            .expect("Failed to run project");
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("Cargo run successful. Output:\n{}", stdout);
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Cargo run failed. Error:\n{}", stderr);
        }
    }

    pub fn build_project(&self, release: bool) {
        if release {
            let output = std::process::Command::new("cargo")
                .arg("build")
                .arg("--release")
                .current_dir(&self.current_project)
                .output()
                .expect("Failed to run project");
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!("Cargo run successful. Output:\n{}", stdout);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Cargo run failed. Error:\n{}", stderr);
            }
        } else {
            let output = std::process::Command::new("cargo")
                .arg("build")
                .current_dir(&self.current_project)
                .output()
                .expect("Failed to run project");
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!("Cargo run successful. Output:\n{}", stdout);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("Cargo run failed. Error:\n{}", stderr);
            }
        }
    }
}

fn main() {
    let mut app_state = AppState::new();
    let event_loop = enigma::EventLoop::new("Enigma 3d - Engine", 1080, 720);

    let camera = Camera::new(None, None, Some(60.0), Some(16.0/9.0), Some(0.1), Some(100.0));
    app_state.set_camera(camera);

    // inject the ui into the app_state
    app_state.inject_gui(Arc::new(ui::project_window));
    app_state.inject_gui(Arc::new(ui::transform_window));
    app_state.inject_gui(Arc::new(ui::scene_entities_window));
    app_state.inject_gui(Arc::new(ui::resource_inspector_window));
    app_state.inject_gui(Arc::new(ui::resources_window));
    event_loop.run(app_state.convert_to_arc_mutex());
}
