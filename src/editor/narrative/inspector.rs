use super::*;
use crate::narrative::{validate, Choice, Comparison, Condition, Effect};
use egui::Ui;

pub(super) fn story(ui: &mut Ui, editor: &mut NarrativeEditor, doc: &mut Document) {
    let index = editor.selected;
    if index >= doc.nodes.len() {
        ui.label("Add a story step to begin.");
        return;
    }
    let targets: Vec<String> = doc.nodes.iter().map(|n| n.id.clone()).collect();
    let target_labels: Vec<String> = doc.nodes.iter().map(step_label).collect();
    let old_id = doc.nodes[index].id.clone();
    theme::panel_heading(
        ui,
        "Write this moment",
        "Dialogue, choices, and consequences",
    );
    if editor.rename_selection != old_id {
        editor.rename = old_id.clone();
        editor.rename_selection = old_id.clone();
    }
    ui.collapsing("Step settings · ID, entry & duplicate", |ui| {
        ui.horizontal(|ui| {
            ui.label("ID");
            ui.add(egui::TextEdit::singleline(&mut editor.rename).desired_width(145.0));
            if ui.small_button("Rename").clicked() {
                let new = editor.rename.trim();
                if new.is_empty() || (new != old_id && targets.iter().any(|id| id == new)) {
                    editor.error = Some("Step IDs must be nonempty and unique".into());
                } else {
                    for n in &mut doc.nodes {
                        for c in &mut n.choices {
                            if c.target == old_id {
                                c.target = new.into();
                            }
                        }
                    }
                    doc.nodes[index].id = new.into();
                    editor.rename_selection = new.into();
                    editor.error = None;
                }
            }
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(index != 0, egui::Button::new("Make entry"))
                .clicked()
            {
                let node = doc.nodes.remove(index);
                doc.nodes.insert(0, node);
                editor.selected = 0;
            }
            if ui.button("Duplicate").clicked() {
                let mut node = doc.nodes[index].clone();
                node.id = unique_id("step", doc.nodes.iter().map(|n| n.id.as_str()));
                node.extra.remove("editor_position");
                doc.nodes.push(node);
                editor.selected = doc.nodes.len() - 1;
            }
        });
    });
    if editor.selected != index {
        return;
    }
    let node = &mut doc.nodes[index];
    ui.separator();
    ui.label("Speaker / character");
    let mut characters = vec![(String::new(), "Narrator / custom speaker".into())];
    characters.extend(
        doc.library
            .characters
            .iter()
            .map(|c| (c.id.clone(), c.name.clone())),
    );
    if crate::editor::widgets::searchable_combo(
        ui,
        "node_character",
        &mut node.character,
        &characters,
    ) {
        if let Some(c) = doc
            .library
            .characters
            .iter()
            .find(|c| c.id == node.character)
        {
            node.speaker = c.name.clone();
        }
        node.portrait.clear();
        node.model.clear();
    }
    ui.add(
        egui::TextEdit::singleline(&mut node.speaker)
            .hint_text("Displayed speaker")
            .desired_width(f32::INFINITY),
    );
    if let Some(c) = doc
        .library
        .characters
        .iter()
        .find(|c| c.id == node.character)
    {
        let mut options = vec![(String::new(), "Default portrait".into())];
        options.extend(c.portraits.keys().map(|name| (name.clone(), name.clone())));
        crate::editor::widgets::searchable_combo(ui, "node_portrait", &mut node.portrait, &options);
        if let Some(path) = c
            .portraits
            .get(&node.portrait)
            .or_else(|| c.portraits.values().next())
        {
            portrait(ui, editor, &doc.root, path, 100.0);
        }
        let mut options = vec![(String::new(), "Default 3D model".into())];
        options.extend(c.models.keys().map(|name| (name.clone(), name.clone())));
        crate::editor::widgets::searchable_combo(ui, "node_model", &mut node.model, &options);
    }
    ui.label("Text");
    ui.add(
        egui::TextEdit::multiline(&mut node.text)
            .hint_text("What happens? What do they say?")
            .desired_rows(10)
            .desired_width(f32::INFINITY),
    );
    ui.label("Audio for this moment");
    let mut audio_options = vec![(String::new(), "None".into())];
    audio_options.extend(
        editor
            .audio_files
            .iter()
            .map(|path| (path.clone(), path.clone())),
    );
    crate::editor::widgets::searchable_combo(ui, "node_audio", &mut node.audio, &audio_options);
    ui.small(
        "Choose project audio (MP3, WAV, OGG or FLAC). Refresh after adding files to resources.",
    );
    ui.add_space(6.0);
    let mut create_branch = false;
    let mut continue_story = false;
    ui.horizontal_wrapped(|ui| {
        ui.strong("What happens next?");
        if node.choices.is_empty() {
            continue_story = ui
                .button("Continue writing →")
                .on_hover_text("Create and open the next moment, linked with a Continue choice")
                .clicked();
        }
        create_branch = ui
            .button("+ Choice & new moment")
            .on_hover_text("Create a new branch; edit its choice here, then open the destination")
            .clicked();
        if ui.small_button("+ Link existing moment").clicked() {
            node.choices.push(Choice {
                text: "New choice".into(),
                target: targets.first().cloned().unwrap_or_default(),
                ..Default::default()
            });
        }
    });
    if node.choices.is_empty() {
        ui.weak("This can be an ending, or continue writing below.");
    }
    let mut remove = None;
    let mut move_choice = None;
    for (i, choice) in node.choices.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            egui::Frame::none()
                .fill(theme::SURFACE)
                .rounding(5.0)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("CHOICE {}", i + 1))
                                .small()
                                .color(theme::ACCENT),
                        );
                        if ui
                            .small_button("×")
                            .on_hover_text("Remove choice (Undo available)")
                            .clicked()
                        {
                            remove = Some(i);
                        }
                        if i > 0 && ui.small_button("↑").clicked() {
                            move_choice = Some((i, i - 1));
                        }
                    });
                    ui.add(
                        egui::TextEdit::multiline(&mut choice.text)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    );
                    ui.horizontal(|ui| {
                        ui.label("Go to");
                        crate::editor::widgets::searchable_combo(
                            ui,
                            "target",
                            &mut choice.target,
                            &targets
                                .iter()
                                .cloned()
                                .zip(target_labels.iter().cloned())
                                .collect::<Vec<_>>(),
                        );
                        if ui
                            .small_button("Write this →")
                            .on_hover_text("Select destination")
                            .clicked()
                        {
                            if let Some(i) = targets.iter().position(|id| id == &choice.target) {
                                editor.selected = i;
                            }
                        }
                    });
                    ui.collapsing("Requirements · all must pass", |ui| {
                        optional_flag(ui, "Requires flag", &mut choice.requires);
                        conditions(ui, &mut choice.conditions, &doc.library);
                    });
                    ui.collapsing("Effects · after choosing", |ui| {
                        optional_flag(ui, "Grants flag", &mut choice.grants);
                        effects(ui, &mut choice.effects, &doc.library);
                    });
                });
            ui.add_space(6.0);
        });
    }
    if let Some(i) = remove {
        node.choices.remove(i);
    } else if let Some((a, b)) = move_choice {
        node.choices.swap(a, b);
    }
    ui.collapsing("Engine hooks · entering this step", |ui| {
        ui.weak("Named events returned to your game, e.g. scene:castle or audio:bell. Your game handles them.");
        let mut remove = None;
        for (i, event) in node.on_enter.iter_mut().enumerate() {
            ui.push_id(i, |ui| { ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(event).desired_width(210.0));
                if ui.small_button("×").clicked() { remove = Some(i); }
            }); });
        }
        if let Some(i) = remove { node.on_enter.remove(i); }
        if ui.small_button("+ Entry event").clicked() { node.on_enter.push("event:new".into()); }
    });
    if continue_story || create_branch {
        if let Some(destination) = add_connected_step(
            &mut doc.nodes,
            index,
            if continue_story {
                "Continue"
            } else {
                "New choice"
            },
        ) {
            if continue_story {
                editor.selected = destination;
                editor.rename_selection.clear();
            }
        }
    }
    ui.separator();
    let incoming = doc
        .nodes
        .iter()
        .filter(|n| n.id != old_id)
        .flat_map(|n| &n.choices)
        .filter(|c| c.target == old_id)
        .count();
    ui.collapsing("Delete step", |ui| {
        if incoming > 0 {
            ui.label(format!(
                "Reconnect or remove {incoming} incoming choices first."
            ));
        }
        if ui
            .add_enabled(
                incoming == 0 && doc.nodes.len() > 1,
                egui::Button::new("Delete this step (Undo available)"),
            )
            .clicked()
        {
            doc.nodes.remove(index);
            editor.selected = index.saturating_sub(1);
        }
    });
}

