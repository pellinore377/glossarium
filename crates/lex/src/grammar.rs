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
    Vos,
    Ovs,
    Osv,
}

impl WordOrder {
    pub const ALL: [WordOrder; 6] = [
        WordOrder::Sov,
        WordOrder::Svo,
        WordOrder::Vso,
        WordOrder::Vos,
        WordOrder::Ovs,
        WordOrder::Osv,
    ];

    pub fn key(self) -> &'static str {
        match self {
            WordOrder::Sov => "sov",
            WordOrder::Svo => "svo",
            WordOrder::Vso => "vso",
            WordOrder::Vos => "vos",
            WordOrder::Ovs => "ovs",
            WordOrder::Osv => "osv",
        }
    }

    pub fn parse(s: &str) -> Option<WordOrder> {
        WordOrder::ALL.iter().copied().find(|w| w.key() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            WordOrder::Sov => "Subject–Object–Verb",
            WordOrder::Svo => "Subject–Verb–Object",
            WordOrder::Vso => "Verb–Subject–Object",
            WordOrder::Vos => "Verb–Object–Subject",
            WordOrder::Ovs => "Object–Verb–Subject",
            WordOrder::Osv => "Object–Subject–Verb",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            WordOrder::Sov => "the most common order on Earth (Japanese, Turkish, Latin)",
            WordOrder::Svo => "a close second (English, Mandarin, Swahili)",
            WordOrder::Vso => "rarer but sturdy (Irish, Classical Arabic, Māori)",
            WordOrder::Vos => "uncommon (Malagasy, Fijian)",
            WordOrder::Ovs => "vanishingly rare (Hixkaryana) — a bold choice",
            WordOrder::Osv => "the rarest attested order — Yoda territory",
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

    pub fn parse(s: &str) -> Option<Marking> {
        match s {
            "suffix" => Some(Marking::Suffix),
            "prefix" => Some(Marking::Prefix),
            "particle" => Some(Marking::Particle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NegationStrategy {
    /// A free word before the verb — the most common strategy.
    Particle,
    /// Bound to the verb, Silágo ke- style.
    Prefix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PronounRow {
    pub person: u8,
    pub plural: bool,
    pub nom: String,
    pub acc: String,
    pub gen: String,
}

impl PronounRow {
    pub fn label(&self) -> &'static str {
        match (self.person, self.plural) {
            (1, false) => "I",
            (2, false) => "you",
            (3, false) => "he/she/it",
            (1, true) => "we",
            (2, true) => "you (pl)",
            _ => "they",
        }
    }
}

/// The full grammar sketch. Every generated form is a suggestion the
/// wizard lets the user overwrite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarSpec {
    // Clause structure
    pub word_order: WordOrder,
    /// true = prepositions, false = postpositions.
    pub prepositions: bool,
    pub adj_before_noun: bool,
    pub possessor_before_noun: bool,
    // Nouns
    pub plural_marking: Marking,
    pub plural_form: String,
    pub definite_article: Option<String>,
    // Pronouns
    pub pronoun_case: bool,
    pub animacy: bool,
    pub pronouns: Vec<PronounRow>,
    // Verbs: present is the bare stem, always.
    pub past_form: String,
    pub future_form: Option<String>,
    pub continuous_form: Option<String>,
    pub perfect_aux: Option<String>,
    /// None = zero copula (predicate stands bare next to its subject).
    pub copula: Option<String>,
    pub negation: NegationStrategy,
    pub negation_form: String,
    // Word-building
    pub modals: Vec<(String, String)>,
    pub derivations: Vec<(String, String)>,
}

pub const MODAL_CONCEPTS: &[&str] =
    &["ability (can)", "possibility (might)", "obligation (must)", "desire (want to)", "necessity (need to)"];

pub const DERIVATION_MEANINGS: &[&str] = &[
    "agent (one who does)",
    "place of",
    "diminutive (small)",
    "augmentative (great)",
    "abstract quality (-ness)",
    "potential (-able)",
    "adverb (-ly)",
    "collection of",
];

/// The vowel glyphs of the universal chart, for allomorphy decisions.
pub const IPA_VOWELS: &str = "iyɨʉɯuɪʏʊeøɘɵɤoəɛœɜɞʌɔæɐaɶɑɒ";

fn ends_in_vowel(s: &str) -> bool {
    s.chars().last().map(|c| IPA_VOWELS.contains(c)).unwrap_or(false)
}

fn starts_with_vowel(s: &str) -> bool {
    s.chars().next().map(|c| IPA_VOWELS.contains(c)).unwrap_or(false)
}

/// Suffix attachment with automatic allomorphy, Silágo-style: when a
/// vowel-final stem meets a vowel-initial suffix, the suffix's vowel
/// drops (kata + et → katat; kat + et → katet).
pub fn attach_suffix(stem: &str, suffix: &str) -> String {
    if ends_in_vowel(stem) && starts_with_vowel(suffix) && suffix.chars().count() > 1 {
        let rest: String = suffix.chars().skip(1).collect();
        format!("{stem}{rest}")
    } else {
        format!("{stem}{suffix}")
    }
}

/// Prefix attachment: a vowel-final prefix elides its vowel before a
/// vowel-initial stem (like Silágo's l'-).
pub fn attach_prefix(prefix: &str, stem: &str) -> String {
    if ends_in_vowel(prefix) && starts_with_vowel(stem) && prefix.chars().count() > 1 {
        let cut: String = prefix
            .chars()
            .take(prefix.chars().count() - 1)
            .collect();
        format!("{cut}{stem}")
    } else {
        format!("{prefix}{stem}")
    }
}

/// Deterministic first-draft grammar from the same phonology that built
/// the words. The wizard shows every value for editing.
pub fn generate(spec: WordSpec) -> Result<GrammarSpec, GenError> {
    let mut g = Generator::new(spec)?;
    let word_order = WordOrder::ALL[g.pick_index(&[40, 35, 12, 6, 4, 3])];
    // SOV languages overwhelmingly take postpositions; others lean pre.
    let prepositions = if word_order == WordOrder::Sov {
        g.pick_index(&[15, 85]) == 0
    } else {
        g.pick_index(&[85, 15]) == 0
    };
    let adj_before_noun = g.pick_index(&[1, 1]) == 0;
    let possessor_before_noun = if word_order == WordOrder::Sov {
        g.pick_index(&[80, 20]) == 0
    } else {
        g.pick_index(&[1, 1]) == 0
    };

    let marking = |i: usize| match i {
        0 => Marking::Suffix,
        1 => Marking::Prefix,
        _ => Marking::Particle,
    };
    let plural_marking = marking(g.pick_index(&[70, 15, 15]));
    let plural_form = g.short_word();
    let definite_article = (g.pick_index(&[40, 60]) == 0).then(|| g.short_word());

    let pronoun_case = g.pick_index(&[60, 40]) == 0;
    let animacy = g.pick_index(&[1, 1]) == 0;
    let acc_suffix = g.short_word();
    let gen_prefix = g.short_word();
    let stems = [g.short_word(), g.short_word(), g.short_word()];
    let mut pronouns = Vec::new();
    for plural in [false, true] {
        for person in 1..=3u8 {
            let base = &stems[(person - 1) as usize];
            let nom = if plural {
                attach_suffix(base, &plural_form)
            } else {
                base.clone()
            };
            let acc = if pronoun_case {
                attach_suffix(&nom, &acc_suffix)
            } else {
                nom.clone()
            };
            let gen = if pronoun_case {
                attach_prefix(&gen_prefix, &nom)
            } else {
                nom.clone()
            };
            pronouns.push(PronounRow { person, plural, nom, acc, gen });
        }
    }

    let past_form = g.short_word();
    let future_form = (g.pick_index(&[70, 30]) == 0).then(|| g.short_word());
    let continuous_form = (g.pick_index(&[60, 40]) == 0).then(|| g.short_word());
    let perfect_aux = (g.pick_index(&[50, 50]) == 0).then(|| g.short_word());
    let copula = (g.pick_index(&[55, 45]) == 0).then(|| g.short_word());
    let negation = if g.pick_index(&[60, 40]) == 0 {
        NegationStrategy::Particle
    } else {
        NegationStrategy::Prefix
    };
    let negation_form = g.short_word();

    let modals = MODAL_CONCEPTS
        .iter()
        .map(|c| (c.to_string(), g.short_word()))
        .collect();
    let derivations = DERIVATION_MEANINGS
        .iter()
        .map(|m| (m.to_string(), g.short_word()))
        .collect();

    Ok(GrammarSpec {
        word_order,
        prepositions,
        adj_before_noun,
        possessor_before_noun,
        plural_marking,
        plural_form,
        definite_article,
        pronoun_case,
        animacy,
        pronouns,
        past_form,
        future_form,
        continuous_form,
        perfect_aux,
        copula,
        negation,
        negation_form,
        modals,
        derivations,
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
            medial_pairs: None,
            seed: 99,
        };
        let a = generate(spec()).unwrap();
        let b = generate(spec()).unwrap();
        assert_eq!(a.word_order, b.word_order);
        assert_eq!(a.plural_form, b.plural_form);
        assert_eq!(a.negation_form, b.negation_form);
        assert_eq!(a.pronouns.len(), 6);
        assert_eq!(a.modals.len(), MODAL_CONCEPTS.len());
        assert_eq!(a.derivations.len(), DERIVATION_MEANINGS.len());
    }

    #[test]
    fn attachment_allomorphy() {
        assert_eq!(attach_suffix("kata", "et"), "katat");
        assert_eq!(attach_suffix("kat", "et"), "katet");
        assert_eq!(attach_prefix("li", "ata"), "lata");
        assert_eq!(attach_prefix("li", "kata"), "likata");
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
