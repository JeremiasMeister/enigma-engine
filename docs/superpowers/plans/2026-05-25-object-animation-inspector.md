# Object Animation Inspector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface enigma-3d's existing skeletal-animation runtime in the editor — pick a clip on the selected object, play/pause/stop/loop/scrub, and have the chosen clip auto-play when the scene is reloaded.

**Architecture:** A two-line plumbing patch to `enigma-3d`'s `ObjectSerializer` makes `current_animation` survive save/load. A new `Animation` section in the object inspector binds UI to `Object::play_animation`, `stop_animation`, and direct mutation of `current_animation` fields. Save normalization happens at the `AppStateSerializer` layer in `enigma-engine` (no live-object mutation, no engine API change).

**Tech Stack:** Rust, glium/egui, the `enigma-3d` crate (a github-tracked sibling repo at `/Users/cg-jm/jm/enigma-3d`).

**Spec reference:** `docs/superpowers/specs/2026-05-25-object-animation-inspector-design.md`

---

## File Structure

**enigma-3d** (sibling repo at `/Users/cg-jm/jm/enigma-3d`):
- Modify: `src/object.rs`
  - `ObjectSerializer` struct (around line 28-39): add `current_animation: Option<animation::AnimationState>`.
  - `Object::to_serializer` (around line 189-214): plumb new field.
  - `Object::from_serializer` (around line 216-239): plumb new field.
  - Add `Object::get_current_animation_mut(&mut self) -> &mut Option<AnimationState>` (near line 538, next to `get_current_animation`).
  - Tests at bottom of file: add round-trip test.

**enigma-engine**:
- Modify: `Cargo.lock` (via `cargo update -p enigma-3d`).
- Modify: `src/editor/inspector/mod.rs` — `pub mod animation;`.
- Modify: `src/editor/panels/inspector.rs` (line 22-25) — call new section after `mesh_material::draw`.
- Modify: `src/project/scene.rs::save_active` — zero `current_animation.time` on serializer before write.
- Create: `src/editor/inspector/animation.rs` — the new inspector section.

---

## Task 1: enigma-3d — add round-trip test for `current_animation` (TDD: failing first)

**Files:**
- Modify: `/Users/cg-jm/jm/enigma-3d/src/object.rs` (tests module at bottom)

- [ ] **Step 1: Create a feature branch in enigma-3d**

```bash
cd /Users/cg-jm/jm/enigma-3d
git checkout main
git pull
git checkout -b feat/persist-current-animation
```

- [ ] **Step 2: Add the failing test to the existing `#[cfg(test)] mod tests` block at the bottom of `/Users/cg-jm/jm/enigma-3d/src/object.rs`**

Insert this test inside `mod tests { ... }` (just before its closing `}`):

```rust
    #[test]
    fn current_animation_survives_round_trip() {
        // Build an Object with one fake animation and start playing it.
        let mut obj = Object::new(Some("anim_holder".into()));
        let anim = animation::Animation {
            name: "Idle".to_string(),
            duration: 2.5,
            channels: Vec::new(),
        };
        obj.get_animations_mut().insert("Idle".to_string(), anim);
        obj.play_animation("Idle", /*looping=*/true);
        // Mutate time/speed so we can see them survive the round-trip.
        if let Some(state) = obj.get_current_animation_mut().as_mut() {
            state.time = 1.25;
            state.speed = 0.0;
        }

        let serializer = obj.to_serializer();
        let restored = Object::from_serializer(serializer);

        let state = restored
            .get_current_animation()
            .as_ref()
            .expect("current_animation should round-trip as Some");
        assert_eq!(state.name, "Idle");
        assert!((state.time - 1.25).abs() < 1e-6, "time = {}", state.time);
        assert!((state.speed - 0.0).abs() < 1e-6, "speed = {}", state.speed);
        assert_eq!(state.looping, true);
    }
```

- [ ] **Step 3: Run the test, expect it to fail to compile**

```bash
cd /Users/cg-jm/jm/enigma-3d
cargo test --lib current_animation_survives_round_trip
```

Expected: compile error referencing missing `get_current_animation_mut` method and (after we add it) missing `current_animation` field on the deserialized object's state. Either failure mode is fine — we want red before green.

- [ ] **Step 4: Commit the failing test**

```bash
cd /Users/cg-jm/jm/enigma-3d
git add src/object.rs
git commit -m "test: round-trip current_animation through ObjectSerializer (red)"
```

---

## Task 2: enigma-3d — add `get_current_animation_mut` and plumb `current_animation` through the serializer

