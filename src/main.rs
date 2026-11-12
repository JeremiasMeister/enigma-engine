mod app;
mod editor;
mod project;
mod resources;
mod ui_runtime;

fn main() {
    let project = std::env::args().nth(1);
    app::run(project.as_deref(), None);
}
