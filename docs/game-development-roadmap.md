# Enigma Game Development Roadmap

Updated: 2026-10-06. Status: first editor usability pass complete; gameplay foundations remain planned.

Enigma should support comfortable, sustained development of complete games: create gameplay, configure it visually, reuse it across scenes, test it quickly, diagnose problems, and ship reliable builds. Unity and Unreal are references for the authoring experience. The first complete game will determine which capabilities need production depth first.

Work will happen incrementally in the owner's free time, without a fixed schedule. Each session should leave a small, understandable improvement and a clear next step.

## Working Agreement

- **Editor work in `enigma-engine`:** the assistant may implement changes directly within the requested task.
- **Engine work in `enigma-3d`:** this is a learning project. The owner types the code manually. The assistant explains the problem, design, and tradeoffs; presents small implementation steps; and helps review and test the owner's changes.
- Do not directly implement or automatically rewrite engine code, including engine test code, on the owner's behalf. Read-only inspection is welcome.
- Before a task, identify which repository owns each change. For engine lessons, explain one concept at a time, give an observable completion check, and continue from the owner's actual code.
- Keep this roadmap and the session checkpoint current as decisions and work progress. Preserve the existing root `AGENTS.md` unchanged.

## Starting Point

The editor already provides scene authoring, materials, animation controls, particles, terrain, and transform gizmos. The runtime has collision queries, update hooks, and typed component storage. Important gaps in the 2026-10-06 source snapshot are:

- Runtime `Object` components use `HashMap<TypeId, Box<dyn Any>>`. `ObjectSerializer` omits them, and object cloning clears them. Custom gameplay data needs persistence and duplication semantics before it can support editor authoring and prefabs. See `enigma-3d/src/object.rs`.
- The [inspector](../src/editor/panels/inspector.rs) exposes fixed built-in types; game-defined components do not automatically get editable properties.
- [Play](../src/editor/actions.rs) saves and launches `cargo run`. This path needs managed stop, pause, stepping, and runtime inspection.
- [Transform fields](../src/editor/inspector/transform.rs) originally mutated objects without marking the editor dirty. Fixed during the first editor usability pass; other editing paths still need a dirty-tracking audit.
- [Scene switching](../src/project/scene.rs) ignores a failed save before continuing. Scene and project saves also need stronger recovery guarantees.

Recheck these findings when resuming; they describe a starting snapshot.

## Development Priorities

| Priority | Developer outcome | Main work and ownership |
| --- | --- | --- |
| 1. Trustworthy editing | Undo mistakes, recover work, save safely, understand errors | Editor: undo/redo, consistent dirty tracking, recoverable saves, autosave, persistent error console |
| 2. Custom gameplay authoring | Attach custom components, edit properties, save and run them | Engine lessons plus editor integration: registration, serialization, property metadata, lifecycle |
| 3. Fast iteration | Play, stop, pause, inspect, adjust, repeat | Both: managed game process, diagnostics, separation of authored and running state |
| 4. Reusable content | Maintain reusable objects and variations across scenes | Both: object hierarchy, prefabs, instance overrides, reusable data assets, stable references |
| 5. Complete gameplay workflows | Build movement, interactions, menus, progression, and transitions | Engine and game modules: input actions, appropriate physics, animation control, UI, audio, save/load |
| 6. Production delivery | Collaborate, profile, upgrade projects, distribute builds | Both: asset reimport, versioned formats, migrations, build profiles, packaging, performance tools |

Advance through playable milestones. Bring basic packaging forward so the game is regularly tested outside the editor. Choose the depth of physics, navigation, animation, and other systems from the first game's needs.

## Gameplay Extensibility

The proposed authoring model has three concepts:

- **Components** store per-object data, such as `Health`, `Inventory`, `Interactable`, or `EnemySettings`.
- **Behaviors or systems** execute movement, damage, interaction, AI, and other gameplay logic.
- **Subsystems** provide services with explicit lifetimes, such as a session-wide quest service or a scene-local encounter manager.

Build on the existing component storage. An authorable type needs a stable identity and data version, editable property metadata with defaults and validation, serialization rules, and explicit duplication behavior. Distinguish authored settings from transient runtime state. Use stable persisted identifiers rather than Rust `TypeId` values in project files.

