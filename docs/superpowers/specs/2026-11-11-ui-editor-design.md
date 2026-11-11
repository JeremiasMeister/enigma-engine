# Visual UI editor and procedural screens

Approved direction: the interactive UI workspace mockup, followed by “looks good build it”. The user also requested procedural/data-driven UI, RPG conversations, textures/sprites, and support for substantially different visual styles.

## Outcome

Author game screens visually inside Enigma, save them with the project, preview interactions, and export Rust UI classes usable through the existing `AppState::inject_gui` hook. Supply runtime data without regenerating layouts. Include a dialogue starter whose speaker, dialogue and choice list can change from game code, with stable choice IDs in events.

## Scope and decisions

- Editor-only implementation in `enigma-engine`; do not change `enigma-3d` (the engine remains the owner's learning project).
- Rust 2021 and existing egui 0.23/serde dependencies. Share the authored model and renderer between editor and exported projects to avoid preview/runtime drift.
- Scene/UI workspace switch; UI assets and hierarchy, widget palette, canvas with selection/drag/resize/snap, properties, Design/Preview/Code modes. UI operations must not trigger scene gizmos or scene deletion.
- Initial widgets: panels, labels, buttons, toggles, sliders, progress bars, text inputs and repeating lists. Panels/repeaters own child widgets. Anchors position fixed-size widgets within their parent. The canvas fits its reference resolution; runtime uses the same scale and anchor math.
- Store screen definitions and preview sample data in a backward-compatible `ui_screens` project field. Use the existing project save/load workflow; keep UI undo history in transient editor state, including screen creation/deletion; undo/redo activates the restored screen so cross-screen changes remain visible. Saving must not clear newer unsaved edits.
- Bind text/value/visible/enabled properties by key. Repeaters read a list of records with stable string IDs and render a child template per row. Row fields override root bindings; runtime events carry the widget name and the item identity/path. Preview edits use cloned data and never overwrite authored defaults.
- Expose `UiData` setters and records for procedural content. Exported screen classes have public `screen`, `data`, and `visible` fields: games can populate bindings, lists, or add/modify nodes programmatically. Dialogue traversal, quests, branching and game state remain handwritten game logic.
- Export generated Rust modules under `src/generated_ui`. Use reserved generated files with ownership markers, refuse to overwrite unmanaged files, validate before writing, and surface errors in the editor. Handwritten callbacks and existing main.rs are never overwritten. Show a copyable registration example for existing projects.
- Include pause-menu and dialogue starters. Export uses the same schema/renderer and generates one Rust wrapper class per screen, plus runtime support. Generated wrappers load a validated embedded JSON layout; no external runtime layout files or engine changes are required.
- Support image widgets, textured widget backgrounds, normalized sprite-sheet UV regions, contain/cover/stretch/nine-slice fitting, pixelated filtering, tint and hover/pressed art. Bind the texture key from runtime data for portraits and item icons. Imported art lives in project resources; exported RGBA assets are embedded and require no working-directory assumptions or new game dependencies.
- Offer per-widget styles and named reusable screen styles: colors, border, corner radius, text alignment, proportional/monospace type, and padding. Provide Modern, Parchment, Pixel, and Neon theme starting points; all remain editable. Themes are separate from bindings and game logic.
- Decode images in the editor using the already cached image crate; bound decoded size and reject invalid/out-of-project paths. Save asset references rather than image bytes in UI documents/history. Missing art yields an actionable preview/export error.
- Defer custom font import, animation timelines, rich text, grid layout and a dialogue graph editor. This version proves visual authoring, custom art/styles and data-driven runtime integration.

## Acceptance

Create/edit/save/reopen a screen; drag and resize widgets; undo/redo; preview without changing defaults; export and compile a generated project. Populate a dialogue with zero, one and several choices, update text at runtime, and receive the right choice ID after reorder. Malformed names, dimensions, parent references and duplicate row IDs must not produce broken generated code or stale/wrong events. Older project files load with no UI screens.
