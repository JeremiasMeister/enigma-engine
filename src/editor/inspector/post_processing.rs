use crate::editor::state::{EditorRoot, ResourceKind};
use crate::editor::widgets;
use egui::Ui;
use enigma_3d::postprocessing::stack::{Parameter, ParameterValue};
use enigma_3d::AppState;

pub fn draw(ui: &mut Ui, app: &mut AppState) {
    ui.separator();
    ui.heading("Post Processing");
    ui.weak("Effects run from top to bottom.");
    let textures = app
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| {
            p.manifest
                .iter()
                .filter(|e| e.kind == ResourceKind::Texture)
                .map(|e| (e.uuid, e.name.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let choices: Vec<_> = app
        .post_processing_registry
        .definitions()
        .map(|d| (d.id.clone(), d.name.clone()))
        .collect();
    let mut changed = false;
    if let Some(id) = widgets::search_menu(ui, "+ Add effect", &choices) {
        if let Some(def) = app.post_processing_registry.get(&id) {
            app.post_processing.entries.push(def.entry());
            changed = true;
        }
    }
    if app.post_processing.entries.is_empty() {
        ui.weak("No effects. Add one to preview it in the scene.");
    }
    let mut remove = None;
    let mut swap = None;
    let mut retry = None;
    let mut image_pick = None;
    let mut image_error = None;
    let count = app.post_processing.entries.len();
    // Split runtime diagnostics from the editable scene data to keep UI edits local.
    let errors: Vec<_> = app
        .post_processing
        .entries
        .iter()
        .map(|e| app.post_processing.error(&e.id).map(str::to_owned))
        .collect();
    for (index, entry) in app.post_processing.entries.iter_mut().enumerate() {
        let definition = app.post_processing_registry.get(&entry.effect);
        let name = definition.map(|d| d.name.as_str()).unwrap_or(&entry.effect);
        ui.push_id(&entry.id, |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                changed |= ui.checkbox(&mut entry.enabled, name).changed();
                if ui
                    .small_button("×")
                    .on_hover_text("Remove effect")
                    .clicked()
                {
                    remove = Some(index);
                }
                if ui
                    .add_enabled(index > 0, egui::Button::new("↑").small())
                    .on_hover_text("Move earlier")
                    .clicked()
                {
                    swap = Some((index, index - 1));
                }
                if ui
                    .add_enabled(index + 1 < count, egui::Button::new("↓").small())
                    .on_hover_text("Move later")
                    .clicked()
                {
                    swap = Some((index, index + 1));
                }
            });
            if let Some(definition) = definition {
                egui::CollapsingHeader::new("Parameters")
                    .default_open(true)
                    .show(ui, |ui| {
                        if definition.parameters.is_empty() {
                            ui.weak("This effect has no parameters.");
                        }
                        for field in &definition.parameters {
                            ui.push_id(&field.name, |ui| {
                                if let Some(value) = entry.parameters.get_mut(&field.name) {
                                    if let ParameterValue::Image { name, .. } = value {
                                        ui.label(&field.label);
                                        ui.weak(name.as_str());
                                        if let Some(uuid) = widgets::search_menu(
                                            ui,
                                            "Choose project texture…",
                                            &textures,
                                        ) {
                                            image_pick = Some((index, field.name.clone(), uuid));
                                        }
                                    } else {
                                        changed |= parameter(ui, field, value);
                                    }
                                } else {
                                    ui.colored_label(
                                        egui::Color32::YELLOW,
                                        format!("Missing {}", field.label),
                                    );
                                }
                            });
                        }
                        if ui.small_button("Reset parameters").clicked() {
                            entry.parameters = definition.entry().parameters;
                            changed = true;
                        }
                    });
            } else {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "Custom effect unavailable in this editor build.",
                );
                ui.weak("Its settings are preserved. Load project effects below.");
            }
            if let Some(error) = &errors[index] {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
                if ui.small_button("Retry").clicked() {
                    retry = Some(entry.id.clone());
                }
            }
        });
    }
    if let Some((index, name, uuid)) = image_pick {
        if let Some(project) = app
            .get_state_data_value::<EditorRoot>("editor")
            .and_then(|r| r.project.as_ref())
        {
            match crate::project::resource::bytes(project, uuid) {
                Ok(bytes) => match image::load_from_memory(&bytes) {
                    Ok(_) => {
                        let label = textures
                            .iter()
                            .find(|(id, _)| *id == uuid)
                            .map(|(_, name)| name.clone())
                            .unwrap_or_default();
                        app.post_processing.entries[index]
                            .parameters
                            .insert(name, ParameterValue::Image { name: label, bytes });
                        changed = true;
                    }
                    Err(e) => image_error = Some(format!("Cannot load texture: {e}")),
                },
                Err(e) => image_error = Some(format!("Cannot read texture: {e:?}")),
            }
        }
    }
    if let Some(index) = remove {
        app.post_processing.entries.remove(index);
        changed = true;
    } else if let Some((a, b)) = swap {
        app.post_processing.entries.swap(a, b);
        changed = true;
    }
    if let Some(id) = retry {
        app.post_processing.retry(&id);
    }
    if let Some(root) = app.get_state_data_value_mut::<EditorRoot>("editor") {
        if changed {
            root.editor.dirty = true;
            // The existing revision guard also protects effect edits made during an async save.
            root.editor.ui_revision = root.editor.ui_revision.wrapping_add(1);
        }
        if let Some(error) = image_error {
            root.editor.modal = Some(crate::editor::state::Modal::ImportError(error));
        }
    }
    ui.separator();
    crate::project::post_processing::draw_project_controls(ui, app);
}

fn parameter(ui: &mut Ui, field: &Parameter, value: &mut ParameterValue) -> bool {
    ui.label(&field.label);
    match value {
        ParameterValue::Float(v) => {
            let mut control = egui::DragValue::new(v).speed(0.01);
            if let Some((min, max)) = field.range {
                control = control.clamp_range(min..=max);
            }
            ui.add(control).changed()
        }
        ParameterValue::Int(v) => {
            let mut control = egui::DragValue::new(v).speed(1.0);
            if let Some((min, max)) = field.range {
                control = control.clamp_range(min..=max);
            }
            ui.add(control).changed()
        }
        ParameterValue::Bool(v) => ui.checkbox(v, "Enabled").changed(),
        ParameterValue::Vec2(v) => {
            let mut changed = false;
            ui.horizontal(|ui| {
                for (axis, value) in v.iter_mut().enumerate() {
                    ui.push_id(axis, |ui| {
                        let mut control = egui::DragValue::new(value).speed(1.0);
                        if let Some((min, max)) = field.range {
                            control = control.clamp_range(min..=max);
                        }
                        changed |= ui.add(control).changed();
                    });
                }
            });
            changed
        }
        ParameterValue::Color(v) => ui.color_edit_button_rgb(v).changed(),
        ParameterValue::Text(v) => ui.text_edit_singleline(v).changed(),
        ParameterValue::Image { .. } => false,
    }
}
