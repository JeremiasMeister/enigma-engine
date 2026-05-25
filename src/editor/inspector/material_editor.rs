use egui::{DragValue, Ui};
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::{EditorRoot, ResourceKind, ShaderChoice};

pub fn draw(ui: &mut Ui, app_state: &mut AppState, material_uuid: Uuid) {
    let (mut def_clone, textures) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = root.project.as_ref() else { return; };
        let Some(d) = project.materials.iter().find(|m| m.uuid == material_uuid) else {
            ui.label("(material not found)");
            return;
        };
        let textures: Vec<(Uuid, String)> = project.manifest.iter()
            .filter(|e| e.kind == ResourceKind::Texture)
            .map(|e| (e.uuid, e.name.clone()))
            .collect();
        (d.clone(), textures)
    };

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("Name");
        changed |= ui.text_edit_singleline(&mut def_clone.name).changed();
    });

    let shaders: Vec<(Uuid, String)> = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| p.manifest.iter()
            .filter(|e| e.kind == ResourceKind::Shader)
            .map(|e| (e.uuid, e.name.clone()))
            .collect())
        .unwrap_or_default();

    ui.horizontal(|ui| {
        ui.label("Shader");
        let label = match def_clone.shader {
            ShaderChoice::PbrLit => "PBR Lit",
            ShaderChoice::Unlit => "Unlit",
            ShaderChoice::Custom { .. } => "Custom",
        };
        egui::ComboBox::from_id_source("material_shader")
            .selected_text(label)
            .show_ui(ui, |ui| {
                if ui.selectable_label(matches!(def_clone.shader, ShaderChoice::PbrLit), "PBR Lit").clicked() {
                    def_clone.shader = ShaderChoice::PbrLit;
                    changed = true;
                }
                if ui.selectable_label(matches!(def_clone.shader, ShaderChoice::Unlit), "Unlit").clicked() {
                    def_clone.shader = ShaderChoice::Unlit;
                    changed = true;
                }
                if ui.selectable_label(matches!(def_clone.shader, ShaderChoice::Custom { .. }), "Custom").clicked() {
                    if !matches!(def_clone.shader, ShaderChoice::Custom { .. }) {
                        def_clone.shader = ShaderChoice::Custom {
                            vertex: None,
                            fragment: None,
                            geometry: None,
                        };
                        changed = true;
                    }
                }
            });
    });

    if let ShaderChoice::Custom { vertex, fragment, geometry } = &mut def_clone.shader {
        ui.label("Custom shader sources (defaults used if empty)");
        changed |= shader_slot(ui, "Vertex", vertex, &shaders);
        changed |= shader_slot(ui, "Fragment", fragment, &shaders);
        changed |= shader_slot(ui, "Geometry", geometry, &shaders);
    }

    ui.separator();
    ui.label("Texture slots");
    changed |= texture_slot(ui, "Albedo", &mut def_clone.albedo, &textures);
    if matches!(def_clone.shader, ShaderChoice::PbrLit) {
        changed |= texture_slot(ui, "Normal", &mut def_clone.normal, &textures);
        changed |= texture_slot(ui, "Roughness", &mut def_clone.roughness, &textures);
        changed |= texture_slot(ui, "Metallic", &mut def_clone.metallic, &textures);
    }
    changed |= texture_slot(ui, "Emissive", &mut def_clone.emissive, &textures);

    ui.separator();
    ui.label("UV Tiling / Offset");
    ui.horizontal(|ui| {
        ui.label("Tiling");
        changed |= ui.add(DragValue::new(&mut def_clone.uv_tiling[0]).speed(0.05).prefix("u ")).changed();
        changed |= ui.add(DragValue::new(&mut def_clone.uv_tiling[1]).speed(0.05).prefix("v ")).changed();
    });
    ui.horizontal(|ui| {
        ui.label("Offset");
        changed |= ui.add(DragValue::new(&mut def_clone.uv_offset[0]).speed(0.01).prefix("u ")).changed();
        changed |= ui.add(DragValue::new(&mut def_clone.uv_offset[1]).speed(0.01).prefix("v ")).changed();
    });

    ui.separator();
    ui.label("Parameters");
    ui.horizontal(|ui| {
        ui.label("Color");
        changed |= ui.color_edit_button_rgb(&mut def_clone.color).changed();
    });
    changed |= ui.add(DragValue::new(&mut def_clone.emissive_strength).speed(0.05).prefix("emissive ")).changed();
    changed |= ui.add(DragValue::new(&mut def_clone.roughness_strength).speed(0.01).prefix("roughness ")).changed();
    changed |= ui.add(DragValue::new(&mut def_clone.metallic_strength).speed(0.01).prefix("metallic ")).changed();
    changed |= ui.add(DragValue::new(&mut def_clone.normal_strength).speed(0.01).prefix("normal ")).changed();
    changed |= ui.checkbox(&mut def_clone.transparent, "Transparent").changed();
    if def_clone.transparent {
        changed |= ui.add(DragValue::new(&mut def_clone.transparency_strength).speed(0.01).prefix("transparency ")).changed();
    }

    if changed {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = root.project.as_mut() {
                if let Some(d) = project.materials.iter_mut().find(|m| m.uuid == material_uuid) {
                    *d = def_clone;
                }
                root.editor.dirty = true;
            }
        }
    }
}

fn shader_slot(
    ui: &mut Ui,
    label: &str,
    slot: &mut Option<Uuid>,
    shaders: &[(Uuid, String)],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let current = slot
            .and_then(|u| shaders.iter().find(|(uu, _)| *uu == u).map(|(_, n)| n.clone()))
            .unwrap_or_else(|| "(default)".into());
        egui::ComboBox::from_id_source(format!("shader-slot-{label}"))
            .selected_text(current)
            .show_ui(ui, |ui| {
                if ui.selectable_label(slot.is_none(), "(default)").clicked() {
                    *slot = None;
                    changed = true;
                }
                for (uuid, name) in shaders {
                    let is = *slot == Some(*uuid);
                    if ui.selectable_label(is, name).clicked() {
                        *slot = Some(*uuid);
                        changed = true;
                    }
                }
            });
    });
    changed
}

fn texture_slot(
    ui: &mut Ui,
    label: &str,
    slot: &mut Option<Uuid>,
    textures: &[(Uuid, String)],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let current = slot
            .and_then(|u| textures.iter().find(|(uu, _)| *uu == u).map(|(_, n)| n.clone()))
            .unwrap_or_else(|| "(none)".into());
        egui::ComboBox::from_id_source(format!("slot-{label}"))
            .selected_text(current)
            .show_ui(ui, |ui| {
                if ui.selectable_label(slot.is_none(), "(none)").clicked() {
                    *slot = None;
                    changed = true;
                }
                for (uuid, name) in textures {
                    let is = *slot == Some(*uuid);
                    if ui.selectable_label(is, name).clicked() {
                        *slot = Some(*uuid);
                        changed = true;
                    }
                }
            });
    });
    changed
}
