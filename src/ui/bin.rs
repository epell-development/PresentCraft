//! The bottom bin: media, props, messages, timers and themes.

use std::time::Instant;

use egui::{RichText, Sense, Ui, vec2};

use super::{tab_strip, theme};
use crate::app::{App, BinTab, Renaming};
use crate::model::*;
use crate::render::{self, Painting};

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::Panel::bottom("bin").resizable(true).default_size(210.0).size_range(120.0..=520.0).frame(egui::Frame::new().fill(theme::PANEL_BG)).show(
        ui,
        |ui| {
            let labels: Vec<&str> = BinTab::ALL.iter().map(|t| t.label()).collect();
            let selected = BinTab::ALL.iter().position(|t| *t == app.bin_tab).unwrap_or(0);
            if let Some(i) = tab_strip(ui, egui::Id::new("bin-tabs"), &labels, selected)
                && let Some(t) = BinTab::ALL.get(i)
            {
                app.bin_tab = *t;
            }
            egui::Frame::new().inner_margin(8).show(ui, |ui| {
                let before = app.lib.clone();
                match app.bin_tab {
                    BinTab::Media => media(app, ui),
                    BinTab::Props => props(app, ui),
                    BinTab::Messages => messages(app, ui),
                    BinTab::Timers => timers(app, ui),
                    BinTab::Themes => themes(app, ui),
                }
                if app.lib != before {
                    app.changed();
                }
            });
        },
    );
}

fn media(app: &mut App, ui: &mut Ui) {
    let now = Instant::now();
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.import_path).hint_text("Image file path, or drop files on the window").desired_width(320.0));
        if ui.button("Import").clicked() {
            let path = std::mem::take(&mut app.import_path);
            app.import_media(&path);
        }
        ui.separator();
        if ui.button("+ Color").clicked() {
            let id = app.lib.next_id();
            app.lib.media.push(MediaItem {
                id,
                name: "Color".into(),
                source: MediaSource::Color { color: Rgba([40, 60, 120, 255]) },
                fit: MediaFit::Fill,
                bin: "Backgrounds".into(),
            });
        }
        if ui.button("+ Gradient").clicked() {
            let id = app.lib.next_id();
            app.lib.media.push(MediaItem {
                id,
                name: "Gradient".into(),
                source: MediaSource::Gradient { top: Rgba([60, 40, 120, 255]), bottom: Rgba([10, 10, 30, 255]) },
                fit: MediaFit::Fill,
                bin: "Backgrounds".into(),
            });
        }
    });
    ui.add_space(6.0);
    let items = app.lib.media.clone();
    let playlists: Vec<(Id, String)> = app.lib.playlists.iter().map(|p| (p.id, p.name.clone())).collect();
    let mut remove = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for m in &items {
                if let Some((Renaming::Media(id), text)) = &mut app.renaming
                    && *id == m.id
                {
                    let r = ui.add(egui::TextEdit::singleline(text).desired_width(140.0));
                    r.request_focus();
                    if r.lost_focus()
                        && let Some((_, name)) = app.renaming.take()
                        && let Some(mm) = app.lib.media.iter_mut().find(|x| x.id == m.id)
                    {
                        mm.name = name;
                    }
                    continue;
                }
                let (rect, r) = ui.allocate_exact_size(vec2(140.0, 96.0), Sense::click());
                let screen = egui::Rect::from_min_size(rect.min, vec2(140.0, 78.75));
                let ctx = ui.ctx().clone();
                Painting { lib: &app.lib, cache: &mut app.cache, ctx: &ctx }.media(ui.painter(), screen, m.id, 1.0);
                if let Some(err) = app.cache.error(m.id) {
                    ui.painter().text(screen.center(), egui::Align2::CENTER_CENTER, "Missing file", egui::FontId::proportional(11.0), theme::DANGER);
                    r.clone().on_hover_text(err.to_string());
                }
                ui.painter().with_clip_rect(rect).text(
                    egui::pos2(rect.left() + 2.0, rect.bottom() - 8.0),
                    egui::Align2::LEFT_CENTER,
                    &m.name,
                    egui::FontId::proportional(11.0),
                    theme::TEXT,
                );
                if app.live.media == Some(m.id) {
                    render::outline(ui.painter(), screen, theme::LIVE, 2.5);
                } else if r.hovered() {
                    render::outline(ui.painter(), screen, theme::HOVER, 1.0);
                }
                if r.clicked() {
                    if app.live.media == Some(m.id) {
                        app.live.clear_media(now);
                    } else {
                        app.live.set_media(Some(m.id), now);
                    }
                }
                r.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        app.renaming = Some((Renaming::Media(m.id), m.name.clone()));
                    }
                    ui.menu_button("Scaling", |ui| {
                        for (fit, name) in [(MediaFit::Fill, "Fill (crop)"), (MediaFit::Fit, "Fit (letterbox)"), (MediaFit::Stretch, "Stretch")] {
                            if ui.selectable_label(m.fit == fit, name).clicked()
                                && let Some(mm) = app.lib.media.iter_mut().find(|x| x.id == m.id)
                            {
                                mm.fit = fit;
                            }
                        }
                    });
                    if let MediaSource::Color { .. } | MediaSource::Gradient { .. } = m.source {
                        ui.menu_button("Colors", |ui| {
                            if let Some(mm) = app.lib.media.iter_mut().find(|x| x.id == m.id) {
                                match &mut mm.source {
                                    MediaSource::Color { color } => color_row(ui, "Color", color),
                                    MediaSource::Gradient { top, bottom } => {
                                        color_row(ui, "Top", top);
                                        color_row(ui, "Bottom", bottom);
                                    }
                                    MediaSource::Image { .. } => {}
                                }
                            }
                        });
                    }
                    ui.menu_button("Add to playlist", |ui| {
                        for (pl, name) in &playlists {
                            if ui.button(name).clicked() {
                                let id = app.lib.next_id();
                                if let Some(p) = app.lib.playlists.iter_mut().find(|p| p.id == *pl) {
                                    p.items.push(PlaylistItem::Media { id, media: m.id });
                                }
                            }
                        }
                    });
                    if let MediaSource::Image { .. } = m.source
                        && ui.button("Reload file").clicked()
                    {
                        app.cache.forget(m.id);
                    }
                    ui.separator();
                    if ui.button(RichText::new("Delete").color(theme::DANGER)).clicked() {
                        remove = Some(m.id);
                    }
                });
            }
        });
    });
    if let Some(id) = remove {
        app.lib.remove_media(id);
        app.cache.forget(id);
        if app.live.media == Some(id) {
            app.live.clear_media(now);
        }
    }
}

