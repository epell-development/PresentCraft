//! The show document: presentations, playlists, media, messages, timers and props.
//!
//! Everything here is plain data with serde derives. Ids are random-free counters handed out by
//! [`Library::next_id`], so a saved library reloads with identical ids.

use serde::{Deserialize, Serialize};

pub type Id = u64;

/// The design canvas every slide is laid out on. Output scales it to the real screen.
pub const CANVAS_W: f32 = 1920.0;
pub const CANVAS_H: f32 = 1080.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgba(pub [u8; 4]);

impl Rgba {
    pub const WHITE: Self = Self([255, 255, 255, 255]);
    pub const BLACK: Self = Self([0, 0, 0, 255]);

    pub fn to_egui(self) -> egui::Color32 {
        let [r, g, b, a] = self.0;
        egui::Color32::from_rgba_unmultiplied(r, g, b, a)
    }

    pub fn from_egui(c: egui::Color32) -> Self {
        Self(c.to_srgba_unmultiplied())
    }
}

/// Group colors offered in the slide context menu, close to the usual song-section palette.
pub const GROUP_COLORS: &[(&str, Rgba)] = &[
    ("Verse", Rgba([45, 108, 223, 255])),
    ("Chorus", Rgba([214, 48, 49, 255])),
    ("Pre-Chorus", Rgba([232, 126, 4, 255])),
    ("Bridge", Rgba([142, 68, 173, 255])),
    ("Tag", Rgba([22, 160, 133, 255])),
    ("Intro", Rgba([39, 174, 96, 255])),
    ("Ending", Rgba([127, 140, 141, 255])),
    ("Blank", Rgba([60, 60, 60, 255])),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VAlign {
    Top,
    #[default]
    Middle,
    Bottom,
}

/// A rectangle on the 1920x1080 canvas.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Frame {
    pub const FULL_TEXT: Self = Self { x: 120.0, y: 90.0, w: 1680.0, h: 900.0 };
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextStyle {
    pub size: f32,
    pub color: Rgba,
    pub h_align: HAlign,
    pub v_align: VAlign,
    pub shadow: bool,
    pub outline: bool,
    pub uppercase: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self { size: 84.0, color: Rgba::WHITE, h_align: HAlign::Center, v_align: VAlign::Middle, shadow: true, outline: false, uppercase: false }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ElementKind {
    Text { text: String, style: TextStyle },
    Shape { fill: Rgba, corner: f32 },
    Media { media: Id },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: Id,
    pub name: String,
    pub frame: Frame,
    pub kind: ElementKind,
    #[serde(default)]
    pub fill: Option<Rgba>,
}

/// One cue in a presentation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slide {
    pub id: Id,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub elements: Vec<Element>,
    /// A solid behind the elements; `None` lets the media layer show through.
    #[serde(default)]
    pub background: Option<Rgba>,
    /// Media to fire on the media layer when this slide is triggered.
    #[serde(default)]
    pub media_action: Option<Id>,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "yes")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

impl Slide {
    /// All text on the slide, used for stage display, search and the reflow editor.
    pub fn text(&self) -> String {
        let parts: Vec<&str> = self
            .elements
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        parts.join("\n")
    }
}

/// A named, colored section of a presentation such as "Verse 1" or "Chorus".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: Id,
    pub name: String,
    pub color: Rgba,
    pub slides: Vec<Slide>,
}

/// An ordering of groups, so a chorus can repeat without duplicating its slides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Arrangement {
    pub id: Id,
    pub name: String,
    pub groups: Vec<Id>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Presentation {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub category: String,
    pub groups: Vec<Group>,
    #[serde(default)]
    pub arrangements: Vec<Arrangement>,
    #[serde(default)]
    pub active_arrangement: Option<Id>,
    /// Applied when new slides are added and by "Apply theme".
    #[serde(default)]
    pub theme: Option<Id>,
    #[serde(default)]
    pub ccli: String,
}

/// A slide's position in the play order: group index, slide index, and the group's color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cue {
    pub group: usize,
    pub slide: usize,
}

impl Presentation {
    /// Groups in play order: the active arrangement if one is set, else document order.
    pub fn play_groups(&self) -> Vec<usize> {
        let arrangement = self.active_arrangement.and_then(|id| self.arrangements.iter().find(|a| a.id == id));
        match arrangement {
            Some(a) => a.groups.iter().filter_map(|gid| self.groups.iter().position(|g| g.id == *gid)).collect(),
            None => (0..self.groups.len()).collect(),
        }
    }

    /// Every enabled slide in play order.
    pub fn cues(&self) -> Vec<Cue> {
        let mut out = Vec::new();
        for g in self.play_groups() {
            if let Some(group) = self.groups.get(g) {
                for (s, slide) in group.slides.iter().enumerate() {
                    if slide.enabled {
                        out.push(Cue { group: g, slide: s });
                    }
                }
            }
        }
        out
    }

