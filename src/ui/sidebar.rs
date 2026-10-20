//! The left sidebar: the presentation library and playlists.

use egui::{Color32, RichText, Sense, Ui, vec2};

use super::{section_header, theme};
use crate::app::{App, NewPresentation, Renaming};
use crate::model::*;

/// Payload for dragging a presentation from the library onto a playlist.
#[derive(Clone, Copy)]
struct DragPresentation(Id);

enum Action {
    Select(Id, Option<Id>),
    Rename(Renaming, String),
    Commit,
    AddToPlaylist(Id, Id),
    Duplicate(Id),
    Delete(Id),
    NewPlaylist,
    DeletePlaylist(Id),
    AddHeader(Id),
    MoveItem(Id, Id, i32),
    RemoveItem(Id, Id),
    SelectPlaylist(Id),
    TriggerMedia(Id),
}

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::Panel::left("sidebar").resizable(true).default_size(240.0).size_range(180.0..=420.0).frame(egui::Frame::new().fill(theme::PANEL_BG)).show(
        ui,
        |ui| {
            let mut actions = Vec::new();
            egui::Frame::new().inner_margin(6).show(ui, |ui| {
                ui.add(egui::TextEdit::singleline(&mut app.search).hint_text("Search library").desired_width(f32::INFINITY));
            });
            let full = ui.available_height();
            ui.allocate_ui(vec2(ui.available_width(), full * 0.5), |ui| {
                section_header(ui, "Library", |ui| {
                    if ui.small_button("+").on_hover_text("New presentation").clicked() {
                        app.new_presentation = Some(NewPresentation::default());
                    }
                });
                egui::ScrollArea::vertical().id_salt("library").auto_shrink([false, false]).show(ui, |ui| library(app, ui, &mut actions));
            });
            ui.separator();
            section_header(ui, "Playlists", |ui| {
                if ui.small_button("+").on_hover_text("New playlist").clicked() {
                    actions.push(Action::NewPlaylist);
                }
            });
            egui::ScrollArea::vertical().id_salt("playlists").auto_shrink([false, false]).show(ui, |ui| playlists(app, ui, &mut actions));
            apply(app, actions);
        },
    );
}

fn rename_field(app: &mut App, ui: &mut Ui, target: Renaming, actions: &mut Vec<Action>) -> bool {
    let Some((t, text)) = &mut app.renaming else {
        return false;
    };
    if *t != target {
        return false;
    }
    let r = ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY));
    r.request_focus();
    if r.lost_focus() {
        actions.push(Action::Commit);
    }
    true
}

fn library(app: &mut App, ui: &mut Ui, actions: &mut Vec<Action>) {
    let needle = app.search.to_lowercase();
    let mut list: Vec<(Id, String, String)> = app
        .lib
        .presentations
        .iter()
        .filter(|p| {
            needle.is_empty()
                || p.name.to_lowercase().contains(&needle)
                || p.groups.iter().flat_map(|g| &g.slides).any(|s| s.text().to_lowercase().contains(&needle))
        })
        .map(|p| (p.id, p.name.clone(), p.category.clone()))
        .collect();
    list.sort_by_key(|a| a.1.to_lowercase());
    let playlists: Vec<(Id, String)> = app.lib.playlists.iter().map(|p| (p.id, p.name.clone())).collect();
    for (id, name, category) in list {
        if rename_field(app, ui, Renaming::Presentation(id), actions) {
            continue;
        }
        let selected = app.selected_presentation == Some(id);
        let live = app.live.slide.map(|l| l.presentation) == Some(id);
        let r = ui.dnd_drag_source(egui::Id::new(("lib", id)), DragPresentation(id), |ui| row(ui, &name, &category, selected, live, None)).response;
        if r.clicked() {
            actions.push(Action::Select(id, None));
        }
        if r.double_clicked() {
            actions.push(Action::Rename(Renaming::Presentation(id), name.clone()));
        }
        r.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                actions.push(Action::Rename(Renaming::Presentation(id), name.clone()));
            }
            ui.menu_button("Add to playlist", |ui| {
                for (pl, pl_name) in &playlists {
                    if ui.button(pl_name).clicked() {
                        actions.push(Action::AddToPlaylist(*pl, id));
                    }
                }
            });
            if ui.button("Duplicate").clicked() {
                actions.push(Action::Duplicate(id));
            }
            ui.separator();
            if ui.button(RichText::new("Delete").color(theme::DANGER)).clicked() {
                actions.push(Action::Delete(id));
            }
        });
    }
}

