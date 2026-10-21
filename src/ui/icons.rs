//! Original, painter-drawn icons. No bitmap or third-party icon artwork is bundled.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Show,
    Edit,
    Reflow,
    Plus,
    Screen,
    Stage,
    ClearAll,
    Slide,
    Media,
    Props,
    Message,
    Logo,
    Play,
    Pause,
    Reset,
    Prev,
    Next,
    Bin,
}

pub fn paint(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height()) * 0.5;
    let c = rect.center();
    let stroke = Stroke::new((s * 0.14).max(1.2), color);
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(c + vec2(x0 * s, y0 * s), c + vec2(x1 * s, y1 * s));
    let pt = |x: f32, y: f32| c + vec2(x * s, y * s);
    match icon {
        Icon::Show => {
            // Three thumbnails in a grid.
            for (x, y) in [(-0.6, -0.45), (0.05, -0.45), (-0.6, 0.1), (0.05, 0.1)] {
                p.rect_filled(r(x, y, x + 0.55, y + 0.38), 1.0, color);
            }
        }
        Icon::Edit => {
            p.line_segment([pt(-0.5, 0.5), pt(0.45, -0.45)], Stroke::new(stroke.width * 1.6, color));
            p.line_segment([pt(-0.6, 0.6), pt(-0.45, 0.6)], stroke);
        }
        Icon::Reflow => {
            for y in [-0.45, -0.15, 0.15, 0.45] {
                let w = if y > 0.3 { 0.2 } else { 0.6 };
                p.line_segment([pt(-0.6, y), pt(w, y)], stroke);
            }
        }
        Icon::Plus => {
            p.line_segment([pt(-0.55, 0.0), pt(0.55, 0.0)], stroke);
            p.line_segment([pt(0.0, -0.55), pt(0.0, 0.55)], stroke);
        }
        Icon::Screen => {
            p.rect_stroke(r(-0.7, -0.5, 0.7, 0.3), 2.0, stroke, egui::StrokeKind::Middle);
            p.line_segment([pt(-0.3, 0.6), pt(0.3, 0.6)], stroke);
            p.line_segment([pt(0.0, 0.3), pt(0.0, 0.6)], stroke);
        }
        Icon::Stage => {
            p.rect_stroke(r(-0.7, -0.5, 0.7, 0.5), 2.0, stroke, egui::StrokeKind::Middle);
            p.rect_filled(r(-0.55, -0.35, 0.0, 0.0), 1.0, color);
            p.line_segment([pt(0.1, -0.25), pt(0.55, -0.25)], stroke);
            p.line_segment([pt(-0.55, 0.25), pt(0.55, 0.25)], stroke);
        }
        Icon::ClearAll => {
            p.line_segment([pt(-0.45, -0.45), pt(0.45, 0.45)], Stroke::new(stroke.width * 1.4, color));
            p.line_segment([pt(0.45, -0.45), pt(-0.45, 0.45)], Stroke::new(stroke.width * 1.4, color));
        }
        Icon::Slide => {
            p.rect_stroke(r(-0.7, -0.45, 0.7, 0.45), 2.0, stroke, egui::StrokeKind::Middle);
            p.line_segment([pt(-0.4, -0.1), pt(0.4, -0.1)], stroke);
            p.line_segment([pt(-0.3, 0.15), pt(0.3, 0.15)], stroke);
        }
        Icon::Media => {
            p.rect_stroke(r(-0.7, -0.5, 0.7, 0.5), 2.0, stroke, egui::StrokeKind::Middle);
            p.add(Shape::convex_polygon(vec![pt(-0.55, 0.38), pt(-0.15, -0.1), pt(0.15, 0.2), pt(0.3, 0.05), pt(0.55, 0.38)], color, Stroke::NONE));
            p.circle_filled(pt(0.32, -0.22), s * 0.1, color);
        }
        Icon::Props => {
            p.rect_stroke(r(-0.7, -0.5, 0.7, 0.5), 2.0, stroke, egui::StrokeKind::Middle);
            p.rect_filled(r(0.15, -0.38, 0.55, -0.12), 1.0, color);
        }
        Icon::Message => {
            p.rect_stroke(r(-0.65, -0.5, 0.65, 0.25), 3.0, stroke, egui::StrokeKind::Middle);
            p.add(Shape::convex_polygon(vec![pt(-0.3, 0.25), pt(0.0, 0.25), pt(-0.35, 0.6)], color, Stroke::NONE));
        }
        Icon::Logo => {
            p.circle_stroke(c, s * 0.6, stroke);
            p.circle_filled(c, s * 0.25, color);
        }
        Icon::Play => {
            p.add(Shape::convex_polygon(vec![pt(-0.35, -0.5), pt(0.5, 0.0), pt(-0.35, 0.5)], color, Stroke::NONE));
        }
        Icon::Pause => {
            p.rect_filled(r(-0.4, -0.5, -0.1, 0.5), 1.0, color);
            p.rect_filled(r(0.1, -0.5, 0.4, 0.5), 1.0, color);
        }
        Icon::Reset => {
            p.add(Shape::Path(egui::epaint::PathShape::line(arc(c, s * 0.5, 0.6, 5.6), stroke)));
            p.add(Shape::convex_polygon(vec![pt(0.25, -0.65), pt(0.6, -0.4), pt(0.2, -0.2)], color, Stroke::NONE));
        }
        Icon::Prev => {
            p.add(Shape::convex_polygon(vec![pt(0.35, -0.5), pt(-0.4, 0.0), pt(0.35, 0.5)], color, Stroke::NONE));
        }
        Icon::Next => {
            p.add(Shape::convex_polygon(vec![pt(-0.35, -0.5), pt(0.4, 0.0), pt(-0.35, 0.5)], color, Stroke::NONE));
        }
        Icon::Bin => {
            p.rect_stroke(r(-0.7, -0.3, 0.7, 0.5), 2.0, stroke, egui::StrokeKind::Middle);
            p.line_segment([pt(-0.7, -0.3), pt(-0.45, -0.55)], stroke);
            p.line_segment([pt(-0.45, -0.55), pt(0.0, -0.55)], stroke);
        }
    }
}

