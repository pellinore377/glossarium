//! IPA chart layout data and aesthetic presets.
//!
//! The consonant grid mirrors the official 2005 pulmonic chart: 11 places
//! across, 8 manners down, with the dental–alveolar–postalveolar span
//! merged where the chart merges it, and shaded cells for articulations
//! judged impossible. Where symbols appear in pairs, left = voiceless,
//! right = voiced; a lone symbol carries `None` for its missing partner.

pub const PLACES: [&str; 11] = [
    "Bilabial",
    "Labiodental",
    "Dental",
    "Alveolar",
    "Postalveolar",
    "Retroflex",
    "Palatal",
    "Velar",
    "Uvular",
    "Pharyngeal",
    "Glottal",
];

pub enum Cell {
    /// A selectable cell: up to two symbols (voiceless, voiced).
    Sounds {
        span: u8,
        vl: Option<&'static str>,
        vd: Option<&'static str>,
    },
    /// Articulation judged impossible — rendered hatched, not clickable.
    Shaded { span: u8 },
    /// Possible but no dedicated IPA symbol — rendered blank.
    Empty { span: u8 },
}

pub struct MannerRow {
    pub name: &'static str,
    pub cells: &'static [Cell],
}

use Cell::{Empty, Shaded, Sounds};

/// Every row's spans sum to 11.
pub const CONSONANT_ROWS: &[MannerRow] = &[
    MannerRow {
        name: "Plosive",
        cells: &[
            Sounds { span: 1, vl: Some("p"), vd: Some("b") },
            Empty { span: 1 },
            Sounds { span: 3, vl: Some("t"), vd: Some("d") },
            Sounds { span: 1, vl: Some("ʈ"), vd: Some("ɖ") },
            Sounds { span: 1, vl: Some("c"), vd: Some("ɟ") },
            Sounds { span: 1, vl: Some("k"), vd: Some("ɡ") },
            Sounds { span: 1, vl: Some("q"), vd: Some("ɢ") },
            Shaded { span: 1 },
            Sounds { span: 1, vl: Some("ʔ"), vd: None },
        ],
    },
    MannerRow {
        name: "Nasal",
        cells: &[
            Sounds { span: 1, vl: None, vd: Some("m") },
            Sounds { span: 1, vl: None, vd: Some("ɱ") },
            Sounds { span: 3, vl: None, vd: Some("n") },
            Sounds { span: 1, vl: None, vd: Some("ɳ") },
            Sounds { span: 1, vl: None, vd: Some("ɲ") },
            Sounds { span: 1, vl: None, vd: Some("ŋ") },
            Sounds { span: 1, vl: None, vd: Some("ɴ") },
            Shaded { span: 1 },
            Shaded { span: 1 },
        ],
    },
    MannerRow {
        name: "Trill",
        cells: &[
            Sounds { span: 1, vl: None, vd: Some("ʙ") },
            Empty { span: 1 },
            Sounds { span: 3, vl: None, vd: Some("r") },
            Empty { span: 1 },
            Empty { span: 1 },
            Shaded { span: 1 },
            Sounds { span: 1, vl: None, vd: Some("ʀ") },
            Empty { span: 1 },
            Shaded { span: 1 },
        ],
    },
    MannerRow {
        name: "Tap or flap",
        cells: &[
            Empty { span: 1 },
            Sounds { span: 1, vl: None, vd: Some("ⱱ") },
            Sounds { span: 3, vl: None, vd: Some("ɾ") },
            Sounds { span: 1, vl: None, vd: Some("ɽ") },
            Empty { span: 1 },
            Shaded { span: 1 },
            Empty { span: 1 },
            Empty { span: 1 },
            Shaded { span: 1 },
        ],
    },
    MannerRow {
        name: "Fricative",
        cells: &[
            Sounds { span: 1, vl: Some("ɸ"), vd: Some("β") },
            Sounds { span: 1, vl: Some("f"), vd: Some("v") },
            Sounds { span: 1, vl: Some("θ"), vd: Some("ð") },
            Sounds { span: 1, vl: Some("s"), vd: Some("z") },
            Sounds { span: 1, vl: Some("ʃ"), vd: Some("ʒ") },
            Sounds { span: 1, vl: Some("ʂ"), vd: Some("ʐ") },
            Sounds { span: 1, vl: Some("ç"), vd: Some("ʝ") },
            Sounds { span: 1, vl: Some("x"), vd: Some("ɣ") },
            Sounds { span: 1, vl: Some("χ"), vd: Some("ʁ") },
            Sounds { span: 1, vl: Some("ħ"), vd: Some("ʕ") },
            Sounds { span: 1, vl: Some("h"), vd: Some("ɦ") },
        ],
    },
    MannerRow {
        name: "Lateral fricative",
        cells: &[
            Shaded { span: 1 },
            Shaded { span: 1 },
            Sounds { span: 3, vl: Some("ɬ"), vd: Some("ɮ") },
            Empty { span: 1 },
            Empty { span: 1 },
            Empty { span: 1 },
            Empty { span: 1 },
            Shaded { span: 1 },
            Shaded { span: 1 },
        ],
    },
    MannerRow {
        name: "Approximant",
        cells: &[
            Empty { span: 1 },
            Sounds { span: 1, vl: None, vd: Some("ʋ") },
            Sounds { span: 3, vl: None, vd: Some("ɹ") },
            Sounds { span: 1, vl: None, vd: Some("ɻ") },
            Sounds { span: 1, vl: None, vd: Some("j") },
            Sounds { span: 1, vl: None, vd: Some("ɰ") },
            Empty { span: 1 },
            Empty { span: 1 },
            Shaded { span: 1 },
        ],
    },
    MannerRow {
        name: "Lateral approximant",
        cells: &[
            Shaded { span: 1 },
            Shaded { span: 1 },
            Sounds { span: 3, vl: None, vd: Some("l") },
            Sounds { span: 1, vl: None, vd: Some("ɭ") },
            Sounds { span: 1, vl: None, vd: Some("ʎ") },
            Sounds { span: 1, vl: None, vd: Some("ʟ") },
            Empty { span: 1 },
            Shaded { span: 1 },
            Shaded { span: 1 },
        ],
    },
];