fn color_row(ui: &mut Ui, label: &str, c: &mut Rgba) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut col = c.to_egui();
        if ui.color_edit_button_srgba(&mut col).changed() {
            *c = Rgba::from_egui(col);
        }
    });
}

fn props(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        if ui.button("+ Lower third").clicked() {
            let (id, el) = (app.lib.next_id(), app.lib.next_id());
            app.lib.props.push(Prop {
                id,
                name: "Lower third".into(),
                elements: vec![Element {
                    id: el,
                    name: "Name".into(),
                    frame: Frame { x: 100.0, y: 820.0, w: 900.0, h: 140.0 },
                    kind: ElementKind::Text {
                        text: "Speaker name".into(),
                        style: TextStyle { size: 56.0, h_align: HAlign::Left, ..TextStyle::default() },
                    },
                    fill: Some(Rgba([20, 30, 60, 220])),
                }],
            });
        }
        ui.label(RichText::new("Props sit above slides and stay up while slides change.").color(theme::TEXT_DIM));
    });
    let mut remove = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for prop in &mut app.lib.props {
            ui.push_id(prop.id, |ui| {
                ui.horizontal(|ui| {
                    let on = app.live.props.contains(&prop.id);
                    if ui.add(egui::Button::new(if on { "• On" } else { "Off" }).selected(on).min_size(vec2(56.0, 0.0))).clicked() {
                        app.live.toggle_prop(prop.id);
                    }
                    ui.add(egui::TextEdit::singleline(&mut prop.name).desired_width(140.0));
                    if let Some(e) = prop.elements.first_mut()
                        && let ElementKind::Text { text, .. } = &mut e.kind
                    {
                        ui.add(egui::TextEdit::singleline(text).desired_width(260.0));
                    }
                    if ui.small_button(RichText::new("Delete").color(theme::DANGER)).clicked() {
                        remove = Some(prop.id);
                    }
                });
            });
        }
    });
    if let Some(id) = remove {
        app.lib.props.retain(|p| p.id != id);
        app.live.props.retain(|p| *p != id);
    }
}

