use super::{unique_name, UiEditorState};
use crate::editor::theme;
use crate::ui_runtime::{ImageFit, UiFont, UiNode, UiScreen, UiStyle, UiTextAlign, WidgetKind};
use egui::{DragValue, Ui};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq)]
enum ThemePreset {
    Modern,
    Parchment,
    Pixel,
    Neon,
}

pub fn role(kind: WidgetKind) -> &'static str {
    match kind {
        WidgetKind::Panel => "panel",
        WidgetKind::Repeater => "list",
        WidgetKind::Label => "label",
        WidgetKind::Button | WidgetKind::Toggle => "button",
        WidgetKind::Image => "image",
        _ => "field",
    }
}

fn apply_theme(screen: &mut UiScreen, preset: ThemePreset) {
    let authored: Vec<_> = screen
        .nodes
        .iter()
        .map(|node| {
            screen
                .styles
                .get(&node.style_key)
                .cloned()
                .unwrap_or_else(|| UiStyle::from_node(node))
        })
        .collect();
    let (fill, text, accent, border, rounding, width, font) = match preset {
        ThemePreset::Modern => (
            [25, 35, 50, 244],
            [232, 239, 250, 255],
            [60, 91, 132, 255],
            [73, 92, 118, 255],
            12.0,
            1.0,
            UiFont::Proportional,
        ),
        ThemePreset::Parchment => (
            [225, 207, 164, 255],
            [54, 36, 23, 255],
            [188, 154, 102, 255],
            [107, 72, 36, 255],
            3.0,
            3.0,
            UiFont::Proportional,
        ),
        ThemePreset::Pixel => (
            [24, 24, 40, 255],
            [244, 238, 206, 255],
            [70, 55, 105, 255],
            [221, 191, 115, 255],
            0.0,
            3.0,
            UiFont::Monospace,
        ),
        ThemePreset::Neon => (
            [8, 19, 29, 240],
            [182, 255, 238, 255],
            [17, 64, 75, 255],
            [54, 230, 209, 255],
            0.0,
            1.0,
            UiFont::Monospace,
        ),
    };
    for key in [
        "panel", "label", "heading", "button", "field", "image", "list",
    ] {
        let mut style = UiStyle::default();
        style.color = match key {
            "button" => accent,
            "image" | "list" => [0; 4],
            _ => fill,
        };
        style.text_color = text;
        style.font_size = if key == "heading" { 26.0 } else { 19.0 };
        style.rounding = rounding;
        style.skin.border_color = border;
        style.skin.border_width = if matches!(key, "label" | "heading" | "image" | "list") {
            0.0
        } else {
            width
        };
        style.skin.font = font;
        style.skin.pixelated = matches!(preset, ThemePreset::Pixel);
        style.skin.align = if key == "button" {
            UiTextAlign::Center
        } else {
            UiTextAlign::Left
        };
        screen.styles.insert(key.into(), style);
    }
    for (node, old) in screen.nodes.iter_mut().zip(authored) {
        let key = if node.kind == WidgetKind::Label && old.font_size >= 24.0 {
            "heading"
        } else {
            role(node.kind)
        };
        let skin = old.skin;
        let has_art = !skin.texture.is_empty()
            || !skin.hover_texture.is_empty()
            || !skin.pressed_texture.is_empty()
            || !skin.image_binding.is_empty();
        if has_art {
            let mut style = screen.styles[key].clone();
            style.skin.texture = skin.texture;
            style.skin.hover_texture = skin.hover_texture;
            style.skin.pressed_texture = skin.pressed_texture;
            style.skin.image_binding = skin.image_binding;
            style.skin.uv = skin.uv;
            style.skin.fit = skin.fit;
            style.skin.nine_slice = skin.nine_slice;
            style.skin.tint = skin.tint;
            let key = format!("theme_{key}_{}", node.id);
            screen.styles.insert(key.clone(), style);
            node.style_key = key;
        } else {
            node.style_key = key.into();
        }
    }
}

fn detach_style(node: &mut UiNode, styles: &BTreeMap<String, UiStyle>) {
    if let Some(style) = styles.get(&node.style_key) {
        node.color = style.color;
        node.text_color = style.text_color;
        node.font_size = style.font_size;
        node.rounding = style.rounding;
        node.skin = style.skin.clone();
    }
    node.style_key.clear();
}

