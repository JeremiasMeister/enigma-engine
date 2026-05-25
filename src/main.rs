mod editor;
mod project;
mod resources;

use std::sync::Arc;
use enigma_3d::AppState;
use enigma_3d::camera::Camera;

use crate::editor::state::EditorRoot;

fn main() {
    let mut app_state = AppState::new();
    let event_loop = enigma_3d::EventLoop::new("Enigma 3d - Engine", 1080, 720);
    event_loop.set_icon_from_resource(resources::ICON);

    let camera = Camera::new(None, None, Some(60.0), Some(16.0/9.0), Some(0.1), Some(100.0));
    app_state.set_camera(camera);

    app_state.add_state_data("editor", Box::new(EditorRoot::empty()));
    app_state.inject_gui(Arc::new(editor::draw));

    event_loop.run(app_state.convert_to_arc_mutex());
}
