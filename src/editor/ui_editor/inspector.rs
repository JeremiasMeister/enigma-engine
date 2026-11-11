use super::UiEditorState;
use crate::editor::theme;
use crate::ui_runtime::{layout_nodes, UiData, UiScreen, WidgetKind};
use egui::{DragValue, RichText, Ui};

fn properties(
    ui: &mut Ui,
    screen: &mut UiScreen,
    state: &mut UiEditorState,
    textures: &[(String, String)],
) {
    let selected_index = screen
        .nodes
        .iter()
        .position(|n| Some(n.id) == state.selected);
    if let Some(index) = selected_index {
        let id = screen.nodes[index].id;
        let parents: Vec<_> = screen
            .nodes
            .iter()
            .filter(|n| n.kind.is_container() && !is_descendant(screen, n.id, id))
            .map(|n| (n.id, n.name.clone()))
            .collect();
        let reference =
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(screen.size[0], screen.size[1]));
        let layout = layout_nodes(screen, reference);
        let parent_size = screen.nodes[index]
            .parent
            .and_then(|id| {
                let rect = layout.iter().find(|(i, _)| *i == id)?.1;
                let parent = screen.nodes.iter().find(|n| n.id == id)?;
                Some([
                    rect.width(),
                    if parent.kind == WidgetKind::Repeater {
                        parent.row_height
                    } else {
                        rect.height()
                    },
                ])
            })
            .unwrap_or(screen.size);
        let node = &mut screen.nodes[index];
        theme::panel_heading(ui, &node.name, node.kind.label());
        text_field(ui, "Name", &mut node.name);
        let parent_label = node
            .parent
            .and_then(|id| {
                parents
                    .iter()
                    .find(|(p, _)| *p == id)
                    .map(|(_, n)| n.as_str())
            })
            .unwrap_or("Screen");
        let old_parent = node.parent;
        egui::ComboBox::from_id_source("ui_parent")
            .selected_text(parent_label)
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut node.parent, None, "Screen");
                for (id, name) in &parents {
                    ui.selectable_value(&mut node.parent, Some(*id), name);
                }
            });
        if old_parent != node.parent {
            node.anchor = [0.0, 0.0];
            node.position = [16.0, 16.0];
        }
        if !matches!(
            node.kind,
            WidgetKind::Panel | WidgetKind::Repeater | WidgetKind::Progress | WidgetKind::Image
        ) {
            ui.add_space(8.0);
            ui.label("Text");
            ui.add(
                egui::TextEdit::multiline(&mut node.text)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );
        }
        ui.add_space(12.0);
        theme::section(ui, "Layout");
        ui.label(RichText::new("Anchor").small().color(theme::MUTED));
        let old_anchor = node.anchor;
        egui::Grid::new("ui_anchor")
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                for y in 0..3 {
                    for x in 0..3 {
                        let anchor = [x as f32 * 0.5, y as f32 * 0.5];
                        if ui
                            .add_sized(
                                [24.0, 23.0],
                                egui::SelectableLabel::new(node.anchor == anchor, "▪"),
                            )
                            .clicked()
                        {
                            node.anchor = anchor;
                        }
                    }
                    ui.end_row();
                }
            });
        if old_anchor != node.anchor {
            for axis in 0..2 {
                node.position[axis] +=
                    (old_anchor[axis] - node.anchor[axis]) * (parent_size[axis] - node.size[axis]);
            }
        }
        pair(
            ui,
            "Offset from anchor",
            ["X", "Y"],
            &mut node.position,
            -100000.0..=100000.0,
        );
        pair(ui, "Size", ["W", "H"], &mut node.size, 8.0..=8192.0);
        if node.kind == WidgetKind::Repeater {
            ui.horizontal(|ui| {
                ui.label("Row height");
                ui.add(
                    DragValue::new(&mut node.row_height)
                        .clamp_range(16.0..=4096.0)
                        .suffix(" px"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Row gap");
                ui.add(
                    DragValue::new(&mut node.gap)
                        .clamp_range(0.0..=1024.0)
                        .suffix(" px"),
                );
            });
        }
        ui.add_space(12.0);
        super::appearance::node_controls(ui, node, &mut screen.styles, &screen.textures);
        ui.horizontal(|ui| {
            ui.checkbox(&mut node.visible, "Visible");
            ui.checkbox(&mut node.enabled, "Enabled");
        });
        match node.kind {
            WidgetKind::Slider => {
                pair(
                    ui,
                    "Value range",
                    ["Min", "Max"],
                    &mut node.range,
                    -100000.0..=100000.0,
                );
                ui.add(DragValue::new(&mut node.value).prefix("Default "));
            }
            WidgetKind::Progress => {
                ui.add(egui::Slider::new(&mut node.value, 0.0..=1.0).text("Default"));
            }
            WidgetKind::Toggle => {
                ui.checkbox(&mut node.checked, "Checked by default");
            }
            _ => {}
        }
        ui.add_space(12.0);
        theme::section(ui, "Data bindings");
        ui.weak("Leave empty to use the authored value.");
        if node.kind == WidgetKind::Repeater {
            text_field(ui, "List key", &mut node.list_binding);
            ui.label(RichText::new("Children form one row template. Each item supplies its own fields and stable ID.").small().color(theme::MUTED));
        } else if !matches!(node.kind, WidgetKind::Panel | WidgetKind::Image) {
            let label = match node.kind {
                WidgetKind::Toggle => "Boolean key",
                WidgetKind::Slider | WidgetKind::Progress => "Number key",
                _ => "Text key",
            };
            text_field(ui, label, &mut node.binding);
        }
        text_field(ui, "Visibility key", &mut node.visible_binding);
        text_field(ui, "Enabled key", &mut node.enabled_binding);
        if matches!(
            node.kind,
            WidgetKind::Button | WidgetKind::Toggle | WidgetKind::Slider | WidgetKind::TextInput
        ) {
            ui.add_space(12.0);
            theme::section(ui, "Event");
            text_field(ui, "Action name", &mut node.event);
            ui.label(
                RichText::new(
                    "Your Rust handler receives this name, the value, and the clicked item's ID.",
                )
                .small()
                .color(theme::MUTED),
            );
        }
    } else {
        theme::panel_heading(ui, "Screen properties", "The root of this UI");
        text_field(ui, "Screen name", &mut screen.name);
        pair(
            ui,
            "Reference resolution",
            ["W", "H"],
            &mut screen.size,
            160.0..=8192.0,
        );
        ui.weak("Layout uses reference points and scales to fit the game window.");
        super::appearance::screen_controls(ui, screen, state, textures);
        ui.add_space(12.0);
        if ui.button("Delete screen").clicked() {
            state.delete_screen = true;
        }
        ui.weak("Undo restores a deleted screen.");
    }
}

