//! The right panel: audience preview, layer clear buttons, next-slide preview, transition,
//! logo, stage message and quick timer controls.

use std::time::{Duration, Instant};

use egui::{RichText, Sense, Ui, vec2};

use super::icons::{self, Icon};
use super::{section_header, theme};
use crate::app::App;
use crate::live::{self, Live};
use crate::model::*;
use crate::render::{self, Painting};

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::Panel::right("preview").resizable(true).default_size(340.0).size_range(260.0..=640.0).frame(egui::Frame::new().fill(theme::PANEL_BG)).show(
        ui,
        |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let now = Instant::now();
                section_header(ui, "Audience", |ui| {
                    let on_air = app.live.slide.is_some()
                        || app.live.media.is_some()
                        || !app.live.props.is_empty()
                        || app.live.message.is_some()
                        || app.live.logo_on;
                    ui.label(RichText::new(if on_air { "• LIVE" } else { "○ CLEAR" }).size(11.0).color(if on_air {
                        theme::LIVE
                    } else {
                        theme::TEXT_DIM
                    }));
                });
                let w = ui.available_width() - 12.0;
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    let (rect, r) = ui.allocate_exact_size(vec2(w, w * 9.0 / 16.0), Sense::click());
                    let ctx = ui.ctx().clone();
                    Painting { lib: &app.lib, cache: &mut app.cache, ctx: &ctx }.output(ui.painter(), rect, &app.live, now);
                    render::outline(ui.painter(), rect, if app.live.slide.is_some() { theme::LIVE } else { theme::HOVER }, 1.5);
                    if r.double_clicked() {
                        app.audience_open = true;
                    }
                    r.on_hover_text("Double-click to open the audience window");
                });
                ui.add_space(6.0);
                clear_buttons(app, ui, now);
                ui.add_space(4.0);
                next_preview(app, ui);
                audio_transport(app, ui);
                show_settings(app, ui);
                stage_controls(app, ui);
                timers(app, ui, now);
            });
        },
    );
}

fn clear_buttons(app: &mut App, ui: &mut Ui, now: Instant) {
    let live = &app.live;
    let audio_on = app.audio.current.is_some();
    let states = [
        (
            Icon::ClearAll,
            "Clear All",
            "F1",
            live.slide.is_some() || live.media.is_some() || !live.props.is_empty() || live.message.is_some() || audio_on,
        ),
        (Icon::Slide, "Slide", "F2", live.slide.is_some()),
        (Icon::Media, "Media", "F3", live.media.is_some()),
        (Icon::Props, "Props", "F4", !live.props.is_empty()),
        (Icon::Message, "Messages", "F5", live.message.is_some()),
        (Icon::Logo, "Logo", "F6", live.logo_on),
        (Icon::Audio, "Audio", "F7", audio_on),
    ];
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        let size = ((ui.available_width() - 6.0 * 8.0) / 7.0).clamp(28.0, 56.0);
        for (i, (icon, label, key, active)) in states.into_iter().enumerate() {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                let style = theme::clear_button(active);
                if icons::small(ui, icon, &format!("{label} ({key})"), size, &style).clicked() {
                    clicked = Some(i);
                }
                ui.add_sized(vec2(size, 12.0), egui::Label::new(RichText::new(label).size(9.5).color(theme::TEXT_DIM)).selectable(false));
            });
        }
    });
    if clicked == Some(0) {
        app.clear_all();
        return;
    }
    if clicked == Some(6) {
        app.audio.stop();
        return;
    }
    let live = &mut app.live;
    match clicked {
        Some(1) => live.clear_slide(now),
        Some(2) => live.clear_media(now),
        Some(3) => live.clear_props(),
        Some(4) => live.clear_messages(),
        Some(5) => {
            if live.logo.is_none() {
                app.notices.push("Pick a logo under Show settings first.".into());
            }
            live.toggle_logo(now);
        }
        _ => {}
    }
}

