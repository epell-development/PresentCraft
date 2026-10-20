//! What is on the audience screens right now, layer by layer, plus running timers.
//!
//! Layers, bottom to top: media (background), slide, props, messages. Each clears on its own.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::model::{Cue, Id, Library, PlaylistItem, Presentation, Slide, Timer, TimerKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveSlide {
    pub presentation: Id,
    pub slide: Id,
}

#[derive(Clone, Debug, Default)]
pub struct TimerRun {
    pub started: Option<Instant>,
    pub accumulated: Duration,
}

impl TimerRun {
    pub fn running(&self) -> bool {
        self.started.is_some()
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        self.accumulated + self.started.map(|s| now.saturating_duration_since(s)).unwrap_or_default()
    }
}

#[derive(Debug)]
pub struct Live {
    pub slide: Option<LiveSlide>,
    /// The slide shown before the current one, kept while the cross-fade runs.
    pub previous_slide: Option<LiveSlide>,
    pub slide_changed: Option<Instant>,
    pub media: Option<Id>,
    pub previous_media: Option<Id>,
    pub media_changed: Option<Instant>,
    pub props: Vec<Id>,
    pub message: Option<(Id, String)>,
    /// A logo shown when everything else is cleared.
    pub logo: Option<Id>,
    pub logo_on: bool,
    pub stage_message: Option<String>,
    pub timers: HashMap<Id, TimerRun>,
    /// Playlist that drives "next" past the end of a presentation.
    pub playlist: Option<Id>,
    pub transition: Duration,
}

impl Default for Live {
    fn default() -> Self {
        Self {
            slide: None,
            previous_slide: None,
            slide_changed: None,
            media: None,
            previous_media: None,
            media_changed: None,
            props: Vec::new(),
            message: None,
            logo: None,
            logo_on: false,
            stage_message: None,
            timers: HashMap::new(),
            playlist: None,
            transition: Duration::from_millis(400),
        }
    }
}

impl Live {
    /// Put a slide on screen, firing its media action if it has one.
    pub fn trigger(&mut self, lib: &Library, presentation: Id, slide: Id, now: Instant) {
        let Some(p) = lib.presentation(presentation) else {
            return;
        };
        let Some((_, s)) = p.find_slide(slide) else {
            return;
        };
        self.previous_slide = self.slide;
        self.slide = Some(LiveSlide { presentation, slide });
        self.slide_changed = Some(now);
        self.logo_on = false;
        if let Some(media) = s.media_action {
            self.set_media(Some(media), now);
        }
    }

    pub fn set_media(&mut self, media: Option<Id>, now: Instant) {
        if self.media == media {
            return;
        }
        self.previous_media = self.media;
        self.media = media;
        self.media_changed = Some(now);
        if media.is_some() {
            self.logo_on = false;
        }
    }

    pub fn clear_slide(&mut self, now: Instant) {
        if self.slide.is_some() {
            self.previous_slide = self.slide.take();
            self.slide_changed = Some(now);
        }
    }

    pub fn clear_media(&mut self, now: Instant) {
        self.set_media(None, now);
    }

    pub fn clear_props(&mut self) {
        self.props.clear();
    }

    pub fn clear_messages(&mut self) {
        self.message = None;
    }

    pub fn clear_all(&mut self, now: Instant) {
        self.clear_slide(now);
        self.clear_media(now);
        self.clear_props();
        self.clear_messages();
    }

    pub fn toggle_logo(&mut self, now: Instant) {
        if self.logo_on {
            self.logo_on = false;
        } else if self.logo.is_some() {
            self.clear_all(now);
            self.logo_on = true;
        }
    }

    pub fn toggle_prop(&mut self, id: Id) {
        if let Some(pos) = self.props.iter().position(|p| *p == id) {
            self.props.remove(pos);
        } else {
            self.props.push(id);
        }
    }

