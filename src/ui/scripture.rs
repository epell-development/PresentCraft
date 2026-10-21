//! Bible mode: look up a passage, preview it, and put it live or into the library.

use egui::{RichText, Sense, Ui, vec2};

use super::theme;
use crate::app::App;
use crate::bible::{self, Bible, Reference};
use crate::model::*;
use crate::render::Painting;

enum Action {
    GoLive,
    AddToLibrary,
    Import,
    Open(Reference),
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let mut action = None;
    if app.scripture.bible >= app.bibles.len() {
        app.scripture.bible = 0;
    }
    egui::Frame::new().fill(theme::HEADER_BG).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let names: Vec<String> = app.bibles.iter().map(|b| format!("{} ({})", b.name, b.abbreviation)).collect();
            egui::ComboBox::from_id_salt("translation").selected_text(names.get(app.scripture.bible).cloned().unwrap_or_default()).show_ui(
                ui,
                |ui| {
                    for (i, n) in names.iter().enumerate() {
                        ui.selectable_value(&mut app.scripture.bible, i, n);
                    }
                },
            );
            let r = ui.add(egui::TextEdit::singleline(&mut app.scripture.reference).hint_text("John 3:16-18").desired_width(220.0));
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                action = Some(Action::GoLive);
            }
            ui.add(egui::TextEdit::singleline(&mut app.scripture.search).hint_text("Search words").desired_width(200.0));
        });
    });

    let Some(bible) = app.bibles.get(app.scripture.bible).cloned() else {
        ui.label("No translations installed.");
        return;
    };
    let reference = bible::parse_reference(&app.scripture.reference);

    egui::Panel::right("scripture-options").exact_size(300.0).frame(egui::Frame::new().fill(theme::PANEL_BG).inner_margin(10)).show(ui, |ui| {
        options(app, ui, &bible, reference, &mut action);
    });
    egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(12)).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if !app.scripture.search.trim().is_empty() {
                let hits = bible.search(&app.scripture.search, 200);
                ui.label(RichText::new(format!("{} matches", hits.len())).color(theme::TEXT_DIM));
                for v in hits {
                    let label = format!("{} {}:{}", bible::book_name(v.book), v.chapter, v.verse);
                    ui.horizontal_wrapped(|ui| {
                        if ui.link(&label).clicked() {
                            action = Some(Action::Open(Reference { book: v.book, chapter: v.chapter, verses: Some((v.verse, v.verse)) }));
                        }
                        ui.label(&v.text);
                    });
                }
                return;
            }
            match reference {
                None => {
                    ui.label(RichText::new("Type a reference such as John 3:16, Ps 23 or 1 Cor 13:4-8.").color(theme::TEXT_DIM));
                }
                Some(r) => {
                    let verses = bible.passage(&r);
                    ui.label(RichText::new(r.label()).strong().size(16.0));
                    if verses.is_empty() {
                        ui.label(RichText::new(format!("{} has no verses for this reference.", bible.abbreviation)).color(theme::TEXT_DIM));
                    }
                    for v in verses {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(v.verse.to_string()).color(theme::ACCENT).strong());
                            ui.label(&v.text);
                        });
                    }
                }
            }
        });
    });

    match action {
        Some(Action::Open(r)) => {
            app.scripture.reference = r.label();
            app.scripture.search.clear();
        }
        Some(Action::GoLive) => build(app, &bible, reference, true),
        Some(Action::AddToLibrary) => build(app, &bible, reference, false),
        Some(Action::Import) => import(app),
        None => {}
    }
}

