//! Lexicon model.
//!
//! Lexemes hang off the *proto* language of a family; daughters carry only
//! derived reflexes and explicit overrides. Concepts are identified by
//! Concepticon-style IDs so the seed list, the IDS expansion (v1.5), and
//! colexification data all speak the same key space.

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

/// A meaning slot, independent of any language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    /// Concepticon concept-set ID where one exists; local IDs otherwise.
    pub concept_id: &'static str,
    pub gloss: &'static str,
    pub pos: Pos,
}

/// The Leipzig–Jakarta 100 core list: the v1 proto-lexicon seed.
/// Chosen over Swadesh for its empirical borrowing-resistance ranking —
/// the right anchor for cognate tracking across a family tree.
///
/// Glosses follow the published list; Concepticon IDs to be filled in when
/// the full concept tables land (data/, v1.5). A representative slice is
/// included here so the generator and UI have something real to chew on;
/// the remainder of the 100 goes in with the lexicon milestone.
pub const LEIPZIG_JAKARTA_SEED: &[Concept] = &[
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
];

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