pub fn node_controls(
    ui: &mut Ui,
    node: &mut UiNode,
    styles: &mut BTreeMap<String, UiStyle>,
    textures: &mut BTreeMap<String, String>,
    candidates: &[(String, String)],
) {
    theme::section(ui, "Appearance");
    let old = node.style_key.clone();
    let mut options = vec![(String::new(), "Local style".into())];
    options.extend(styles.keys().map(|key| (key.clone(), key.clone())));
    crate::editor::widgets::searchable_combo(ui, "style_key", &mut node.style_key, &options);
    if !old.is_empty() && node.style_key.is_empty() {
        node.style_key = old;
        detach_style(node, styles);
    }
    if node.style_key.is_empty() {
        let mut style = UiStyle::from_node(node);
        style_controls(ui, &mut style, textures, candidates, "local");
        node.color = style.color;
        node.text_color = style.text_color;
        node.font_size = style.font_size;
        node.rounding = style.rounding;
        node.skin = style.skin;
        if ui.small_button("Save as reusable style").clicked() {
            let key = unique_name(&node.name, styles.keys().map(String::as_str));
            styles.insert(key.clone(), UiStyle::from_node(node));
            node.style_key = key;
        }
    } else if let Some(style) = styles.get_mut(&node.style_key) {
        ui.weak("Editing this style updates every widget using it.");
        style_controls(ui, style, textures, candidates, &node.style_key);
    } else {
        ui.colored_label(
            theme::AMBER,
            "Style is missing. Select a style or Local style.",
        );
    }
}

fn color(ui: &mut Ui, label: &str, value: &mut [u8; 4]) {
    ui.horizontal(|ui| {
        ui.color_edit_button_srgba_unmultiplied(value);
        ui.label(label);
    });
}

fn number(ui: &mut Ui, label: &str, value: &mut f32, max: f32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(DragValue::new(value).clamp_range(0.0..=max).speed(0.25));
    });
}

fn texture_picker(
    ui: &mut Ui,
    label: &str,
    value: &mut String,
    textures: &mut BTreeMap<String, String>,
    candidates: &[(String, String)],
) {
    ui.label(label);
    let mut selected = (value.clone(), None::<String>);
    let mut options = vec![((String::new(), None), "None".into())];
    options.extend(
        textures
            .keys()
            .map(|key| ((key.clone(), None), key.clone())),
    );
    for (name, path) in candidates {
        if textures.len() < 64 && !textures.values().any(|registered| registered == path) {
            options.push(((name.clone(), Some(path.clone())), name.clone()));
        }
    }
    if crate::editor::widgets::searchable_combo(ui, label, &mut selected, &options) {
        if let Some(path) = selected.1 {
            let key = unique_name(&selected.0, textures.keys().map(String::as_str));
            textures.insert(key.clone(), path);
            *value = key;
        } else {
            *value = selected.0;
        }
    }
}

fn project_texture_options(
    ui: &mut Ui,
    textures: &mut BTreeMap<String, String>,
    candidates: &[(String, String)],
) -> Option<String> {
    let options: Vec<_> = candidates
        .iter()
        .filter(|(_, path)| {
            textures.len() < 64 || textures.values().any(|registered| registered == path)
        })
        .map(|(name, path)| ((name.clone(), path.clone()), format!("{name} · {path}")))
        .collect();
    let (name, path) = crate::editor::widgets::searchable_action(
        ui,
        "project_texture",
        "Use project texture…",
        &options,
    )?;
    if let Some((key, _)) = textures.iter().find(|(_, registered)| **registered == path) {
        return Some(key.clone());
    }
    let key = unique_name(&name, textures.keys().map(String::as_str));
    textures.insert(key.clone(), path);
    Some(key)
}

fn style_controls(
    ui: &mut Ui,
    style: &mut UiStyle,
    textures: &mut BTreeMap<String, String>,
    candidates: &[(String, String)],
    identity: &str,
) {
    // TextEdit and DragValue use auto IDs in egui 0.23; seed an independent
    // editing session when switching between local and named styles.
    let mut fields = Ui::new(
        ui.ctx().clone(),
        ui.layer_id(),
        ui.make_persistent_id(("style_fields", identity)),
        ui.available_rect_before_wrap(),
        ui.clip_rect(),
    );
    fields.set_style(ui.style().clone());
    fields.set_enabled(ui.is_enabled());
    style_fields(&mut fields, style, textures, candidates);
    ui.allocate_rect(fields.min_rect(), egui::Sense::hover());
}

