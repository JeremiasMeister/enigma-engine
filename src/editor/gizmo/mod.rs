pub mod math;
pub mod toolbar;

use egui::{Context, Rect, Ui};
use enigma_3d::AppState;
use nalgebra::Vector3;

use crate::editor::state::{EditorRoot, Selection};

/// Mode-switch hotkeys, hit-tests handles, begins/ends/updates drag.
/// Called from viewport::draw after camera input.
pub fn handle_input(_ctx: &Context, _rect: Rect, _app_state: &mut AppState) {
    // Implemented in later tasks.
}

/// Paints gizmo handles using the viewport rect's painter.
/// Called from viewport::draw after the click-to-select branch.
pub fn draw(ui: &mut Ui, rect: Rect, app_state: &mut AppState) {
    toolbar::draw(ui.ctx(), rect, app_state);
}

/// Resolve the current selection to a draggable target's world pivot.
pub(crate) fn selection_pivot(app_state: &AppState) -> Option<Vector3<f32>> {
    let root = app_state.get_state_data_value::<EditorRoot>("editor")?;
    match &root.editor.selection {
        Selection::SceneObject(uuid) => app_state
            .objects
            .iter()
            .find(|o| o.get_unique_id() == *uuid)
            .map(|o| o.transform.position),
        Selection::Light(idx) => app_state
            .light
            .get(*idx)
            .map(|l| Vector3::from(l.position)),
        Selection::ParticleInstance(uuid) => root
            .project
            .as_ref()
            .and_then(|p| p.scenes.get(p.active_scene_index))
            .and_then(|s| s.particle_instances.iter().find(|i| i.uuid == *uuid))
            .map(|i| Vector3::from(i.position)),
        _ => None,
    }
}
