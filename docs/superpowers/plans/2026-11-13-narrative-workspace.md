# Narrative Workspace Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement task-by-task.

**Goal:** Author characters and branching narratives inside the engine.
**Architecture:** Shared serializable model/evaluator; guarded project filesystem operations; egui workspace using the same evaluator for preview.
**Tech Stack:** Rust, serde_json, egui 0.23, existing image/rfd crates.
**Spec:** docs/superpowers/specs/2026-11-13-narrative-workspace-design.md

## Global Constraints
Keep legacy story arrays compatible. Preserve unknown fields. Import artwork rather than generate it. No changes to sibling projects required. Runtime effects return named engine events.

## Review Focus
- Legacy unknown fields survive save.
- Failed/locked transitions never mutate state.
- Dirty drafts survive load failures and external conflicts.
- Portrait paths cannot escape project resources.
- Structural reachability is not presented as proof of conditional reachability.

### Task 1: Runtime and model
Files: src/narrative/mod.rs, src/narrative/tests.rs; src/lib.rs and src/main.rs module declarations.
Interfaces: StoryNode, Choice, Character, CharacterLibrary, Condition, Effect, PlayState; parse/serialize arrays; PlayState::start and choose return events.
- [x] Write regression tests for legacy JSON round trips, locked choice rejection, numeric attributes and events, validation.
- [x] Run targeted tests to establish missing behavior.
- [x] Implement model/evaluator and run targeted tests.

### Task 2: Project storage
Files: src/project/narrative.rs, src/project/mod.rs.
Interfaces: Document::load/create/save, discovery, import_portrait, export_runtime.
- [x] Write filesystem tests for conflicts, invalid input, safe portraits and save/reopen.
- [x] Run tests, implement persistence, rerun tests.

### Task 3: Workspace
Files: src/editor/narrative/{mod,inspector,graph,preview}.rs; state/mod/toolbar integration.
Interfaces: NarrativeEditor::default/draw/save/dirty; document snapshots for undo; library and story inspector; graph; dependency explorer; PlayState preview.
- [x] Implement UI with headless frame tests and edit-history tests.
- [x] Add workspace tab, dedicated save shortcut, status and project-switch handling.
- [x] Document runtime adoption and authoring workflow.

### Task 4: Verify and review
- [x] Run cargo test, cargo check, focused rustfmt check and clippy.
- [x] Launch editor and inspect affected workspace if graphical environment permits.
- [x] Review whole change, fix material issues and report evidence/limitations.

## Verification record
- `cargo test --offline`: 136 library tests and 136 binary tests passed; no failures.
- Isolated exported-runtime consumer: compiled without editor dependencies and played the existing story-game JSON successfully.
- `cargo check --offline`: passed.
- `cargo clippy --all-targets --offline`: passed with existing warnings; no diagnostics in the new narrative modules.
- Focused rustfmt check: passed for all new modules. Repository-wide fmt reports existing formatting drift in legacy files.
- Desktop launch with a temporary project: succeeded outside the sandbox (`Using GL 1.4`). Automated desktop inspection timed out, so interactive visual QA is not claimed. All four workspace modes have headless rendering coverage.
- Independent review: fixed per-step speaker override loss and nested metadata loss, each with a failing regression test then a passing suite. Invalid reference saving is explicitly labeled as draft saving; Build/Play and preview reject invalid data.
- Added resource catalog discovery, picker UI for both representations, manifest registration for imports, and Build/Play draft saving after follow-up requirements.

## Delivery
Implementation remains on `feat/narrative-workspace` as reviewable working-tree changes. The sibling story-game source is unchanged; runtime adoption is documented in `docs/narrative-editor.md`.

## Follow-up completed
- Writer-first view with optional graph and connected continuation/branch actions.
- Shared searchable dropdowns throughout the editor; pointer/focus/keyboard regressions.
- Explicit audio resources on steps; actual pointer-selection and save/reload regression.
- story-game runtime/data migration, audio framework adoption and safe legacy-save upgrade.
- Final installed-source verification: 145 tests per engine target and 48 game tests pass.
- Recent user portrait assignments preserved; sibling changes backed up before installation.
