# Scene Authoring Core — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the editor's first usable form: docked-panel UI on enigma-3d, multi-scene authoring, resource manifest with uuids, material editing with textures, light authoring, save/load/play.

**Architecture:** Editor is itself an enigma-3d app. New `editor/` and `project/` modules replace the current flat `Engine` struct. `ProjectState` (serializable) + `EditorState` (transient) sit in `app_state.state_data["editor"]`. Docked egui layout (top toolbar, left hierarchy, right inspector, bottom resource browser) over a transparent central panel that lets enigma's framebuffer show through.

**Tech Stack:** Rust 2021, enigma-3d (git), egui 0.23 + egui_glium + egui-winit, rfd, async-std, serde + serde_json, uuid, bincode.

**Spec:** `docs/superpowers/specs/2026-05-25-scene-authoring-core-design.md`

---

## Task ordering rationale

Tasks 1–6 land the pure-data foundation (no UI) with full TDD coverage. Tasks 7–11 build the docked UI shell on top. Tasks 12–17 fill the per-section inspector and material editor. Tasks 18–22 finish drag-drop, ray-pick, and modal flows. Task 23 is a manual smoke-test pass against the spec's acceptance criteria.

Commits are frequent — every task ends with `git commit`. Each task is independently reviewable.

---

## Task 1: Add dependencies and create module skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `src/editor/mod.rs`, `src/editor/state.rs`, `src/editor/actions.rs`
- Create: `src/editor/panels/mod.rs`
- Create: `src/editor/inspector/mod.rs`
- Create: `src/project/mod.rs`, `src/project/resource.rs`, `src/project/scene.rs`, `src/project/material.rs`
- Modify: `src/main.rs` (declare new modules)
- Delete: `src/serialization.rs` (empty), `src/ui/` (will be replaced)
- Delete: `src/ui/mod.rs`, `src/ui/project_window.rs`, `src/ui/resource_window.rs` (will be replaced)

- [ ] **Step 1.1: Add uuid crate to Cargo.toml**

```toml
uuid = { version = "1.10", features = ["v4", "serde"] }
```

Add after the existing `serde_json` line.

- [ ] **Step 1.2: Run cargo check to confirm clean baseline**

```bash
cargo check
```
Expected: clean (warnings about unused imports OK if pre-existing).

- [ ] **Step 1.3: Create empty module files with minimal stubs**

Each new module file starts with `// stub` and is otherwise empty. The `mod.rs` files declare their submodules:

`src/editor/mod.rs`:
```rust
pub mod state;
pub mod actions;
pub mod panels;
pub mod inspector;
```

`src/editor/panels/mod.rs`:
```rust
// panels added in later tasks
```

`src/editor/inspector/mod.rs`:
```rust
// sections added in later tasks
```

`src/project/mod.rs`:
```rust
pub mod resource;
pub mod scene;
pub mod material;
// project lifecycle (new/open/save) added in task 3
```

`src/editor/state.rs`, `src/editor/actions.rs`, `src/project/resource.rs`, `src/project/scene.rs`, `src/project/material.rs`: each starts with `// stub`.

- [ ] **Step 1.4: Move main.rs into module declarations**

Replace existing `mod ui; mod project; mod resources; mod serialization;` with:

```rust
mod editor;
mod project;
mod resources;
```

The rest of `main.rs` will be reshaped in later tasks. For this task it still compiles by keeping the existing `Engine` struct + `main()` body, just stop referencing `ui` and `serialization`. Update `app_state.inject_gui(...)` line to a placeholder closure that calls nothing:

```rust
app_state.inject_gui(Arc::new(|_ctx, _state| {}));
```

- [ ] **Step 1.5: Delete obsolete files**

```bash
rm src/serialization.rs
rm src/ui/mod.rs src/ui/project_window.rs src/ui/resource_window.rs
rmdir src/ui
```

- [ ] **Step 1.6: Run cargo check**

```bash
cargo check
```
Expected: clean. Existing `Engine` struct stays; project still launches a window with no UI.

- [ ] **Step 1.7: Commit**

```bash
git add -A
git commit -m "scaffold editor and project module trees, add uuid dep"
```

---

## Task 2: Define ProjectState and related types

**Files:**
- Modify: `src/editor/state.rs`

This task defines all the serializable + transient data types from the spec (§Data model) without wiring them into the runtime. Pure type definitions + a roundtrip test.

- [ ] **Step 2.1: Write the types into `src/editor/state.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProjectState {
    pub name: String,
    pub root_path: String,
    pub manifest: Vec<ResourceEntry>,
    pub scenes: Vec<SceneRef>,
    pub active_scene_index: usize,
    pub startup_scene_index: usize,
    pub materials: Vec<MaterialDef>,
    // (scene_uuid, object_uuid) -> material_uuid. Scene-level data persisted in project file
    // because enigma_3d's ObjectSerializer doesn't carry editor material uuids.
    pub material_assignments: HashMap<(Uuid, Uuid), Uuid>,
}

impl ProjectState {
    pub fn new(name: String, root_path: String) -> Self {
        ProjectState {
            name,
            root_path,
            manifest: Vec::new(),
            scenes: Vec::new(),
            active_scene_index: 0,
            startup_scene_index: 0,
            materials: Vec::new(),
            material_assignments: HashMap::new(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ResourceEntry {
    pub uuid: Uuid,
    pub name: String,
    pub kind: ResourceKind,
    pub relative_path: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum ResourceKind { Model, Texture, Shader, Audio, Other }

impl ResourceKind {
    pub fn dir_name(&self) -> &'static str {
        match self {
            ResourceKind::Model => "models",
            ResourceKind::Texture => "textures",
            ResourceKind::Shader => "shader",
            ResourceKind::Audio => "audio",
            ResourceKind::Other => "other",
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SceneRef {
    pub uuid: Uuid,
    pub name: String,
    pub relative_path: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct MaterialDef {
    pub uuid: Uuid,
    pub name: String,
    pub shader: ShaderChoice,
    pub albedo: Option<Uuid>,
    pub normal: Option<Uuid>,
    pub roughness: Option<Uuid>,
    pub metallic: Option<Uuid>,
    pub emissive: Option<Uuid>,
    pub color: [f32; 3],
    pub emissive_strength: f32,
    pub roughness_strength: f32,
    pub metallic_strength: f32,
    pub normal_strength: f32,
    pub transparent: bool,
    pub transparency_strength: f32,
}

impl MaterialDef {
    pub fn default_pbr(name: String) -> Self {
        MaterialDef {
            uuid: Uuid::new_v4(),
            name,
            shader: ShaderChoice::PbrLit,
            albedo: None, normal: None, roughness: None, metallic: None, emissive: None,
            color: [1.0, 1.0, 1.0],
            emissive_strength: 0.0,
            roughness_strength: 0.5,
            metallic_strength: 0.0,
            normal_strength: 1.0,
            transparent: false,
            transparency_strength: 1.0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ShaderChoice { PbrLit, Unlit, Custom(Uuid) }

// --- Transient editor state (never serialized) ---

pub struct EditorRoot {
    pub project: Option<ProjectState>,
    pub editor: EditorState,
}

impl EditorRoot {
    pub fn empty() -> Self {
        EditorRoot { project: None, editor: EditorState::default() }
    }
}

#[derive(Default)]
pub struct EditorState {
    pub selection: Selection,
    pub viewport_rect: Option<egui::Rect>,
    pub drag: Option<DragPayload>,
    pub modal: Option<Modal>,
    pub resource_browser_tab: ResourceTab,
    pub dirty: bool,
    pub renaming: Option<RenameTarget>,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub enum Selection {
    #[default] None,
    SceneObject(Uuid),
    Light(Uuid),
    Camera,
    Material(Uuid),
    Resource(Uuid),
}

#[derive(Clone, PartialEq, Debug)]
pub enum DragPayload {
    Resource(Uuid),
    Material(Uuid),
}

#[derive(Clone, Debug)]
pub enum Modal {
    ConfirmDelete { label: String, pending: PendingDelete },
    ImportError(String),
    NewSceneName(String),
}

#[derive(Clone, Debug)]
pub enum PendingDelete {
    Resource(Uuid),
    Material(Uuid),
    Scene(usize),
    SceneObject(Uuid),
    Light(Uuid),
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum ResourceTab {
    #[default] Models,
    Textures,
    Shaders,
    Materials,
    Scenes,
    Audio,
    Other,
}

#[derive(Clone, Debug)]
pub enum RenameTarget {
    Resource { uuid: Uuid, draft: String },
    Material { uuid: Uuid, draft: String },
    Scene { index: usize, draft: String },
    SceneObject { uuid: Uuid, draft: String },
    Light { uuid: Uuid, draft: String },
}
```

- [ ] **Step 2.2: Write a roundtrip serialization test**

Add at the bottom of `src/editor/state.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_state_serde_roundtrip() {
        let mut p = ProjectState::new("test".into(), "/tmp/test".into());
        p.manifest.push(ResourceEntry {
            uuid: Uuid::new_v4(),
            name: "albedo".into(),
            kind: ResourceKind::Texture,
            relative_path: "textures/albedo.png".into(),
        });
        p.scenes.push(SceneRef {
            uuid: Uuid::new_v4(),
            name: "main".into(),
            relative_path: "scenes/main.json".into(),
        });
        p.materials.push(MaterialDef::default_pbr("Material 1".into()));
        let json = serde_json::to_string_pretty(&p).unwrap();
        let parsed: ProjectState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, p.name);
        assert_eq!(parsed.manifest, p.manifest);
        assert_eq!(parsed.scenes, p.scenes);
        assert_eq!(parsed.materials, p.materials);
    }

    #[test]
    fn shader_choice_variants_roundtrip() {
        for choice in [ShaderChoice::PbrLit, ShaderChoice::Unlit, ShaderChoice::Custom(Uuid::new_v4())] {
            let json = serde_json::to_string(&choice).unwrap();
            let parsed: ShaderChoice = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, choice);
        }
    }

    #[test]
    fn resource_kind_dir_names() {
        assert_eq!(ResourceKind::Model.dir_name(), "models");
        assert_eq!(ResourceKind::Texture.dir_name(), "textures");
        assert_eq!(ResourceKind::Shader.dir_name(), "shader");
        assert_eq!(ResourceKind::Audio.dir_name(), "audio");
        assert_eq!(ResourceKind::Other.dir_name(), "other");
    }
}
```

- [ ] **Step 2.3: Run tests**

```bash
cargo test --lib editor::state
```
Expected: 3 passing tests.

Note: this requires `main.rs` to be a binary that can be tested. If `cargo test` complains, the test setup may need an `[lib]` section in Cargo.toml or moving these types into a `lib.rs`. Resolve by either: (a) running tests via `cargo test --bin enigma-engine`, or (b) moving shared code into a library target. Prefer (a) for minimal disruption.

- [ ] **Step 2.4: Commit**

```bash
git add src/editor/state.rs
git commit -m "data model: ProjectState, ResourceEntry, MaterialDef, EditorState"
```

---

## Task 3: Refactor project lifecycle to use ProjectState

**Files:**
- Modify: `src/project/mod.rs`
- Delete: existing `src/project.rs` body (the `try_new_project` / `try_load_project` / `try_save_project` move here)
- Modify: `src/main.rs` (remove old `Engine` struct, use `EditorRoot`)

The current `Engine` struct in `main.rs` gets removed. `state_data` key changes from `"engine"` to `"editor"` holding an `EditorRoot`.

