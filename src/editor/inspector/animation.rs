use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

pub fn draw(ui: &mut Ui, app_state: &mut AppState, object_uuid: Uuid) {
    let Some(obj_idx) = app_state.objects.iter().position(|o| o.get_unique_id() == object_uuid) else {
        return;
    };
    if !app_state.objects[obj_idx].has_skeletal_animation() {
        return;
    }

    egui::CollapsingHeader::new("Animation").default_open(true).show(ui, |ui| {
        let obj = &app_state.objects[obj_idx];

        // Collect clip names sorted.
        let mut clip_names: Vec<String> = obj.get_animations().keys().cloned().collect();
        clip_names.sort();

        // Current selection.
        let current_name: Option<String> = obj.get_current_animation()
            .as_ref()
            .map(|s| s.name.clone());
        let current_looping: bool = obj.get_current_animation()
            .as_ref()
            .map(|s| s.looping)
            .unwrap_or(false);

        let label = current_name.clone().unwrap_or_else(|| "<None>".to_string());

        let mut picked: Option<Option<String>> = None;
        egui::ComboBox::from_label("Clip")
            .selected_text(label)
            .show_ui(ui, |ui| {
                if ui.selectable_label(current_name.is_none(), "<None>").clicked() {
                    picked = Some(None);
                }
                for name in &clip_names {
                    let selected = current_name.as_deref() == Some(name.as_str());
                    if ui.selectable_label(selected, name).clicked() {
                        picked = Some(Some(name.clone()));
                    }
                }
            });

        if let Some(choice) = picked {
            let obj_mut = &mut app_state.objects[obj_idx];
            match choice {
                None => obj_mut.stop_animation(),
                Some(name) => {
                    // Skip the call if it would just reset the already-current clip.
                    if current_name.as_deref() != Some(name.as_str()) {
                        obj_mut.play_animation(&name, current_looping);
                    }
                }
            }
        }

        // --- Transport row ---
        let has_current = app_state.objects[obj_idx].get_current_animation().is_some();
        ui.horizontal(|ui| {
            if ui.add_enabled(has_current, egui::Button::new("Play")).clicked() {
                if let Some(state) = app_state.objects[obj_idx].get_current_animation_mut().as_mut() {
                    state.speed = 1.0;
                }
            }
            if ui.add_enabled(has_current, egui::Button::new("Pause")).clicked() {
                if let Some(state) = app_state.objects[obj_idx].get_current_animation_mut().as_mut() {
                    state.speed = 0.0;
                }
            }
            if ui.add_enabled(has_current, egui::Button::new("Stop")).clicked() {
                app_state.objects[obj_idx].stop_animation();
            }
        });

        // --- Loop checkbox ---
        let mut looping = current_looping;
        let loop_resp = ui.add_enabled(has_current, egui::Checkbox::new(&mut looping, "Loop"));
        if loop_resp.changed() {
            if let Some(state) = app_state.objects[obj_idx].get_current_animation_mut().as_mut() {
                state.looping = looping;
            }
        }

        // --- Time scrubber ---
        // Compute duration from the active clip (default 0 if not found / no current animation).
        let (mut time_value, duration, clip_name_for_readout): (f32, f32, Option<String>) = {
            let obj = &app_state.objects[obj_idx];
            match obj.get_current_animation().as_ref() {
                Some(state) => {
                    let dur = obj.get_animations()
                        .get(&state.name)
                        .map(|a| a.duration)
                        .unwrap_or(0.0);
                    (state.time, dur, Some(state.name.clone()))
                }
                None => (0.0, 0.0, None),
            }
        };

        let scrub_resp = ui.add_enabled(
            has_current && duration > 0.0,
            egui::Slider::new(&mut time_value, 0.0..=duration.max(f32::EPSILON)).text("Time"),
        );
        if scrub_resp.changed() {
            if let Some(state) = app_state.objects[obj_idx].get_current_animation_mut().as_mut() {
                state.time = time_value;
            }
        }

        // --- Read-out ---
        if let Some(name) = clip_name_for_readout {
            ui.label(format!("{:.2} / {:.2}s   {}", time_value, duration, name));
        }
    });
}
