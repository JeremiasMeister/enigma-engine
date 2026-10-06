use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

use crate::editor::state::EditorRoot;
use crate::editor::widgets;

pub fn draw(ui: &mut Ui, app_state: &mut AppState, object_uuid: Uuid) {
    egui::CollapsingHeader::new("Mesh & Material").default_open(true).show(ui, |ui| {
        let shape_count = app_state.objects.iter()
            .find(|o| o.get_unique_id() == object_uuid)
            .map(|o| o.get_shapes().len())
            .unwrap_or(0);

        ui.label(format!("Shapes: {shape_count}"));

        if shape_count == 0 { return; }

        let (materials, scene_uuid, per_shape) = {
            let Some(root) = app_state.get_state_data_value::<EditorRoot>("editor") else {
                return;
            };
            let Some(project) = root.project.as_ref() else { return; };
            let mats: Vec<(Uuid, String)> = project.materials.iter()
                .map(|m| (m.uuid, m.name.clone()))
                .collect();
            let scene_uuid = project.scenes.get(project.active_scene_index).map(|s| s.uuid);
            let per_shape: Vec<Option<Uuid>> = scene_uuid
                .map(|s| (0..shape_count)
                    .map(|i| project.get_assignment(s, object_uuid, i))
                    .collect())
                .unwrap_or_default();
            (mats, scene_uuid, per_shape)
        };

        let Some(scene_uuid) = scene_uuid else { return; };

        let mut updates: Vec<(usize, Option<Uuid>)> = Vec::new();

        for shape_idx in 0..shape_count {
            let mut current = per_shape.get(shape_idx).copied().unwrap_or(None);
            if widgets::asset_picker(ui, &format!("Shape {shape_idx}"), &mut current, &materials, "None") {
                updates.push((shape_idx, current));
            }
        }

        if !updates.is_empty() {
            if let Some(root) = app_state.get_state_data_value_mut::<EditorRoot>("editor") {
                if let Some(project) = root.project.as_mut() {
                    for (shape_idx, value) in updates {
                        match value {
                            Some(mat_uuid) => project.set_assignment(scene_uuid, object_uuid, shape_idx, mat_uuid),
                            None => project.clear_assignment(scene_uuid, object_uuid, shape_idx),
                        }
                    }
                    root.editor.dirty = true;
                }
            }
        }
    });
}
