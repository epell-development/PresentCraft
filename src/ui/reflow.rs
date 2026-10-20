//! Reflow mode: edit a presentation's words as one block of text. Blank lines split slides and
//! `[Group]` lines start groups; applying rebuilds the slides with the presentation's theme.

use egui::{RichText, Ui};

use super::theme;
use crate::app::App;
use crate::model::*;

pub fn show(app: &mut App, ui: &mut Ui) {
    let Some(p) = app.presentation().cloned() else {
        ui.centered_and_justified(|ui| ui.label(RichText::new("Select a presentation to reflow").color(theme::TEXT_DIM)));
        return;
    };
    if app.reflow.as_ref().map(|(id, _)| *id) != Some(p.id) {
        app.reflow = Some((p.id, Library::lyrics_of(&p)));
    }
    let mut apply = false;
    let mut revert = false;
    egui::Frame::new().inner_margin(12).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&p.name).strong().size(15.0));
            ui.label(RichText::new("Blank line = new slide · [Name] = new group").color(theme::TEXT_DIM));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                apply = ui.button("Apply").clicked();
                revert = ui.button("Revert").clicked();
            });
        });
        ui.add_space(6.0);
        if let Some((_, text)) = &mut app.reflow {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(text).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(30));
            });
        }
    });
    if revert {
        app.reflow = None;
    }
    if apply && let Some((_, text)) = app.reflow.clone() {
        let groups = app.lib.groups_from_lyrics(&text, p.theme);
        let groups = carry_over(&p, groups);
        if let Some(pm) = app.lib.presentation_mut(p.id) {
            pm.groups = groups;
            // Arrangements refer to group ids; keep the ones whose groups survived by name.
            let names: Vec<(Id, String)> = pm.groups.iter().map(|g| (g.id, g.name.clone())).collect();
            for a in &mut pm.arrangements {
                a.groups = a
                    .groups
                    .iter()
                    .filter_map(|old| {
                        p.groups.iter().find(|g| g.id == *old).and_then(|g| names.iter().find(|(_, n)| *n == g.name)).map(|(id, _)| *id)
                    })
                    .collect();
            }
        }
        app.selected_slide = None;
        app.changed();
    }
}

/// Keep group colors, slide media actions, notes and per-slide backgrounds where the new text
/// lines up with the old slides, so a typo fix does not wipe the cues.
fn carry_over(old: &Presentation, mut groups: Vec<Group>) -> Vec<Group> {
    for (gi, g) in groups.iter_mut().enumerate() {
        let before = old.groups.iter().find(|o| o.name == g.name).or_else(|| old.groups.get(gi));
        let Some(before) = before else { continue };
        if before.name == g.name {
            g.color = before.color;
        }
        for (si, s) in g.slides.iter_mut().enumerate() {
            if let Some(o) = before.slides.get(si) {
                s.media_action = o.media_action;
                s.notes = o.notes.clone();
                s.enabled = o.enabled;
                if o.background.is_some() {
                    s.background = o.background;
                }
            }
        }
    }
    groups
}
