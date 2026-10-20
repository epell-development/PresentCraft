//! The dark operator theme: neutral grays so the slides stand out, orange for "live".

use craft_ui::buttons::{ButtonVisuals, IconButtonStyle};
use craft_ui::theme::WidgetPalette;
use egui::{Color32, CornerRadius, Stroke};

pub const WINDOW_BG: Color32 = Color32::from_rgb(28, 28, 30);
pub const PANEL_BG: Color32 = Color32::from_rgb(38, 38, 41);
pub const CENTER_BG: Color32 = Color32::from_rgb(24, 24, 26);
pub const HEADER_BG: Color32 = Color32::from_rgb(46, 46, 50);
pub const TAB_BG: Color32 = Color32::from_rgb(32, 32, 35);
pub const HOVER: Color32 = Color32::from_rgb(58, 58, 63);
pub const SELECTED: Color32 = Color32::from_rgb(52, 84, 140);
pub const ACCENT: Color32 = Color32::from_rgb(90, 150, 255);
/// Outline and badges for whatever is on the audience screen.
pub const LIVE: Color32 = Color32::from_rgb(255, 140, 26);
pub const TEXT: Color32 = Color32::from_rgb(230, 230, 232);
pub const TEXT_DIM: Color32 = Color32::from_rgb(150, 150, 156);
pub const DANGER: Color32 = Color32::from_rgb(214, 64, 64);

pub fn apply(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::dark());
    let mut style = (*ctx.global_style()).clone();
    let v = &mut style.visuals;
    v.panel_fill = PANEL_BG;
    v.window_fill = PANEL_BG;
    v.extreme_bg_color = Color32::from_rgb(20, 20, 22);
    v.faint_bg_color = Color32::from_rgb(34, 34, 37);
    v.selection.bg_fill = SELECTED;
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;
    WidgetPalette {
        noninteractive: PANEL_BG,
        inactive: Color32::from_rgb(52, 52, 56),
        hovered: HOVER,
        active: Color32::from_rgb(70, 70, 76),
        open: Color32::from_rgb(60, 60, 66),
    }
    .apply_to(&mut v.widgets, CornerRadius::same(4), Stroke::new(1.0, TEXT));
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(20, 20, 22));
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 3.0);
    ctx.set_global_style(style);
}

pub fn toolbar_button() -> IconButtonStyle {
    let normal = ButtonVisuals { fill: Color32::TRANSPARENT, stroke: Stroke::NONE, icon: TEXT_DIM };
    IconButtonStyle {
        normal,
        hovered: ButtonVisuals { fill: HOVER, icon: TEXT, ..normal },
        pressed: Some(ButtonVisuals { fill: Color32::from_rgb(70, 70, 76), icon: TEXT, ..normal }),
        selected: ButtonVisuals { fill: SELECTED, icon: Color32::WHITE, stroke: Stroke::NONE },
        corner_radius: CornerRadius::same(5),
        focus_stroke: Stroke::new(1.0, ACCENT),
    }
}

/// Clear buttons light up while their layer has something on it.
pub fn clear_button(active: bool) -> IconButtonStyle {
    let base = if active { Color32::from_rgb(110, 60, 20) } else { Color32::from_rgb(48, 48, 52) };
    let normal = ButtonVisuals {
        fill: base,
        stroke: Stroke::new(1.0, if active { LIVE } else { Color32::from_rgb(64, 64, 70) }),
        icon: if active { LIVE } else { TEXT_DIM },
    };
    IconButtonStyle {
        normal,
        hovered: ButtonVisuals { fill: HOVER, icon: TEXT, ..normal },
        pressed: None,
        selected: normal,
        corner_radius: CornerRadius::same(5),
        focus_stroke: Stroke::new(1.0, ACCENT),
    }
}