Game projects must be able to supply their own types. An initial Rust approach could export a component schema from the compiled game for the editor to read. The editor can then present properties without a custom inspector implementation for every gameplay type. This remains a design proposal to work through in the engine lessons.

**Success criterion:** adding a `Health` component to a game requires no changes to engine or editor source.

## Everyday Editor Experience

Prioritize frequent operations and predictable behavior:

- Search objects and assets; select multiple objects and edit common properties.
- Duplicate, rename, parent, drag, and delete consistently.
- Reset properties and copy/paste components.
- Show invalid references beside the affected field.
- Retain logs after the game exits and navigate from errors to relevant code.
- Preserve asset references across moves and reimports.
- Create reusable prefabs with visible instance overrides.
- Keep authored scenes intact when entering and leaving Play.

For undo, a continuous transform drag should become one meaningful undo action. A failed save must leave the current scene available and show a recoverable error.

## Gameplay Language Direction

The initial recommendation is Rust gameplay modules with editable properties and a managed game process. This fits the existing project and provides a direct learning path; compilation time is an iteration cost to measure.

Embedded scripting could improve iteration but adds bindings, debugging, and reload semantics. Visual scripting adds a substantial authoring and debugging system. Revisit these choices when a concrete gameplay workflow demonstrates the need. Treat data reload and native code reload as separate problems.

## First Playable Milestone

Use a collectible interaction to prove the connection between engine, editor, and game code:

1. Define custom collectible data in game code.
2. Attach it to an object and configure its value in the inspector.
3. Run the game and collect the object.
4. Update an inventory service that lives for the game session.
5. Stop, save, reopen, and verify the authored settings survive.
6. Extend the object into a reusable prefab and validate a packaged build.

Alongside this, improve editor reliability through undoable transform edits, correct dirty tracking, safe scene switching, and visible save errors.

Track practical outcomes: time from an edit to testing it, steps to create a behavior, save/reopen correctness, and how often game development requires changes to the engine itself.

## Session Checkpoint

- **Completed:** initial source assessment, development priorities, and the working agreement above. Engine source remains unchanged.
- **Editor usability pass:** cohesive dark styling, clearer panel headings and selection context, viewport controls, status bar, and Cmd/Ctrl+S. Added sliders with precise typed entry, colored XYZ fields, and camera FOV displayed in degrees while preserving runtime radians. Transform, camera, and light edits mark the scene dirty.
- **Search:** inspector dropdowns and scene/model/particle menus support filtering. Asset and hierarchy search preserve underlying identities. Text entry suppresses scene-delete and viewport shortcuts.
- **Layout follow-up:** fixed repeated panel growth caused by the search row underestimating its clear-button width. The row now measures the button before allocating the remaining width. Added an idle-frame regression test; a separate synthetic full-editor diagnostic confirmed stable sidebar widths across scene, camera, light, material, particle, and terrain inspectors at standard and Retina scaling.
- **Checks:** all 39 tests passed, including eight widget tests for precision, units, narrow layouts, filtering, popup interaction, and layout stability. `cargo build --offline` passed after the layout fix. The initial usability pass also completed `cargo check --offline`, Clippy, native launch, and a populated-editor visual check; existing warnings remain. Save/reopen workflows still need a dedicated reliability pass.
- **Next suggested editor task:** implement undoable transform edits, audit dirty tracking, and handle failed saves before scene switches. Then add recoverable saves and autosave.
- **Next suggested engine lesson:** trace the current component store and serializer, then design persistence for one simple component before extending it to registration and inspector metadata.
- **Open decision:** choose the first complete game, or at least its genre and representative gameplay loop.
- **Provisional scope:** desktop single-player development. Target platforms, team needs, and gameplay-language preferences remain to be confirmed.

At the end of each working session, record what changed, checks performed, unresolved issues, and the next small task here. Resume by reading this checkpoint and checking the current source and Git status.

## Reference Workflows

- [Unity prefabs](https://docs.unity3d.com/6000.0/Documentation/Manual/Prefabs.html): reusable object configurations with components, properties, and children.
- [Unreal subsystems](https://dev.epicgames.com/documentation/en-us/unreal-engine/programming-subsystems-in-unreal-engine): services with managed lifetimes and extension points.
