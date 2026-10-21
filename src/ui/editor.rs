//! Edit mode: a slide list, the canvas with movable text boxes and shapes, and an inspector.

use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, vec2};

use super::theme;
use crate::app::{App, CanvasDrag};
use crate::model::*;
use crate::render::{self, Painting};

pub fn show(app: &mut App, ui: &mut Ui) {
    let Some(p) = app.presentation().cloned() else {
        ui.centered_and_justified(|ui| ui.label(RichText::new("Select a presentation to edit").color(theme::TEXT_DIM)));
        return;
    };
    let slide_id =
        app.selected_slide.filter(|id| p.find_slide(*id).is_some()).or_else(|| p.groups.iter().flat_map(|g| &g.slides).next().map(|s| s.id));
    app.selected_slide = slide_id;

    slide_list(app, ui, &p);
    let Some(original) = slide_id.and_then(|id| p.find_slide(id)).map(|(_, s)| s.clone()) else {
        ui.centered_and_justified(|ui| {
            if ui.button("Add a slide").clicked() {
                add_slide(app, p.id);
            }
        });
        return;
    };
    let mut slide = original.clone();
    let mut theme_action = None;
    egui::Panel::right("inspector").resizable(true).default_size(280.0).frame(egui::Frame::new().fill(theme::PANEL_BG).inner_margin(8)).show(
        ui,
        |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| inspector(app, ui, &mut slide, &mut theme_action));
        },
    );
    egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::CENTER_BG).inner_margin(10)).show(ui, |ui| {
        canvas_toolbar(app, ui, &mut slide);
        canvas(app, ui, &mut slide);
    });

    if slide != original {
        if let Some(s) = app.lib.presentation_mut(p.id).and_then(|p| p.find_slide_mut(original.id)) {
            *s = slide.clone();
        }
        app.changed();
    }
    if let Some(action) = theme_action {
        apply_theme_action(app, p.id, &slide, action);
    }
}

fn add_slide(app: &mut App, pid: Id) {
    let theme = app.lib.presentation(pid).and_then(|p| p.theme);
    let slide = app.lib.new_slide("New slide", theme);
    let gid = app.lib.next_id();
    let sid = slide.id;
    if let Some(p) = app.lib.presentation_mut(pid) {
        if p.groups.is_empty() {
            p.groups.push(Group { id: gid, name: "Slides".into(), color: group_color("Verse"), slides: Vec::new() });
        }
        let after = app.selected_slide.and_then(|id| p.find_slide(id)).map(|(c, _)| c);
        match after {
            Some(c) => p.groups[c.group].slides.insert(c.slide + 1, slide),
            None => {
                if let Some(g) = p.groups.last_mut() {
                    g.slides.push(slide);
                }
            }
        }
    }
    app.selected_slide = Some(sid);
    app.changed();
}

fn slide_list(app: &mut App, ui: &mut Ui, p: &Presentation) {
    egui::Panel::left("edit-slides").exact_size(170.0).frame(egui::Frame::new().fill(theme::PANEL_BG).inner_margin(6)).show(ui, |ui| {
        if ui.button("+ New slide").clicked() {
            add_slide(app, p.id);
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for g in &p.groups {
                ui.label(RichText::new(&g.name).color(g.color.to_egui()).strong().size(11.0));
                for s in &g.slides {
                    let w = ui.available_width();
                    let (rect, r) = ui.allocate_exact_size(vec2(w, w * 9.0 / 16.0), Sense::click());
                    let ctx = ui.ctx().clone();
                    Painting { lib: &app.lib, cache: &mut app.cache, ctx: &ctx }.thumbnail(ui.painter(), rect, s);
                    if app.selected_slide == Some(s.id) {
                        render::outline(ui.painter(), rect, theme::ACCENT, 2.0);
                    }
                    if r.clicked() {
                        app.selected_slide = Some(s.id);
                        app.selected_element = None;
                    }
                }
            }
        });
    });
}

