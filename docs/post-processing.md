# Scene post-processing

Select **Scene Settings** in the hierarchy. Under **Post Processing**, add Bloom, Depth Fog, Edge Outline, Grayscale, Vignette, or Lens Dirt. Effects execute from top to bottom. Use the checkbox to bypass an effect, the arrows to reorder it, and × to remove it. Parameters update the viewport immediately; Reset parameters restores that effect's defaults. Lens Dirt can use an imported project texture.

The ordered stack, enabled flags, parameters, and selected image bytes are saved in each scene JSON. Older scenes load with no effects. The editor grid is transient and renders after the scene effects. Save the scene before opening it outside the editor.

## Native project effects

Custom effects use the same `enigma_3d::postprocessing::PostProcessingEffect` trait as the built-ins. They can own shader programs and textures, inspect `AppState`, read depth, and use the supplied work buffers for multiple passes. Their rendering code is ordinary Rust.

Click **Create example effect** to add `src/postprocessing/mod.rs` and `src/postprocessing/tint.glsl`. This preserves existing source. New projects already include these files and the game registration hook. Edit them in the Code workspace or another editor.

The module exports:

```rust
pub fn register(
    registry: &mut enigma_3d::postprocessing::stack::EffectRegistry,
) -> Result<(), String>
```

Register each native effect with a stable ID, display name, parameter definitions, constructor, and update function. For example, adapting the engine's Bloom type:

```rust
use enigma_3d::postprocessing::{bloom::Bloom, stack::{EffectRegistry, Parameter}};

pub fn register(registry: &mut EffectRegistry) -> Result<(), String> {
    registry.register(
        "my_game.soft_bloom",
        "Soft Bloom",
        vec![
            Parameter::float("threshold", "Threshold", 0.9, 0.0, 10.0),
            Parameter::int("radius", "Blur radius", 15, 1, 64),
        ],
        |display, values| Ok(Bloom::new(
            display, values.float("threshold")?, values.int("radius")?,
        )),
        |effect, values| {
            effect.threshold = values.float("threshold")?;
            effect.iterations = values.int("radius")?;
            Ok(())
        },
    )
}
```

The constructor runs when an instance is first needed. The update function changes the existing instance when its settings change, avoiding shader compilation while dragging values. Image changes recreate the instance. Return `Err` for invalid settings or shader compilation failures; the editor displays the error and skips that effect. Validate interdependent values before mutating the instance. Use **Retry** after correcting an external resource.

`ParameterValue` supports floats, integers, booleans, two-component vectors, RGB colors, text, and embedded images. `Parameter::new` defines other parameter kinds; `range` optionally restricts numeric values. Parameter IDs must remain stable for existing saved scenes. Unrecognized parameters are retained. If a new required parameter is absent, the editor reports it and offers Reset parameters.

## Loading changed Rust code

Click **Load project effects**. This saves scene, project, and open source edits, then compiles and opens a project-specific editor. The original editor stays read-only while that window runs, then reloads the saved project when it closes. The project editor offers the registered effects in **Add effect**, with the same live controls as built-ins. After changing Rust or an `include_str!` shader, compile again using the original editor after closing the project editor, or run the generated package directly:

```sh
cargo run --manifest-path .enigma/editor/Cargo.toml
```

The generated package links the editor and the project's registration module into one executable. It copies explicit project dependencies and features, resolves relative dependency paths, and uses the editor's enigma-3d and glium versions. Registration modules should be self-contained relative to the project crate root: use submodules and dependency crates rather than imports from the game's `main.rs`. Workspace-inherited dependencies must be made explicit. The editor source checkout must remain available. Generated `.enigma/editor/` build files can be ignored in version control.

Opening another project drops the previous project's custom registrations. Load the new project's effects to obtain its own registrations. Unavailable custom effects remain in the stack with their settings, so opening a scene in the standard editor does not discard them.

## Game startup

New projects register their effects before loading the scene. For existing games, add the same hook to the game's startup:

```rust
mod postprocessing;

// After constructing AppState, before loading the scene:
postprocessing::register(&mut app_state.post_processing_registry)?;
```

Use `expect("Register project effects")` if `main` does not return a `Result`. Play and Build invoked through the editor automatically use the same enigma-3d source as the preview, without changing the project manifest. For a standalone `cargo run` or CI build, configure an enigma-3d version containing `postprocessing::stack`. The engine's `inject_serializer` restores the stack and constructs the registered effects on the render thread. Existing `add_post_process` integrations continue to work and render after the serialized scene stack.
