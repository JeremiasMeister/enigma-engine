# Post-processing implementation plan

**Goal:** Add, edit, enable, reorder, and remove scene post-processing, including project-written Rust effects using `PostProcessingEffect`.

**Architecture:** Add a serializable effect stack and a constructor/parameter registry to enigma-3d. Keep existing `add_post_process` callers compatible. Render scene effects before the editor grid. Expose the editor as a library so project code can register native effects in a compiled project editor, without crossing a dynamic Rust ABI.

**Constraints:** Existing Rust 2021, glium, egui, serde. Old scenes default to an empty stack. Unknown custom types preserve their settings and report an error. Project builds use the same registration module as the project editor. Do not rewrite user source files.

## Tasks

- [x] Add engine stack, parameter validation, registry and built-in adapters; test serialization, unknown effects, validation and reorder/remove behavior.
- [x] Wire stack serialization, scene replacement and rendering; test legacy scene compatibility and persistence through the editor save paths.
- [x] Add Scene Settings controls and a hierarchy shortcut; mark edits dirty and preserve changes made during a save.
- [x] Add project registration scaffold, compiled editor launcher and game registration documentation. Verify generated code with a real custom effect.
- [x] Run editor/engine tests, type checks, targeted formatting, runtime smoke test and independent code review. Visual UI interaction was unavailable because Computer Use was not approved.

## Review focus

- Project and scene switches must not leak previous effects or serialize the grid.
- Unknown or invalid custom effects must preserve authored data without crashing the editor.
- Parameter edits must update existing GPU effects instead of recompiling shaders every frame.
- Generated integration must preserve handwritten project files and use project dependencies.
- Save, reopen and game execution must restore effect order, enablement and values.

## Implementation record

- Baseline: `cargo test --bin enigma-engine` passes (105 tests).
- Custom format follows the existing Rust trait, as requested. Changed Rust source requires compilation; the project editor is statically linked with its registrations.
- Work is performed in the existing clean checkouts; changes remain uncommitted for review.

- Final editor suite: 110 passed. Editor build and new-file rustfmt checks pass.
- Renderer suite with desktop audio access: 152 passed; unchanged `terrain::tests::vertex_color_flat_low` fails at `terrain.rs:745`.
- Generated native project editor compiles with its example Rust effect.
- Review finding fixed: the parent editor locks during the child editor session, reloads disk state on return, and stays locked if reload fails. Regression test added.
- Play regression diagnosed in `story-game`: both effects were present in the staged scene, but the game used the older Git renderer. Play/Build now use a Cargo command override pointing to the editor's exact renderer source, without rewriting game manifests.
- A temporary copy of `story-game` compiled and passed a desktop runtime assertion: saved depth fog and vignette were restored and both GPU effects were constructed. Original game files were not changed.
