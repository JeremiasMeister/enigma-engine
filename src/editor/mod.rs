pub mod state;
pub mod actions;
pub mod panels;
pub mod inspector;

use egui::Context;
use enigma_3d::AppState;

use crate::editor::state::{EditorRoot, Modal, PendingDelete};

pub fn draw(ctx: &Context, app_state: &mut AppState) {
    set_style(ctx);
    crate::editor::actions::poll_job(app_state);
    crate::project::poll_project_load(app_state);
    crate::project::poll_save_job(app_state);
    reconcile_materials(app_state);
    apply_material_assignments(app_state);
    reconcile_skybox(app_state);
    reconcile_particle_preview(app_state);
    reconcile_particle_instances(app_state);
    reconcile_terrain(app_state);

    // Keep repainting while a job is running so the spinner animates and
    // the poll picks up completion promptly.
    let busy = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.job.is_some() || r.editor.project_load.is_some() || r.editor.save_job.is_some())
        .unwrap_or(false);
    if busy {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        panels::toolbar::draw(ui, app_state);
    });

    egui::SidePanel::left("hierarchy")
        .default_width(220.0)
        .min_width(160.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| panels::hierarchy::draw(ui, app_state));
        });

    egui::SidePanel::right("inspector")
        .default_width(320.0)
        .min_width(240.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| panels::inspector::draw(ui, app_state));
        });

    egui::TopBottomPanel::bottom("resource_browser")
        .default_height(220.0)
        .min_height(80.0)
        .resizable(true)
        .show(ctx, |ui| {
            panels::resource_browser::draw(ui, app_state);
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            panels::viewport::draw(ui, app_state);
        });

    process_modals(ctx, app_state);
    draw_job_overlay(ctx, app_state);
}

fn draw_job_overlay(ctx: &Context, app_state: &mut AppState) {
    let (label, elapsed, lines) = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        if let Some(j) = r.editor.job.as_ref() {
            (j.label.clone(), j.started_at.elapsed(), j.lines.clone())
        } else if let Some(j) = r.editor.project_load.as_ref() {
            (j.label.clone(), j.started_at.elapsed(), j.lines.clone())
        } else if let Some(j) = r.editor.save_job.as_ref() {
            (j.label.clone(), j.started_at.elapsed(), j.lines.clone())
        } else {
            return;
        }
    };
    egui::Window::new("cargo")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .default_height(360.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(20.0));
                ui.heading(format!("{label}…"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.monospace(format!("{:.1}s", elapsed.as_secs_f32()));
                });
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if lines.is_empty() {
                        ui.weak("(waiting for cargo output…)");
                    } else {
                        for line in &lines {
                            ui.monospace(line);
                        }
                    }
                });
        });
}

fn process_modals(ctx: &Context, app_state: &mut AppState) {
    let modal = match app_state.get_state_data_value::<EditorRoot>("editor") {
        Some(r) => r.editor.modal.clone(),
        None => return,
    };
    let Some(modal) = modal else { return; };

    let mut close = false;
    egui::Window::new("Dialog")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            match modal {
                Modal::NewSceneName(mut draft) => {
                    ui.label("Scene name:");
                    let response = ui.text_edit_singleline(&mut draft);
                    response.request_focus();
                    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                        r.editor.modal = Some(Modal::NewSceneName(draft.clone()));
                    }
                    ui.horizontal(|ui| {
                        let create = ui.button("Create").clicked()
                            || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                        if create && !draft.trim().is_empty() {
                            if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                                if let Some(proj) = r.project.as_mut() {
                                    if let Err(e) = crate::project::scene::new_scene(proj, draft.trim().to_string()) {
                                        eprintln!("new scene failed: {e:?}");
                                    } else {
                                        r.editor.dirty = true;
                                    }
                                }
                            }
                            close = true;
                        }
                        if ui.button("Cancel").clicked() { close = true; }
                    });
                }
                Modal::ConfirmDelete { label, pending } => {
                    ui.label(format!("Delete {label}?"));
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            apply_pending_delete(app_state, pending);
                            close = true;
                        }
                        if ui.button("Cancel").clicked() { close = true; }
                    });
                }
                Modal::ImportError(msg) => {
                    ui.label(msg);
                    if ui.button("OK").clicked() { close = true; }
                }
            }
        });

    if close {
        if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            r.editor.modal = None;
        }
    }
}

