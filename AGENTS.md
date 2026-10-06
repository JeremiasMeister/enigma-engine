# Repository Guidelines

## Project Structure & Module Organization

This Rust 2021 binary is a desktop game editor built on `enigma-3d` and `egui`.

- `src/main.rs`: application startup, camera, and rendering hooks.
- `src/editor/`: editor state and actions; `panels/` contains the UI, `inspector/` edits components, and `gizmo/` implements transform tools and math.
- `src/project/`: project creation/loading, scene persistence, resource imports, and materials.
- `src/resources/`: embedded icon and `.resource` templates for generated projects.
- `docs/superpowers/specs/` and `docs/superpowers/plans/`: feature designs and implementation plans.

Tests live alongside source code in `#[cfg(test)] mod tests` blocks.

## Setup & Development Commands

Install Rust and Cargo. `Cargo.toml` patches `enigma-3d` to `/Users/cg-jm/jm/enigma-3d`; point this override at your compatible local checkout before building. Keep machine-specific path changes out of unrelated contributions.

- `cargo check`: type-check without building the executable.
- `cargo build`: compile the editor; add `--release` for an optimized build.
- `cargo run`: launch the desktop editor; requires a graphical environment with OpenGL support.
- `cargo test --bin enigma-engine`: run the binary's unit tests.
- `cargo test --bin enigma-engine gizmo::math`: run only gizmo math tests.
- `cargo fmt -- --check` and `cargo clippy --all-targets`: inspect formatting and lint diagnostics.

## Coding Style & Naming Conventions

Use four-space indentation, `snake_case` for files/functions/variables, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Follow surrounding Rust conventions and use rustfmt for formatting; no custom formatter or Clippy configuration is present. Keep formatting changes focused. Put UI logic in editor modules and persistence logic in project modules. Document non-obvious coordinate conventions and serialization behavior.

## Testing Guidelines

Use Rust's built-in `#[test]` framework and `tempfile` for filesystem fixtures. Name tests after behavior, such as `new_scene_sanitizes_name`. Cover serialization round trips, resource/scene operations, and mathematical edge cases. No numeric coverage threshold is configured. For UI changes, also smoke-test affected panels, selection, gizmos, and save/reopen behavior through `cargo run`.

## Commit & Pull Request Guidelines

Recent history favors scoped Conventional Commit messages: `feat(editor/gizmo): ...`, `fix(editor): ...`, and `refactor(editor): ...`. Use concise, imperative summaries and focused commits.

PRs should explain the change, link relevant issues or design documents, record checks performed, and include screenshots or short recordings for visible editor changes. Exclude generated `target/` output and `enigma_logs/`.