    pub fn slide(&self, cue: Cue) -> Option<&Slide> {
        self.groups.get(cue.group)?.slides.get(cue.slide)
    }

    pub fn find_slide(&self, slide_id: Id) -> Option<(Cue, &Slide)> {
        self.groups.iter().enumerate().find_map(|(g, group)| {
            group.slides.iter().enumerate().find(|(_, s)| s.id == slide_id).map(|(s, slide)| (Cue { group: g, slide: s }, slide))
        })
    }

    pub fn find_slide_mut(&mut self, slide_id: Id) -> Option<&mut Slide> {
        self.groups.iter_mut().flat_map(|g| g.slides.iter_mut()).find(|s| s.id == slide_id)
    }

    pub fn slide_count(&self) -> usize {
        self.groups.iter().map(|g| g.slides.len()).sum()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlaylistItem {
    Presentation { id: Id, presentation: Id },
    Header { id: Id, name: String, color: Rgba },
    Media { id: Id, media: Id },
}

impl PlaylistItem {
    pub fn id(&self) -> Id {
        match self {
            Self::Presentation { id, .. } | Self::Header { id, .. } | Self::Media { id, .. } => *id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: Id,
    pub name: String,
    pub items: Vec<PlaylistItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MediaSource {
    /// A still image on disk.
    Image { path: String },
    /// A generated solid color, handy for backgrounds without files.
    Color { color: Rgba },
    /// A generated two-color vertical gradient.
    Gradient { top: Rgba, bottom: Rgba },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MediaFit {
    #[default]
    Fill,
    Fit,
    Stretch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: Id,
    pub name: String,
    pub source: MediaSource,
    #[serde(default)]
    pub fit: MediaFit,
    /// Media bin playlist (folder) the item belongs to.
    #[serde(default)]
    pub bin: String,
}

/// A reusable slide look applied to presentations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub id: Id,
    pub name: String,
    pub frame: Frame,
    pub style: TextStyle,
    pub background: Option<Rgba>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TimerKind {
    Countdown { seconds: u32 },
    CountdownTo { hour: u8, minute: u8 },
    Elapsed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Timer {
    pub id: Id,
    pub name: String,
    pub kind: TimerKind,
    #[serde(default)]
    pub overrun: bool,
}

/// A message shown over everything on the messages layer, e.g. "Parent of child 42".
/// `{token}` placeholders are filled in when the message is shown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Id,
    pub name: String,
    pub template: String,
    #[serde(default)]
    pub timer: Option<Id>,
}

impl Message {
    /// Names of `{token}` placeholders, in order, without duplicates.
    pub fn tokens(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut rest = self.template.as_str();
        while let Some(start) = rest.find('{') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('}') else { break };
            let name = after[..end].trim().to_string();
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
            rest = &after[end + 1..];
        }
        out
    }

    pub fn fill(&self, values: &[(String, String)], timer_text: Option<&str>) -> String {
        let mut text = self.template.clone();
        for (k, v) in values {
            text = text.replace(&format!("{{{k}}}"), v);
        }
        if let Some(t) = timer_text {
            text = text.replace("{timer}", t);
        }
        text
    }
}

/// A prop is a slide that lives on its own layer above slides, such as a logo bug or lower third.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prop {
    pub id: Id,
    pub name: String,
    pub elements: Vec<Element>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StageLayout {
    #[default]
    CurrentNext,
    TextOnly,
    NotesFocus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub version: u32,
    pub next_id: Id,
    pub presentations: Vec<Presentation>,
    pub playlists: Vec<Playlist>,
    pub media: Vec<MediaItem>,
    pub themes: Vec<Theme>,
    pub timers: Vec<Timer>,
    pub messages: Vec<Message>,
    pub props: Vec<Prop>,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            presentations: Vec::new(),
            playlists: Vec::new(),
            media: Vec::new(),
            themes: Vec::new(),
            timers: Vec::new(),
            messages: Vec::new(),
            props: Vec::new(),
        }
    }
}

impl Library {
    pub fn next_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    pub fn presentation(&self, id: Id) -> Option<&Presentation> {
        self.presentations.iter().find(|p| p.id == id)
    }

    pub fn presentation_mut(&mut self, id: Id) -> Option<&mut Presentation> {
        self.presentations.iter_mut().find(|p| p.id == id)
    }

    pub fn media_item(&self, id: Id) -> Option<&MediaItem> {
        self.media.iter().find(|m| m.id == id)
    }

    pub fn theme(&self, id: Id) -> Option<&Theme> {
        self.themes.iter().find(|t| t.id == id)
    }

    pub fn text_element(&mut self, text: &str, theme: Option<&Theme>) -> Element {
        let (frame, style) = theme.map(|t| (t.frame, t.style.clone())).unwrap_or((Frame::FULL_TEXT, TextStyle::default()));
        Element { id: self.next_id(), name: "Text".into(), frame, kind: ElementKind::Text { text: text.into(), style }, fill: None }
    }

    pub fn new_slide(&mut self, text: &str, theme_id: Option<Id>) -> Slide {
        let theme = theme_id.and_then(|id| self.theme(id)).cloned();
        let elements = if text.is_empty() && theme.is_none() { Vec::new() } else { vec![self.text_element(text, theme.as_ref())] };
        Slide {
            id: self.next_id(),
            label: String::new(),
            elements,
            background: theme.and_then(|t| t.background),
            media_action: None,
            notes: String::new(),
            enabled: true,
        }
    }

    /// Build a presentation from lyrics: blank lines split slides, and a line such as
    /// `[Chorus]` or `Verse 2:` starts a new group.
    pub fn presentation_from_lyrics(&mut self, name: &str, lyrics: &str, theme: Option<Id>) -> Presentation {
        let id = self.next_id();
        let groups = self.groups_from_lyrics(lyrics, theme);
        Presentation {
            id,
            name: name.into(),
            category: "Songs".into(),
            groups,
            arrangements: Vec::new(),
            active_arrangement: None,
            theme,
            ccli: String::new(),
        }
    }

    pub fn groups_from_lyrics(&mut self, lyrics: &str, theme: Option<Id>) -> Vec<Group> {
        let mut groups: Vec<Group> = Vec::new();
        let mut current: Vec<String> = Vec::new();
        let flush = |lib: &mut Library, groups: &mut Vec<Group>, current: &mut Vec<String>| {
            if current.is_empty() {
                return;
            }
            let text = current.join("\n");
            current.clear();
            if groups.is_empty() {
                let gid = lib.next_id();
                groups.push(Group { id: gid, name: "Verse 1".into(), color: group_color("Verse"), slides: Vec::new() });
            }
            let slide = lib.new_slide(&text, theme);
            if let Some(g) = groups.last_mut() {
                g.slides.push(slide);
            }
        };
        for raw in lyrics.lines() {
            let line = raw.trim();
            if line.is_empty() {
                flush(self, &mut groups, &mut current);
            } else if let Some(name) = section_header(line) {
                flush(self, &mut groups, &mut current);
                let gid = self.next_id();
                groups.push(Group { id: gid, color: group_color(&name), name, slides: Vec::new() });
            } else {
                current.push(line.to_string());
            }
        }
        flush(self, &mut groups, &mut current);
        groups
    }

    /// Lyrics text for the reflow editor, the inverse of [`Self::groups_from_lyrics`].
    pub fn lyrics_of(presentation: &Presentation) -> String {
        let mut out = String::new();
        for g in &presentation.groups {
            out.push_str(&format!("[{}]\n", g.name));
            for s in &g.slides {
                out.push_str(s.text().trim());
                out.push_str("\n\n");
            }
        }
        out.trim_end().to_string()
    }

    /// Every place a presentation id is referenced, so deleting one leaves no dangling items.
    pub fn remove_presentation(&mut self, id: Id) {
        self.presentations.retain(|p| p.id != id);
        for pl in &mut self.playlists {
            pl.items.retain(|i| !matches!(i, PlaylistItem::Presentation { presentation, .. } if *presentation == id));
        }
    }

    pub fn remove_media(&mut self, id: Id) {
        self.media.retain(|m| m.id != id);
        for pl in &mut self.playlists {
            pl.items.retain(|i| !matches!(i, PlaylistItem::Media { media, .. } if *media == id));
        }
        for p in &mut self.presentations {
            for s in p.groups.iter_mut().flat_map(|g| g.slides.iter_mut()) {
                if s.media_action == Some(id) {
                    s.media_action = None;
                }
                s.elements.retain(|e| !matches!(e.kind, ElementKind::Media { media } if media == id));
            }
        }
    }
}

/// `[Chorus]`, `Chorus:`, `Verse 2` and similar known section names start a group.
pub fn section_header(line: &str) -> Option<String> {
    let bracketed = line.strip_prefix('[').and_then(|l| l.strip_suffix(']'));
    if let Some(name) = bracketed {
        let name = name.trim();
        return (!name.is_empty()).then(|| name.to_string());
    }
    let candidate = line.strip_suffix(':').unwrap_or(line).trim();
    let first = candidate.split_whitespace().next().unwrap_or("");
    let known = ["verse", "chorus", "pre-chorus", "prechorus", "bridge", "tag", "intro", "outro", "ending", "refrain", "interlude"];
    let rest_is_number = candidate.split_whitespace().skip(1).all(|w| w.chars().all(|c| c.is_ascii_digit()));
    let words = candidate.split_whitespace().count();
    (known.contains(&first.to_ascii_lowercase().as_str()) && words <= 2 && rest_is_number).then(|| candidate.to_string())
}

pub fn group_color(name: &str) -> Rgba {
    let lower = name.to_ascii_lowercase();
    let key = if lower.starts_with("pre") {
        "Pre-Chorus"
    } else if lower.starts_with("chorus") || lower.starts_with("refrain") {
        "Chorus"
    } else if lower.starts_with("bridge") {
        "Bridge"
    } else if lower.starts_with("tag") {
        "Tag"
    } else if lower.starts_with("intro") || lower.starts_with("interlude") {
        "Intro"
    } else if lower.starts_with("end") || lower.starts_with("outro") {
        "Ending"
    } else {
        "Verse"
    };
    GROUP_COLORS.iter().find(|(n, _)| *n == key).map(|(_, c)| *c).unwrap_or(Rgba::WHITE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lyrics_split_into_groups_and_slides() {
        let mut lib = Library::default();
        let p = lib.presentation_from_lyrics("Song", "[Verse 1]\na\nb\n\nc\n\nChorus:\nd\n\n[Bridge]\ne", None);
        let names: Vec<&str> = p.groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["Verse 1", "Chorus", "Bridge"]);
        assert_eq!(p.groups[0].slides.len(), 2);
        assert_eq!(p.groups[0].slides[0].text(), "a\nb");
        assert_eq!(p.groups[1].color, group_color("Chorus"));
    }

    #[test]
    fn text_without_headers_lands_in_a_default_group() {
        let mut lib = Library::default();
        let p = lib.presentation_from_lyrics("Song", "one\n\ntwo", None);
        assert_eq!(p.groups.len(), 1);
        assert_eq!(p.slide_count(), 2);
    }

    #[test]
    fn ordinary_lines_are_not_headers() {
        assert_eq!(section_header("Verse of the day is here"), None);
        assert_eq!(section_header("Amazing grace"), None);
        assert_eq!(section_header("Verse 2"), Some("Verse 2".into()));
        assert_eq!(section_header("[Anything]"), Some("Anything".into()));
    }

    #[test]
    fn arrangement_controls_play_order() {
        let mut lib = Library::default();
        let mut p = lib.presentation_from_lyrics("Song", "[Verse]\na\n\n[Chorus]\nb", None);
        let (v, c) = (p.groups[0].id, p.groups[1].id);
        p.arrangements.push(Arrangement { id: 99, name: "Live".into(), groups: vec![c, v, c] });
        p.active_arrangement = Some(99);
        let order: Vec<usize> = p.cues().iter().map(|c| c.group).collect();
        assert_eq!(order, [1, 0, 1]);
    }

    #[test]
    fn disabled_slides_are_skipped() {
        let mut lib = Library::default();
        let mut p = lib.presentation_from_lyrics("Song", "a\n\nb\n\nc", None);
        p.groups[0].slides[1].enabled = false;
        assert_eq!(p.cues().len(), 2);
    }

    #[test]
    fn message_tokens_fill_in() {
        let m = Message { id: 1, name: "Parent".into(), template: "Parent of {Child} please come to {Room}. {Child}".into(), timer: None };
        assert_eq!(m.tokens(), ["Child", "Room"]);
        let text = m.fill(&[("Child".into(), "42".into()), ("Room".into(), "Nursery".into())], None);
        assert_eq!(text, "Parent of 42 please come to Nursery. 42");
    }

    #[test]
    fn lyrics_round_trip() {
        let mut lib = Library::default();
        let p = lib.presentation_from_lyrics("Song", "[Verse 1]\na\nb\n\n[Chorus]\nc", None);
        let text = Library::lyrics_of(&p);
        let again = lib.groups_from_lyrics(&text, None);
        assert_eq!(again.len(), 2);
        assert_eq!(again[0].slides[0].text(), "a\nb");
    }

    #[test]
    fn removing_media_clears_references() {
        let mut lib = Library::default();
        let mid = lib.next_id();
        lib.media.push(MediaItem {
            id: mid,
            name: "bg".into(),
            source: MediaSource::Color { color: Rgba::BLACK },
            fit: MediaFit::Fill,
            bin: String::new(),
        });
        let mut p = lib.presentation_from_lyrics("Song", "a", None);
        p.groups[0].slides[0].media_action = Some(mid);
        lib.presentations.push(p);
        lib.remove_media(mid);
        assert_eq!(lib.presentations[0].groups[0].slides[0].media_action, None);
    }
}
