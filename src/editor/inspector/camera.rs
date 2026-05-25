use egui::{DragValue, Ui};
use enigma_3d::AppState;

pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
    let Some(cam) = app_state.camera.as_mut() else {
        ui.label("(no camera in scene)");
        return;
    };

    egui::CollapsingHeader::new("Transform").default_open(true).show(ui, |ui| {
        let mut p = cam.get_position();
        ui.label("Position");
        let mut changed = false;
        ui.horizontal(|ui| {
            changed |= ui.add(DragValue::new(&mut p[0]).speed(0.05).prefix("x ")).changed();
            changed |= ui.add(DragValue::new(&mut p[1]).speed(0.05).prefix("y ")).changed();
            changed |= ui.add(DragValue::new(&mut p[2]).speed(0.05).prefix("z ")).changed();
        });
        if changed { cam.set_position(p); }

        let mut r = cam.get_rotation();
        ui.label("Rotation (deg)");
        let mut rc = false;
        ui.horizontal(|ui| {
            rc |= ui.add(DragValue::new(&mut r[0]).speed(1.0).prefix("x ")).changed();
            rc |= ui.add(DragValue::new(&mut r[1]).speed(1.0).prefix("y ")).changed();
            rc |= ui.add(DragValue::new(&mut r[2]).speed(1.0).prefix("z ")).changed();
        });
        if rc { cam.set_rotation(r); }
    });

    egui::CollapsingHeader::new("Camera").default_open(true).show(ui, |ui| {
        ui.add(DragValue::new(&mut cam.fov).speed(0.5).prefix("fov "));
        ui.add(DragValue::new(&mut cam.near).speed(0.01).prefix("near "));
        ui.add(DragValue::new(&mut cam.far).speed(1.0).prefix("far "));
    });
}
