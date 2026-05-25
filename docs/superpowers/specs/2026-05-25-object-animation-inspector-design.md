# Object Animation Inspector — Design

**Status:** approved
**Date:** 2026-05-25
**Scope:** Editor-side controls for previewing and persisting skeletal animations that ship with imported gltf objects. Smallest possible cut: leverage the runtime that already exists in `enigma-3d`.

## Goal

Allow the user to pick, play, pause, stop, scrub, and loop the skeletal animations that come bundled with a selected object, and have the chosen clip auto-play when the scene is loaded at runtime.

## Non-goals

- No timeline / keyframe authoring.
- No transform-only (non-skeletal) animation.
- No state machines, blending, crossfades, or events.
- No new public API on `enigma-3d` beyond serializing one already-existing field.
- No scene-level animation timeline; controls live on the selected object only.
- No undo/redo for the new fields in this pass.

## What already exists (no work needed)

- `enigma-3d::animation` defines `Bone`, `Skeleton`, `Animation`, `AnimationChannel`, `AnimationKeyframe`, `AnimationState`, all with serializers.
- `Object` already owns `animations: HashMap<String, animation::Animation>`, `skeleton: Option<Skeleton>`, and `current_animation: Option<AnimationState>`.
- `Object::play_animation(name, looping)`, `stop_animation()`, `get_current_animation()`, `has_skeletal_animation()`, `update(delta_time)`, `get_bone_transform_buffer(...)` are all in place.
- The main loop in `enigma-3d::lib.rs` (around line 1082) already calls `object.update(deltatime)` every frame, so once `current_animation` is `Some`, playback advances automatically — including in the editor.
- gltf import already populates `animations` and `skeleton` on the loaded `Object`.

## What's missing — the work this spec covers

1. `ObjectSerializer` in `enigma-3d` does not include `current_animation`, so the chosen clip is dropped on save.
2. `enigma-engine`'s inspector has no UI for animation; users can't see what clips an object has or trigger playback.
3. Nothing normalizes the playback time on save, so a scrubbed preview would otherwise pollute the saved scene.

## Architecture

### Change 1 — `enigma-3d` patch

Add a single optional field to the object serializer and wire it through both directions.

- **`ObjectSerializer`**: add `current_animation: Option<animation::AnimationState>`. `AnimationState` already derives `Serialize`/`Deserialize`, so no new serializer type is needed.
- **`Object::to_serializer()`**: copy `self.current_animation.clone()` into the new field.
- **`Object::from_serializer(...)`**: after the existing reconstruction, assign `object.current_animation = serializer.current_animation;`. Must not conflict with the default `current_animation: None` set in the constructor — assigning after construction is sufficient.

That is the entirety of the engine change. No new functions, no behavior change beyond plumbing.

### Change 2 — `enigma-engine` inspector section

A new file `src/editor/inspector/animation.rs`, containing one function the inspector calls when rendering an object's inspector panel. Wired into the existing inspector composition next to `mesh_material.rs`, `transform.rs`, etc.

The section renders only when `object.has_skeletal_animation()` is `true`. When the selected entity isn't an object, or the object has no skeletal data, the section is hidden entirely (not greyed out).

### Change 3 — save normalization in `enigma-engine`

Before serializing the scene (in `src/project/scene.rs::save_active`), walk `app_state.objects` and, for any object with `current_animation: Some(state)`, set `state.time = 0.0`. Speed and looping are left as-is. After serialization completes, restore the previous time values on the in-memory objects so the editor preview is not disturbed by the save action.

Alternative implementation if cleaner: clone the relevant state into the serializer, never mutating the live objects. The chosen approach should keep the live editor preview untouched after a save.

## UI specification

Section title: **Animation**. Collapsible, matches the visual style of existing inspector sections.

Contents (top to bottom):

1. **Clip dropdown**
   - Label: "Clip".
   - Options: `<None>` plus the keys of `object.get_animations()`, sorted alphabetically.
   - Selecting a clip different from the current one calls `object.play_animation(name, current_looping)`. The clip name change resets `time` to 0 (this happens inside `play_animation` already).
   - Selecting `<None>` calls `object.stop_animation()`.
   - If the persisted `current_animation.name` does not exist in `object.get_animations()` (e.g. the gltf was re-exported), the dropdown shows `<None>` and the underlying state is left intact until the user picks something else. No panic.

