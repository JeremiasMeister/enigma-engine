use egui::{Color32, Context, FontId, Frame, Margin, RichText, Rounding, Stroke, TextStyle, Ui};

pub const BACKGROUND: Color32 = Color32::from_rgb(23, 28, 38);
pub const PANEL: Color32 = Color32::from_rgb(31, 38, 51);
pub const SURFACE: Color32 = Color32::from_rgb(43, 53, 70);
pub const BORDER: Color32 = Color32::from_rgb(57, 69, 89);
pub const TEXT: Color32 = Color32::from_rgb(229, 234, 243);
pub const MUTED: Color32 = Color32::from_rgb(159, 174, 195);
pub const ACCENT: Color32 = Color32::from_rgb(132, 181, 250);
pub const AMBER: Color32 = Color32::from_rgb(231, 185, 106);

pub fn apply(ctx: &Context) {
    let id = egui::Id::new("enigma_theme_initialized");
    if ctx.data(|data| data.get_temp::<bool>(id).unwrap_or(false)) {
        return;
    }

    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(13.0));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(13.0));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(18.0));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(11.0));
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size = egui::vec2(40.0, 25.0);
    style.spacing.indent = 14.0;
    style.spacing.slider_width = 160.0;
    style.spacing.combo_width = 140.0;

    let visuals = &mut style.visuals;
    *visuals = egui::Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = BACKGROUND;
    visuals.faint_bg_color = SURFACE;
    visuals.code_bg_color = BACKGROUND;
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = AMBER;
    visuals.error_fg_color = Color32::from_rgb(245, 135, 140);
    visuals.selection.bg_fill = Color32::from_rgb(49, 77, 112);
    visuals.selection.stroke = Stroke::new(1.0_f32, TEXT);
    visuals.window_rounding = Rounding::same(8.0);
    visuals.menu_rounding = Rounding::same(6.0);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.window_shadow.extrusion = 12.0;
    visuals.window_shadow.color = Color32::from_black_alpha(65);
    visuals.collapsing_header_frame = true;
    visuals.indent_has_left_vline = true;
    visuals.slider_trailing_fill = true;

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.rounding = Rounding::same(4.0);
        widget.fg_stroke = Stroke::new(1.0_f32, TEXT);
        widget.bg_stroke = Stroke::new(1.0_f32, BORDER);
        widget.expansion = 0.0;
    }
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, MUTED);
    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.inactive.weak_bg_fill = SURFACE;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(58, 75, 99);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(49, 63, 84);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.active.bg_fill = Color32::from_rgb(60, 91, 130);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(49, 77, 112);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.open.bg_fill = SURFACE;
    visuals.widgets.open.weak_bg_fill = SURFACE;
    ctx.set_style(style);
    ctx.data_mut(|data| data.insert_temp(id, true));
}

pub fn panel_frame() -> Frame {
    Frame::none()
        .fill(PANEL)
        .inner_margin(Margin::same(12.0))
        .stroke(Stroke::new(1.0_f32, BORDER))
}

pub fn panel_heading(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(16.0).strong().color(TEXT));
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).small().color(MUTED));
    }
    ui.add_space(2.0);
    ui.separator();
}

pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(title).strong().color(TEXT));
}