fn options(app: &mut App, ui: &mut Ui, bible: &Bible, reference: Option<Reference>, action: &mut Option<Action>) {
    ui.heading("Slides");
    ui.add(egui::Slider::new(&mut app.scripture.per_slide, 1..=4).text("Verses per slide"));
    ui.checkbox(&mut app.scripture.verse_numbers, "Verse numbers");
    let themes: Vec<(Id, String)> = app.lib.themes.iter().map(|t| (t.id, t.name.clone())).collect();
    egui::ComboBox::from_label("Theme")
        .selected_text(app.scripture.theme.and_then(|t| themes.iter().find(|(id, _)| *id == t)).map(|(_, n)| n.as_str()).unwrap_or("Default"))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.scripture.theme, None, "Default");
            for (id, name) in &themes {
                ui.selectable_value(&mut app.scripture.theme, Some(*id), name);
            }
        });

    // Preview of the first slide, built the same way "Go live" builds it.
    if let Some(r) = reference {
        let mut scratch = Library { themes: app.lib.themes.clone(), ..Library::default() };
        if let Some(p) = bible::presentation(&mut scratch, bible, &r, app.scripture.per_slide, app.scripture.theme, app.scripture.verse_numbers)
            && let Some(first) = p.groups.first().and_then(|g| g.slides.first())
        {
            let w = ui.available_width();
            let (rect, _) = ui.allocate_exact_size(vec2(w, w * 9.0 / 16.0), Sense::hover());
            let ctx = ui.ctx().clone();
            Painting { lib: &app.lib, cache: &mut app.cache, ctx: &ctx }.thumbnail(ui.painter(), rect, first);
            ui.label(RichText::new(format!("{} slides", p.slide_count())).color(theme::TEXT_DIM));
        }
    }
    ui.horizontal(|ui| {
        let ok = reference.is_some();
        if ui
            .add_enabled(ok, egui::Button::new(RichText::new("Go live").strong()))
            .on_hover_text("Enter in the reference box does the same")
            .clicked()
        {
            *action = Some(Action::GoLive);
        }
        if ui.add_enabled(ok, egui::Button::new("Add to library")).clicked() {
            *action = Some(Action::AddToLibrary);
        }
    });
    if !bible.copyright.is_empty() {
        ui.label(RichText::new(&bible.copyright).size(10.0).color(theme::TEXT_DIM));
    }

    ui.add_space(12.0);
    ui.separator();
    ui.heading("Import a translation");
    ui.label(
        RichText::new("A text file with one verse per line, either \"John 3:16 For God so…\" or tab-separated Book, Chapter, Verse, Text.")
            .size(11.0)
            .color(theme::TEXT_DIM),
    );
    ui.add(egui::TextEdit::singleline(&mut app.scripture.import_path).hint_text("File path"));
    ui.add(egui::TextEdit::singleline(&mut app.scripture.import_name).hint_text("Name, e.g. World English Bible"));
    ui.add(egui::TextEdit::singleline(&mut app.scripture.import_abbreviation).hint_text("Abbreviation, e.g. WEB"));
    if ui.button("Import").clicked() {
        *action = Some(Action::Import);
    }
}

/// Create (or refresh) the passage's presentation; with `live`, put its first slide on screen.
pub fn build(app: &mut App, bible: &Bible, reference: Option<Reference>, live: bool) {
    let Some(r) = reference else {
        app.notices.push(format!("\"{}\" is not a reference PresentCraft understands.", app.scripture.reference));
        return;
    };
    let Some(mut p) = bible::presentation(&mut app.lib, bible, &r, app.scripture.per_slide, app.scripture.theme, app.scripture.verse_numbers) else {
        app.notices.push(format!("{} has no verses for {}.", bible.abbreviation, r.label()));
        return;
    };
    p.name = format!("{} ({})", r.label(), bible.abbreviation);
    // Looking the same passage up again refreshes it rather than piling up copies.
    let existing = app.lib.presentations.iter().position(|x| x.category == "Scripture" && x.name == p.name);
    let id = match existing.and_then(|i| app.lib.presentations.get_mut(i)) {
        Some(old) => {
            old.groups = p.groups;
            old.theme = p.theme;
            old.id
        }
        None => {
            let id = p.id;
            app.lib.presentations.push(p);
            id
        }
    };
    app.selected_presentation = Some(id);
    app.changed();
    if live {
        let first = app.lib.presentation(id).and_then(|p| p.cues().first().and_then(|c| p.slide(*c)).map(|s| s.id));
        if let Some(s) = first {
            app.trigger(id, s);
        }
    }
}

fn import(app: &mut App) {
    let s = &app.scripture;
    let path = s.import_path.trim().trim_matches('"').to_string();
    let abbreviation = s.import_abbreviation.trim().to_string();
    let name = if s.import_name.trim().is_empty() { abbreviation.clone() } else { s.import_name.trim().to_string() };
    if path.is_empty() || abbreviation.is_empty() {
        app.notices.push("Give the file path and an abbreviation to import a translation.".into());
        return;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            app.notices.push(format!("{path} could not be read: {e}"));
            return;
        }
    };
    let (bible, skipped) = Bible::import(&name, &abbreviation, &text);
    if bible.verses.is_empty() {
        app.notices.push(format!("No verses were found in {path}."));
        return;
    }
    let count = bible.verses.len();
    if let Some(dir) = bible::bibles_dir(app.path.as_deref())
        && let Err(e) = bible::save(&dir, &bible)
    {
        app.notices.push(format!("{abbreviation} is loaded for now but could not be saved: {e}"));
    }
    app.bibles.retain(|b| !b.abbreviation.eq_ignore_ascii_case(&abbreviation));
    app.bibles.push(bible);
    app.scripture.bible = app.bibles.len() - 1;
    let note = if skipped > 0 { format!(" ({skipped} lines were not verses and were skipped)") } else { String::new() };
    app.notices.push(format!("Imported {count} verses of {abbreviation}{note}."));
}