2. **Transport row** — three buttons side-by-side.
   - **Play**: set `current_animation.speed = 1.0`. Disabled (greyed out) when `current_animation` is `None`. User must pick a clip from the dropdown first.
   - **Pause**: set `current_animation.speed = 0.0`. Disabled when `current_animation` is `None`.
   - **Stop**: calls `object.stop_animation()` (sets `current_animation` to `None`). Disabled when already `None`.

3. **Loop checkbox**
   - Label: "Loop".
   - Bound to `current_animation.looping` when `Some`. Disabled when `None`.

4. **Time scrubber**
   - Slider, range `0.0 ..= clip.duration` where `clip` is the currently-selected animation.
   - Bound to `current_animation.time` when `Some`. Disabled when `None`.
   - Dragging while paused (`speed == 0`) re-poses the object at that time. Dragging while playing yanks playback to that time and continues from there.

**Side-effect note on clip switching:** `play_animation(name, looping)` constructs a fresh `AnimationState` with `time = 0.0` and `speed = 1.0`. So picking a new clip from the dropdown *always* starts that clip playing, even if the previous clip was paused. This is the engine's existing behavior and we inherit it.

5. **Read-out line**
   - Small text: `"{time:.2} / {duration:.2}s   {clip_name}"`. Hidden when `current_animation` is `None`.

## Persistence semantics

- On save: `current_animation.time` normalized to `0.0` on every object that has one (the editor's live state is restored after serialization). Loop, speed, and clip name are saved as-is.
- On load: `inject_serializer` already populates objects from `ObjectSerializer`. With the new field plumbed, `current_animation` is restored and the existing per-frame `update(dt)` tick advances it — clip auto-plays.
- Stop / `<None>`: `current_animation` saves as `None`. Scene loads with no animation playing.

## Data flow (per frame, editor preview)

1. User picks clip in inspector → `object.play_animation(name, looping)` mutates `current_animation` on the live `Object` in `app_state.objects`.
2. Main loop tick → `object.update(dt)` advances `current_animation.time` by `dt * speed`.
3. Render path calls `object.get_bone_transform_buffer(display)` → bone matrices computed from `current_animation.time` and the skeleton.
4. Render uses those matrices via the existing uniform buffer path. No new render code.

## Failure modes and edge cases

- **Object with `skeleton: Some(...)` but empty `animations`**: `has_skeletal_animation()` returns `false`, so the section is hidden. Acceptable.
- **Object with `animations` but `skeleton: None`**: same — section hidden. Skeletal playback needs both.
- **Saved clip name no longer present after gltf re-export**: dropdown shows `<None>`. State preserved until user changes it. No crash.
- **Clip with `duration == 0.0`**: scrubber range collapses to `0.0..=0.0`. Slider effectively disabled. Acceptable, rare.
- **Switching clips while one is playing**: handled by `play_animation`, which overwrites `current_animation` with a fresh state at time 0.

## Order of operations

1. Land the `enigma-3d` patch on its `main` branch and push.
2. In `enigma-engine`, `cargo update -p enigma-3d` to pull the new SHA.
3. Implement the inspector section and the save-normalization step.
4. Manual verification: load a scene with a gltf-animated object, play / pause / scrub / stop, save, reload, confirm autoplay.

The engine patch must land first because `enigma-engine`'s code will not compile without the new `current_animation` field on `ObjectSerializer` once the inspector wiring is in.

## Files touched

- `enigma-3d/src/object.rs` — add field on `ObjectSerializer`, plumb through `to_serializer` and `from_serializer`.
- `enigma-engine/Cargo.lock` — updated via `cargo update`.
- `enigma-engine/src/editor/inspector/animation.rs` — new.
- `enigma-engine/src/editor/inspector/mod.rs` — register the new section.
- `enigma-engine/src/editor/panels/inspector.rs` — call the new section from the object-inspector flow (if that's where sections are composed).
- `enigma-engine/src/project/scene.rs` — pre-save normalization, post-save restore.

## Testing

- Unit test for the `enigma-3d` change: round-trip an `Object` with `current_animation: Some(...)` through `to_serializer` / `from_serializer` and assert equality of the relevant fields.
- Integration test (or manual): scene round-trip in `enigma-engine` with an animated object, verifying `current_animation.time == 0.0` on the saved JSON and that the in-memory time is preserved across save.
- Manual editor verification: gltf object loaded, play / pause / stop / scrub / loop all behave; reload restores the chosen clip and starts playing from t=0.
