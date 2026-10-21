//! Show mode: the selected presentation as a grid of slide thumbnails. Clicking a slide puts it
//! live; groups color the label bar under each thumbnail.

use egui::{Color32, RichText, Sense, Ui, pos2, vec2};

use super::theme;
use crate::app::{App, Mode, Renaming};
use crate::model::*;
use crate::render::{self, Painting};

enum Action {
    Trigger(Id),
    Select(Id),
    Edit(Id),
    Toggle(Id),
    Delete(Id),
    Duplicate(Id),
    InsertAfter(Id),
    Copy(Id),
    PasteAfter(Id),
    SetMedia(Id, Option<Id>),
    GroupColor(Id, Rgba),
    SplitGroup(Id),
    RenameGroup(Id, String),
    Arrangement(Option<Id>),
    NewArrangement,
    ArrangementPush(Id, Id),
    ArrangementRemove(Id, usize),
    DeleteArrangement(Id),
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let Some(p) = app.presentation().cloned() else {
        ui.centered_and_justified(|ui| ui.label(RichText::new("Select a presentation from the library or a playlist").color(theme::TEXT_DIM)));
        return;
    };
    let mut actions = Vec::new();
    header(app, ui, &p, &mut actions);
    if app.renaming.as_ref().is_some_and(|(r, _)| matches!(r, Renaming::Group(..))) {
        rename_group(app, ui);
    }

    let thumb_w = app.prefs.get().layout.thumbnail_width.clamp(120.0, 480.0);
    let thumb_h = thumb_w * CANVAS_H / CANVAS_W;
    let live = app.live.slide.filter(|l| l.presentation == p.id).map(|l| l.slide);
    let media: Vec<(Id, String)> = app.lib.media.iter().map(|m| (m.id, m.name.clone())).collect();

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing = vec2(10.0, 12.0);
        let mut number = 0;
        ui.horizontal_wrapped(|ui| {
            ui.add_space(8.0);
            for gi in p.play_groups() {
                let Some(group) = p.groups.get(gi) else {
                    continue;
                };
                for slide in &group.slides {
                    number += 1;
                    let (rect, r) = ui.allocate_exact_size(vec2(thumb_w, thumb_h + 22.0), Sense::click());
                    if !ui.is_rect_visible(rect) {
                        continue;
                    }
                    let screen = egui::Rect::from_min_size(rect.min, vec2(thumb_w, thumb_h));
                    let ctx = ui.ctx().clone();
                    Painting { lib: &app.lib, cache: &mut app.cache, ctx: &ctx }.thumbnail(ui.painter(), screen, slide);
                    let bar = egui::Rect::from_min_max(pos2(rect.left(), screen.bottom()), rect.max);
                    ui.painter().rect_filled(bar, 0.0, group.color.to_egui().gamma_multiply(0.85));
                    let label = if slide.label.is_empty() { group.name.clone() } else { slide.label.clone() };
                    ui.painter().with_clip_rect(bar).text(
                        bar.left_center() + vec2(6.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        format!("{number}  {label}"),
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                    if slide.media_action.is_some() {
                        ui.painter().text(
                            bar.right_center() - vec2(6.0, 0.0),
                            egui::Align2::RIGHT_CENTER,
                            "▣",
                            egui::FontId::proportional(12.0),
                            Color32::WHITE,
                        );
                    }
                    if !slide.enabled {
                        ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(160));
                        ui.painter().text(
                            screen.center(),
                            egui::Align2::CENTER_CENTER,
                            "Disabled",
                            egui::FontId::proportional(14.0),
                            theme::TEXT_DIM,
                        );
                    }
                    if live == Some(slide.id) {
                        render::outline(ui.painter(), rect, theme::LIVE, 3.0);
                    } else if app.selected_slide == Some(slide.id) {
                        render::outline(ui.painter(), rect, theme::ACCENT, 2.0);
                    } else if r.hovered() {
                        render::outline(ui.painter(), rect, theme::HOVER, 1.0);
                    }
                    if r.clicked() {
                        if slide.enabled {
                            actions.push(Action::Trigger(slide.id));
                        } else {
                            actions.push(Action::Select(slide.id));
                        }
                    }
                    if r.double_clicked() {
                        actions.push(Action::Edit(slide.id));
                    }
                    let sid = slide.id;
                    r.context_menu(|ui| slide_menu(ui, sid, slide, group, &media, app.clipboard.is_some(), &mut actions));
                }
            }
        });
        if p.slide_count() == 0 && ui.button("Add a slide").clicked() {
            actions.push(Action::InsertAfter(0));
        }
    });
    apply(app, p.id, actions);
}