- [ ] **Step 3.1: Write the new project lifecycle functions**

`src/project/mod.rs`:

```rust
pub mod resource;
pub mod scene;
pub mod material;

use std::fs;
use std::path::Path;
use enigma_3d::AppState;
use serde::{Deserialize, Serialize};
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

    // scaffold cargo project + resource subdirs
    create_folder_struct(&path, &project_name)?;

    // initial scene
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

    // normalize root_path to the directory of the project file
    let root_dir = Path::new(&path).parent()
        .and_then(|p| p.to_str())
        .ok_or(ProjectError::BadPath)?
        .to_string();
    project.root_path = root_dir;

    let root = app_state.get_state_data_value_mut::<EditorRoot>("editor")
        .ok_or(ProjectError::EditorRootMissing)?;
    root.project = Some(project);
    Ok(())
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
        }
    }
}
```

- [ ] **Step 3.2: Delete the obsolete `src/project.rs`**

```bash
rm src/project.rs
```

- [ ] **Step 3.3: Rewrite main.rs**

```rust
mod editor;
mod project;
mod resources;

use std::sync::Arc;
use enigma_3d::AppState;
use enigma_3d::camera::Camera;

use crate::editor::state::EditorRoot;

fn main() {
    let mut app_state = AppState::new();
    let event_loop = enigma_3d::EventLoop::new("Enigma 3d - Engine", 1080, 720);
    event_loop.set_icon_from_resource(resources::ICON);

    let camera = Camera::new(None, None, Some(60.0), Some(16.0/9.0), Some(0.1), Some(100.0));
    app_state.set_camera(camera);

    app_state.add_state_data("editor", Box::new(EditorRoot::empty()));
    app_state.inject_gui(Arc::new(editor::draw));

    event_loop.run(app_state.convert_to_arc_mutex());
}
```

- [ ] **Step 3.4: Add a top-level editor draw stub**

`src/editor/mod.rs`:

```rust
pub mod state;
pub mod actions;
pub mod panels;
pub mod inspector;

use egui::Context;
use enigma_3d::AppState;

pub fn draw(_ctx: &Context, _app_state: &mut AppState) {
    // panels populated in later tasks
}
```

- [ ] **Step 3.5: Update `src/resources/mod.rs` — drop obsolete BinaryResource/TextResource**

```rust
pub const CARGO_TOML: &str = include_str!("Cargo.toml.resource");
pub const MAIN_RS: &str = include_str!("main.rs.resource");
pub const ICON: &'static [u8] = include_bytes!("icon.png.resource");
```

Delete the rest (BinaryResource, TextResource, import_resource_binary, import_resource_text, ResourceType). They were dead code from the old design — the new ResourceKind in editor/state.rs supersedes them.

- [ ] **Step 3.6: Build**

```bash
cargo check
```
Expected: clean.

- [ ] **Step 3.7: Manual smoke**

```bash
cargo run
```
Expected: window opens with title "Enigma 3d - Engine", empty viewport. No crash. Close window.

- [ ] **Step 3.8: Commit**

```bash
git add -A
git commit -m "refactor: Engine struct -> EditorRoot, project lifecycle uses ProjectState"
```

---

## Task 4: Resource pipeline (import / find / bytes / delete)

**Files:**
- Modify: `src/project/resource.rs`

- [ ] **Step 4.1: Write the failing tests first**

`src/project/resource.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::editor::state::{ProjectState, ResourceEntry, ResourceKind};

pub fn import(src_path: &Path, kind: ResourceKind, project: &mut ProjectState) -> Result<Uuid, ImportError> {
    if !src_path.is_file() {
        return Err(ImportError::SourceNotFound);
    }
    let file_name = src_path.file_name()
        .and_then(|n| n.to_str())
        .ok_or(ImportError::BadSourcePath)?;
    let target_dir = Path::new(&project.root_path).join("src/resources").join(kind.dir_name());
    fs::create_dir_all(&target_dir).map_err(ImportError::Io)?;

    let target_path = unique_target(&target_dir, file_name);
    fs::copy(src_path, &target_path).map_err(ImportError::Io)?;

    let relative_path = format!(
        "{}/{}",
        kind.dir_name(),
        target_path.file_name().unwrap().to_str().unwrap()
    );
    let uuid = Uuid::new_v4();
    project.manifest.push(ResourceEntry {
        uuid,
        name: file_stem(&target_path),
        kind,
        relative_path,
    });
    Ok(uuid)
}

pub fn find(project: &ProjectState, uuid: Uuid) -> Option<&ResourceEntry> {
    project.manifest.iter().find(|e| e.uuid == uuid)
}

pub fn bytes(project: &ProjectState, uuid: Uuid) -> Result<Vec<u8>, BytesError> {
    let entry = find(project, uuid).ok_or(BytesError::NotFound)?;
    let path = Path::new(&project.root_path).join("src/resources").join(&entry.relative_path);
    fs::read(&path).map_err(BytesError::Io)
}

pub fn delete(project: &mut ProjectState, uuid: Uuid) -> Result<(), DeleteError> {
    let idx = project.manifest.iter().position(|e| e.uuid == uuid)
        .ok_or(DeleteError::NotFound)?;
    let entry = &project.manifest[idx];

    let src = Path::new(&project.root_path).join("src/resources").join(&entry.relative_path);
    let trash_dir = Path::new(&project.root_path).join(".trash");
    fs::create_dir_all(&trash_dir).map_err(DeleteError::Io)?;
    let trash_path = unique_target(&trash_dir, entry.relative_path.rsplit('/').next().unwrap());
    if src.exists() {
        fs::rename(&src, &trash_path).map_err(DeleteError::Io)?;
    }

    project.manifest.remove(idx);
    Ok(())
}

fn unique_target(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() { return candidate; }
    let (stem, ext) = match file_name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (file_name.to_string(), String::new()),
    };
    for n in 2..1000 {
        let next = dir.join(format!("{stem}_{n}{ext}"));
        if !next.exists() { return next; }
    }
    dir.join(file_name)  // give up, will overwrite
}

fn file_stem(p: &Path) -> String {
    p.file_stem().and_then(|s| s.to_str()).unwrap_or("unnamed").to_string()
}

#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    SourceNotFound,
    BadSourcePath,
}

#[derive(Debug)]
pub enum BytesError {
    NotFound,
    Io(std::io::Error),
}

#[derive(Debug)]
pub enum DeleteError {
    NotFound,
    Io(std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_project(tmp: &Path) -> ProjectState {
        ProjectState::new("test".into(), tmp.to_string_lossy().into_owned())
    }

    #[test]
    fn import_copies_file_and_adds_manifest_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());

        // create a fake source file
        let src = tmp.path().join("source.png");
        fs::write(&src, b"fake-png").unwrap();

        let uuid = import(&src, ResourceKind::Texture, &mut project).unwrap();

        assert_eq!(project.manifest.len(), 1);
        let entry = &project.manifest[0];
        assert_eq!(entry.uuid, uuid);
        assert_eq!(entry.kind, ResourceKind::Texture);
        assert_eq!(entry.name, "source");
        assert_eq!(entry.relative_path, "textures/source.png");
        assert!(tmp.path().join("src/resources/textures/source.png").is_file());
    }

    #[test]
    fn import_collision_appends_suffix() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        let src = tmp.path().join("source.png");
        fs::write(&src, b"a").unwrap();

        import(&src, ResourceKind::Texture, &mut project).unwrap();
        import(&src, ResourceKind::Texture, &mut project).unwrap();

        assert_eq!(project.manifest[0].relative_path, "textures/source.png");
        assert_eq!(project.manifest[1].relative_path, "textures/source_2.png");
    }

    #[test]
    fn import_missing_source_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        let bogus = tmp.path().join("nope.png");
        match import(&bogus, ResourceKind::Texture, &mut project) {
            Err(ImportError::SourceNotFound) => (),
            other => panic!("expected SourceNotFound, got {other:?}"),
        }
    }

    #[test]
    fn bytes_reads_file_via_uuid() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        let src = tmp.path().join("source.png");
        fs::write(&src, b"hello").unwrap();
        let uuid = import(&src, ResourceKind::Texture, &mut project).unwrap();

        let data = bytes(&project, uuid).unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn delete_moves_to_trash_and_removes_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let mut project = make_project(tmp.path());
        let src = tmp.path().join("source.png");
        fs::write(&src, b"x").unwrap();
        let uuid = import(&src, ResourceKind::Texture, &mut project).unwrap();

        delete(&mut project, uuid).unwrap();
        assert_eq!(project.manifest.len(), 0);
        assert!(!tmp.path().join("src/resources/textures/source.png").exists());
        assert!(tmp.path().join(".trash/source.png").exists());
    }
}
```

- [ ] **Step 4.2: Add `tempfile` to dev-dependencies**

`Cargo.toml`:
```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 4.3: Run tests**

```bash
cargo test --bin enigma-engine project::resource
```
Expected: 5 passing tests.

- [ ] **Step 4.4: Commit**

```bash
git add -A
git commit -m "feat: resource import/find/bytes/delete with tests"
```

---

## Task 5: Scene management (switch / new / delete / save-active)

**Files:**
- Modify: `src/project/scene.rs`

This task wires multi-scene support. enigma-3d's `inject_serializer` is the main API; we drive it.

- [ ] **Step 5.1: Write the scene module**

`src/project/scene.rs`:

```rust
use std::fs;
use std::path::Path;
use enigma_3d::{AppState, AppStateSerializer};
use glium::Display;
use glium::glutin::surface::WindowSurface;
use uuid::Uuid;

use crate::editor::state::{ProjectState, SceneRef};

pub fn save_active(project: &ProjectState, app_state: &AppState) -> Result<(), SceneError> {
    let scene = project.scenes.get(project.active_scene_index).ok_or(SceneError::NoActiveScene)?;
    let path = scene_path(project, scene);
    let serializer = app_state.to_serializer();
    let text = serde_json::to_string_pretty(&serializer).map_err(SceneError::Parse)?;
    fs::write(&path, text).map_err(SceneError::Io)?;
    Ok(())
}

