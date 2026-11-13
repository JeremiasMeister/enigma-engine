# Narrative editor

Open a project, then choose **Narrative** in the engine toolbar. Existing stories in `src/resources/story/*.json` appear in the story selector. `characters.json` is reserved for the shared character library.

## Authoring

- **Story:** opens a focused writing pane. Pick a moment by its dialogue/speaker preview in the outline, choose who speaks, and write. **Continue writing →** creates and opens the next moment with a connected Continue choice. **+ Choice & new moment** creates a linked branch: write the choice text, then **Write this →** to author its outcome. Repeat to offer alternatives. **+ Link existing moment** reconnects to a previously written passage using readable destination previews. Requirements and effects stay folded away until needed; character attributes are selected by character name and attribute. Custom world keys remain available under the advanced picker. Step settings contain stable IDs, rename, duplicate, and “Make entry”; rename rewrites incoming links.
- **Audio:** each moment has an **Audio for this moment** searchable picker, including **None**. Choose MP3, WAV, OGG, or FLAC files from any folder beneath `src/resources`; use Refresh after importing or adding files. The story stores the resource-relative path in `audio`, so a game can play that moment’s clip on entry.
- **Graph:** an optional overview with the same writing inspector. Drag cards to arrange, drag the background to pan, and use Focus to bring the selected step into view. You can author an entire story without opening Graph.
- **Characters:** create a cast member or build the cast from existing speakers. Add a description, numeric starting attributes, 2D portrait variants, and 3D model variants. IDs are stable; display names are editable. Each character can have images, models, both, or neither.
- **Project resources:** both images and models have searchable resource pickers. Pick an existing image/model without copying it, or import new artwork. Discovery includes nested resource folders and existing files not listed in the manifest. Refresh rescans after external changes. New imports are registered in the project asset manifest; save the project to persist that manifest.
- **Dependencies:** inspect incoming choices, requirement producers, starting attribute values, broken references, and structurally disconnected steps. A producer link is a place that changes the value, not proof that every path through it satisfies the condition.
- **Playthrough:** walk through the actual choices. Unavailable choices stay visible but disabled, with the unmet requirements explained. Inspect or change flags/attributes to simulate state provided by gameplay. Restart or step back; the preview never modifies the authored story.

Use **Save narrative** or **Cmd/Ctrl+S** in this workspace to save the story and shared library. Story saving is separate from scene/project saving. Build/Play also saves the current project’s narrative draft and stops if it has validation errors or external conflicts. Undo/Redo buttons restore authoring changes; text fields also have their own text undo. Switching stories or reloading asks how to handle dirty drafts. Changing projects keeps an unsaved narrative draft attached to its original project. External file edits block saving instead of being overwritten. Invalid references may be saved as a draft, explicitly labeled with validation errors; playthrough requires valid references.

## Character presentation

Images are project-resource-relative paths. Image import validates and copies the artwork to a unique PNG in `src/resources/portraits`. A variant can represent an expression, costume, full-body illustration, or other 2D presentation.

3D assets are glTF (`.gltf`) or binary glTF (`.glb`). Model import validates the model and copies its relative buffers and images into a unique directory under `src/resources/models`. External URL, parent-directory, or encoded dependency paths require exporting a self-contained GLB first. A model binding includes a positive uniform scale and optional animation clip name. These are presentation instructions consumed by gameplay.

Each story step can choose its image and model variants independently. An empty variant means the first variant in alphabetical order. The runtime resolves **both** representations; a game can render an image, instantiate a model, or do both. The editor's dialogue preview shows the 2D image and the resolved 3D binding, rather than rendering an interactive 3D viewport.

## Conditions and effects

All requirements on a choice must pass:

- Legacy `requires` checks one flag; legacy `grants` adds one flag.
- Additional conditions test whether flags are present/absent or compare a numeric attribute using `>=`, `<=`, `=`, or `!=`.
- Character attributes use `character_id.attribute` keys, for example `character_1.trust`. World attributes can use unqualified keys such as `coins`.
- Missing attributes do not satisfy comparisons. Initialize world values in game code or set them in the preview inspector.
- Effects grant/remove flags, set/add numeric values, or emit named events.
- `on_enter` emits events when entering a step, including the initial step.