fn optional_flag(ui: &mut Ui, label: &str, value: &mut Option<String>) {
    ui.label(label);
    let mut text = value.clone().unwrap_or_default();
    if ui
        .add(
            egui::TextEdit::singleline(&mut text)
                .hint_text("None")
                .desired_width(f32::INFINITY),
        )
        .changed()
    {
        *value = if text.is_empty() { None } else { Some(text) };
    }
}
fn attribute_key(ui: &mut Ui, key: &mut String, library: &CharacterLibrary) {
    let attributes: Vec<_> = library
        .characters
        .iter()
        .flat_map(|character| {
            character.attributes.keys().map(move |attr| {
                (
                    format!("{}.{attr}", character.id),
                    format!("{} · {attr}", character.name),
                )
            })
        })
        .collect();
    crate::editor::widgets::searchable_combo(ui, "attribute_pick", key, &attributes);
    ui.collapsing("World attribute / custom key", |ui| {
        ui.add(
            egui::TextEdit::singleline(key)
                .hint_text("e.g. coins")
                .desired_width(180.0),
        );
    });
}
fn conditions(ui: &mut Ui, items: &mut Vec<Condition>, library: &CharacterLibrary) {
    let mut remove = None;
    for (i, condition) in items.iter_mut().enumerate() {
        ui.push_id(("condition", i), |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                ui.small(condition.label());
                if ui.small_button("×").clicked() {
                    remove = Some(i);
                }
            });
            match condition {
                Condition::Flag { flag, present } => {
                    ui.text_edit_singleline(flag);
                    ui.checkbox(present, "Flag must be present");
                }
                Condition::Attribute {
                    key,
                    comparison,
                    value,
                } => {
                    attribute_key(ui, key, library);
                    ui.horizontal(|ui| {
                        crate::editor::widgets::searchable_combo(
                            ui,
                            "comparison",
                            comparison,
                            &[
                                Comparison::AtLeast,
                                Comparison::AtMost,
                                Comparison::Equal,
                                Comparison::NotEqual,
                            ]
                            .into_iter()
                            .map(|cmp| (cmp, cmp.label().to_owned()))
                            .collect::<Vec<_>>(),
                        );
                        ui.add(egui::DragValue::new(value));
                    });
                }
            }
        });
    }
    if let Some(i) = remove {
        items.remove(i);
    }
    ui.horizontal(|ui| {
        if ui.small_button("+ Flag condition").clicked() {
            items.push(Condition::Flag {
                flag: "flag".into(),
                present: true,
            });
        }
        if ui.small_button("+ Attribute condition").clicked() {
            items.push(Condition::Attribute {
                key: String::new(),
                comparison: Comparison::AtLeast,
                value: 1,
            });
        }
    });
}
fn effects(ui: &mut Ui, items: &mut Vec<Effect>, library: &CharacterLibrary) {
    let mut remove = None;
    for (i, effect) in items.iter_mut().enumerate() {
        ui.push_id(("effect", i), |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                ui.small(effect.label());
                if ui.small_button("×").clicked() {
                    remove = Some(i);
                }
            });
            match effect {
                Effect::Flag { flag, present } => {
                    ui.text_edit_singleline(flag);
                    ui.checkbox(present, "Grant (uncheck to remove)");
                }
                Effect::Set { key, value } | Effect::Add { key, value } => {
                    attribute_key(ui, key, library);
                    ui.add(egui::DragValue::new(value));
                }
                Effect::Event { name } => {
                    ui.add(egui::TextEdit::singleline(name).hint_text("scene:castle"));
                }
            }
        });
    }
    if let Some(i) = remove {
        items.remove(i);
    }
    let options: Vec<_> = [
        (
            "Grant / remove flag",
            Effect::Flag {
                flag: "flag".into(),
                present: true,
            },
        ),
        (
            "Set attribute",
            Effect::Set {
                key: String::new(),
                value: 0,
            },
        ),
        (
            "Add to attribute",
            Effect::Add {
                key: String::new(),
                value: 1,
            },
        ),
        (
            "Engine event",
            Effect::Event {
                name: "event:new".into(),
            },
        ),
    ]
    .into_iter()
    .map(|(label, effect)| (effect, label.to_owned()))
    .collect();
    if let Some(effect) =
        crate::editor::widgets::searchable_action(ui, "add_effect", "+ Effect", &options)
    {
        items.push(effect);
    }
}