pub fn switch(
    project: &mut ProjectState,
    app_state: &mut AppState,
    display: &Display<WindowSurface>,
    target_index: usize,
) -> Result<(), SceneError> {
    save_active(project, app_state).ok();   // best-effort — failing save shouldn't block switch

    let scene = project.scenes.get(target_index).ok_or(SceneError::BadIndex)?.clone();
    let path = scene_path(project, &scene);
    let text = fs::read_to_string(&path).map_err(SceneError::Io)?;
    let serializer: AppStateSerializer = serde_json::from_str(&text).map_err(SceneError::Parse)?;

    clear_app_state(app_state);
    app_state.inject_serializer(serializer, display.clone(), /*additive=*/false);
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
    fs::write(&target, "{}").map_err(SceneError::Io)?;

    let uuid = Uuid::new_v4();
    project.scenes.push(SceneRef { uuid, name, relative_path });
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

fn clear_app_state(app_state: &mut AppState) {
    // enigma's AppState has Vec<Object> and Vec<Light> directly. Reset to empty.
    app_state.objects.clear();
    app_state.light.clear();
    app_state.materials.clear();
    // skybox, ambient light, camera stay (camera is editor-provided)
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
}
```

- [ ] **Step 5.2: Check enigma-3d's AppState field visibility**

The `clear_app_state` helper assumes `objects`, `light`, `materials` are public fields. From the earlier grep:
- `pub fn add_object(...)` and `pub fn add_light(...)` exist
- Fields may be private

```bash
grep -E "^    pub|pub objects|pub light|pub materials" /Users/cg-jm/jm/enigma-3d/src/lib.rs | head -20
```

If fields are private, either:
- (a) Add `pub fn clear_objects/clear_lights/clear_materials` to enigma-3d (preferred — small change to the framework)
- (b) Replace via `*app_state = AppState::new()` + re-attach camera

Plan: open enigma-3d and add minimal `pub fn` accessors if needed. Document the change here. If a small enigma-3d edit is required, make that the FIRST commit of this task — separately:

```bash
cd /Users/cg-jm/jm/enigma-3d
git checkout -b add-scene-clear-helpers
# add clear methods
git commit -m "add AppState clear_scene helpers"
# push and update enigma-engine Cargo.toml to a sha if needed
```

For v1, prefer (a) with `pub fn clear_scene(&mut self)` that clears objects/light/materials only.

If enigma-3d changes are required, do them first in their own commit (separate repo) before continuing.

- [ ] **Step 5.3: Tests for sanitize and new_scene**

Add at the bottom of `scene.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::state::ProjectState;

    fn make_project(tmp: &Path) -> ProjectState {
        let mut p = ProjectState::new("t".into(), tmp.to_string_lossy().into_owned());
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
}
```

- [ ] **Step 5.4: Run tests**

```bash
cargo test --bin enigma-engine project::scene
```
Expected: 3 passing tests.

- [ ] **Step 5.5: Commit**

```bash
git add -A
git commit -m "feat: scene switch/new/delete with tests"
```

---

## Task 6: Material realization (MaterialDef → enigma_3d::Material)

**Files:**
- Modify: `src/project/material.rs`

- [ ] **Step 6.1: Write the material realize/reconcile functions**

`src/project/material.rs`:

```rust
use enigma_3d::{AppState, material::Material};
use enigma_3d::texture::TextureType;
use enigma_3d::shader::Shader;
use glium::Display;
use glium::glutin::surface::WindowSurface;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

use crate::editor::state::{MaterialDef, ProjectState, ShaderChoice};
use crate::project::resource;

pub fn realize(
    def: &MaterialDef,
    project: &ProjectState,
    display: Display<WindowSurface>,
) -> Result<Material, RealizeError> {
    let mut mat = match &def.shader {
        ShaderChoice::PbrLit => Material::lit_pbr(display.clone(), def.transparent),
        ShaderChoice::Unlit => Material::unlit(display.clone(), def.transparent),
        ShaderChoice::Custom(shader_uuid) => {
            let bytes = resource::bytes(project, *shader_uuid)
                .map_err(|_| RealizeError::ShaderNotFound)?;
            // Treat as combined .glsl — exact loader depends on enigma's API.
            // Fall back to PBR if shader loading is more involved than v1 allows.
            let _ = bytes;  // TODO when custom shaders land
            Material::lit_pbr(display.clone(), def.transparent)
        }
    };
    mat.set_name(&def.name);
    mat.set_color(def.color);
    mat.set_emissive_strength(def.emissive_strength);
    mat.set_roughness_strength(def.roughness_strength);
    mat.set_metallic_strength(def.metallic_strength);
    mat.set_normal_strength(def.normal_strength);
    if def.transparent {
        mat.set_transparency_strength(def.transparency_strength);
    }

    apply_texture(&mut mat, project, def.albedo,    TextureType::Albedo)?;
    apply_texture(&mut mat, project, def.normal,    TextureType::Normal)?;
    apply_texture(&mut mat, project, def.roughness, TextureType::Roughness)?;
    apply_texture(&mut mat, project, def.metallic,  TextureType::Metallic)?;
    apply_texture(&mut mat, project, def.emissive,  TextureType::Emissive)?;

    Ok(mat)
}

fn apply_texture(
    mat: &mut Material,
    project: &ProjectState,
    slot: Option<Uuid>,
    kind: TextureType,
) -> Result<(), RealizeError> {
    if let Some(uuid) = slot {
        let bytes = resource::bytes(project, uuid).map_err(|_| RealizeError::TextureNotFound)?;
        mat.set_texture_from_resource(&bytes, kind);
    }
    Ok(())
}

pub fn material_hash(def: &MaterialDef) -> u64 {
    let mut h = DefaultHasher::new();
    def.name.hash(&mut h);
    format!("{:?}", def.shader).hash(&mut h);
    def.albedo.hash(&mut h);
    def.normal.hash(&mut h);
    def.roughness.hash(&mut h);
    def.metallic.hash(&mut h);
    def.emissive.hash(&mut h);
    // f32 doesn't impl Hash — hash the bit pattern
    def.color.iter().for_each(|f| f.to_bits().hash(&mut h));
    def.emissive_strength.to_bits().hash(&mut h);
    def.roughness_strength.to_bits().hash(&mut h);
    def.metallic_strength.to_bits().hash(&mut h);
    def.normal_strength.to_bits().hash(&mut h);
    def.transparent.hash(&mut h);
    def.transparency_strength.to_bits().hash(&mut h);
    h.finish()
}

#[derive(Debug)]
pub enum RealizeError {
    TextureNotFound,
    ShaderNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_hash_stable_for_equal_defs() {
        let a = MaterialDef::default_pbr("m".into());
        let mut b = a.clone();
        assert_eq!(material_hash(&a), material_hash(&b));

        b.color = [0.5, 0.5, 0.5];
        assert_ne!(material_hash(&a), material_hash(&b));
    }
}
```

- [ ] **Step 6.2: Verify enigma-3d TextureType variants**

```bash
grep -A8 "pub enum TextureType" /Users/cg-jm/jm/enigma-3d/src/material.rs
```

Expected variants: `Albedo`, `Normal`, `Roughness`, `Metallic`, `Emissive`. Adjust import if names differ.

- [ ] **Step 6.3: Note: `reconcile()` deferred to Task 17**

`reconcile()` lives in this module but is wired into the frame lifecycle later (Task 17 — material editor). Defining only `realize` and `material_hash` here is sufficient.

- [ ] **Step 6.4: Run tests**

```bash
cargo test --bin enigma-engine project::material
```
Expected: 1 passing test.

- [ ] **Step 6.5: Commit**

```bash
git add -A
git commit -m "feat: MaterialDef -> enigma_3d::Material realization + hash"
```

---

## Task 7: Editor draw entry + docked layout skeleton

**Files:**
- Modify: `src/editor/mod.rs`
- Create: `src/editor/panels/toolbar.rs`
- Create: `src/editor/panels/hierarchy.rs`
- Create: `src/editor/panels/inspector.rs`
- Create: `src/editor/panels/resource_browser.rs`
- Create: `src/editor/panels/viewport.rs`
- Modify: `src/editor/panels/mod.rs`

- [ ] **Step 7.1: Wire panel modules**

`src/editor/panels/mod.rs`:

```rust
pub mod toolbar;
pub mod hierarchy;
pub mod inspector;
pub mod resource_browser;
pub mod viewport;
```

- [ ] **Step 7.2: Skeleton panels (each a no-op for now)**

Each of `toolbar.rs`, `hierarchy.rs`, `inspector.rs`, `resource_browser.rs`, `viewport.rs`:

```rust
// Example for toolbar.rs:
use egui::Context;
use enigma_3d::AppState;

pub fn draw(_ctx: &Context, _app_state: &mut AppState) {
    // populated in later tasks
}
```

Same shape for the other 4 panel files (just change the comment).

- [ ] **Step 7.3: Top-level draw orchestrates dock**

`src/editor/mod.rs`:

```rust
pub mod state;
pub mod actions;
pub mod panels;
pub mod inspector;

use egui::Context;
use enigma_3d::AppState;

pub fn draw(ctx: &Context, app_state: &mut AppState) {
    set_style(ctx);

    egui::TopBottomPanel::top("toolbar").show(ctx, |_ui| {
        panels::toolbar::draw(ctx, app_state);
    });

    egui::SidePanel::left("hierarchy")
        .default_width(220.0)
        .min_width(160.0)
        .show(ctx, |_ui| {
            panels::hierarchy::draw(ctx, app_state);
        });

    egui::SidePanel::right("inspector")
        .default_width(320.0)
        .min_width(240.0)
        .show(ctx, |_ui| {
            panels::inspector::draw(ctx, app_state);
        });

    egui::TopBottomPanel::bottom("resource_browser")
        .default_height(200.0)
        .min_height(120.0)
        .resizable(true)
        .show(ctx, |_ui| {
            panels::resource_browser::draw(ctx, app_state);
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            panels::viewport::draw(ctx, app_state, ui);
        });
}

fn set_style(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.window_shadow.extrusion = 0.0;
    style.visuals.window_shadow.color = egui::Color32::TRANSPARENT;
    style.visuals.window_stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
    ctx.set_style(style);
}
```

Update viewport panel signature to take `ui: &mut egui::Ui`:

`src/editor/panels/viewport.rs`:

```rust
use egui::{Context, Ui};
use enigma_3d::AppState;

use crate::editor::state::EditorRoot;

pub fn draw(_ctx: &Context, app_state: &mut AppState, ui: &mut Ui) {
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.viewport_rect = Some(ui.max_rect());
    }
}
```

But the other panels currently take `(_ctx, _app_state)` from inside `.show()`. That's wrong — egui's show callback gives you a `&mut Ui`, not raw context. Refactor the top-level `draw` to pass `ui` into each panel.

Final shape of editor::draw:

```rust
pub fn draw(ctx: &Context, app_state: &mut AppState) {
    set_style(ctx);

    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        panels::toolbar::draw(ui, app_state);
    });

    egui::SidePanel::left("hierarchy")
        .default_width(220.0).min_width(160.0).resizable(true)
        .show(ctx, |ui| { panels::hierarchy::draw(ui, app_state); });

    egui::SidePanel::right("inspector")
        .default_width(320.0).min_width(240.0).resizable(true)
        .show(ctx, |ui| { panels::inspector::draw(ui, app_state); });

    egui::TopBottomPanel::bottom("resource_browser")
        .default_height(200.0).min_height(120.0).resizable(true)
        .show(ctx, |ui| { panels::resource_browser::draw(ui, app_state); });

    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| { panels::viewport::draw(ui, app_state); });
}
```

And each panel becomes `pub fn draw(ui: &mut egui::Ui, app_state: &mut AppState)` taking the `Ui` directly.

- [ ] **Step 7.4: Run and verify dock layout**

```bash
cargo run
```
Expected: window opens with four empty panels around a transparent central area. No crash.

- [ ] **Step 7.5: Commit**

```bash
git add -A
git commit -m "feat: docked editor layout skeleton — 5 empty panels"
```

---

## Task 8: Toolbar panel — File menu + Project menu + Play/Build buttons

**Files:**
- Modify: `src/editor/panels/toolbar.rs`
- Modify: `src/editor/actions.rs`

Port the existing project lifecycle button logic from the old `project_window.rs` (now gone). File dialog flows via `rfd`.

- [ ] **Step 8.1: Write the actions for run/build**

`src/editor/actions.rs`:

```rust
use std::process::Command;
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
```

- [ ] **Step 8.2: Write the toolbar UI**

`src/editor/panels/toolbar.rs`:

```rust
use egui::Ui;
use enigma_3d::AppState;
use rfd::AsyncFileDialog;

