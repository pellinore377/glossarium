//! Lexicon model.
//!
//! Lexemes hang off the *proto* language of a family; daughters carry only
//! derived reflexes and explicit overrides. Concepts are identified by
//! Concepticon-style IDs so the seed list, the IDS expansion (v1.5), and
//! colexification data all speak the same key space.

pub mod gen;
pub mod grammar;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pos {
    Noun,
    Verb,
    Adjective,
    Adverb,
    Pronoun,
    Numeral,
    Particle,
}

impl Pos {
    pub const ALL: &'static [Pos] = &[
        Pos::Noun,
        Pos::Verb,
        Pos::Adjective,
        Pos::Adverb,
        Pos::Pronoun,
        Pos::Numeral,
        Pos::Particle,
    ];

    /// Stable storage key — matches the serde snake_case rename.
    pub fn as_str(self) -> &'static str {
        match self {
            Pos::Noun => "noun",
            Pos::Verb => "verb",
            Pos::Adjective => "adjective",
            Pos::Adverb => "adverb",
            Pos::Pronoun => "pronoun",
            Pos::Numeral => "numeral",
            Pos::Particle => "particle",
        }
    }

    /// Dictionary-style abbreviation for display.
    pub fn abbrev(self) -> &'static str {
        match self {
            Pos::Noun => "n.",
            Pos::Verb => "v.",
            Pos::Adjective => "adj.",
            Pos::Adverb => "adv.",
            Pos::Pronoun => "pron.",
            Pos::Numeral => "num.",
            Pos::Particle => "part.",
        }
    }

    pub fn parse(s: &str) -> Option<Pos> {
        Pos::ALL.iter().copied().find(|p| p.as_str() == s)
    }
}

/// A meaning slot, independent of any language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    /// Concepticon concept-set ID where one exists; local IDs otherwise.
    pub concept_id: &'static str,
    pub gloss: &'static str,
    pub pos: Pos,
}

