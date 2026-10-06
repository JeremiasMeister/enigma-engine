use egui::{DragValue, Ui};
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::{EditorRoot, TerrainDef};
use crate::editor::widgets;

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let mut def_clone: Option<TerrainDef> = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = root.project.as_ref() else { return; };
        let scene = project.scenes.get(project.active_scene_index);
        scene.and_then(|s| s.terrain.clone())
    };

    let mut changed = false;
    let mut clear_terrain = false;
    let mut enable_terrain = false;

    match def_clone.as_mut() {
        None => {
            ui.label("No terrain in this scene.");
            if ui.button("+ Add Terrain").clicked() {
                enable_terrain = true;
            }
        }
        Some(def) => {
            ui.horizontal(|ui| {
                ui.heading("Terrain");
                if ui.small_button("Remove").on_hover_text("Remove terrain from scene").clicked() {
                    clear_terrain = true;
                }
            });

            egui::CollapsingHeader::new("Position").default_open(true).show(ui, |ui| {
                changed |= widgets::vector3(ui, "Position", &mut def.position, 0.1);
            });

            egui::CollapsingHeader::new("Size").default_open(true).show(ui, |ui| {
                changed |= ui.add(DragValue::new(&mut def.width).speed(1.0).prefix("width ")).changed();
                changed |= ui.add(DragValue::new(&mut def.depth).speed(1.0).prefix("depth ")).changed();
                changed |= ui.add(DragValue::new(&mut def.max_height).speed(0.5).prefix("max height ")).changed();
            });

            egui::CollapsingHeader::new("Mesh").default_open(false).show(ui, |ui| {
                changed |= ui.add(DragValue::new(&mut def.resolution).speed(1.0).prefix("resolution ")).changed();
                changed |= ui.add(DragValue::new(&mut def.tile_count).speed(1.0).prefix("tiles per side ")).changed();
                ui.weak("resolution must be divisible by tile count");
            });

            egui::CollapsingHeader::new("Noise").default_open(true).show(ui, |ui| {
                changed |= widgets::slider(ui, "Noise scale", &mut def.noise_scale, 0.001..=0.5);
                changed |= widgets::slider(ui, "Amplitude", &mut def.noise_amplitude, 0.0..=2.0);
                changed |= ui.add(DragValue::new(&mut def.noise_octaves).speed(1.0).prefix("octaves ")).changed();
                changed |= widgets::slider(ui, "Persistence", &mut def.noise_persistence, 0.0..=1.0);
            });

            egui::CollapsingHeader::new("Material").default_open(true).show(ui, |ui| {
                let materials: Vec<(Uuid, String)> = app_state
                    .get_state_data_value::<EditorRoot>("editor")
                    .and_then(|r| r.project.as_ref())
                    .map(|p| p.materials.iter().map(|m| (m.uuid, m.name.clone())).collect())
                    .unwrap_or_default();
                changed |= widgets::asset_picker(ui, "Material", &mut def.material, &materials, "Vertex colors");
                ui.weak("when set, the terrain shader samples the material's albedo");
            });

            egui::CollapsingHeader::new("Colors").default_open(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Flat low");
                    changed |= ui.color_edit_button_rgb(&mut def.color_flat_low).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Flat high");
                    changed |= ui.color_edit_button_rgb(&mut def.color_flat_high).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Slope");
                    changed |= ui.color_edit_button_rgb(&mut def.color_slope).changed();
                });
                changed |= widgets::slider(ui, "Slope threshold", &mut def.slope_threshold, 0.0..=1.0);
                changed |= widgets::slider(ui, "Height blend", &mut def.height_mid, 0.0..=1.0);
                changed |= widgets::slider(ui, "Texture scale", &mut def.uv_scale, 0.1..=50.0);
            });
        }
    }

    if changed || clear_terrain || enable_terrain {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = root.project.as_mut() {
                let active_index = project.active_scene_index;
                if let Some(scene) = project.scenes.get_mut(active_index) {
                    if enable_terrain {
                        scene.terrain = Some(TerrainDef::new_default());
                    } else if clear_terrain {
                        scene.terrain = None;
                    } else if let Some(updated) = def_clone {
                        scene.terrain = Some(updated);
                    }
                }
                root.editor.dirty = true;
            }
        }
    }
}