**Files:**
- Modify: `/Users/cg-jm/jm/enigma-3d/src/object.rs`

- [ ] **Step 1: Add `current_animation` field to `ObjectSerializer`**

Locate the struct (it currently looks like this, around line 28):

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectSerializer {
    pub name: String,
    pub transform: TransformSerializer,
    collision: bool,
    shapes: Vec<Shape>,
    materials: Vec<String>,
    unique_id: String,
    cloned_id: String,
    animations: HashMap<String, animation::AnimationSerializer>,
    skeleton: Option<animation::SkeletonSerializer>,
}
```

Add the new field (use `#[serde(default)]` so old scene files without the field still deserialize):

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectSerializer {
    pub name: String,
    pub transform: TransformSerializer,
    collision: bool,
    shapes: Vec<Shape>,
    materials: Vec<String>,
    unique_id: String,
    cloned_id: String,
    animations: HashMap<String, animation::AnimationSerializer>,
    skeleton: Option<animation::SkeletonSerializer>,
    #[serde(default)]
    pub current_animation: Option<AnimationState>,
}
```

> `pub` is intentional: the editor crate (`enigma-engine`) needs to read and mutate this field directly when normalizing playback time at save (Task 9). The rest of `ObjectSerializer`'s fields stay private because nothing outside enigma-3d touches them.

- [ ] **Step 2: Populate the new field in `Object::to_serializer`**

Locate `to_serializer` (around line 189). It currently ends with the struct literal:

```rust
        ObjectSerializer {
            name,
            transform,
            shapes,
            materials,
            unique_id,
            cloned_id,
            collision: self.collision,
            animations,
            skeleton: match &self.skeleton {
                Some(skeleton) => Some(skeleton.to_serializer()),
                None => None
            },
        }
```

Add the new field to the literal:

```rust
        ObjectSerializer {
            name,
            transform,
            shapes,
            materials,
            unique_id,
            cloned_id,
            collision: self.collision,
            animations,
            skeleton: match &self.skeleton {
                Some(skeleton) => Some(skeleton.to_serializer()),
                None => None
            },
            current_animation: self.current_animation.clone(),
        }
```

- [ ] **Step 3: Restore the new field in `Object::from_serializer`**

Locate `from_serializer` (around line 216). It currently ends with the skeleton assignment then `object`:

```rust
        object.skeleton = match serializer.skeleton {
            Some(s) => Some(animation::Skeleton::from_serializer(s)),
            None => None
        };
        object
    }
```

Insert the assignment just before `object`:

```rust
        object.skeleton = match serializer.skeleton {
            Some(s) => Some(animation::Skeleton::from_serializer(s)),
            None => None
        };
        object.current_animation = serializer.current_animation;
        object
    }
```

- [ ] **Step 4: Add `get_current_animation_mut`**

Locate `get_current_animation` (around line 538):

```rust
    pub fn get_current_animation(&self) -> &Option<AnimationState> {
        &self.current_animation
    }
```

Add a mutable accessor right after it:

```rust
    pub fn get_current_animation(&self) -> &Option<AnimationState> {
        &self.current_animation
    }

    pub fn get_current_animation_mut(&mut self) -> &mut Option<AnimationState> {
        &mut self.current_animation
    }