pub fn draw(
    ui: &mut Ui,
    screen: &mut UiScreen,
    state: &mut UiEditorState,
    textures: &[(String, String)],
) {
    let screen_scope = ui.make_persistent_id((state.egui_id, &screen.id));
    // In egui 0.23 push_id changes persistent IDs but not the automatic IDs
    // used by DragValue/TextEdit. Seed a UI for each editing target instead.
    let mut fields = scoped_ui(ui, screen_scope.with(state.selected));
    properties(&mut fields, screen, state, textures);
    ui.allocate_rect(fields.min_rect(), egui::Sense::hover());
    let mut samples = scoped_ui(ui, screen_scope.with("samples"));
    sample_data(&mut samples, screen, state);
    ui.allocate_rect(samples.min_rect(), egui::Sense::hover());
}

fn scoped_ui(ui: &Ui, id: egui::Id) -> Ui {
    let mut child = Ui::new(
        ui.ctx().clone(),
        ui.layer_id(),
        id,
        ui.available_rect_before_wrap(),
        ui.clip_rect(),
    );
    child.set_style(ui.style().clone());
    child.set_enabled(ui.is_enabled());
    child
}

fn sample_data(ui: &mut Ui, screen: &mut UiScreen, state: &mut UiEditorState) {
    ui.add_space(18.0);
    egui::CollapsingHeader::new("Sample data / procedural content").id_source("ui_sample_data").show(ui, |ui| {
        ui.label("Preview initial data for this screen. Your game can replace it at any time.");
        ui.label(RichText::new("text: strings · numbers: values · flags: booleans · lists: records with id and data").small().color(theme::MUTED));
        egui::ScrollArea::vertical().id_source("ui_sample_json").max_height(240.0).show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut state.sample_json).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(10));
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("Apply sample data").clicked() {
                match serde_json::from_str::<UiData>(&state.sample_json) {
                    Ok(data) => {
                        let mut candidate = screen.clone(); candidate.sample_data = data;
                        match candidate.validate() {
                            Ok(()) => { screen.sample_data = candidate.sample_data; state.preview_data = screen.sample_data.clone(); state.error = None; state.status = "Sample data applied. Preview uses a separate copy.".into(); }
                            Err(error) => state.error = Some(error),
                        }
                    }
                    Err(error) => state.error = Some(format!("Invalid sample JSON: {error}")),
                }
            }
            if ui.button("Reset preview").clicked() { state.refresh_data(screen); }
        });
    });
}