use crate::editor::actions;
use crate::editor::state::EditorRoot;
use crate::project;

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.menu_button("File", |ui| {
            if ui.button("New Project").clicked() {
                pick_folder_and(|path| {
                    if let Err(e) = project::try_new_project(&path, app_state) {
                        eprintln!("new project failed: {e}");
                    }
                }, app_state);
                ui.close_menu();
            }
            if ui.button("Open Project").clicked() {
                pick_file_and("json", |path| {
                    if let Err(e) = project::try_open_project(&path, app_state) {
                        eprintln!("open project failed: {e}");
                    }
                }, app_state);
                ui.close_menu();
            }
            if ui.button("Save Project").clicked() {
                let _ = project::try_save_project(app_state);
                ui.close_menu();
            }
            if ui.button("Save Scene").clicked() {
                save_scene(app_state);
                ui.close_menu();
            }
        });

        // Play / Build buttons
        let project_loaded = app_state.get_state_data_value::<EditorRoot>("editor")
            .and_then(|r| r.project.as_ref()).is_some();

        ui.add_enabled_ui(project_loaded, |ui| {
            if ui.button("Play").clicked() {
                if let Some(p) = current_project_clone(app_state) {
                    actions::run_project(&p);
                }
            }
            if ui.button("Debug Build").clicked() {
                if let Some(p) = current_project_clone(app_state) {
                    actions::build_project(&p, false);
                }
            }
            if ui.button("Release Build").clicked() {
                if let Some(p) = current_project_clone(app_state) {
                    actions::build_project(&p, true);
                }
            }
        });

        // Status (right-aligned)
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") {
                if let Some(p) = &root.project {
                    if root.editor.dirty {
                        ui.label("•");
                    }
                    if let Some(scene) = p.scenes.get(p.active_scene_index) {
                        ui.label(format!("scene: {}", scene.name));
                    }
                    ui.label(format!("project: {}", p.name));
                } else {
                    ui.label("no project");
                }
            }
        });
    });
}

fn current_project_clone(app_state: &AppState) -> Option<crate::editor::state::ProjectState> {
    app_state.get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.clone())
}

fn save_scene(app_state: &mut AppState) {
    let project_clone = current_project_clone(app_state);
    if let Some(p) = project_clone {
        let _ = crate::project::scene::save_active(&p, app_state);
    }
}

fn pick_folder_and(handler: impl FnOnce(String), _app_state: &mut AppState) {
    let (tx, rx) = std::sync::mpsc::channel();
    let dialog = AsyncFileDialog::new().pick_folder();
    async_std::task::spawn(async move {
        if let Some(path) = dialog.await {
            let _ = tx.send(path.path().to_string_lossy().into_owned());
        }
    });
    if let Ok(path) = rx.recv() { handler(path); }
}

fn pick_file_and(filter: &str, handler: impl FnOnce(String), _app_state: &mut AppState) {
    let (tx, rx) = std::sync::mpsc::channel();
    let dialog = AsyncFileDialog::new().add_filter(filter, &[filter]).pick_file();
    async_std::task::spawn(async move {
        if let Some(path) = dialog.await {
            let _ = tx.send(path.path().to_string_lossy().into_owned());
        }
    });
    if let Ok(path) = rx.recv() { handler(path); }
}
```

**Borrow-checker note:** the closures passed to `pick_folder_and` / `pick_file_and` capture `app_state` mutably, which conflicts with the outer use. If `cargo check` errors here, refactor to: dialog returns the path; the outer code then calls the project function. The dialog helper signature changes from `(handler, app_state)` to `() -> Option<String>`.

If the closure capture is awkward, use this simpler pattern instead:

```rust
if ui.button("New Project").clicked() {
    if let Some(path) = pick_folder_blocking() {
        if let Err(e) = project::try_new_project(&path, app_state) {
            eprintln!("new project failed: {e}");
        }
    }
    ui.close_menu();
}
```

with `pick_folder_blocking() -> Option<String>` doing the rfd + recv inline. Prefer this — simpler.

- [ ] **Step 8.3: Pick the simpler pattern**

Final toolbar.rs uses inline dialog helpers:

```rust
fn pick_folder() -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    async_std::task::spawn(async move {
        if let Some(p) = AsyncFileDialog::new().pick_folder().await {
            let _ = tx.send(p.path().to_string_lossy().into_owned());
        }
    });
    rx.recv().ok()
}

fn pick_file(filter: &str) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let f = filter.to_string();
    async_std::task::spawn(async move {
        if let Some(p) = AsyncFileDialog::new().add_filter(&f, &[&f]).pick_file().await {
            let _ = tx.send(p.path().to_string_lossy().into_owned());
        }
    });
    rx.recv().ok()
}
```

And the button handlers become:

```rust
if ui.button("New Project").clicked() {
    if let Some(path) = pick_folder() {
        let _ = project::try_new_project(&path, app_state);
    }
    ui.close_menu();
}
```

- [ ] **Step 8.4: Build**

```bash
cargo check
cargo run
```
Expected: window opens with File menu. Clicking File → New Project opens a folder picker. Picking an empty dir creates a project; status bar updates.

- [ ] **Step 8.5: Commit**

```bash
git add -A
git commit -m "feat: toolbar with File menu, Play/Build, project status"
```

---

## Task 9: Hierarchy panel — list scene contents, selection

**Files:**
- Modify: `src/editor/panels/hierarchy.rs`

For v1 the hierarchy is three flat groups (Objects / Lights / Camera), no parenting. Click → selection.

- [ ] **Step 9.1: Write hierarchy listing**

`src/editor/panels/hierarchy.rs`:

```rust
use egui::Ui;
use enigma_3d::AppState;

use crate::editor::state::{EditorRoot, Selection};

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
    let project_loaded = root.project.is_some();
    drop(root);

    if !project_loaded {
        ui.label("No project loaded");
        return;
    }

    ui.heading("Hierarchy");
    ui.separator();

    // Snapshot the data we need (avoid keeping &mut borrows during nested egui)
    let object_rows: Vec<(uuid::Uuid, String)> = app_state.objects.iter()
        .map(|o| (o.get_unique_id(), o.name.clone()))
        .collect();
    let light_rows: Vec<(uuid::Uuid, String)> = app_state.light.iter()
        .map(|l| (l.uuid, format!("Light ({}, {}, {})", l.position[0], l.position[1], l.position[2])))
        .collect();
    let has_camera = app_state.camera.is_some();

    let current_selection = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.selection.clone())
        .unwrap_or(Selection::None);

    let mut new_selection: Option<Selection> = None;

    egui::CollapsingHeader::new(format!("Objects ({})", object_rows.len()))
        .default_open(true)
        .show(ui, |ui| {
            for (uuid, name) in &object_rows {
                let selected = matches!(current_selection, Selection::SceneObject(s) if s == *uuid);
                if ui.selectable_label(selected, name).clicked() {
                    new_selection = Some(Selection::SceneObject(*uuid));
                }
            }
        });

    egui::CollapsingHeader::new(format!("Lights ({})", light_rows.len()))
        .default_open(true)
        .show(ui, |ui| {
            for (uuid, name) in &light_rows {
                let selected = matches!(current_selection, Selection::Light(s) if s == *uuid);
                if ui.selectable_label(selected, name).clicked() {
                    new_selection = Some(Selection::Light(*uuid));
                }
            }
        });

    egui::CollapsingHeader::new("Camera")
        .default_open(true)
        .show(ui, |ui| {
            if has_camera {
                let selected = matches!(current_selection, Selection::Camera);
                if ui.selectable_label(selected, "Camera").clicked() {
                    new_selection = Some(Selection::Camera);
                }
            } else {
                ui.label("(no camera)");
            }
        });

    if let Some(sel) = new_selection {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.selection = sel;
        }
    }
}
```

- [ ] **Step 9.2: Verify enigma-3d Object/Light field access**

```bash
grep -n "pub.*uuid\|pub.*name\|get_unique_id\|pub.*position" /Users/cg-jm/jm/enigma-3d/src/object.rs /Users/cg-jm/jm/enigma-3d/src/light.rs | head -20
```

Adjust field/method names to match what's public. If `name` is private, add `pub fn name() -> &str` to enigma-3d as a small framework addition.

- [ ] **Step 9.3: Build and smoke test**

```bash
cargo run
```
Expected: with a new project, hierarchy shows "Objects (0) / Lights (0) / Camera (Camera)". Camera row clickable; toggles selection visibly.

- [ ] **Step 9.4: Commit**

```bash
git add -A
git commit -m "feat: hierarchy panel — list groups, click to select"
```

---

## Task 10: Hierarchy Add menu — empty / primitives / lights

**Files:**
- Modify: `src/editor/panels/hierarchy.rs`
- Modify: `src/editor/actions.rs`

- [ ] **Step 10.1: Add object/light spawn actions**

In `src/editor/actions.rs`, append:

```rust
use enigma_3d::AppState;
use enigma_3d::object::Object;
use enigma_3d::light::{Light, LightEmissionType};

pub enum ObjectTemplate {
    Empty,
    Cube,
    Sphere,
    Plane,
    Quad,
}

pub fn add_object(app_state: &mut AppState, template: ObjectTemplate) {
    let obj = match template {
        ObjectTemplate::Empty => Object::default(),
        ObjectTemplate::Cube => primitive_cube(),
        ObjectTemplate::Sphere => primitive_sphere(),
        ObjectTemplate::Plane => primitive_plane(),
        ObjectTemplate::Quad => primitive_quad(),
    };
    app_state.add_object(obj);
}

pub fn add_light(app_state: &mut AppState, kind: LightTemplate) {
    let light = match kind {
        LightTemplate::Directional => Light::new([0.0, 5.0, 0.0], [1.0, 1.0, 1.0], 1.0, Some([0.0, -1.0, 0.0]), true),
        LightTemplate::Point => Light::new([0.0, 1.0, 0.0], [1.0, 1.0, 1.0], 1.0, None, false),
        LightTemplate::Ambient => Light::new([0.0, 0.0, 0.0], [0.1, 0.1, 0.1], 1.0, None, false),
    };
    let emission = match kind {
        LightTemplate::Ambient => LightEmissionType::Ambient,
        _ => LightEmissionType::Source,
    };
    app_state.add_light(light, emission);
}

pub enum LightTemplate { Directional, Point, Ambient }