fn canvas_toolbar(app: &mut App, ui: &mut Ui, slide: &mut Slide) {
    ui.horizontal_wrapped(|ui| {
        if ui.button("T  Text").clicked() {
            let theme = app.presentation().and_then(|p| p.theme).and_then(|t| app.lib.theme(t)).cloned();
            let mut e = app.lib.text_element("Double-click to edit", theme.as_ref());
            e.frame = Frame { x: 360.0, y: 390.0, w: 1200.0, h: 300.0 };
            app.selected_element = Some(e.id);
            slide.elements.push(e);
        }
        if ui.button("■  Shape").clicked() {
            let id = app.lib.next_id();
            slide.elements.push(Element {
                id,
                name: "Shape".into(),
                frame: Frame { x: 660.0, y: 340.0, w: 600.0, h: 400.0 },
                kind: ElementKind::Shape { fill: Rgba([45, 108, 223, 255]), corner: 16.0 },
                fill: None,
            });
            app.selected_element = Some(id);
        }
        ui.menu_button("▣  Media", |ui| {
            let items: Vec<(Id, String)> =
                app.lib.media.iter().filter(|m| !matches!(m.source, MediaSource::Audio { .. })).map(|m| (m.id, m.name.clone())).collect();
            for (mid, name) in items {
                if ui.button(name).clicked() {
                    let id = app.lib.next_id();
                    slide.elements.push(Element {
                        id,
                        name: "Media".into(),
                        frame: Frame { x: 480.0, y: 270.0, w: 960.0, h: 540.0 },
                        kind: ElementKind::Media { media: mid },
                        fill: None,
                    });
                    app.selected_element = Some(id);
                }
            }
        });
        ui.separator();
        let sel = app.selected_element.and_then(|id| slide.elements.iter().position(|e| e.id == id));
        if ui.add_enabled(sel.is_some(), egui::Button::new("Forward")).clicked()
            && let Some(i) = sel
            && i + 1 < slide.elements.len()
        {
            slide.elements.swap(i, i + 1);
        }
        if ui.add_enabled(sel.is_some(), egui::Button::new("Backward")).clicked()
            && let Some(i) = sel
            && i > 0
        {
            slide.elements.swap(i, i - 1);
        }
        if ui.add_enabled(sel.is_some(), egui::Button::new(RichText::new("Delete").color(theme::DANGER))).clicked()
            && let Some(i) = sel
        {
            slide.elements.remove(i);
            app.selected_element = None;
        }
    });
    ui.add_space(6.0);
}

