//! Scripture: translations, reference parsing and turning passages into slides.
//!
//! Translations are imported from plain text and kept as JSON files beside the library. A small
//! public-domain (King James Version) sample ships built in so the feature works out of the box.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::*;

/// The 66 books with the abbreviations people commonly type.
pub const BOOKS: &[(&str, &[&str])] = &[
    ("Genesis", &["gen", "ge", "gn"]),
    ("Exodus", &["exod", "exo", "ex"]),
    ("Leviticus", &["lev", "le", "lv"]),
    ("Numbers", &["num", "nu", "nm"]),
    ("Deuteronomy", &["deut", "deu", "dt"]),
    ("Joshua", &["josh", "jos"]),
    ("Judges", &["judg", "jdg"]),
    ("Ruth", &["ru", "rth"]),
    ("1 Samuel", &["1sam", "1sa", "1 sam", "1 sa", "i samuel"]),
    ("2 Samuel", &["2sam", "2sa", "2 sam", "2 sa", "ii samuel"]),
    ("1 Kings", &["1kgs", "1ki", "1 kgs", "1 ki", "i kings"]),
    ("2 Kings", &["2kgs", "2ki", "2 kgs", "2 ki", "ii kings"]),
    ("1 Chronicles", &["1chr", "1ch", "1 chr", "1 ch", "i chronicles"]),
    ("2 Chronicles", &["2chr", "2ch", "2 chr", "2 ch", "ii chronicles"]),
    ("Ezra", &["ezr"]),
    ("Nehemiah", &["neh", "ne"]),
    ("Esther", &["esth", "est"]),
    ("Job", &["jb"]),
    ("Psalms", &["ps", "psa", "psalm", "pss"]),
    ("Proverbs", &["prov", "pro", "prv"]),
    ("Ecclesiastes", &["eccl", "ecc", "qoh"]),
    ("Song of Solomon", &["song", "sos", "song of songs", "canticles"]),
    ("Isaiah", &["isa", "is"]),
    ("Jeremiah", &["jer", "je"]),
    ("Lamentations", &["lam", "la"]),
    ("Ezekiel", &["ezek", "eze"]),
    ("Daniel", &["dan", "da", "dn"]),
    ("Hosea", &["hos", "ho"]),
    ("Joel", &["jl"]),
    ("Amos", &["am"]),
    ("Obadiah", &["obad", "ob"]),
    ("Jonah", &["jon"]),
    ("Micah", &["mic"]),
    ("Nahum", &["nah", "na"]),
    ("Habakkuk", &["hab"]),
    ("Zephaniah", &["zeph", "zep"]),
    ("Haggai", &["hag"]),
    ("Zechariah", &["zech", "zec"]),
    ("Malachi", &["mal"]),
    ("Matthew", &["matt", "mt", "mat"]),
    ("Mark", &["mk", "mr", "mrk"]),
    ("Luke", &["lk", "luk"]),
    ("John", &["jn", "jhn", "joh"]),
    ("Acts", &["act", "ac"]),
    ("Romans", &["rom", "ro", "rm"]),
    ("1 Corinthians", &["1cor", "1co", "1 cor", "1 co", "i corinthians"]),
    ("2 Corinthians", &["2cor", "2co", "2 cor", "2 co", "ii corinthians"]),
    ("Galatians", &["gal", "ga"]),
    ("Ephesians", &["eph", "ephes"]),
    ("Philippians", &["phil", "php", "pp"]),
    ("Colossians", &["col", "co"]),
    ("1 Thessalonians", &["1thess", "1th", "1 thess", "1 th", "i thessalonians"]),
    ("2 Thessalonians", &["2thess", "2th", "2 thess", "2 th", "ii thessalonians"]),
    ("1 Timothy", &["1tim", "1ti", "1 tim", "1 ti", "i timothy"]),
    ("2 Timothy", &["2tim", "2ti", "2 tim", "2 ti", "ii timothy"]),
    ("Titus", &["tit", "ti"]),
    ("Philemon", &["phlm", "phm", "philem"]),
    ("Hebrews", &["heb"]),
    ("James", &["jas", "jm"]),
    ("1 Peter", &["1pet", "1pe", "1 pet", "1 pe", "i peter"]),
    ("2 Peter", &["2pet", "2pe", "2 pet", "2 pe", "ii peter"]),
    ("1 John", &["1jn", "1jo", "1 jn", "1 jo", "1john", "i john"]),
    ("2 John", &["2jn", "2jo", "2 jn", "2 jo", "2john", "ii john"]),
    ("3 John", &["3jn", "3jo", "3 jn", "3 jo", "3john", "iii john"]),
    ("Jude", &["jud", "jd"]),
    ("Revelation", &["rev", "re", "revelations", "apocalypse"]),
];

