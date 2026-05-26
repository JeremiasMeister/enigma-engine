mod editor;
mod project;
mod resources;

use std::sync::Arc;
use enigma_3d::AppState;
use enigma_3d::camera::Camera;
use enigma_3d::postprocessing::grid_overlay::GridOverlay;

use crate::editor::state::EditorRoot;

fn main() {
    let mut app_state = AppState::new();
    let event_loop = enigma_3d::EventLoop::new("Enigma 3d - Engine", 1080, 720);
    event_loop.set_icon_from_resource(resources::ICON);

    let camera = Camera::new(None, None, Some(60.0), Some(16.0/9.0), Some(0.1), Some(100.0));
    app_state.set_camera(camera);

    app_state.add_state_data("editor", Box::new(EditorRoot::empty()));
    app_state.inject_gui(Arc::new(editor::draw));
    app_state.inject_start_function(Arc::new(register_grid_overlay));

    event_loop.run(app_state.convert_to_arc_mutex());
}

/// Build the GridOverlay (needs the GL display, which only exists after the
/// event loop initialises) and share its enable handle with the editor's
/// toolbar so toggling the bool in `EditorRoot` updates the post-process.
fn register_grid_overlay(app_state: &mut AppState) {
    let Some(display) = app_state.display.as_ref().cloned() else { return; };
    let overlay = GridOverlay::new(&display);
    let handle = overlay.enabled_handle();
    app_state.add_post_process(Box::new(overlay));
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.gizmo.grid_overlay_enable = Some(handle);
    }
}
