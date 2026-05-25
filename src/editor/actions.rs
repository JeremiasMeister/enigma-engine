use std::process::Command;

use enigma_3d::AppState;
use enigma_3d::light::{Light, LightEmissionType};
use enigma_3d::object::Object;

use crate::editor::state::ProjectState;

pub fn run_project(project: &ProjectState) {
    spawn_cargo(project, &["run"]);
}

pub fn build_project(project: &ProjectState, release: bool) {
    if release {
        spawn_cargo(project, &["build", "--release"]);
    } else {
        spawn_cargo(project, &["build"]);
    }
}

fn spawn_cargo(project: &ProjectState, args: &[&str]) {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(&project.root_path)
        .output();
    match output {
        Ok(o) if o.status.success() => {
            println!("cargo {}: ok", args.join(" "));
            println!("{}", String::from_utf8_lossy(&o.stdout));
        }
        Ok(o) => {
            eprintln!("cargo {} failed", args.join(" "));
            eprintln!("{}", String::from_utf8_lossy(&o.stderr));
        }
        Err(e) => eprintln!("could not spawn cargo: {e}"),
    }
}

pub enum ObjectTemplate {
    Empty,
    Cube,
    Sphere,
}

pub fn add_object(app_state: &mut AppState, template: ObjectTemplate) {
    let obj = match template {
        ObjectTemplate::Empty => Object::default(),
        ObjectTemplate::Cube => Object::cube(0.5),
        ObjectTemplate::Sphere => Object::sphere(0.5, 16, 24),
    };
    app_state.add_object(obj);
}

pub enum LightTemplate {
    Directional,
    Point,
    Ambient,
}

pub fn add_light(app_state: &mut AppState, kind: LightTemplate) {
    let (light, emission) = match kind {
        LightTemplate::Directional => (
            Light::new([0.0, 5.0, 0.0], [1.0, 1.0, 1.0], 1.0, Some([0.0, -1.0, 0.0]), true),
            LightEmissionType::Source,
        ),
        LightTemplate::Point => (
            Light::new([0.0, 1.0, 0.0], [1.0, 1.0, 1.0], 1.0, None, false),
            LightEmissionType::Source,
        ),
        LightTemplate::Ambient => (
            Light::new([0.0, 0.0, 0.0], [0.1, 0.1, 0.1], 1.0, None, false),
            LightEmissionType::Ambient,
        ),
    };
    app_state.add_light(light, emission);
}
