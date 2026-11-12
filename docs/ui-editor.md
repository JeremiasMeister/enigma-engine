# Visual UI editor

Open a project and choose **UI** in the top toolbar. Use **+ Screen** to start with a blank screen, pause menu, or RPG dialogue.

## Author a screen

Add widgets from the palette. Selecting a panel or repeater makes new widgets its children. Select widgets on the canvas or in the hierarchy, drag to move, and drag the lower-right handle to resize. **Snap 8 px** snaps geometry; hold Alt while dragging to bypass it. **Fit** shows the whole screen; **100%** allows scrolling at reference size.

The inspector edits content, parent, anchor, offsets, size, appearance, bindings, and events. Anchors use the same point on the parent and widget: center anchor with zero offsets centers a widget. Reparenting starts the widget at the new parent's top-left with a 16-point inset. Reference coordinates scale uniformly to fit the game window, with unused space centered around the screen.

Select the screen root in the hierarchy to rename it or change its reference resolution. Up/Down change the stacking order among siblings. Duplicate copies a whole subtree with new identities. Delete removes a subtree. Cmd/Ctrl+Z and Cmd/Ctrl+Shift+Z undo/redo UI changes, including screen creation/deletion. Undo across screens opens the restored screen. Text fields retain their own normal text-editing undo.

**Save scene**, Cmd/Ctrl+S, and **File → Save Project** persist UI definitions in `enigma_project.json`. Existing projects without UI data still load normally. **Generate Rust** exports the current UI definitions; save the project as well to preserve the editable source.

## Textures, sprites and visual styles

Project textures are available directly under a widget's **Appearance → Texture / sprite → Texture**, and in its hover/pressed texture pickers and reusable styles. Choosing a project texture automatically registers it on that UI screen without copying the file. Existing screen aliases are reused; duplicate names receive a suffix.

Select the **screen root** in the hierarchy to find **Textures & sprites**. **Import artwork…** copies a PNG, JPEG, GIF, BMP or TGA (first frame only) into `src/resources/textures` and adds it to the project manifest and the screen. **Use project texture…** can register extra textures for procedural bindings. While the UI workspace is open, images already in `src/resources/textures` (including subfolders) are discovered in the background and added to the manifest without copying them. **Save Project** persists the manifest; discovery marks it unsaved. The displayed screen asset name is the key used by widgets and data bindings. **Reload** rescans the folder and refreshes edited source art. Removing a UI reference keeps the source file and project asset.

Add an **Image** widget for a portrait, icon, decoration or backdrop. Panels, buttons and other controls can also have textured backgrounds. Under **Appearance → Texture / sprite**, choose the asset, tint, and filtering. Enable **Pixelated filtering** for pixel art. Choose **Contain** to preserve the whole image, **Cover** to fill the bounds with cropping, or **Stretch** to fill the widget exactly.

**Sprite region** selects normalized UV coordinates, measured from the image's top-left. For a four-column sprite sheet, the second column spans Left `0.25`, Right `0.5`, Top `0`, Bottom `1`. **Nine-slice** preserves the specified left/top/right/bottom margins in source pixels while stretching the center and edges; use it for illustrated windows and button borders. Hover and pressed textures can override the normal background on interactive controls.

Set **Texture data key** to `portrait` or `icon`, then supply the asset name in `data.text`. A repeater reads each row's key first, so inventory rows can show different icons and dialogue speakers can change portraits without rebuilding the screen:

```rust
dialogue.data.set_text("portrait", "warden_portrait");
// On a row: UiItem::new("potion").with_text("icon", "potion_icon")
```

All possible artwork must be registered on the screen before exporting. The exporter validates and embeds decoded art in generated files; the game does not need an image decoder, source images, or a particular working directory. Images are limited to 4096 pixels per side, 64 textures and 128 MiB of decoded pixels per screen. Missing or invalid artwork is reported in the editor and blocks export.

Use **Screen theme → Apply a theme…** for editable **Modern**, **Parchment**, **Pixel**, and **Neon** starting points. This assigns shared styles to existing widgets and preserves layout, artwork, bindings and events; Undo restores the previous appearance. New widgets use the matching shared style when available.

Every widget can use a **Local style** or a named reusable style. **Save as reusable style** turns a local appearance into a shared style. Editing a shared style updates all widgets using it; switching back to Local style copies the current appearance. Styles include colors, proportional/monospace type, text alignment, padding, borders, rounded corners, image regions, tint, filtering and interactive states. Use your own artwork and editable styles to build a visual identity beyond the four presets.

## Preview and data

**Design** shows authored defaults and one template row per repeater. **Preview** runs the same renderer exported to your game, using a separate copy of the sample data. Its status line reports event names, widget names and item IDs. Changing an input in Preview does not change the saved defaults. **Reset data** starts the preview over.

The inspector's **Sample data / procedural content** section accepts JSON with four maps:

