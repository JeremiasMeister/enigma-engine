use super::*;
use egui::Ui;

pub(super) fn draw(ui: &mut Ui, editor: &mut NarrativeEditor, doc: &mut Document) {
    theme::panel_heading(
        ui,
        "Character library",
        "Shared across every story in this project",
    );
    if ui
        .button("Build cast from story speakers")
        .on_hover_text("Create missing characters and link existing dialogue to them")
        .clicked()
    {
        for node in &mut doc.nodes {
            if node.character.is_empty() && !node.speaker.trim().is_empty() {
                let id = if let Some(c) = doc
                    .library
                    .characters
                    .iter()
                    .find(|c| c.name == node.speaker)
                {
                    c.id.clone()
                } else {
                    let id = unique_id(
                        "character",
                        doc.library.characters.iter().map(|c| c.id.as_str()),
                    );
                    doc.library
                        .characters
                        .push(Character::new(id.clone(), node.speaker.clone()));
                    id
                };
                node.character = id;
            }
        }
    }
    let Some(character) = doc.library.characters.get_mut(editor.character) else {
        ui.label("Add a character in the cast panel, or build the cast from existing dialogue.");
        return;
    };
    ui.separator();
    ui.monospace(format!("Stable ID: {}", character.id));
    let previous_name = character.name.clone();
    ui.label("Name");
    ui.add(egui::TextEdit::singleline(&mut character.name).desired_width(400.0));
    ui.label("Description / writing notes");
    ui.add(
        egui::TextEdit::multiline(&mut character.description)
            .desired_rows(4)
            .desired_width(f32::INFINITY),
    );
    ui.add_space(8.0);
    ui.heading("Portraits & expressions");
    ui.weak("Images are copied into the project. The first variant is the default portrait.");
    let mut remove = None;
    for (variant, path) in &mut character.portraits {
        ui.push_id(variant, |ui| {
            egui::Frame::none()
                .fill(theme::SURFACE)
                .rounding(6.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        portrait(ui, editor, &doc.root, path, 150.0);
                        ui.vertical(|ui| {
                            ui.strong(variant);
                            ui.small(path.as_str());
                            if let Some(selected) = resource_picker(
                                ui,
                                "image_resource",
                                "Choose project image…",
                                false,
                                editor,
                            ) {
                                *path = selected;
                            }
                            if ui.button("Remove variant").clicked() {
                                remove = Some(variant.clone());
                            }
                        });
                    });
                });
        });
    }
    if let Some(variant) = remove {
        if doc
            .nodes
            .iter()
            .any(|n| n.character == character.id && n.portrait == variant)
        {
            editor.error = Some(
                "This portrait is used by a story step. Change that step's portrait first.".into(),
            );
        } else {
            // Other stories share this library too. Keep referenced variants intact.
            match referenced_elsewhere(&doc.root, &doc.path, &character.id, Some(&variant)) {
                Ok(false) => {
                    character.portraits.remove(&variant);
                }
                Ok(true) => editor.error = Some("Another story uses this portrait variant.".into()),
                Err(e) => editor.error = Some(e),
            }
        }
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Variant");
        ui.add(
            egui::TextEdit::singleline(&mut editor.flag_input)
                .hint_text("neutral, happy, worried…")
                .desired_width(190.0),
        );
        if let Some(path) = resource_picker(
            ui,
            "new_image_resource",
            "Select project image…",
            false,
            editor,
        ) {
            let variant = editor.flag_input.trim();
            if variant.is_empty() {
                editor.error = Some("Enter a portrait variant name first".into());
            } else {
                character.portraits.insert(variant.into(), path);
                editor.error = None;
            }
        }
        if ui.button("Import image…").clicked() {
            let variant = editor.flag_input.trim().to_owned();
            if variant.is_empty() {
                editor.error = Some("Enter a portrait variant name first".into());
            } else if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp", "tga"])
                .pick_file()
            {
                match storage::import_portrait(&doc.root, &path) {
                    Ok(path) => {
                        editor.imports.push((doc.root.clone(), path.clone(), false));
                        character.portraits.insert(variant, path);
                        editor.reload_assets(&doc.root);
                        editor.error = None;
                    }
                    Err(e) => editor.error = Some(e),
                }
            }
        }
    });
    ui.add_space(10.0);
    ui.heading("3D models & appearances");
    ui.weak("Independent of portraits: gameplay can use 2D, 3D, or both. The first model variant is the default.");
    let mut remove_model = None;
    for (variant, model) in &mut character.models {
        ui.push_id(("model", variant), |ui| {
            egui::Frame::none()
                .fill(theme::SURFACE)
                .rounding(6.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.strong(variant);
                    ui.monospace(&model.path);
                    if let Some(path) =
                        resource_picker(ui, "model_resource", "Choose project model…", true, editor)
                    {
                        model.path = path;
                        model.animation.clear();
                    }
                    ui.horizontal(|ui| {
                        ui.label("Scale");
                        ui.add(
                            egui::DragValue::new(&mut model.scale)
                                .speed(0.01)
                                .clamp_range(0.001..=1000.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Animation");
                        ui.add(
                            egui::TextEdit::singleline(&mut model.animation)
                                .hint_text("Optional clip name, e.g. Idle"),
                        );
                    });
                    ui.weak("Resolved by the game when presenting this character in 3D.");
                    if ui.small_button("Remove model variant").clicked() {
                        remove_model = Some(variant.clone());
                    }
                });
        });
    }
    if let Some(variant) = remove_model {
        let used = doc
            .nodes
            .iter()
            .any(|n| n.character == character.id && n.model == variant);
        match model_referenced_elsewhere(&doc.root, &doc.path, &character.id, &variant) {
            Ok(false) if !used => {
                character.models.remove(&variant);
            }
            Ok(_) => {
                editor.error =
                    Some("A story uses this model variant. Change its binding first.".into())
            }
            Err(e) => editor.error = Some(e),
        }
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Variant");
        ui.add(
            egui::TextEdit::singleline(&mut editor.model_input)
                .hint_text("default, armored…")
                .desired_width(190.0),
        );
        if let Some(path) = resource_picker(
            ui,
            "new_model_resource",
            "Select project model…",
            true,
            editor,
        ) {
            let variant = editor.model_input.trim();
            if variant.is_empty() {
                editor.error = Some("Enter a model variant name first".into());
            } else {
                character.models.insert(
                    variant.into(),
                    crate::narrative::ModelAsset {
                        path,
                        ..Default::default()
                    },
                );
                editor.error = None;
            }
        }
        if ui.button("Import 3D model…").clicked() {
            let variant = editor.model_input.trim().to_owned();
            if variant.is_empty() {
                editor.error = Some("Enter a model variant name first".into());
            } else if let Some(path) = rfd::FileDialog::new()
                .add_filter("glTF model", &["glb", "gltf"])
                .pick_file()
            {
                match storage::import_model(&doc.root, &path) {
                    Ok(model) => {
                        editor
                            .imports
                            .push((doc.root.clone(), model.path.clone(), true));
                        character.models.insert(variant, model);
                        editor.reload_assets(&doc.root);
                        editor.error = None;
                    }
                    Err(e) => editor.error = Some(e),
                }
            }
        }
    });
    ui.add_space(10.0);
    ui.heading("Starting attributes");
    ui.weak("Numeric values such as trust, health, coins. Conditions use character_id.attribute.");
    let mut remove = None;
    for (key, value) in &mut character.attributes {
        ui.push_id(key, |ui| {
            ui.horizontal(|ui| {
                ui.monospace(format!("{}.{key}", character.id));
                ui.add(egui::DragValue::new(value));
                if ui.small_button("×").clicked() {
                    remove = Some(key.clone());
                }
            });
        });
    }
    if let Some(key) = remove {
        character.attributes.remove(&key);
    }
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut editor.attribute_input)
                .hint_text("trust")
                .desired_width(180.0),
        );
        if ui.button("+ Attribute").clicked() {
            let key = editor.attribute_input.trim();
            if !key.is_empty() && !key.contains('.') {
                character.attributes.entry(key.into()).or_insert(0);
                editor.attribute_input.clear();
            } else {
                editor.error = Some("Use a nonempty attribute name without dots".into());
            }
        }
    });
    ui.separator();
    let id = character.id.clone();
    ui.collapsing("Dialogue using this character", |ui| {
        for (i, node) in doc
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.character == id)
        {
            if ui
                .link(format!(
                    "{} · {}",
                    node.id,
                    inspector::excerpt(&node.text, 60)
                ))
                .clicked()
            {
                editor.selected = i;
                editor.mode = Mode::Story;
            }
        }
    });
    // Keep the legacy speaker field useful for games that have not adopted character IDs.
    if character.name != previous_name {
        for node in &mut doc.nodes {
            if node.character == character.id && node.speaker == previous_name {
                node.speaker = character.name.clone();
            }
        }
    }
}

