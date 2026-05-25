use std::process::Command;

use enigma_3d::AppState;
use enigma_3d::light::{Light, LightEmissionType};
use enigma_3d::object::Object;
use uuid::Uuid;

use crate::editor::state::{EditorRoot, MaterialDef, ProjectState};
use crate::project;

pub fn run_project(app_state: &mut AppState) {
    let Some(project) = save_before_run(app_state) else { return; };
    spawn_cargo(&project, &["run"]);
}

pub fn build_project(app_state: &mut AppState, release: bool) {
    let Some(project) = save_before_run(app_state) else { return; };
    if release {
        spawn_cargo(&project, &["build", "--release"]);
    } else {
        spawn_cargo(&project, &["build"]);
    }
}

fn save_before_run(app_state: &mut AppState) -> Option<ProjectState> {
    let project = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.clone())?;
    if let Err(e) = project::scene::save_active(&project, app_state) {
        eprintln!("save scene failed: {e:?}");
        return None;
    }
    if let Err(e) = project::try_save_project(app_state) {
        eprintln!("save project failed: {e}");
        return None;
    }
    if let Err(e) = stage_startup_scene(&project) {
        eprintln!("stage startup scene failed: {e}");
        return None;
    }
    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        r.editor.dirty = false;
    }
    Some(project)
}

const STARTUP_SCENE_FILE: &str = "src/resources/scenes/enigma_main_scene.json";

fn stage_startup_scene(project: &ProjectState) -> std::io::Result<()> {
    let scene = project.scenes.get(project.startup_scene_index)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "startup scene index out of range"))?;
    let src = std::path::Path::new(&project.root_path)
        .join("src/resources")
        .join(&scene.relative_path);
    let dst = std::path::Path::new(&project.root_path).join(STARTUP_SCENE_FILE);
    if src == dst {
        return Ok(());
    }
    if !src.exists() {
        std::fs::write(&src, "{}")?;
    }
    std::fs::copy(&src, &dst)?;
    Ok(())
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
    let default_mat = ensure_default_material(app_state);
    let mut obj = match template {
        ObjectTemplate::Empty => Object::default(),
        ObjectTemplate::Cube => Object::cube(0.5),
        ObjectTemplate::Sphere => Object::sphere(0.5, 16, 24),
    };
    if let Some(mat_uuid) = default_mat {
        obj.add_material(mat_uuid);
    }
    app_state.add_object(obj);
}

fn ensure_default_material(app_state: &mut AppState) -> Option<Uuid> {
    let root = app_state.get_state_data_value_mut::<EditorRoot>("editor")?;
    let project = root.project.as_mut()?;
    if let Some(first) = project.materials.first() {
        return Some(first.uuid);
    }
    let def = MaterialDef::default_pbr("Default".to_string());
    let uuid = def.uuid;
    project.materials.push(def);
    root.editor.dirty = true;
    Some(uuid)
}

pub enum LightTemplate {
    Directional,
    Point,
    Ambient,
}

pub fn spawn_from_model(app_state: &mut AppState, model_uuid: Uuid) {
    let bytes = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = root.project.as_ref() else { return; };
        match project::resource::bytes(project, model_uuid) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("spawn_from_model: {e:?}");
                return;
            }
        }
    };
    let default_mat = ensure_default_material(app_state);
    let mut obj = Object::load_from_gltf_resource(&bytes, None);
    if obj.get_materials().is_empty() {
        if let Some(mat_uuid) = default_mat {
            obj.add_material(mat_uuid);
        }
    }
    app_state.add_object(obj);
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
