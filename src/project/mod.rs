pub mod resource;
pub mod scene;
pub mod material;

use std::fs;
use std::path::Path;
use enigma_3d::{AppState, AppStateSerializer};
use uuid::Uuid;

use crate::editor::state::{EditorRoot, ProjectState, SceneRef};

const PROJECT_FILE: &str = "enigma_project.json";

pub fn try_new_project(path: &str, app_state: &mut AppState) -> Result<(), ProjectError> {
    let path = path.replace('\\', "/");
    if !check_empty_directory(&path)? {
        return Err(ProjectError::DirectoryNotEmpty);
    }
    let project_name = Path::new(&path).file_name()
        .and_then(|n| n.to_str())
        .ok_or(ProjectError::BadPath)?
        .to_string();

    let mut project = ProjectState::new(project_name.clone(), path.clone());

    create_folder_struct(&path, &project_name)?;

    let scene_uuid = Uuid::new_v4();
    project.scenes.push(SceneRef {
        uuid: scene_uuid,
        name: "main".into(),
        relative_path: "scenes/main.json".into(),
    });
    project.active_scene_index = 0;
    project.startup_scene_index = 0;

    write_project_file(&path, &project)?;

    let root = app_state.get_state_data_value_mut::<EditorRoot>("editor")
        .ok_or(ProjectError::EditorRootMissing)?;
    root.project = Some(project);
    Ok(())
}

pub fn try_open_project(path: &str, app_state: &mut AppState) -> Result<(), ProjectError> {
    let path = path.replace('\\', "/");
    if !is_valid_project_file(&path) {
        return Err(ProjectError::InvalidProjectFile);
    }
    let text = fs::read_to_string(&path).map_err(ProjectError::Io)?;
    let mut project: ProjectState = serde_json::from_str(&text).map_err(ProjectError::Parse)?;

    let root_dir = Path::new(&path).parent()
        .and_then(|p| p.to_str())
        .ok_or(ProjectError::BadPath)?
        .to_string();
    project.root_path = root_dir.clone();

    let active_scene_path = project.scenes.get(project.active_scene_index)
        .map(|s| Path::new(&root_dir).join("src/resources").join(&s.relative_path));

    let root = app_state.get_state_data_value_mut::<EditorRoot>("editor")
        .ok_or(ProjectError::EditorRootMissing)?;
    root.project = Some(project);
    root.editor.material_cache.clear();

    if let Some(path) = active_scene_path {
        if let Err(e) = inject_scene_file(app_state, &path) {
            eprintln!("warning: failed to load active scene: {e}");
        }
    }

    Ok(())
}

fn inject_scene_file(app_state: &mut AppState, path: &Path) -> Result<(), ProjectError> {
    if !path.is_file() {
        return Ok(());
    }
    let text = fs::read_to_string(path).map_err(ProjectError::Io)?;
    if text.trim().is_empty() || text.trim() == "{}" {
        clear_scene(app_state);
        return Ok(());
    }
    let serializer: AppStateSerializer = serde_json::from_str(&text).map_err(ProjectError::Parse)?;
    let display = app_state.display.clone().ok_or(ProjectError::NoDisplay)?;
    clear_scene(app_state);
    app_state.inject_serializer(serializer, display, false);
    Ok(())
}

fn clear_scene(app_state: &mut AppState) {
    app_state.objects.clear();
    app_state.light.clear();
    app_state.materials.clear();
}

pub fn try_save_project(app_state: &mut AppState) -> Result<(), ProjectError> {
    let root = app_state.get_state_data_value_mut::<EditorRoot>("editor")
        .ok_or(ProjectError::EditorRootMissing)?;
    let project = root.project.as_ref().ok_or(ProjectError::NoProject)?;
    write_project_file(&project.root_path, project)
}

pub fn is_valid_project_file(path: &str) -> bool {
    let p = Path::new(path);
    p.is_file() && p.file_name().and_then(|n| n.to_str()) == Some(PROJECT_FILE)
}

fn check_empty_directory(path: &str) -> Result<bool, ProjectError> {
    let dir = Path::new(path);
    if !dir.is_dir() { return Ok(false); }
    let mut entries = fs::read_dir(dir).map_err(ProjectError::Io)?;
    Ok(entries.next().is_none())
}

fn create_folder_struct(path: &str, project_name: &str) -> Result<(), ProjectError> {
    let project_dir = Path::new(path);
    for sub in ["src/resources/textures", "src/resources/models", "src/resources/shader",
                "src/resources/scenes", "src/resources/audio", "src/resources/other"] {
        fs::create_dir_all(project_dir.join(sub)).map_err(ProjectError::Io)?;
    }

    let cargo = crate::resources::CARGO_TOML.replace("ENIGMA_PROJECT_NAME", project_name);
    fs::write(project_dir.join("Cargo.toml"), cargo).map_err(ProjectError::Io)?;

    let main_rs = crate::resources::MAIN_RS.replace("ENIGMA_PROJECT_NAME", project_name);
    fs::write(project_dir.join("src/main.rs"), main_rs).map_err(ProjectError::Io)?;

    fs::write(project_dir.join("src/resources/scenes/main.json"), "{}").map_err(ProjectError::Io)?;
    Ok(())
}

fn write_project_file(root: &str, project: &ProjectState) -> Result<(), ProjectError> {
    let text = serde_json::to_string_pretty(project).map_err(ProjectError::Parse)?;
    fs::write(Path::new(root).join(PROJECT_FILE), text).map_err(ProjectError::Io)?;
    Ok(())
}

#[derive(Debug)]
pub enum ProjectError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    DirectoryNotEmpty,
    InvalidProjectFile,
    BadPath,
    EditorRootMissing,
    NoProject,
    NoDisplay,
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::Io(e) => write!(f, "I/O error: {e}"),
            ProjectError::Parse(e) => write!(f, "JSON error: {e}"),
            ProjectError::DirectoryNotEmpty => write!(f, "directory is not empty"),
            ProjectError::InvalidProjectFile => write!(f, "not a valid enigma_project.json"),
            ProjectError::BadPath => write!(f, "invalid path"),
            ProjectError::EditorRootMissing => write!(f, "internal: editor root not initialized"),
            ProjectError::NoProject => write!(f, "no project loaded"),
            ProjectError::NoDisplay => write!(f, "internal: display not yet ready"),
        }
    }
}
