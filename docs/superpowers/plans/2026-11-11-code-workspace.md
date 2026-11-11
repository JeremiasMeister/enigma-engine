# Project Code Workspace Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans. Steps use checkbox syntax.

**Goal:** Open the Rust project externally and preview/edit project files within Enigma.

**Architecture:** Project file operations own confinement, text encoding, conflict detection, atomic saves and draft recovery. A transient Code workspace owns tree/tabs/editing/preview and safe external launcher state; existing toolbar and build actions route to it.

**Tech Stack:** Rust 2021, egui 0.23, std, serde, existing image decoder and tempfile tests.

**Spec:** ../specs/2026-11-11-code-workspace-design.md

## Global Constraints

- Preserve current uncommitted UI work and AGENTS.md. No engine source changes, new dependencies, commit or push.
- Only explicit Save changes source files; dirty draft persistence is separate.
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

- [ ] Test bounded listing, confinement, UTF-8/BOM/CRLF, conflict-safe saves, permissions, draft recovery and launch arguments; observe failures.
- [ ] Implement safe operations, then run focused tests green.

### Task 2: Code workspace and integration

Files: new src/editor/code_editor/{mod.rs,highlight.rs}; editor state/draw/toolbar, actions integration.

- [ ] Add searchable tree, tabs, text editor/line numbers, image/binary previews and save/reload/close flows.
- [ ] Add external project/file opening and folder reveal with visible results/errors.
- [ ] Route shortcuts and build saves; retain project-owned documents and recover drafts.
- [ ] Cover tab ownership, editing identity and save-before-build/conflict behavior with tests.

### Task 3: Verify and document

- [ ] Run tests, check/build/Clippy, focused formatting and whitespace checks.
- [ ] Native smoke preview/edit/save/reopen and inspect external-launch plans without installing software.
- [ ] Independent code review, fix substantive issues and rerun covering checks.
- [ ] Update README/roadmap and document usage, recovery and limits.