fn next_preview(app: &mut App, ui: &mut Ui) {
    section_header(ui, "Next", |_| {});
    let next = if app.live.slide.is_some() {
        app.live.next_cue(&app.lib)
    } else {
        app.presentation()
            .and_then(|p| p.cues().first().and_then(|c| p.slide(*c)).map(|s| crate::live::LiveSlide { presentation: p.id, slide: s.id }))
    };
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        let w = (ui.available_width() - 12.0) * 0.5;
        let (rect, r) = ui.allocate_exact_size(vec2(w, w * 9.0 / 16.0), Sense::click());
        let slide = next.and_then(|n| app.lib.presentation(n.presentation).and_then(|p| p.find_slide(n.slide)).map(|(_, s)| s.clone()));
        let ctx = ui.ctx().clone();
        match &slide {
            Some(s) => Painting { lib: &app.lib, cache: &mut app.cache, ctx: &ctx }.thumbnail(ui.painter(), rect, s),
            None => {
                ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "End", egui::FontId::proportional(12.0), theme::TEXT_DIM);
            }
        }
        if r.clicked() {
            app.go_next();
        }
        ui.vertical(|ui| {
            if let Some(n) = next
                && let Some(p) = app.lib.presentation(n.presentation)
            {
                ui.label(RichText::new(&p.name).color(theme::TEXT_DIM).size(11.0));
            }
            ui.label(RichText::new("Space or Right: next\nLeft: previous").size(10.5).color(theme::TEXT_DIM));
        });
    });
    ui.add_space(4.0);
}

fn show_settings(app: &mut App, ui: &mut Ui) {
    section_header(ui, "Show settings", |_| {});
    egui::Frame::new().inner_margin(8).show(ui, |ui| {
        let mut ms = app.prefs.get().show.transition_ms;
        if ui.add(egui::Slider::new(&mut ms, 0..=3000).text("Transition ms")).changed() {
            app.prefs.update(|p| p.show.transition_ms = ms);
            app.live.transition = Duration::from_millis(u64::from(ms));
        }
        let mut adv = app.prefs.get().show.advance_through_playlist;
        if ui.checkbox(&mut adv, "Continue into next playlist item").changed() {
            app.prefs.update(|p| p.show.advance_through_playlist = adv);
        }
        let media: Vec<(Id, String)> =
            app.lib.media.iter().filter(|m| !matches!(m.source, MediaSource::Audio { .. })).map(|m| (m.id, m.name.clone())).collect();
        egui::ComboBox::from_label("Logo")
            .selected_text(app.live.logo.and_then(|l| media.iter().find(|(id, _)| *id == l)).map(|(_, n)| n.as_str()).unwrap_or("None"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.live.logo, None, "None");
                for (id, name) in &media {
                    ui.selectable_value(&mut app.live.logo, Some(*id), name);
                }
            });
    });
}

fn stage_controls(app: &mut App, ui: &mut Ui) {
    section_header(ui, "Stage", |_| {});
    egui::Frame::new().inner_margin(8).show(ui, |ui| {
        let mut layout = app.prefs.get().show.stage_layout;
        egui::ComboBox::from_label("Layout").selected_text(stage_layout_name(layout)).show_ui(ui, |ui| {
            for l in [StageLayout::CurrentNext, StageLayout::TextOnly, StageLayout::NotesFocus] {
                ui.selectable_value(&mut layout, l, stage_layout_name(l));
            }
        });
        if layout != app.prefs.get().show.stage_layout {
            app.prefs.update(|p| p.show.stage_layout = layout);
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.stage_message_draft)
                    .hint_text("Message to the stage")
                    .desired_width(ui.available_width() - 110.0),
            );
            if ui.button("Show").clicked() && !app.stage_message_draft.trim().is_empty() {
                app.live.stage_message = Some(app.stage_message_draft.trim().to_string());
            }
            if ui.add_enabled(app.live.stage_message.is_some(), egui::Button::new("Hide")).clicked() {
                app.live.stage_message = None;
            }
        });
        if let Some(m) = &app.live.stage_message {
            ui.label(RichText::new(format!("On stage: {m}")).color(theme::LIVE).size(11.0));
        }
    });
}

