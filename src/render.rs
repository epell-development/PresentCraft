//! Painting slides and layers. The same code draws thumbnails, the editor canvas, the preview and
//! the audience output, so what the operator sees is what the room sees.

use std::collections::HashMap;
use std::time::Instant;

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, CornerRadius, FontId, Mesh, Painter, Pos2, Rect, Stroke, StrokeKind, TextureHandle, Vec2, pos2, vec2};

use crate::live::Live;
use crate::model::*;

/// Decoded media textures, keyed by media id. Failures are remembered so a missing file is not
/// re-read every frame; `forget` clears an entry when the item changes.
#[derive(Default)]
pub struct MediaCache {
    textures: HashMap<Id, Result<TextureHandle, String>>,
}

impl MediaCache {
    pub fn texture(&mut self, ctx: &egui::Context, item: &MediaItem) -> Option<&TextureHandle> {
        let MediaSource::Image { path } = &item.source else {
            return None;
        };
        let entry = self.textures.entry(item.id).or_insert_with(|| load_texture(ctx, item.id, path));
        entry.as_ref().ok()
    }

    pub fn error(&self, id: Id) -> Option<&str> {
        self.textures.get(&id).and_then(|r| r.as_ref().err()).map(String::as_str)
    }

    pub fn forget(&mut self, id: Id) {
        self.textures.remove(&id);
    }
}

fn load_texture(ctx: &egui::Context, id: Id, path: &str) -> Result<TextureHandle, String> {
    let img = image::open(path).map_err(|e| format!("{path}: {e}"))?;
    // Output never needs more than 4K; large photos are scaled down to keep GPU memory sane.
    let img = if img.width() > 3840 || img.height() > 2160 { img.resize(3840, 2160, image::imageops::FilterType::Triangle) } else { img };
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Ok(ctx.load_texture(format!("media-{id}"), color, egui::TextureOptions::LINEAR))
}

/// Everything needed to paint: the library, cached textures and the egui context for loading.
pub struct Painting<'a> {
    pub lib: &'a Library,
    pub cache: &'a mut MediaCache,
    pub ctx: &'a egui::Context,
}

/// The largest 16:9 rectangle centered in `area`.
pub fn fit_16x9(area: Rect) -> Rect {
    let aspect = CANVAS_W / CANVAS_H;
    let (w, h) =
        if area.width() / area.height().max(1.0) > aspect { (area.height() * aspect, area.height()) } else { (area.width(), area.width() / aspect) };
    Rect::from_center_size(area.center(), vec2(w, h))
}

/// Map a canvas frame into screen space inside `screen`.
pub fn frame_rect(screen: Rect, f: Frame) -> Rect {
    let s = screen.width() / CANVAS_W;
    Rect::from_min_size(screen.min + vec2(f.x * s, f.y * s), vec2(f.w * s, f.h * s))
}

fn fade(c: Color32, alpha: f32) -> Color32 {
    if alpha >= 1.0 { c } else { c.gamma_multiply(alpha.clamp(0.0, 1.0)) }
}