fn style_fields(
    ui: &mut Ui,
    style: &mut UiStyle,
    textures: &mut BTreeMap<String, String>,
    candidates: &[(String, String)],
) {
    color(ui, "Fill", &mut style.color);
    color(ui, "Text / value", &mut style.text_color);
    ui.horizontal(|ui| {
        ui.label("Text size");
        ui.add(
            DragValue::new(&mut style.font_size)
                .clamp_range(8.0..=160.0)
                .suffix(" px"),
        );
    });
    number(ui, "Corners", &mut style.rounding, 100.0);
    egui::CollapsingHeader::new("Typography & border").show(ui, |ui| {
        crate::editor::widgets::searchable_combo(
            ui,
            "font",
            &mut style.skin.font,
            &[
                (UiFont::Proportional, "Proportional".into()),
                (UiFont::Monospace, "Monospace".into()),
            ],
        );
        ui.horizontal_wrapped(|ui| {
            for (align, label) in [
                (UiTextAlign::Left, "Left"),
                (UiTextAlign::Center, "Center"),
                (UiTextAlign::Right, "Right"),
            ] {
                ui.selectable_value(&mut style.skin.align, align, label);
            }
        });
        number(ui, "Padding", &mut style.skin.padding, 128.0);
        color(ui, "Border", &mut style.skin.border_color);
        number(ui, "Border width", &mut style.skin.border_width, 32.0);
    });
    egui::CollapsingHeader::new("Texture / sprite").show(ui, |ui| {
        texture_picker(ui, "Texture", &mut style.skin.texture, textures, candidates);
        if textures.is_empty() && candidates.is_empty() {
            ui.weak("Select the screen root to import artwork.");
        }
        color(ui, "Tint", &mut style.skin.tint);
        ui.checkbox(&mut style.skin.pixelated, "Pixelated filtering");
        crate::editor::widgets::searchable_combo(
            ui,
            "image_fit",
            &mut style.skin.fit,
            &[
                (ImageFit::Stretch, "Stretch".into()),
                (ImageFit::Contain, "Contain".into()),
                (ImageFit::Cover, "Cover".into()),
                (ImageFit::NineSlice, "Nine-slice".into()),
            ],
        );
        ui.label("Sprite region (UV 0–1)");
        egui::Grid::new("sprite_uv").num_columns(2).show(ui, |ui| {
            for (index, label) in ["Left", "Top", "Right", "Bottom"].into_iter().enumerate() {
                ui.label(label);
                ui.add(
                    DragValue::new(&mut style.skin.uv[index])
                        .clamp_range(0.0..=1.0)
                        .speed(0.01),
                );
                ui.end_row();
            }
        });
        if style.skin.fit == ImageFit::NineSlice {
            ui.label("Preserved borders (source pixels)");
            for (index, label) in ["Left", "Top", "Right", "Bottom"].into_iter().enumerate() {
                number(ui, label, &mut style.skin.nine_slice[index], 4096.0);
            }
        }
        ui.label("Texture data key");
        ui.text_edit_singleline(&mut style.skin.image_binding);
        ui.weak("Read a texture name from data, e.g. portrait or icon.");
    });
    egui::CollapsingHeader::new("Hover & pressed states").show(ui, |ui| {
        texture_picker(
            ui,
            "Hover texture",
            &mut style.skin.hover_texture,
            textures,
            candidates,
        );
        texture_picker(
            ui,
            "Pressed texture",
            &mut style.skin.pressed_texture,
            textures,
            candidates,
        );
        optional_color(ui, "Hover fill", &mut style.skin.hover_color, style.color);
        optional_color(
            ui,
            "Pressed fill",
            &mut style.skin.pressed_color,
            style.color,
        );
        ui.weak("Empty states use the normal artwork.");
    });
}

fn optional_color(ui: &mut Ui, label: &str, value: &mut Option<[u8; 4]>, fallback: [u8; 4]) {
    let mut enabled = value.is_some();
    if ui.checkbox(&mut enabled, label).changed() {
        *value = enabled.then_some(fallback);
    }
    if let Some(value) = value {
        color(ui, label, value);
    }
}

