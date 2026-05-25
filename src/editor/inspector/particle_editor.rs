use egui::{DragValue, Ui};
use enigma_3d::AppState;
use enigma_3d::particle::{
    BlendMode, ColorRange, EmitterShape, FlipbookConfig, InitialVelocity, Range, RenderStyle,
};
use uuid::Uuid;

use crate::editor::state::{EditorRoot, ResourceKind};

pub fn draw(ui: &mut Ui, app_state: &mut AppState, uuid: Uuid) {
    let mut def_clone = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = root.project.as_ref() else { return; };
        let Some(def) = project.particle_systems.iter().find(|p| p.uuid == uuid) else {
            ui.label("(particle system not found)");
            return;
        };
        def.clone()
    };

    let materials: Vec<(Uuid, String)> = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| p.materials.iter()
            .filter(|m| !m.name.starts_with("INTERNAL::"))
            .map(|m| (m.uuid, m.name.clone())).collect())
        .unwrap_or_default();
    let textures: Vec<(Uuid, String)> = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.project.as_ref())
        .map(|p| p.manifest.iter()
            .filter(|e| e.kind == ResourceKind::Texture)
            .map(|e| (e.uuid, e.name.clone())).collect())
        .unwrap_or_default();

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("Name");
        changed |= ui.text_edit_singleline(&mut def_clone.config.name).changed();
    });

    ui.horizontal(|ui| {
        ui.label("Texture");
        let current = def_clone.texture
            .and_then(|u| textures.iter().find(|(uu, _)| *uu == u).map(|(_, n)| n.clone()))
            .unwrap_or_else(|| "(none)".to_string());
        egui::ComboBox::from_id_source("particle_texture")
            .selected_text(current)
            .show_ui(ui, |ui| {
                if ui.selectable_label(def_clone.texture.is_none(), "(none)").clicked() {
                    def_clone.texture = None;
                    changed = true;
                }
                for (uuid, name) in &textures {
                    let selected = def_clone.texture == Some(*uuid);
                    if ui.selectable_label(selected, name).clicked() {
                        def_clone.texture = Some(*uuid);
                        changed = true;
                    }
                }
            });
    });

    egui::CollapsingHeader::new("Material override (advanced)").default_open(false).show(ui, |ui| {
        ui.weak("Custom material must use a particle shader, or rendering will crash.");
        ui.horizontal(|ui| {
            ui.label("Material");
            let current = def_clone.material
                .and_then(|u| materials.iter().find(|(uu, _)| *uu == u).map(|(_, n)| n.clone()))
                .unwrap_or_else(|| "(use texture above)".to_string());
            egui::ComboBox::from_id_source("particle_material")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(def_clone.material.is_none(), "(use texture above)").clicked() {
                        def_clone.material = None;
                        changed = true;
                    }
                    for (uuid, name) in &materials {
                        let selected = def_clone.material == Some(*uuid);
                        if ui.selectable_label(selected, name).clicked() {
                            def_clone.material = Some(*uuid);
                            changed = true;
                        }
                    }
                });
        });
    });

    let cfg = &mut def_clone.config;

    egui::CollapsingHeader::new("Emission").default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Max particles");
            changed |= ui.add(DragValue::new(&mut cfg.max_particles).speed(1.0)).changed();
        });
        ui.horizontal(|ui| {
            changed |= ui.checkbox(&mut cfg.looped, "Loop").changed();
            ui.label("Duration");
            changed |= ui.add(DragValue::new(&mut cfg.duration).speed(0.1)).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Emission rate (p/s)");
            changed |= ui.add(DragValue::new(&mut cfg.emission_rate).speed(0.5)).changed();
        });
        changed |= ui.checkbox(&mut cfg.prewarm, "Prewarm").changed();
    });

    egui::CollapsingHeader::new("Emitter Shape").default_open(true).show(ui, |ui| {
        changed |= emitter_shape_editor(ui, &mut cfg.emitter_shape);
    });

    egui::CollapsingHeader::new("Initial Velocity").default_open(true).show(ui, |ui| {
        changed |= initial_velocity_editor(ui, &mut cfg.initial_velocity);
    });

    egui::CollapsingHeader::new("Initial Particle").default_open(true).show(ui, |ui| {
        changed |= range_editor(ui, "Lifetime", &mut cfg.initial_lifetime, 0.1);
        changed |= range_editor(ui, "Size", &mut cfg.initial_size, 0.05);
        changed |= range_editor(ui, "Rotation (rad)", &mut cfg.initial_rotation, 0.1);
        changed |= color_range_editor(ui, "Color", &mut cfg.initial_color);
    });

    egui::CollapsingHeader::new("Forces").default_open(false).show(ui, |ui| {
        let mut has_gravity = cfg.gravity.is_some();
        if ui.checkbox(&mut has_gravity, "Gravity").changed() {
            cfg.gravity = if has_gravity { Some([0.0, -9.81, 0.0]) } else { None };
            changed = true;
        }
        if let Some(g) = cfg.gravity.as_mut() {
            ui.horizontal(|ui| {
                changed |= ui.add(DragValue::new(&mut g[0]).speed(0.1).prefix("x ")).changed();
                changed |= ui.add(DragValue::new(&mut g[1]).speed(0.1).prefix("y ")).changed();
                changed |= ui.add(DragValue::new(&mut g[2]).speed(0.1).prefix("z ")).changed();
            });
        }

        let mut has_drag = cfg.drag.is_some();
        if ui.checkbox(&mut has_drag, "Drag").changed() {
            cfg.drag = if has_drag { Some(0.1) } else { None };
            changed = true;
        }
        if let Some(d) = cfg.drag.as_mut() {
            changed |= ui.add(DragValue::new(d).speed(0.01)).changed();
        }
    });

    egui::CollapsingHeader::new("Render").default_open(true).show(ui, |ui| {
        changed |= render_style_editor(ui, &mut cfg.render);
    });

    if changed {
        if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            if let Some(project) = root.project.as_mut() {
                if let Some(def) = project.particle_systems.iter_mut().find(|p| p.uuid == uuid) {
                    *def = def_clone;
                }
                root.editor.dirty = true;
            }
        }
    }
}