fn primitive_cube() -> Object { /* placeholder — enigma may provide */ todo!("cube") }
fn primitive_sphere() -> Object { todo!("sphere") }
fn primitive_plane() -> Object { todo!("plane") }
fn primitive_quad() -> Object { todo!("quad") }
```

- [ ] **Step 10.2: Resolve primitives**

Check if enigma-3d ships primitive helpers:

```bash
grep -rn "fn cube\|fn sphere\|fn plane\|fn quad\|primitive\|new_cube" /Users/cg-jm/jm/enigma-3d/src/ | head
```

If not, build minimal inline meshes (a 24-vertex cube is ~30 lines of `Vertex` data). For v1, prefer just **Empty** + **From Model…** and document that primitives land in Task 19 once we have model spawning. Remove the `Cube/Sphere/Plane/Quad` variants from the v1 menu — replace with "Empty" only initially.

Revised `ObjectTemplate` for v1:

```rust
pub enum ObjectTemplate { Empty }
```

The menu then offers only "Empty" + "From Model…" (model spawning lands in Task 19).

- [ ] **Step 10.3: Wire the Add menu into hierarchy.rs**

Inside the Objects collapsing header:

```rust
ui.menu_button("+ Add", |ui| {
    if ui.button("Empty").clicked() {
        actions::add_object(app_state, ObjectTemplate::Empty);
        ui.close_menu();
    }
    // "From Model…" populated in Task 19
});
```

And inside Lights collapsing header:

```rust
ui.menu_button("+ Add", |ui| {
    if ui.button("Directional").clicked() {
        actions::add_light(app_state, LightTemplate::Directional);
        ui.close_menu();
    }
    if ui.button("Point").clicked() {
        actions::add_light(app_state, LightTemplate::Point);
        ui.close_menu();
    }
    if ui.button("Ambient").clicked() {
        actions::add_light(app_state, LightTemplate::Ambient);
        ui.close_menu();
    }
});
```

- [ ] **Step 10.4: Build and smoke test**

```bash
cargo run
```
Open project, click + Add → Empty: object count goes to 1. + Add Directional: light count goes to 1.

- [ ] **Step 10.5: Commit**

```bash
git add -A
git commit -m "feat: hierarchy Add menu — empty object, lights"
```

---

## Task 11: Inspector dispatch shell

**Files:**
- Modify: `src/editor/panels/inspector.rs`
- Modify: `src/editor/inspector/mod.rs`

Empty dispatch shell that routes by `Selection` variant. Per-variant sections are filled in tasks 12-17.

- [ ] **Step 11.1: Inspector module skeleton**

`src/editor/inspector/mod.rs`:

```rust
pub mod transform;
pub mod mesh_material;
pub mod light;
pub mod camera;
pub mod material_editor;
pub mod resource_meta;
```

For each (`transform.rs`, `mesh_material.rs`, `light.rs`, `camera.rs`, `material_editor.rs`, `resource_meta.rs`), start with a stub:

```rust
use egui::Ui;
use enigma_3d::AppState;

pub fn draw(_ui: &mut Ui, _app_state: &mut AppState) {
    // populated in later tasks
}
```

Each `draw` may take additional args (e.g., the uuid of the object being inspected). Refine signatures per task.

- [ ] **Step 11.2: Inspector panel dispatches**

`src/editor/panels/inspector.rs`:

```rust
use egui::Ui;
use enigma_3d::AppState;

use crate::editor::state::{EditorRoot, Selection};
use crate::editor::inspector;

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let selection = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        root.editor.selection.clone()
    };

    ui.heading("Inspector");
    ui.separator();

    match selection {
        Selection::None => {
            ui.label("Select something in the hierarchy or resource browser.");
        }
        Selection::SceneObject(uuid) => {
            inspector::transform::draw_for_object(ui, app_state, uuid);
            inspector::mesh_material::draw(ui, app_state, uuid);
        }
        Selection::Light(uuid) => {
            inspector::transform::draw_for_light(ui, app_state, uuid);
            inspector::light::draw(ui, app_state, uuid);
        }
        Selection::Camera => {
            inspector::camera::draw(ui, app_state);
        }
        Selection::Material(uuid) => {
            inspector::material_editor::draw(ui, app_state, uuid);
        }
        Selection::Resource(uuid) => {
            inspector::resource_meta::draw(ui, app_state, uuid);
        }
    }
}
```

- [ ] **Step 11.3: Update per-section signatures**

Adjust stubs so they match the calls above (e.g., `transform::draw_for_object(ui, app_state, uuid)`).

- [ ] **Step 11.4: Build**

```bash
cargo check
```
Expected: clean. Inspector shows "Select something…" by default; selecting in hierarchy shows the section headers (empty for now).

- [ ] **Step 11.5: Commit**

```bash
git add -A
git commit -m "feat: inspector dispatch shell"
```

---

## Task 12: Inspector — Transform section

**Files:**
- Modify: `src/editor/inspector/transform.rs`

- [ ] **Step 12.1: Write the transform UI for objects and lights**

```rust
use egui::{Ui, DragValue};
use enigma_3d::AppState;
use uuid::Uuid;

pub fn draw_for_object(ui: &mut Ui, app_state: &mut AppState, uuid: Uuid) {
    let Some(obj) = app_state.objects.iter_mut().find(|o| o.get_unique_id() == uuid) else { return; };

    egui::CollapsingHeader::new("Transform").default_open(true).show(ui, |ui| {
        // enigma Object's transform API: assume `transform.position`, `transform.rotation`, `transform.scale`
        // Adjust to actual API after grep.
        vec3_edit(ui, "Position", &mut obj.transform.position);
        vec3_edit(ui, "Rotation", &mut obj.transform.rotation);
        vec3_edit(ui, "Scale",    &mut obj.transform.scale);
    });
}

pub fn draw_for_light(ui: &mut Ui, app_state: &mut AppState, uuid: Uuid) {
    let Some(light) = app_state.light.iter_mut().find(|l| l.uuid == uuid) else { return; };
    egui::CollapsingHeader::new("Transform").default_open(true).show(ui, |ui| {
        ui.label("Position");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(&mut light.position[0]).speed(0.05));
            ui.add(DragValue::new(&mut light.position[1]).speed(0.05));
            ui.add(DragValue::new(&mut light.position[2]).speed(0.05));
        });
    });
}

fn vec3_edit(ui: &mut Ui, label: &str, v: &mut [f32; 3]) {
    ui.label(label);
    ui.horizontal(|ui| {
        ui.add(DragValue::new(&mut v[0]).speed(0.05).prefix("x "));
        ui.add(DragValue::new(&mut v[1]).speed(0.05).prefix("y "));
        ui.add(DragValue::new(&mut v[2]).speed(0.05).prefix("z "));
    });
}
```

- [ ] **Step 12.2: Verify enigma-3d Object.transform shape**

```bash
grep -n "pub.*transform\|pub struct Transform\|pub position\|pub rotation\|pub scale" /Users/cg-jm/jm/enigma-3d/src/object.rs | head
```

Adjust struct field access to match the real API. If `transform.rotation` is a quaternion not Vec3, replace rotation edit with `nalgebra::UnitQuaternion` controls or just expose Euler XYZ for v1.

- [ ] **Step 12.3: Run and smoke test**

```bash
cargo run
```
Open a project, Add Empty object, click it in hierarchy, edit position values — values change.

- [ ] **Step 12.4: Commit**

```bash
git add -A
git commit -m "feat: inspector Transform section for objects and lights"
```

---

## Task 13: Inspector — Mesh & Material assignment for SceneObject

**Files:**
- Modify: `src/editor/inspector/mesh_material.rs`

- [ ] **Step 13.1: Write the mesh/material section**

```rust
use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::EditorRoot;

pub fn draw(ui: &mut Ui, app_state: &mut AppState, object_uuid: Uuid) {
    egui::CollapsingHeader::new("Mesh & Material").default_open(true).show(ui, |ui| {
        // Mesh ref: read-only display for v1; spawning chooses the mesh.
        ui.label("Mesh: (from spawn)");

        // Material picker: dropdown of project materials
        let materials: Vec<(Uuid, String)> = app_state
            .get_state_data_value::<EditorRoot>("editor")
            .and_then(|r| r.project.as_ref())
            .map(|p| p.materials.iter().map(|m| (m.uuid, m.name.clone())).collect())
            .unwrap_or_default();

        // Read object's current material assignment (which lives in enigma::Object — for v1 we
        // store the editor's material uuid in a parallel HashMap on EditorRoot since enigma's
        // Object refers to materials by index into AppState.materials, not uuid).
        // ... see note below.

        let mut current_label = "(none)".to_string();
        ui.horizontal(|ui| {
            ui.label("Material:");
            egui::ComboBox::from_label("")
                .selected_text(&current_label)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(false, "(none)").clicked() { /* clear */ }
                    for (uuid, name) in &materials {
                        if ui.selectable_label(false, name).clicked() {
                            // assignment handled in Task 17 once material realization is wired
                            let _ = (uuid, object_uuid);
                        }
                    }
                });
        });
    });
}
```

**Material assignment data location.** enigma's `Object` holds material by *index* into `AppState.materials`, not uuid. To bridge: add a `material_assignments: HashMap<Uuid /* object */, Uuid /* material */>` field to `EditorState` (NOT `ProjectState` — these are scene-level not project-level… actually material assignments ARE scene-level and survive scene save).

**Better:** persist material assignments inside the scene file by extending enigma's `ObjectSerializer`. But that requires enigma-3d changes.

**v1 compromise:** store assignments in a `HashMap<Uuid, Uuid>` field on `ProjectState` keyed by `(scene_uuid, object_uuid) -> material_uuid`. Add to ProjectState:

```rust
pub material_assignments: HashMap<(Uuid, Uuid), Uuid>,  // (scene, object) -> material
```

Update Task 2's struct definition to include this. Load/save naturally with the project file.

- [ ] **Step 13.2: (already in Task 2) confirm `material_assignments` map**

`ProjectState.material_assignments: HashMap<(Uuid, Uuid), Uuid>` is already defined in Task 2's struct. This step verifies it's there and adds an assignment-presence assertion to the roundtrip test if not already covered.

- [ ] **Step 13.3: Wire the dropdown to update assignments**

In `mesh_material.rs`, replace the placeholder with real read/write of `project.material_assignments`. On click, set/clear the entry; on read, compute the current label from the project.

- [ ] **Step 13.4: Build and smoke test**

```bash
cargo run
```
Material dropdown shows materials once Task 17 creates them.

- [ ] **Step 13.5: Commit**

```bash
git add -A
git commit -m "feat: inspector Mesh & Material section + material assignment map"
```

---

## Task 14: Inspector — Light properties

**Files:**
- Modify: `src/editor/inspector/light.rs`

- [ ] **Step 14.1: Write the light props section**

```rust
use egui::{Ui, DragValue};
use enigma_3d::AppState;
use uuid::Uuid;