fn canvas(app: &mut App, ui: &mut Ui, slide: &mut Slide) {
    // The toolbar can overhang a narrow panel; never let the canvas follow it under the inspector.
    let area = ui.available_rect_before_wrap().intersect(ui.clip_rect());
    let screen = render::fit_16x9(area);
    let r = ui.allocate_rect(screen, Sense::click_and_drag());
    let painter = ui.painter_at(screen);
    let ctx = ui.ctx().clone();
    Painting { lib: &app.lib, cache: &mut app.cache, ctx: &ctx }.thumbnail(&painter, screen, slide);
    let scale = screen.width() / CANVAS_W;

    // Title-safe guide at 5% in from each edge.
    painter.rect_stroke(screen.shrink(screen.width() * 0.05), 0.0, Stroke::new(1.0, Color32::from_white_alpha(30)), StrokeKind::Inside);

    let handle = |r: Rect, i: u8| -> Pos2 {
        match i {
            1 => r.left_top(),
            2 => r.right_top(),
            3 => r.right_bottom(),
            _ => r.left_bottom(),
        }
    };

    for e in &slide.elements {
        let er = render::frame_rect(screen, e.frame);
        let selected = app.selected_element == Some(e.id);
        painter.rect_stroke(er, 0.0, Stroke::new(1.0, if selected { theme::ACCENT } else { Color32::from_white_alpha(40) }), StrokeKind::Middle);
        if selected {
            for i in 1..=4 {
                painter.rect_filled(Rect::from_center_size(handle(er, i), vec2(9.0, 9.0)), 1.0, theme::ACCENT);
            }
        }
    }

    let pointer = ui.ctx().pointer_interact_pos();
    if r.drag_started()
        && let Some(pos) = pointer
    {
        let sel = app.selected_element.and_then(|id| slide.elements.iter().find(|e| e.id == id));
        let on_handle = sel.and_then(|e| {
            let er = render::frame_rect(screen, e.frame);
            (1..=4).find(|i| handle(er, *i).distance(pos) < 10.0).map(|i| (e.id, i, e.frame))
        });
        let hit =
            on_handle.or_else(|| slide.elements.iter().rev().find(|e| render::frame_rect(screen, e.frame).contains(pos)).map(|e| (e.id, 0, e.frame)));
        if let Some((id, h, start)) = hit {
            app.selected_element = Some(id);
            app.canvas_drag = Some(CanvasDrag { element: id, handle: h, start });
        }
    }
    if r.dragged()
        && let Some(drag) = app.canvas_drag
        && let Some(e) = slide.elements.iter_mut().find(|e| e.id == drag.element)
    {
        let total = ui.input(|i| i.pointer.press_origin().zip(i.pointer.interact_pos()).map(|(a, b)| b - a)).unwrap_or_default() / scale;
        let s = drag.start;
        let snap = |v: f32| {
            if ui.input(|i| i.modifiers.shift) { v } else { (v / 10.0).round() * 10.0 }
        };
        e.frame = match drag.handle {
            0 => Frame { x: snap(s.x + total.x), y: snap(s.y + total.y), ..s },
            1 => Frame { x: snap(s.x + total.x), y: snap(s.y + total.y), w: (s.w - total.x).max(20.0), h: (s.h - total.y).max(20.0) },
            2 => Frame { y: snap(s.y + total.y), w: (s.w + total.x).max(20.0), h: (s.h - total.y).max(20.0), ..s },
            3 => Frame { w: (s.w + total.x).max(20.0), h: (s.h + total.y).max(20.0), ..s },
            _ => Frame { x: snap(s.x + total.x), w: (s.w - total.x).max(20.0), h: (s.h + total.y).max(20.0), ..s },
        };
    }
    if r.drag_stopped() {
        app.canvas_drag = None;
    }
    if r.clicked()
        && let Some(pos) = pointer
    {
        app.selected_element = slide.elements.iter().rev().find(|e| render::frame_rect(screen, e.frame).contains(pos)).map(|e| e.id);
    }
    if ui.input(|i| i.key_pressed(egui::Key::Delete))
        && !ui.ctx().egui_wants_keyboard_input()
        && let Some(id) = app.selected_element
    {
        slide.elements.retain(|e| e.id != id);
        app.selected_element = None;
    }
}

enum ThemeAction {
    ApplyToAll,
    SaveAsTheme,
    UseTheme(Option<Id>),
}

fn color_edit(ui: &mut Ui, c: &mut Rgba) {
    let mut col = c.to_egui();
    if ui.color_edit_button_srgba(&mut col).changed() {
        *c = Rgba::from_egui(col);
    }
}

