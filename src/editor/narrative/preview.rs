use super::*;
use crate::narrative::validate;

pub(super) fn draw(ui: &mut egui::Ui, editor: &mut NarrativeEditor, doc: &Document) {
    theme::panel_heading(
        ui,
        "Playthrough",
        "Test the actual path, requirements, and consequences",
    );
    let errors: Vec<_> = validate(&doc.nodes, &doc.library)
        .into_iter()
        .filter(|i| i.error)
        .collect();
    if !errors.is_empty() {
        ui.colored_label(
            theme::AMBER,
            "Fix these references before starting a playthrough:",
        );
        for issue in errors {
            ui.label(format!("{}: {}", issue.node, issue.message));
        }
        return;
    }
    if editor.play.is_none() {
        match PlayState::start(&doc.nodes, &doc.library) {
            Ok((state, events)) => {
                editor.play = Some(state);
                editor.events = events;
                editor.play_history.clear();
            }
            Err(e) => {
                ui.label(e);
                return;
            }
        }
    }
    ui.horizontal(|ui| {
        if ui.button("Restart from entry").clicked() {
            editor.play = None;
        }
        if ui
            .add_enabled(
                !editor.play_history.is_empty(),
                egui::Button::new("Back one choice"),
            )
            .clicked()
        {
            if let Some((state, events)) = editor.play_history.pop() {
                editor.play = Some(state);
                editor.events = events;
            }
        }
        ui.weak("Preview only · does not change your authored data");
    });
    let Some(mut state) = editor.play.take() else {
        return;
    };
    ui.separator();
    if let Some(node) = doc.nodes.iter().find(|n| n.id == state.node) {
        ui.small(format!("Current step: {}", node.id));
        let character = doc
            .library
            .characters
            .iter()
            .find(|c| c.id == node.character);
        ui.horizontal(|ui| {
            if let Some(character) = character {
                if let Some(path) = character
                    .portraits
                    .get(&node.portrait)
                    .or_else(|| character.portraits.values().next())
                {
                    portrait(ui, editor, &doc.root, path, 170.0);
                }
            }
            ui.vertical(|ui| {
                ui.heading(character.map(|c| c.name.as_str()).unwrap_or(&node.speaker));
                ui.add(egui::Label::new(RichText::new(&node.text).size(17.0)).wrap(true));
            });
        });
        if let Some(presentation) = doc.library.presentation(node) {
            if let Some(model) = presentation.model {
                ui.collapsing("3D presentation binding", |ui| {
                    ui.monospace(&model.path);
                    ui.label(format!("Scale: {} · Animation: {}", model.scale, if model.animation.is_empty() { "None" } else { &model.animation }));
                    ui.weak("The runtime exposes this model alongside the portrait. This dialogue preview displays the 2D image; gameplay can instantiate the 3D model.");
                });
            }
        }
        ui.add_space(18.0);
        let mut choose = None;
        for (index, choice) in node.choices.iter().enumerate() {
            let unmet = state.unmet(choice);
            ui.push_id(index, |ui| {
                egui::Frame::none()
                    .fill(theme::SURFACE)
                    .rounding(6.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        if ui
                            .add_enabled(
                                unmet.is_empty(),
                                egui::Button::new(&choice.text)
                                    .min_size(egui::vec2(ui.available_width(), 34.0))
                                    .wrap(true),
                            )
                            .clicked()
                        {
                            choose = Some(index);
                        }
                        if unmet.is_empty() {
                            ui.small(format!("Available → {}", choice.target));
                        } else {
                            for reason in unmet {
                                ui.colored_label(theme::AMBER, format!("Hidden in game: {reason}"));
                            }
                        }
                        if let Some(flag) = &choice.grants {
                            ui.small(format!("Grants: {flag}"));
                        }
                        for effect in &choice.effects {
                            ui.small(effect.label());
                        }
                    });
            });
            ui.add_space(5.0);
        }
        if node.choices.is_empty() {
            ui.colored_label(theme::ACCENT, "End of this path.");
        }
        if let Some(index) = choose {
            let before = state.clone();
            match state.choose(&doc.nodes, index) {
                Ok(events) => {
                    editor.play_history.push((before, editor.events.clone()));
                    if editor.play_history.len() > 100 {
                        editor.play_history.remove(0);
                    }
                    editor.events.extend(events);
                    if editor.events.len() > 200 {
                        editor.events.drain(..editor.events.len() - 200);
                    }
                    editor.error = None;
                }
                Err(e) => editor.error = Some(e),
            }
        }
    }
    ui.add_space(16.0);
    ui.collapsing("Inspect / change preview state", |ui| {
        ui.weak("Simulate inventory, flags, or attributes supplied by game code.");
        ui.strong("Flags");
        let mut remove = None;
        for flag in &state.flags {
            ui.horizontal(|ui| {
                ui.monospace(flag);
                if ui.small_button(format!("Remove {flag}")).clicked() {
                    remove = Some(flag.clone());
                }
            });
        }
        if let Some(flag) = remove {
            state.flags.remove(&flag);
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut editor.flag_input).hint_text("flag name"));
            if ui.button("Grant flag").clicked() && !editor.flag_input.trim().is_empty() {
                state.flags.insert(editor.flag_input.trim().into());
                editor.flag_input.clear();
            }
        });
        ui.strong("Attributes");
        for (key, value) in &mut state.attributes {
            ui.push_id(key, |ui| {
                ui.horizontal(|ui| {
                    ui.monospace(key);
                    ui.add(egui::DragValue::new(value));
                });
            });
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut editor.attribute_input)
                    .hint_text("world key or character.attribute"),
            );
            if ui.button("Add attribute").clicked() && !editor.attribute_input.trim().is_empty() {
                state
                    .attributes
                    .entry(editor.attribute_input.trim().into())
                    .or_insert(0);
                editor.attribute_input.clear();
            }
        });
    });
    ui.collapsing("Engine event log", |ui| {
        ui.weak("Events are emitted in order. Preview records them; game code handles them.");
        if editor.events.is_empty() {
            ui.label("No events emitted on this path.");
        }
        for (i, event) in editor.events.iter().enumerate() {
            ui.monospace(format!("{}  {event}", i + 1));
        }
    });
    editor.play = Some(state);
}