pub fn stage_layout_name(l: StageLayout) -> &'static str {
    match l {
        StageLayout::CurrentNext => "Current and next",
        StageLayout::TextOnly => "Lyrics only",
        StageLayout::NotesFocus => "Notes and clock",
    }
}

fn timers(app: &mut App, ui: &mut Ui, now: Instant) {
    section_header(ui, "Timers", |ui| {
        ui.label(RichText::new(live::clock_text()).size(11.0).color(theme::TEXT));
    });
    let style = theme::toolbar_button();
    egui::Frame::new().inner_margin(8).show(ui, |ui| {
        let list = app.lib.timers.clone();
        for t in &list {
            timer_row(&mut app.live, ui, t, now, &style);
        }
        if list.is_empty() {
            ui.label(RichText::new("Add timers in the Timers bin").color(theme::TEXT_DIM));
        }
    });
}

pub fn timer_row(live: &mut Live, ui: &mut Ui, t: &Timer, now: Instant, style: &craft_ui::buttons::IconButtonStyle) {
    ui.push_id(t.id, |ui| {
        ui.horizontal(|ui| {
            let running = live.timers.get(&t.id).is_some_and(|r| r.running());
            if icons::small(ui, if running { Icon::Pause } else { Icon::Play }, if running { "Pause" } else { "Start" }, 22.0, style).clicked() {
                if running {
                    live.stop_timer(t.id, now);
                } else {
                    live.start_timer(t.id, now);
                }
            }
            if icons::small(ui, Icon::Reset, "Reset", 22.0, style).clicked() {
                live.reset_timer(t.id);
            }
            let text = live.timer_text(t, now);
            let color = if text.starts_with('-') {
                theme::DANGER
            } else if running {
                theme::LIVE
            } else {
                theme::TEXT
            };
            ui.label(RichText::new(text).monospace().size(15.0).color(color));
            ui.label(RichText::new(&t.name).color(theme::TEXT_DIM));
        });
    });
}

fn audio_transport(app: &mut App, ui: &mut Ui) {
    if app.audio.current.is_none() && app.audio.error.is_none() {
        return;
    }
    section_header(ui, "Audio", |_| {});
    let style = theme::toolbar_button();
    egui::Frame::new().inner_margin(8).show(ui, |ui| {
        if let Some(err) = app.audio.error.clone() {
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(RichText::new(err).color(theme::DANGER).size(11.0)).wrap());
                if ui.small_button("Dismiss").clicked() {
                    app.audio.error = None;
                }
            });
        }
        let Some(id) = app.audio.current else { return };
        let name = app.lib.media_item(id).map(|m| m.name.clone()).unwrap_or_default();
        ui.label(RichText::new(name).color(theme::LIVE));
        ui.horizontal(|ui| {
            let paused = app.audio.is_paused();
            if icons::small(ui, if paused { Icon::Play } else { Icon::Pause }, if paused { "Play" } else { "Pause" }, 22.0, &style).clicked() {
                app.audio.toggle_pause();
            }
            if icons::small(ui, Icon::ClearAll, "Stop", 22.0, &style).clicked() {
                app.audio.stop();
            }
            let pos = app.audio.position();
            let total = app.audio.duration;
            let text = match total {
                Some(t) => format!("{} / {}", live::format_clock(pos.as_secs() as i64), live::format_clock(t.as_secs() as i64)),
                None => live::format_clock(pos.as_secs() as i64),
            };
            ui.label(RichText::new(text).monospace());
        });
        if let Some(total) = app.audio.duration.filter(|t| !t.is_zero()) {
            let mut at = app.audio.position().as_secs_f32();
            if ui.add(egui::Slider::new(&mut at, 0.0..=total.as_secs_f32()).show_value(false).text("Position")).changed() {
                app.audio.seek(Duration::from_secs_f32(at));
            }
        }
        let mut v = app.audio.volume();
        if ui.add(egui::Slider::new(&mut v, 0.0..=1.5).text("Volume")).changed() {
            app.audio.set_volume(v);
        }
    });
}