fn text_field(ui: &mut Ui, label: &str, value: &mut String) {
    ui.push_id(label, |ui| {
        ui.label(RichText::new(label).small().color(theme::MUTED));
        ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
        ui.add_space(4.0);
    });
}

fn pair(
    ui: &mut Ui,
    label: &str,
    labels: [&str; 2],
    value: &mut [f32; 2],
    range: std::ops::RangeInclusive<f32>,
) {
    ui.push_id(label, |ui| {
        ui.add_space(7.0);
        ui.label(RichText::new(label).small().color(theme::MUTED));
        ui.horizontal(|ui| {
            let width = ((ui.available_width() - 28.0) / 2.0).max(50.0);
            for index in 0..2 {
                ui.label(labels[index]);
                ui.add_sized(
                    [width - 15.0, 24.0],
                    DragValue::new(&mut value[index])
                        .speed(1.0)
                        .clamp_range(range.clone()),
                );
            }
        });
    });
}

fn is_descendant(screen: &UiScreen, mut candidate: u64, ancestor: u64) -> bool {
    for _ in 0..=screen.nodes.len() {
        if candidate == ancestor {
            return true;
        }
        match screen
            .nodes
            .iter()
            .find(|n| n.id == candidate)
            .and_then(|n| n.parent)
        {
            Some(parent) => candidate = parent,
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_runtime::UiNode;

    fn frame(
        ctx: &egui::Context,
        screen: &mut UiScreen,
        state: &mut UiEditorState,
        time: f64,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 1200.0),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| draw(ui, screen, state, &[]));
            },
        )
    }
    fn name_point(output: &egui::FullOutput, name: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == name => {
                    Some(text.pos + egui::vec2(4.0, 5.0))
                }
                _ => None,
            })
            .nth(1)
            .expect("name field is drawn")
    }
    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }
    }
    #[test]
    fn text_undo_in_another_widget_cannot_restore_the_previous_widgets_name() {
        let ctx = egui::Context::default();
        let mut screen = UiScreen::new("Screen", "screen");
        screen.nodes = vec![
            UiNode::new(1, WidgetKind::Button, "Alpha"),
            UiNode::new(2, WidgetKind::Button, "Beta"),
        ];
        let mut state = UiEditorState::default();
        state.selected = Some(1);
        let out = frame(&ctx, &mut screen, &mut state, 0.0, vec![]);
        let click = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let a = name_point(&out, "Alpha");
        frame(
            &ctx,
            &mut screen,
            &mut state,
            1.0,
            vec![egui::Event::PointerMoved(a), click(a, true)],
        );
        frame(&ctx, &mut screen, &mut state, 1.1, vec![click(a, false)]);
        frame(
            &ctx,
            &mut screen,
            &mut state,
            2.0,
            vec![key(egui::Key::A), egui::Event::Text("Alpha edit".into())],
        );
        frame(&ctx, &mut screen, &mut state, 3.0, vec![]);
        assert_eq!(screen.nodes[0].name, "Alpha edit");
        state.selected = Some(2);
        let out = frame(&ctx, &mut screen, &mut state, 4.0, vec![]);
        let b = name_point(&out, "Beta");
        frame(
            &ctx,
            &mut screen,
            &mut state,
            4.1,
            vec![egui::Event::PointerMoved(b), click(b, true)],
        );
        frame(&ctx, &mut screen, &mut state, 4.2, vec![click(b, false)]);
        frame(&ctx, &mut screen, &mut state, 5.0, vec![]);
        frame(&ctx, &mut screen, &mut state, 6.0, vec![key(egui::Key::Z)]);
        assert_eq!(screen.nodes[1].name, "Beta");
    }
}