fn resource_picker(
    ui: &mut Ui,
    id: &str,
    label: &str,
    model: bool,
    editor: &mut NarrativeEditor,
) -> Option<String> {
    let options: Vec<_> = editor
        .assets
        .iter()
        .filter(|asset| asset.model == model)
        .map(|asset| (asset.path.clone(), asset.path.clone()))
        .collect();
    crate::editor::widgets::searchable_action(ui, id, label, &options)
}

fn referenced_elsewhere(
    root: &Path,
    current: &Path,
    character: &str,
    variant: Option<&str>,
) -> Result<bool, String> {
    for path in storage::discover(root)? {
        if path == current {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let nodes: Vec<StoryNode> = serde_json::from_str(&text)
            .map_err(|e| format!("Cannot check {}: {e}", path.display()))?;
        if nodes
            .iter()
            .any(|n| n.character == character && variant.is_none_or(|v| n.portrait == v))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn model_referenced_elsewhere(
    root: &Path,
    current: &Path,
    character: &str,
    variant: &str,
) -> Result<bool, String> {
    for path in storage::discover(root)? {
        if path == current {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let nodes: Vec<StoryNode> = serde_json::from_str(&text)
            .map_err(|e| format!("Cannot check {}: {e}", path.display()))?;
        if nodes
            .iter()
            .any(|n| n.character == character && n.model == variant)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod picker_tests {
    use super::*;

    fn frame(
        ctx: &Context,
        editor: &mut NarrativeEditor,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, Option<String>) {
        let mut selected = None;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    selected = resource_picker(ui, "test_picker", "Choose artwork", false, editor);
                });
            },
        );
        (output, selected)
    }
    fn text_position(output: &egui::FullOutput, text: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == text => {
                    Some(t.pos + egui::vec2(5.0, 5.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Text not rendered: {text}"))
    }
    fn click(
        ctx: &Context,
        editor: &mut NarrativeEditor,
        pos: egui::Pos2,
    ) -> (egui::FullOutput, Option<String>) {
        frame(
            ctx,
            editor,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        frame(
            ctx,
            editor,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
    }
    #[test]
    fn resource_search_keeps_popup_open_and_focused_until_selection_or_dismissal() {
        let ctx = Context::default();
        let mut editor = NarrativeEditor {
            assets: vec![
                storage::PresentationAsset {
                    path: "portraits/hero.png".into(),
                    model: false,
                },
                storage::PresentationAsset {
                    path: "portraits/villain.png".into(),
                    model: false,
                },
            ],
            ..Default::default()
        };
        let (output, _) = frame(&ctx, &mut editor, vec![]);
        let button = text_position(&output, "Choose artwork");
        click(&ctx, &mut editor, button);
        let (output, _) = frame(&ctx, &mut editor, vec![]);
        let search = text_position(&output, "Search options…");
        click(&ctx, &mut editor, search);
        assert!(
            ctx.memory(|m| m.any_popup_open()),
            "clicking search must not dismiss the popup"
        );
        let (output, _) = frame(&ctx, &mut editor, vec![egui::Event::Text("hero".into())]);
        assert!(ctx.memory(|m| m.any_popup_open()));
        let asset = text_position(&output, "portraits/hero.png");
        let (_, selected) = click(&ctx, &mut editor, asset);
        assert_eq!(selected.as_deref(), Some("portraits/hero.png"));
        assert!(!ctx.memory(|m| m.any_popup_open()));
        click(&ctx, &mut editor, button);
        frame(&ctx, &mut editor, vec![]);
        click(&ctx, &mut editor, egui::pos2(700.0, 500.0));
        assert!(!ctx.memory(|m| m.any_popup_open()));
        click(&ctx, &mut editor, button);
        frame(
            &ctx,
            &mut editor,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(!ctx.memory(|m| m.any_popup_open()));
    }
}