fn inspector(app: &mut App, ui: &mut Ui, slide: &mut Slide, theme_action: &mut Option<ThemeAction>) {
    let media: Vec<(Id, String)> =
        app.lib.media.iter().filter(|m| !matches!(m.source, MediaSource::Audio { .. })).map(|m| (m.id, m.name.clone())).collect();
    let audio: Vec<(Id, String)> =
        app.lib.media.iter().filter(|m| matches!(m.source, MediaSource::Audio { .. })).map(|m| (m.id, m.name.clone())).collect();
    if let Some(e) = app.selected_element.and_then(|id| slide.elements.iter_mut().find(|e| e.id == id)) {
        ui.heading("Element");
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut e.name);
        });
        egui::Grid::new("frame").num_columns(4).show(ui, |ui| {
            ui.label("X");
            ui.add(egui::DragValue::new(&mut e.frame.x).speed(2.0));
            ui.label("Y");
            ui.add(egui::DragValue::new(&mut e.frame.y).speed(2.0));
            ui.end_row();
            ui.label("W");
            ui.add(egui::DragValue::new(&mut e.frame.w).speed(2.0).range(10.0..=8000.0));
            ui.label("H");
            ui.add(egui::DragValue::new(&mut e.frame.h).speed(2.0).range(10.0..=8000.0));
            ui.end_row();
        });
        if ui.button("Fill the slide").clicked() {
            e.frame = Frame { x: 0.0, y: 0.0, w: CANVAS_W, h: CANVAS_H };
        }
        ui.horizontal(|ui| {
            let mut has = e.fill.is_some();
            if ui.checkbox(&mut has, "Background fill").changed() {
                e.fill = has.then_some(Rgba([0, 0, 0, 160]));
            }
            if let Some(f) = &mut e.fill {
                color_edit(ui, f);
            }
        });
        ui.separator();
        match &mut e.kind {
            ElementKind::Text { text, style } => {
                ui.label("Text");
                ui.add(egui::TextEdit::multiline(text).desired_rows(5).desired_width(f32::INFINITY));
                text_style(ui, style);
            }
            ElementKind::Shape { fill, corner } => {
                ui.horizontal(|ui| {
                    ui.label("Fill");
                    color_edit(ui, fill);
                });
                ui.add(egui::Slider::new(corner, 0.0..=200.0).text("Corner radius"));
            }
            ElementKind::Media { media: m } => {
                egui::ComboBox::from_label("Media")
                    .selected_text(media.iter().find(|(id, _)| id == m).map(|(_, n)| n.as_str()).unwrap_or("Missing"))
                    .show_ui(ui, |ui| {
                        for (id, name) in &media {
                            ui.selectable_value(m, *id, name);
                        }
                    });
            }
        }
        ui.separator();
    }

    ui.heading("Slide");
    ui.horizontal(|ui| {
        ui.label("Label");
        ui.add(egui::TextEdit::singleline(&mut slide.label).hint_text("Uses the group name"));
    });
    ui.checkbox(&mut slide.enabled, "Enabled");
    ui.horizontal(|ui| {
        let mut has = slide.background.is_some();
        if ui.checkbox(&mut has, "Background color").changed() {
            slide.background = has.then_some(Rgba::BLACK);
        }
        if let Some(bg) = &mut slide.background {
            color_edit(ui, bg);
        }
    });
    egui::ComboBox::from_label("Media action")
        .selected_text(slide.media_action.and_then(|m| media.iter().find(|(id, _)| *id == m)).map(|(_, n)| n.as_str()).unwrap_or("None"))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut slide.media_action, None, "None");
            for (id, name) in &media {
                ui.selectable_value(&mut slide.media_action, Some(*id), name);
            }
        });
    egui::ComboBox::from_label("Audio action")
        .selected_text(slide.audio_action.and_then(|m| audio.iter().find(|(id, _)| *id == m)).map(|(_, n)| n.as_str()).unwrap_or("None"))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut slide.audio_action, None, "None");
            for (id, name) in &audio {
                ui.selectable_value(&mut slide.audio_action, Some(*id), name);
            }
        });
    ui.label("Notes (stage display)");
    ui.add(egui::TextEdit::multiline(&mut slide.notes).desired_rows(3).desired_width(f32::INFINITY));

    ui.separator();
    ui.heading("Theme");
    let current = app.presentation().and_then(|p| p.theme);
    let themes: Vec<(Id, String)> = app.lib.themes.iter().map(|t| (t.id, t.name.clone())).collect();
    egui::ComboBox::from_label("Presentation theme")
        .selected_text(current.and_then(|c| themes.iter().find(|(id, _)| *id == c)).map(|(_, n)| n.as_str()).unwrap_or("None"))
        .show_ui(ui, |ui| {
            if ui.selectable_label(current.is_none(), "None").clicked() {
                *theme_action = Some(ThemeAction::UseTheme(None));
            }
            for (id, name) in &themes {
                if ui.selectable_label(current == Some(*id), name).clicked() {
                    *theme_action = Some(ThemeAction::UseTheme(Some(*id)));
                }
            }
        });
    ui.horizontal(|ui| {
        if ui.button("Apply to all slides").on_hover_text("Restyle every slide's first text box with this theme").clicked() {
            *theme_action = Some(ThemeAction::ApplyToAll);
        }
        if ui.button("Save slide as theme").clicked() {
            *theme_action = Some(ThemeAction::SaveAsTheme);
        }
    });
}

