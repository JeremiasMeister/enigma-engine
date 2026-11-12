//! Embed the editor in a project executable to use native project effects.
mod app;
mod editor;
mod project;
mod resources;
mod ui_runtime;

pub use app::run;
pub use enigma_3d;
pub use project::post_processing::prepare_editor as prepare_project_editor;