```

- [ ] **Step 5: Run the test, expect it to pass**

```bash
cd /Users/cg-jm/jm/enigma-3d
cargo test --lib current_animation_survives_round_trip
```

Expected: `test object::tests::current_animation_survives_round_trip ... ok`

- [ ] **Step 6: Run the full enigma-3d test suite to verify nothing regressed**

```bash
cd /Users/cg-jm/jm/enigma-3d
cargo test --lib
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
cd /Users/cg-jm/jm/enigma-3d
git add src/object.rs
git commit -m "feat: persist current_animation through ObjectSerializer + add get_current_animation_mut"
```

---

## Task 3: enigma-3d — push to main

- [ ] **Step 1: Confirm the branch is clean and tests pass**

```bash
cd /Users/cg-jm/jm/enigma-3d
git status
cargo test --lib
```

Expected: clean status, all tests pass.

- [ ] **Step 2: Merge into main locally and push**

`enigma-engine`'s `Cargo.toml` tracks `enigma-3d` by `branch = "main"`. We need the change on main.

```bash
cd /Users/cg-jm/jm/enigma-3d
git checkout main
git merge --ff-only feat/persist-current-animation
git push origin main
```

Expected: fast-forward succeeds, push reports a new SHA on origin/main.

- [ ] **Step 3: Note the new SHA for reference**

```bash
cd /Users/cg-jm/jm/enigma-3d
git rev-parse HEAD
```

Expected: prints a SHA. Save it — useful for verifying the Cargo.lock update in the next task.

---

## Task 4: enigma-engine — update dependency and confirm clean build

**Files:**
- Modify: `/Users/cg-jm/jm/enigma-engine/Cargo.lock`

- [ ] **Step 1: Update the enigma-3d dependency**

```bash
cd /Users/cg-jm/jm/enigma-engine
cargo update -p enigma-3d
```

Expected: cargo updates the SHA in `Cargo.lock` to the one printed in Task 3 Step 3.

- [ ] **Step 2: Verify the build still works without any inspector changes yet**

```bash
cd /Users/cg-jm/jm/enigma-engine
cargo check
```

Expected: clean compile, no warnings related to the engine change.

- [ ] **Step 3: Commit the lockfile bump**

```bash
cd /Users/cg-jm/jm/enigma-engine
git add Cargo.lock
git commit -m "chore: bump enigma-3d to pick up serialized current_animation"
```

---

## Task 5: enigma-engine — create the animation inspector skeleton

**Files:**
- Create: `/Users/cg-jm/jm/enigma-engine/src/editor/inspector/animation.rs`
- Modify: `/Users/cg-jm/jm/enigma-engine/src/editor/inspector/mod.rs`
- Modify: `/Users/cg-jm/jm/enigma-engine/src/editor/panels/inspector.rs`

- [ ] **Step 1: Create the new module with a stub `draw` function**

Write to `/Users/cg-jm/jm/enigma-engine/src/editor/inspector/animation.rs`:

```rust
use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

pub fn draw(ui: &mut Ui, app_state: &mut AppState, object_uuid: Uuid) {
    let has_anim = app_state.objects.iter()
        .find(|o| o.get_unique_id() == object_uuid)
        .map(|o| o.has_skeletal_animation())
        .unwrap_or(false);

    if !has_anim {
        return;
    }

    egui::CollapsingHeader::new("Animation").default_open(true).show(ui, |ui| {
        ui.label("(animation UI goes here)");
    });
}
```

- [ ] **Step 2: Register the module in `inspector/mod.rs`**

Open `/Users/cg-jm/jm/enigma-engine/src/editor/inspector/mod.rs`. Current contents:

```rust
pub mod transform;
pub mod mesh_material;
pub mod light;
pub mod camera;
pub mod material_editor;
pub mod resource_meta;
pub mod scene_settings;
pub mod particle_editor;
pub mod particle_instance;
pub mod terrain_editor;
```

Add the new module (alphabetically near the top is fine):

```rust
pub mod animation;
pub mod transform;
pub mod mesh_material;
pub mod light;
pub mod camera;
pub mod material_editor;
pub mod resource_meta;
pub mod scene_settings;
pub mod particle_editor;
pub mod particle_instance;
pub mod terrain_editor;
```

- [ ] **Step 3: Call the new section from `panels/inspector.rs`**

Open `/Users/cg-jm/jm/enigma-engine/src/editor/panels/inspector.rs`. The `Selection::SceneObject(uuid)` arm currently reads:

```rust
        Selection::SceneObject(uuid) => {
            inspector::transform::draw_for_object(ui, app_state, uuid);
            inspector::mesh_material::draw(ui, app_state, uuid);
        }
```

Add the animation call after mesh_material:

```rust
        Selection::SceneObject(uuid) => {
            inspector::transform::draw_for_object(ui, app_state, uuid);
            inspector::mesh_material::draw(ui, app_state, uuid);
            inspector::animation::draw(ui, app_state, uuid);
        }
```

- [ ] **Step 4: Build to verify the skeleton compiles**

```bash
cd /Users/cg-jm/jm/enigma-engine
cargo check
```

Expected: clean compile.

- [ ] **Step 5: Commit**

```bash
cd /Users/cg-jm/jm/enigma-engine
git add src/editor/inspector/animation.rs src/editor/inspector/mod.rs src/editor/panels/inspector.rs
git commit -m "feat(editor): scaffold animation inspector section"
```

---

## Task 6: enigma-engine — implement the clip dropdown

**Files:**
- Modify: `/Users/cg-jm/jm/enigma-engine/src/editor/inspector/animation.rs`

- [ ] **Step 1: Replace the stub `draw` with a version that has the clip dropdown**

Open `/Users/cg-jm/jm/enigma-engine/src/editor/inspector/animation.rs` and replace its contents with:

```rust
use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

