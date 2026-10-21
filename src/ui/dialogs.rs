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