/// The Leipzig–Jakarta 100 core list, in published borrowability-rank
/// order: the v1 proto-lexicon seed. Chosen over Swadesh for its
/// empirical borrowing-resistance ranking — the right anchor for cognate
/// tracking across a family tree.
///
/// Glosses follow the published list; Concepticon IDs to be filled in when
/// the full concept tables land (data/, v1.5).
pub const LEIPZIG_JAKARTA_100: &[Concept] = &[
    Concept { concept_id: "LJ-001", gloss: "fire", pos: Pos::Noun },
    Concept { concept_id: "LJ-002", gloss: "nose", pos: Pos::Noun },
    Concept { concept_id: "LJ-003", gloss: "to go", pos: Pos::Verb },
    Concept { concept_id: "LJ-004", gloss: "water", pos: Pos::Noun },
    Concept { concept_id: "LJ-005", gloss: "mouth", pos: Pos::Noun },
    Concept { concept_id: "LJ-006", gloss: "tongue", pos: Pos::Noun },
    Concept { concept_id: "LJ-007", gloss: "blood", pos: Pos::Noun },
    Concept { concept_id: "LJ-008", gloss: "bone", pos: Pos::Noun },
    Concept { concept_id: "LJ-009", gloss: "you (sg)", pos: Pos::Pronoun },
    Concept { concept_id: "LJ-010", gloss: "root", pos: Pos::Noun },
    Concept { concept_id: "LJ-011", gloss: "to come", pos: Pos::Verb },
    Concept { concept_id: "LJ-012", gloss: "breast", pos: Pos::Noun },
    Concept { concept_id: "LJ-013", gloss: "rain", pos: Pos::Noun },
    Concept { concept_id: "LJ-014", gloss: "I", pos: Pos::Pronoun },
    Concept { concept_id: "LJ-015", gloss: "name", pos: Pos::Noun },
    Concept { concept_id: "LJ-016", gloss: "louse", pos: Pos::Noun },
    Concept { concept_id: "LJ-017", gloss: "wing", pos: Pos::Noun },
    Concept { concept_id: "LJ-018", gloss: "flesh, meat", pos: Pos::Noun },
    Concept { concept_id: "LJ-019", gloss: "arm, hand", pos: Pos::Noun },
    Concept { concept_id: "LJ-020", gloss: "fly (insect)", pos: Pos::Noun },
    Concept { concept_id: "LJ-021", gloss: "night", pos: Pos::Noun },
    Concept { concept_id: "LJ-022", gloss: "ear", pos: Pos::Noun },
    Concept { concept_id: "LJ-023", gloss: "neck", pos: Pos::Noun },
    Concept { concept_id: "LJ-024", gloss: "far", pos: Pos::Adjective },
    Concept { concept_id: "LJ-025", gloss: "to do, make", pos: Pos::Verb },
    Concept { concept_id: "LJ-026", gloss: "house", pos: Pos::Noun },
    Concept { concept_id: "LJ-027", gloss: "stone, rock", pos: Pos::Noun },
    Concept { concept_id: "LJ-028", gloss: "bitter", pos: Pos::Adjective },
    Concept { concept_id: "LJ-029", gloss: "to say", pos: Pos::Verb },
    Concept { concept_id: "LJ-030", gloss: "tooth", pos: Pos::Noun },
    Concept { concept_id: "LJ-031", gloss: "hair", pos: Pos::Noun },
    Concept { concept_id: "LJ-032", gloss: "big", pos: Pos::Adjective },
    Concept { concept_id: "LJ-033", gloss: "one", pos: Pos::Numeral },
    Concept { concept_id: "LJ-034", gloss: "who?", pos: Pos::Pronoun },
    Concept { concept_id: "LJ-035", gloss: "he/she/it", pos: Pos::Pronoun },
    Concept { concept_id: "LJ-036", gloss: "to hit, beat", pos: Pos::Verb },
    Concept { concept_id: "LJ-037", gloss: "leg, foot", pos: Pos::Noun },
    Concept { concept_id: "LJ-038", gloss: "horn", pos: Pos::Noun },
    Concept { concept_id: "LJ-039", gloss: "this", pos: Pos::Pronoun },
    Concept { concept_id: "LJ-040", gloss: "fish", pos: Pos::Noun },
    Concept { concept_id: "LJ-041", gloss: "yesterday", pos: Pos::Adverb },
    Concept { concept_id: "LJ-042", gloss: "to drink", pos: Pos::Verb },
    Concept { concept_id: "LJ-043", gloss: "black", pos: Pos::Adjective },
    Concept { concept_id: "LJ-044", gloss: "navel", pos: Pos::Noun },
    Concept { concept_id: "LJ-045", gloss: "to stand", pos: Pos::Verb },
    Concept { concept_id: "LJ-046", gloss: "to bite", pos: Pos::Verb },
    Concept { concept_id: "LJ-047", gloss: "back", pos: Pos::Noun },
    Concept { concept_id: "LJ-048", gloss: "wind", pos: Pos::Noun },
    Concept { concept_id: "LJ-049", gloss: "smoke", pos: Pos::Noun },
    Concept { concept_id: "LJ-050", gloss: "what?", pos: Pos::Pronoun },
    Concept { concept_id: "LJ-051", gloss: "child (kin)", pos: Pos::Noun },
    Concept { concept_id: "LJ-052", gloss: "egg", pos: Pos::Noun },
    Concept { concept_id: "LJ-053", gloss: "to give", pos: Pos::Verb },
    Concept { concept_id: "LJ-054", gloss: "new", pos: Pos::Adjective },
    Concept { concept_id: "LJ-055", gloss: "to burn (intr.)", pos: Pos::Verb },
    Concept { concept_id: "LJ-056", gloss: "not", pos: Pos::Particle },
    Concept { concept_id: "LJ-057", gloss: "good", pos: Pos::Adjective },
    Concept { concept_id: "LJ-058", gloss: "to know", pos: Pos::Verb },
    Concept { concept_id: "LJ-059", gloss: "knee", pos: Pos::Noun },
    Concept { concept_id: "LJ-060", gloss: "sand", pos: Pos::Noun },
    Concept { concept_id: "LJ-061", gloss: "to laugh", pos: Pos::Verb },
    Concept { concept_id: "LJ-062", gloss: "to hear", pos: Pos::Verb },
    Concept { concept_id: "LJ-063", gloss: "soil", pos: Pos::Noun },
    Concept { concept_id: "LJ-064", gloss: "leaf", pos: Pos::Noun },
    Concept { concept_id: "LJ-065", gloss: "red", pos: Pos::Adjective },
    Concept { concept_id: "LJ-066", gloss: "liver", pos: Pos::Noun },
    Concept { concept_id: "LJ-067", gloss: "to hide", pos: Pos::Verb },
    Concept { concept_id: "LJ-068", gloss: "skin, hide", pos: Pos::Noun },
    Concept { concept_id: "LJ-069", gloss: "to suck", pos: Pos::Verb },
    Concept { concept_id: "LJ-070", gloss: "to carry", pos: Pos::Verb },
    Concept { concept_id: "LJ-071", gloss: "ant", pos: Pos::Noun },
    Concept { concept_id: "LJ-072", gloss: "heavy", pos: Pos::Adjective },
    Concept { concept_id: "LJ-073", gloss: "to take", pos: Pos::Verb },
    Concept { concept_id: "LJ-074", gloss: "old", pos: Pos::Adjective },
    Concept { concept_id: "LJ-075", gloss: "to eat", pos: Pos::Verb },
    Concept { concept_id: "LJ-076", gloss: "thigh", pos: Pos::Noun },
    Concept { concept_id: "LJ-077", gloss: "thick", pos: Pos::Adjective },
    Concept { concept_id: "LJ-078", gloss: "long", pos: Pos::Adjective },
    Concept { concept_id: "LJ-079", gloss: "to blow", pos: Pos::Verb },
    Concept { concept_id: "LJ-080", gloss: "wood", pos: Pos::Noun },
    Concept { concept_id: "LJ-081", gloss: "to run", pos: Pos::Verb },
    Concept { concept_id: "LJ-082", gloss: "to fall", pos: Pos::Verb },
    Concept { concept_id: "LJ-083", gloss: "eye", pos: Pos::Noun },
    Concept { concept_id: "LJ-084", gloss: "ash", pos: Pos::Noun },
    Concept { concept_id: "LJ-085", gloss: "tail", pos: Pos::Noun },
    Concept { concept_id: "LJ-086", gloss: "dog", pos: Pos::Noun },
    Concept { concept_id: "LJ-087", gloss: "to cry, weep", pos: Pos::Verb },
    Concept { concept_id: "LJ-088", gloss: "to tie", pos: Pos::Verb },
    Concept { concept_id: "LJ-089", gloss: "to see", pos: Pos::Verb },
    Concept { concept_id: "LJ-090", gloss: "sweet", pos: Pos::Adjective },
    Concept { concept_id: "LJ-091", gloss: "rope", pos: Pos::Noun },
    Concept { concept_id: "LJ-092", gloss: "shade, shadow", pos: Pos::Noun },
    Concept { concept_id: "LJ-093", gloss: "bird", pos: Pos::Noun },
    Concept { concept_id: "LJ-094", gloss: "salt", pos: Pos::Noun },
    Concept { concept_id: "LJ-095", gloss: "small", pos: Pos::Adjective },
    Concept { concept_id: "LJ-096", gloss: "wide", pos: Pos::Adjective },
    Concept { concept_id: "LJ-097", gloss: "star", pos: Pos::Noun },
    Concept { concept_id: "LJ-098", gloss: "in", pos: Pos::Particle },
    Concept { concept_id: "LJ-099", gloss: "hard", pos: Pos::Adjective },
    Concept { concept_id: "LJ-100", gloss: "to crush, grind", pos: Pos::Verb },
];

