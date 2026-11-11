use super::{Mode, UiEditorState};
use crate::editor::theme;
use crate::ui_runtime::{draw_screen_with_textures, layout_nodes, UiScreen};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

pub struct Drag {
    id: u64,
    pointer: Pos2,
    position: [f32; 2],
    size: [f32; 2],
    resize: bool,
}

pub fn draw(ui: &mut egui::Ui, screen: &mut UiScreen, state: &mut UiEditorState) {
    egui::Frame::none()
        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(&screen.name).strong());
                ui.separator();
                egui::ComboBox::from_id_source("ui_canvas_resolution")
                    .selected_text(format!(
                        "{} × {}",
                        screen.size[0] as u32, screen.size[1] as u32
                    ))
                    .show_ui(ui, |ui| {
                        for size in [
                            [1280.0, 720.0],
                            [1920.0, 1080.0],
                            [960.0, 720.0],
                            [720.0, 1280.0],
                        ] {
                            ui.selectable_value(
                                &mut screen.size,
                                size,
                                format!("{} × {}", size[0] as u32, size[1] as u32),
                            );
                        }
                    });
                ui.selectable_value(&mut state.zoom, 0.0, "Fit");
                ui.selectable_value(&mut state.zoom, 1.0, "100%");
                ui.toggle_value(&mut state.snap, "Snap 8 px");
                if state.mode == Mode::Preview && ui.button("Reset data").clicked() {
                    state.preview_data = screen.sample_data.clone();
                }
            });
        });
    let available = ui.available_size().max(Vec2::splat(1.0));
    if screen.size.iter().any(|v| !v.is_finite() || *v <= 0.0) {
        ui.colored_label(
            theme::AMBER,
            "Set a positive screen resolution in properties.",
        );
        return;
    }
    let fit = ((available.x - 48.0) / screen.size[0])
        .min((available.y - 54.0) / screen.size[1])
        .clamp(0.02, 2.0);
    let scale = if state.zoom > 0.0 { state.zoom } else { fit };
    egui::ScrollArea::both()
        .id_source("ui_canvas_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let screen_size = egui::vec2(screen.size[0], screen.size[1]) * scale;
            let desired = (screen_size + egui::vec2(48.0, 54.0)).max(available);
            let (outer, response) = ui.allocate_exact_size(desired, Sense::click_and_drag());
            let board = Rect::from_center_size(outer.center() + egui::vec2(0.0, 8.0), screen_size);
            let painter = ui.painter().with_clip_rect(outer.intersect(ui.clip_rect()));
            painter.rect_filled(outer, 0.0, theme::BACKGROUND);
            let visible = outer.intersect(ui.clip_rect());
            let start_x = (visible.left() / 18.0).floor() as i32;
            let start_y = (visible.top() / 18.0).floor() as i32;
            for y in start_y..=(visible.bottom() / 18.0) as i32 {
                for x in start_x..=(visible.right() / 18.0) as i32 {
                    painter.circle_filled(
                        egui::pos2(x as f32 * 18.0, y as f32 * 18.0),
                        0.7,
                        theme::BORDER,
                    );
                }
            }
            painter.text(
                board.left_top() - egui::vec2(0.0, 18.0),
                egui::Align2::LEFT_CENTER,
                format!(
                    "{}  ·  {} × {}  ·  {:.0}%",
                    screen.name,
                    screen.size[0],
                    screen.size[1],
                    scale * 100.0
                ),
                egui::FontId::proportional(11.0),
                theme::MUTED,
            );
            painter.rect_filled(board, 0.0, Color32::from_rgb(19, 27, 38));
            painter.rect_stroke(board, 0.0, Stroke::new(1.0_f32, theme::BORDER));
            let mut child = ui.child_ui_with_id_source(
                board,
                egui::Layout::top_down(egui::Align::Min),
                &screen.id,
            );
            child.set_clip_rect(board.intersect(ui.clip_rect()));
            let events = draw_screen_with_textures(
                &mut child,
                screen,
                &mut state.preview_data,
                board,
                state.mode == Mode::Preview,
                &state.textures,
            );
            if let Some(event) = events.last() {
                state.status = format!(
                    "Event: {}  ·  widget: {}{}  ·  {:?}",
                    event.name,
                    event.widget,
                    event
                        .item_id
                        .as_ref()
                        .map(|id| format!("  ·  item: {id}"))
                        .unwrap_or_default(),
                    event.value
                );
            }
            if state.mode == Mode::Preview {
                return;
            }
            let layout = layout_nodes(screen, board);
            let selected_rect = state
                .selected
                .and_then(|id| layout.iter().find(|(i, _)| *i == id).map(|(_, r)| *r));
            if let Some(rect) = selected_rect {
                let p = ui.painter().with_clip_rect(board.intersect(ui.clip_rect()));
                p.rect_stroke(rect.expand(2.0), 1.0, Stroke::new(1.5_f32, theme::ACCENT));
                let corner = Rect::from_center_size(rect.right_bottom(), Vec2::splat(9.0));
                p.rect_filled(corner, 1.0, theme::BACKGROUND);
                p.rect_stroke(corner, 1.0, Stroke::new(1.5_f32, theme::ACCENT));
                if let Some(node) = screen.nodes.iter().find(|n| Some(n.id) == state.selected) {
                    p.text(
                        rect.left_top() - egui::vec2(0.0, 5.0),
                        egui::Align2::LEFT_BOTTOM,
                        &node.name,
                        egui::FontId::proportional(11.0),
                        theme::ACCENT,
                    );
                }
            }
            let pointer = ui.input(|i| i.pointer.interact_pos());
            if let Some(pos) = pointer {
                let in_canvas = board.intersect(ui.clip_rect()).contains(pos) && response.hovered();
                let resize = selected_rect
                    .map(|r| {
                        Rect::from_center_size(r.right_bottom(), Vec2::splat(18.0)).contains(pos)
                    })
                    .unwrap_or(false);
                if in_canvas && resize {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                }
                if in_canvas && ui.input(|i| i.pointer.primary_pressed()) {
                    if !resize {
                        state.selected = layout
                            .iter()
                            .rev()
                            .find(|(_, r)| r.contains(pos))
                            .map(|(id, _)| *id);
                    }
                    if let Some(node) = screen.nodes.iter().find(|n| Some(n.id) == state.selected) {
                        state.drag = Some(Drag {
                            id: node.id,
                            pointer: pos,
                            position: node.position,
                            size: node.size,
                            resize,
                        });
                    }
                }
                if ui.input(|i| i.pointer.primary_down()) {
                    if let Some(drag) = state.drag.as_ref() {
                        if let Some(node) = screen.nodes.iter_mut().find(|n| n.id == drag.id) {
                            let delta = (pos - drag.pointer) / scale;
                            let snap = state.snap && !ui.input(|i| i.modifiers.alt);
                            let (position, size) = transform_drag(
                                drag.position,
                                drag.size,
                                node.anchor,
                                [delta.x, delta.y],
                                drag.resize,
                                snap,
                            );
                            node.position = position;
                            node.size = size;
                            ui.ctx().set_cursor_icon(if drag.resize {
                                egui::CursorIcon::ResizeNwSe
                            } else {
                                egui::CursorIcon::Grabbing
                            });
                        }
                    }
                }
            }
            if !ui.input(|i| i.pointer.primary_down()) {
                state.drag = None;
            }
        });
}

