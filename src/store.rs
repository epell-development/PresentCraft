//! Loading and saving the library file, and the user preferences.
//!
//! The library is `library.json` in the app's data folder, written with `craft-atomic` so a
//! crash mid-save never leaves a truncated show. Preferences go through `craft-settings`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use craft_settings::{DocumentMut, MigrateError, Settings};
use serde::{Deserialize, Serialize};

use crate::model::*;

pub const APP_ID: &str = "presentcraft";

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Prefs {
    pub show: ShowPrefs,
    pub layout: LayoutPrefs,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct ShowPrefs {
    pub transition_ms: u32,
    pub advance_through_playlist: bool,
    pub stage_layout: StageLayout,
}

impl Default for ShowPrefs {
    fn default() -> Self {
        Self { transition_ms: 400, advance_through_playlist: true, stage_layout: StageLayout::CurrentNext }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct LayoutPrefs {
    pub thumbnail_width: f32,
    pub show_media_bin: bool,
}

impl Default for LayoutPrefs {
    fn default() -> Self {
        Self { thumbnail_width: 220.0, show_media_bin: true }
    }
}

impl Settings for Prefs {
    const APP_ID: &'static str = APP_ID;
    const SCHEMA_VERSION: u32 = 1;

    fn migrate(_from: u32, _doc: &mut DocumentMut) -> Result<(), MigrateError> {
        Ok(())
    }
}

/// Where the library lives. `PRESENTCRAFT_DATA_DIR` overrides it; set but empty keeps the
/// library in memory only, which tests and screenshots rely on.
pub fn library_path() -> Option<PathBuf> {
    match std::env::var_os("PRESENTCRAFT_DATA_DIR") {
        Some(v) if v.is_empty() => None,
        Some(v) => Some(PathBuf::from(v).join("library.json")),
        None => craft_settings::paths::data_dir(APP_ID).map(|d| d.join("library.json")),
    }
}

pub enum Loaded {
    Existing(Library),
    /// No file yet: a starter library to explore.
    Fresh(Library),
    /// The file exists but could not be read; it was left untouched and saving is disabled.
    Damaged(Library, String),
}

pub fn load(path: Option<&Path>) -> Loaded {
    let Some(path) = path else {
        return Loaded::Fresh(demo_library());
    };
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<Library>(&bytes) {
            Ok(lib) => Loaded::Existing(lib),
            Err(e) => Loaded::Damaged(demo_library(), format!("{} could not be read: {e}", path.display())),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Loaded::Fresh(demo_library()),
        Err(e) => Loaded::Damaged(demo_library(), format!("{} could not be opened: {e}", path.display())),
    }
}

pub fn save(path: &Path, lib: &Library) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = serde_json::to_vec_pretty(lib).map_err(std::io::Error::other)?;
    craft_atomic::atomic_write(path, &bytes)
}

/// Saves a second after the last change, so typing in the editor does not write every frame.
#[derive(Debug, Default)]
pub struct Autosave {
    dirty_since: Option<Instant>,
    pub last_error: Option<String>,
    pub disabled: bool,
}

impl Autosave {
    pub const DELAY: Duration = Duration::from_secs(1);

    pub fn mark(&mut self, now: Instant) {
        self.dirty_since.get_or_insert(now);
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty_since.is_some()
    }

    /// Returns how long until the next save is due, if one is pending.
    pub fn tick(&mut self, path: Option<&Path>, lib: &Library, now: Instant, force: bool) -> Option<Duration> {
        let since = self.dirty_since?;
        let due = since + Self::DELAY;
        if !force && now < due {
            return Some(due - now);
        }
        if self.disabled {
            return None;
        }
        let Some(path) = path else {
            self.dirty_since = None;
            return None;
        };
        match save(path, lib) {
            Ok(()) => {
                self.dirty_since = None;
                self.last_error = None;
                None
            }
            Err(e) => {
                self.last_error = Some(format!("Could not save the library: {e}"));
                // Retry later rather than every frame.
                self.dirty_since = Some(now);
                Some(Self::DELAY * 5)
            }
        }
    }
}

const AMAZING: &str = "[Verse 1]
Morning breaks across the hills
Light is pouring through the trees

All the valleys wake and sing
Every heart is set at ease

[Chorus]
Rise up, rise up, the day has come
Lift your voice and run

Rise up, rise up, our song is one
Underneath the sun

[Verse 2]
Rivers carry what we've known
Mountains hold the stories old

Hand in hand we journey on
Every promise to unfold

[Bridge]
We will not be silent
We will not be still

We will keep on singing
Over every hill";

const HARBOR: &str = "[Verse 1]
Quiet harbor, steady light
Guide me through the darkest night

[Chorus]
You are the harbor, you are the shore
Calling me home forevermore

[Verse 2]
When the waves are rising high
You are calm beneath the sky

[Ending]
Calling me home
Forevermore";

/// A small starter show using original lyrics, so a first launch has something to click.
pub fn demo_library() -> Library {
    let mut lib = Library::default();

    let theme_id = lib.next_id();
    lib.themes.push(Theme { id: theme_id, name: "Lyrics: centered".into(), frame: Frame::FULL_TEXT, style: TextStyle::default(), background: None });
    let lower_id = lib.next_id();
    lib.themes.push(Theme {
        id: lower_id,
        name: "Lyrics: lower third".into(),
        frame: Frame { x: 120.0, y: 760.0, w: 1680.0, h: 260.0 },
        style: TextStyle { size: 64.0, v_align: VAlign::Bottom, ..TextStyle::default() },
        background: None,
    });
    let title_id = lib.next_id();
    lib.themes.push(Theme {
        id: title_id,
        name: "Title".into(),
        frame: Frame { x: 160.0, y: 300.0, w: 1600.0, h: 480.0 },
        style: TextStyle { size: 120.0, uppercase: true, ..TextStyle::default() },
        background: Some(Rgba([18, 22, 38, 255])),
    });

    let bins = [
        ("Deep blue", MediaSource::Gradient { top: Rgba([10, 24, 64, 255]), bottom: Rgba([2, 6, 20, 255]) }),
        ("Sunrise", MediaSource::Gradient { top: Rgba([250, 140, 60, 255]), bottom: Rgba([120, 30, 80, 255]) }),
        ("Forest", MediaSource::Gradient { top: Rgba([30, 90, 60, 255]), bottom: Rgba([8, 30, 22, 255]) }),
        ("Slate", MediaSource::Color { color: Rgba([40, 44, 52, 255]) }),
        ("Black", MediaSource::Color { color: Rgba::BLACK }),
    ];
    let mut media_ids = Vec::new();
    for (name, source) in bins {
        let id = lib.next_id();
        media_ids.push(id);
        lib.media.push(MediaItem { id, name: name.into(), source, fit: MediaFit::Fill, bin: "Backgrounds".into() });
    }

    let mut welcome = Presentation {
        id: lib.next_id(),
        name: "Welcome".into(),
        category: "Service".into(),
        groups: Vec::new(),
        arrangements: Vec::new(),
        active_arrangement: None,
        theme: Some(title_id),
        ccli: String::new(),
    };
    let gid = lib.next_id();
    let mut s1 = lib.new_slide("Welcome", Some(title_id));
    s1.notes = "Walk-in. Start the countdown.".into();
    let s2 = lib.new_slide("Please silence your phones", Some(title_id));
    welcome.groups.push(Group { id: gid, name: "Welcome".into(), color: Rgba([39, 174, 96, 255]), slides: vec![s1, s2] });

    let mut song1 = lib.presentation_from_lyrics("Rise Up", AMAZING, Some(theme_id));
    if let Some(first) = song1.groups.first_mut().and_then(|g| g.slides.first_mut()) {
        first.media_action = media_ids.first().copied();
        first.notes = "Band in on the downbeat.".into();
    }
    let (v1, ch, v2, br) = (song1.groups[0].id, song1.groups[1].id, song1.groups[2].id, song1.groups[3].id);
    let arr = lib.next_id();
    song1.arrangements.push(Arrangement { id: arr, name: "Sunday".into(), groups: vec![v1, ch, v2, ch, br, ch] });
    song1.ccli = "Original song for the PresentCraft demo".into();

    let mut song2 = lib.presentation_from_lyrics("Harbor", HARBOR, Some(lower_id));
    if let Some(first) = song2.groups.first_mut().and_then(|g| g.slides.first_mut()) {
        first.media_action = media_ids.get(2).copied();
    }

    let pl_items = [welcome.id, song1.id, song2.id];
    lib.presentations.push(welcome);
    lib.presentations.push(song1);
    lib.presentations.push(song2);

    let header = lib.next_id();
    let mut items = vec![PlaylistItem::Header { id: header, name: "Worship".into(), color: Rgba([214, 48, 49, 255]) }];
    for p in pl_items {
        let id = lib.next_id();
        items.push(PlaylistItem::Presentation { id, presentation: p });
    }
    let plid = lib.next_id();
    lib.playlists.push(Playlist { id: plid, name: "Sunday Service".into(), items });

    let t1 = lib.next_id();
    lib.timers.push(Timer { id: t1, name: "Service countdown".into(), kind: TimerKind::Countdown { seconds: 300 }, overrun: true });
    let t2 = lib.next_id();
    lib.timers.push(Timer { id: t2, name: "Sermon".into(), kind: TimerKind::Elapsed, overrun: false });

    let m1 = lib.next_id();
    lib.messages.push(Message {
        id: m1,
        name: "Parent pager".into(),
        template: "Parent of {Number}, please come to the nursery".into(),
        timer: None,
    });
    let m2 = lib.next_id();
    lib.messages.push(Message { id: m2, name: "Countdown".into(), template: "Service begins in {timer}".into(), timer: Some(t1) });

    let prop = lib.next_id();
    let el = lib.next_id();
    lib.props.push(Prop {
        id: prop,
        name: "Corner bug".into(),
        elements: vec![Element {
            id: el,
            name: "Bug".into(),
            frame: Frame { x: 1580.0, y: 40.0, w: 300.0, h: 80.0 },
            kind: ElementKind::Text { text: "LIVE".into(), style: TextStyle { size: 48.0, ..TextStyle::default() } },
            fill: Some(Rgba([214, 48, 49, 220])),
        }],
    });
    lib
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("presentcraft-test-{}", std::process::id()));
        let path = dir.join("library.json");
        let lib = demo_library();
        assert!(save(&path, &lib).is_ok());
        let again = match load(Some(&path)) {
            Loaded::Existing(again) => Some(again),
            _ => None,
        };
        assert_eq!(again, Some(lib));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn damaged_file_is_reported_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("presentcraft-bad-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("library.json");
        let _ = std::fs::write(&path, b"{ not json");
        assert!(matches!(load(Some(&path)), Loaded::Damaged(..)));
        assert_eq!(std::fs::read(&path).ok().as_deref(), Some(&b"{ not json"[..]));
        let _ = std::fs::remove_dir_all(dir);
    }
}