fn slide_menu(ui: &mut Ui, sid: Id, slide: &Slide, group: &Group, media: &[(Id, String)], can_paste: bool, actions: &mut Vec<Action>) {
    if ui.button("Edit slide").clicked() {
        actions.push(Action::Edit(sid));
    }
    if ui.button(if slide.enabled { "Disable" } else { "Enable" }).clicked() {
        actions.push(Action::Toggle(sid));
    }
    ui.separator();
    ui.menu_button("Group", |ui| {
        if ui.button("Rename group…").clicked() {
            actions.push(Action::RenameGroup(group.id, group.name.clone()));
        }
        if ui.button("Start a new group here").clicked() {
            actions.push(Action::SplitGroup(sid));
        }
        ui.separator();
        for (name, color) in GROUP_COLORS {
            if ui.button(RichText::new(format!("■ {name}")).color(color.to_egui())).clicked() {
                actions.push(Action::GroupColor(group.id, *color));
            }
        }
    });
    ui.menu_button("Media action", |ui| {
        if ui.selectable_label(slide.media_action.is_none(), "None").clicked() {
            actions.push(Action::SetMedia(sid, None));
        }
        for (id, name) in media {
            if ui.selectable_label(slide.media_action == Some(*id), name).clicked() {
                actions.push(Action::SetMedia(sid, Some(*id)));
            }
        }
    });
    ui.separator();
    if ui.button("Copy").clicked() {
        actions.push(Action::Copy(sid));
    }
    if ui.add_enabled(can_paste, egui::Button::new("Paste after")).clicked() {
        actions.push(Action::PasteAfter(sid));
    }
    if ui.button("Duplicate").clicked() {
        actions.push(Action::Duplicate(sid));
    }
    if ui.button("New slide after").clicked() {
        actions.push(Action::InsertAfter(sid));
    }
    ui.separator();
    if ui.button(RichText::new("Delete slide").color(theme::DANGER)).clicked() {
        actions.push(Action::Delete(sid));
    }
}

fn header(app: &mut App, ui: &mut Ui, p: &Presentation, actions: &mut Vec<Action>) {
    egui::Frame::new().fill(theme::HEADER_BG).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(&p.name).strong().size(15.0));
            ui.label(RichText::new(format!("{} slides", p.slide_count())).color(theme::TEXT_DIM));
            ui.separator();
            let current = p
                .active_arrangement
                .and_then(|id| p.arrangements.iter().find(|a| a.id == id))
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Master".into());
            egui::ComboBox::from_id_salt("arrangement").selected_text(format!("Arrangement: {current}")).show_ui(ui, |ui| {
                if ui.selectable_label(p.active_arrangement.is_none(), "Master (all groups in order)").clicked() {
                    actions.push(Action::Arrangement(None));
                }
                for a in &p.arrangements {
                    if ui.selectable_label(p.active_arrangement == Some(a.id), &a.name).clicked() {
                        actions.push(Action::Arrangement(Some(a.id)));
                    }
                }
                ui.separator();
                if ui.button("New arrangement").clicked() {
                    actions.push(Action::NewArrangement);
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut w = app.prefs.get().layout.thumbnail_width;
                if ui.add(egui::Slider::new(&mut w, 120.0..=480.0).show_value(false)).on_hover_text("Thumbnail size").changed() {
                    app.prefs.update(|pr| pr.layout.thumbnail_width = w);
                }
                ui.label(RichText::new("Size").color(theme::TEXT_DIM));
            });
        });
        // Arrangement editor: the group order as removable chips, plus buttons to append a group.
        if let Some(a) = p.active_arrangement.and_then(|id| p.arrangements.iter().find(|a| a.id == id)) {
            ui.horizontal_wrapped(|ui| {
                for (i, gid) in a.groups.iter().enumerate() {
                    if let Some(g) = p.groups.iter().find(|g| g.id == *gid)
                        && ui
                            .add(egui::Button::new(RichText::new(format!("{} ×", g.name)).color(Color32::WHITE)).fill(g.color.to_egui()))
                            .on_hover_text("Remove from arrangement")
                            .clicked()
                    {
                        actions.push(Action::ArrangementRemove(a.id, i));
                    }
                }
                ui.menu_button("+ Group", |ui| {
                    for g in &p.groups {
                        if ui.button(&g.name).clicked() {
                            actions.push(Action::ArrangementPush(a.id, g.id));
                        }
                    }
                });
                if ui.small_button(RichText::new("Delete arrangement").color(theme::DANGER)).clicked() {
                    actions.push(Action::DeleteArrangement(a.id));
                }
            });
        }
    });
}

