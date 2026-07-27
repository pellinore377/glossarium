//! Deterministic proto-root generator.
//!
//! Given a phoneme inventory and syllable constraints, produce word forms
//! that (a) are reproducible from a seed — same language, same seed, same
//! lexicon, forever — and (b) sound like they belong to one language
//! rather than a random symbol soup. Two mechanisms carry (b): phonemes
//! are sampled by cross-linguistic frequency weights rather than
//! uniformly, and clusters must ramp sonority toward the nucleus (with
//! the classic sibilant-plus-stop exception, because /st-/ is too good to
//! ban).
//!
//! No `rand` dependency: a hand-rolled splitmix64 keeps the byte-for-byte
//! output of a seed independent of any crate's version bump.

use std::collections::HashSet;
use std::fmt;

// ---------- RNG ----------

/// splitmix64: tiny, fast, and — critically — ours, so a `cargo update`
/// can never silently reshuffle every proto-language on the server.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, n). Modulo bias is irrelevant at these ranges.
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }

    fn pick_weighted<'a, T>(&mut self, items: &'a [(T, u32)]) -> &'a T {
        let total: u64 = items.iter().map(|(_, w)| u64::from(*w)).sum();
        let mut r = self.below(total.max(1));
        for (item, w) in items {
            let w = u64::from(*w);
            if r < w {
                return item;
            }
            r -= w;
        }
        &items[items.len() - 1].0
    }
}

// ---------- Frequency weights ----------

/// Rough cross-linguistic phoneme frequencies (UPSID-flavored): /t n m k/
/// are everywhere, /ɢ ʙ ɶ/ are prized rarities. Anything unlisted gets
/// DEFAULT_WEIGHT — present but uncommon, which is exactly right for the
/// exotic corners of the chart.
const CONSONANT_WEIGHTS: &[(&str, u32)] = &[
    ("t", 97), ("n", 96), ("m", 95), ("k", 90), ("s", 86), ("j", 84),
    ("p", 82), ("w", 80), ("l", 76), ("r", 72), ("b", 70), ("h", 64),
    ("d", 64), ("ɡ", 62), ("ŋ", 52), ("f", 48), ("ʔ", 42), ("ʃ", 40),
    ("ɾ", 38), ("tʃ", 36), ("ɲ", 34), ("x", 30), ("z", 30), ("v", 28), ("ts", 26),
    ("dʒ", 24), ("ʒ", 20), ("c", 18), ("ɟ", 16), ("q", 14), ("dz", 10), ("ð", 10),
    ("θ", 10), ("β", 10), ("ɣ", 14), ("χ", 10), ("ħ", 8), ("ʂ", 12),
    ("ʐ", 8), ("ɳ", 10), ("ʈ", 10), ("ɖ", 8),
];

const VOWEL_WEIGHTS: &[(&str, u32)] = &[
    ("a", 98), ("i", 92), ("u", 88), ("e", 74), ("o", 74), ("ə", 48),
    ("ɛ", 42), ("ɔ", 42), ("æ", 30), ("ɑ", 30), ("ɪ", 30), ("ʊ", 26),
    ("ɨ", 22), ("ʌ", 20), ("ɯ", 18), ("y", 14), ("ø", 12), ("œ", 10),
];

const DEFAULT_WEIGHT: u32 = 12;
const DIPHTHONG_WEIGHT: u32 = 14;

fn weight_of(sym: &str, table: &[(&str, u32)]) -> u32 {
    table
        .iter()
        .find(|(s, _)| *s == sym)
        .map(|(_, w)| *w)
        .unwrap_or(DEFAULT_WEIGHT)
}

// ---------- Sonority ----------

/// 1 = plosive … 5 = glide/approximant. Onsets must climb toward the
/// nucleus, codas must descend from it.
fn sonority(sym: &str) -> u8 {
    const NASALS: &[&str] = &["m", "ɱ", "n", "ɳ", "ɲ", "ŋ", "ɴ"];
    const LIQUIDS: &[&str] = &[
        "l", "ɭ", "ʎ", "ʟ", "r", "ʀ", "ʙ", "ɾ", "ɽ", "ⱱ", "ɬ", "ɮ",
    ];
    const GLIDES: &[&str] = &["j", "ɰ", "w", "ʍ", "ɥ", "ʋ", "ɹ", "ɻ"];
    const FRICATIVES: &[&str] = &[
        "ɸ", "β", "f", "v", "θ", "ð", "s", "z", "ʃ", "ʒ", "ʂ", "ʐ", "ç",
        "ʝ", "x", "ɣ", "χ", "ʁ", "ħ", "ʕ", "h", "ɦ",
    ];
    if GLIDES.contains(&sym) {
        5
    } else if LIQUIDS.contains(&sym) {
        4
    } else if NASALS.contains(&sym) {
        3
    } else if FRICATIVES.contains(&sym) {
        2
    } else {
        1 // plosives, and a safe default for anything unclassified
    }
}

