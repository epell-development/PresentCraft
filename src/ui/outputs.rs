//! The audience output and stage display, each in its own native window.

use std::time::Instant;

use egui::{Color32, FontId, Rect, Ui, ViewportBuilder, ViewportCommand, ViewportId, pos2, vec2};

use super::theme;
use crate::app::App;
use crate::live;
use crate::model::*;
use crate::render::{self, Painting};

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.audience_open {
        let builder =
            ViewportBuilder::default().with_title("PresentCraft — Audience").with_inner_size([960.0, 540.0]).with_min_inner_size([320.0, 180.0]);
        ctx.show_viewport_immediate(ViewportId::from_hash_of("audience"), builder, |ui, _class| audience(app, ui));
    }
    if app.stage_open {
        let builder =
            ViewportBuilder::default().with_title("PresentCraft — Stage").with_inner_size([960.0, 540.0]).with_min_inner_size([320.0, 180.0]);
        ctx.show_viewport_immediate(ViewportId::from_hash_of("stage"), builder, |ui, _class| stage(app, ui));
    }
}

fn window_controls(app: &mut App, ui: &Ui, open: bool) -> bool {
    let ctx = ui.ctx();
    app.keyboard(ctx);
    if ctx.input(|i| i.viewport().close_requested()) {
        return false;
    }
    let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
    if ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!fullscreen));
    }
    if fullscreen && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
    }
    open
}

pub fn audience(app: &mut App, ui: &mut Ui) {
    let open = window_controls(app, ui, true);
    app.audience_open = open;
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, Color32::BLACK);
    let screen = render::fit_16x9(rect);
    let ctx = ui.ctx().clone();
    Painting { lib: &app.lib, cache: &mut app.cache, ctx: &ctx }.output(ui.painter(), screen, &app.live, Instant::now());
}

