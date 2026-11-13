use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use enigma_3d::AppState;
use enigma_3d::light::{Light, LightEmissionType};
use enigma_3d::object::Object;
use uuid::Uuid;

use crate::editor::state::{EditorRoot, JobMessage, JobOutcome, MaterialDef, ProjectState, RunningJob};
use crate::project;

pub fn run_project(app_state: &mut AppState) {
    if is_busy(app_state) { return; }
    let Some(project) = save_before_run(app_state) else { return; };
    start_cargo(app_state, &project, "Run", vec!["run".into()]);
}

pub fn build_project(app_state: &mut AppState, release: bool) {
    if is_busy(app_state) { return; }
    let Some(project) = save_before_run(app_state) else { return; };
    let (label, args) = if release {
        ("Release Build".to_string(), vec!["build".into(), "--release".into()])
    } else {
        ("Debug Build".to_string(), vec!["build".into()])
    };
    start_cargo(app_state, &project, &label, args);
}

pub fn update_dependencies(app_state: &mut AppState) {
    if is_busy(app_state) { return; }
    let Some(project) = save_before_run(app_state) else { return; };
    start_cargo(app_state, &project, "Update Dependencies", vec!["update".into()]);
}

pub fn is_busy(app_state: &AppState) -> bool {
    app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.job.is_some())
        .unwrap_or(false)
}

pub fn poll_job(app_state: &mut AppState) {
    let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") else { return; };
    let Some(job) = r.editor.job.as_mut() else { return; };
    let mut outcome: Option<JobOutcome> = None;
    let mut disconnected = false;
    loop {
        match job.rx.try_recv() {
            Ok(JobMessage::Line(s)) => {
                job.lines.push(s);
                if job.lines.len() > 500 {
                    let overflow = job.lines.len() - 500;
                    job.lines.drain(0..overflow);
                }
            }
            Ok(JobMessage::Done(o)) => {
                outcome = Some(o);
                break;
            }
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => {
                disconnected = true;
                break;
            }
        }
    }
    if disconnected && outcome.is_none() {
        outcome = Some(JobOutcome {
            label: job.label.clone(),
            success: false,
            duration: job.started_at.elapsed(),
            message: "job worker disconnected".to_string(),
        });
    }
    if let Some(outcome) = outcome {
        let reload = finish_cargo_job(r, outcome);
        if let Some(path) = reload {
            reload_project_editor_handoff(app_state, &path);
        }
    }
}

fn finish_cargo_job(root: &mut EditorRoot, outcome: JobOutcome) -> Option<String> {
    root.editor.job = None;
    root.editor.last_job = Some(outcome);
    root.editor.project_editor_handoff.clone()
}

pub(crate) fn reload_project_editor_handoff(app: &mut AppState, path: &str) {
    let result = project::try_open_project(path, app);
    if let Some(root) = app.get_state_data_value_mut::<EditorRoot>("editor") {
        match result {
            Ok(()) => root.editor.project_editor_handoff = None,
            Err(error) => root.editor.modal = Some(crate::editor::state::Modal::ImportError(format!("Cannot reload project after closing its editor: {error}"))),
        }
    }
}

pub(crate) fn start_cargo(app_state: &mut AppState, project: &ProjectState, label: &str, args: Vec<String>) {
    let (tx, rx) = mpsc::channel();
    let label_owned = label.to_string();
    let root_path = project.root_path.clone();
    let started_at = Instant::now();
    let thread_label = label_owned.clone();
    thread::spawn(move || {
        let child = match project::build::cargo_command(std::path::Path::new(&root_path), &args) {
            Ok(mut command) => command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn(),
            Err(error) => Err(std::io::Error::other(error)),
        };

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(JobMessage::Done(JobOutcome {
                    label: thread_label,
                    success: false,
                    duration: started_at.elapsed(),
                    message: format!("could not spawn cargo: {e}"),
                }));
                return;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let tx_out = tx.clone();
        let tx_err = tx.clone();
        let out_thread = stdout.map(|s| thread::spawn(move || {
            for line in BufReader::new(s).lines().flatten() {
                if tx_out.send(JobMessage::Line(line)).is_err() { break; }
            }
        }));
        let err_thread = stderr.map(|s| thread::spawn(move || {
            for line in BufReader::new(s).lines().flatten() {
                if tx_err.send(JobMessage::Line(line)).is_err() { break; }
            }
        }));

        let status = child.wait();
        if let Some(t) = out_thread { let _ = t.join(); }
        if let Some(t) = err_thread { let _ = t.join(); }

        let outcome = match status {
            Ok(s) => JobOutcome {
                label: thread_label,
                success: s.success(),
                duration: started_at.elapsed(),
                message: if s.success() { "ok".into() } else { format!("exit {:?}", s.code()) },
            },
            Err(e) => JobOutcome {
                label: thread_label,
                success: false,
                duration: started_at.elapsed(),
                message: format!("wait error: {e}"),
            },
        };
        let _ = tx.send(JobMessage::Done(outcome));
    });
    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        r.editor.job = Some(RunningJob {
            label: label_owned,
            started_at,
            rx,
            lines: Vec::new(),
        });
    }
}

fn save_before_run(app_state: &mut AppState) -> Option<ProjectState> {
    let project = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.clone())?;
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        if let Err(error) = root.editor.narrative.save_project(std::path::Path::new(&project.root_path)) {
            root.editor.workspace = crate::editor::state::Workspace::Narrative;
            root.editor.modal = Some(crate::editor::state::Modal::ImportError(format!("Build/Play stopped: {error}")));
            return None;
        }
        if let Err(error) = root.editor.code_editor.save_project(std::path::Path::new(&project.root_path)) {
            root.editor.workspace = crate::editor::state::Workspace::Code;
            root.editor.modal = Some(crate::editor::state::Modal::ImportError(format!("Build/Play stopped: {error}")));
            return None;
        }
    }
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

#[cfg(test)]
mod project_editor_tests {
    use super::*;

    #[test]
    fn completed_project_editor_requests_reload_and_keeps_parent_locked() {
        let mut root = EditorRoot::empty();
        root.editor.project_editor_handoff = Some("/tmp/game/enigma_project.json".into());
        let outcome = JobOutcome { label: "Project Editor".into(), success: true, duration: Default::default(), message: "ok".into() };
        let reload = finish_cargo_job(&mut root, outcome);
        assert_eq!(reload.as_deref(), Some("/tmp/game/enigma_project.json"));
        assert!(root.editor.project_editor_handoff.is_some(), "Only a successful disk reload unlocks the parent");
        assert!(root.editor.job.is_none());
    }
}