pub(super) fn dependencies(ui: &mut Ui, editor: &mut NarrativeEditor, doc: &Document) {
    theme::panel_heading(
        ui,
        "Requirements & dependencies",
        "Understand what unlocks each choice and where it comes from",
    );
    ui.weak("Connections show possible routes. Playthrough evaluates exact conditions for the path you take; game code can also supply state.");
    let Some(node) = doc.nodes.get(editor.selected) else {
        return;
    };
    ui.heading(&node.id);
    ui.collapsing("Incoming choices", |ui| {
        for (i, source) in doc.nodes.iter().enumerate() {
            for choice in &source.choices {
                if choice.target == node.id
                    && ui
                        .link(format!("{} → {}", source.id, choice.text))
                        .clicked()
                {
                    editor.selected = i;
                }
            }
        }
    });
    for (index, choice) in node.choices.iter().enumerate() {
        egui::Frame::none().fill(theme::SURFACE).rounding(6.0).inner_margin(12.0).show(ui, |ui| {
            ui.strong(format!("{}. {} → {}", index + 1, choice.text, choice.target));
            let mut requirements = choice.conditions.clone();
            if let Some(flag) = &choice.requires { requirements.insert(0, Condition::Flag { flag: flag.clone(), present: true }); }
            if requirements.is_empty() { ui.colored_label(theme::ACCENT, "Always available at this step"); }
            for requirement in requirements {
                ui.label(requirement.label());
                let mut found = false;
                if let Condition::Attribute { key, .. } = &requirement {
                    for c in &doc.library.characters { for (attr, value) in &c.attributes {
                        if format!("{}.{attr}", c.id) == *key { ui.small(format!("Starts at {value} on {}", c.name)); found = true; }
                    } }
                }
                for (i, source) in doc.nodes.iter().enumerate() { for c in &source.choices {
                    let produces = match &requirement {
                        Condition::Flag { flag, present } => (*present && c.grants.as_ref() == Some(flag)) || c.effects.iter().any(|e| matches!(e, Effect::Flag { flag: f, present: p } if f == flag && p == present)),
                        Condition::Attribute { key, .. } => c.effects.iter().any(|e| matches!(e, Effect::Add { key: k, .. } | Effect::Set { key: k, .. } if k == key)),
                    };
                    if produces { found = true; if ui.link(format!("{} · {}", source.id, c.text)).clicked() { editor.selected = i; editor.mode = Mode::Story; } }
                } }
                if !found { ui.colored_label(theme::AMBER, "No matching effect in this story; may depend on initial or external state."); }
            }
            if let Some(flag) = &choice.grants { ui.small(format!("Grants: {flag}")); }
            for effect in &choice.effects { ui.small(effect.label()); }
        });
        ui.add_space(8.0);
    }
    ui.separator();
    ui.heading("Story checks");
    let issues = validate(&doc.nodes, &doc.library);
    if issues.is_empty() {
        ui.label("No broken references or disconnected steps found.");
    }
    for issue in issues {
        let color = if issue.error {
            egui::Color32::LIGHT_RED
        } else {
            theme::AMBER
        };
        if ui
            .link(RichText::new(format!("{}: {}", issue.node, issue.message)).color(color))
            .clicked()
        {
            if let Some(i) = doc.nodes.iter().position(|n| n.id == issue.node) {
                editor.selected = i;
                editor.mode = Mode::Story;
            }
        }
    }
}

pub(super) fn excerpt(text: &str, limit: usize) -> String {
    let clean = text.replace('\n', " ");
    let mut excerpt: String = clean.chars().take(limit).collect();
    if clean.chars().count() > limit {
        excerpt.push('…');
    }
    excerpt
}