impl Painting<'_> {
    pub fn media(&mut self, painter: &Painter, screen: Rect, id: Id, alpha: f32) {
        let Some(item) = self.lib.media_item(id) else {
            return;
        };
        match &item.source {
            MediaSource::Color { color } => {
                painter.rect_filled(screen, 0.0, fade(color.to_egui(), alpha));
            }
            MediaSource::Gradient { top, bottom } => {
                let mut mesh = Mesh::default();
                let (t, b) = (fade(top.to_egui(), alpha), fade(bottom.to_egui(), alpha));
                mesh.colored_vertex(screen.left_top(), t);
                mesh.colored_vertex(screen.right_top(), t);
                mesh.colored_vertex(screen.right_bottom(), b);
                mesh.colored_vertex(screen.left_bottom(), b);
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                painter.add(mesh);
            }
            MediaSource::Image { .. } => {
                let fit = item.fit;
                let ctx = self.ctx.clone();
                let Some(tex) = self.cache.texture(&ctx, item) else {
                    painter.rect_filled(screen, 0.0, fade(Color32::from_rgb(60, 20, 20), alpha));
                    return;
                };
                let size = tex.size_vec2();
                let (rect, uv) = image_placement(screen, size, fit);
                let mut mesh = Mesh::with_texture(tex.id());
                mesh.add_rect_with_uv(rect, uv, fade(Color32::WHITE, alpha));
                painter.with_clip_rect(screen).add(mesh);
            }
        }
    }

    pub fn elements(&mut self, painter: &Painter, screen: Rect, elements: &[Element], alpha: f32) {
        let painter = painter.with_clip_rect(screen.intersect(painter.clip_rect()));
        for e in elements {
            let r = frame_rect(screen, e.frame);
            if let Some(fill) = e.fill {
                painter.rect_filled(r, 0.0, fade(fill.to_egui(), alpha));
            }
            match &e.kind {
                ElementKind::Shape { fill, corner } => {
                    let radius = (corner * screen.width() / CANVAS_W).clamp(0.0, 255.0) as u8;
                    painter.rect_filled(r, CornerRadius::same(radius), fade(fill.to_egui(), alpha));
                }
                ElementKind::Media { media } => {
                    self.media(&painter, r, *media, alpha);
                }
                ElementKind::Text { text, style } => {
                    paint_text(&painter, screen, r, text, style, alpha);
                }
            }
        }
    }

    pub fn slide(&mut self, painter: &Painter, screen: Rect, slide: &Slide, alpha: f32) {
        if let Some(bg) = slide.background {
            painter.rect_filled(screen, 0.0, fade(bg.to_egui(), alpha));
        }
        self.elements(painter, screen, &slide.elements, alpha);
    }

    /// A thumbnail: the slide over its own media action (or black), as the slide grid shows it.
    pub fn thumbnail(&mut self, painter: &Painter, screen: Rect, slide: &Slide) {
        painter.rect_filled(screen, 0.0, Color32::BLACK);
        if let Some(m) = slide.media_action {
            self.media(painter, screen, m, 1.0);
        }
        self.slide(painter, screen, slide, 1.0);
    }

    /// The whole audience composite, with cross-fades, at `screen`.
    pub fn output(&mut self, painter: &Painter, screen: Rect, live: &Live, now: Instant) {
        painter.rect_filled(screen, 0.0, Color32::BLACK);
        let painter = painter.with_clip_rect(screen.intersect(painter.clip_rect()));

        let mf = live.fade(live.media_changed, now);
        if mf < 1.0
            && let Some(prev) = live.previous_media
        {
            self.media(&painter, screen, prev, 1.0 - mf);
        }
        if let Some(m) = live.media {
            self.media(&painter, screen, m, mf);
        }
        if live.logo_on
            && let Some(logo) = live.logo
        {
            self.media(&painter, screen, logo, 1.0);
        }

        let sf = live.fade(live.slide_changed, now);
        if sf < 1.0
            && let Some((_, prev)) = live.previous_slide_ref(self.lib)
            && live.previous_slide != live.slide
        {
            self.slide(&painter, screen, prev, 1.0 - sf);
        }
        if let Some((_, s)) = live.live_slide(self.lib) {
            self.slide(&painter, screen, s, sf);
        }

        for prop_id in &live.props {
            if let Some(prop) = self.lib.props.iter().find(|p| p.id == *prop_id) {
                self.elements(&painter, screen, &prop.elements, 1.0);
            }
        }

        if let Some((_, text)) = &live.message {
            paint_message(&painter, screen, text);
        }
    }
}

/// Where an image of `size` goes in `screen`, and which part of it, for each fit mode.
pub fn image_placement(screen: Rect, size: Vec2, fit: MediaFit) -> (Rect, Rect) {
    let full_uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
    if size.x <= 0.0 || size.y <= 0.0 {
        return (screen, full_uv);
    }
    let img_aspect = size.x / size.y;
    let scr_aspect = screen.width() / screen.height().max(1.0);
    match fit {
        MediaFit::Stretch => (screen, full_uv),
        MediaFit::Fit => {
            let (w, h) =
                if img_aspect > scr_aspect { (screen.width(), screen.width() / img_aspect) } else { (screen.height() * img_aspect, screen.height()) };
            (Rect::from_center_size(screen.center(), vec2(w, h)), full_uv)
        }
        MediaFit::Fill => {
            // Crop the image's uv range instead of drawing outside the screen.
            if img_aspect > scr_aspect {
                let visible = scr_aspect / img_aspect;
                let m = (1.0 - visible) / 2.0;
                (screen, Rect::from_min_max(pos2(m, 0.0), pos2(1.0 - m, 1.0)))
            } else {
                let visible = img_aspect / scr_aspect;
                let m = (1.0 - visible) / 2.0;
                (screen, Rect::from_min_max(pos2(0.0, m), pos2(1.0, 1.0 - m)))
            }
        }
    }
}

