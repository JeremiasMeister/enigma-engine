# Project Code Workspace Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans. Steps use checkbox syntax.

**Goal:** Open the Rust project externally and preview/edit project files within Enigma.

**Architecture:** Project file operations own confinement, text encoding, conflict detection, atomic saves and draft recovery. A transient Code workspace owns tree/tabs/editing/preview and safe external launcher state; existing toolbar and build actions route to it.

**Tech Stack:** Rust 2021, egui 0.23, std, serde, existing image decoder and tempfile tests.

**Spec:** ../specs/2026-11-11-code-workspace-design.md

## Global Constraints

- Preserve current uncommitted UI work and AGENTS.md. No engine source changes, new dependencies, commit or push.
- Save and build/play actions write source files; dirty draft persistence is separate.
- All file writes stay inside the owning project and reject links/escapes; preserve external edits and encoding.
- Code shortcuts never trigger scene operations. Tabs retain their project ownership across switches.

## Review Focus

- External edits/deleted files/permission failures cannot silently destroy either version.
- Dirty tabs survive workspace/project changes and engine-owned close behavior.
- Generated/engine-managed files expose the right overwrite/editing restrictions.
- File scans, decoding, syntax coloring and external launches stay bounded/responsive.
- Paths with spaces/unicode/metacharacters are single process arguments.

### Task 1: Project file and launch operations

Files: new src/project/code_files.rs, src/editor/code_editor/external.rs; module registration.

- [x] Test bounded listing, confinement, UTF-8/BOM/CRLF, conflict-safe saves, permissions, draft recovery and launch arguments; observe failures.
- [x] Implement safe operations, then run focused tests green.

### Task 2: Code workspace and integration

Files: new src/editor/code_editor/{mod.rs,highlight.rs}; editor state/draw/toolbar, actions integration.

- [x] Add searchable tree, tabs, text editor/line numbers, image/binary previews and save/reload/close flows.
- [x] Add external project/file opening and folder reveal with visible results/errors.
- [x] Route shortcuts and build saves; retain project-owned documents and recover drafts.
- [x] Cover tab ownership, editing identity and save-before-build/conflict behavior with tests.

### Task 3: Verify and document

- [x] Run tests, check/build/Clippy, focused formatting and whitespace checks.
- [x] Native launch/layout inspection and external-launch argument verification; exercise typing/save/reopen through real egui input and filesystem tests.
- [x] Independent code review, fix substantive issues and rerun covering checks.
- [x] Update README/roadmap and document usage, recovery and limits.

## Verification results

- `cargo test --offline --bin enigma-engine`: 100 passed. Includes source-widget pointer/keyboard input, Unicode, save/reopen, draft recovery, project-owned tabs, conflict stopping, failed reload, and read-only recovery cases.
- `cargo check --offline`, `cargo build --offline`, and `cargo clippy --offline --all-targets` completed. Existing repository/dependency warnings remain; no diagnostics in the new code modules.
- New code modules pass rustfmt; the diff passes whitespace checks. Existing integration files retain surrounding formatting.
- Native macOS window rendered the Code tree, tabs, source coloring, line numbers and actions. The observed native status reported the project opened in RustRover. The user was using that window, so no further automated changes were made there. CUA also experienced timeouts; native typing/save/reopen were covered with actual egui input tests instead of an uninterrupted desktop automation sequence. Windows/Linux launches have argument-level tests, not native smoke coverage.
- Independent review found three recovery defects; each was reproduced with a failing test and fixed: read-only recovery save must keep the draft/tab; backup errors must remain visible above the scroll area; valid drafts must recover even if disk content becomes binary/oversized. Also bounded input by UTF-8/CRLF bytes and prepared reloads before removing recovery data.
- Added `docs/code-workspace.md` and README/roadmap usage notes. No engine changes, dependencies, commit or push.