fn range_editor(ui: &mut Ui, label: &str, range: &mut Range<f32>, speed: f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed |= ui.add(DragValue::new(&mut range.min).speed(speed).prefix("min ")).changed();
        changed |= ui.add(DragValue::new(&mut range.max).speed(speed).prefix("max ")).changed();
    });
    changed
}

fn emitter_shape_editor(ui: &mut Ui, shape: &mut EmitterShape) -> bool {
    let mut changed = false;
    let current = match shape {
        EmitterShape::Point => "Point",
        EmitterShape::Sphere { .. } => "Sphere",
        EmitterShape::Box { .. } => "Box",
        EmitterShape::Cone { .. } => "Cone",
        EmitterShape::Disk { .. } => "Disk",
    };
    egui::ComboBox::from_id_source("emitter_shape").selected_text(current).show_ui(ui, |ui| {
        if ui.selectable_label(matches!(shape, EmitterShape::Point), "Point").clicked() {
            *shape = EmitterShape::Point; changed = true;
        }
        if ui.selectable_label(matches!(shape, EmitterShape::Sphere { .. }), "Sphere").clicked() {
            if !matches!(shape, EmitterShape::Sphere { .. }) {
                *shape = EmitterShape::Sphere { radius: 0.5 }; changed = true;
            }
        }
        if ui.selectable_label(matches!(shape, EmitterShape::Box { .. }), "Box").clicked() {
            if !matches!(shape, EmitterShape::Box { .. }) {
                *shape = EmitterShape::Box { half_extents: [0.5, 0.5, 0.5] }; changed = true;
            }
        }
        if ui.selectable_label(matches!(shape, EmitterShape::Cone { .. }), "Cone").clicked() {
            if !matches!(shape, EmitterShape::Cone { .. }) {
                *shape = EmitterShape::Cone { angle: 0.5, height: 1.0 }; changed = true;
            }
        }
        if ui.selectable_label(matches!(shape, EmitterShape::Disk { .. }), "Disk").clicked() {
            if !matches!(shape, EmitterShape::Disk { .. }) {
                *shape = EmitterShape::Disk { radius: 0.5 }; changed = true;
            }
        }
    });

    match shape {
        EmitterShape::Point => {}
        EmitterShape::Sphere { radius } => {
            ui.horizontal(|ui| {
                ui.label("Radius");
                changed |= ui.add(DragValue::new(radius).speed(0.05)).changed();
            });
        }
        EmitterShape::Box { half_extents } => {
            ui.horizontal(|ui| {
                ui.label("Half extents");
                changed |= ui.add(DragValue::new(&mut half_extents[0]).speed(0.05).prefix("x ")).changed();
                changed |= ui.add(DragValue::new(&mut half_extents[1]).speed(0.05).prefix("y ")).changed();
                changed |= ui.add(DragValue::new(&mut half_extents[2]).speed(0.05).prefix("z ")).changed();
            });
        }
        EmitterShape::Cone { angle, height } => {
            ui.horizontal(|ui| {
                ui.label("Angle (rad)");
                changed |= ui.add(DragValue::new(angle).speed(0.05)).changed();
                ui.label("Height");
                changed |= ui.add(DragValue::new(height).speed(0.05)).changed();
            });
        }
        EmitterShape::Disk { radius } => {
            ui.horizontal(|ui| {
                ui.label("Radius");
                changed |= ui.add(DragValue::new(radius).speed(0.05)).changed();
            });
        }
    }
    changed
}

