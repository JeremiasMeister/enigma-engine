use egui::{DragValue, Ui};
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::{EditorRoot, ResourceKind, ShaderChoice};
use crate::editor::{theme, widgets};

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
        if let Some(choice) = widgets::enum_picker(ui, "material_shader", label, &["PBR Lit", "Unlit", "Custom"]) {
            def_clone.shader = match choice {
                "PBR Lit" => ShaderChoice::PbrLit,
                "Unlit" => ShaderChoice::Unlit,
                _ => ShaderChoice::Custom { vertex: None, fragment: None, geometry: None },
            };
            changed = true;
        }
    });

    if let ShaderChoice::Custom { vertex, fragment, geometry } = &mut def_clone.shader {
        ui.label("Custom shader sources (defaults used if empty)");
        changed |= shader_slot(ui, "Vertex", vertex, &shaders);
        changed |= shader_slot(ui, "Fragment", fragment, &shaders);
        changed |= shader_slot(ui, "Geometry", geometry, &shaders);
    }

    ui.separator();
    theme::section(ui, "Texture slots");
    changed |= texture_slot(ui, "Albedo", &mut def_clone.albedo, &textures);
    if matches!(def_clone.shader, ShaderChoice::PbrLit) {
        changed |= texture_slot(ui, "Normal", &mut def_clone.normal, &textures);
        changed |= texture_slot(ui, "Roughness", &mut def_clone.roughness, &textures);
        changed |= texture_slot(ui, "Metallic", &mut def_clone.metallic, &textures);
    }
    changed |= texture_slot(ui, "Emissive", &mut def_clone.emissive, &textures);

    ui.separator();
    theme::section(ui, "Texture coordinates");
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
    theme::section(ui, "Parameters");
    ui.horizontal(|ui| {
        ui.label("Color");
        changed |= ui.color_edit_button_rgb(&mut def_clone.color).changed();
    });
    changed |= widgets::slider(ui, "Emissive", &mut def_clone.emissive_strength, 0.0..=10.0);
    changed |= widgets::slider(ui, "Roughness", &mut def_clone.roughness_strength, 0.0..=1.0);
    changed |= widgets::slider(ui, "Metallic", &mut def_clone.metallic_strength, 0.0..=1.0);
    changed |= widgets::slider(ui, "Normal strength", &mut def_clone.normal_strength, 0.0..=2.0);
    changed |= ui.checkbox(&mut def_clone.transparent, "Transparent").changed();
    if def_clone.transparent {
        changed |= widgets::slider(ui, "Transparency", &mut def_clone.transparency_strength, 0.0..=1.0);
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
    widgets::asset_picker(ui, label, slot, shaders, "Default shader")
}

fn texture_slot(
    ui: &mut Ui,
    label: &str,
    slot: &mut Option<Uuid>,
    textures: &[(Uuid, String)],
) -> bool {
    widgets::asset_picker(ui, label, slot, textures, "None")
}