```json
{
  "text": {"speaker": "Warden Iona", "dialogue": "Will you help us?"},
  "numbers": {"health": 0.75},
  "flags": {"show_dialogue": true},
  "lists": {
    "choices": [
      {"id": "accept", "data": {"text": {"text": "I'll help."}, "flags": {"enabled": true}}},
      {"id": "leave", "data": {"text": {"text": "Maybe later."}, "flags": {"enabled": true}}}
    ]
  }
}
```

Press **Apply sample data** to validate and save these defaults. Missing maps default to empty. Bind a label or button's **Text key** to `speaker`, `dialogue`, or another text key; bind sliders/progress to numbers, toggles to flags, and visibility/enabled to flags. A text input reads and writes its text binding. Unbound inputs maintain their own transient values. Binding keys are literal names, not Rust expressions.

A **Repeater** reads its **List key** and renders its child widgets for every record. Its row height and gap control vertical layout; longer lists scroll. Within a row, bindings read the row's data first, then screen-level data. A changed input writes back to the scope it resolved. Each record needs a unique, nonempty stable `id`; reordering items preserves focus and event identity. Invalid or duplicate runtime IDs are skipped, including every copy of a duplicate. Authored sample data with duplicate IDs is rejected.

Events expose `name`, `widget`, `value` (`Click`, `Bool`, `Number`, or `Text`), `item_id` for the innermost row and `item_path` for nested rows. Buttons and inputs emit only when an action name is configured. Keep IDs stable as content changes; do not use row positions as IDs.

## Generate and register a class

**Generate Rust** writes `src/generated_ui/mod.rs`, `runtime.rs`, and a module for each screen. A screen named `Dialogue` exports `DialogueUi`. The **Code** view shows its generated class and a copyable registration example. The output embeds the validated layout and shares the editor renderer. It needs the project's existing `enigma-3d`, `serde` with derive, and `serde_json` dependencies.

Generated files carry an ownership header. Only those files may be replaced; conflicting handwritten files or symlink destinations cause an error before export. Renaming/deleting a screen updates the generated module index; old generated files can remain on disk but are no longer imported. Keep game logic outside this directory. Existing `main.rs` is never rewritten.

Add `mod generated_ui;` at the top of your game's `main.rs`, then install the screen before running the event loop. For example, the dialogue starter defines the `choose` action and three choices (`crossing`, `help`, `leave`):

```rust
mod generated_ui;

use enigma_3d::AppState;
use generated_ui::{runtime::UiItem, DialogueUi};

fn install_dialogue(app: &mut AppState) {
    DialogueUi::install(app, |event, app| {
        if event.name != "choose" {
            return;
        }
        let Some(dialogue) =
            app.get_state_data_value_mut::<DialogueUi>(DialogueUi::STATE_KEY)
        else {
            return;
        };

        match event.item_id.as_deref() {
            Some("crossing") => {
                dialogue.data.set_text("speaker", "Warden Iona");
                dialogue.data.set_text("dialogue", "The bridge needs repairs. Will you help?");
                dialogue.data.set_items("choices", vec![
                    UiItem::new("help").with_text("text", "Tell me what you need."),
                    UiItem::new("leave").with_text("text", "I should go."),
                ]);
            }
            Some("help") => {
                // Start a quest here, then supply its next dialogue node.
                dialogue.data.set_text("dialogue", "Bring timber from the old mill.");
                dialogue.data.set_items("choices", vec![
                    UiItem::new("leave").with_text("text", "I'll return soon."),
                ]);
            }
            Some("leave") => dialogue.visible = false,
            _ => {}
        }
    });
}
```

Call `install_dialogue(&mut app_state)` during your normal game startup. The registration helper stores the screen in AppState and installs its GUI callback once. The callback releases its mutable screen borrow before running your event handler, so the handler can safely mutate the UI or other game state. To reopen a conversation, retrieve `DialogueUi` using `STATE_KEY`, set `visible = true`, and replace its speaker, dialogue, and choices.

You can also instantiate `DialogueUi::default()` and call `draw(ctx)` yourself. Every generated class exposes `screen`, `data`, and `visible`. Populate `data` each frame from inventory, quest or combat systems with `set_text`, `set_number`, `set_bool` and `set_items`. For fully procedural structure, add or modify `UiNode` values in `screen.nodes`; use `screen.next_id()` and unique names, and call `screen.validate()` to check the result. Runtime data changes require no code generation or recompilation.

## Current limits

This version supports panels, labels, buttons, toggles, sliders, progress bars, text inputs, images, and vertical repeaters. Layouts have fixed authored sizes plus parent anchors and uniform screen scaling. Custom font import, animated sprites/timelines, rich text, grid layout and dialogue graph authoring are not included. Your game owns conversation traversal and branching; the editor builds and binds the conversation's presentation.
