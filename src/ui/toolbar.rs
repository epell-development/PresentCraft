//! The top toolbar: mode switch, transport, new presentation and screen toggles.

use egui::{RichText, Ui};

use super::icons::{self, Icon};
use super::theme;
use crate::app::{App, Mode, NewPresentation};

pub fn show(app: &mut App, ui: &mut Ui) {
    let style = theme::toolbar_button();
    let (mut bin, mut stage, mut audience) = (app.prefs.get().layout.show_media_bin, app.stage_open, app.audience_open);
    let mut screens = app.screens_open;
    let save_state = if let Some(err) = &app.autosave.last_error {
        Some((RichText::new("Not saved").color(theme::DANGER), Some(err.clone())))
    } else if app.autosave.is_dirty() {
        Some((RichText::new("Saving…").color(theme::TEXT_DIM), None))
    } else if app.path.is_some() && !app.autosave.disabled {
        Some((RichText::new("Saved").color(theme::TEXT_DIM), None))
    } else {
        None
    };
    egui::Panel::top("toolbar").exact_size(56.0).frame(egui::Frame::new().fill(theme::WINDOW_BG).inner_margin(egui::Margin::symmetric(10, 4))).show(
        ui,
        |ui| {
            ui.horizontal_centered(|ui| {
                left(app, ui, &style);
                // Right-aligned group: reserve its width so the captioned buttons keep their order.
                let right_width = 4.0 * 50.0 + 100.0;
                ui.add_space((ui.available_width() - right_width).max(0.0));
                if icons::tool(ui, Icon::Props, "Screens", screens, &style).on_hover_text("Where the output windows go").clicked() {
                    screens = !screens;
                }
                if icons::tool(ui, Icon::Screen, "Audience", audience, &style).on_hover_text("Audience output window (Ctrl+1)").clicked() {
                    audience = !audience;
                }
                if icons::tool(ui, Icon::Stage, "Stage", stage, &style).on_hover_text("Stage display window (Ctrl+2)").clicked() {
                    stage = !stage;
                }
                if icons::tool(ui, Icon::Bin, "Bin", bin, &style).on_hover_text("Show or hide the media bin").clicked() {
                    bin = !bin;
                }
                ui.separator();
                if let Some((text, hover)) = save_state {
                    let r = ui.label(text);
                    if let Some(h) = hover {
                        r.on_hover_text(h);
                    }
                }
            });
        },
    );
    if bin != app.prefs.get().layout.show_media_bin {
        app.prefs.update(|p| p.layout.show_media_bin = bin);
    }
    app.stage_open = stage;
    app.screens_open = screens;
    app.audience_open = audience;
    if !app.notices.is_empty() {
        egui::Panel::top("notices").frame(egui::Frame::new().fill(egui::Color32::from_rgb(90, 60, 20)).inner_margin(6)).show(ui, |ui| {
            let mut dismiss = None;
            for (i, n) in app.notices.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(n).color(theme::TEXT));
                    if ui.small_button("Dismiss").clicked() {
                        dismiss = Some(i);
                    }
                });
            }
            if let Some(i) = dismiss {
                app.notices.remove(i);
            }
        });
    }
}

fn left(app: &mut App, ui: &mut Ui, style: &craft_ui::buttons::IconButtonStyle) {
    ui.label(RichText::new("PresentCraft").strong().size(15.0).color(theme::TEXT));
    ui.add_space(14.0);
    for (mode, icon, label) in [
        (Mode::Show, Icon::Show, "Show"),
        (Mode::Edit, Icon::Edit, "Edit"),
        (Mode::Reflow, Icon::Reflow, "Reflow"),
        (Mode::Bible, Icon::Bible, "Bible"),
    ] {
        if icons::tool(ui, icon, label, app.mode == mode, style).clicked() {
            app.mode = mode;
            if mode == Mode::Edit && app.selected_slide.is_none() {
                app.selected_slide = app.presentation().and_then(|p| p.groups.iter().flat_map(|g| &g.slides).next()).map(|s| s.id);
            }
        }
    }
    ui.separator();
    if icons::tool(ui, Icon::Plus, "New", false, style).on_hover_text("New presentation (paste lyrics to split into slides)").clicked() {
        app.new_presentation = Some(NewPresentation { add_to_playlist: app.selected_playlist.is_some(), ..Default::default() });
    }
    ui.separator();
    if icons::tool(ui, Icon::Prev, "Previous", false, style).clicked() {
        app.go_prev();
    }
    if icons::tool(ui, Icon::Next, "Next", false, style).clicked() {
        app.go_next();
    }
    if let Some((p, s)) = app.live.live_slide(&app.lib) {
        let cues = p.cues();
        let index = cues.iter().position(|c| p.slide(*c).map(|x| x.id) == Some(s.id)).map(|i| i + 1).unwrap_or(0);
        ui.label(RichText::new(format!("• {}  {}/{}", p.name, index, cues.len())).color(theme::LIVE));
    } else {
        ui.label(RichText::new("Nothing live").color(theme::TEXT_DIM));
    }
}
