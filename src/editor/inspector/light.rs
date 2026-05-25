use egui::{DragValue, Ui};
use enigma_3d::AppState;

pub fn draw(ui: &mut Ui, app_state: &mut AppState, index: usize) {
    let Some(light) = app_state.light.get_mut(index) else { return; };
    draw_props(ui, light, /*ambient=*/false);
}

pub fn draw_ambient(ui: &mut Ui, app_state: &mut AppState) {
    let Some(light) = app_state.ambient_light.as_mut() else {
        ui.label("(no ambient light)");
        return;
    };
    draw_props(ui, light, /*ambient=*/true);
}

fn draw_props(ui: &mut Ui, light: &mut enigma_3d::light::Light, ambient: bool) {
    let label = if ambient { "Ambient Light" } else { "Light" };
    egui::CollapsingHeader::new(label).default_open(true).show(ui, |ui| {
        ui.label("Color");
        ui.color_edit_button_rgb(&mut light.color);

        ui.label("Intensity");
        ui.add(DragValue::new(&mut light.intensity).speed(0.05).clamp_range(0.0..=20.0));

        if !ambient {
            ui.checkbox(&mut light.cast_shadow, "Cast shadow");

            let mut directional = light.is_directional();
            if ui.checkbox(&mut directional, "Directional").changed() {
                light.direction = if directional { [0.0, -1.0, 0.0] } else { [0.0, 0.0, 0.0] };
            }
            if directional {
                ui.label("Direction");
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut light.direction[0]).speed(0.01).prefix("x "));
                    ui.add(DragValue::new(&mut light.direction[1]).speed(0.01).prefix("y "));
                    ui.add(DragValue::new(&mut light.direction[2]).speed(0.01).prefix("z "));
                });
            }
        }
    });
}
