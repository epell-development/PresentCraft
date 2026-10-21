//! Application state and the frame loop that lays out the main window.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use craft_settings::{Location, SaveStatus, SettingsStore};

use crate::audio::AudioEngine;
use crate::bible::{self, Bible};
use crate::live::Live;
use crate::model::*;
use crate::render::MediaCache;
use crate::store::{self, Autosave, Loaded, Prefs};
use crate::ui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Show,
    Edit,
    Reflow,
    Bible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinTab {
    Media,
    Audio,
    Props,
    Messages,
    Timers,
    Themes,
}

impl BinTab {
    pub const ALL: [BinTab; 6] = [BinTab::Media, BinTab::Audio, BinTab::Props, BinTab::Messages, BinTab::Timers, BinTab::Themes];

    pub fn label(self) -> &'static str {
        match self {
            BinTab::Media => "Media",
            BinTab::Audio => "Audio",
            BinTab::Props => "Props",
            BinTab::Messages => "Messages",
            BinTab::Timers => "Timers",
            BinTab::Themes => "Themes",
        }
    }
}

/// What the rename field in the sidebar is editing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Renaming {
    Presentation(Id),
    Playlist(Id),
    Group(Id, Id),
    Media(Id),
}

#[derive(Default)]
pub struct NewPresentation {
    pub name: String,
    pub lyrics: String,
    pub theme: Option<Id>,
    pub add_to_playlist: bool,
}

/// The scripture panel's inputs.
pub struct ScriptureUi {
    pub bible: usize,
    pub reference: String,
    pub search: String,
    pub per_slide: usize,
    pub verse_numbers: bool,
    pub theme: Option<Id>,
    pub import_path: String,
    pub import_name: String,
    pub import_abbreviation: String,
}

impl Default for ScriptureUi {
    fn default() -> Self {
        Self {
            bible: 0,
            reference: "Psalm 23".into(),
            search: String::new(),
            per_slide: 1,
            verse_numbers: true,
            theme: None,
            import_path: String::new(),
            import_name: String::new(),
            import_abbreviation: String::new(),
        }
    }
}

/// An in-progress drag on the editor canvas.
#[derive(Clone, Copy, Debug)]
pub struct CanvasDrag {
    pub element: Id,
    /// 0 = move, 1..=4 = corner handles (tl, tr, br, bl).
    pub handle: u8,
    pub start: Frame,
}

/// File extensions imported as sound rather than pictures.
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "flac", "ogg", "oga"];

pub struct App {
    pub lib: Library,
    pub live: Live,
    pub cache: MediaCache,
    pub audio: AudioEngine,
    pub bibles: Vec<Bible>,
    pub scripture: ScriptureUi,
    pub prefs: SettingsStore<Prefs>,
    pub autosave: Autosave,
    pub path: Option<PathBuf>,

    pub mode: Mode,
    pub bin_tab: BinTab,
    pub selected_playlist: Option<Id>,
    pub selected_presentation: Option<Id>,
    pub selected_slide: Option<Id>,
    pub selected_element: Option<Id>,
    pub search: String,
    pub renaming: Option<(Renaming, String)>,
    pub new_presentation: Option<NewPresentation>,
    pub reflow: Option<(Id, String)>,
    pub message_values: HashMap<Id, Vec<(String, String)>>,
    pub stage_message_draft: String,
    pub import_path: String,
    pub canvas_drag: Option<CanvasDrag>,
    pub clipboard: Option<Slide>,

    pub audience_open: bool,
    pub stage_open: bool,
    pub screens_open: bool,
    pub notices: Vec<String>,
}

impl App {
    pub fn new() -> Self {
        let (prefs, report) = SettingsStore::<Prefs>::open(Location::Auto);
        let path = store::library_path();
        let mut notices = Vec::new();
        if report.has_problems() {
            notices.push("Some preferences could not be read and were reset.".to_string());
        }
        let mut autosave = Autosave::default();
        let lib = match store::load(path.as_deref()) {
            Loaded::Existing(lib) | Loaded::Fresh(lib) => lib,
            Loaded::Damaged(lib, why) => {
                notices.push(format!("{why}. Saving is off so the file is not overwritten; a demo library is open instead."));
                autosave.disabled = true;
                lib
            }
        };
        Self::with(lib, prefs, path, autosave, notices)
    }