    /// Fade progress from 0 (just changed) to 1 (settled).
    pub fn fade(&self, changed: Option<Instant>, now: Instant) -> f32 {
        let Some(at) = changed else { return 1.0 };
        if self.transition.is_zero() {
            return 1.0;
        }
        (now.saturating_duration_since(at).as_secs_f32() / self.transition.as_secs_f32()).clamp(0.0, 1.0)
    }

    pub fn live_slide<'a>(&self, lib: &'a Library) -> Option<(&'a Presentation, &'a Slide)> {
        slide_ref(lib, self.slide)
    }

    pub fn previous_slide_ref<'a>(&self, lib: &'a Library) -> Option<(&'a Presentation, &'a Slide)> {
        slide_ref(lib, self.previous_slide)
    }

    /// The cue after the live one, crossing into the next playlist presentation when needed.
    pub fn next_cue(&self, lib: &Library) -> Option<LiveSlide> {
        self.step(lib, 1)
    }

    pub fn prev_cue(&self, lib: &Library) -> Option<LiveSlide> {
        self.step(lib, -1)
    }

    fn step(&self, lib: &Library, dir: i32) -> Option<LiveSlide> {
        let live = self.slide?;
        let p = lib.presentation(live.presentation)?;
        let cues = p.cues();
        let (cue, _) = p.find_slide(live.slide)?;
        // A slide can appear several times in an arrangement; continue from the first match
        // after the previous position when possible.
        let index = cues.iter().position(|c| *c == cue)?;
        let target = index as i64 + i64::from(dir);
        if target >= 0 && (target as usize) < cues.len() {
            let c = cues[target as usize];
            return Some(LiveSlide { presentation: p.id, slide: p.slide(c)?.id });
        }
        // Past either end: move to the neighbouring presentation in the live playlist.
        let playlist = lib.playlists.iter().find(|pl| Some(pl.id) == self.playlist)?;
        let pres: Vec<Id> = playlist
            .items
            .iter()
            .filter_map(|i| match i {
                PlaylistItem::Presentation { presentation, .. } => Some(*presentation),
                _ => None,
            })
            .collect();
        let at = pres.iter().position(|id| *id == p.id)? as i64 + i64::from(dir);
        if at < 0 {
            return None;
        }
        let neighbour = lib.presentation(*pres.get(at as usize)?)?;
        let ncues = neighbour.cues();
        let c: Cue = if dir > 0 { *ncues.first()? } else { *ncues.last()? };
        Some(LiveSlide { presentation: neighbour.id, slide: neighbour.slide(c)?.id })
    }

    pub fn start_timer(&mut self, id: Id, now: Instant) {
        let run = self.timers.entry(id).or_default();
        if run.started.is_none() {
            run.started = Some(now);
        }
    }

    pub fn stop_timer(&mut self, id: Id, now: Instant) {
        if let Some(run) = self.timers.get_mut(&id)
            && let Some(s) = run.started.take()
        {
            run.accumulated += now.saturating_duration_since(s);
        }
    }

    pub fn reset_timer(&mut self, id: Id) {
        self.timers.remove(&id);
    }

    /// Display text for a timer, e.g. `04:59` or `-00:12` once a countdown overruns.
    pub fn timer_text(&self, timer: &Timer, now: Instant) -> String {
        let elapsed = self.timers.get(&timer.id).map(|r| r.elapsed(now)).unwrap_or_default().as_secs() as i64;
        let remaining = match timer.kind {
            TimerKind::Elapsed => return format_clock(elapsed),
            TimerKind::Countdown { seconds } => i64::from(seconds) - elapsed,
            TimerKind::CountdownTo { hour, minute } => {
                let target = i64::from(hour) * 3600 + i64::from(minute) * 60;
                let mut left = target - seconds_of_day_local();
                if left < -12 * 3600 {
                    left += 24 * 3600;
                }
                left
            }
        };
        if remaining < 0 && !timer.overrun { format_clock(0) } else { format_clock(remaining) }
    }
}

fn slide_ref(lib: &Library, live: Option<LiveSlide>) -> Option<(&Presentation, &Slide)> {
    let live = live?;
    let p = lib.presentation(live.presentation)?;
    let (_, s) = p.find_slide(live.slide)?;
    Some((p, s))
}

