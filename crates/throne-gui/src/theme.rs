use egui::{Color32, CornerRadius, Stroke, Visuals};

pub fn apply_throne_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // Dark slate & navy palette matching Throne's Qt6 stylesheet
    let bg_dark = Color32::from_rgb(18, 22, 31);         // #12161f
    let bg_card = Color32::from_rgb(25, 32, 44);         // #19202c
    let bg_panel = Color32::from_rgb(22, 28, 38);        // #161c26
    let border = Color32::from_rgb(48, 54, 61);          // #30363d
    let text_bright = Color32::from_rgb(230, 237, 243);  // #e6edf3
    let text_muted = Color32::from_rgb(139, 148, 158);   // #8b949e
    let accent_blue = Color32::from_rgb(43, 114, 214);   // #2b72d6
    let accent_hover = Color32::from_rgb(56, 139, 253);  // #388bfd

    visuals.dark_mode = true;
    visuals.override_text_color = Some(text_bright);
    visuals.panel_fill = bg_dark;
    visuals.window_fill = bg_card;
    visuals.faint_bg_color = bg_panel;
    visuals.extreme_bg_color = Color32::from_rgb(13, 17, 23);

    visuals.widgets.noninteractive.bg_fill = bg_card;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, text_muted);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);

    visuals.widgets.inactive.bg_fill = bg_panel;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, text_bright);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(33, 42, 58);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, accent_hover);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);

    visuals.widgets.active.bg_fill = accent_blue;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, accent_hover);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(43, 114, 214, 100);
    visuals.selection.stroke = Stroke::new(1.0_f32, accent_blue);

    ctx.set_visuals(visuals);
}