fn apply_pending_delete(app_state: &mut AppState, p: PendingDelete) {
    match p {
        PendingDelete::Resource(uuid) => {
            if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(project) = r.project.as_mut() {
                    let _ = crate::project::resource::delete(project, uuid);
                }
            }
        }
        PendingDelete::Material(uuid) => {
            if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(project) = r.project.as_mut() {
                    project.materials.retain(|m| m.uuid != uuid);
                }
            }
        }
        PendingDelete::Scene(idx) => {
            if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(project) = r.project.as_mut() {
                    let _ = crate::project::scene::delete(project, idx);
                }
            }
        }
        PendingDelete::SceneObject(uuid) => {
            app_state.objects.retain(|o| o.get_unique_id() != uuid);
        }
        PendingDelete::Light(idx) => {
            if idx < app_state.light.len() {
                app_state.light.remove(idx);
            }
        }
        PendingDelete::AmbientLight => {
            app_state.ambient_light = None;
        }
        PendingDelete::Particle(uuid) => {
            if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(project) = r.project.as_mut() {
                    project.particle_systems.retain(|p| p.uuid != uuid);
                    // also drop any instances referencing it
                    for scene in &mut project.scenes {
                        scene.particle_instances.retain(|i| i.def_uuid != uuid);
                    }
                }
            }
            app_state.particle_systems.retain(|s| s.handle != uuid);
        }
        PendingDelete::ParticleInstance(uuid) => {
            if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(project) = r.project.as_mut() {
                    let active = project.active_scene_index;
                    if let Some(scene) = project.scenes.get_mut(active) {
                        scene.particle_instances.retain(|i| i.uuid != uuid);
                    }
                }
            }
            app_state.particle_systems.retain(|s| s.handle != uuid);
        }
    }
    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        r.editor.selection = crate::editor::state::Selection::None;
        r.editor.dirty = true;
    }
}

fn apply_material_assignments(app_state: &mut AppState) {
    let (scene_uuid, default_mat) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = root.project.as_ref() else { return; };
        let Some(scene) = project.scenes.get(project.active_scene_index) else { return; };
        let default_mat = project.materials.first().map(|m| m.uuid);
        (scene.uuid, default_mat)
    };

    // Snapshot all per-object/per-shape assignments up front so we don't
    // re-borrow the editor root inside the object loop.
    let obj_assignments: Vec<(uuid::Uuid, Vec<(usize, uuid::Uuid)>)> = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = root.project.as_ref() else { return; };
        app_state.objects.iter()
            .map(|o| {
                let id = o.get_unique_id();
                (id, project.assignments_for_object(scene_uuid, id))
            })
            .collect()
    };

    for obj in app_state.objects.iter_mut() {
        let obj_uuid = obj.get_unique_id();
        let shape_count = obj.get_shapes().len();
        if shape_count == 0 { continue; }

        let by_shape = obj_assignments.iter().find(|(u, _)| *u == obj_uuid)
            .map(|(_, v)| v.clone()).unwrap_or_default();

        // Existing material slot 0 acts as the per-object fallback for shapes
        // that have no explicit assignment yet.
        let existing_first = obj.get_materials().first().copied();

        let mut new_materials: Vec<uuid::Uuid> = Vec::with_capacity(shape_count);
        for shape_idx in 0..shape_count {
            let chosen = by_shape.iter().find(|(s, _)| *s == shape_idx).map(|(_, m)| *m)
                .or(existing_first)
                .or(default_mat);
            let Some(uuid) = chosen else { break; };
            new_materials.push(uuid);
        }
        if new_materials.len() != shape_count { continue; }

        *obj.get_materials_mut() = new_materials;
        for (i, shape) in obj.get_shapes_mut().iter_mut().enumerate() {
            shape.material_index = i;
        }
    }
}