fn is_sibilant(sym: &str) -> bool {
    matches!(sym, "s" | "z" | "ʃ" | "ʒ" | "ʂ" | "ʐ")
}

// ---------- Generator ----------

#[derive(Debug, Clone)]
pub struct WordSpec {
    pub consonants: Vec<String>,
    pub vowels: Vec<String>,
    pub diphthongs: Vec<String>,
    pub onset_min: u8,
    pub onset_max: u8,
    pub coda_min: u8,
    pub coda_max: u8,
    /// Explicitly allowed two-consonant sequences ("pr", "st") for onsets
    /// and codas. `None` = fall back to the sonority heuristic; `Some`
    /// (even empty) = the user has curated the list, obey it exactly.
    pub onset_pairs: Option<Vec<String>>,
    pub coda_pairs: Option<Vec<String>>,
    /// Which single consonants may appear in each margin at all
    /// (many languages allow only nasals in codas, ban ŋ initially…).
    /// `None` = every consonant.
    pub onset_singles: Option<Vec<String>>,
    pub coda_singles: Option<Vec<String>>,
    /// Allowed coda+onset sequences across a syllable boundary ("nt",
    /// "sp"). `None` = the default heuristic: anything except geminates
    /// and voicing-mismatched obstruent pairs (no more "lidtep").
    pub medial_pairs: Option<Vec<String>>,
    pub seed: u64,
}

/// Do two segments clash in obstruent voicing (like /d/+/t/)? Checked
/// against the universal feature table.
fn obstruent_voicing_clash(a: &str, b: &str) -> bool {
    let seg = |s: &str| phon::universal_inventory().iter().find(|x| x.ipa == s);
    match (seg(a), seg(b)) {
        (Some(x), Some(y)) => {
            use phon::{Feature::*, FeatureValue::*};
            x.get(Sonorant) == Minus && y.get(Sonorant) == Minus && x.get(Voice) != y.get(Voice)
        }
        _ => false,
    }
}

/// Default cross-syllable junction list for the wizard: every coda ×
/// onset pair except geminates and obstruent voicing clashes.
pub fn default_medial_pairs(codas: &[String], onsets: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for a in codas {
        for b in onsets {
            if a != b && !obstruent_voicing_clash(a, b) {
                out.push(format!("{a}{b}"));
            }
        }
    }
    out
}