pub fn draw(ui: &mut Ui, app_state: &mut AppState, uuid: Uuid) {
    let Some(light) = app_state.light.iter_mut().find(|l| l.uuid == uuid) else { return; };

    egui::CollapsingHeader::new("Light").default_open(true).show(ui, |ui| {
        ui.label("Color");
        let mut color = [light.color[0], light.color[1], light.color[2]];
        if ui.color_edit_button_rgb(&mut color).changed() {
            light.color = color;
        }

        ui.label("Intensity");
        ui.add(DragValue::new(&mut light.intensity).speed(0.05).clamp_range(0.0..=20.0));

        ui.label("Cast shadow");
        ui.checkbox(&mut light.cast_shadow, "");

        ui.label("Directional");
        let mut directional = light.direction.is_some();
        if ui.checkbox(&mut directional, "").changed() {
            light.direction = if directional { Some([0.0, -1.0, 0.0]) } else { None };
        }
        if let Some(dir) = &mut light.direction {
            ui.horizontal(|ui| {
                ui.add(DragValue::new(&mut dir[0]).speed(0.01));
                ui.add(DragValue::new(&mut dir[1]).speed(0.01));
                ui.add(DragValue::new(&mut dir[2]).speed(0.01));
            });
        }
    });
}
```

- [ ] **Step 14.2: Verify Light fields are public**

```bash
grep -A20 "pub struct Light " /Users/cg-jm/jm/enigma-3d/src/light.rs
```

If `color`, `intensity`, `cast_shadow`, `direction`, `position` aren't public, add small `pub fn set_*` and `pub fn *` accessors to enigma-3d, or make the fields `pub`.

- [ ] **Step 14.3: Build and smoke test**

```bash
cargo run
```
Add a light, click it, change color / intensity — viewport updates.

- [ ] **Step 14.4: Commit**

```bash
git add -A
git commit -m "feat: inspector Light props"
```

---

## Task 15: Inspector — Camera properties

**Files:**
- Modify: `src/editor/inspector/camera.rs`

- [ ] **Step 15.1: Write camera UI**

```rust
use egui::{Ui, DragValue};
use enigma_3d::AppState;

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let Some(cam) = app_state.camera.as_mut() else {
        ui.label("(no camera in scene)");
        return;
    };

    egui::CollapsingHeader::new("Transform").default_open(true).show(ui, |ui| {
        ui.label("Position");
        ui.horizontal(|ui| {
            let mut p = cam.position();
            if ui.add(DragValue::new(&mut p[0]).speed(0.05)).changed() { cam.set_position(p); }
            if ui.add(DragValue::new(&mut p[1]).speed(0.05)).changed() { cam.set_position(p); }
            if ui.add(DragValue::new(&mut p[2]).speed(0.05)).changed() { cam.set_position(p); }
        });
    });

    egui::CollapsingHeader::new("Camera").default_open(true).show(ui, |ui| {
        let mut fov = cam.fov();
        if ui.add(DragValue::new(&mut fov).speed(0.5).prefix("fov ")).changed() {
            cam.set_fov(fov);
        }
        // near/far similarly
    });
}
```

- [ ] **Step 15.2: Verify camera API**

```bash
grep -n "pub fn " /Users/cg-jm/jm/enigma-3d/src/camera.rs | head
```

Adjust to actual setters/getters.

- [ ] **Step 15.3: Build and smoke test, commit**

```bash
cargo run
git add -A
git commit -m "feat: inspector Camera props"
```

---

## Task 16: Inspector — Resource meta (texture/model/shader/audio)

**Files:**
- Modify: `src/editor/inspector/resource_meta.rs`

- [ ] **Step 16.1: Resource metadata display**

```rust
use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::{EditorRoot, ResourceKind};

pub fn draw(ui: &mut Ui, app_state: &mut AppState, uuid: Uuid) {
    let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
    let Some(project) = &root.project else { return; };
    let Some(entry) = project.manifest.iter().find(|e| e.uuid == uuid) else { return; };

    ui.label(format!("Name: {}", entry.name));
    ui.label(format!("Kind: {:?}", entry.kind));
    ui.label(format!("Path: {}", entry.relative_path));

    match entry.kind {
        ResourceKind::Texture => {
            // Thumbnail rendering — defer to later polish task; show file size for now
            let path = std::path::Path::new(&project.root_path)
                .join("src/resources")
                .join(&entry.relative_path);
            if let Ok(meta) = std::fs::metadata(&path) {
                ui.label(format!("Size: {} bytes", meta.len()));
            }
        }
        _ => {}
    }
}
```

Texture thumbnail rendering via `egui::ColorImage` + `TextureHandle` is straightforward but adds I/O on every frame if naive. Defer to polish.

- [ ] **Step 16.2: Build, smoke test, commit**

```bash
cargo run
git add -A
git commit -m "feat: inspector resource metadata display"
```

---

## Task 17: Inspector — Material editor (the heart)

**Files:**
- Modify: `src/editor/inspector/material_editor.rs`
- Modify: `src/project/material.rs` (add `reconcile`)
- Modify: `src/editor/mod.rs` (call reconcile each frame)

- [ ] **Step 17.1: Write reconcile**

In `src/project/material.rs`, add:

```rust
use std::collections::HashMap;

pub fn reconcile(
    project: &ProjectState,
    app_state: &mut AppState,
    display: Display<WindowSurface>,
    cache: &mut HashMap<Uuid, u64>,    // material uuid -> hash, lives in EditorState
) -> Result<(), RealizeError> {
    // Build a list of materials currently realized: enigma::Material has `name` we can use as a key.
    // For v1, key by name; better: extend enigma to keep a uuid->material map. Simplest:
    // rebuild all materials whose hash changed.

    for def in &project.materials {
        let new_hash = material_hash(def);
        let stale = cache.get(&def.uuid).copied() != Some(new_hash);
        if stale {
            let mat = realize(def, project, display.clone())?;
            // Replace or append in app_state.materials. Find by name (set in realize).
            if let Some(pos) = app_state.materials.iter().position(|m| m.get_name() == def.name) {
                app_state.materials[pos] = mat;
            } else {
                app_state.materials.push(mat);
            }
            cache.insert(def.uuid, new_hash);
        }
    }
    Ok(())
}
```

Add `material_cache: HashMap<Uuid, u64>` to `EditorState`. Update `Default for EditorState`.

- [ ] **Step 17.2: Call reconcile each frame**

In `src/editor/mod.rs::draw`, before any panel draws, after the style is set:

```rust
// Reconcile materials before rendering panels — keeps the viewport in sync with edits.
if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
    if let Some(p) = root.project.clone() {
        let mut cache = std::mem::take(&mut root.editor.material_cache);
        // need a Display reference — fetch from app_state if exposed; else skip reconcile here
        // and trigger it on explicit material edits only (acceptable for v1).
        root.editor.material_cache = cache;
    }
}
```

**Display access.** enigma's `AppState` may or may not expose its `Display`. Check:

```bash
grep -n "pub.*display\|fn display" /Users/cg-jm/jm/enigma-3d/src/lib.rs
```

If `display` isn't accessible at gui injection time, run reconcile on explicit edits only (call from inside material editor when a slider changes). Acceptable.

- [ ] **Step 17.3: Write the material editor UI**

```rust
use egui::{Ui, DragValue};
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::{EditorRoot, ResourceKind, ShaderChoice};

pub fn draw(ui: &mut Ui, app_state: &mut AppState, material_uuid: Uuid) {
    // Snapshot textures list for the picker
    let textures: Vec<(Uuid, String)> = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| p.manifest.iter()
            .filter(|e| e.kind == ResourceKind::Texture)
            .map(|e| (e.uuid, e.name.clone()))
            .collect())
        .unwrap_or_default();

    let mut def_clone = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = &root.project else { return; };
        let Some(d) = project.materials.iter().find(|m| m.uuid == material_uuid) else { return; };
        d.clone()
    };

    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Name");
        changed |= ui.text_edit_singleline(&mut def_clone.name).changed();
    });

    ui.horizontal(|ui| {
        ui.label("Shader");
        egui::ComboBox::from_label("##shader")
            .selected_text(format!("{:?}", def_clone.shader))
            .show_ui(ui, |ui| {
                if ui.selectable_label(matches!(def_clone.shader, ShaderChoice::PbrLit), "PBR Lit").clicked() {
                    def_clone.shader = ShaderChoice::PbrLit;
                    changed = true;
                }
                if ui.selectable_label(matches!(def_clone.shader, ShaderChoice::Unlit), "Unlit").clicked() {
                    def_clone.shader = ShaderChoice::Unlit;
                    changed = true;
                }
            });
    });

    ui.separator();
    ui.label("Texture slots");

    changed |= texture_slot(ui, "Albedo",    &mut def_clone.albedo,    &textures);
    if matches!(def_clone.shader, ShaderChoice::PbrLit) {
        changed |= texture_slot(ui, "Normal",    &mut def_clone.normal,    &textures);
        changed |= texture_slot(ui, "Roughness", &mut def_clone.roughness, &textures);
        changed |= texture_slot(ui, "Metallic",  &mut def_clone.metallic,  &textures);
    }
    changed |= texture_slot(ui, "Emissive",  &mut def_clone.emissive,  &textures);

    ui.separator();
    ui.label("Parameters");
    changed |= ui.color_edit_button_rgb(&mut def_clone.color).changed();
    changed |= ui.add(DragValue::new(&mut def_clone.emissive_strength).speed(0.05).prefix("emissive ")).changed();
    changed |= ui.add(DragValue::new(&mut def_clone.roughness_strength).speed(0.01).clamp_range(0.0..=1.0).prefix("roughness ")).changed();
    changed |= ui.add(DragValue::new(&mut def_clone.metallic_strength).speed(0.01).clamp_range(0.0..=1.0).prefix("metallic ")).changed();
    changed |= ui.add(DragValue::new(&mut def_clone.normal_strength).speed(0.01).clamp_range(0.0..=2.0).prefix("normal ")).changed();
    changed |= ui.checkbox(&mut def_clone.transparent, "Transparent").changed();
    if def_clone.transparent {
        changed |= ui.add(DragValue::new(&mut def_clone.transparency_strength).speed(0.01).clamp_range(0.0..=1.0)).changed();
    }

    if changed {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = root.project.as_mut() {
                if let Some(d) = project.materials.iter_mut().find(|m| m.uuid == material_uuid) {
                    *d = def_clone;
                }
                root.editor.dirty = true;
            }
        }
        // Reconcile materials on next frame (handled in editor::draw entry point — or trigger here
        // explicitly if Display is reachable).
    }
}

fn texture_slot(
    ui: &mut Ui,
    label: &str,
    slot: &mut Option<Uuid>,
    textures: &[(Uuid, String)],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let current = slot.and_then(|u| textures.iter().find(|(uu, _)| *uu == u).map(|(_, n)| n.clone()))
            .unwrap_or_else(|| "(none)".into());
        egui::ComboBox::from_label(format!("##slot-{label}"))
            .selected_text(current)
            .show_ui(ui, |ui| {
                if ui.selectable_label(slot.is_none(), "(none)").clicked() {
                    *slot = None;
                    changed = true;
                }
                for (uuid, name) in textures {
                    let is = *slot == Some(*uuid);
                    if ui.selectable_label(is, name).clicked() {
                        *slot = Some(*uuid);
                        changed = true;
                    }
                }
            });
    });
    changed
}
```

- [ ] **Step 17.4: Build and manual test**

```bash
cargo run
```
Resource browser → Materials tab → "+ New" (which lands in Task 19). For now, materials can be created via a test path: temporarily wire a "Create test material" button in toolbar that adds a default-pbr material. Verify the inspector renders all the slots and parameter sliders.

- [ ] **Step 17.5: Commit**

```bash
git add -A
git commit -m "feat: material editor — slots, params, live edit"
```

---

## Task 18: Resource browser shell + tab strip

**Files:**
- Modify: `src/editor/panels/resource_browser.rs`

- [ ] **Step 18.1: Resource browser draw**

```rust
use egui::Ui;
use enigma_3d::AppState;

use crate::editor::state::{EditorRoot, ResourceKind, ResourceTab, Selection};

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
    let project_loaded = root.project.is_some();
    let current_tab = root.editor.resource_browser_tab;

    if !project_loaded {
        ui.label("No project loaded");
        return;
    }

    let mut new_tab: Option<ResourceTab> = None;
    ui.horizontal(|ui| {
        for tab in [
            ResourceTab::Models, ResourceTab::Textures, ResourceTab::Shaders,
            ResourceTab::Materials, ResourceTab::Scenes, ResourceTab::Audio, ResourceTab::Other,
        ] {
            if ui.selectable_label(current_tab == tab, format!("{tab:?}")).clicked() {
                new_tab = Some(tab);
            }
        }
    });
    ui.separator();

    if let Some(t) = new_tab {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.resource_browser_tab = t;
        }
    }

    match current_tab {
        ResourceTab::Models => list_kind(ui, app_state, ResourceKind::Model),
        ResourceTab::Textures => list_kind(ui, app_state, ResourceKind::Texture),
        ResourceTab::Shaders => list_kind(ui, app_state, ResourceKind::Shader),
        ResourceTab::Audio => list_kind(ui, app_state, ResourceKind::Audio),
        ResourceTab::Other => list_kind(ui, app_state, ResourceKind::Other),
        ResourceTab::Materials => list_materials(ui, app_state),
        ResourceTab::Scenes => list_scenes(ui, app_state),
    }
}