    /// An app with a given library and nothing on disk, for tests and screenshots.
    #[cfg(test)]
    pub fn in_memory(lib: Library) -> Self {
        let (prefs, _) = SettingsStore::<Prefs>::open(Location::Memory);
        Self::with(lib, prefs, None, Autosave::default(), Vec::new())
    }

    fn with(lib: Library, prefs: SettingsStore<Prefs>, path: Option<PathBuf>, autosave: Autosave, notices: Vec<String>) -> Self {
        let selected_playlist = lib.playlists.first().map(|p| p.id);
        let selected_presentation = lib.playlists.first().and_then(|pl| {
            pl.items.iter().find_map(|i| match i {
                PlaylistItem::Presentation { presentation, .. } => Some(*presentation),
                _ => None,
            })
        });
        let (bibles, notices) = {
            let (b, errors) = bible::load_all(bible::bibles_dir(path.as_deref()).as_deref());
            let mut n = notices;
            n.extend(errors);
            (b, n)
        };
        let open_outputs = prefs.get().outputs.open_on_launch;
        let mut live = Live { playlist: selected_playlist, ..Default::default() };
        live.transition = Duration::from_millis(u64::from(prefs.get().show.transition_ms));
        Self {
            lib,
            live,
            cache: MediaCache::default(),
            audio: AudioEngine::new(),
            bibles,
            scripture: ScriptureUi::default(),
            prefs,
            autosave,
            path,
            mode: Mode::Show,
            bin_tab: BinTab::Media,
            selected_playlist,
            selected_presentation,
            selected_slide: None,
            selected_element: None,
            search: String::new(),
            renaming: None,
            new_presentation: None,
            reflow: None,
            message_values: HashMap::new(),
            stage_message_draft: String::new(),
            import_path: String::new(),
            canvas_drag: None,
            clipboard: None,
            audience_open: open_outputs,
            stage_open: open_outputs,
            screens_open: false,
            notices,
        }
    }

    /// Record that the library changed so autosave picks it up.
    pub fn changed(&mut self) {
        self.autosave.mark(Instant::now());
    }

    pub fn presentation(&self) -> Option<&Presentation> {
        self.selected_presentation.and_then(|id| self.lib.presentation(id))
    }

    pub fn trigger(&mut self, presentation: Id, slide: Id) {
        self.live.trigger(&self.lib, presentation, slide, Instant::now());
        self.selected_slide = Some(slide);
        let audio = self.lib.presentation(presentation).and_then(|p| p.find_slide(slide)).and_then(|(_, s)| s.audio_action);
        if let Some(a) = audio {
            self.play_audio(a);
        }
    }

    /// Start an audio item on the audio layer, or report why it cannot play.
    pub fn play_audio(&mut self, id: Id) {
        if let Some(MediaSource::Audio { path }) = self.lib.media_item(id).map(|m| m.source.clone()) {
            self.audio.play_reporting(id, &path);
        }
    }

    /// Clear All also silences the audio layer.
    pub fn clear_all(&mut self) {
        self.live.clear_all(Instant::now());
        self.audio.stop();
    }

    pub fn go_next(&mut self) {
        if self.live.slide.is_none() {
            // Nothing live yet: start the selected presentation.
            if let Some(p) = self.presentation()
                && let Some(c) = p.cues().first().copied()
                && let Some(s) = p.slide(c)
            {
                let (pid, sid) = (p.id, s.id);
                self.trigger(pid, sid);
            }
            return;
        }
        let advance = self.prefs.get().show.advance_through_playlist;
        let saved = self.live.playlist;
        if !advance {
            self.live.playlist = None;
        }
        let next = self.live.next_cue(&self.lib);
        self.live.playlist = saved;
        if let Some(n) = next {
            self.selected_presentation = Some(n.presentation);
            self.trigger(n.presentation, n.slide);
        }
    }