pub fn paint_text(painter: &Painter, screen: Rect, r: Rect, text: &str, style: &TextStyle, alpha: f32) {
    let scale = screen.width() / CANVAS_W;
    let size = (style.size * scale).max(1.0);
    let shown = if style.uppercase { text.to_uppercase() } else { text.to_string() };
    let align = match style.h_align {
        HAlign::Left => Align::LEFT,
        HAlign::Center => Align::Center,
        HAlign::Right => Align::RIGHT,
    };
    let job = |color: Color32| {
        let mut job = LayoutJob::single_section(shown.clone(), TextFormat { font_id: FontId::proportional(size), color, ..Default::default() });
        job.wrap.max_width = r.width().max(1.0);
        job.halign = align;
        job
    };
    let galley = painter.layout_job(job(fade(style.color.to_egui(), alpha)));
    let gs = galley.size();
    let x = match style.h_align {
        HAlign::Left => r.left(),
        HAlign::Center => r.center().x,
        HAlign::Right => r.right(),
    };
    let y = match style.v_align {
        VAlign::Top => r.top(),
        VAlign::Middle => r.center().y - gs.y / 2.0,
        VAlign::Bottom => r.bottom() - gs.y,
    };
    let pos = pos2(x, y);
    if style.shadow {
        let shadow = painter.layout_job(job(fade(Color32::from_black_alpha(170), alpha)));
        let off = (4.0 * scale).max(1.0);
        painter.galley(pos + vec2(off, off), shadow, Color32::BLACK);
    }
    if style.outline {
        let outline = painter.layout_job(job(fade(Color32::BLACK, alpha)));
        let w = (3.0 * scale).max(0.75);
        for d in [vec2(-w, 0.0), vec2(w, 0.0), vec2(0.0, -w), vec2(0.0, w), vec2(-w, -w), vec2(w, w), vec2(-w, w), vec2(w, -w)] {
            painter.galley(pos + d, outline.clone(), Color32::BLACK);
        }
    }
    painter.galley(pos, galley, Color32::WHITE);
}

/// Messages run as a band across the bottom of the screen, above everything else.
pub fn paint_message(painter: &Painter, screen: Rect, text: &str) {
    let scale = screen.width() / CANVAS_W;
    let band = Rect::from_min_max(pos2(screen.left(), screen.bottom() - 120.0 * scale), screen.right_bottom());
    painter.rect_filled(band, 0.0, Color32::from_black_alpha(200));
    painter.text(band.center(), egui::Align2::CENTER_CENTER, text, FontId::proportional((54.0 * scale).max(1.0)), Color32::WHITE);
}

/// A selection outline drawn on top of a thumbnail or element.
pub fn outline(painter: &Painter, r: Rect, color: Color32, width: f32) {
    painter.rect_stroke(r, 2.0, Stroke::new(width, color), StrokeKind::Outside);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_keeps_16x9_inside_any_area() {
        let r = fit_16x9(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 1000.0)));
        assert!((r.width() / r.height() - 16.0 / 9.0).abs() < 0.01);
        assert!((r.width() - 1000.0).abs() < 0.01);
        let r = fit_16x9(Rect::from_min_size(Pos2::ZERO, vec2(4000.0, 900.0)));
        assert!((r.height() - 900.0).abs() < 0.01);
    }

    #[test]
    fn fill_crops_uv_and_fit_letterboxes() {
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(160.0, 90.0));
        let (rect, uv) = image_placement(screen, vec2(100.0, 100.0), MediaFit::Fill);
        assert_eq!(rect, screen);
        assert!(uv.height() < 1.0 && (uv.width() - 1.0).abs() < 1e-6);
        let (rect, uv) = image_placement(screen, vec2(100.0, 100.0), MediaFit::Fit);
        assert!((rect.width() - 90.0).abs() < 0.01);
        assert_eq!(uv.width(), 1.0);
    }
}