fn messages(app: &mut App, ui: &mut Ui) {
    let now = Instant::now();
    if ui.button("+ Message").clicked() {
        let id = app.lib.next_id();
        app.lib.messages.push(Message { id, name: "Message".into(), template: "Your text with {Token}".into(), timer: None });
    }
    let timers: Vec<(Id, String)> = app.lib.timers.iter().map(|t| (t.id, t.name.clone())).collect();
    let mut remove = None;
    let mut show = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for m in &mut app.lib.messages {
            ui.push_id(m.id, |ui| {
                ui.horizontal(|ui| {
                    let on = app.live.message.as_ref().is_some_and(|(id, _)| *id == m.id);
                    if ui.add(egui::Button::new(if on { "• Showing" } else { "Show" }).selected(on).min_size(vec2(72.0, 0.0))).clicked() {
                        if on {
                            app.live.message = None;
                        } else {
                            show = Some(m.id);
                        }
                    }
                    ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(120.0));
                    ui.add(egui::TextEdit::singleline(&mut m.template).desired_width(300.0));
                    egui::ComboBox::from_id_salt("timer")
                        .selected_text(m.timer.and_then(|t| timers.iter().find(|(id, _)| *id == t)).map(|(_, n)| n.as_str()).unwrap_or("No timer"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut m.timer, None, "No timer");
                            for (id, name) in &timers {
                                ui.selectable_value(&mut m.timer, Some(*id), name);
                            }
                        });
                    if ui.small_button(RichText::new("Delete").color(theme::DANGER)).clicked() {
                        remove = Some(m.id);
                    }
                });
                let tokens: Vec<String> = m.tokens().into_iter().filter(|t| t != "timer").collect();
                if !tokens.is_empty() {
                    let values = app.message_values.entry(m.id).or_default();
                    ui.horizontal(|ui| {
                        ui.add_space(80.0);
                        for t in tokens {
                            ui.label(RichText::new(&t).color(theme::TEXT_DIM));
                            let idx = match values.iter().position(|(k, _)| *k == t) {
                                Some(i) => i,
                                None => {
                                    values.push((t.clone(), String::new()));
                                    values.len() - 1
                                }
                            };
                            if let Some((_, v)) = values.get_mut(idx) {
                                ui.add(egui::TextEdit::singleline(v).desired_width(90.0));
                            }
                        }
                    });
                }
            });
        }
    });
    if let Some(id) = show
        && let Some(m) = app.lib.messages.iter().find(|m| m.id == id)
    {
        let values = app.message_values.get(&id).cloned().unwrap_or_default();
        let timer = m.timer.and_then(|t| app.lib.timers.iter().find(|x| x.id == t)).map(|t| app.live.timer_text(t, now));
        app.live.message = Some((id, m.fill(&values, timer.as_deref())));
    }
    if let Some(id) = remove {
        app.lib.messages.retain(|m| m.id != id);
        if app.live.message.as_ref().is_some_and(|(m, _)| *m == id) {
            app.live.message = None;
        }
    }
}

/// A message with a timer re-renders every frame so its countdown keeps moving on screen.
pub fn refresh_message(app: &mut App, now: Instant) {
    let Some((id, _)) = app.live.message.clone() else {
        return;
    };
    let Some(m) = app.lib.messages.iter().find(|m| m.id == id) else {
        return;
    };
    let Some(t) = m.timer.and_then(|t| app.lib.timers.iter().find(|x| x.id == t)) else {
        return;
    };
    let values = app.message_values.get(&id).cloned().unwrap_or_default();
    let text = m.fill(&values, Some(&app.live.timer_text(t, now)));
    app.live.message = Some((id, text));
}