fn initial_velocity_editor(ui: &mut Ui, v: &mut InitialVelocity) -> bool {
    let mut changed = false;
    let current = match v {
        InitialVelocity::Outward { .. } => "Outward",
        InitialVelocity::Direction { .. } => "Direction",
        InitialVelocity::Cone { .. } => "Cone",
        InitialVelocity::Hemisphere { .. } => "Hemisphere",
    };
    egui::ComboBox::from_id_source("init_velocity").selected_text(current).show_ui(ui, |ui| {
        if ui.selectable_label(matches!(v, InitialVelocity::Outward { .. }), "Outward").clicked() {
            if !matches!(v, InitialVelocity::Outward { .. }) {
                *v = InitialVelocity::Outward { speed: Range::new(1.0, 2.0) }; changed = true;
            }
        }
        if ui.selectable_label(matches!(v, InitialVelocity::Direction { .. }), "Direction").clicked() {
            if !matches!(v, InitialVelocity::Direction { .. }) {
                *v = InitialVelocity::Direction { direction: [0.0, 1.0, 0.0], speed: Range::new(1.0, 2.0) }; changed = true;
            }
        }
        if ui.selectable_label(matches!(v, InitialVelocity::Cone { .. }), "Cone").clicked() {
            if !matches!(v, InitialVelocity::Cone { .. }) {
                *v = InitialVelocity::Cone { direction: [0.0, 1.0, 0.0], angle: 0.5, speed: Range::new(1.0, 2.0) }; changed = true;
            }
        }
        if ui.selectable_label(matches!(v, InitialVelocity::Hemisphere { .. }), "Hemisphere").clicked() {
            if !matches!(v, InitialVelocity::Hemisphere { .. }) {
                *v = InitialVelocity::Hemisphere { normal: [0.0, 1.0, 0.0], speed: Range::new(1.0, 2.0) }; changed = true;
            }
        }
    });

    match v {
        InitialVelocity::Outward { speed } => {
            changed |= range_editor(ui, "Speed", speed, 0.1);
        }
        InitialVelocity::Direction { direction, speed } => {
            ui.horizontal(|ui| {
                ui.label("Direction");
                changed |= ui.add(DragValue::new(&mut direction[0]).speed(0.05).prefix("x ")).changed();
                changed |= ui.add(DragValue::new(&mut direction[1]).speed(0.05).prefix("y ")).changed();
                changed |= ui.add(DragValue::new(&mut direction[2]).speed(0.05).prefix("z ")).changed();
            });
            changed |= range_editor(ui, "Speed", speed, 0.1);
        }
        InitialVelocity::Cone { direction, angle, speed } => {
            ui.horizontal(|ui| {
                ui.label("Direction");
                changed |= ui.add(DragValue::new(&mut direction[0]).speed(0.05).prefix("x ")).changed();
                changed |= ui.add(DragValue::new(&mut direction[1]).speed(0.05).prefix("y ")).changed();
                changed |= ui.add(DragValue::new(&mut direction[2]).speed(0.05).prefix("z ")).changed();
            });
            ui.horizontal(|ui| {
                ui.label("Angle (rad)");
                changed |= ui.add(DragValue::new(angle).speed(0.05)).changed();
            });
            changed |= range_editor(ui, "Speed", speed, 0.1);
        }
        InitialVelocity::Hemisphere { normal, speed } => {
            ui.horizontal(|ui| {
                ui.label("Normal");
                changed |= ui.add(DragValue::new(&mut normal[0]).speed(0.05).prefix("x ")).changed();
                changed |= ui.add(DragValue::new(&mut normal[1]).speed(0.05).prefix("y ")).changed();
                changed |= ui.add(DragValue::new(&mut normal[2]).speed(0.05).prefix("z ")).changed();
            });
            changed |= range_editor(ui, "Speed", speed, 0.1);
        }
    }
    changed
}