pub fn draw(ui: &mut Ui, app_state: &mut AppState, object_uuid: Uuid) {
    let Some(obj_idx) = app_state.objects.iter().position(|o| o.get_unique_id() == object_uuid) else {
        return;
    };
    if !app_state.objects[obj_idx].has_skeletal_animation() {
        return;
    }

    egui::CollapsingHeader::new("Animation").default_open(true).show(ui, |ui| {
        let obj = &app_state.objects[obj_idx];

        // Collect clip names sorted.
        let mut clip_names: Vec<String> = obj.get_animations().keys().cloned().collect();
        clip_names.sort();

        // Current selection.
        let current_name: Option<String> = obj.get_current_animation()
            .as_ref()
            .map(|s| s.name.clone());
        let current_looping: bool = obj.get_current_animation()
            .as_ref()
            .map(|s| s.looping)
            .unwrap_or(false);

        let label = current_name.clone().unwrap_or_else(|| "<None>".to_string());

        let mut picked: Option<Option<String>> = None;
        egui::ComboBox::from_label("Clip")
            .selected_text(label)
            .show_ui(ui, |ui| {
                if ui.selectable_label(current_name.is_none(), "<None>").clicked() {
                    picked = Some(None);
                }
                for name in &clip_names {
                    let selected = current_name.as_deref() == Some(name.as_str());
                    if ui.selectable_label(selected, name).clicked() {
                        picked = Some(Some(name.clone()));
                    }
                }
            });

        if let Some(choice) = picked {
            let obj_mut = &mut app_state.objects[obj_idx];
            match choice {
                None => obj_mut.stop_animation(),
                Some(name) => {
                    // Skip the call if it would just reset the already-current clip.
                    if current_name.as_deref() != Some(name.as_str()) {
                        obj_mut.play_animation(&name, current_looping);
                    }
                }
            }
        }
    });
}
```

- [ ] **Step 2: Build and run the editor to sanity-check that the dropdown shows up on an animated object**

```bash
cd /Users/cg-jm/jm/enigma-engine
cargo check
```

Expected: clean compile. (Full `cargo run` verification happens in the final manual-verification task — `cargo check` is enough here.)

- [ ] **Step 3: Commit**

```bash
cd /Users/cg-jm/jm/enigma-engine
git add src/editor/inspector/animation.rs
git commit -m "feat(editor): animation inspector — clip dropdown"
```

---

## Task 7: enigma-engine — add the transport row (Play / Pause / Stop)

**Files:**
- Modify: `/Users/cg-jm/jm/enigma-engine/src/editor/inspector/animation.rs`

- [ ] **Step 1: Insert the transport row after the dropdown's `if let Some(choice)` block**

In `animation.rs`, at the end of the `CollapsingHeader::new("Animation")` closure body (after the dropdown handling), add:

```rust
        // --- Transport row ---
        let has_current = app_state.objects[obj_idx].get_current_animation().is_some();
        ui.horizontal(|ui| {
            if ui.add_enabled(has_current, egui::Button::new("Play")).clicked() {
                if let Some(state) = app_state.objects[obj_idx].get_current_animation_mut().as_mut() {
                    state.speed = 1.0;
                }
            }
            if ui.add_enabled(has_current, egui::Button::new("Pause")).clicked() {
                if let Some(state) = app_state.objects[obj_idx].get_current_animation_mut().as_mut() {
                    state.speed = 0.0;
                }
            }
            if ui.add_enabled(has_current, egui::Button::new("Stop")).clicked() {
                app_state.objects[obj_idx].stop_animation();
            }
        });