fn list_kind(ui: &mut Ui, app_state: &mut AppState, kind: ResourceKind) {
    let rows: Vec<(uuid::Uuid, String)> = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| p.manifest.iter()
            .filter(|e| e.kind == kind)
            .map(|e| (e.uuid, e.name.clone()))
            .collect())
        .unwrap_or_default();

    let current_sel = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.selection.clone())
        .unwrap_or(Selection::None);

    let mut new_sel: Option<Selection> = None;
    if ui.button("+ Import").clicked() {
        // import flow lands in Task 19
    }
    for (uuid, name) in &rows {
        let selected = matches!(&current_sel, Selection::Resource(u) if u == uuid);
        if ui.selectable_label(selected, name).clicked() {
            new_sel = Some(Selection::Resource(*uuid));
        }
    }
    if let Some(s) = new_sel {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.selection = s;
        }
    }
}

fn list_materials(ui: &mut Ui, app_state: &mut AppState) {
    let rows: Vec<(uuid::Uuid, String)> = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| p.materials.iter().map(|m| (m.uuid, m.name.clone())).collect())
        .unwrap_or_default();

    let mut create = false;
    if ui.button("+ New").clicked() { create = true; }

    let current_sel = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.selection.clone()).unwrap_or(Selection::None);
    let mut new_sel: Option<Selection> = None;
    for (uuid, name) in &rows {
        let selected = matches!(&current_sel, Selection::Material(u) if u == uuid);
        if ui.selectable_label(selected, name).clicked() {
            new_sel = Some(Selection::Material(*uuid));
        }
    }

    if create {
        let new_uuid = if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = root.project.as_mut() {
                let name = format!("Material {}", project.materials.len() + 1);
                let mat = crate::editor::state::MaterialDef::default_pbr(name);
                let uuid = mat.uuid;
                project.materials.push(mat);
                root.editor.selection = Selection::Material(uuid);
                root.editor.dirty = true;
                Some(uuid)
            } else { None }
        } else { None };
        let _ = new_uuid;
    } else if let Some(s) = new_sel {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.selection = s;
        }
    }
}

fn list_scenes(ui: &mut Ui, app_state: &mut AppState) {
    // listed in Task 20
    ui.label("Scenes (lands in Task 20)");
}
```

- [ ] **Step 18.2: Build, smoke test, commit**

```bash
cargo run
```
Tabs switch; Materials "+ New" creates a material; click selects it; inspector shows material editor.

```bash
git add -A
git commit -m "feat: resource browser shell with tabs and materials list"
```

---

## Task 19: Resource browser — import flow

**Files:**
- Modify: `src/editor/panels/resource_browser.rs`
- Modify: `src/editor/panels/hierarchy.rs` (From Model spawn)

- [ ] **Step 19.1: Wire import per kind**

In `list_kind`, replace `// import flow lands in Task 19` with:

```rust
if ui.button("+ Import").clicked() {
    let filter = filter_for(kind);
    if let Some(src) = pick_file_with_filter(&filter) {
        let result = if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = root.project.as_mut() {
                Some(crate::project::resource::import(std::path::Path::new(&src), kind, project))
            } else { None }
        } else { None };
        if let Some(Err(e)) = result {
            eprintln!("import failed: {e:?}");
        }
    }
}

fn filter_for(kind: ResourceKind) -> Vec<&'static str> {
    match kind {
        ResourceKind::Model => vec!["gltf", "glb"],
        ResourceKind::Texture => vec!["png", "jpg", "jpeg", "tga", "dds", "bmp"],
        ResourceKind::Shader => vec!["glsl", "vert", "frag", "vs", "fs"],
        ResourceKind::Audio => vec!["wav", "ogg", "mp3", "flac"],
        ResourceKind::Other => vec![],
    }
}

fn pick_file_with_filter(exts: &[&str]) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut dialog = rfd::AsyncFileDialog::new();
    if !exts.is_empty() {
        dialog = dialog.add_filter("file", exts);
    }
    async_std::task::spawn(async move {
        if let Some(f) = dialog.pick_file().await {
            let _ = tx.send(f.path().to_string_lossy().into_owned());
        }
    });
    rx.recv().ok()
}
```

- [ ] **Step 19.2: Add "From Model" to hierarchy Add menu**

In `hierarchy.rs` inside the Objects "+ Add" menu, append:

```rust
ui.menu_button("From Model…", |ui| {
    let models: Vec<(Uuid, String)> = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| p.manifest.iter()
            .filter(|e| e.kind == ResourceKind::Model)
            .map(|e| (e.uuid, e.name.clone())).collect())
        .unwrap_or_default();
    for (uuid, name) in models {
        if ui.button(&name).clicked() {
            spawn_from_model(app_state, uuid);
            ui.close_menu();
        }
    }
});
```

And `spawn_from_model`:

```rust
fn spawn_from_model(app_state: &mut AppState, model_uuid: Uuid) {
    let bytes = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = &root.project else { return; };
        match crate::project::resource::bytes(project, model_uuid) {
            Ok(b) => b,
            Err(_) => return,
        }
    };
    // Use enigma's GLTF loader from bytes
    let obj = enigma_3d::object::Object::load_from_gltf_resource(&bytes, /* display */);
    app_state.add_object(obj);
}
```

Check enigma-3d's GLTF loader API:

```bash
grep -n "from_gltf\|load_from\|Object::from" /Users/cg-jm/jm/enigma-3d/src/object.rs | head
```

Use whichever loader takes bytes (or a path — if path-only, write bytes to a temp file). Adjust accordingly.

- [ ] **Step 19.3: Build, smoke test, commit**

```bash
cargo run
```
Import a model → appears in Models tab. "+ Add → From Model" lists it → spawn.

```bash
git add -A
git commit -m "feat: resource import flow + spawn from model"
```

---

## Task 20: Scenes tab + Scene menu in toolbar

**Files:**
- Modify: `src/editor/panels/resource_browser.rs` (list_scenes)
- Modify: `src/editor/panels/toolbar.rs` (Scene menu)

- [ ] **Step 20.1: Scenes tab list**

Replace `list_scenes` in `resource_browser.rs`:

```rust
fn list_scenes(ui: &mut Ui, app_state: &mut AppState) {
    let scenes: Vec<(usize, String, bool)> = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| p.scenes.iter().enumerate()
            .map(|(i, s)| (i, s.name.clone(), i == p.startup_scene_index))
            .collect())
        .unwrap_or_default();

    let active_index = app_state.get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| p.active_scene_index)
        .unwrap_or(0);

    let mut switch_to: Option<usize> = None;
    let mut new_clicked = false;

    if ui.button("+ New Scene").clicked() { new_clicked = true; }
    for (idx, name, is_startup) in &scenes {
        let mut label = name.clone();
        if *is_startup { label.push_str(" (startup)"); }
        if *idx == active_index { label.push_str(" — active"); }
        if ui.selectable_label(false, &label).double_clicked() {
            switch_to = Some(*idx);
        }
    }

    if new_clicked {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.modal = Some(crate::editor::state::Modal::NewSceneName(String::new()));
        }
    }
    // switch is invoked from toolbar / modal too — wire via actions
    if let Some(idx) = switch_to {
        // need display ref; defer to action that has access
        let _ = idx;  // TODO actual switch — modal-driven from Task 22
    }
}
```

Scene switch requires the `Display` reference (from enigma). For v1, gate switch behind an explicit toolbar action that runs in the next frame (since egui callbacks don't have direct access to the glium Display).

**Resolution:** stash a pending action in `EditorState` (`pub pending_action: Option<PendingAction>`) and process it in `editor::draw` after panel draws — at the point where you do have `&mut AppState` and likely `Display` reachable through one of enigma's APIs. Check:

```bash
grep -n "fn.*display\|fn render\|.display" /Users/cg-jm/jm/enigma-3d/src/lib.rs | head
```

If `Display` is captured at `EventLoop::run` time and not exposed, enigma-3d needs a small addition: pass `Display` into the GUI callback. The current `inject_gui(Arc<dyn Fn(&Context, &mut AppState)>)` has no Display. Add `inject_gui_with_display(Arc<dyn Fn(&Context, &mut AppState, &Display)>)` — minor enigma extension.

For v1, if extending enigma is in scope, do it. Otherwise: store pending actions and resolve them at the start of the next frame from a static-held Display (acquired once at startup via app_state).

**Decision for plan-writing:** schedule a small enigma-3d change as Task 20.0 — add `inject_gui_with_display` (or just expose `AppState::display() -> &Display`).

- [ ] **Step 20.0: Extend enigma-3d to expose Display**

In a separate commit in the enigma-3d repo (this plan documents the requirement; actual change happens in that repo):

```rust
// in src/lib.rs AppState
pub fn display(&self) -> &Display<WindowSurface> { &self.display }
```

Update enigma-engine's Cargo.toml if a SHA pin is needed. (Or commit to the enigma-3d main branch; the editor's git dep tracks main.)

- [ ] **Step 20.1: (now) Wire Scene menu in toolbar**

In `toolbar.rs`, add after File menu:

```rust
ui.menu_button("Scene", |ui| {
    let project_clone = current_project_clone(app_state);
    if let Some(p) = &project_clone {
        if ui.button("New Scene…").clicked() {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                root.editor.modal = Some(Modal::NewSceneName(String::new()));
            }
            ui.close_menu();
        }
        ui.menu_button("Switch Scene", |ui| {
            for (idx, s) in p.scenes.iter().enumerate() {
                if ui.button(&s.name).clicked() {
                    let display = app_state.display().clone();
                    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                        if let Some(proj) = root.project.as_mut() {
                            let _ = crate::project::scene::switch(proj, app_state, &display, idx);
                        }
                    }
                    ui.close_menu();
                }
            }
        });
        if ui.button("Set Current As Startup").clicked() {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(proj) = root.project.as_mut() {
                    proj.startup_scene_index = proj.active_scene_index;
                }
            }
            ui.close_menu();
        }
    }
});
```

Note: the borrow checker may complain about `app_state.display().clone()` and `app_state.get_state_data_value_mut(...)` in the same scope. Resolution: snapshot the display first into a local variable before grabbing the mutable borrow:

```rust
let display = app_state.display().clone();
if let Some(root) = ...
```

- [ ] **Step 20.2: Process NewSceneName modal**

In `editor::draw`, after panel draws, add modal handler:

