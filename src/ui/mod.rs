//! The main window's panels and the output windows.

pub mod bin;
pub mod dialogs;
pub mod editor;
pub mod grid;
pub mod icons;
pub mod outputs;
pub mod preview;
pub mod reflow;
pub mod scripture;
pub mod sidebar;
pub mod theme;
pub mod toolbar;

use egui::{Color32, Id, Rect, Sense, Ui, vec2};

/// A strip of tabs built on `craft_ui::tabs`, returning the newly clicked index.
pub fn tab_strip(ui: &mut Ui, id: Id, labels: &[&str], selected: usize) -> Option<usize> {
    let height = 26.0;
    let font = egui::FontId::proportional(13.0);
    let natural: Vec<f32> =
        labels.iter().map(|l| ui.painter().layout_no_wrap((*l).to_string(), font.clone(), Color32::WHITE).size().x + 24.0).collect();
    let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let fit = craft_ui::tabs::fit(&natural, selected, strip.width(), 48.0, 28.0);
    let mut x = strip.left();
    let mut clicked = None;
    for (index, width) in fit.shown {
        let rect = Rect::from_min_size(egui::pos2(x, strip.top()), vec2(width, height));
        x += width;
        let label = labels.get(index).copied().unwrap_or("");
        let is_sel = index == selected;
        let response =
            craft_ui::tabs::Tab::new(id.with(index), label, is_sel).focus_stroke(egui::Stroke::new(1.0, theme::ACCENT)).show_at(ui, rect, |ui, r| {
                let fill = if is_sel {
                    theme::PANEL_BG
                } else if r.hovered() {
                    theme::HOVER
                } else {
                    theme::TAB_BG
                };
                ui.painter().rect_filled(rect, 0.0, fill);
                if is_sel {
                    ui.painter().line_segment([rect.left_bottom(), rect.right_bottom()], egui::Stroke::new(2.0, theme::ACCENT));
                }
                let color = if is_sel { theme::TEXT } else { theme::TEXT_DIM };
                ui.painter().with_clip_rect(rect).text(rect.center(), egui::Align2::CENTER_CENTER, label, font.clone(), color);
            });
        if response.clicked() {
            clicked = Some(index);
        }
    }
    if !fit.overflow.is_empty() {
        let rect = Rect::from_min_size(egui::pos2(x, strip.top()), vec2(fit.overflow_width, height));
        let r = ui.interact(rect, id.with("overflow"), Sense::click());
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "»", font.clone(), theme::TEXT_DIM);
        egui::Popup::menu(&r).show(|ui| {
            for i in &fit.overflow {
                if ui.button(labels.get(*i).copied().unwrap_or("")).clicked() {
                    clicked = Some(*i);
                }
            }
        });
    }
    clicked
}

/// A small header bar used at the top of sidebar sections and bins.
pub fn section_header(ui: &mut Ui, title: &str, add_right: impl FnOnce(&mut Ui)) {
    egui::Frame::new().fill(theme::HEADER_BG).inner_margin(egui::Margin::symmetric(8, 4)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(title.to_uppercase()).size(11.0).strong().color(theme::TEXT_DIM));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add_right);
        });
    });
}
