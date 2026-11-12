use std::fs;
use std::path::Path;
use enigma_3d::{AppState, AppStateSerializer};
use uuid::Uuid;

use crate::editor::state::{ProjectState, SceneRef};

pub fn save_active(project: &ProjectState, app_state: &AppState) -> Result<(), SceneError> {
    save_snapshot(project, app_state.to_serializer())
}

fn save_snapshot(project: &ProjectState, mut serializer: AppStateSerializer) -> Result<(), SceneError> {
    let scene = project.scenes.get(project.active_scene_index).ok_or(SceneError::NoActiveScene)?;
    let path = scene_path(project, scene);
    normalize_animation_times(&mut serializer.objects);
    let text = serde_json::to_string_pretty(&serializer).map_err(SceneError::Parse)?;
    fs::write(&path, text).map_err(SceneError::Io)?;
    Ok(())
}

pub fn switch(
    project: &mut ProjectState,
    app_state: &mut AppState,
    target_index: usize,
) -> Result<(), SceneError> {
    save_active(project, app_state).ok();   // best-effort

    let scene = project.scenes.get(target_index).ok_or(SceneError::BadIndex)?.clone();
    let path = scene_path(project, &scene);
    let text = fs::read_to_string(&path).map_err(SceneError::Io)?;
    let trimmed = text.trim();

    let serializer = if !trimmed.is_empty() && trimmed != "{}" {
        Some(serde_json::from_str::<AppStateSerializer>(&text).map_err(SceneError::Parse)?)
    } else { None };
    let display = app_state.display.clone().ok_or(SceneError::NoDisplay)?;
    clear_scene(app_state);
    if let Some(serializer) = serializer {
        app_state.inject_serializer(serializer, display, /*additive=*/false);
    }
    project.active_scene_index = target_index;
    Ok(())
}

pub fn new_scene(project: &mut ProjectState, name: String) -> Result<Uuid, SceneError> {
    let sanitized = sanitize(&name);
    let relative_path = format!("scenes/{sanitized}.json");

    let target = Path::new(&project.root_path).join("src/resources").join(&relative_path);
    if target.exists() {
        return Err(SceneError::AlreadyExists);
    }
    // ensure parent dir exists (in case scaffold didn't include it for some reason)
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(SceneError::Io)?;
    }
    fs::write(&target, "{}").map_err(SceneError::Io)?;

    let uuid = Uuid::new_v4();
    project.scenes.push(SceneRef { uuid, name, relative_path, terrain: None, particle_instances: Vec::new() });
    Ok(uuid)
}

pub fn delete(project: &mut ProjectState, index: usize) -> Result<(), SceneError> {
    if project.scenes.len() <= 1 {
        return Err(SceneError::CannotDeleteLast);
    }
    if index >= project.scenes.len() {
        return Err(SceneError::BadIndex);
    }
    let scene = project.scenes.remove(index);
    let path = Path::new(&project.root_path).join("src/resources").join(&scene.relative_path);
    if path.exists() {
        let trash_dir = Path::new(&project.root_path).join(".trash");
        fs::create_dir_all(&trash_dir).ok();
        let trash_path = trash_dir.join(format!("scene_{}.json", scene.name));
        fs::rename(path, trash_path).map_err(SceneError::Io)?;
    }
    if project.active_scene_index >= project.scenes.len() {
        project.active_scene_index = project.scenes.len() - 1;
    }
    if project.startup_scene_index >= project.scenes.len() {
        project.startup_scene_index = 0;
    }
    Ok(())
}

fn scene_path(project: &ProjectState, scene: &SceneRef) -> std::path::PathBuf {
    Path::new(&project.root_path).join("src/resources").join(&scene.relative_path)
}

fn clear_scene(app_state: &mut AppState) {
    // enigma_3d's AppState exposes these as pub fields.
    app_state.objects.clear();
    app_state.light.clear();
    app_state.materials.clear();
    app_state.post_processing.clear();
    // camera, ambient_light, skybox left intact — re-set by inject_serializer if present in the scene.
}

/// Zero out `current_animation.time` on every object serializer so the saved scene
/// always starts the chosen clip from the beginning. Loop, speed, and clip name
/// are preserved. Operates on the serializer copy only; live objects are unaffected.
pub(crate) fn normalize_animation_times(objects: &mut [enigma_3d::object::ObjectSerializer]) {
    for obj_ser in objects.iter_mut() {
        if let Some(state) = obj_ser.current_animation.as_mut() {
            state.time = 0.0;
        }
    }
}