pub fn text_style(ui: &mut Ui, style: &mut TextStyle) {
    ui.add(egui::Slider::new(&mut style.size, 12.0..=300.0).text("Size"));
    ui.horizontal(|ui| {
        ui.label("Color");
        color_edit(ui, &mut style.color);
    });
    ui.horizontal(|ui| {
        ui.selectable_value(&mut style.h_align, HAlign::Left, "Left");
        ui.selectable_value(&mut style.h_align, HAlign::Center, "Center");
        ui.selectable_value(&mut style.h_align, HAlign::Right, "Right");
    });
    ui.horizontal(|ui| {
        ui.selectable_value(&mut style.v_align, VAlign::Top, "Top");
        ui.selectable_value(&mut style.v_align, VAlign::Middle, "Middle");
        ui.selectable_value(&mut style.v_align, VAlign::Bottom, "Bottom");
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut style.shadow, "Shadow");
        ui.checkbox(&mut style.outline, "Outline");
        ui.checkbox(&mut style.uppercase, "ALL CAPS");
    });
}

fn apply_theme_action(app: &mut App, pid: Id, slide: &Slide, action: ThemeAction) {
    match action {
        ThemeAction::UseTheme(t) => {
            if let Some(p) = app.lib.presentation_mut(pid) {
                p.theme = t;
            }
        }
        ThemeAction::SaveAsTheme => {
            let text = slide.elements.iter().find_map(|e| match &e.kind {
                ElementKind::Text { style, .. } => Some((e.frame, style.clone())),
                _ => None,
            });
            let (frame, style) = text.unwrap_or((Frame::FULL_TEXT, TextStyle::default()));
            let id = app.lib.next_id();
            let name = format!("Theme {}", app.lib.themes.len() + 1);
            app.lib.themes.push(Theme { id, name, frame, style, background: slide.background });
            if let Some(p) = app.lib.presentation_mut(pid) {
                p.theme = Some(id);
            }
        }
        ThemeAction::ApplyToAll => {
            let Some(t) = app.lib.presentation(pid).and_then(|p| p.theme).and_then(|t| app.lib.theme(t)).cloned() else {
                app.notices.push("Choose a theme for this presentation first.".into());
                return;
            };
            if let Some(p) = app.lib.presentation_mut(pid) {
                apply_theme(p, &t);
            }
        }
    }
    app.changed();
}

/// Restyle the first text box of every slide and set its background, keeping the words.
pub fn apply_theme(p: &mut Presentation, t: &Theme) {
    for s in p.groups.iter_mut().flat_map(|g| g.slides.iter_mut()) {
        s.background = t.background;
        if let Some(e) = s.elements.iter_mut().find(|e| matches!(e.kind, ElementKind::Text { .. })) {
            e.frame = t.frame;
            if let ElementKind::Text { style, .. } = &mut e.kind {
                *style = t.style.clone();
            }
        }
    }
}