Choice indices always refer to the full authored choice list, including unavailable choices. Never pass the position in a filtered list. A failed transition leaves flags, attributes, and the current step unchanged. The runtime returns choice events followed by destination entry events; game code decides how to handle them.

## Connect a game

Legacy story arrays remain compatible. The sibling `story-game` now uses the shared runtime, character library, attributes, consequences, journal, and presentation bindings. Other games using older flag-only runtimes must adopt the runtime below to use those features. The sample embeds JSON and referenced assets at compile time, so rebuild after saving.

Choose **Export Rust runtime** to write `src/narrative_runtime.rs` into the project. The export requires `serde` with derive support and `serde_json`, already present in `story-game`. It refuses to overwrite a different existing module. Alternatively, projects linking the editor can use `enigma_engine::narrative` directly.

```rust
mod narrative_runtime;
use narrative_runtime::{CharacterLibrary, PlayState, StoryNode};

let nodes: Vec<StoryNode> = serde_json::from_str(include_str!(
    "resources/story/chapter_01.de.json"
))?;
let characters: CharacterLibrary = serde_json::from_str(include_str!(
    "resources/story/characters.json"
))?;
let (mut state, entry_events) = PlayState::start(&nodes, &characters)?;
// Dispatch entry_events to your scene/audio/game systems.

let current = nodes.iter().find(|node| node.id == state.node).unwrap();
let visible_choices: Vec<_> = current.choices.iter().enumerate()
    .filter(|(_, choice)| state.unmet(choice).is_empty())
    .collect();
// Keep the original index when building your UI action payload.
let original_index = visible_choices[0].0;
let events = state.choose(&nodes, original_index)?;
// Dispatch events, then refresh the displayed dialogue and character.
```

Resolve presentation from the new current step:

```rust
let current = nodes.iter().find(|node| node.id == state.node).unwrap();
if let Some(presentation) = characters.presentation(current) {
    if let Some(image_path) = presentation.image {
        // Load from src/resources/image_path into your 2D UI texture system.
    }
    if let Some(model) = presentation.model {
        let path = resources_directory.join(&model.path);
        let mut object = enigma_3d::object::Object::load_from_gltf(
            path.to_str().unwrap(), None,
        );
        object.name = presentation.character.name.clone();
        object.transform.set_scale([model.scale; 3]);
        if !model.animation.is_empty() {
            object.play_animation(&model.animation, true);
        }
        // app_state.add_object(object), or update an existing character object.
    }
}
```

The engine's glTF loader handles scene objects as usual: the game owns placement, materials, animation lifetime, and removal. Avoid adding a new object every render frame. A text-only or 2D game can simply ignore the model binding.

A game's event dispatcher can match names such as `scene:castle`, `audio:bell`, or `quest:complete`. The narrative runtime deliberately returns strings instead of executing code or changing scenes itself.

## File compatibility

Stories remain arrays. Optional `character`, `portrait`, `model`, `audio`, `on_enter`, `conditions`, and `effects` fields extend each step/choice. Graph positions are stored in the optional `editor_position` metadata. Unknown fields on nodes, choices, characters, models, and the library survive round trips. Unsupported condition/effect fields or kinds cause a load error so they cannot be silently lost or interpreted incorrectly.

Characters live in a shared library object:

```json
{
  "characters": [{
    "id": "character_1",
    "name": "Bera",
    "description": "A healer with a guarded past.",
    "portraits": { "neutral": "textures/bera.png" },
    "models": {
      "default": { "path": "models/bera.glb", "scale": 1.0, "animation": "Idle" }
    },
    "attributes": { "trust": 2 }
  }]
}
```

New character names update matching legacy speaker labels in the currently open story while preserving per-step overrides. Other stories retain their saved speaker strings; games using character references resolve the current library name directly. Portrait/model variants in use by other discovered stories cannot be removed. Asset files themselves are not deleted by removing a variant or undoing an import.
