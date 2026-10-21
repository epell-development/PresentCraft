//! Modal-style windows: the new presentation dialog.

use egui::RichText;

use super::theme;
use crate::app::App;
use crate::model::*;

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(mut np) = app.new_presentation.take() else {
        return;
    };
    let mut open = true;
    let mut create = false;
    let themes: Vec<(Id, String)> = app.lib.themes.iter().map(|t| (t.id, t.name.clone())).collect();
    if np.theme.is_none() {
        np.theme = themes.first().map(|(id, _)| *id);
    }
    egui::Window::new("New presentation").open(&mut open).collapsible(false).resizable(true).default_width(480.0).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.add(egui::TextEdit::singleline(&mut np.name).hint_text("Song or presentation title").desired_width(f32::INFINITY)).request_focus();
        });
        egui::ComboBox::from_label("Theme")
            .selected_text(np.theme.and_then(|t| themes.iter().find(|(id, _)| *id == t)).map(|(_, n)| n.as_str()).unwrap_or("None"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut np.theme, None, "None");
                for (id, name) in &themes {
                    ui.selectable_value(&mut np.theme, Some(*id), name);
                }
            });
        ui.label(RichText::new("Paste lyrics or text. Blank lines split slides; [Chorus] or \"Verse 2:\" starts a group.").color(theme::TEXT_DIM));
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut np.lyrics).desired_rows(14).desired_width(f32::INFINITY));
        });
        if app.selected_playlist.is_some() {
            ui.checkbox(&mut np.add_to_playlist, "Add to the selected playlist");
        }
        ui.horizontal(|ui| {
            create = ui.add_enabled(!np.name.trim().is_empty(), egui::Button::new("Create")).clicked();
        });
    });
    if create {
        let p = app.lib.presentation_from_lyrics(np.name.trim(), &np.lyrics, np.theme);
        let pid = p.id;
        app.lib.presentations.push(p);
        if np.add_to_playlist
            && let Some(pl) = app.selected_playlist
        {
            let id = app.lib.next_id();
            if let Some(list) = app.lib.playlists.iter_mut().find(|x| x.id == pl) {
                list.items.push(PlaylistItem::Presentation { id, presentation: pid });
            }
        }
        app.selected_presentation = Some(pid);
        app.selected_slide = None;
        app.changed();
    } else if open {
        app.new_presentation = Some(np);
    }
}

/// The Screens window: where each output opens, its size, and full screen.
pub fn screens(app: &mut App, ctx: &egui::Context) {
    if !app.screens_open {
        return;
    }
    let mut open = true;
    let mut outputs = app.prefs.get().outputs.clone();
    // egui reports the size of the display the main window is on; a second display usually starts
    // where the first one ends.
    let monitor = ctx.input(|i| i.viewport().monitor_size);
    egui::Window::new("Screens").open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        if let Some(m) = monitor {
            ui.label(RichText::new(format!("This display is {} × {}.", m.x.round(), m.y.round())).color(theme::TEXT_DIM));
        }
        for (label, screen, window_open) in
            [("Audience", &mut outputs.audience, &mut app.audience_open), ("Stage", &mut outputs.stage, &mut app.stage_open)]
        {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).strong());
                ui.checkbox(window_open, "Open");
                ui.checkbox(&mut screen.fullscreen, "Full screen");
            });
            egui::Grid::new(label).num_columns(4).show(ui, |ui| {
                ui.label("X");
                ui.add(egui::DragValue::new(&mut screen.x).speed(4.0));
                ui.label("Y");
                ui.add(egui::DragValue::new(&mut screen.y).speed(4.0));
                ui.end_row();
                ui.label("Width");
                ui.add(egui::DragValue::new(&mut screen.width).speed(4.0).range(320.0..=8192.0));
                ui.label("Height");
                ui.add(egui::DragValue::new(&mut screen.height).speed(4.0).range(180.0..=8192.0));
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.button("This display").clicked() {
                    screen.x = 40.0;
                    screen.y = 40.0;
                    screen.fullscreen = false;
                }
                if let Some(m) = monitor
                    && ui
                        .button("Display to the right")
                        .on_hover_text("Moves the window past the right edge of this display and makes it full screen")
                        .clicked()
                {
                    screen.x = m.x;
                    screen.y = 0.0;
                    screen.fullscreen = true;
                }
                for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0)] {
                    if ui.button(format!("{w}×{h}")).clicked() {
                        screen.width = w;
                        screen.height = h;
                    }
                }
            });
        }
        ui.separator();
        ui.checkbox(&mut outputs.open_on_launch, "Open both outputs when PresentCraft starts");
        ui.label(RichText::new("Tip: double-click an output to toggle full screen; Esc leaves it.").color(theme::TEXT_DIM));
    });
    if outputs != app.prefs.get().outputs {
        app.prefs.update(|p| p.outputs = outputs);
    }
    app.screens_open = open;
}