fn timers(app: &mut App, ui: &mut Ui) {
    let now = Instant::now();
    if ui.button("+ Timer").clicked() {
        let id = app.lib.next_id();
        app.lib.timers.push(Timer { id, name: "Timer".into(), kind: TimerKind::Countdown { seconds: 600 }, overrun: false });
    }
    let style = theme::toolbar_button();
    let mut remove = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for t in &mut app.lib.timers {
            ui.push_id(t.id, |ui| {
                ui.horizontal(|ui| {
                    super::preview::timer_row(&mut app.live, ui, t, now, &style);
                    ui.separator();
                    ui.add(egui::TextEdit::singleline(&mut t.name).desired_width(140.0));
                    let kind_name = match t.kind {
                        TimerKind::Countdown { .. } => "Countdown",
                        TimerKind::CountdownTo { .. } => "Count down to time",
                        TimerKind::Elapsed => "Elapsed",
                    };
                    egui::ComboBox::from_id_salt("kind").selected_text(kind_name).show_ui(ui, |ui| {
                        if ui.selectable_label(matches!(t.kind, TimerKind::Countdown { .. }), "Countdown").clicked() {
                            t.kind = TimerKind::Countdown { seconds: 300 };
                        }
                        if ui.selectable_label(matches!(t.kind, TimerKind::CountdownTo { .. }), "Count down to time").clicked() {
                            t.kind = TimerKind::CountdownTo { hour: 10, minute: 0 };
                        }
                        if ui.selectable_label(matches!(t.kind, TimerKind::Elapsed), "Elapsed").clicked() {
                            t.kind = TimerKind::Elapsed;
                        }
                    });
                    match &mut t.kind {
                        TimerKind::Countdown { seconds } => {
                            let mut minutes = *seconds / 60;
                            let mut secs = *seconds % 60;
                            ui.add(egui::DragValue::new(&mut minutes).range(0..=999).suffix(" min"));
                            ui.add(egui::DragValue::new(&mut secs).range(0..=59).suffix(" s"));
                            *seconds = minutes * 60 + secs;
                        }
                        TimerKind::CountdownTo { hour, minute } => {
                            ui.add(egui::DragValue::new(hour).range(0..=23).prefix("at "));
                            ui.label(":");
                            ui.add(egui::DragValue::new(minute).range(0..=59));
                        }
                        TimerKind::Elapsed => {}
                    }
                    ui.checkbox(&mut t.overrun, "Overrun");
                    if ui.small_button(RichText::new("Delete").color(theme::DANGER)).clicked() {
                        remove = Some(t.id);
                    }
                });
            });
        }
    });
    if let Some(id) = remove {
        app.lib.timers.retain(|t| t.id != id);
        app.live.reset_timer(id);
    }
}

fn themes(app: &mut App, ui: &mut Ui) {
    let current = app.presentation().map(|p| p.id);
    let mut apply = None;
    let mut remove = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for t in &mut app.lib.themes {
                ui.push_id(t.id, |ui| {
                    egui::Frame::new().fill(theme::HEADER_BG).inner_margin(6).corner_radius(4).show(ui, |ui| {
                        ui.set_width(260.0);
                        ui.text_edit_singleline(&mut t.name);
                        let (rect, _) = ui.allocate_exact_size(vec2(248.0, 139.5), Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, t.background.map(|b| b.to_egui()).unwrap_or(egui::Color32::from_rgb(30, 40, 70)));
                        crate::render::paint_text(
                            ui.painter(),
                            rect,
                            crate::render::frame_rect(rect, t.frame),
                            "Sample lyric line\nSecond line",
                            &t.style,
                            1.0,
                        );
                        ui.collapsing("Style", |ui| {
                            super::editor::text_style(ui, &mut t.style);
                            let mut has = t.background.is_some();
                            if ui.checkbox(&mut has, "Background").changed() {
                                t.background = has.then_some(Rgba::BLACK);
                            }
                            if let Some(bg) = &mut t.background {
                                color_row(ui, "Color", bg);
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.add_enabled(current.is_some(), egui::Button::new("Apply to presentation")).clicked() {
                                apply = Some(t.clone());
                            }
                            if ui.small_button(RichText::new("Delete").color(theme::DANGER)).clicked() {
                                remove = Some(t.id);
                            }
                        });
                    });
                });
            }
        });
    });
    if let Some(t) = apply
        && let Some(p) = current.and_then(|id| app.lib.presentation_mut(id))
    {
        p.theme = Some(t.id);
        super::editor::apply_theme(p, &t);
    }
    if let Some(id) = remove {
        app.lib.themes.retain(|t| t.id != id);
        for p in &mut app.lib.presentations {
            if p.theme == Some(id) {
                p.theme = None;
            }
        }
    }
}