/// A full-width selectable row with an optional color chip; returns its click response.
fn row(ui: &mut Ui, name: &str, detail: &str, selected: bool, live: bool, chip: Option<Color32>) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
    let fill = if selected {
        theme::SELECTED
    } else if r.hovered() {
        theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    let mut x = rect.left() + 10.0;
    if let Some(c) = chip {
        ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(x, rect.top() + 6.0), vec2(4.0, 12.0)), 1.0, c);
        x += 10.0;
    }
    let color = if live { theme::LIVE } else { theme::TEXT };
    ui.painter().with_clip_rect(rect).text(egui::pos2(x, rect.center().y), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(13.0), color);
    if !detail.is_empty() {
        ui.painter().with_clip_rect(rect).text(
            egui::pos2(rect.right() - 8.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            detail,
            egui::FontId::proportional(10.0),
            theme::TEXT_DIM,
        );
    }
    r
}

fn playlists(app: &mut App, ui: &mut Ui, actions: &mut Vec<Action>) {
    let pls: Vec<Playlist> = app.lib.playlists.clone();
    for pl in pls {
        if rename_field(app, ui, Renaming::Playlist(pl.id), actions) {
            continue;
        }
        let selected = app.selected_playlist == Some(pl.id);
        let (resp, dropped) = ui.dnd_drop_zone::<DragPresentation, _>(egui::Frame::new(), |ui| {
            let r = row(ui, &pl.name, &format!("{}", pl.items.len()), selected && app.selected_presentation.is_none(), false, None);
            if r.clicked() {
                actions.push(Action::SelectPlaylist(pl.id));
            }
            r.context_menu(|ui| {
                if ui.button("Rename").clicked() {
                    actions.push(Action::Rename(Renaming::Playlist(pl.id), pl.name.clone()));
                }
                if ui.button("Add header").clicked() {
                    actions.push(Action::AddHeader(pl.id));
                }
                ui.separator();
                if ui.button(RichText::new("Delete playlist").color(theme::DANGER)).clicked() {
                    actions.push(Action::DeletePlaylist(pl.id));
                }
            });
            for item in &pl.items {
                let iid = item.id();
                let (name, chip, pres, is_live, is_sel) = match item {
                    PlaylistItem::Presentation { presentation, .. } => {
                        let n = app.lib.presentation(*presentation).map(|p| p.name.clone()).unwrap_or_else(|| "Missing".into());
                        let live = app.live.slide.map(|l| l.presentation) == Some(*presentation);
                        (format!("    {n}"), None, Some(*presentation), live, app.selected_presentation == Some(*presentation) && selected)
                    }
                    PlaylistItem::Header { name, color, .. } => (name.to_uppercase(), Some(color.to_egui()), None, false, false),
                    PlaylistItem::Media { media, .. } => {
                        let n = app.lib.media_item(*media).map(|m| m.name.clone()).unwrap_or_else(|| "Missing".into());
                        (format!("    ▣ {n}"), None, None, app.live.media == Some(*media), false)
                    }
                };
                let r = row(ui, &name, "", is_sel, is_live, chip);
                if r.clicked() {
                    if let Some(p) = pres {
                        actions.push(Action::Select(p, Some(pl.id)));
                    } else if let PlaylistItem::Media { media, .. } = item {
                        actions.push(Action::TriggerMedia(*media));
                    }
                }
                r.context_menu(|ui| {
                    if ui.button("Move up").clicked() {
                        actions.push(Action::MoveItem(pl.id, iid, -1));
                    }
                    if ui.button("Move down").clicked() {
                        actions.push(Action::MoveItem(pl.id, iid, 1));
                    }
                    ui.separator();
                    if ui.button("Remove from playlist").clicked() {
                        actions.push(Action::RemoveItem(pl.id, iid));
                    }
                });
            }
        });
        let _ = resp;
        if let Some(d) = dropped {
            actions.push(Action::AddToPlaylist(pl.id, d.0));
        }
    }
}

