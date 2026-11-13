# Narrative workspace

Approved direction: an engine-native workspace for writing connected stories and building reusable characters with imported portraits. Implementation defaults: import existing artwork; numeric character/world attributes plus flags; retain existing JSON arrays and optional requires/grants fields. Image generation is outside this change.

## Approaches
A form-only editor hides branching; a graph-only editor makes prose cumbersome. Use a graph overview, searchable outline, writing/character inspector, dependencies, and a playable preview in the existing egui application.

## Data and integration
Stories remain JSON arrays under src/resources/story. Optional character, portrait, conditions, effects, and on_enter event fields extend the format. Shared characters live in characters.json in that folder. The first node is the entry; legacy opening is moved first only by explicit author action. Preserve unknown JSON fields. Portrait paths are relative to src/resources; imported images are copied into portraits with unique names.

Expose the rendering-independent model and evaluator as enigma_engine::narrative and export its portable source for projects that do not depend on the editor. Preview and runtime share evaluation. Choice events and node-entry events are returned to the caller for engine handling; no arbitrary code execution. Existing story-game continues reading legacy fields; richer semantics require adopting this runtime, with explicit integration documentation.

## Behavior
Create/load/save stories, add/duplicate/delete steps, edit text and destinations, bind characters and portrait variants. Character IDs stay stable; names/descriptions/attributes/images are editable. Show incoming links, flag producers, validation issues, and unmet requirements in preview. Support undo/redo, unsaved change confirmation when changing documents, conflict detection before saving, and errors without discarding drafts. Separate narrative save shortcut and indicator; project/scene saves must not claim story drafts are saved.

## Validation
Reject duplicate/empty IDs, broken destinations and character/portrait references. Warn about structurally unreachable steps and flags never granted; distinguish structural diagnostics from path-dependent availability. Tests cover legacy/extended round trips, conditions, effects, hooks, failed transitions, persistence conflicts, safe image paths, and editor rendering. Smoke-test the desktop when available.

## Accepted follow-up requirements
Characters support both 2D images and 3D models, with independent variants selected on story steps. Both representations must be selectable from existing project resources. The runtime resolves both so gameplay selects the rendering approach. Import glTF/GLB with sidecar dependencies; model bindings include uniform scale and an animation clip. New imports register in the asset manifest. The dialogue preview shows images plus 3D binding details; model rendering remains owned by gameplay.

## Implementation rulings
Invalid references may be saved as explicitly labeled drafts; playthrough rejects them. This preserves incomplete writing work. Unknown metadata is preserved on extensible records; unknown condition/effect fields are rejected at load to avoid silently changing semantics. Per-step speaker overrides survive character inspection and renaming. Existing sibling game Rust code is not modified; exported runtime adoption is documented in docs/narrative-editor.md.

## Implemented gameplay and authoring follow-up
The user requested richer sample gameplay, writing without graph navigation, searchable engine dropdowns, and explicit audio assignments on steps. Story is now a focused writing view; Graph is optional. New moments and branches create connected targets directly. Shared searchable pickers are used throughout editor value selectors and keep focus during input. StoryNode.audio is optional and resource-relative; the sample game embeds and plays it directly.

The sibling story-game now consumes the portable runtime and authored characters. It includes supplies, injuries, trust, additional conditional choices, journal events, music intensity events, asset-driven portraits and the guardian's knight model. Original 27 passage IDs, prose, speaker labels, graph positions and recordings are preserved. It has 40 choices; all are reachable through actual game progression. All 27 narration clips are explicitly assigned. Legacy save migration preserves progress and accounts for existing injuries and dragon-parley trust.

Final verification: 145 engine tests in each library/binary target; 48 installed story-game tests; both cargo check commands pass; engine cargo build and clippy pass with existing warnings. UI pointer tests cover dropdown search and step-audio selection; rendered UI tests cover writing/playthrough and gameplay. No full live desktop interaction test is claimed. Replaced sibling files are backed up under story-game/.enigma/backups/narrative-migration-20261113-114852.