fn color_range_editor(ui: &mut Ui, label: &str, c: &mut ColorRange) -> bool {
    let mut changed = false;
    let current = match c {
        ColorRange::Single(_) => "Single",
        ColorRange::Range { .. } => "Range",
        ColorRange::Hdr { .. } => "HDR",
        ColorRange::HdrRange { .. } => "HDR Range",
    };
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_source("color_range").selected_text(current).show_ui(ui, |ui| {
            if ui.selectable_label(matches!(c, ColorRange::Single(_)), "Single").clicked() {
                if !matches!(c, ColorRange::Single(_)) {
                    *c = ColorRange::Single([1.0, 1.0, 1.0, 1.0]); changed = true;
                }
            }
            if ui.selectable_label(matches!(c, ColorRange::Range { .. }), "Range").clicked() {
                if !matches!(c, ColorRange::Range { .. }) {
                    *c = ColorRange::Range { min: [1.0; 4], max: [1.0; 4] }; changed = true;
                }
            }
            if ui.selectable_label(matches!(c, ColorRange::Hdr { .. }), "HDR").clicked() {
                if !matches!(c, ColorRange::Hdr { .. }) {
                    *c = ColorRange::Hdr { rgb: [1.0; 3], intensity: 1.0, alpha: 1.0 }; changed = true;
                }
            }
        });
    });
    match c {
        ColorRange::Single(col) => {
            let mut rgb = [col[0], col[1], col[2]];
            if ui.color_edit_button_rgb(&mut rgb).changed() {
                col[0] = rgb[0]; col[1] = rgb[1]; col[2] = rgb[2]; changed = true;
            }
            changed |= ui.add(DragValue::new(&mut col[3]).speed(0.01).prefix("a ")).changed();
        }
        ColorRange::Range { min, max } => {
            let mut min_rgb = [min[0], min[1], min[2]];
            let mut max_rgb = [max[0], max[1], max[2]];
            ui.horizontal(|ui| {
                ui.label("min");
                if ui.color_edit_button_rgb(&mut min_rgb).changed() {
                    min[0] = min_rgb[0]; min[1] = min_rgb[1]; min[2] = min_rgb[2]; changed = true;
                }
                changed |= ui.add(DragValue::new(&mut min[3]).speed(0.01).prefix("a ")).changed();
            });
            ui.horizontal(|ui| {
                ui.label("max");
                if ui.color_edit_button_rgb(&mut max_rgb).changed() {
                    max[0] = max_rgb[0]; max[1] = max_rgb[1]; max[2] = max_rgb[2]; changed = true;
                }
                changed |= ui.add(DragValue::new(&mut max[3]).speed(0.01).prefix("a ")).changed();
            });
        }
        ColorRange::Hdr { rgb, intensity, alpha } => {
            changed |= ui.color_edit_button_rgb(rgb).changed();
            changed |= ui.add(DragValue::new(intensity).speed(0.05).prefix("intensity ")).changed();
            changed |= ui.add(DragValue::new(alpha).speed(0.01).prefix("a ")).changed();
        }
        ColorRange::HdrRange { .. } => {
            ui.label("(HDR Range — edit JSON for now)");
        }
    }
    changed
}