fn sanitize(name: &str) -> String {
    name.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect()
}

#[derive(Debug)]
pub enum SceneError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    NoActiveScene,
    BadIndex,
    AlreadyExists,
    CannotDeleteLast,
    NoDisplay,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_snapshot_round_trip_preserves_effects_and_accepts_legacy_scenes() {
        use enigma_3d::postprocessing::stack::{ParameterValue, EffectEntry};
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        new_scene(&mut project, "main".into()).unwrap();
        let registry = enigma_3d::postprocessing::stack::EffectRegistry::default();
        let mut effect = registry.get("enigma.bloom").unwrap().entry();
        effect.parameters.insert("threshold".into(), ParameterValue::Float(0.42));
        effect.enabled = false;
        let mut snapshot: AppStateSerializer = serde_json::from_str(r#"{"camera":null,"light":[],"ambient_light":null,"skybox":null,"materials":[],"skybox_texture":null,"objects":[],"object_selection":[]}"#).unwrap();
        snapshot.post_processing.push(effect.clone());
        save_snapshot(&project, snapshot).unwrap();
        let text = fs::read_to_string(tmp.path().join("src/resources/scenes/main.json")).unwrap();
        let saved: AppStateSerializer = serde_json::from_str(&text).unwrap();
        assert_eq!(saved.post_processing, vec![effect]);
        let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
        legacy.as_object_mut().unwrap().remove("post_processing");
        let legacy: AppStateSerializer = serde_json::from_value(legacy).unwrap();
        assert_eq!(legacy.post_processing, Vec::<EffectEntry>::new());
    }

    fn make_project(tmp: &Path) -> ProjectState {
        let p = ProjectState::new("t".into(), tmp.to_string_lossy().into_owned());
        fs::create_dir_all(tmp.join("src/resources/scenes")).unwrap();
        p
    }

    #[test]
    fn new_scene_writes_file_and_appends() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        let uuid = new_scene(&mut project, "level1".into()).unwrap();
        assert_eq!(project.scenes.len(), 1);
        assert_eq!(project.scenes[0].uuid, uuid);
        assert_eq!(project.scenes[0].relative_path, "scenes/level1.json");
        assert!(tmp.path().join("src/resources/scenes/level1.json").is_file());
    }

    #[test]
    fn new_scene_sanitizes_name() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        new_scene(&mut project, "hello world!".into()).unwrap();
        assert_eq!(project.scenes[0].relative_path, "scenes/hello_world_.json");
    }

    #[test]
    fn cannot_delete_only_scene() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        new_scene(&mut project, "main".into()).unwrap();
        match delete(&mut project, 0) {
            Err(SceneError::CannotDeleteLast) => (),
            other => panic!("expected CannotDeleteLast, got {other:?}"),
        }
    }

    #[test]
    fn new_scene_collision_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        new_scene(&mut project, "main".into()).unwrap();
        match new_scene(&mut project, "main".into()) {
            Err(SceneError::AlreadyExists) => (),
            other => panic!("expected AlreadyExists, got {other:?}"),
        }
    }

    #[test]
    fn normalize_animation_times_zeros_time_on_serializer() {
        use enigma_3d::object::{Object, ObjectSerializer};
        use enigma_3d::animation::Animation;

        // Build an object with a played animation at time=1.7s.
        let mut obj = Object::new(Some("rig".into()));
        let anim = Animation { name: "Walk".into(), duration: 3.0, channels: Vec::new() };
        obj.get_animations_mut().insert("Walk".into(), anim);
        obj.play_animation("Walk", true);
        if let Some(state) = obj.get_current_animation_mut().as_mut() {
            state.time = 1.7;
            state.speed = 1.0;
        }

        let mut serializers: Vec<ObjectSerializer> = vec![obj.to_serializer()];
        normalize_animation_times(&mut serializers);

        let state = serializers[0].current_animation.as_ref().expect("Some");
        assert_eq!(state.time, 0.0);
        assert_eq!(state.name, "Walk");
        assert_eq!(state.speed, 1.0);
        assert_eq!(state.looping, true);
    }

    #[test]
    fn delete_removes_entry_and_trashes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        new_scene(&mut project, "main".into()).unwrap();
        new_scene(&mut project, "level2".into()).unwrap();
        let path = tmp.path().join("src/resources/scenes/level2.json");
        assert!(path.is_file());
        delete(&mut project, 1).unwrap();
        assert_eq!(project.scenes.len(), 1);
        assert!(!path.exists());
        assert!(tmp.path().join(".trash/scene_level2.json").is_file());
    }
}
