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
