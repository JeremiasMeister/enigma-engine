# Scene Authoring Core — Design

**Date:** 2026-05-25
**Status:** Spec — awaiting plan
**Project:** enigma-engine (editor for enigma-3d)
**Sub-project:** 1 of 5 (Scene Authoring Core → Viewport Gizmos → Environment → Animation → Build Pipeline)
**Out of scope:** particle authoring (its own sub-project), gizmos, skybox/post-processing, animation timeline, resource embedding/packaging.

## Goal

Land the editor's first usable form: a docked-panel UI on top of an enigma-3d render surface that lets the user import resources, compose a scene with multiple meshes / lights / a camera, assign materials with textures, save/load multi-scene projects, and press Play to launch the project as a child `cargo run`. After this sub-project, the editor is capable of producing a scene that looks like a shippable game scene; later sub-projects add gizmos, environment editing, animation, and packaging.

## Architectural commitments (decided during brainstorming)

1. **Editor is itself an enigma-3d app.** Rendering surface = enigma's framebuffer; UI = egui drawn over it.
2. **Resource storage is hybrid.** Files on disk under `src/resources/<kind>/` are canonical. `ProjectState` holds a manifest with uuids and relative paths. Build-time `include_bytes!` codegen lands in the later Build Pipeline sub-project; identities (uuids) are committed now so we don't have to migrate.
3. **Multi-scene from day one.** `ProjectState.scenes: Vec<SceneRef>` plus an active index. The runtime project's `main.rs` launches into a designated `startup_scene_index`.
4. **Docked layout** (Unity/Godot-style): top toolbar, left hierarchy, right inspector, bottom resource browser, central viewport (transparent egui CentralPanel over enigma's framebuffer — no FBO required).
5. **Editor module structure with explicit state separation:** `ProjectState` (serializable) vs `EditorState` (transient).
6. **Play still shells out to `cargo run`** as today. Play-in-editor (dylib hot-reload) is not in scope.

---

## Module layout

```
src/
  main.rs                       # AppState setup, inject editor draw fn, run
  editor/
    mod.rs                      # top-level draw(context, app_state); init editor state
    state.rs                    # ProjectState + EditorState definitions
    actions.rs                  # mutator primitives panels invoke (pure where possible)
    panels/
      mod.rs
      toolbar.rs                # top dock: File / Scene menus, Play / Build buttons
      hierarchy.rs              # left dock: scene tree + Add menu
      inspector.rs              # right dock: dispatches by selection kind
      resource_browser.rs       # bottom dock: tabs over resource kinds
      viewport.rs               # central panel: captures viewport rect, handles ray-pick clicks
    inspector/
      mod.rs
      transform.rs              # Transform section (shared by SceneObject/Light/Camera)
      mesh_material.rs          # Mesh + Material assignment for SceneObject
      material_editor.rs        # Full material editor when Material is selected
      light.rs                  # Light props section
      camera.rs                 # Camera props section
      resource_meta.rs          # Resource preview/metadata sections
  project/
    mod.rs                      # new/open/save project (refactored from current project.rs)
    resource.rs                 # import, uuid generation, manifest lookup, bytes loader
    scene.rs                    # multi-scene management: switch / load / save active scene
    material.rs                 # MaterialDef <-> enigma::Material realization & reconciliation
  resources/                    # unchanged: icon + scaffold templates for child projects
```

Existing files reshaped:
- `src/main.rs` — replaces the `Engine` struct definition with a thinner shell that holds `EditorRoot { project: Option<ProjectState>, editor: EditorState }`. The `state_data` key changes from `"engine"` to `"editor"`.
- `src/project.rs` (current) — split into `project/mod.rs` (new/open/save), `project/resource.rs`, `project/scene.rs`, `project/material.rs`.
- `src/ui/project_window.rs` — split into `editor/panels/toolbar.rs` (the menu actions) plus the existing scaffolding code redistributed.
- `src/ui/resource_window.rs` — replaced by `editor/panels/resource_browser.rs`.
- `src/serialization.rs` — currently empty; delete.

---

## Data model

```rust
// editor/state.rs

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub name: String,
    pub root_path: String,            // absolute; runtime-derived on open, persisted for diagnostics
    pub manifest: Vec<ResourceEntry>,
    pub scenes: Vec<SceneRef>,
    pub active_scene_index: usize,
    pub startup_scene_index: usize,
    pub materials: Vec<MaterialDef>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ResourceEntry {
    pub uuid: Uuid,
    pub name: String,
    pub kind: ResourceKind,
    pub relative_path: String,        // "models/foo.gltf", relative to src/resources/
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceKind { Model, Texture, Shader, Audio, Other }

#[derive(Clone, Serialize, Deserialize)]
pub struct SceneRef {
    pub uuid: Uuid,
    pub name: String,
    pub relative_path: String,        // "scenes/main.json"
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MaterialDef {
    pub uuid: Uuid,
    pub name: String,
    pub shader: ShaderChoice,
    pub albedo:    Option<Uuid>,
    pub normal:    Option<Uuid>,
    pub roughness: Option<Uuid>,
    pub metallic:  Option<Uuid>,
    pub emissive:  Option<Uuid>,
    pub color: [f32; 3],
    pub emissive_strength: f32,
    pub roughness_strength: f32,
    pub metallic_strength: f32,
    pub normal_strength: f32,
    pub transparent: bool,
    pub transparency_strength: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ShaderChoice {
    PbrLit,
    Unlit,
    Custom(Uuid),                     // shader resource uuid
}

// Stored once in app_state.state_data under key "editor".
pub struct EditorRoot {
    pub project: Option<ProjectState>,    // None until New/Open Project
    pub editor: EditorState,
}

// transient — never serialized
pub struct EditorState {
    pub selection: Selection,
    pub viewport_rect: Option<egui::Rect>,
    pub drag: Option<DragPayload>,
    pub modal: Option<Modal>,
    pub resource_browser_tab: ResourceTab,
    pub dirty: bool,                  // unsaved-changes indicator
}

pub enum Selection {
    None,
    SceneObject(Uuid),
    Light(Uuid),
    Camera,
    Material(Uuid),
    Resource(Uuid),
}

pub enum DragPayload {
    Resource(Uuid),                   // dragging from resource browser
    Material(Uuid),                   // dragging from Materials tab
}

pub enum Modal {
    ConfirmDelete { label: String, pending: PendingDelete },
    ImportError(String),
    NewSceneName(String),    // names entered for newly created scenes (no scene yet to inline-rename)
}

// Rename for existing scenes, materials, resources, objects, lights is inline
// (double-click row → text edit in place). No modal variants for those.

pub enum PendingDelete {
    Resource(Uuid),
    Material(Uuid),
    Scene(usize),
    SceneObject(Uuid),
    Light(Uuid),
}

pub enum ResourceTab { Models, Textures, Shaders, Materials, Scenes, Audio, Other }
```

### Material realization

`MaterialDef` (project-level data) does not stay in sync with `enigma::Material` automatically — enigma's `Material` is a GPU-bound resource that lives in `AppState.materials`. The bridge:

- `project::material::realize(def: &MaterialDef, project_root: &Path, display: &Display) -> enigma::Material` — constructs a fresh enigma `Material` from a definition by loading the referenced textures from disk and calling enigma's setter methods.
- `project::material::reconcile(project: &ProjectState, app_state: &mut AppState, display: &Display)` — called whenever a `MaterialDef` is edited (or on scene load). Iterates over `project.materials`, ensures each uuid maps to a live `enigma::Material`, rebuilds those that have changed. Tracks a per-material content hash to skip unchanged entries.

This means edits to a material in the inspector update the viewport immediately on the next frame.

### Selection invariants

- `Selection::SceneObject(uuid)` is only valid if the active scene's `AppState.objects` contains that uuid. If a scene switch removes it, selection collapses to `Selection::None`.
- Same for `Light(uuid)`.
- `Selection::Material(uuid)` and `Resource(uuid)` are project-level and survive scene switches.

---

## Project lifecycle

### New Project

(refactor of the current `project::try_new_project`)

1. User picks an empty directory via file dialog.
2. Validate directory is empty.
3. Scaffold:
   - `Cargo.toml` (template, with project name interpolated)
   - `src/main.rs` (template, with project name interpolated)
   - `src/resources/{models,textures,shader,audio,scenes,other}/`
   - `src/resources/scenes/main.json` — empty `AppStateSerializer` JSON
   - `enigma_project.json` — `ProjectState` with one `SceneRef` for `main.json`, empty manifest/materials, active = startup = 0.
4. `EditorRoot { project: Some(state), editor: EditorState::default() }`.

### Open Project

1. User picks a `enigma_project.json`.
2. `serde_json::from_str::<ProjectState>(...)`. On parse failure: modal "Project file is invalid: <err>"; previous state preserved.
3. Validate the project root contains the scaffolded folder structure; auto-create missing dirs (don't fail, just warn).
4. Load `scenes[active_scene_index].relative_path` into `AppState` via `inject_serializer(... additive=false)`.
5. Run `material::reconcile`.

### Save Project

Serialize `ProjectState` → `enigma_project.json` (pretty JSON). Clears `EditorState.dirty`.

### Save Scene

Serialize `AppState.to_serializer()` → `<project_root>/src/resources/scenes/<active_scene.relative_path file>`.

### Save Project & Scene (Cmd-S binding)

Both — Save Scene then Save Project. Standard editor expectation.

### Play / Debug Build / Release Build

(unchanged from current `Engine::run_project` / `build_project`) — shell out to `cargo run` / `cargo build [--release]` in the project root.

---

## Resource pipeline

### Import flow

Per resource kind, on "+ Import" or OS drag-drop onto a tab:

```rust
// project/resource.rs

pub fn import(
    src_path: &Path,
    kind: ResourceKind,
    project: &mut ProjectState,
) -> Result<Uuid, ImportError> {
    let kind_dir = kind.dir_name();   // "models" | "textures" | "shader" | "audio" | "other"
    let target_dir = Path::new(&project.root_path).join("src/resources").join(kind_dir);
    let file_name = src_path.file_name().ok_or(ImportError::BadSourcePath)?;
    let target_path = unique_path(&target_dir, file_name);  // adds _2, _3 if needed
    std::fs::copy(src_path, &target_path)?;

    let relative_path = format!("{}/{}", kind_dir, target_path.file_name().unwrap().to_str().unwrap());
    let uuid = Uuid::new_v4();
    project.manifest.push(ResourceEntry {
        uuid,
        name: file_stem(&target_path),
        kind,
        relative_path,
    });
    Ok(uuid)
}
```

File-dialog filters per kind:
- Models: `gltf`, `glb`
- Textures: `png`, `jpg`, `jpeg`, `tga`, `dds`, `bmp`
- Shaders: `glsl`, `vert`, `frag`, `vs`, `fs`
- Audio: `wav`, `ogg`, `mp3`, `flac`
- Other: (no filter)

### Lookup helpers

```rust
pub fn find<'a>(project: &'a ProjectState, uuid: Uuid) -> Option<&'a ResourceEntry>;
pub fn bytes(project: &ProjectState, uuid: Uuid) -> Result<Vec<u8>, BytesError>;
pub fn delete(project: &mut ProjectState, uuid: Uuid) -> Result<(), DeleteError>;
```

`delete` does not unlink the file outright. It moves the physical file to `<project_root>/.trash/<original_name>` and removes the manifest entry. The `.trash/` dir is project-local and gitignored by the scaffold. (User can manually empty.)

Deletion validates: if any `MaterialDef` references the uuid, prompt "Used by N materials — delete anyway?". Confirmed delete sets those references to `None`.

### No in-memory bytes cache (v1)

Each material realization re-reads texture bytes from disk. Profiling can drive caching later. Saves us a lifetime/borrow story for v1.

---

## Scene management

### Switch Scene

```rust
// project/scene.rs

pub fn switch(
    project: &mut ProjectState,
    target_index: usize,
    app_state: &mut AppState,
    display: &Display,
) -> Result<(), SceneError> {
    save_active(project, app_state)?;             // serialize current AppState to disk
    let target = &project.scenes[target_index];
    let serialized = read_scene_file(project, target)?;   // JSON -> AppStateSerializer
    clear_app_state(app_state);
    app_state.inject_serializer(serialized, display.clone(), /*additive=*/false);
    project.active_scene_index = target_index;
    crate::project::material::reconcile(project, app_state, display)?;
    Ok(())
}
```

`clear_app_state` is a helper: removes all objects, lights, materials, the skybox, camera reset. enigma's API: `objects.clear()`, `light.clear()`, etc. — small additions if enigma doesn't already expose them; alternatively, set `*app_state = AppState::new()` and re-apply the editor's camera defaults. Plan-writing will resolve which.

### New Scene

1. Prompt for name (modal).
2. Write empty scene JSON: `{}` parses to a default `AppStateSerializer`, but to be safe we write a default-serialized empty `AppStateSerializer` so the schema matches.
3. Append `SceneRef`; offer "Switch to new scene" (default yes).

### Set Startup Scene

Sets `project.startup_scene_index = active_scene_index`. The scaffolded `main.rs` template reads this index from `enigma_project.json` at build time (we'll regenerate `main.rs` only on explicit "Regenerate Main" action — out of v1 scope; for v1 the template is hardcoded to read scene 0, and Set Startup is a no-op pass-through that we document as "applies on next regen". This is acceptable for the spine.)

> **Honest scope note:** "Set Startup Scene" wires the data but not the runtime effect in v1. The Build Pipeline sub-project will close this loop properly when it generates the embedded scene module. Until then, the runtime project always launches into `scenes[0]`. The UI shows a "Startup" badge on that entry.

### Save active

`AppState.to_serializer()` → serde_json → write to `<root>/src/resources/<scene.relative_path>`.

---

## Selection model

### Hierarchy click

Direct: clicking a hierarchy row sets `EditorState.selection` to the matching variant.

### Resource browser click

Same: clicks set `Selection::Resource(uuid)` or `Selection::Material(uuid)`.

### Viewport click (ray-pick)

```rust
// editor/panels/viewport.rs

pub fn handle_viewport_input(ctx: &egui::Context, app_state: &mut AppState, editor: &mut EditorState) {
    let Some(rect) = editor.viewport_rect else { return; };
    let input = ctx.input(|i| i.clone());
    if !input.pointer.primary_released() { return; }
    let Some(pos) = input.pointer.interact_pos() else { return; };
    if !rect.contains(pos) { return; }
    // (skip if a drag is in progress)
    if editor.drag.is_some() { return; }

    let ndc = screen_to_ndc(pos, rect);
    let camera = app_state.camera.as_ref().expect("camera");
    let ray_origin_dir = camera.ray_for_ndc(ndc);   // helper to be added in enigma-engine if not in enigma-3d
    let mut ray = enigma::collision_world::Ray::new(
        ray_origin_dir.origin,
        ray_origin_dir.direction,
        camera.far_plane(),
    );
    ray.cast(app_state);
    if let Some(uuid) = ray.get_intersection_uuids().first().copied() {
        editor.selection = Selection::SceneObject(uuid);
    } else {
        editor.selection = Selection::None;
    }
}
```

`camera.ray_for_ndc` may not exist in enigma-3d today. If so, we add it as a small helper in enigma-3d (it's a natural fit there: NDC → unprojected ray). Plan-writing will confirm.

`viewport_rect` is captured each frame inside `CentralPanel::show`:

```rust
egui::CentralPanel::default()
    .frame(egui::Frame::none())     // transparent — live render shows through
    .show(ctx, |ui| {
        editor.viewport_rect = Some(ui.max_rect());
        // no other content; ray-pick happens before this in the frame
    });
```

### Hierarchy shape

For v1 the hierarchy panel renders three flat top-level groups: **Objects**, **Lights**, **Camera**. enigma's `Object` does not model parent/child (per the framework's CLAUDE.md), so no nesting. A future sub-project may add parenting; this is not blocked by our data model (we'd add `parent: Option<Uuid>` to the object's serializer in enigma-3d, not in the editor's data).

---

## UI panels

### Toolbar (`TopBottomPanel::top`)

- **Menu bar** (left-aligned):
  - **File** — New Project · Open Project · Save Project · Save Scene · Save Project & Scene · ─ · Quit
  - **Scene** — New Scene… · Switch Scene ▸ {dynamic list of scenes} · Set Current As Startup · Rename Active… · Delete Active…
  - **Play** (button) · **Debug Build** (button) · **Release Build** (button)
- **Status** (right-aligned): project name · active scene name · "•" dirty indicator

### Hierarchy (`SidePanel::left`, default width 220px, resizable, min 160px)

- Header: scene name (read-only, indicates which scene's contents are shown)
- Three collapsible groups (`egui::CollapsingHeader`):
  - **Objects** — list of `Object` rows by uuid
  - **Lights** — list of `Light` rows; ambient light row has "(ambient)" tag
  - **Camera** — single row
- Per-row:
  - icon + name (double-click to rename inline)
  - row highlighted when selected
  - right-click context menu: Rename · Duplicate · Delete
- **"+ Add"** dropdown per group:
  - Objects: Empty · Cube · Sphere · Plane · Quad · From Model… (sub-menu showing model resources)
  - Lights: Directional · Point · Ambient (replaces existing ambient with confirmation if present)
  - Camera: Reset Camera

Primitive object spawning uses enigma-3d's built-in primitive generators (cube/sphere/plane); if those don't exist as public API, we add small inline mesh builders in the editor.

### Inspector (`SidePanel::right`, default width 320px, resizable, min 240px)

- **Header**: selection type label + name (editable for nameable selections)
- **Body**: dispatches by selection variant; each section is a `CollapsingHeader` open by default:

| Selection | Sections shown |
|---|---|
| `None` | Empty state hint: "Select something in the hierarchy or resource browser." |
| `SceneObject(uuid)` | Transform · Mesh & Material |
| `Light(uuid)` | Transform · Light props |
| `Camera` | Transform · Camera props |
| `Material(uuid)` | Material editor (full §Material editor below) |
| `Resource(uuid, Texture)` | Resource info · Preview thumbnail (256×256 max) |
| `Resource(uuid, Model)` | Resource info · Vertex/Triangle counts (if loadable) |
| `Resource(uuid, Shader)` | Resource info · "Open in external editor" button (system default opener) |
| `Resource(uuid, Audio)` | Resource info · "Play" button (transient, uses `rodio` directly to preview) |
| `Resource(uuid, Other)` | Resource info only |

### Resource browser (`TopBottomPanel::bottom`, default height 200px, resizable, min 120px)

- **Tab strip** at top: Models · Textures · Shaders · Materials · Scenes · Audio · Other
- **Body** per tab:
  - List/grid view toggle (icon button top-right)
  - Entries shown as cards: icon/thumbnail + name. Click → select. Drag → starts `DragPayload::Resource(uuid)`. Dropping a model drag onto the hierarchy spawns the object at world origin (drop-in-viewport at hit position is a follow-up). Double-click on Models → spawn at origin. Double-click on Materials → focus inspector. Double-click on Scenes → Switch Scene.
  - Right-click row: Rename · Delete · Reveal in Finder (macOS) / file manager (linux/windows)
  - **+ Import** button (Models/Textures/Shaders/Audio/Other)
  - **+ New** button (Materials/Scenes)
  - **+ Duplicate** appears for Materials when one is selected

### Central panel (viewport)

`CentralPanel::default().frame(egui::Frame::none()).show(...)` — transparent. The live enigma render fills the screen; the egui panels are opaque overlays. We capture `ui.max_rect()` as `EditorState.viewport_rect` for ray-pick math. No content rendered inside.

---

## Material editor

Shown in the inspector when `Selection::Material(uuid)`.

```
Name                  [text input]
Shader                [▾ PBR Lit | Unlit | Custom]
  (if Custom)         [▾ <shader resources>]

[ Texture Slots ]                                    (rows shown depend on shader)
  Albedo      [thumb] [Browse…] [Clear]                    ← drag target
  Normal      [thumb] [Browse…] [Clear]
  Roughness   [thumb] [Browse…] [Clear]
  Metallic    [thumb] [Browse…] [Clear]
  Emissive    [thumb] [Browse…] [Clear]

[ Parameters ]
  Color                [color picker, [f32;3]]
  Roughness strength   [slider 0..1]
  Metallic strength    [slider 0..1]
  Normal strength      [slider 0..2]
  Emissive strength    [slider 0..10]
  Transparent          [checkbox]
  Transparency strength [slider 0..1, shown when transparent]
```

Shown slots by shader:
- **PBR Lit**: all 5
- **Unlit**: Albedo, Emissive
- **Custom**: all 5 (shader author may ignore some)

Each thumbnail row is both a drag-drop target (accepts `DragPayload::Resource(uuid)` where `kind == Texture`) and clickable to open a texture picker modal.

Edits dirty the live realized material immediately:
- On any edit, run `project::material::realize(def, root, display)` and replace the corresponding `enigma::Material` in `AppState.materials`. Next frame renders with new look.

### Creating materials

- Materials tab "+ New" → push a `MaterialDef::default_pbr()` with auto-name "Material N", select it, open inspector.
- "+ Duplicate" → clones the selected `MaterialDef` with a new uuid and " (Copy)" suffix.

### Object → Material assignment

In `SceneObject` inspector under "Mesh & Material":
- Mesh ref: dropdown of model resources (or "primitive: Cube" etc. for primitives)
- Material ref: dropdown of `project.materials` by name + "(none)"
- When changed, object's stored material uuid updates and `AppState`'s realized material association rebuilds.

### Missing material handling

If an object references a material uuid not in `project.materials`:
- Inspector shows a red "Missing material" badge.
- Object renders with a magenta debug material (constant unlit `vec3(1, 0, 1)`) so it's visible in-viewport.
- This is part of `material::reconcile`: any uuid referenced but not defined uses a singleton debug material.

---

## Light authoring

Shown in the inspector when `Selection::Light(uuid)`.

```
Name                  [text input]
Emission              [▾ Source | Ambient]    (ambient: at most one per scene)
Position              [Vec3 input]
Color                 [color picker]
Intensity             [slider 0..10+]
Direction             [Vec3 input, optional toggle]    (off = point light, on = directional)
Cast shadow           [checkbox]                        (greyed out if ≥4 shadow casters)
```

Maps directly to enigma-3d's `Light::new(position, color, intensity, direction, cast_shadow)` and `app_state.add_light(light, LightEmissionType::{Source|Ambient})`.

The "Add → Ambient" path is a singleton: if an ambient light exists, the action either replaces it (with confirmation modal) or no-ops if the user cancels.

The 4-shadow-caster cap is enforced at UI level by greying the checkbox; the enforcement is informational — enigma's shadow system will silently ignore extra casters per its existing behavior.

---

## Frame lifecycle

Per frame, in order:

1. enigma's update tick (camera input, any injected updates).
2. enigma's render passes (opaque, skybox, transparent, post-fx, blit).
3. egui pass — `editor::draw(ctx, app_state)`:
   1. Take `EditorRoot` out of `state_data` (mutable reference).
   2. Render toolbar (TopBottomPanel::top).
   3. Render hierarchy (SidePanel::left).
   4. Render inspector (SidePanel::right).
   5. Render resource browser (TopBottomPanel::bottom).
   6. Render CentralPanel (transparent) — capture viewport rect into `editor.viewport_rect`.
   7. Handle viewport input (ray-pick mouse-up before draws on next frame uses this frame's rect — first-frame mismatch is one frame of stale rect, harmless).
   8. Render any active modal as a top-level `egui::Window`.
4. Save? Build? Play? — handled by toolbar actions invoked during egui pass.

State mutation happens *during* egui rendering (panels invoke `actions::*`). Reads and writes happen on the same mutable reference; no async/queueing for v1.

---

## Actions module

`editor/actions.rs` collects mutation primitives. Where possible these are pure(-ish) — they take `&mut ProjectState` (+ context like `&mut AppState`, `&Display`) and apply a discrete change. Centralizing them makes a future undo system tractable.

Representative signatures (not exhaustive):

```rust
pub fn select(editor: &mut EditorState, selection: Selection);
pub fn import_resource(project: &mut ProjectState, src: &Path, kind: ResourceKind) -> Result<Uuid, ImportError>;
pub fn delete_resource(project: &mut ProjectState, uuid: Uuid) -> Result<(), DeleteError>;
pub fn rename_resource(project: &mut ProjectState, uuid: Uuid, new_name: String);

pub fn add_object(project: &ProjectState, app_state: &mut AppState, display: &Display, template: ObjectTemplate);
pub fn delete_object(app_state: &mut AppState, uuid: Uuid);
pub fn set_object_transform(app_state: &mut AppState, uuid: Uuid, transform: Transform);
pub fn set_object_material(app_state: &mut AppState, uuid: Uuid, material: Option<Uuid>);

pub fn add_light(app_state: &mut AppState, kind: LightTemplate);
pub fn set_light_props(app_state: &mut AppState, uuid: Uuid, props: LightProps);

pub fn create_material(project: &mut ProjectState) -> Uuid;
pub fn duplicate_material(project: &mut ProjectState, src: Uuid) -> Uuid;
pub fn delete_material(project: &mut ProjectState, uuid: Uuid) -> Result<(), DeleteError>;
pub fn update_material(project: &mut ProjectState, app_state: &mut AppState, display: &Display, uuid: Uuid, change: MaterialChange);

pub fn switch_scene(project: &mut ProjectState, app_state: &mut AppState, display: &Display, target: usize) -> Result<(), SceneError>;
pub fn new_scene(project: &mut ProjectState, name: String) -> Result<Uuid, SceneError>;
pub fn delete_scene(project: &mut ProjectState, index: usize) -> Result<(), SceneError>;
pub fn set_startup_scene(project: &mut ProjectState, index: usize);

pub fn save_project(project: &ProjectState) -> Result<(), SaveError>;
pub fn save_scene(project: &ProjectState, app_state: &AppState) -> Result<(), SaveError>;
```

These are not `Command` enum variants (we deferred the reducer pattern). Just plain functions.

---

## Error handling

| Situation | Behavior |
|---|---|
| Import file IO fails | Modal: "Could not import \<path>: \<err>". Manifest unchanged. |
| `enigma_project.json` malformed on Open | Modal: "Project file is invalid: \<err>". Project NOT loaded — current editor state preserved. |
| Scene file missing/malformed on Switch | Modal with error. Active scene unchanged. Bad `SceneRef` marked with red badge in Scene menu. |
| Resource uuid referenced by scene but missing in manifest | Object renders with magenta debug material; inspector shows red "Missing" badge. |
| Material uuid referenced by object but missing in `project.materials` | Same — magenta debug material; "Missing material" badge. |
| Deleting a resource used by N materials | Modal: "Used by N materials — delete anyway?". Confirm → orphan references set to `None`. |
| Deleting a material used by N objects | Modal: "Used by N objects — delete anyway?". Confirm → orphan refs trigger missing-material rendering. |
| Save/Save Scene with no project loaded | Buttons greyed. |
| Name collision on import | Auto-suffix `_2`, `_3`. Toast: "Renamed to \<X> to avoid collision." |
| Project root missing expected folders on Open | Auto-create missing subdirs. Log warning. Don't fail. |
| Camera not present in scene | Inspector hides Camera node. Toolbar shows "Add default camera" affordance. (enigma needs a camera to render.) |

All errors are recoverable — no panic paths reachable through user actions.

---

## Testing strategy

| Layer | Tested how |
|---|---|
| `actions::*` reducers | Unit tests with constructed `ProjectState`/`AppState`. Assert state after action. |
| Resource import | Tempdir + fake src file → `import()` → assert file copied, manifest entry present, uuid returned. |
| Manifest roundtrip | Construct `ProjectState` with all variants → JSON → parse → assert equality. |
| Scene save/load roundtrip | Construct `AppState` programmatically (objects + lights + camera + materials) → `to_serializer()` → JSON → `inject_serializer()` → re-serialize → assert equality. |
| Material realization | Build a `MaterialDef` with all 5 slots filled with fake texture bytes → `realize()` → assert resulting `enigma::Material` serializer fields match. |
| Material reconcile no-op | Run `reconcile` twice in a row → second call should not rebuild any materials (verify via a counter injected for tests). |
| Selection invariants | Switch scene that doesn't contain selected object uuid → assert selection collapses to `None`. |
| Ray-pick math | Given a viewport rect and a camera, assert NDC produced for a known mouse pos matches expected NDC. |

UI itself (egui rendering) has no unit tests. Manual smoke tests are documented inline below (a scripted run-through).

### Manual smoke test script

After implementation:

1. New Project in an empty dir → editor opens with default camera, no objects.
2. Import a `.gltf` model → appears in Models tab.
3. Import a `.png` texture → appears in Textures tab with a thumbnail.
4. Drag the model from Models tab onto the hierarchy → object spawns, viewport shows it (magenta because no material).
5. Materials tab → "+ New" → "Material 1" appears, inspector shows it. Drag the texture onto the Albedo slot.
6. In hierarchy, select the object → inspector → Material dropdown → pick "Material 1" → viewport shows textured object.
7. Hierarchy "+ Add" → Directional Light. Inspector → set color, intensity. Viewport updates.
8. Save Project & Scene. Close editor.
9. Reopen the project. Verify scene loads with the object and light present, material reference intact.
10. Press Play → child `cargo run` launches a window showing the scene.
11. Scene menu → New Scene "level2" → switch to it → hierarchy empty. Switch back → original scene returns intact.

---

## Out of scope (explicit, for clarity)

- **Viewport gizmos** (translate/rotate/scale handles). Edit via inspector input fields only in v1. → Sub-project 2.
- **Skybox picker, post-processing stack editing.** → Sub-project 3.
- **Animation timeline / skeletal scrubbing.** → Sub-project 4.
- **Build pipeline: `include_bytes!` codegen, packaging, icon embedding for built games, startup scene wiring at compile time.** → Sub-project 5.
- **Particle system authoring.** → Sub-project 1b (separate spec).
- **Object parent/child hierarchy.** Requires enigma-3d changes; not blocked by this design.
- **Undo/redo.** The actions module is shaped to make this possible later via a reducer wrapper.
- **Play-in-editor (dylib hot-reload).** Not planned.
- **Asset hot-reload from disk.** Not in v1.
- **Multi-select** (selecting >1 object at once). Not in v1.
- **Search / filtering** in resource browser. Not in v1.

---

## Migration from existing code

This sub-project is partly a refactor. The current code:

- `Engine` struct (in `src/main.rs`) and its serialization → replaced by `ProjectState`. The `Vec<BinaryResource>`/`Vec<TextResource>` fields holding inline bytes go away; the manifest pattern takes over.
- `src/project.rs` — split into `src/project/{mod, resource, scene, material}.rs`.
- `src/ui/project_window.rs` — split into `src/editor/panels/{toolbar,...}.rs`; menu actions move to `actions.rs`.
- `src/ui/resource_window.rs` — replaced by `src/editor/panels/resource_browser.rs`.
- `src/resources/mod.rs` — keep `CARGO_TOML`, `MAIN_RS`, `ICON` constants used for project scaffolding. Remove `BinaryResource`/`TextResource` definitions (no longer used).
- `src/serialization.rs` — empty file, delete.

The `main.rs` template (`src/resources/main.rs.resource`) used to scaffold runtime projects may need a minor update to load scene 0 by file path. Verify when implementing.

Old `enigma_project.json` files from before this work won't deserialize into `ProjectState`. Since no projects exist in the wild, no migration tool needed. Plan-writing should note this if any test fixtures use the old format.

---

## Acceptance criteria

This sub-project is done when, on a fresh clone:

1. `cargo run` launches the editor.
2. The manual smoke test script above completes end-to-end without crashes.
3. All listed unit tests pass.
4. `cargo check` is clean (no warnings introduced by this work — existing warnings unchanged).
5. The five panels (toolbar, hierarchy, inspector, resource browser, viewport) render correctly with the docked layout, resizable.
6. Scene save/load roundtrip preserves all editable state.
7. The runtime child project (output of "Play") opens a window and shows the saved scene with materials, lights, and camera present.
