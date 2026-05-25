use egui::{DragValue, Ui};
use enigma_3d::AppState;
use uuid::Uuid;

pub fn draw_for_object(ui: &mut Ui, app_state: &mut AppState, uuid: Uuid) {
    let Some(obj) = app_state.objects.iter_mut().find(|o| o.get_unique_id() == uuid) else { return; };

    egui::CollapsingHeader::new("Transform").default_open(true).show(ui, |ui| {
        let mut position: [f32; 3] = obj.transform.position.into();
        if vec3_edit(ui, "Position", &mut position, 0.05) {
            obj.transform.set_position(position);
        }

        let mut rotation: [f32; 3] = obj.transform.get_rotation().into();
        if vec3_edit(ui, "Rotation (deg)", &mut rotation, 1.0) {
            obj.transform.set_rotation(rotation);
        }

        let mut scale: [f32; 3] = obj.transform.scale.into();
        if vec3_edit(ui, "Scale", &mut scale, 0.05) {
            obj.transform.set_scale(scale);
        }
    });
}

pub fn draw_for_light(ui: &mut Ui, app_state: &mut AppState, index: usize) {
    let Some(light) = app_state.light.get_mut(index) else { return; };
    egui::CollapsingHeader::new("Transform").default_open(true).show(ui, |ui| {
        ui.label("Position");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(&mut light.position[0]).speed(0.05).prefix("x "));
            ui.add(DragValue::new(&mut light.position[1]).speed(0.05).prefix("y "));
            ui.add(DragValue::new(&mut light.position[2]).speed(0.05).prefix("z "));
        });
    });
}

fn vec3_edit(ui: &mut Ui, label: &str, v: &mut [f32; 3], speed: f32) -> bool {
    ui.label(label);
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui.add(DragValue::new(&mut v[0]).speed(speed).prefix("x ")).changed();
        changed |= ui.add(DragValue::new(&mut v[1]).speed(speed).prefix("y ")).changed();
        changed |= ui.add(DragValue::new(&mut v[2]).speed(speed).prefix("z ")).changed();
    });
    changed
}
