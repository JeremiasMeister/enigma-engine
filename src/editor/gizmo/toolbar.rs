use egui::{Area, Context, Order, Rect};
use enigma_3d::AppState;

use crate::editor::state::{EditorRoot, GizmoMode, Space};

pub fn draw(ctx: &Context, rect: Rect, app_state: &mut AppState) {
    let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") else {
        return;
    };
    let g = &mut root.editor.gizmo;

    Area::new("gizmo_toolbar")
        .order(Order::Foreground)
        .fixed_pos(rect.min + egui::vec2(8.0, 8.0))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.set_max_width((rect.width() - 40.0).max(160.0));
                    let mut mode = g.mode;
                    ui.selectable_value(&mut mode, GizmoMode::None, "Select")
                        .on_hover_text("Select objects (Q)");
                    ui.selectable_value(&mut mode, GizmoMode::Translate, "Move")
                        .on_hover_text("Move selection (W)");
                    ui.selectable_value(&mut mode, GizmoMode::Rotate, "Rotate")
                        .on_hover_text("Rotate selection (E)");
                    ui.selectable_value(&mut mode, GizmoMode::Scale, "Scale")
                        .on_hover_text("Scale selection (R)");
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

                    ui.toggle_value(&mut g.snap_enabled, "Snap")
                        .on_hover_text("Snap transforms; hold Ctrl to invert");
                    ui.toggle_value(&mut g.grid_enabled, "Grid")
                        .on_hover_text("Show the ground grid");
                });
            });
        });
}