fn apply(app: &mut App, actions: Vec<Action>) {
    for a in actions {
        match a {
            Action::Select(id, playlist) => {
                app.selected_presentation = Some(id);
                app.selected_slide = None;
                app.selected_element = None;
                app.reflow = None;
                if let Some(pl) = playlist {
                    app.selected_playlist = Some(pl);
                    app.live.playlist = Some(pl);
                }
            }
            Action::SelectPlaylist(id) => {
                app.selected_playlist = Some(id);
                app.live.playlist = Some(id);
            }
            Action::Rename(target, text) => app.renaming = Some((target, text)),
            Action::Commit => {
                if let Some((target, text)) = app.renaming.take() {
                    let text = text.trim().to_string();
                    if text.is_empty() {
                        continue;
                    }
                    match target {
                        Renaming::Presentation(id) => {
                            if let Some(p) = app.lib.presentation_mut(id) {
                                p.name = text;
                            }
                        }
                        Renaming::Playlist(id) => {
                            if let Some(p) = app.lib.playlists.iter_mut().find(|p| p.id == id) {
                                p.name = text;
                            }
                        }
                        Renaming::Group(pid, gid) => {
                            if let Some(g) = app.lib.presentation_mut(pid).and_then(|p| p.groups.iter_mut().find(|g| g.id == gid)) {
                                g.name = text;
                            }
                        }
                        Renaming::Media(id) => {
                            if let Some(m) = app.lib.media.iter_mut().find(|m| m.id == id) {
                                m.name = text;
                            }
                        }
                    }
                    app.changed();
                }
            }
            Action::AddToPlaylist(pl, pres) => {
                let id = app.lib.next_id();
                if let Some(p) = app.lib.playlists.iter_mut().find(|p| p.id == pl) {
                    p.items.push(PlaylistItem::Presentation { id, presentation: pres });
                }
                app.changed();
            }
            Action::Duplicate(id) => {
                if let Some(mut copy) = app.lib.presentation(id).cloned() {
                    copy.id = app.lib.next_id();
                    copy.name = format!("{} copy", copy.name);
                    // Fresh slide ids keep "live" tracking unambiguous between the two copies.
                    for g in &mut copy.groups {
                        for s in &mut g.slides {
                            s.id = app.lib.next_id();
                        }
                    }
                    app.selected_presentation = Some(copy.id);
                    app.lib.presentations.push(copy);
                    app.changed();
                }
            }
            Action::Delete(id) => {
                app.lib.remove_presentation(id);
                if app.selected_presentation == Some(id) {
                    app.selected_presentation = None;
                }
                if app.live.slide.map(|l| l.presentation) == Some(id) {
                    app.live.clear_slide(std::time::Instant::now());
                }
                app.changed();
            }
            Action::NewPlaylist => {
                let id = app.lib.next_id();
                app.lib.playlists.push(Playlist { id, name: "New playlist".into(), items: Vec::new() });
                app.renaming = Some((Renaming::Playlist(id), "New playlist".into()));
                app.changed();
            }
            Action::DeletePlaylist(id) => {
                app.lib.playlists.retain(|p| p.id != id);
                if app.selected_playlist == Some(id) {
                    app.selected_playlist = None;
                }
                app.changed();
            }
            Action::AddHeader(pl) => {
                let id = app.lib.next_id();
                if let Some(p) = app.lib.playlists.iter_mut().find(|p| p.id == pl) {
                    p.items.push(PlaylistItem::Header { id, name: "Header".into(), color: Rgba([45, 108, 223, 255]) });
                }
                app.changed();
            }
            Action::MoveItem(pl, item, dir) => {
                if let Some(p) = app.lib.playlists.iter_mut().find(|p| p.id == pl)
                    && let Some(i) = p.items.iter().position(|x| x.id() == item)
                {
                    let j = i as i64 + i64::from(dir);
                    if j >= 0 && (j as usize) < p.items.len() {
                        p.items.swap(i, j as usize);
                    }
                }
                app.changed();
            }
            Action::RemoveItem(pl, item) => {
                if let Some(p) = app.lib.playlists.iter_mut().find(|p| p.id == pl) {
                    p.items.retain(|x| x.id() != item);
                }
                app.changed();
            }
            Action::TriggerMedia(m) => app.live.set_media(Some(m), std::time::Instant::now()),
        }
    }
}
