use egui::{Area, Context, Order, Rect};
use enigma_3d::AppState;

use crate::editor::state::{EditorRoot, GizmoMode, Space};

pub fn draw(ctx: &Context, rect: Rect, app_state: &mut AppState) {
    let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") else { return; };
    let g = &mut root.editor.gizmo;

    Area::new("gizmo_toolbar")
        .order(Order::Foreground)
        .fixed_pos(rect.min + egui::vec2(8.0, 8.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut mode = g.mode;
                    ui.selectable_value(&mut mode, GizmoMode::None, "Select");
                    ui.selectable_value(&mut mode, GizmoMode::Translate, "Move");
                    ui.selectable_value(&mut mode, GizmoMode::Rotate, "Rotate");
                    ui.selectable_value(&mut mode, GizmoMode::Scale, "Scale");
                    g.mode = mode;

                    ui.separator();

                    let label_space = match g.space {
                        Space::World => "World",
                        Space::Local => "Local",
                    };
                    if ui.button(label_space).clicked() {
                        g.space = match g.space {
                            Space::World => Space::Local,
                            Space::Local => Space::World,
                        };
                    }

                    let snap_label = if g.snap_enabled { "Snap: On" } else { "Snap: Off" };
                    if ui.button(snap_label).clicked() {
                        g.snap_enabled = !g.snap_enabled;
                    }
                });
            });
        });
}