```

- [ ] **Step 2: Build**

```bash
cd /Users/cg-jm/jm/enigma-engine
cargo check
```

Expected: clean compile.

- [ ] **Step 3: Commit**

```bash
cd /Users/cg-jm/jm/enigma-engine
git add src/editor/inspector/animation.rs
git commit -m "feat(editor): animation inspector — transport row"
```

---

## Task 8: enigma-engine — add Loop checkbox, time scrubber, and read-out

**Files:**
- Modify: `/Users/cg-jm/jm/enigma-engine/src/editor/inspector/animation.rs`

- [ ] **Step 1: Insert the remaining controls after the transport row**

After the transport `ui.horizontal(...)`, add:

```rust
        // --- Loop checkbox ---
        let mut looping = current_looping;
        let loop_resp = ui.add_enabled(has_current, egui::Checkbox::new(&mut looping, "Loop"));
        if loop_resp.changed() {
            if let Some(state) = app_state.objects[obj_idx].get_current_animation_mut().as_mut() {
                state.looping = looping;
            }
        }

        // --- Time scrubber ---
        // Compute duration from the active clip (default 0 if not found / no current animation).
        let (mut time_value, duration, clip_name_for_readout): (f32, f32, Option<String>) = {
            let obj = &app_state.objects[obj_idx];
            match obj.get_current_animation().as_ref() {
                Some(state) => {
                    let dur = obj.get_animations()
                        .get(&state.name)
                        .map(|a| a.duration)
                        .unwrap_or(0.0);
                    (state.time, dur, Some(state.name.clone()))
                }
                None => (0.0, 0.0, None),
            }
        };

        let scrub_resp = ui.add_enabled(
            has_current && duration > 0.0,
            egui::Slider::new(&mut time_value, 0.0..=duration.max(f32::EPSILON)).text("Time"),
        );
        if scrub_resp.changed() {
            if let Some(state) = app_state.objects[obj_idx].get_current_animation_mut().as_mut() {
                state.time = time_value;
            }
        }

        // --- Read-out ---
        if let Some(name) = clip_name_for_readout {
            ui.label(format!("{:.2} / {:.2}s   {}", time_value, duration, name));
        }
```

- [ ] **Step 2: Build**

```bash
cd /Users/cg-jm/jm/enigma-engine
cargo check
```

Expected: clean compile.

- [ ] **Step 3: Commit**

```bash
cd /Users/cg-jm/jm/enigma-engine
git add src/editor/inspector/animation.rs
git commit -m "feat(editor): animation inspector — loop checkbox, time scrubber, read-out"
```

---

## Task 9: enigma-engine — save-time normalization of `current_animation.time`

**Files:**
- Modify: `/Users/cg-jm/jm/enigma-engine/src/project/scene.rs`

- [ ] **Step 1: Add a failing unit test for the normalization helper**

Open `/Users/cg-jm/jm/enigma-engine/src/project/scene.rs`. Locate the `#[cfg(test)] mod tests` block at the bottom. Add this test inside it:

```rust
    #[test]
    fn normalize_animation_times_zeros_time_on_serializer() {
        use enigma_3d::object::{Object, ObjectSerializer};
        use enigma_3d::animation::{Animation, AnimationState};

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
```

- [ ] **Step 2: Run the test, confirm it fails to compile**

```bash
cd /Users/cg-jm/jm/enigma-engine
cargo test --lib normalize_animation_times_zeros_time_on_serializer
```

Expected: compile error — `normalize_animation_times` not found. (`ObjectSerializer.current_animation` is already `pub` per Task 2 Step 1.)

- [ ] **Step 3: Implement `normalize_animation_times` in `scene.rs`**

At module scope in `/Users/cg-jm/jm/enigma-engine/src/project/scene.rs` (anywhere outside the existing functions and tests block — e.g. after `clear_scene`), add:

```rust
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
```

- [ ] **Step 4: Wire it into `save_active`**

Locate `save_active` at the top of `scene.rs`. It currently reads:

```rust
pub fn save_active(project: &ProjectState, app_state: &AppState) -> Result<(), SceneError> {
    let scene = project.scenes.get(project.active_scene_index).ok_or(SceneError::NoActiveScene)?;
    let path = scene_path(project, scene);
    let serializer = app_state.to_serializer();
    let text = serde_json::to_string_pretty(&serializer).map_err(SceneError::Parse)?;
    fs::write(&path, text).map_err(SceneError::Io)?;
    Ok(())
}
```

Change it to make the serializer mutable and call the helper:

```rust
pub fn save_active(project: &ProjectState, app_state: &AppState) -> Result<(), SceneError> {
    let scene = project.scenes.get(project.active_scene_index).ok_or(SceneError::NoActiveScene)?;
    let path = scene_path(project, scene);
    let mut serializer = app_state.to_serializer();
    normalize_animation_times(&mut serializer.objects);
    let text = serde_json::to_string_pretty(&serializer).map_err(SceneError::Parse)?;
    fs::write(&path, text).map_err(SceneError::Io)?;
    Ok(())
}
```

- [ ] **Step 5: Run the test, expect pass**

```bash
cd /Users/cg-jm/jm/enigma-engine
cargo test --lib normalize_animation_times_zeros_time_on_serializer
```

Expected: PASS.

- [ ] **Step 6: Run the full test suite to verify no regression**

