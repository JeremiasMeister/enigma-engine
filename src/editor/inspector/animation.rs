use egui::Ui;
use enigma_3d::AppState;
use uuid::Uuid;

pub fn draw(ui: &mut Ui, app_state: &mut AppState, object_uuid: Uuid) {
    let has_anim = app_state.objects.iter()
        .find(|o| o.get_unique_id() == object_uuid)
        .map(|o| o.has_skeletal_animation())
        .unwrap_or(false);

    if !has_anim {
        return;
    }

    egui::CollapsingHeader::new("Animation").default_open(true).show(ui, |ui| {
        ui.label("(animation UI goes here)");
    });
}
