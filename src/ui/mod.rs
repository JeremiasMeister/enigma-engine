use egui::{Context, Style};

pub mod project_window;
pub mod resource_window;

pub fn get_ui_style(context: &Context) -> Style {
    let mut style = (*context.style()).clone();

    style.visuals.window_shadow.extrusion = 0.0;
    style.visuals.window_shadow.color = egui::Color32::TRANSPARENT;

    style.visuals.window_fill = egui::Color32::from_rgba_premultiplied(0, 0, 0, 200);
    style.visuals.override_text_color = Some(egui::Color32::WHITE);
    style.visuals.window_stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);

    style
}