```rust
fn process_modals(ctx: &Context, app_state: &mut AppState) {
    let modal = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        root.editor.modal.clone()
    };
    let Some(modal) = modal else { return; };

    let mut close = false;
    egui::Window::new("Dialog").collapsible(false).show(ctx, |ui| {
        match modal {
            Modal::NewSceneName(mut draft) => {
                ui.label("Scene name:");
                ui.text_edit_singleline(&mut draft);
                // Persist draft back to state
                if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                    root.editor.modal = Some(Modal::NewSceneName(draft.clone()));
                }
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() && !draft.is_empty() {
                        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                            if let Some(proj) = root.project.as_mut() {
                                let _ = crate::project::scene::new_scene(proj, draft.clone());
                            }
                        }
                        close = true;
                    }
                    if ui.button("Cancel").clicked() { close = true; }
                });
            }
            Modal::ConfirmDelete { label, pending } => {
                ui.label(format!("Delete {label}?"));
                ui.horizontal(|ui| {
                    if ui.button("Delete").clicked() {
                        // dispatch pending delete — handled per-variant
                        apply_pending_delete(app_state, pending);
                        close = true;
                    }
                    if ui.button("Cancel").clicked() { close = true; }
                });
            }
            Modal::ImportError(msg) => {
                ui.label(msg);
                if ui.button("OK").clicked() { close = true; }
            }
        }
    });
    if close {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.modal = None;
        }
    }
}

fn apply_pending_delete(app_state: &mut AppState, p: PendingDelete) {
    let display = app_state.display().clone();
    let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") else { return; };
    let Some(project) = root.project.as_mut() else { return; };
    match p {
        PendingDelete::Resource(uuid) => { let _ = crate::project::resource::delete(project, uuid); }
        PendingDelete::Material(uuid) => { project.materials.retain(|m| m.uuid != uuid); }
        PendingDelete::Scene(idx) => { let _ = crate::project::scene::delete(project, idx); }
        PendingDelete::SceneObject(uuid) => { app_state.objects.retain(|o| o.get_unique_id() != uuid); }
        PendingDelete::Light(uuid) => { app_state.light.retain(|l| l.uuid != uuid); }
    }
}
```

- [ ] **Step 20.3: Build, smoke test, commit**

```bash
cargo run
```
File → New Project; Scene → New Scene… → enter "level2" → Create. Scene menu → Switch Scene → level2 → hierarchy empties.

```bash
git add -A
git commit -m "feat: scene switching, new scene modal, pending delete dispatch"
```

---

## Task 21: Viewport ray-pick selection

**Files:**
- Modify: `src/editor/panels/viewport.rs`

- [ ] **Step 21.1: Implement ray-pick on viewport click**

```rust
use egui::{Ui, Pos2, Rect};
use enigma_3d::AppState;
use enigma_3d::collision_world::Ray;
use cgmath::{InnerSpace, Vector3};

use crate::editor::state::{EditorRoot, Selection};

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let rect = ui.max_rect();
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.viewport_rect = Some(rect);
    }

    let ctx = ui.ctx();
    let primary_released = ctx.input(|i| i.pointer.primary_released());
    if !primary_released { return; }
    let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) else { return; };
    if !rect.contains(pos) { return; }

    let drag_active = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.drag.is_some()).unwrap_or(false);
    if drag_active { return; }

    let ndc = screen_to_ndc(pos, rect);
    let Some(camera) = app_state.camera.as_ref() else { return; };
    let (origin, dir) = unproject(camera, ndc);

    let mut ray = Ray::new(origin, dir, camera.far_plane());
    ray.cast(app_state);

    let new_selection = ray.get_intersection_uuids().first().copied()
        .map(Selection::SceneObject)
        .unwrap_or(Selection::None);

    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.selection = new_selection;
    }
}

fn screen_to_ndc(pos: Pos2, rect: Rect) -> (f32, f32) {
    let x = (pos.x - rect.min.x) / rect.width() * 2.0 - 1.0;
    let y = -((pos.y - rect.min.y) / rect.height() * 2.0 - 1.0);
    (x, y)
}

fn unproject(camera: &enigma_3d::camera::Camera, ndc: (f32, f32)) -> (Vector3<f32>, Vector3<f32>) {
    // Use camera's view/projection inverse. enigma may or may not expose this directly.
    // For v1, approximate via forward + right + up vectors and FOV.
    let fov = camera.fov().to_radians();
    let aspect = camera.aspect_ratio();
    let half_h = (fov / 2.0).tan();
    let half_w = half_h * aspect;
    let forward: Vector3<f32> = camera.forward().into();
    let right:   Vector3<f32> = camera.right().into();
    let up:      Vector3<f32> = camera.up().into();
    let dir = (forward + right * (ndc.0 * half_w) + up * (ndc.1 * half_h)).normalize();
    let origin: Vector3<f32> = camera.position().into();
    (origin, dir)
}
```

- [ ] **Step 21.2: Verify camera helpers (forward / right / up / position)**

```bash
grep -n "pub fn forward\|pub fn right\|pub fn up\|pub fn position\|pub fn fov\|pub fn aspect" /Users/cg-jm/jm/enigma-3d/src/camera.rs
```

If absent, add minimal helpers to enigma-3d.

- [ ] **Step 21.3: Build, smoke test, commit**

```bash
cargo run
```
Add a primitive object via "From Model" → click on the rendered geometry in viewport → hierarchy selection updates.

```bash
git add -A
git commit -m "feat: viewport ray-pick selection"
```

---

## Task 22: Drag-drop polish — texture → material slot

**Files:**
- Modify: `src/editor/panels/resource_browser.rs` (drag source)
- Modify: `src/editor/inspector/material_editor.rs` (drop target)

This is polish; the dropdown already works. Adding drag-drop is nice-to-have. For v1 we can skip if time pressure mounts.

- [ ] **Step 22.1: Drag source in resource browser**

In `list_kind`, wrap each row in egui's drag-and-drop:

```rust
for (uuid, name) in &rows {
    let resp = ui.add(egui::Label::new(name).sense(egui::Sense::click_and_drag()));
    if resp.clicked() {
        new_sel = Some(Selection::Resource(*uuid));
    }
    if resp.dragged() {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            root.editor.drag = Some(DragPayload::Resource(*uuid));
        }
    }
}
```

Reset `drag` to `None` on `primary_released`.

- [ ] **Step 22.2: Drop target on material slots**

In `texture_slot` (material_editor.rs), wrap the row in a drop zone:

```rust
let response = ui.allocate_response(egui::vec2(120.0, 24.0), egui::Sense::hover());
let dropped = ctx.input(|i| i.pointer.primary_released() && response.hovered());
if dropped {
    if let Some(DragPayload::Resource(tex_uuid)) = drag_payload {
        *slot = Some(tex_uuid);
        changed = true;
    }
}
```

This is finicky in egui 0.23 (no built-in DnD API). Skip if it doesn't land cleanly — dropdown is sufficient for v1.

- [ ] **Step 22.3: Commit (or skip if not landing)**

```bash
git add -A
git commit -m "feat: drag-drop texture to material slot (optional polish)"
```

---

## Task 23: Manual smoke test pass against acceptance criteria

**Files:** none modified — verification step.

This task runs the manual smoke test from the spec (§Manual smoke test script) to confirm the sub-project meets acceptance.

- [ ] **Step 23.1: Run the script**

1. `cargo run` — editor opens.
2. File → New Project → pick empty dir → editor shows empty hierarchy, scene "main", project shown in status.
3. Resource browser → Models tab → "+ Import" → pick a .gltf → appears in list.
4. Resource browser → Textures tab → "+ Import" → pick a .png → appears in list.
5. Hierarchy → Objects → "+ Add → From Model → \<model>" → object appears in hierarchy and viewport.
6. Resource browser → Materials tab → "+ New" → "Material 1" appears, inspector shows it. Set Albedo to imported texture.
7. Hierarchy → click object → Inspector → Material dropdown → "Material 1" → viewport shows textured object.
8. Hierarchy → Lights → "+ Add → Directional" → light appears; inspector → set color and intensity.
9. File → Save Project. File → Save Scene. Close editor.
10. `cargo run` again → File → Open Project → pick the project file → scene loads with object, light, material intact.
11. File → Play → child cargo run launches a window showing the scene.
12. Scene → New Scene → "level2" → Scene → Switch Scene → "level2" → hierarchy empty. Switch back → scene returns.

- [ ] **Step 23.2: Run unit tests**

```bash
cargo test --bin enigma-engine
```
Expected: all tests pass.

- [ ] **Step 23.3: Run cargo check / build**

```bash
cargo check
cargo build --release
```
Expected: clean (no new warnings introduced).

- [ ] **Step 23.4: Commit any final fixes**

```bash
git add -A
git commit -m "polish: smoke test fixes"
```

- [ ] **Step 23.5: Tag the sub-project completion**

```bash
git tag scene-authoring-core-v1
```

---

## Plan self-review

**1. Spec coverage:**
- §Architectural commitments → Task 1 (modules), 3 (state separation), 7 (docked), 21 (viewport rect, ray-pick)
- §Module layout → Task 1
- §Data model → Task 2, Task 13 (assignments addition)
- §Project lifecycle → Task 3
- §Resource pipeline → Task 4, Task 19 (import UI)
- §Scene management → Task 5, Task 20 (UI)
- §Selection model → Task 9 (hierarchy click), Task 18 (browser click), Task 21 (ray-pick)
- §UI panels → Tasks 7-11, 18
- §Material editor → Task 17
- §Light authoring → Task 14
- §Error handling → Modal-based; Task 20 (modal infrastructure); explicit ImportError paths in Task 4/Task 19
- §Testing strategy → Unit tests embedded in Tasks 2, 4, 5, 6. Manual smoke test = Task 23.

**Gaps identified during review:**
- "Reveal in Finder" right-click action — out of v1 scope; not in plan, acceptable.
- Inline rename (double-click to edit) — out of plan; rename via type-in-place not implemented. Acceptable for v1; the spec mentions inline rename but it's polish, not core.
- Texture thumbnails in resource browser/inspector — deferred to polish, not in any task; acceptable.
- Hot-reload of assets — explicitly out of scope.
- Right-click context menus — not in plan; manual delete via Inspector or pending modal. Acceptable for v1.
- The `material_assignments: HashMap<(Uuid, Uuid), Uuid>` change to ProjectState is introduced in Task 13 mid-flight. Should be in Task 2 as part of the initial data model definition.

**Fix:** Note that Task 2's struct definition should include `material_assignments: HashMap<(Uuid, Uuid), Uuid>` from the start. Adding this inline now:

In `src/editor/state.rs` (Task 2), `ProjectState` gains:

```rust
pub material_assignments: HashMap<(Uuid, Uuid), Uuid>,  // (scene_uuid, object_uuid) -> material_uuid
```

And `ProjectState::new` initializes it as `HashMap::new()`. Update the roundtrip test to include an assignment.

**2. Placeholder scan:** Several places defer to enigma-3d API verification (e.g., field visibility checks in Tasks 7, 9, 12, 14, 15, 17, 19, 20.0, 21). These are deliberate — the spec already noted "Plan-writing will resolve which" for ambiguous enigma APIs. The plan steps tell the engineer exactly how to verify (grep commands provided). Not a placeholder failure.

The `todo!("cube")` in Task 10 is genuine — but Task 10.2 explicitly removes those variants from v1. The `todo!()` lines are deleted before the task ends. OK.

**3. Type consistency:** ProjectState/EditorState/etc. used across tasks match Task 2 definitions. After the Task 13 fix above, `material_assignments` is consistent.

**4. Ambiguity:** "From Model" submenu shows models — model names may collide; UI shows first. Acceptable. Drag-drop in Task 22 is explicitly tagged as optional polish.

---

## Execution choices

Plan complete and saved to `docs/superpowers/plans/2026-05-25-scene-authoring-core.md`. Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