/// The sonority-derived default cluster list the wizard starts from:
/// rising toward the nucleus for onsets (with the sibilant+stop
/// exception), falling for codas. Pairs, not triples — longer clusters
/// are chains of allowed pairs.
pub fn default_pairs(consonants: &[String], rising: bool) -> Vec<String> {
    let mut out = Vec::new();
    for a in consonants {
        for b in consonants {
            if a == b {
                continue;
            }
            let ok = if rising {
                sonority(b) > sonority(a) || (is_sibilant(a) && sonority(b) == 1)
            } else {
                sonority(b) < sonority(a)
            };
            if ok && !obstruent_voicing_clash(a, b) {
                out.push(format!("{a}{b}"));
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenError {
    /// No vowels or diphthongs — nothing can be a nucleus.
    NoNuclei,
    /// The template demands consonants the inventory doesn't have.
    NoConsonants,
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenError::NoNuclei => {
                write!(f, "the inventory has no vowels to build syllables around")
            }
            GenError::NoConsonants => write!(
                f,
                "the syllable template requires consonants, but none are selected"
            ),
        }
    }
}

impl std::error::Error for GenError {}

pub struct Generator {
    consonants: Vec<(String, u32)>,
    onset_pool: Vec<(String, u32)>,
    coda_pool: Vec<(String, u32)>,
    nuclei: Vec<(String, u32)>,
    onset_min: u8,
    onset_max: u8,
    coda_min: u8,
    coda_max: u8,
    onset_pairs: Option<HashSet<String>>,
    coda_pairs: Option<HashSet<String>>,
    medial_pairs: Option<HashSet<String>>,
    rng: Rng,
    used: HashSet<String>,
}

/// Preference for margin length within the allowed range. Onsets like to
/// exist (most syllables on Earth start with a consonant); codas like to
/// not. Index = cluster length.
const ONSET_LEN_WEIGHTS: [u32; 4] = [3, 10, 3, 1];
const CODA_LEN_WEIGHTS: [u32; 4] = [10, 5, 2, 1];

/// Root length in syllables: disyllables dominate, monosyllables are
/// common, trisyllables spice.
const SYLLABLE_COUNT_WEIGHTS: [(usize, u32); 3] = [(1, 30), (2, 50), (3, 20)];

impl Generator {
    pub fn new(spec: WordSpec) -> Result<Self, GenError> {
        let mut nuclei: Vec<(String, u32)> = spec
            .vowels
            .iter()
            .map(|v| (v.clone(), weight_of(v, VOWEL_WEIGHTS)))
            .collect();
        nuclei.extend(
            spec.diphthongs
                .iter()
                .map(|d| (d.clone(), DIPHTHONG_WEIGHT)),
        );
        if nuclei.is_empty() {
            return Err(GenError::NoNuclei);
        }
        let consonants: Vec<(String, u32)> = spec
            .consonants
            .iter()
            .map(|c| (c.clone(), weight_of(c, CONSONANT_WEIGHTS)))
            .collect();
        if consonants.is_empty() && (spec.onset_min > 0 || spec.coda_min > 0) {
            return Err(GenError::NoConsonants);
        }
        let filter_pool = |allow: &Option<Vec<String>>| -> Vec<(String, u32)> {
            match allow {
                None => consonants.clone(),
                Some(list) => consonants
                    .iter()
                    .filter(|(s, _)| list.contains(s))
                    .cloned()
                    .collect(),
            }
        };
        let onset_pool = filter_pool(&spec.onset_singles);
        let coda_pool = filter_pool(&spec.coda_singles);
        Ok(Self {
            consonants,
            onset_pool,
            coda_pool,
            nuclei,
            onset_min: spec.onset_min,
            onset_max: spec.onset_max,
            coda_min: spec.coda_min,
            coda_max: spec.coda_max,
            onset_pairs: spec.onset_pairs.map(|v| v.into_iter().collect()),
            coda_pairs: spec.coda_pairs.map(|v| v.into_iter().collect()),
            medial_pairs: spec.medial_pairs.map(|v| v.into_iter().collect()),
            rng: Rng(spec.seed),
            used: HashSet::new(),
        })
    }

    fn is_consonant(&self, s: &str) -> bool {
        self.consonants.iter().any(|(c, _)| c == s)
    }

    /// May `coda` and `onset` touch across a syllable boundary?
    fn medial_ok(&self, coda: &str, onset: &str) -> bool {
        match &self.medial_pairs {
            Some(set) => set.contains(&format!("{coda}{onset}")),
            None => coda != onset && !obstruent_voicing_clash(coda, onset),
        }
    }


    /// Is `prev` + `next` a legal cluster sequence? A curated pair list
    /// wins outright; otherwise the sonority heuristic decides.
    fn pair_ok(&self, prev: &str, next: &str, rising: bool) -> bool {
        let explicit = if rising { &self.onset_pairs } else { &self.coda_pairs };
        if let Some(set) = explicit {
            return set.contains(&format!("{prev}{next}"));
        }
        let sonority_ok = if rising {
            sonority(next) > sonority(prev) || (is_sibilant(prev) && sonority(next) == 1)
        } else {
            sonority(next) < sonority(prev)
        };
        sonority_ok && !obstruent_voicing_clash(prev, next)
    }

    fn margin_len(&mut self, min: u8, max: u8, weights: &[u32; 4], rising: bool) -> usize {
        let pool_empty = if rising {
            self.onset_pool.is_empty()
        } else {
            self.coda_pool.is_empty()
        };
        if pool_empty {
            return 0;
        }
        let choices: Vec<(usize, u32)> = (min..=max.min(3))
            .map(|l| (l as usize, weights[l as usize]))
            .collect();
        if choices.is_empty() {
            return min as usize;
        }
        *self.rng.pick_weighted(&choices)
    }

    /// One consonant obeying the cluster constraint against `prev`
    /// (curated pair list, or sonority when none). A few rejected
    /// samples and we give up and end the cluster early — a shorter
    /// cluster is always legal.
    fn cluster_next(&mut self, prev: Option<&str>, rising: bool) -> Option<String> {
        for _ in 0..12 {
            let c = if rising {
                self.rng.pick_weighted(&self.onset_pool).clone()
            } else {
                self.rng.pick_weighted(&self.coda_pool).clone()
            };
            match prev {
                None => return Some(c),
                Some(p) => {
                    if p == c {
                        continue; // no geminates inside a cluster
                    }
                    if self.pair_ok(p, &c, rising) {
                        return Some(c);
                    }
                }
            }
        }
        None
    }

    fn cluster(&mut self, len: usize, rising: bool) -> Vec<String> {
        let mut out: Vec<String> = Vec::with_capacity(len);
        for _ in 0..len {
            match self.cluster_next(out.last().map(String::as_str), rising) {
                Some(c) => out.push(c),
                None => break,
            }
        }
        out
    }

    /// One word form, not yet checked for uniqueness.
    fn raw_word(&mut self, syllables: usize) -> String {
        let mut segs: Vec<String> = Vec::new();
        for i in 0..syllables {
            let onset_len =
                self.margin_len(self.onset_min, self.onset_max, &ONSET_LEN_WEIGHTS, true);
            let mut onset = self.cluster(onset_len, true);
            // Cross-syllable junction: if the previous syllable's coda
            // can't legally touch this onset, the coda retreats — words
            // like "lidtep" die here.
            if let Some(first) = onset.first().cloned() {
                while let Some(last) = segs.last().cloned() {
                    if self.is_consonant(&last) && !self.medial_ok(&last, &first) {
                        segs.pop();
                    } else {
                        break;
                    }
                }
            }
            // Respect a mandatory onset even if the cluster walk stalled.
            if onset.len() < self.onset_min as usize && !self.onset_pool.is_empty() {
                while onset.len() < self.onset_min as usize {
                    let c = self.rng.pick_weighted(&self.onset_pool).clone();
                    onset.push(c);
                }
            }
            segs.extend(onset);
            segs.push(self.rng.pick_weighted(&self.nuclei).clone());
            // Word-internal codas are rarer than word-final ones; skip
            // them half the time to keep medial clusters tasteful.
            let is_final = i + 1 == syllables;
            let coda_allowed = is_final || self.coda_min > 0 || self.rng.below(2) == 0;
            if coda_allowed {
                let coda_len =
                    self.margin_len(self.coda_min, self.coda_max, &CODA_LEN_WEIGHTS, false);
                let mut coda = self.cluster(coda_len, false);
                if coda.len() < self.coda_min as usize && !self.coda_pool.is_empty() {
                    while coda.len() < self.coda_min as usize {
                        let c = self.rng.pick_weighted(&self.coda_pool).clone();
                        coda.push(c);
                    }
                }
                segs.extend(coda);
            }
        }
        segs.concat()
    }

    /// A short (single-syllable) unique form — affix and particle
    /// material for the grammar generator.
    pub fn short_word(&mut self) -> String {
        for _ in 0..100 {
            let w = self.raw_word(1);
            if self.used.insert(w.clone()) {
                return w;
            }
        }
        let w = self.raw_word(2);
        self.used.insert(w.clone());
        w
    }

    /// Weighted choice over indices — exposed so grammar decisions share
    /// this generator's deterministic RNG stream.
    pub fn pick_index(&mut self, weights: &[u32]) -> usize {
        let items: Vec<(usize, u32)> = weights.iter().copied().enumerate().collect();
        *self.rng.pick_weighted(&items)
    }

    /// A word no previous call on this generator has returned.
    pub fn word(&mut self) -> String {
        for attempt in 0..200usize {
            // Once collisions start, push toward longer words for entropy.
            let syllables = if attempt < 50 {
                *self.rng.pick_weighted(&SYLLABLE_COUNT_WEIGHTS)
            } else {
                2 + (attempt / 50).min(2)
            };
            let w = self.raw_word(syllables);
            if self.used.insert(w.clone()) {
                return w;
            }
        }
        // Astronomically unlikely with any usable inventory; accept the
        // homophone rather than loop forever. Natural languages have
        // homophones too.
        let w = self.raw_word(3);
        self.used.insert(w.clone());
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn spec(seed: u64) -> WordSpec {
        WordSpec {
            consonants: s(&["p", "t", "k", "m", "n", "s", "l", "r", "j"]),
            vowels: s(&["a", "i", "u", "e", "o"]),
            diphthongs: s(&["ai", "au"]),
            onset_min: 0,
            onset_max: 2,
            coda_min: 0,
            coda_max: 1,
            onset_pairs: None,
            coda_pairs: None,
            onset_singles: None,
            coda_singles: None,
            medial_pairs: None,
            seed,
        }
    }

    #[test]
    fn no_voicing_clash_across_syllables() {
        let mut sp = spec(41);
        sp.consonants = s(&["p", "b", "t", "d", "k", "s", "m", "l"]);
        let mut g = Generator::new(sp).unwrap();
        let voiceless = "ptks";
        let voiced = "bd";
        for _ in 0..300 {
            let w = g.word();
            let chars: Vec<char> = w.chars().collect();
            for pair in chars.windows(2) {
                let clash = (voiced.contains(pair[0]) && voiceless.contains(pair[1]))
                    || (voiceless.contains(pair[0]) && voiced.contains(pair[1]));
                assert!(!clash, "voicing clash in {w}");
            }
        }
    }

    #[test]
    fn positional_pools_are_respected() {
        let mut sp = spec(31);
        // Only nasals may close a syllable; ŋ-style ban on onsets.
        sp.coda_singles = Some(s(&["m", "n"]));
        sp.onset_singles = Some(s(&["p", "t", "k", "s", "l", "r", "j", "m", "n"]));
        let mut g = Generator::new(sp).unwrap();
        for _ in 0..200 {
            let w = g.word();
            let last = w.chars().last().unwrap();
            assert!(
                "aiueomn".contains(last),
                "{w} ends in a non-nasal consonant despite coda pool"
            );
        }
    }

    #[test]
    fn default_pairs_follow_sonority() {
        let cons = s(&["p", "s", "r", "l"]);
        let rising = default_pairs(&cons, true);
        assert!(rising.contains(&"pr".to_string()), "stop+liquid onset");
        assert!(rising.contains(&"sp".to_string()), "sibilant+stop exception");
        assert!(!rising.contains(&"rp".to_string()), "falling onset rejected");
        let falling = default_pairs(&cons, false);
        assert!(falling.contains(&"rp".to_string()), "liquid+stop coda");
        assert!(!falling.contains(&"pr".to_string()), "rising coda rejected");
    }

    #[test]
    fn curated_pairs_are_obeyed() {
        let mut sp = spec(21);
        sp.onset_pairs = Some(vec!["pr".to_string()]);
        let mut g = Generator::new(sp).unwrap();
        let consonants = "ptkmnslrj";
        for _ in 0..200 {
            let w = g.word();
            let chars: Vec<char> = w.chars().collect();
            // Any consonant pair at the very start of a word is an onset
            // cluster and must be the one allowed pair.
            if chars.len() >= 2
                && consonants.contains(chars[0])
                && consonants.contains(chars[1])
            {
                assert_eq!(&w[..2], "pr", "illegal onset cluster in {w}");
            }
        }
    }

    #[test]
    fn same_seed_same_lexicon() {
        let words = |seed| {
            let mut g = Generator::new(spec(seed)).unwrap();
            (0..50).map(|_| g.word()).collect::<Vec<_>>()
        };
        assert_eq!(words(42), words(42));
        assert_ne!(words(42), words(43));
    }

    #[test]
    fn words_are_unique_and_from_inventory() {
        let mut g = Generator::new(spec(7)).unwrap();
        let mut seen = HashSet::new();
        for _ in 0..200 {
            let w = g.word();
            assert!(seen.insert(w.clone()), "duplicate {w}");
            for c in w.chars() {
                assert!(
                    "ptkmnslrjaiueo".contains(c),
                    "{w} contains out-of-inventory {c}"
                );
            }
        }
    }

    #[test]
    fn mandatory_onset_respected() {
        let mut sp = spec(9);
        sp.onset_min = 1;
        let mut g = Generator::new(sp).unwrap();
        for _ in 0..100 {
            let w = g.word();
            let first = w.chars().next().unwrap();
            assert!(
                "ptkmnslrj".contains(first),
                "{w} starts with a vowel despite onset_min=1"
            );
        }
    }

    #[test]
    fn cv_only_spec_yields_cv_words() {
        let mut sp = spec(11);
        sp.onset_max = 1;
        sp.coda_max = 0;
        let mut g = Generator::new(sp).unwrap();
        for _ in 0..100 {
            let w = g.word();
            assert!(
                w.chars().last().map(|c| "aiueo".contains(c)).unwrap(),
                "{w} ends in a consonant despite coda_max=0"
            );
        }
    }

    #[test]
    fn empty_inventory_is_an_error() {
        let mut sp = spec(1);
        sp.vowels.clear();
        sp.diphthongs.clear();
        assert_eq!(Generator::new(sp).unwrap_err(), GenError::NoNuclei);
    }
}