fn render_style_editor(ui: &mut Ui, r: &mut RenderStyle) -> bool {
    let mut changed = false;
    let current = match r {
        RenderStyle::Sprite { .. } => "Sprite",
        RenderStyle::Ribbon { .. } => "Ribbon",
    };
    ui.horizontal(|ui| {
        ui.label("Style");
        egui::ComboBox::from_id_source("render_style").selected_text(current).show_ui(ui, |ui| {
            if ui.selectable_label(matches!(r, RenderStyle::Sprite { .. }), "Sprite").clicked() {
                if !matches!(r, RenderStyle::Sprite { .. }) {
                    *r = RenderStyle::Sprite {
                        flipbook: None,
                        blend_mode: BlendMode::Alpha,
                        soft_particles: false,
                        soft_fade_distance: 0.0,
                        velocity_stretch: 0.0,
                    };
                    changed = true;
                }
            }
        });
    });

    if let RenderStyle::Sprite { flipbook, blend_mode, soft_particles, soft_fade_distance, velocity_stretch } = r {
        let blend_label = match blend_mode {
            BlendMode::Additive => "Additive",
            BlendMode::Alpha => "Alpha",
            BlendMode::PremultipliedAlpha => "Premultiplied Alpha",
        };
        ui.horizontal(|ui| {
            ui.label("Blend");
            egui::ComboBox::from_id_source("blend_mode").selected_text(blend_label).show_ui(ui, |ui| {
                if ui.selectable_label(matches!(blend_mode, BlendMode::Additive), "Additive").clicked() {
                    *blend_mode = BlendMode::Additive; changed = true;
                }
                if ui.selectable_label(matches!(blend_mode, BlendMode::Alpha), "Alpha").clicked() {
                    *blend_mode = BlendMode::Alpha; changed = true;
                }
                if ui.selectable_label(matches!(blend_mode, BlendMode::PremultipliedAlpha), "Premultiplied").clicked() {
                    *blend_mode = BlendMode::PremultipliedAlpha; changed = true;
                }
            });
        });
        changed |= ui.checkbox(soft_particles, "Soft particles").changed();
        if *soft_particles {
            changed |= ui.add(DragValue::new(soft_fade_distance).speed(0.05).prefix("fade ")).changed();
        }
        changed |= ui.add(DragValue::new(velocity_stretch).speed(0.05).prefix("velocity stretch ")).changed();

        let mut has_flipbook = flipbook.is_some();
        if ui.checkbox(&mut has_flipbook, "Flipbook (sprite sheet)").changed() {
            *flipbook = if has_flipbook {
                Some(FlipbookConfig {
                    cols: 4,
                    rows: 4,
                    frame_count: 16,
                    fps: 10.0,
                    blend: false,
                    randomize_start_frame: false,
                    fixed_frame: None,
                })
            } else {
                None
            };
            changed = true;
        }
        if let Some(fb) = flipbook.as_mut() {
            ui.indent("flipbook_inner", |ui| {
                ui.horizontal(|ui| {
                    changed |= ui.add(DragValue::new(&mut fb.cols).speed(1.0).prefix("cols ")).changed();
                    changed |= ui.add(DragValue::new(&mut fb.rows).speed(1.0).prefix("rows ")).changed();
                });
                changed |= ui.add(DragValue::new(&mut fb.frame_count).speed(1.0).prefix("frame count ")).changed();
                changed |= ui.add(DragValue::new(&mut fb.fps).speed(0.5).prefix("fps ")).changed();
                changed |= ui.checkbox(&mut fb.blend, "Blend frames").changed();
                changed |= ui.checkbox(&mut fb.randomize_start_frame, "Random start frame").changed();
                let mut has_fixed = fb.fixed_frame.is_some();
                if ui.checkbox(&mut has_fixed, "Lock to single frame").changed() {
                    fb.fixed_frame = if has_fixed { Some(0) } else { None };
                    changed = true;
                }
                if let Some(f) = fb.fixed_frame.as_mut() {
                    changed |= ui.add(DragValue::new(f).speed(1.0).prefix("frame ")).changed();
                }
            });
        }
    }
    changed
}
