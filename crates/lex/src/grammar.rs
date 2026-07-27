//! Grammar sketch generation and the story skeleton.
//!
//! A grammar here is deliberately small: word order, adjective placement,
//! and how plural, past, and negation are marked — enough to realize a
//! short story and to give the sketch page something true to say. All
//! choices come off the same deterministic RNG stream as the lexicon, so
//! a language's grammar is as reproducible as its words.
//!
//! The story is a glossed skeleton: lines reference concepts by their
//! seed gloss, and realization happens in the web layer where the
//! lexicon lives. Daughters realize the proto's sentence and push every
//! word through their sound-change chain — grammar evolves by erosion,
//! exactly like vocabulary.

use crate::gen::{GenError, Generator, WordSpec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordOrder {
    Sov,
    Svo,
    Vso,
}

impl WordOrder {
    pub fn label(self) -> &'static str {
        match self {
            WordOrder::Sov => "Subject–Object–Verb",
            WordOrder::Svo => "Subject–Verb–Object",
            WordOrder::Vso => "Verb–Subject–Object",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            WordOrder::Sov => "the most common order on Earth (Japanese, Turkish, Latin)",
            WordOrder::Svo => "a close second (English, Mandarin, Swahili)",
            WordOrder::Vso => "rarer but sturdy (Irish, Classical Arabic, Māori)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Marking {
    Suffix,
    Prefix,
    Particle,
}

impl Marking {
    pub fn label(self) -> &'static str {
        match self {
            Marking::Suffix => "suffix",
            Marking::Prefix => "prefix",
            Marking::Particle => "particle",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarSpec {
    pub word_order: WordOrder,
    pub adj_before_noun: bool,
    pub plural_marking: Marking,
    pub plural_form: String,
    pub past_marking: Marking,
    pub past_form: String,
    /// Negation is always a free particle placed before the verb — the
    /// single most common strategy cross-linguistically.
    pub negation_form: String,
}

/// Deterministic grammar from the same phonology that built the words.
pub fn generate(spec: WordSpec) -> Result<GrammarSpec, GenError> {
    let mut g = Generator::new(spec)?;
    let word_order = match g.pick_index(&[45, 40, 15]) {
        0 => WordOrder::Sov,
        1 => WordOrder::Svo,
        _ => WordOrder::Vso,
    };
    let adj_before_noun = g.pick_index(&[1, 1]) == 0;
    let marking = |i: usize| match i {
        0 => Marking::Suffix,
        1 => Marking::Prefix,
        _ => Marking::Particle,
    };
    let plural_marking = marking(g.pick_index(&[70, 15, 15]));
    let past_marking = marking(g.pick_index(&[60, 15, 25]));
    Ok(GrammarSpec {
        word_order,
        adj_before_noun,
        plural_marking,
        plural_form: g.short_word(),
        past_marking,
        past_form: g.short_word(),
        negation_form: g.short_word(),
    })
}

// ---------- The story skeleton ----------

/// One clause. Phrases list glosses adjective-first, head noun last;
/// realization reorders per the grammar. All verbs are past tense —
/// it's a story.
pub struct StoryLine {
    pub english: &'static str,
    pub subject: &'static [&'static str],
    pub subject_plural: bool,
    pub verb: Option<&'static str>,
    pub object: &'static [&'static str],
    /// (adposition gloss, phrase)
    pub oblique: Option<(&'static str, &'static [&'static str])>,
    pub negated: bool,
}

pub const STORY_TITLE: &str = "The wolf and the water";

/// Six lines, using only glosses guaranteed present in the seed lexicon.
pub const STORY: &[StoryLine] = &[
    StoryLine {
        english: "The old wolf saw the fire.",
        subject: &["old", "wolf"],
        subject_plural: false,
        verb: Some("to see"),
        object: &["fire"],
        oblique: None,
        negated: false,
    },
    StoryLine {
        english: "The night was cold.",
        subject: &["night"],
        subject_plural: false,
        verb: None, // zero copula: predicate adjective in object slot
        object: &["cold"],
        oblique: None,
        negated: false,
    },
    StoryLine {
        english: "He went to the water.",
        subject: &["he/she/it"],
        subject_plural: false,
        verb: Some("to go"),
        object: &[],
        oblique: Some(("in", &["water"])),
        negated: false,
    },
    StoryLine {
        english: "He drank the sweet water.",
        subject: &["he/she/it"],
        subject_plural: false,
        verb: Some("to drink"),
        object: &["sweet", "water"],
        oblique: None,
        negated: false,
    },
    StoryLine {
        english: "The stars stood in the sky.",
        subject: &["star"],
        subject_plural: true,
        verb: Some("to stand"),
        object: &[],
        oblique: Some(("in", &["sky"])),
        negated: false,
    },
    StoryLine {
        english: "The wolf did not sleep in the house.",
        subject: &["wolf"],
        subject_plural: false,
        verb: Some("to sleep"),
        object: &[],
        oblique: Some(("in", &["house"])),
        negated: true,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn grammar_is_deterministic() {
        let spec = || WordSpec {
            consonants: s(&["p", "t", "k", "m", "n", "s", "l"]),
            vowels: s(&["a", "i", "u"]),
            diphthongs: vec![],
            onset_min: 0,
            onset_max: 1,
            coda_min: 0,
            coda_max: 1,
            onset_pairs: None,
            coda_pairs: None,
            onset_singles: None,
            coda_singles: None,
            seed: 99,
        };
        let a = generate(spec()).unwrap();
        let b = generate(spec()).unwrap();
        assert_eq!(a.word_order, b.word_order);
        assert_eq!(a.plural_form, b.plural_form);
        assert_eq!(a.negation_form, b.negation_form);
    }

    #[test]
    fn story_glosses_exist_in_seed() {
        let all: Vec<&str> = crate::seed_concepts().map(|c| c.gloss).collect();
        for line in STORY {
            for g in line.subject.iter().chain(line.object) {
                assert!(all.contains(g), "missing gloss {g}");
            }
            if let Some(v) = line.verb {
                assert!(all.contains(&v), "missing verb {v}");
            }
            if let Some((adp, phrase)) = line.oblique {
                assert!(all.contains(&adp), "missing adposition {adp}");
                for g in phrase {
                    assert!(all.contains(g), "missing gloss {g}");
                }
            }
        }
    }
}