    pub fn go_prev(&mut self) {
        if let Some(n) = self.live.prev_cue(&self.lib) {
            self.selected_presentation = Some(n.presentation);
            self.trigger(n.presentation, n.slide);
        }
    }

    pub fn keyboard(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        use egui::Key;
        let now = Instant::now();
        let pressed = |k: Key| ctx.input(|i| i.key_pressed(k));
        if self.mode == Mode::Show {
            if pressed(Key::Space) || pressed(Key::ArrowRight) || pressed(Key::PageDown) || pressed(Key::ArrowDown) {
                self.go_next();
            }
            if pressed(Key::ArrowLeft) || pressed(Key::PageUp) || pressed(Key::ArrowUp) {
                self.go_prev();
            }
        }
        if pressed(Key::F1) {
            self.clear_all();
        }
        if pressed(Key::F7) {
            self.audio.stop();
        }
        if pressed(Key::F2) {
            self.live.clear_slide(now);
        }
        if pressed(Key::F3) {
            self.live.clear_media(now);
        }
        if pressed(Key::F4) {
            self.live.clear_props();
        }
        if pressed(Key::F5) {
            self.live.clear_messages();
        }
        if pressed(Key::F6) {
            self.live.toggle_logo(now);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Num1)) {
            self.audience_open = !self.audience_open;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Num2)) {
            self.stage_open = !self.stage_open;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::S)) {
            self.autosave.mark(now);
            self.save_now();
        }
    }

    pub fn save_now(&mut self) {
        let now = Instant::now();
        self.autosave.tick(self.path.as_deref(), &self.lib, now, true);
    }

    /// Files dropped anywhere on the main window become media items.
    fn dropped_files(&mut self, ctx: &egui::Context) {
        let files = ctx.input(|i| i.raw.dropped_files.clone());
        for f in files {
            let path = f.path();
            if !path.as_os_str().is_empty() {
                self.import_media(&path.to_string_lossy());
            }
        }
    }

    pub fn import_media(&mut self, path: &str) {
        let path = path.trim().trim_matches('"');
        if path.is_empty() {
            return;
        }
        let name = std::path::Path::new(path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string());
        let ext = std::path::Path::new(path).extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let (source, bin) = if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            (MediaSource::Audio { path: path.to_string() }, "Audio")
        } else {
            (MediaSource::Image { path: path.to_string() }, "Imported")
        };
        let id = self.lib.next_id();
        self.lib.media.push(MediaItem { id, name, source, fit: MediaFit::Fill, bin: bin.into() });
        self.changed();
    }

    /// One frame of the whole app, given the root viewport's UI.
    pub fn frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let now = Instant::now();
        self.cache.animating = false;
        self.audio.tick();
        self.keyboard(&ctx);
        self.dropped_files(&ctx);

        ui::toolbar::show(self, ui);
        ui::preview::show(self, ui);
        ui::sidebar::show(self, ui);
        if self.prefs.get().layout.show_media_bin {
            ui::bin::show(self, ui);
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(ui::theme::CENTER_BG)).show(ui, |ui| match self.mode {
            Mode::Show => ui::grid::show(self, ui),
            Mode::Edit => ui::editor::show(self, ui),
            Mode::Reflow => ui::reflow::show(self, ui),
            Mode::Bible => ui::scripture::show(self, ui),
        });
        ui::bin::refresh_message(self, now);
        ui::dialogs::show(self, &ctx);
        ui::dialogs::screens(self, &ctx);
        ui::outputs::show(self, &ctx);

        // Keep the clock, timers and fades moving; outputs are live even with no input.
        let animating =
            self.live.fade(self.live.slide_changed, now) < 1.0 || self.live.fade(self.live.media_changed, now) < 1.0 || self.cache.animating;
        if animating {
            ctx.request_repaint();
        } else if self.audio.is_playing() {
            ctx.request_repaint_after(Duration::from_millis(100));
        } else {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        if let Some(wait) = self.autosave.tick(self.path.as_deref(), &self.lib, now, false) {
            ctx.request_repaint_after(wait);
        }
        let tick = self.prefs.tick(now);
        if let SaveStatus::Failed(err) = &tick.status {
            let msg = format!("Preferences could not be saved: {err}");
            if !self.notices.contains(&msg) {
                self.notices.push(msg);
            }
        }
        if let Some(deadline) = tick.next_deadline {
            ctx.request_repaint_after(deadline.saturating_duration_since(now));
        }
    }

    pub fn on_exit(&mut self) {
        self.save_now();
        let _ = self.prefs.flush();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        App::on_exit(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 900.0))),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.frame(ui));
        out.textures_delta.clear();
    }

    fn key(k: egui::Key) -> egui::Event {
        egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }
    }

    #[test]
    fn every_mode_and_window_renders_headless() {
        let ctx = egui::Context::default();
        crate::ui::theme::apply(&ctx);
        let mut app = App::in_memory(store::demo_library());
        app.audience_open = true;
        app.stage_open = true;
        for mode in [Mode::Show, Mode::Edit, Mode::Reflow, Mode::Bible] {
            app.mode = mode;
            for tab in BinTab::ALL {
                app.bin_tab = tab;
                run(&mut app, &ctx, Vec::new());
            }
        }
        app.new_presentation = Some(NewPresentation::default());
        app.screens_open = true;
        run(&mut app, &ctx, Vec::new());
        assert!(app.screens_open);
        assert!(app.new_presentation.is_some());
    }

    #[test]
    fn space_walks_the_playlist_and_f1_clears() {
        let ctx = egui::Context::default();
        let mut app = App::in_memory(store::demo_library());
        run(&mut app, &ctx, Vec::new());
        run(&mut app, &ctx, vec![key(egui::Key::Space)]);
        let first = app.live.slide;
        assert!(first.is_some());
        // Walk far enough to leave the first presentation for the next playlist item.
        for _ in 0..4 {
            run(&mut app, &ctx, vec![key(egui::Key::Space)]);
        }
        assert_ne!(app.live.slide.map(|l| l.presentation), first.map(|l| l.presentation));
        run(&mut app, &ctx, vec![key(egui::Key::ArrowLeft)]);
        assert!(app.live.slide.is_some());
        run(&mut app, &ctx, vec![key(egui::Key::F1)]);
        assert!(app.live.slide.is_none() && app.live.media.is_none());
    }

    #[test]
    fn scripture_goes_live_and_refreshes_instead_of_duplicating() {
        let ctx = egui::Context::default();
        let mut app = App::in_memory(store::demo_library());
        app.mode = Mode::Bible;
        app.scripture.reference = "Ps 23:1-2".into();
        run(&mut app, &ctx, Vec::new());
        let r = crate::bible::parse_reference(&app.scripture.reference);
        let bible = app.bibles[0].clone();
        crate::ui::scripture::build(&mut app, &bible, r, true);
        crate::ui::scripture::build(&mut app, &bible, r, true);
        let count = app.lib.presentations.iter().filter(|p| p.category == "Scripture").count();
        assert_eq!(count, 1);
        let live = app.live.live_slide(&app.lib).map(|(_, s)| s.text());
        assert!(live.is_some_and(|t| t.contains("The LORD is my shepherd")));
        run(&mut app, &ctx, Vec::new());
    }

    #[test]
    fn sound_files_import_as_audio_and_never_paint() {
        let mut app = App::in_memory(store::demo_library());
        app.import_media("/music/walk-in.mp3");
        app.import_media("/pictures/loop.gif");
        let n = app.lib.media.len();
        assert!(matches!(app.lib.media[n - 2].source, MediaSource::Audio { .. }));
        assert!(matches!(app.lib.media[n - 1].source, MediaSource::Image { .. }));
        // Playing a missing file reports an error instead of failing.
        let id = app.lib.media[n - 2].id;
        app.play_audio(id);
        assert!(app.audio.error.is_some());
        assert_eq!(app.audio.current, None);
    }
}
