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
    reconcile_materials(app_state);
    apply_material_assignments(app_state);
    reconcile_skybox(app_state);

    // Keep repainting while a job is running so the spinner animates and
    // the poll picks up completion promptly.
    let busy = app_state.get_state_data_value::<EditorRoot>("editor")
        .map(|r| r.editor.job.is_some() || r.editor.project_load.is_some())
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
    let (job_label, job_elapsed, job_lines) = {
        let Some(r) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        match (r.editor.job.as_ref(), r.editor.project_load.as_ref()) {
            (Some(j), _) => (j.label.clone(), j.started_at.elapsed(), j.lines.clone()),
            (None, Some(j)) => (j.label.clone(), j.started_at.elapsed(), j.lines.clone()),
            (None, None) => return,
        }
    };

    let (label, elapsed, lines) = (job_label, job_elapsed, job_lines);
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
    }
    if let Some(r) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
        r.editor.selection = crate::editor::state::Selection::None;
        r.editor.dirty = true;
    }
}

fn apply_material_assignments(app_state: &mut AppState) {
    let (assignments, default_mat) = {
        let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else { return; };
        let Some(project) = root.project.as_ref() else { return; };
        let Some(scene) = project.scenes.get(project.active_scene_index) else { return; };
        let default_mat = project.materials.first().map(|m| m.uuid);
        (project.assignments_for_scene(scene.uuid), default_mat)
    };

    for obj in app_state.objects.iter_mut() {
        let obj_uuid = obj.get_unique_id();
        let target = assignments.get(&obj_uuid).copied().or(default_mat);
        let Some(mat_uuid) = target else { continue; };
        let mats = obj.get_materials_mut();
        if mats.is_empty() {
            mats.push(mat_uuid);
        } else if mats[0] != mat_uuid {
            mats[0] = mat_uuid;
        }
    }
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