fn rename_group(app: &mut App, ui: &mut Ui) {
    let mut commit = false;
    ui.horizontal(|ui| {
        ui.label("Group name");
        if let Some((_, text)) = &mut app.renaming {
            let r = ui.text_edit_singleline(text);
            r.request_focus();
            commit = r.lost_focus();
        }
    });
    if commit
        && let Some((Renaming::Group(pid, gid), text)) = app.renaming.take()
        && let Some(g) = app.lib.presentation_mut(pid).and_then(|p| p.groups.iter_mut().find(|g| g.id == gid))
        && !text.trim().is_empty()
    {
        g.name = text.trim().to_string();
        app.changed();
    }
}

/// Where a slide sits: group index and slide index within the presentation.
fn locate(p: &Presentation, sid: Id) -> Option<(usize, usize)> {
    p.find_slide(sid).map(|(c, _)| (c.group, c.slide))
}

fn apply(app: &mut App, pid: Id, actions: Vec<Action>) {
    for a in actions {
        match a {
            Action::Trigger(sid) => app.trigger(pid, sid),
            Action::Select(sid) => app.selected_slide = Some(sid),
            Action::Edit(sid) => {
                app.selected_slide = Some(sid);
                app.selected_element = None;
                app.mode = Mode::Edit;
            }
            Action::RenameGroup(gid, name) => app.renaming = Some((Renaming::Group(pid, gid), name)),
            Action::Copy(sid) => app.clipboard = app.lib.presentation(pid).and_then(|p| p.find_slide(sid)).map(|(_, s)| s.clone()),
            other => {
                edit(app, pid, other);
                app.changed();
            }
        }
    }
}

fn edit(app: &mut App, pid: Id, action: Action) {
    let new_id = app.lib.next_id();
    let group_id = app.lib.next_id();
    let theme = app.lib.presentation(pid).and_then(|p| p.theme);
    let blank = app.lib.new_slide("", theme);
    let clipboard = app.clipboard.clone();
    let Some(p) = app.lib.presentation_mut(pid) else {
        return;
    };
    match action {
        Action::Toggle(sid) => {
            if let Some(s) = p.find_slide_mut(sid) {
                s.enabled = !s.enabled;
            }
        }
        Action::Delete(sid) => {
            if let Some((g, s)) = locate(p, sid) {
                p.groups[g].slides.remove(s);
            }
        }
        Action::Duplicate(sid) | Action::InsertAfter(sid) | Action::PasteAfter(sid) => {
            let insert = match &action {
                Action::Duplicate(_) => p.find_slide(sid).map(|(_, s)| s.clone()),
                Action::PasteAfter(_) => clipboard,
                _ => Some(blank),
            };
            if let Some(mut s) = insert {
                s.id = new_id;
                match locate(p, sid) {
                    Some((g, i)) => p.groups[g].slides.insert(i + 1, s),
                    None => {
                        if p.groups.is_empty() {
                            p.groups.push(Group { id: group_id, name: "Slides".into(), color: group_color("Verse"), slides: Vec::new() });
                        }
                        if let Some(g) = p.groups.last_mut() {
                            g.slides.push(s);
                        }
                    }
                }
            }
        }
        Action::SetMedia(sid, media) => {
            if let Some(s) = p.find_slide_mut(sid) {
                s.media_action = media;
            }
        }
        Action::GroupColor(gid, color) => {
            if let Some(g) = p.groups.iter_mut().find(|g| g.id == gid) {
                g.color = color;
            }
        }
        Action::SplitGroup(sid) => {
            if let Some((g, i)) = locate(p, sid)
                && i > 0
            {
                let rest = p.groups[g].slides.split_off(i);
                let name = format!("{} (2)", p.groups[g].name);
                let color = p.groups[g].color;
                p.groups.insert(g + 1, Group { id: new_id, name, color, slides: rest });
            }
        }
        Action::Arrangement(id) => p.active_arrangement = id,
        Action::NewArrangement => {
            let groups = p.groups.iter().map(|g| g.id).collect();
            let n = p.arrangements.len() + 1;
            p.arrangements.push(Arrangement { id: new_id, name: format!("Arrangement {n}"), groups });
            p.active_arrangement = Some(new_id);
        }
        Action::ArrangementPush(aid, gid) => {
            if let Some(a) = p.arrangements.iter_mut().find(|a| a.id == aid) {
                a.groups.push(gid);
            }
        }
        Action::ArrangementRemove(aid, i) => {
            if let Some(a) = p.arrangements.iter_mut().find(|a| a.id == aid)
                && i < a.groups.len()
            {
                a.groups.remove(i);
            }
        }
        Action::DeleteArrangement(aid) => {
            p.arrangements.retain(|a| a.id != aid);
            if p.active_arrangement == Some(aid) {
                p.active_arrangement = None;
            }
        }
        Action::Trigger(_) | Action::Select(_) | Action::Edit(_) | Action::RenameGroup(..) | Action::Copy(_) => {}
    }
}
