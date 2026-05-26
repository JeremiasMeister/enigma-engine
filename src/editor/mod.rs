pub mod state;
pub mod actions;
pub mod panels;
pub mod inspector;
pub mod gizmo;

use std::collections::HashMap;

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
    ensure_internal_particle_materials(app_state);
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

fn ensure_internal_particle_materials(app_state: &mut AppState) {
    // Self-heal: if the cached uuid isn't actually present in app_state.materials
    // (e.g. clear_scene wiped it on scene switch), treat it as missing.
    let (needs_sprite, needs_ribbon) = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let sprite_present = r.editor.internal_particle_sprite_material
            .map(|u| app_state.materials.iter().any(|m| m.uuid == u))
            .unwrap_or(false);
        let ribbon_present = r.editor.internal_particle_ribbon_material
            .map(|u| app_state.materials.iter().any(|m| m.uuid == u))
            .unwrap_or(false);
        (!sprite_present, !ribbon_present)
    };
    if !needs_sprite && !needs_ribbon { return; }
    let Some(display) = app_state.display.clone() else { return; };

    // Clear stored uuid before rebuild so the new uuid is recorded.
    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        if needs_sprite { r.editor.internal_particle_sprite_material = None; }
        if needs_ribbon { r.editor.internal_particle_ribbon_material = None; }
    }

    if needs_sprite {
        let mut mat = enigma_3d::material::Material::particle_sprite(&display);
        mat.set_name("INTERNAL::ParticleSprite");
        let uuid = mat.uuid;
        app_state.materials.push(mat);
        if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            r.editor.internal_particle_sprite_material = Some(uuid);
        }
    }
    if needs_ribbon {
        let mut mat = enigma_3d::material::Material::particle_ribbon(&display);
        mat.set_name("INTERNAL::ParticleRibbon");
        let uuid = mat.uuid;
        app_state.materials.push(mat);
        if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
            r.editor.internal_particle_ribbon_material = Some(uuid);
        }
    }
}

fn default_particle_material(app_state: &AppState, render: &enigma_3d::particle::RenderStyle) -> Option<uuid::Uuid> {
    let r = app_state.get_state_data_value::<EditorRoot>("editor")?;
    match render {
        enigma_3d::particle::RenderStyle::Sprite { .. } => r.editor.internal_particle_sprite_material,
        enigma_3d::particle::RenderStyle::Ribbon { .. } => r.editor.internal_particle_ribbon_material,
    }
}

/// If the particle def has a `texture` set, build (or update) a per-def
/// particle material with that texture as the albedo. Returns the uuid
/// of the per-def material if texture is present and could be loaded,
/// else None (caller falls back to the shared built-in).
fn ensure_per_def_particle_material(
    app_state: &mut AppState,
    def_uuid: uuid::Uuid,
) -> Option<uuid::Uuid> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let (texture_uuid, render_style, project_root_path) = {
        let r = app_state.get_state_data_value::<EditorRoot>("editor")?;
        let project = r.project.as_ref()?;
        let def = project.particle_systems.iter().find(|d| d.uuid == def_uuid)?;
        let tex = def.texture?;
        (tex, def.config.render.clone(), project.root_path.clone())
    };
    let _ = project_root_path;

    let cached = app_state.get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.editor.per_def_particle_materials.get(&def_uuid).copied());

    let bytes = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return None; };
        let Some(project) = r.project.as_ref() else { return None; };
        match crate::project::resource::bytes(project, texture_uuid) {
            Ok(b) => b,
            Err(_) => return None,
        }
    };

    let mut h = DefaultHasher::new();
    texture_uuid.hash(&mut h);
    matches!(render_style, enigma_3d::particle::RenderStyle::Ribbon { .. }).hash(&mut h);
    let new_hash = h.finish();

    if let Some((mat_uuid, applied_hash)) = cached {
        if applied_hash == new_hash && app_state.materials.iter().any(|m| m.uuid == mat_uuid) {
            return Some(mat_uuid);
        }
        // Stale → drop and rebuild.
        app_state.materials.retain(|m| m.uuid != mat_uuid);
    }

    let Some(display) = app_state.display.clone() else { return None; };
    let mut mat = match render_style {
        enigma_3d::particle::RenderStyle::Sprite { .. } => enigma_3d::material::Material::particle_sprite(&display),
        enigma_3d::particle::RenderStyle::Ribbon { .. } => enigma_3d::material::Material::particle_ribbon(&display),
    };
    mat.set_name(&format!("INTERNAL::ParticleDef::{def_uuid}"));
    mat.set_texture_from_resource(&bytes, enigma_3d::material::TextureType::Albedo);
    let new_uuid = mat.uuid;
    app_state.materials.push(mat);

    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        r.editor.per_def_particle_materials.insert(def_uuid, (new_uuid, new_hash));
    }
    Some(new_uuid)
}