fn reconcile_particle_preview(app_state: &mut AppState) {
    use crate::editor::state::Selection;

    let (desired_uuid, desired_config) = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = r.project.as_ref() else { return; };
        match &r.editor.selection {
            Selection::Particle(u) => {
                let def = project.particle_systems.iter().find(|p| p.uuid == *u);
                match def {
                    Some(d) => (Some(*u), Some(d.config.clone())),
                    None => (None, None),
                }
            }
            _ => (None, None),
        }
    };

    let applied = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.editor.previewed_particle);

    let new_hash = desired_config.as_ref().map(hash_particle_config);
    let stale = match (desired_uuid, applied, new_hash) {
        (Some(u), Some((au, ah)), Some(h)) => u != au || ah != h,
        (Some(_), None, _) => true,
        (None, Some(_), _) => true,
        _ => false,
    };

    if !stale { return; }

    // Remove any previously previewed particle instance.
    if let Some((au, _)) = applied {
        app_state.particle_systems.retain(|s| s.handle != au);
    }

    if let (Some(uuid), Some(cfg)) = (desired_uuid, desired_config) {
        match enigma_3d::particle::ParticleSystem::from_config(cfg) {
            Ok(mut sys) => {
                sys.handle = uuid;
                app_state.particle_systems.push(sys);
                if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                    let h = r.project.as_ref()
                        .and_then(|p| p.particle_systems.iter().find(|d| d.uuid == uuid))
                        .map(|d| hash_particle_config(&d.config))
                        .unwrap_or(0);
                    r.editor.previewed_particle = Some((uuid, h));
                }
            }
            Err(e) => {
                eprintln!("particle config invalid: {e:?}");
                if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                    r.editor.previewed_particle = None;
                }
            }
        }
    } else {
        if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            r.editor.previewed_particle = None;
        }
    }
}

fn hash_particle_config(cfg: &enigma_3d::particle::ParticleSystemConfig) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    // ParticleSystemConfig isn't Hash, so hash a serialized form.
    if let Ok(json) = serde_json::to_string(cfg) {
        json.hash(&mut h);
    }
    h.finish()
}

fn reconcile_particle_instances(app_state: &mut AppState) {
    // Snapshot all instances in the active scene + def configs.
    let snapshot: Vec<(uuid::Uuid, uuid::Uuid, [f32; 3], u64)> = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = r.project.as_ref() else { return; };
        let Some(scene) = project.scenes.get(project.active_scene_index) else { return; };
        scene.particle_instances.iter().filter_map(|inst| {
            let def = project.particle_systems.iter().find(|d| d.uuid == inst.def_uuid)?;
            let hash = hash_particle_config(&def.config) ^ position_hash(inst.position);
            Some((inst.uuid, inst.def_uuid, inst.position, hash))
        }).collect()
    };

    let live_uuids: std::collections::HashSet<uuid::Uuid> = snapshot.iter().map(|(u, ..)| *u).collect();

    // Drop instances that are no longer in the scene.
    let applied = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        r.editor.applied_particle_instances.clone()
    };
    let stale_to_remove: Vec<uuid::Uuid> = applied.keys()
        .filter(|u| !live_uuids.contains(u))
        .copied()
        .collect();
    for u in &stale_to_remove {
        app_state.particle_systems.retain(|s| s.handle != *u);
    }

    // Build / rebuild instances.
    for (inst_uuid, def_uuid, position, hash) in snapshot {
        let prev_hash = applied.get(&inst_uuid).copied();
        if prev_hash == Some(hash) { continue; }

        let config = {
            let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { continue; };
            let Some(project) = r.project.as_ref() else { continue; };
            project.particle_systems.iter().find(|d| d.uuid == def_uuid).map(|d| d.config.clone())
        };
        let Some(config) = config else { continue; };

        // Remove existing instance with this uuid if any.
        app_state.particle_systems.retain(|s| s.handle != inst_uuid);

        match enigma_3d::particle::ParticleSystem::from_config(config) {
            Ok(mut sys) => {
                sys.handle = inst_uuid;
                let mut m = [
                    [1.0_f32, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [position[0], position[1], position[2], 1.0],
                ];
                sys.transform = m;
                app_state.particle_systems.push(sys);
                if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                    r.editor.applied_particle_instances.insert(inst_uuid, hash);
                }
                let _ = m;
            }
            Err(e) => {
                eprintln!("particle instance build failed: {e:?}");
            }
        }
    }

    // Drop stale entries from the applied map.
    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        r.editor.applied_particle_instances.retain(|k, _| live_uuids.contains(k));
    }
}

fn position_hash(p: [f32; 3]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    p[0].to_bits().hash(&mut h);
    p[1].to_bits().hash(&mut h);
    p[2].to_bits().hash(&mut h);
    h.finish()
}