fn transform_drag(
    position: [f32; 2],
    size: [f32; 2],
    anchor: [f32; 2],
    delta: [f32; 2],
    resize: bool,
    snap: bool,
) -> ([f32; 2], [f32; 2]) {
    let quantize = |v: f32| if snap { (v / 8.0).round() * 8.0 } else { v };
    let mut new_position = position;
    let mut new_size = size;
    for axis in 0..2 {
        if delta[axis].abs() <= 0.01 {
            continue;
        }
        if resize {
            new_size[axis] = quantize(size[axis] + delta[axis]).clamp(8.0, 8192.0);
            // The matching anchor/pivot would move the near corner; compensate.
            new_position[axis] += (new_size[axis] - size[axis]) * anchor[axis];
        } else if delta[axis].abs() > 0.01 {
            new_position[axis] = quantize(position[axis] + delta[axis]);
        }
    }
    (new_position, new_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clicking_resize_handle_without_motion_preserves_unsnapped_size() {
        assert_eq!(
            transform_drag(
                [13.0, 17.0],
                [91.0, 41.0],
                [0.5, 0.5],
                [0.0, 0.0],
                true,
                true
            ),
            ([13.0, 17.0], [91.0, 41.0])
        );
    }
    #[test]
    fn resizing_center_anchored_widget_keeps_its_near_corner_fixed() {
        let (position, size) = transform_drag(
            [0.0, 0.0],
            [100.0, 50.0],
            [0.5, 0.5],
            [20.0, 10.0],
            true,
            false,
        );
        assert_eq!(position, [10.0, 5.0]);
        assert_eq!(size, [120.0, 60.0]);
        assert_eq!(
            [position[0] - size[0] * 0.5, position[1] - size[1] * 0.5],
            [-50.0, -25.0]
        );
    }
    #[test]
    fn click_without_motion_does_not_snap_an_existing_widget() {
        assert_eq!(
            transform_drag(
                [13.0, 17.0],
                [91.0, 41.0],
                [0.0, 0.0],
                [0.0, 0.0],
                false,
                true
            )
            .0,
            [13.0, 17.0]
        );
    }
}