/// Every selectable symbol on the chart — server-side validation set.
pub fn all_consonant_symbols() -> Vec<&'static str> {
    let mut out = Vec::new();
    for row in CONSONANT_ROWS {
        for cell in row.cells {
            if let Cell::Sounds { vl, vd, .. } = cell {
                if let Some(s) = vl {
                    out.push(*s);
                }
                if let Some(s) = vd {
                    out.push(*s);
                }
            }
        }
    }
    out
}

pub struct Aesthetic {
    pub id: &'static str,
    pub name: &'static str,
    pub blurb: &'static str,
    pub consonants: &'static [&'static str],
    pub vowels: &'static [&'static str],
}

/// Presets pre-fill the charts; any hand edit afterward silently detaches
/// the label (aesthetic becomes "custom"). "Custom" itself pre-fills
/// nothing and leaves existing selections untouched.
pub const AESTHETICS: &[Aesthetic] = &[
    Aesthetic {
        id: "melodic",
        name: "Melodic & flowing",
        blurb: "Open, sonorant-heavy, Oceanic in spirit. Few obstruents, \
                lots of vowel space, syllables that end open.",
        consonants: &["p", "t", "k", "ʔ", "m", "n", "ŋ", "l", "w", "j", "h"],
        vowels: &["i", "u", "e", "o", "a"],
    },
    Aesthetic {
        id: "guttural",
        name: "Stark & guttural",
        blurb: "Back-of-the-throat texture: uvulars, pharyngeals, a spare \
                vowel triangle. Reads carved rather than sung.",
        consonants: &[
            "t", "k", "q", "ʔ", "b", "d", "ɡ", "s", "ʃ", "χ", "ʁ", "ħ", "ʕ",
            "m", "n", "r", "l", "w", "j",
        ],
        vowels: &["i", "u", "a"],
    },
    Aesthetic {
        id: "crisp",
        name: "Crisp & clipped",
        blurb: "Small, tidy, symmetrical. Plain stops, one sibilant, clean \
                liquids — precise without being harsh.",
        consonants: &["p", "t", "k", "b", "d", "ɡ", "s", "h", "m", "n", "r", "l", "j"],
        vowels: &["i", "y", "u", "e", "ø", "o", "a"],
    },
    Aesthetic {
        id: "sibilant",
        name: "Hushed & sibilant",
        blurb: "Fricative-rich and whispering: paired sibilants, voiced \
                fricatives, a soft rustle through every word.",
        consonants: &[
            "p", "b", "t", "d", "k", "ɡ", "f", "v", "s", "z", "ʃ", "ʒ", "x",
            "m", "n", "r", "l", "j",
        ],
        vowels: &["i", "e", "a", "o", "u"],
    },
];