fn arc(c: Pos2, radius: f32, from: f32, to: f32) -> Vec<Pos2> {
    (0..=24).map(|i| from + (to - from) * i as f32 / 24.0).map(|a| pos2(c.x + radius * a.cos(), c.y + radius * a.sin())).collect()
}

/// A toolbar icon button with a caption under it.
pub fn tool(ui: &mut egui::Ui, icon: Icon, label: &str, selected: bool, style: &craft_ui::buttons::IconButtonStyle) -> egui::Response {
    ui.push_id(label, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            let r = craft_ui::buttons::IconButton::new(label, vec2(44.0, 30.0), style).selected(selected).show(ui, |ui, rect, tint| {
                paint(ui.painter(), rect.shrink2(vec2(12.0, 6.0)), icon, tint);
            });
            ui.add_sized(vec2(44.0, 12.0), egui::Label::new(egui::RichText::new(label).size(10.0).color(super::theme::TEXT_DIM)).selectable(false));
            r
        })
        .inner
    })
    .inner
    .on_hover_text(label)
}

/// A compact square icon button for panel headers and transport controls.
pub fn small(ui: &mut egui::Ui, icon: Icon, label: &str, size: f32, style: &craft_ui::buttons::IconButtonStyle) -> egui::Response {
    ui.push_id(label, |ui| {
        craft_ui::buttons::IconButton::new(label, vec2(size, size), style).show(ui, |ui, rect, tint| {
            paint(ui.painter(), rect.shrink(size * 0.22), icon, tint);
        })
    })
    .inner
    .on_hover_text(label)
}