fn reconcile_terrain(app_state: &mut AppState) {
    let (desired, applied) = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = r.project.as_ref() else { return; };
        let Some(scene) = project.scenes.get(project.active_scene_index) else { return; };
        (scene.terrain.clone(), r.editor.applied_terrain)
    };

    let new_hash = desired.as_ref().map(hash_terrain_def);
    let stale = new_hash != applied;
    if !stale { return; }

    if let Some(def) = desired {
        let Some(display) = app_state.display.clone() else { return; };
        let mut config = enigma_3d::terrain::TerrainConfig::default();
        config.width = def.width;
        config.depth = def.depth;
        config.max_height = def.max_height;
        config.resolution = def.resolution;
        config.tile_count = def.tile_count;
        config.noise_scale = def.noise_scale;
        config.noise_amplitude = def.noise_amplitude;
        config.noise_octaves = def.noise_octaves;
        config.noise_persistence = def.noise_persistence;
        config.color_flat_low = def.color_flat_low;
        config.color_flat_high = def.color_flat_high;
        config.color_slope = def.color_slope;
        config.slope_threshold = def.slope_threshold;
        config.height_mid = def.height_mid;
        config.uv_scale = def.uv_scale;
        config.custom_noise = None;

        // tile_count must divide resolution; clamp gracefully.
        if config.tile_count == 0 || config.resolution % config.tile_count != 0 {
            eprintln!("terrain: resolution {} must be divisible by tile_count {}",
                config.resolution, config.tile_count);
            return;
        }

        let mut terrain = enigma_3d::terrain::Terrain::new(&display, config);
        terrain.set_position(def.position);
        app_state.set_terrain(terrain);
        if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            r.editor.applied_terrain = new_hash;
        }
    } else {
        app_state.terrain = None;
        if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            r.editor.applied_terrain = None;
        }
    }
}

fn hash_terrain_def(def: &crate::editor::state::TerrainDef) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    if let Ok(json) = serde_json::to_string(def) {
        json.hash(&mut h);
    }
    h.finish()
}

fn reconcile_skybox(app_state: &mut AppState) {
    let (desired, applied) = match app_state.get_state_data_value::<EditorRoot>("editor") {
        Some(r) => (
            r.project.as_ref().and_then(|p| p.skybox),
            r.editor.applied_skybox,
        ),
        None => return,
    };
    if desired == applied {
        return;
    }

    if let Some(uuid) = desired {
        let bytes = {
            let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
            let Some(p) = r.project.as_ref() else { return; };
            match crate::project::resource::bytes(p, uuid) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("skybox texture load failed: {e:?}");
                    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                        r.editor.applied_skybox = desired;
                    }
                    return;
                }
            }
        };
        let Some(display) = app_state.display.clone() else { return; };
        let texture = enigma_3d::texture::Texture::from_resource(&display, &bytes);
        apply_skybox_texture(app_state, texture, display);
    } else {
        app_state.skybox = None;
        app_state.skybox_texture = None;
    }

    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        r.editor.applied_skybox = desired;
    }
}

fn apply_skybox_texture(
    app_state: &mut AppState,
    texture: enigma_3d::texture::Texture,
    display: glium::Display<glium::glutin::surface::WindowSurface>,
) {
    use enigma_3d::material::{Material, TextureType};
    use enigma_3d::object::Object;

    let mut material = Material::unlit(display.clone(), false);
    material.set_name("INTERNAL::SkyBox");
    material.set_texture(texture, TextureType::Albedo);
    let mut object = Object::load_from_gltf_resource(enigma_3d::resources::skybox(), None);
    object.add_material(material.uuid);
    object.get_shapes_mut()[0].set_material_from_object_list(0);
    object.name = "Skybox".to_string();
    object.transform.set_scale([1.0, 1.0, 1.0]);
    app_state.add_material(material);
    app_state.set_skybox(object);
}

fn reconcile_materials(app_state: &mut AppState) {
    let project = match app_state.get_state_data_value::<EditorRoot>("editor") {
        Some(r) => r.project.clone(),
        None => return,
    };
    let Some(project) = project else { return; };

    let mut cache = match app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        Some(r) => std::mem::take(&mut r.editor.material_cache),
        None => return,
    };
    let _ = crate::project::material::reconcile(&project, app_state, &mut cache);
    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        r.editor.material_cache = cache;
    }
}

fn set_style(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.window_shadow.extrusion = 0.0;
    style.visuals.window_shadow.color = egui::Color32::TRANSPARENT;
    style.visuals.window_stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
    ctx.set_style(style);
}
