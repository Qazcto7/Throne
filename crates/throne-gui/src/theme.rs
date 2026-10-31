use egui::{Color32, CornerRadius, Stroke, Visuals};

pub fn apply_throne_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // Dark charcoal palette matching Throne 1.2.0 Qt6 stylesheet
    let bg_dark = Color32::from_rgb(22, 24, 28);         // #16181c
    let bg_card = Color32::from_rgb(28, 30, 36);         // #1c1e24
    let bg_panel = Color32::from_rgb(24, 26, 31);        // #181a1f
    let border = Color32::from_rgb(52, 58, 68);          // #343a44
    let text_bright = Color32::from_rgb(235, 240, 248);  // #ebf0f8
    let text_muted = Color32::from_rgb(145, 155, 168);   // #919ba8
    let purple_accent = Color32::from_rgb(124, 77, 255); // #7c4dff (Old Throne signature selection)
    let purple_hover = Color32::from_rgb(142, 98, 255);

    visuals.dark_mode = true;
    visuals.override_text_color = Some(text_bright);
    visuals.panel_fill = bg_dark;
    visuals.window_fill = bg_card;
    visuals.faint_bg_color = bg_panel;
    visuals.extreme_bg_color = Color32::from_rgb(16, 17, 20);

    visuals.widgets.noninteractive.bg_fill = bg_card;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text_muted);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);

    visuals.widgets.inactive.bg_fill = bg_panel;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text_bright);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 42, 52);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, purple_hover);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);

    visuals.widgets.active.bg_fill = purple_accent;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, purple_hover);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);

    visuals.selection.bg_fill = purple_accent;
    visuals.selection.stroke = Stroke::new(1.0_f32, purple_hover);

    ctx.set_visuals(visuals);
}