pub fn stage(app: &mut App, ui: &mut Ui) {
    let open = window_controls(app, ui, true);
    app.stage_open = open;
    let rect = ui.max_rect();
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, Color32::BLACK);
    let now = Instant::now();
    let h = rect.height();
    let layout = app.prefs.get().show.stage_layout;

    let current = app.live.live_slide(&app.lib).map(|(_, s)| s.clone());
    let next =
        app.live.next_cue(&app.lib).and_then(|n| app.lib.presentation(n.presentation).and_then(|p| p.find_slide(n.slide)).map(|(_, s)| s.clone()));
    let group = app
        .live
        .live_slide(&app.lib)
        .and_then(|(p, s)| p.find_slide(s.id).and_then(|(c, _)| p.groups.get(c.group)).map(|g| (g.name.clone(), g.color)));

    // Top bar: clock, the first running timer, and the group of the live slide.
    let bar = Rect::from_min_size(rect.min, vec2(rect.width(), h * 0.1));
    painter.rect_filled(bar, 0.0, Color32::from_rgb(22, 22, 24));
    let bar_font = FontId::monospace(bar.height() * 0.55);
    painter.text(bar.left_center() + vec2(16.0, 0.0), egui::Align2::LEFT_CENTER, live::clock_text(), bar_font.clone(), Color32::WHITE);
    let running = app.lib.timers.iter().find(|t| app.live.timers.get(&t.id).is_some_and(|r| r.running()));
    if let Some(t) = running {
        let text = app.live.timer_text(t, now);
        let color = if text.starts_with('-') { theme::DANGER } else { theme::LIVE };
        painter.text(bar.center(), egui::Align2::CENTER_CENTER, format!("{}  {}", t.name, text), bar_font.clone(), color);
    }
    if let Some((name, color)) = &group {
        painter.text(
            bar.right_center() - vec2(16.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            name,
            FontId::proportional(bar.height() * 0.5),
            color.to_egui(),
        );
    }

    let body = Rect::from_min_max(pos2(rect.left(), bar.bottom()), rect.max);
    let text_of = |s: &Option<Slide>| s.as_ref().map(|s| s.text()).unwrap_or_default();
    match layout {
        StageLayout::CurrentNext => {
            let notes_h = if current.as_ref().is_some_and(|s| !s.notes.is_empty()) { body.height() * 0.2 } else { 0.0 };
            let main = Rect::from_min_max(body.min, pos2(body.left() + body.width() * 0.62, body.bottom() - notes_h));
            let side = Rect::from_min_max(pos2(main.right(), body.top()), pos2(body.right(), body.bottom() - notes_h));
            block(painter, main.shrink(16.0), "CURRENT", &text_of(&current), h * 0.07, Color32::WHITE);
            painter.line_segment([side.left_top(), side.left_bottom()], egui::Stroke::new(1.0, Color32::from_gray(60)));
            block(painter, side.shrink(16.0), "NEXT", &text_of(&next), h * 0.045, Color32::from_gray(170));
            if notes_h > 0.0 {
                let notes = Rect::from_min_max(pos2(body.left(), body.bottom() - notes_h), body.max);
                painter.rect_filled(notes, 0.0, Color32::from_rgb(30, 26, 10));
                block(
                    painter,
                    notes.shrink(12.0),
                    "NOTES",
                    current.as_ref().map(|s| s.notes.as_str()).unwrap_or(""),
                    h * 0.04,
                    Color32::from_rgb(255, 214, 102),
                );
            }
        }
        StageLayout::TextOnly => {
            block(painter, body.shrink(24.0), "", &text_of(&current), h * 0.09, Color32::WHITE);
            let first_next = text_of(&next).lines().next().unwrap_or("").to_string();
            if !first_next.is_empty() {
                painter.text(
                    body.center_bottom() - vec2(0.0, 20.0),
                    egui::Align2::CENTER_BOTTOM,
                    format!("Next: {first_next}"),
                    FontId::proportional(h * 0.04),
                    Color32::from_gray(150),
                );
            }
        }
        StageLayout::NotesFocus => {
            painter.text(
                body.center_top() + vec2(0.0, body.height() * 0.2),
                egui::Align2::CENTER_CENTER,
                live::clock_text(),
                FontId::monospace(h * 0.18),
                Color32::WHITE,
            );
            let mut y = body.top() + body.height() * 0.38;
            for t in &app.lib.timers {
                painter.text(
                    pos2(body.center().x, y),
                    egui::Align2::CENTER_CENTER,
                    format!("{}  {}", t.name, app.live.timer_text(t, now)),
                    FontId::monospace(h * 0.06),
                    Color32::from_gray(200),
                );
                y += h * 0.08;
            }
            let notes = Rect::from_min_max(pos2(body.left(), y), body.max);
            block(
                painter,
                notes.shrink(16.0),
                "NOTES",
                current.as_ref().map(|s| s.notes.as_str()).unwrap_or(""),
                h * 0.05,
                Color32::from_rgb(255, 214, 102),
            );
        }
    }

    // Stage message flashes over the top so it is hard to miss.
    if let Some(msg) = &app.live.stage_message {
        let pulse = ((ui.input(|i| i.time) * 2.0).sin() * 0.5 + 0.5) as f32;
        let band = Rect::from_min_size(pos2(rect.left(), bar.bottom()), vec2(rect.width(), h * 0.12));
        painter.rect_filled(band, 0.0, Color32::from_rgb(160, 20, 20).gamma_multiply(0.7 + 0.3 * pulse));
        painter.text(band.center(), egui::Align2::CENTER_CENTER, msg, FontId::proportional(h * 0.06), Color32::WHITE);
        ui.ctx().request_repaint();
    }
}

fn block(painter: &egui::Painter, r: Rect, title: &str, text: &str, size: f32, color: Color32) {
    let mut top = r.top();
    if !title.is_empty() {
        painter.text(r.left_top(), egui::Align2::LEFT_TOP, title, FontId::proportional((size * 0.4).max(10.0)), Color32::from_gray(110));
        top += size * 0.6;
    }
    let galley = painter.layout(text.to_string(), FontId::proportional(size.max(8.0)), color, r.width().max(1.0));
    painter.with_clip_rect(r).galley(pos2(r.left(), top), galley, color);
}
