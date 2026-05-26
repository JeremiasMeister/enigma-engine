pub mod math;
pub mod toolbar;
pub mod translate;

use egui::{Context, Pos2, Rect, Ui};
use enigma_3d::AppState;
use nalgebra::Vector3;

use crate::editor::state::{Drag, EditorRoot, GizmoMode, Selection, Space};

pub fn handle_input(ctx: &Context, rect: Rect, app_state: &mut AppState) {
    // Reset the per-frame consumed flag at the start of each frame.
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.gizmo.consumed_click_this_frame = false;
    }

    let rmb = ctx.input(|i| i.pointer.secondary_down());
    if rmb { return; } // RMB-fly suppresses gizmo input.

    let Some(cursor) = ctx.input(|i| i.pointer.interact_pos()) else { return; };
    if !rect.contains(cursor) { return; }

    let Some(pivot) = selection_pivot(app_state) else { return; };
    let Some(camera) = app_state.camera.as_ref() else { return; };
    let camera = camera.clone();

    let (mode, space, snap_enabled, drag_some) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        (
            root.editor.gizmo.mode,
            root.editor.gizmo.space,
            root.editor.gizmo.snap_enabled,
            root.editor.gizmo.drag.is_some(),
        )
    };

    // Effective snap: toolbar XOR Ctrl-held.
    let ctrl = ctx.input(|i| i.modifiers.ctrl);
    let snap = snap_enabled ^ ctrl;

    let rotation = selection_rotation(app_state);

    // Drag in progress: update and possibly end.
    if drag_some {
        let released = ctx.input(|i| i.pointer.primary_released());
        update_active_drag(app_state, cursor, space, rotation, snap, &camera, rect);
        if released {
            end_drag(app_state);
        }
        return;
    }

    // No drag: hit-test and possibly begin.
    let camera_pos = Vector3::from(camera.get_position());
    let size = translate::handle_world_size(camera_pos, pivot, camera.fov);

    let hovered = match mode {
        GizmoMode::Translate => translate::hit_test(cursor, pivot, size, space, rotation, &camera, rect)
            .map(crate::editor::state::Handle::Axis),
        _ => None,
    };

    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.gizmo.hovered_handle = hovered;
    }

    let pressed = ctx.input(|i| i.pointer.primary_pressed());
    if pressed {
        if let Some(crate::editor::state::Handle::Axis(axis)) = hovered {
            let drag = translate::begin_drag(axis, cursor, pivot, space, rotation, &camera, rect);
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                root.editor.gizmo.drag = Some(drag);
                root.editor.gizmo.consumed_click_this_frame = true;
            }
        }
    }
}

pub fn draw(ui: &mut Ui, rect: Rect, app_state: &mut AppState) {
    if let Some(pivot) = selection_pivot(app_state) {
        if let Some(camera) = app_state.camera.as_ref() {
            let camera = camera.clone();
            let (mode, space, hovered_axis, dragging_axis) = {
                let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else {
                    toolbar::draw(ui.ctx(), rect, app_state);
                    return;
                };
                let drag_axis = match &root.editor.gizmo.drag {
                    Some(Drag::Translate { axis, .. }) => Some(*axis),
                    _ => None,
                };
                let hover_axis = match root.editor.gizmo.hovered_handle {
                    Some(crate::editor::state::Handle::Axis(a)) => Some(a),
                    _ => None,
                };
                (
                    root.editor.gizmo.mode,
                    root.editor.gizmo.space,
                    hover_axis,
                    drag_axis,
                )
            };

            let rotation = selection_rotation(app_state);
            let camera_pos = Vector3::from(camera.get_position());
            let size = translate::handle_world_size(camera_pos, pivot, camera.fov);

            // PositionOnly targets always show the translate gizmo regardless of mode
            // (so long as a gizmo mode is active).
            let target_full = matches!(
                app_state.get_state_data_value::<EditorRoot>("editor")
                    .map(|r| &r.editor.selection),
                Some(Selection::SceneObject(_))
            );
            let show_translate = matches!(mode, GizmoMode::Translate)
                || (!target_full && !matches!(mode, GizmoMode::None));

            if show_translate {
                translate::draw(ui, rect, pivot, size, space, rotation, &camera,
                    hovered_axis, dragging_axis);
            }
        }
    }
    toolbar::draw(ui.ctx(), rect, app_state);
}

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

pub(crate) fn selection_rotation(app_state: &AppState) -> Vector3<f32> {
    let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else {
        return Vector3::zeros();
    };
    if let Selection::SceneObject(uuid) = &root.editor.selection {
        if let Some(o) = app_state.objects.iter().find(|o| o.get_unique_id() == *uuid) {
            return o.transform.rotation;
        }
    }
    Vector3::zeros()
}

fn update_active_drag(
    app_state: &mut AppState,
    cursor: Pos2,
    space: Space,
    rotation: Vector3<f32>,
    snap: bool,
    camera: &enigma_3d::camera::Camera,
    rect: Rect,
) {
    let drag_snapshot = app_state.get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.editor.gizmo.drag.as_ref().map(|d| match d {
            Drag::Translate { axis, start_pos, start_on_axis } =>
                Some((*axis, *start_pos, *start_on_axis)),
            _ => None,
        }))
        .flatten();
    if let Some((axis, start_pos, start_on_axis)) = drag_snapshot {
        let new_pos = translate::update_drag(
            axis, start_pos, start_on_axis, cursor, space, rotation, snap, camera, rect,
        );
        apply_position(app_state, new_pos);
    }
}

fn apply_position(app_state: &mut AppState, new_pos: Vector3<f32>) {
    let selection = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.selection.clone());
    let Some(selection) = selection else { return; };
    match selection {
        Selection::SceneObject(uuid) => {
            if let Some(o) = app_state.objects.iter_mut().find(|o| o.get_unique_id() == uuid) {
                o.transform.position = new_pos;
            }
        }
        Selection::Light(idx) => {
            if let Some(l) = app_state.light.get_mut(idx) {
                l.position = [new_pos.x, new_pos.y, new_pos.z];
            }
        }
        Selection::ParticleInstance(uuid) => {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(project) = root.project.as_mut() {
                    if let Some(scene) = project.scenes.get_mut(project.active_scene_index) {
                        if let Some(inst) = scene.particle_instances.iter_mut().find(|i| i.uuid == uuid) {
                            inst.position = [new_pos.x, new_pos.y, new_pos.z];
                        }
                    }
                }
            }
        }
        _ => {}
    }
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.dirty = true;
    }
}

fn end_drag(app_state: &mut AppState) {
    if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        root.editor.gizmo.drag = None;
        root.editor.gizmo.consumed_click_this_frame = true;
    }
}
