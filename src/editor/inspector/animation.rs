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
    });
}
