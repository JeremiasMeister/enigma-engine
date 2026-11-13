use super::*;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::{BTreeMap, VecDeque};

pub(super) fn draw(ui: &mut egui::Ui, editor: &mut NarrativeEditor, doc: &mut Document) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("STORY MAP").small().color(theme::MUTED));
        ui.add(
            egui::Slider::new(&mut editor.zoom, 0.35..=1.5)
                .show_value(false)
                .text("Zoom"),
        );
        if ui
            .small_button("Arrange")
            .on_hover_text("Arrange steps by distance from entry")
            .clicked()
        {
            for node in &mut doc.nodes {
                node.extra.remove("editor_position");
            }
            editor.pan = Vec2::new(30.0, 30.0);
        }
        if ui.small_button("Focus").clicked() {
            let positions = positions(&doc.nodes);
            if let Some(pos) = positions.get(editor.selected) {
                editor.pan = Vec2::new(35.0, 35.0) - pos.to_vec2() * editor.zoom;
            }
        }
    });
    ui.label(
        RichText::new("Drag cards to arrange · Drag background to pan · Select a step to write")
            .small()
            .color(theme::MUTED),
    );
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    if response.dragged() {
        editor.pan += response.drag_delta();
    }
    let origin = response.rect.min + editor.pan;
    let zoom = editor.zoom;
    let positions = positions(&doc.nodes);
    let size = Vec2::new(220.0, 126.0) * zoom;
    let rects: Vec<_> = positions
        .iter()
        .map(|p| Rect::from_min_size(origin + p.to_vec2() * zoom, size))
        .collect();
    // Connections are drawn underneath cards so crossings never cover the writing.
    for (i, node) in doc.nodes.iter().enumerate() {
        for (choice_index, choice) in node.choices.iter().enumerate() {
            let Some(j) = doc.nodes.iter().position(|n| n.id == choice.target) else {
                continue;
            };
            let from = rects[i].right_center()
                + Vec2::new(0.0, (choice_index as f32 * 9.0).min(35.0) * zoom);
            let to = rects[j].left_center();
            let offset = ((to.x - from.x).abs() * 0.5).max(55.0 * zoom);
            let gated = choice.requires.is_some() || !choice.conditions.is_empty();
            let color = if i == editor.selected {
                if gated {
                    theme::AMBER
                } else {
                    theme::ACCENT
                }
            } else {
                Color32::from_rgb(78, 94, 119)
            };
            let controls = if i == j {
                [
                    from,
                    from + Vec2::new(90.0, -130.0) * zoom,
                    to + Vec2::new(-90.0, -130.0) * zoom,
                    to,
                ]
            } else {
                [
                    from,
                    from + Vec2::new(offset, 0.0),
                    to - Vec2::new(offset, 0.0),
                    to,
                ]
            };
            painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                controls,
                false,
                Color32::TRANSPARENT,
                Stroke::new(
                    if i == editor.selected {
                        2.0_f32
                    } else {
                        1.0_f32
                    },
                    color,
                ),
            ));
            painter.line_segment(
                [to, to + Vec2::new(-7.0, -4.0) * zoom],
                Stroke::new(1.5_f32, color),
            );
            painter.line_segment(
                [to, to + Vec2::new(-7.0, 4.0) * zoom],
                Stroke::new(1.5_f32, color),
            );
            if i == editor.selected && zoom > 0.6 {
                let label = format!(
                    "{}{}",
                    choice_index + 1,
                    if gated { " · conditional" } else { "" }
                );
                painter.text(
                    from + Vec2::new(8.0, -8.0),
                    Align2::LEFT_BOTTOM,
                    label,
                    FontId::proportional(10.0 * zoom),
                    color,
                );
            }
        }
    }
    for (i, node) in doc.nodes.iter_mut().enumerate() {
        let rect = rects[i];
        if !rect.intersects(response.rect) {
            continue;
        }
        let card = ui.interact(
            rect,
            ui.id().with(("story_card", &node.id)),
            Sense::click_and_drag(),
        );
        if card.clicked() || card.drag_started() {
            editor.selected = i;
        }
        if card.dragged() {
            let p = positions[i] + card.drag_delta() / zoom;
            node.extra
                .insert("editor_position".into(), serde_json::json!([p.x, p.y]));
        }
        let selected = editor.selected == i;
        painter.rect(
            rect,
            7.0 * zoom,
            if selected {
                Color32::from_rgb(40, 58, 82)
            } else {
                theme::PANEL
            },
            Stroke::new(
                if selected { 2.0_f32 } else { 1.0_f32 },
                if selected {
                    theme::ACCENT
                } else {
                    theme::BORDER
                },
            ),
        );
        let clip = painter.with_clip_rect(rect.shrink(8.0 * zoom).intersect(response.rect));
        let x = rect.left() + 12.0 * zoom;
        clip.text(
            Pos2::new(x, rect.top() + 13.0 * zoom),
            Align2::LEFT_TOP,
            format!("{}{}", if i == 0 { "▶ " } else { "" }, node.id),
            FontId::proportional(15.0 * zoom),
            theme::TEXT,
        );
        clip.text(
            Pos2::new(x, rect.top() + 37.0 * zoom),
            Align2::LEFT_TOP,
            &node.speaker,
            FontId::proportional(12.0 * zoom),
            theme::ACCENT,
        );
        clip.text(
            Pos2::new(x, rect.top() + 59.0 * zoom),
            Align2::LEFT_TOP,
            inspector::excerpt(&node.text, 31),
            FontId::proportional(11.0 * zoom),
            theme::MUTED,
        );
        let gated = node
            .choices
            .iter()
            .filter(|c| c.requires.is_some() || !c.conditions.is_empty())
            .count();
        let summary = if node.choices.is_empty() {
            "ENDING".into()
        } else {
            format!("{} choices · {} conditional", node.choices.len(), gated)
        };
        clip.text(
            Pos2::new(x, rect.bottom() - 15.0 * zoom),
            Align2::LEFT_BOTTOM,
            summary,
            FontId::proportional(10.0 * zoom),
            if gated > 0 {
                theme::AMBER
            } else {
                theme::MUTED
            },
        );
        card.on_hover_text(&node.text);
    }
}