pub fn format_clock(seconds: i64) -> String {
    let sign = if seconds < 0 { "-" } else { "" };
    let s = seconds.unsigned_abs();
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 { format!("{sign}{h}:{m:02}:{sec:02}") } else { format!("{sign}{m:02}:{sec:02}") }
}

/// Seconds since local midnight, in the computer's time zone.
pub fn seconds_of_day_local() -> i64 {
    use chrono::Timelike;
    i64::from(chrono::Local::now().num_seconds_from_midnight())
}

pub fn clock_text() -> String {
    let s = seconds_of_day_local();
    let (h, m) = (s / 3600, (s % 3600) / 60);
    let (h12, ampm) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    format!("{h12}:{m:02} {ampm}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Playlist, PlaylistItem};

    fn lib_with_two_songs() -> (Library, Id, Id, Id) {
        let mut lib = Library::default();
        let a = lib.presentation_from_lyrics("A", "a1\n\na2", None);
        let b = lib.presentation_from_lyrics("B", "b1\n\nb2", None);
        let (aid, bid) = (a.id, b.id);
        lib.presentations.push(a);
        lib.presentations.push(b);
        let plid = lib.next_id();
        let (i1, i2) = (lib.next_id(), lib.next_id());
        lib.playlists.push(Playlist {
            id: plid,
            name: "Sunday".into(),
            items: vec![PlaylistItem::Presentation { id: i1, presentation: aid }, PlaylistItem::Presentation { id: i2, presentation: bid }],
        });
        (lib, aid, bid, plid)
    }

    #[test]
    fn next_crosses_into_the_next_playlist_item() {
        let (lib, aid, bid, plid) = lib_with_two_songs();
        let mut live = Live { playlist: Some(plid), ..Default::default() };
        let now = Instant::now();
        let a = lib.presentation(aid).map(|p| p.groups[0].slides[1].id).unwrap_or(0);
        live.trigger(&lib, aid, a, now);
        let next = live.next_cue(&lib);
        let b_first = lib.presentation(bid).map(|p| p.groups[0].slides[0].id).unwrap_or(0);
        assert_eq!(next, Some(LiveSlide { presentation: bid, slide: b_first }));
    }

    #[test]
    fn next_stops_at_the_end_without_a_playlist() {
        let (lib, aid, _, _) = lib_with_two_songs();
        let mut live = Live::default();
        let a = lib.presentation(aid).map(|p| p.groups[0].slides[1].id).unwrap_or(0);
        live.trigger(&lib, aid, a, Instant::now());
        assert_eq!(live.next_cue(&lib), None);
        assert!(live.prev_cue(&lib).is_some());
    }

    #[test]
    fn clear_all_empties_every_layer() {
        let (lib, aid, _, _) = lib_with_two_songs();
        let mut live = Live::default();
        let now = Instant::now();
        let a = lib.presentation(aid).map(|p| p.groups[0].slides[0].id).unwrap_or(0);
        live.trigger(&lib, aid, a, now);
        live.set_media(Some(5), now);
        live.toggle_prop(7);
        live.message = Some((1, "hi".into()));
        live.clear_all(now);
        assert!(live.slide.is_none() && live.media.is_none() && live.props.is_empty() && live.message.is_none());
    }

    #[test]
    fn countdown_overrun_shows_negative_only_when_enabled() {
        let mut live = Live::default();
        let now = Instant::now();
        let mut t = Timer { id: 1, name: "t".into(), kind: TimerKind::Countdown { seconds: 10 }, overrun: false };
        live.timers.insert(1, TimerRun { started: None, accumulated: Duration::from_secs(15) });
        assert_eq!(live.timer_text(&t, now), "00:00");
        t.overrun = true;
        assert_eq!(live.timer_text(&t, now), "-00:05");
    }

    #[test]
    fn clock_formats() {
        assert_eq!(format_clock(65), "01:05");
        assert_eq!(format_clock(3725), "1:02:05");
    }
}