/// A second hundred beyond Leipzig–Jakarta: numerals, kinship, landscape,
/// weather, everyday verbs and property words. Local GX ids — these are
/// curated for usefulness in story realization, not ranked by
/// borrowability.
pub const EXTENSION_100: &[Concept] = &[
    Concept { concept_id: "GX-101", gloss: "two", pos: Pos::Numeral },
    Concept { concept_id: "GX-102", gloss: "three", pos: Pos::Numeral },
    Concept { concept_id: "GX-103", gloss: "four", pos: Pos::Numeral },
    Concept { concept_id: "GX-104", gloss: "five", pos: Pos::Numeral },
    Concept { concept_id: "GX-105", gloss: "we", pos: Pos::Pronoun },
    Concept { concept_id: "GX-106", gloss: "you (pl)", pos: Pos::Pronoun },
    Concept { concept_id: "GX-107", gloss: "they", pos: Pos::Pronoun },
    Concept { concept_id: "GX-108", gloss: "that", pos: Pos::Pronoun },
    Concept { concept_id: "GX-109", gloss: "mother", pos: Pos::Noun },
    Concept { concept_id: "GX-110", gloss: "father", pos: Pos::Noun },
    Concept { concept_id: "GX-111", gloss: "sister", pos: Pos::Noun },
    Concept { concept_id: "GX-112", gloss: "brother", pos: Pos::Noun },
    Concept { concept_id: "GX-113", gloss: "son", pos: Pos::Noun },
    Concept { concept_id: "GX-114", gloss: "daughter", pos: Pos::Noun },
    Concept { concept_id: "GX-115", gloss: "husband", pos: Pos::Noun },
    Concept { concept_id: "GX-116", gloss: "wife", pos: Pos::Noun },
    Concept { concept_id: "GX-117", gloss: "head", pos: Pos::Noun },
    Concept { concept_id: "GX-118", gloss: "heart", pos: Pos::Noun },
    Concept { concept_id: "GX-119", gloss: "belly", pos: Pos::Noun },
    Concept { concept_id: "GX-120", gloss: "finger", pos: Pos::Noun },
    Concept { concept_id: "GX-121", gloss: "sun", pos: Pos::Noun },
    Concept { concept_id: "GX-122", gloss: "moon", pos: Pos::Noun },
    Concept { concept_id: "GX-123", gloss: "sky", pos: Pos::Noun },
    Concept { concept_id: "GX-124", gloss: "sea", pos: Pos::Noun },
    Concept { concept_id: "GX-125", gloss: "river", pos: Pos::Noun },
    Concept { concept_id: "GX-126", gloss: "mountain", pos: Pos::Noun },
    Concept { concept_id: "GX-127", gloss: "tree", pos: Pos::Noun },
    Concept { concept_id: "GX-128", gloss: "flower", pos: Pos::Noun },
    Concept { concept_id: "GX-129", gloss: "grass", pos: Pos::Noun },
    Concept { concept_id: "GX-130", gloss: "snow", pos: Pos::Noun },
    Concept { concept_id: "GX-131", gloss: "ice", pos: Pos::Noun },
    Concept { concept_id: "GX-132", gloss: "cloud", pos: Pos::Noun },
    Concept { concept_id: "GX-133", gloss: "lake", pos: Pos::Noun },
    Concept { concept_id: "GX-134", gloss: "forest", pos: Pos::Noun },
    Concept { concept_id: "GX-135", gloss: "day", pos: Pos::Noun },
    Concept { concept_id: "GX-136", gloss: "year", pos: Pos::Noun },
    Concept { concept_id: "GX-137", gloss: "today", pos: Pos::Adverb },
    Concept { concept_id: "GX-138", gloss: "tomorrow", pos: Pos::Adverb },
    Concept { concept_id: "GX-139", gloss: "summer", pos: Pos::Noun },
    Concept { concept_id: "GX-140", gloss: "winter", pos: Pos::Noun },
    Concept { concept_id: "GX-141", gloss: "snake", pos: Pos::Noun },
    Concept { concept_id: "GX-142", gloss: "mouse", pos: Pos::Noun },
    Concept { concept_id: "GX-143", gloss: "wolf", pos: Pos::Noun },
    Concept { concept_id: "GX-144", gloss: "bear", pos: Pos::Noun },
    Concept { concept_id: "GX-145", gloss: "deer", pos: Pos::Noun },
    Concept { concept_id: "GX-146", gloss: "frog", pos: Pos::Noun },
    Concept { concept_id: "GX-147", gloss: "spider", pos: Pos::Noun },
    Concept { concept_id: "GX-148", gloss: "bee", pos: Pos::Noun },
    Concept { concept_id: "GX-149", gloss: "worm", pos: Pos::Noun },
    Concept { concept_id: "GX-150", gloss: "to sleep", pos: Pos::Verb },
    Concept { concept_id: "GX-151", gloss: "to die", pos: Pos::Verb },
    Concept { concept_id: "GX-152", gloss: "to live", pos: Pos::Verb },
    Concept { concept_id: "GX-153", gloss: "to walk", pos: Pos::Verb },
    Concept { concept_id: "GX-154", gloss: "to swim", pos: Pos::Verb },
    Concept { concept_id: "GX-155", gloss: "to fly", pos: Pos::Verb },
    Concept { concept_id: "GX-156", gloss: "to sit", pos: Pos::Verb },
    Concept { concept_id: "GX-157", gloss: "to lie (down)", pos: Pos::Verb },
    Concept { concept_id: "GX-158", gloss: "to sing", pos: Pos::Verb },
    Concept { concept_id: "GX-159", gloss: "to play", pos: Pos::Verb },
    Concept { concept_id: "GX-160", gloss: "to want", pos: Pos::Verb },
    Concept { concept_id: "GX-161", gloss: "to love", pos: Pos::Verb },
    Concept { concept_id: "GX-162", gloss: "to think", pos: Pos::Verb },
    Concept { concept_id: "GX-163", gloss: "to count", pos: Pos::Verb },
    Concept { concept_id: "GX-164", gloss: "to open", pos: Pos::Verb },
    Concept { concept_id: "GX-165", gloss: "to close", pos: Pos::Verb },
    Concept { concept_id: "GX-166", gloss: "to cut", pos: Pos::Verb },
    Concept { concept_id: "GX-167", gloss: "to break", pos: Pos::Verb },
    Concept { concept_id: "GX-168", gloss: "to throw", pos: Pos::Verb },
    Concept { concept_id: "GX-169", gloss: "to wash", pos: Pos::Verb },
    Concept { concept_id: "GX-170", gloss: "to cook", pos: Pos::Verb },
    Concept { concept_id: "GX-171", gloss: "to build", pos: Pos::Verb },
    Concept { concept_id: "GX-172", gloss: "to find", pos: Pos::Verb },
    Concept { concept_id: "GX-173", gloss: "to seek", pos: Pos::Verb },
    Concept { concept_id: "GX-174", gloss: "to hold", pos: Pos::Verb },
    Concept { concept_id: "GX-175", gloss: "to pull", pos: Pos::Verb },
    Concept { concept_id: "GX-176", gloss: "to push", pos: Pos::Verb },
    Concept { concept_id: "GX-177", gloss: "to dig", pos: Pos::Verb },
    Concept { concept_id: "GX-178", gloss: "to sew", pos: Pos::Verb },
    Concept { concept_id: "GX-179", gloss: "white", pos: Pos::Adjective },
    Concept { concept_id: "GX-180", gloss: "green", pos: Pos::Adjective },
    Concept { concept_id: "GX-181", gloss: "yellow", pos: Pos::Adjective },
    Concept { concept_id: "GX-182", gloss: "warm", pos: Pos::Adjective },
    Concept { concept_id: "GX-183", gloss: "cold", pos: Pos::Adjective },
    Concept { concept_id: "GX-184", gloss: "dry", pos: Pos::Adjective },
    Concept { concept_id: "GX-185", gloss: "wet", pos: Pos::Adjective },
    Concept { concept_id: "GX-186", gloss: "full", pos: Pos::Adjective },
    Concept { concept_id: "GX-187", gloss: "empty", pos: Pos::Adjective },
    Concept { concept_id: "GX-188", gloss: "near", pos: Pos::Adjective },
    Concept { concept_id: "GX-189", gloss: "high", pos: Pos::Adjective },
    Concept { concept_id: "GX-190", gloss: "low", pos: Pos::Adjective },
    Concept { concept_id: "GX-191", gloss: "round", pos: Pos::Adjective },
    Concept { concept_id: "GX-192", gloss: "sharp", pos: Pos::Adjective },
    Concept { concept_id: "GX-193", gloss: "straight", pos: Pos::Adjective },
    Concept { concept_id: "GX-194", gloss: "dirty", pos: Pos::Adjective },
    Concept { concept_id: "GX-195", gloss: "soft", pos: Pos::Adjective },
    Concept { concept_id: "GX-196", gloss: "light (weight)", pos: Pos::Adjective },
    Concept { concept_id: "GX-197", gloss: "path, road", pos: Pos::Noun },
    Concept { concept_id: "GX-198", gloss: "village", pos: Pos::Noun },
    Concept { concept_id: "GX-199", gloss: "boat", pos: Pos::Noun },
    Concept { concept_id: "GX-200", gloss: "knife", pos: Pos::Noun },
];

/// Every concept the seeder generates a root for, in seeding order.
pub fn seed_concepts() -> impl Iterator<Item = &'static Concept> {
    LEIPZIG_JAKARTA_100.iter().chain(EXTENSION_100)
}

/// A dictionary entry as authored: a form attached to one or more concepts.
/// Multiple concept IDs per lexeme is deliberate — that's where
/// colexification ("arm" and "hand" as one word) plugs in later without a
/// schema change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lexeme {
    pub form_ipa: String,
    pub concept_ids: Vec<String>,
    pub pos: Pos,
    pub notes: Option<String>,
}
