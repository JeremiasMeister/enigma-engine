pub mod state;
pub mod actions;
pub mod panels;
pub mod inspector;

use egui::Context;
use enigma_3d::AppState;

pub fn draw(ctx: &Context, app_state: &mut AppState) {
    set_style(ctx);

    egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
        panels::toolbar::draw(ui, app_state);
    });

    egui::SidePanel::left("hierarchy")
        .default_width(220.0)
        .min_width(160.0)
        .resizable(true)
        .show(ctx, |ui| {
            panels::hierarchy::draw(ui, app_state);
        });

    egui::SidePanel::right("inspector")
        .default_width(320.0)
        .min_width(240.0)
        .resizable(true)
        .show(ctx, |ui| {
            panels::inspector::draw(ui, app_state);
        });

    egui::TopBottomPanel::bottom("resource_browser")
        .default_height(200.0)
        .min_height(120.0)
        .resizable(true)
        .show(ctx, |ui| {
            panels::resource_browser::draw(ui, app_state);
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            panels::viewport::draw(ui, app_state);
        });
}

fn set_style(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.window_shadow.extrusion = 0.0;
    style.visuals.window_shadow.color = egui::Color32::TRANSPARENT;
    style.visuals.window_stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
    ctx.set_style(style);
}