fn positions(nodes: &[StoryNode]) -> Vec<egui::Pos2> {
    let mut depths = BTreeMap::new();
    let mut queue = VecDeque::new();
    if let Some(first) = nodes.first() {
        depths.insert(first.id.as_str(), 0usize);
        queue.push_back(first.id.as_str());
    }
    while let Some(id) = queue.pop_front() {
        if let Some(node) = nodes.iter().find(|n| n.id == id) {
            let next_depth = depths[id] + 1;
            for choice in &node.choices {
                if !depths.contains_key(choice.target.as_str()) {
                    depths.insert(choice.target.as_str(), next_depth);
                    queue.push_back(choice.target.as_str());
                }
            }
        }
    }
    let disconnected = depths.values().copied().max().unwrap_or(0) + 1;
    let mut lanes = BTreeMap::new();
    nodes
        .iter()
        .map(|node| {
            let depth = depths
                .get(node.id.as_str())
                .copied()
                .unwrap_or(disconnected);
            let lane = lanes.entry(depth).or_insert(0usize);
            let default = Pos2::new(depth as f32 * 290.0, *lane as f32 * 175.0);
            *lane += 1;
            node.extra
                .get("editor_position")
                .and_then(|v| {
                    let x = v.get(0)?.as_f64()? as f32;
                    let y = v.get(1)?.as_f64()? as f32;
                    if x.is_finite() && y.is_finite() {
                        Some(Pos2::new(x, y))
                    } else {
                        None
                    }
                })
                .unwrap_or(default)
        })
        .collect()
}