```bash
cd /Users/cg-jm/jm/enigma-engine
cargo test --lib
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
cd /Users/cg-jm/jm/enigma-engine
git add src/project/scene.rs
git commit -m "feat(editor): normalize current_animation.time to 0 on save"
```

---

## Task 10: Manual end-to-end verification in the editor

This is the only check that proves the UI actually animates the rig and that save/reload restores the chosen clip. The earlier `cargo check` calls verify it compiles, not that it works.

- [ ] **Step 1: Run the editor with a project that contains an animated gltf object**

```bash
cd /Users/cg-jm/jm/enigma-engine
cargo run --release
```

Open a project where at least one scene contains a gltf object with skeletal animations (or import one via the existing resource browser).

- [ ] **Step 2: Verify the Animation section appears only for animated objects**

- Click a regular cube/sphere in the hierarchy → Animation section is **hidden**.
- Click the animated object → Animation section is **visible**, dropdown lists the imported clips, transport buttons start disabled (since `current_animation` is `None`).

- [ ] **Step 3: Verify playback**

- Pick a clip from the dropdown → the object should start moving in the viewport.
- Click **Pause** → motion freezes mid-clip; the time read-out stops advancing.
- Click **Play** → motion resumes from that paused time.
- Click **Stop** → object snaps back to bind pose; transport buttons disable themselves; dropdown shows `<None>`.

- [ ] **Step 4: Verify scrubber**

- Pick a clip, click Pause, drag the Time slider → the pose updates live as you scrub.
- Click Play while scrubbed mid-clip → playback continues from that time.

- [ ] **Step 5: Verify Loop**

- Pick a clip, set Loop on. Watch the animation reach `duration` — it should wrap and continue.
- Toggle Loop off mid-playback. Once the clip reaches `duration`, it should freeze on the final frame.

- [ ] **Step 6: Verify save/reload autoplay**

- Pick a clip with Loop on, click Play, let it run for a few seconds.
- File → Save Scene.
- Open the saved JSON file and confirm the relevant object has `"current_animation": { "name": "...", "time": 0.0, "speed": 1.0, "looping": true }`.
- Confirm the in-editor playback time is still at whatever it was before save (not reset to 0 in memory).
- Reload the scene (switch to another scene and back, or restart the editor and reopen the project).
- The animation should start playing automatically from the beginning.

- [ ] **Step 7: Verify stale-clip resilience**

- Manually edit the saved scene JSON, change the `current_animation.name` to `"DoesNotExist"`, save the file.
- Reload the scene → editor should not crash. Dropdown shows `<None>` for that object. The mismatched state is left intact internally until the user picks a real clip.

- [ ] **Step 8: Commit a verification note (optional)**

If anything in steps 1–7 surprised you, jot the observation in the PR description. No code commit required for this task unless a fix is needed.

---

## Self-Review

Checked plan against `docs/superpowers/specs/2026-05-25-object-animation-inspector-design.md`:

- ✅ Spec §"Architecture / Change 1" → Tasks 1–3.
- ✅ Spec §"Architecture / Change 2" → Tasks 5–8.
- ✅ Spec §"Architecture / Change 3" → Task 9.
- ✅ Spec §"UI specification / Clip dropdown" → Task 6.
- ✅ Spec §"UI specification / Transport row" → Task 7.
- ✅ Spec §"UI specification / Loop, scrubber, read-out" → Task 8.
- ✅ Spec §"Persistence semantics" → Task 9 (zeros time, preserves loop/speed/name).
- ✅ Spec §"Failure modes / stale clip name" → Task 6 (dropdown falls back to `<None>` when name not in animations) + Task 10 Step 7 (manual verification).
- ✅ Spec §"Failure modes / duration == 0" → Task 8 (scrubber disabled when `duration <= 0`).
- ✅ Spec §"Testing / round-trip" → Task 1 + Task 2.
- ✅ Spec §"Testing / save normalization" → Task 9.
- ✅ Spec §"Testing / manual editor verification" → Task 10.
- ✅ Spec §"Order of operations" → tasks ordered engine-first.

**Placeholder scan:** No "TBD", no "TODO", no "implement later". Every code step shows the actual code.

**Type consistency:** `get_current_animation_mut` defined in Task 2, used in Tasks 7, 8, 9. `normalize_animation_times` defined in Task 9 Step 3, used in Step 4. `ObjectSerializer.current_animation` defined in Task 2 Step 1, used in Task 9 test (Step 1).

No open questions remain. The plan is self-contained.