fn reconcile_particle_preview(app_state: &mut AppState) {
    use crate::editor::state::Selection;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let selected_def: Option<uuid::Uuid> = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        match &r.editor.selection {
            Selection::Particle(u) => Some(*u),
            _ => None,
        }
    };
    let per_def_mat = selected_def.and_then(|u| ensure_per_def_particle_material(app_state, u));

    let (desired_uuid, desired_config, effective_material) = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = r.project.as_ref() else { return; };
        let sprite_default = r.editor.internal_particle_sprite_material;
        let ribbon_default = r.editor.internal_particle_ribbon_material;
        match &r.editor.selection {
            Selection::Particle(u) => {
                let def = project.particle_systems.iter().find(|p| p.uuid == *u);
                match def {
                    Some(d) => {
                        let explicit_valid = d.material.filter(|u| app_state.materials.iter().any(|m| m.uuid == *u));
                        let fallback = match d.config.render {
                            enigma_3d::particle::RenderStyle::Sprite { .. } => sprite_default,
                            enigma_3d::particle::RenderStyle::Ribbon { .. } => ribbon_default,
                        };
                        let mat = explicit_valid.or(per_def_mat).or(fallback);
                        let mut cfg = d.config.clone();
                        sanitize_particle_config(&mut cfg);
                        (Some(*u), Some(cfg), mat)
                    }
                    None => (None, None, None),
                }
            }
            _ => (None, None, None),
        }
    };

    let applied = app_state
        .get_state_data_value::<EditorRoot>("editor")
        .and_then(|r| r.editor.previewed_particle);

    let new_hash = desired_config.as_ref().map(|cfg| {
        let mat_part = effective_material.map(|m| {
            let mut h = DefaultHasher::new();
            m.hash(&mut h);
            h.finish()
        }).unwrap_or(0);
        hash_particle_config(cfg) ^ mat_part
    });
    let stale = match (desired_uuid, applied, new_hash) {
        (Some(u), Some((au, ah)), Some(h)) => u != au || ah != h,
        (Some(_), None, _) => true,
        (None, Some(_), _) => true,
        _ => false,
    };

    if !stale { return; }

    if let Some((au, _)) = applied {
        app_state.particle_systems.retain(|s| s.handle != au);
    }

    if let (Some(uuid), Some(cfg), Some(hash)) = (desired_uuid, desired_config, new_hash) {
        match enigma_3d::particle::ParticleSystem::from_config(cfg) {
            Ok(mut sys) => {
                sys.handle = uuid;
                sys.material_id = effective_material;
                app_state.particle_systems.push(sys);
                if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                    r.editor.previewed_particle = Some((uuid, hash));
                }
            }
            Err(e) => {
                eprintln!("particle config invalid: {e:?}");
                if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                    r.editor.previewed_particle = None;
                }
            }
        }
    } else if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        r.editor.previewed_particle = None;
    }
}