/// Index into [`BOOKS`] for a typed book name, accepting full names, abbreviations and
/// unambiguous prefixes ("matt", "1 cor", "Phil").
pub fn find_book(name: &str) -> Option<u8> {
    let n = normalize(name);
    if n.is_empty() {
        return None;
    }
    for (i, (full, abbrevs)) in BOOKS.iter().enumerate() {
        if normalize(full) == n || abbrevs.iter().any(|a| normalize(a) == n) {
            return u8::try_from(i).ok();
        }
    }
    let matches: Vec<usize> = BOOKS.iter().enumerate().filter(|(_, (full, _))| normalize(full).starts_with(&n)).map(|(i, _)| i).collect();
    match matches.as_slice() {
        [one] => u8::try_from(*one).ok(),
        _ => None,
    }
}

fn normalize(s: &str) -> String {
    s.to_lowercase().replace('.', "").split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn book_name(book: u8) -> &'static str {
    BOOKS.get(usize::from(book)).map(|(n, _)| *n).unwrap_or("?")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reference {
    pub book: u8,
    pub chapter: u16,
    /// Inclusive verse range; `None` means the whole chapter.
    pub verses: Option<(u16, u16)>,
}

impl Reference {
    pub fn label(&self) -> String {
        match self.verses {
            None => format!("{} {}", book_name(self.book), self.chapter),
            Some((a, b)) if a == b => format!("{} {}:{}", book_name(self.book), self.chapter, a),
            Some((a, b)) => format!("{} {}:{}-{}", book_name(self.book), self.chapter, a, b),
        }
    }
}

pub fn parse_reference(input: &str) -> Option<Reference> {
    let s = input.trim();
    // The book is everything before the last token that starts with a digit (the "3:16" part).
    let split = s.rfind(|c: char| c.is_whitespace())?;
    let (book_part, nums) = (s[..split].trim(), s[split..].trim());
    let book = find_book(book_part)?;
    let (chapter, verses) = match nums.split_once([':', '.']) {
        Some((c, v)) => {
            let chapter: u16 = c.trim().parse().ok()?;
            let (a, b) = match v.split_once(['-', '–']) {
                Some((a, b)) => (a.trim().parse().ok()?, b.trim().parse().ok()?),
                None => {
                    let a: u16 = v.trim().parse().ok()?;
                    (a, a)
                }
            };
            (chapter, Some((a.min(b), a.max(b))))
        }
        None => (nums.parse().ok()?, None),
    };
    (chapter > 0).then_some(Reference { book, chapter, verses })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Verse {
    pub book: u8,
    pub chapter: u16,
    pub verse: u16,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bible {
    pub name: String,
    pub abbreviation: String,
    #[serde(default)]
    pub copyright: String,
    pub verses: Vec<Verse>,
}

impl Bible {
    pub fn passage(&self, r: &Reference) -> Vec<&Verse> {
        self.verses
            .iter()
            .filter(|v| v.book == r.book && v.chapter == r.chapter && r.verses.is_none_or(|(a, b)| (a..=b).contains(&v.verse)))
            .collect()
    }

    /// Verses whose text contains every word of `query`, for the search box.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Verse> {
        let words: Vec<String> = query.to_lowercase().split_whitespace().map(str::to_string).collect();
        if words.is_empty() {
            return Vec::new();
        }
        self.verses
            .iter()
            .filter(|v| {
                let t = v.text.to_lowercase();
                words.iter().all(|w| t.contains(w.as_str()))
            })
            .take(limit)
            .collect()
    }

    /// Import from text, one verse per line, as either `Book<TAB>Chapter<TAB>Verse<TAB>Text` or
    /// `Book Chapter:Verse Text`. Returns the translation and how many lines were skipped.
    pub fn import(name: &str, abbreviation: &str, text: &str) -> (Bible, usize) {
        let mut verses = Vec::new();
        let mut skipped = 0;
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            match parse_line(line) {
                Some(v) => verses.push(v),
                None => skipped += 1,
            }
        }
        (Bible { name: name.into(), abbreviation: abbreviation.into(), copyright: String::new(), verses }, skipped)
    }
}

fn parse_line(line: &str) -> Option<Verse> {
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() >= 4 {
        return Some(Verse {
            book: find_book(cols[0])?,
            chapter: cols[1].trim().parse().ok()?,
            verse: cols[2].trim().parse().ok()?,
            text: cols[3..].join(" ").trim().to_string(),
        });
    }
    // "Book Chapter:Verse Text": find the first token shaped like N:N after the book name.
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let at = tokens.iter().position(|t| t.split_once(':').is_some_and(|(a, b)| a.parse::<u16>().is_ok() && b.parse::<u16>().is_ok()))?;
    let book = find_book(&tokens[..at].join(" "))?;
    let (c, v) = tokens[at].split_once(':')?;
    Some(Verse { book, chapter: c.parse().ok()?, verse: v.parse().ok()?, text: tokens[at + 1..].join(" ") })
}

/// Public-domain King James Version passages bundled so scripture works before any import.
pub fn sample() -> Bible {
    let text = "\
Psalms 23:1 The LORD is my shepherd; I shall not want.
Psalms 23:2 He maketh me to lie down in green pastures: he leadeth me beside the still waters.
Psalms 23:3 He restoreth my soul: he leadeth me in the paths of righteousness for his name's sake.
Psalms 23:4 Yea, though I walk through the valley of the shadow of death, I will fear no evil: for thou art with me; thy rod and thy staff they comfort me.
Psalms 23:5 Thou preparest a table before me in the presence of mine enemies: thou anointest my head with oil; my cup runneth over.
Psalms 23:6 Surely goodness and mercy shall follow me all the days of my life: and I will dwell in the house of the LORD for ever.
John 3:16 For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.
John 3:17 For God sent not his Son into the world to condemn the world; but that the world through him might be saved.
1 Corinthians 13:4 Charity suffereth long, and is kind; charity envieth not; charity vaunteth not itself, is not puffed up,
1 Corinthians 13:5 Doth not behave itself unseemly, seeketh not her own, is not easily provoked, thinketh no evil;
1 Corinthians 13:6 Rejoiceth not in iniquity, but rejoiceth in the truth;
1 Corinthians 13:7 Beareth all things, believeth all things, hopeth all things, endureth all things.
1 Corinthians 13:8 Charity never faileth: but whether there be prophecies, they shall fail; whether there be tongues, they shall cease; whether there be knowledge, it shall vanish away.
Romans 8:28 And we know that all things work together for good to them that love God, to them who are the called according to his purpose.
Philippians 4:13 I can do all things through Christ which strengtheneth me.";
    let (mut b, _) = Bible::import("King James Version (sample)", "KJV", text);
    b.copyright = "Public domain".into();
    b
}

/// The folder translations are stored in, next to the library file.
pub fn bibles_dir(library: Option<&Path>) -> Option<PathBuf> {
    library.and_then(Path::parent).map(|d| d.join("bibles"))
}

/// Every saved translation, plus the bundled sample when no full KJV is installed.
pub fn load_all(dir: Option<&Path>) -> (Vec<Bible>, Vec<String>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    if let Some(dir) = dir
        && let Ok(entries) = std::fs::read_dir(dir)
    {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "json") {
                match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<Bible>(&b).map_err(|e| e.to_string())) {
                    Ok(b) => out.push(b),
                    Err(e) => errors.push(format!("{} could not be read: {e}", p.display())),
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    if !out.iter().any(|b| b.abbreviation.eq_ignore_ascii_case("KJV")) {
        out.push(sample());
    }
    (out, errors)
}

pub fn save(dir: &Path, bible: &Bible) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let safe: String = bible.abbreviation.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let file = dir.join(format!("{}.json", if safe.is_empty() { "bible" } else { &safe }));
    let bytes = serde_json::to_vec(bible).map_err(std::io::Error::other)?;
    craft_atomic::atomic_write(&file, &bytes)
}

/// Build a presentation for a passage: `per_slide` verses on each slide, each slide carrying the
/// passage reference in a smaller box along the bottom.
pub fn presentation(
    lib: &mut Library,
    bible: &Bible,
    r: &Reference,
    per_slide: usize,
    theme: Option<Id>,
    show_numbers: bool,
) -> Option<Presentation> {
    let verses = bible.passage(r);
    if verses.is_empty() {
        return None;
    }
    let theme_value = theme.and_then(|t| lib.theme(t)).cloned();
    let mut slides = Vec::new();
    for chunk in verses.chunks(per_slide.max(1)) {
        let text = chunk.iter().map(|v| if show_numbers { format!("{} {}", v.verse, v.text) } else { v.text.clone() }).collect::<Vec<_>>().join(" ");
        let first = chunk.first().map(|v| v.verse).unwrap_or(0);
        let last = chunk.last().map(|v| v.verse).unwrap_or(first);
        let label = Reference { verses: Some((first, last)), ..*r }.label();
        let mut body = lib.text_element(&text, theme_value.as_ref());
        body.frame = Frame { x: 140.0, y: 100.0, w: 1640.0, h: 780.0 };
        if let ElementKind::Text { style, .. } = &mut body.kind {
            style.size = style.size.min(68.0);
            style.uppercase = false;
        }
        let reference = Element {
            id: lib.next_id(),
            name: "Reference".into(),
            frame: Frame { x: 140.0, y: 900.0, w: 1640.0, h: 110.0 },
            kind: ElementKind::Text {
                text: format!("{label} {}", bible.abbreviation),
                style: TextStyle { size: 44.0, h_align: HAlign::Right, v_align: VAlign::Middle, ..TextStyle::default() },
            },
            fill: None,
        };
        slides.push(Slide {
            id: lib.next_id(),
            label,
            elements: vec![body, reference],
            background: theme_value.as_ref().and_then(|t| t.background),
            media_action: None,
            audio_action: None,
            notes: String::new(),
            enabled: true,
        });
    }
    let gid = lib.next_id();
    Some(Presentation {
        id: lib.next_id(),
        name: r.label(),
        category: "Scripture".into(),
        groups: vec![Group { id: gid, name: r.label(), color: Rgba([180, 140, 60, 255]), slides }],
        arrangements: Vec::new(),
        active_arrangement: None,
        theme,
        ccli: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_parse_in_common_shapes() {
        let john = find_book("John").map(usize::from);
        assert_eq!(john, BOOKS.iter().position(|(n, _)| *n == "John"));
        assert_eq!(parse_reference("John 3:16").map(|r| r.label()), Some("John 3:16".into()));
        assert_eq!(parse_reference("jn 3:16-18").map(|r| r.label()), Some("John 3:16-18".into()));
        assert_eq!(parse_reference("1 Cor 13").map(|r| r.label()), Some("1 Corinthians 13".into()));
        assert_eq!(parse_reference("Ps. 23:4").map(|r| r.label()), Some("Psalms 23:4".into()));
        assert_eq!(parse_reference("Phil 4:13").map(|r| r.label()), Some("Philippians 4:13".into()));
        assert_eq!(parse_reference("Nonsense 1:1"), None);
        assert_eq!(parse_reference("John"), None);
    }

    #[test]
    fn ambiguous_prefixes_are_rejected() {
        // "J" could be Joshua, Judges, Job, John...
        assert_eq!(find_book("j"), None);
        assert!(find_book("Rev").is_some());
    }

    #[test]
    fn import_reads_both_formats_and_counts_bad_lines() {
        let text = "Genesis\t1\t1\tIn the beginning\nGenesis 1:2 And the earth\nnot a verse\n# comment";
        let (b, skipped) = Bible::import("Test", "TST", text);
        assert_eq!(b.verses.len(), 2);
        assert_eq!(skipped, 1);
        assert_eq!(b.verses[1].text, "And the earth");
    }

    #[test]
    fn passage_becomes_slides_with_references() {
        let mut lib = Library::default();
        let bible = sample();
        let r = parse_reference("Psalm 23:1-6").unwrap_or(Reference { book: 0, chapter: 0, verses: None });
        let p = presentation(&mut lib, &bible, &r, 2, None, true);
        let p = p.unwrap_or_else(|| lib.presentation_from_lyrics("x", "", None));
        assert_eq!(p.slide_count(), 3);
        assert!(p.groups[0].slides[0].text().starts_with("1 The LORD is my shepherd"));
        assert!(p.groups[0].slides[2].text().contains("Psalms 23:5-6 KJV"));
    }

    #[test]
    fn search_matches_all_words() {
        let b = sample();
        assert_eq!(b.search("shepherd want", 10).len(), 1);
        assert!(b.search("", 10).is_empty());
    }
}