pub fn screen_controls(
    ui: &mut Ui,
    screen: &mut UiScreen,
    state: &mut UiEditorState,
    candidates: &[(String, String)],
) {
    ui.add_space(12.0);
    theme::section(ui, "Screen theme");
    if let Some(preset) = crate::editor::widgets::searchable_action(
        ui,
        "theme_preset",
        "Apply a theme…",
        &[
            (ThemePreset::Modern, "Modern".into()),
            (ThemePreset::Parchment, "Parchment".into()),
            (ThemePreset::Pixel, "Pixel".into()),
            (ThemePreset::Neon, "Neon".into()),
        ],
    ) {
        apply_theme(screen, preset);
    }
    ui.weak("Themes change appearance; layout and data bindings stay intact.");
    if !screen.styles.is_empty() {
        egui::CollapsingHeader::new("Reusable styles").show(ui, |ui| {
            crate::editor::widgets::searchable_combo(
                ui,
                "edit_shared_style",
                &mut state.selected_style,
                &screen
                    .styles
                    .keys()
                    .map(|key| (key.clone(), key.clone()))
                    .collect::<Vec<_>>(),
            );
            if let Some(style) = screen.styles.get_mut(&state.selected_style) {
                style_controls(
                    ui,
                    style,
                    &mut screen.textures,
                    candidates,
                    &state.selected_style,
                );
            }
        });
    }
    ui.add_space(12.0);
    theme::section(ui, "Textures & sprites");
    ui.horizontal_wrapped(|ui| {
        if ui.button("Import artwork…").clicked() {
            state.import_texture = true;
        }
        if ui.button("Reload").clicked() {
            state.texture_sources = None;
            state.texture_discovery.refresh();
        }
    });
    if !candidates.is_empty() {
        project_texture_options(ui, &mut screen.textures, candidates);
    }
    ui.weak("PNG, JPEG, GIF, BMP or TGA. Images are embedded when exporting Rust.");
    let mut remove = None;
    for (name, path) in &screen.textures {
        ui.push_id(name, |ui| {
            ui.horizontal(|ui| {
                ui.label(name).on_hover_text(path);
                if ui
                    .small_button("×")
                    .on_hover_text("Remove reference; keep the source file")
                    .clicked()
                {
                    remove = Some(name.clone());
                }
            });
        });
    }
    if let Some(key) = remove {
        screen.textures.remove(&key);
    }
    ui.weak("Choose a texture in a widget's Texture / sprite section. Use UVs to select a sprite-sheet region.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_theme_preserves_each_widgets_artwork_and_image_binding() {
        let mut screen = UiScreen::dialogue("dialogue");
        screen
            .textures
            .insert("atlas".into(), "textures/atlas.png".into());
        let mut portrait = UiNode::new(6, WidgetKind::Image, "Portrait");
        portrait.skin.texture = "atlas".into();
        portrait.skin.image_binding = "portrait".into();
        portrait.skin.uv = [0.25, 0.0, 0.5, 1.0];
        portrait.skin.fit = ImageFit::Contain;
        screen.nodes.push(portrait);
        for preset in [
            ThemePreset::Parchment,
            ThemePreset::Pixel,
            ThemePreset::Neon,
        ] {
            apply_theme(&mut screen, preset);
            let skin = &screen.styles[&screen.nodes[5].style_key].skin;
            assert_eq!(skin.image_binding, "portrait");
            assert_eq!(skin.texture, "atlas");
            assert_eq!(skin.uv, [0.25, 0.0, 0.5, 1.0]);
            assert_eq!(skin.fit, ImageFit::Contain);
            assert_eq!(
                screen.styles[&screen.nodes[3].style_key].skin.border_width,
                0.0
            );
            screen.validate().unwrap();
        }
    }

    #[test]
    fn themes_preserve_dialogue_bindings_and_allow_distinct_styles() {
        let original = UiScreen::dialogue("dialogue");
        let mut screen = original.clone();
        apply_theme(&mut screen, ThemePreset::Parchment);
        assert_eq!(screen.sample_data, original.sample_data);
        for (node, old) in screen.nodes.iter().zip(&original.nodes) {
            assert_eq!(node.binding, old.binding);
            assert_eq!(node.event, old.event);
            assert_eq!(node.position, old.position);
            assert!(screen.styles.contains_key(&node.style_key));
        }
        let parchment = screen.styles.clone();
        apply_theme(&mut screen, ThemePreset::Pixel);
        assert_ne!(screen.styles, parchment);
        assert_eq!(screen.styles["button"].rounding, 0.0);
        assert!(screen.styles["button"].skin.pixelated);
        screen.validate().unwrap();
    }

    #[test]
    fn detaching_a_shared_style_keeps_its_current_appearance() {
        let mut screen = UiScreen::dialogue("dialogue");
        apply_theme(&mut screen, ThemePreset::Neon);
        let expected = screen.styles[&screen.nodes[0].style_key].clone();
        detach_style(&mut screen.nodes[0], &screen.styles);
        assert!(screen.nodes[0].style_key.is_empty());
        assert_eq!(UiStyle::from_node(&screen.nodes[0]), expected);
    }
}