/// Clamp ranges and shape params to non-degenerate values so rand::gen_range
/// doesn't panic mid-drag while the user is editing the inspector.
fn sanitize_particle_config(cfg: &mut enigma_3d::particle::ParticleSystemConfig) {
    use enigma_3d::particle::{ColorRange, EmitterShape, InitialVelocity};

    fn fix_range(r: &mut enigma_3d::particle::Range<f32>, min_floor: f32) {
        if r.min.is_nan() { r.min = min_floor; }
        if r.max.is_nan() { r.max = min_floor; }
        if r.min < min_floor { r.min = min_floor; }
        if r.max < r.min { r.max = r.min; }
    }

    cfg.max_particles = cfg.max_particles.max(1);
    if cfg.duration <= 0.0 { cfg.duration = 0.001; }
    if cfg.emission_rate.is_nan() || cfg.emission_rate < 0.0 { cfg.emission_rate = 0.0; }
    fix_range(&mut cfg.initial_lifetime, 0.001);
    fix_range(&mut cfg.initial_size, 0.0);
    fix_range(&mut cfg.initial_rotation, f32::NEG_INFINITY);

    match &mut cfg.emitter_shape {
        EmitterShape::Sphere { radius } => { if *radius < 0.0 { *radius = 0.0; } }
        EmitterShape::Box { half_extents } => {
            for e in half_extents.iter_mut() { if *e < 0.0 { *e = 0.0; } }
        }
        EmitterShape::Cone { angle, height } => {
            if *angle < 0.0 { *angle = 0.0; }
            if *height <= 0.0 { *height = 0.001; }
        }
        EmitterShape::Disk { radius } => { if *radius < 0.0 { *radius = 0.0; } }
        EmitterShape::Point => {}
    }

    match &mut cfg.initial_velocity {
        InitialVelocity::Outward { speed }
        | InitialVelocity::Direction { speed, .. }
        | InitialVelocity::Hemisphere { speed, .. } => fix_range(speed, 0.0),
        InitialVelocity::Cone { angle, speed, .. } => {
            if *angle < 0.0 { *angle = 0.0; }
            fix_range(speed, 0.0);
        }
    }

    if let ColorRange::Range { min, max } = &mut cfg.initial_color {
        for i in 0..4 {
            if max[i] < min[i] { max[i] = min[i]; }
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
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // First pass: ensure per-def textured materials for every def referenced
    // by a scene instance. Done outside the snapshot borrow so we can mutate
    // app_state.materials.
    let referenced_defs: Vec<uuid::Uuid> = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = r.project.as_ref() else { return; };
        let Some(scene) = project.scenes.get(project.active_scene_index) else { return; };
        let mut s = std::collections::HashSet::new();
        for inst in &scene.particle_instances { s.insert(inst.def_uuid); }
        s.into_iter().collect()
    };
    let mut per_def: HashMap<uuid::Uuid, Option<uuid::Uuid>> = HashMap::new();
    for d in referenced_defs {
        per_def.insert(d, ensure_per_def_particle_material(app_state, d));
    }

    // Snapshot all instances in the active scene + def configs. Resolve the
    // effective material (explicit if valid, else per-def textured, else
    // built-in fallback) and include in the hash so material changes
    // trigger a rebuild.
    let snapshot: Vec<(uuid::Uuid, uuid::Uuid, [f32; 3], u64, Option<uuid::Uuid>)> = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = r.project.as_ref() else { return; };
        let Some(scene) = project.scenes.get(project.active_scene_index) else { return; };
        let sprite_default = r.editor.internal_particle_sprite_material;
        let ribbon_default = r.editor.internal_particle_ribbon_material;
        scene.particle_instances.iter().filter_map(|inst| {
            let def = project.particle_systems.iter().find(|d| d.uuid == inst.def_uuid)?;
            let explicit_valid = def.material.filter(|u| app_state.materials.iter().any(|m| m.uuid == *u));
            let textured = per_def.get(&inst.def_uuid).copied().flatten();
            let fallback = match def.config.render {
                enigma_3d::particle::RenderStyle::Sprite { .. } => sprite_default,
                enigma_3d::particle::RenderStyle::Ribbon { .. } => ribbon_default,
            };
            let effective_mat = explicit_valid.or(textured).or(fallback);
            let mat_part = effective_mat.map(|m| {
                let mut h = DefaultHasher::new();
                m.hash(&mut h);
                h.finish()
            }).unwrap_or(0);
            let hash = hash_particle_config(&def.config) ^ position_hash(inst.position) ^ mat_part;
            Some((inst.uuid, inst.def_uuid, inst.position, hash, effective_mat))
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
    for (inst_uuid, def_uuid, position, hash, effective_material) in snapshot {
        let prev_hash = applied.get(&inst_uuid).copied();
        if prev_hash == Some(hash) { continue; }

        let config = {
            let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { continue; };
            let Some(project) = r.project.as_ref() else { continue; };
            project.particle_systems.iter().find(|d| d.uuid == def_uuid).map(|d| d.config.clone())
        };
        let Some(mut config) = config else { continue; };
        sanitize_particle_config(&mut config);

        // Remove existing instance with this uuid if any.
        app_state.particle_systems.retain(|s| s.handle != inst_uuid);

        match enigma_3d::particle::ParticleSystem::from_config(config) {
            Ok(mut sys) => {
                sys.handle = inst_uuid;
                sys.material_id = effective_material;
                let m = [
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
    let (desired, applied, material_def_hash) = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = r.project.as_ref() else { return; };
        let Some(scene) = project.scenes.get(project.active_scene_index) else { return; };
        let mat_hash = scene.terrain.as_ref().and_then(|t| t.material).and_then(|u| {
            project.materials.iter().find(|m| m.uuid == u)
                .map(|m| crate::project::material::material_hash(m))
        }).unwrap_or(0);
        (scene.terrain.clone(), r.editor.applied_terrain, mat_hash)
    };

    let new_hash = desired.as_ref().map(|d| hash_terrain_def(d) ^ material_def_hash);
    let stale = new_hash != applied;
    if !stale { return; }

    if let Some(def) = desired {
        let Some(display) = app_state.display.clone() else { return; };
        let material_uuid = def.material;
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
        if let Some(mat_uuid) = material_uuid {
            if let Some(mat) = app_state.materials.iter().find(|m| m.uuid == mat_uuid) {
                terrain.set_material(mat.clone());
            }
        }
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

    // Drop any previously-applied INTERNAL::SkyBox materials so they don't
    // accumulate across skybox swaps.
    app_state.materials.retain(|m| m.name != "INTERNAL::SkyBox");

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