pub fn aesthetic_by_id(id: &str) -> Option<&'static Aesthetic> {
    AESTHETICS.iter().find(|a| a.id == id)
}

// ---------- Vowels ----------

/// A point on the vowel quadrilateral. Coordinates are percentages of the
/// chart area; where two symbols share a point, left = unrounded,
/// right = rounded, per IPA convention.
pub struct VowelPoint {
    pub x: f32,
    pub y: f32,
    pub unrounded: Option<&'static str>,
    pub rounded: Option<&'static str>,
}

pub const VOWEL_POINTS: &[VowelPoint] = &[
    // Close
    VowelPoint { x: 8.0, y: 7.0, unrounded: Some("i"), rounded: Some("y") },
    VowelPoint { x: 50.0, y: 7.0, unrounded: Some("ɨ"), rounded: Some("ʉ") },
    VowelPoint { x: 92.0, y: 7.0, unrounded: Some("ɯ"), rounded: Some("u") },
    // Near-close
    VowelPoint { x: 26.0, y: 20.5, unrounded: Some("ɪ"), rounded: Some("ʏ") },
    VowelPoint { x: 80.0, y: 20.5, unrounded: None, rounded: Some("ʊ") },
    // Close-mid
    VowelPoint { x: 19.0, y: 34.0, unrounded: Some("e"), rounded: Some("ø") },
    VowelPoint { x: 55.5, y: 34.0, unrounded: Some("ɘ"), rounded: Some("ɵ") },
    VowelPoint { x: 92.0, y: 34.0, unrounded: Some("ɤ"), rounded: Some("o") },
    // Mid
    VowelPoint { x: 58.0, y: 47.5, unrounded: Some("ə"), rounded: None },
    // Open-mid
    VowelPoint { x: 29.0, y: 61.0, unrounded: Some("ɛ"), rounded: Some("œ") },
    VowelPoint { x: 60.5, y: 61.0, unrounded: Some("ɜ"), rounded: Some("ɞ") },
    VowelPoint { x: 92.0, y: 61.0, unrounded: Some("ʌ"), rounded: Some("ɔ") },
    // Near-open
    VowelPoint { x: 36.0, y: 74.5, unrounded: Some("æ"), rounded: None },
    VowelPoint { x: 63.0, y: 74.5, unrounded: Some("ɐ"), rounded: None },
    // Open
    VowelPoint { x: 38.0, y: 88.0, unrounded: Some("a"), rounded: Some("ɶ") },
    VowelPoint { x: 92.0, y: 88.0, unrounded: Some("ɑ"), rounded: Some("ɒ") },
];

pub fn all_vowel_symbols() -> Vec<&'static str> {
    let mut out = Vec::new();
    for p in VOWEL_POINTS {
        if let Some(s) = p.unrounded {
            out.push(s);
        }
        if let Some(s) = p.rounded {
            out.push(s);
        }
    }
    out
}

/// Chart-order index for stable sorting of selected vowels in the
/// diphthong grid; unknown symbols sort last.
pub fn vowel_order(sym: &str) -> usize {
    all_vowel_symbols()
        .iter()
        .position(|s| *s == sym)
        .unwrap_or(usize::MAX)
}

