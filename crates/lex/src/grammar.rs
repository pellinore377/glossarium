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

/// The language's overall morphological temperament. Not a straitjacket
/// — a bias that shapes which strategies the draft reaches for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MorphType {
    Isolating,
    Agglutinative,
    Fusional,
}

impl MorphType {
    pub const ALL: [MorphType; 3] =
        [MorphType::Isolating, MorphType::Agglutinative, MorphType::Fusional];

    pub fn key(self) -> &'static str {
        match self {
            MorphType::Isolating => "isolating",
            MorphType::Agglutinative => "agglutinative",
            MorphType::Fusional => "fusional",
        }
    }

    pub fn parse(s: &str) -> Option<MorphType> {
        MorphType::ALL.iter().copied().find(|m| m.key() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            MorphType::Isolating => "Isolating",
            MorphType::Agglutinative => "Agglutinative",
            MorphType::Fusional => "Fusional",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            MorphType::Isolating => {
                "meaning lives in word order and particles; words rarely \
                 inflect (Mandarin, Vietnamese)"
            }
            MorphType::Agglutinative => {
                "words are bead-strings of clean, single-purpose affixes \
                 (Turkish, Swahili, Japanese)"
            }
            MorphType::Fusional => {
                "affixes fuse several meanings into one form (Latin, \
                 Russian, Spanish)"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumStrategy {
    Suffix,
    Prefix,
    Particle,
    /// The stem doubles: kela → kelakela (Indonesian, Malay).
    Reduplication,
}

impl NumStrategy {
    pub fn key(self) -> &'static str {
        match self {
            NumStrategy::Suffix => "suffix",
            NumStrategy::Prefix => "prefix",
            NumStrategy::Particle => "particle",
            NumStrategy::Reduplication => "reduplication",
        }
    }
    pub fn parse(s: &str) -> Option<NumStrategy> {
        [NumStrategy::Suffix, NumStrategy::Prefix, NumStrategy::Particle, NumStrategy::Reduplication]
            .into_iter()
            .find(|m| m.key() == s)
    }
    pub fn label(self) -> &'static str {
        match self {
            NumStrategy::Suffix => "suffix",
            NumStrategy::Prefix => "prefix",
            NumStrategy::Particle => "particle",
            NumStrategy::Reduplication => "reduplication",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum NumberSystem {
    /// Number from context or numerals only (Mandarin nouns).
    NoMarking,
    Plural { strategy: NumStrategy, plural: String },
    /// One, two, many (Arabic, Slovene).
    DualPlural { strategy: NumStrategy, dual: String, plural: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    /// No case on nouns; word order does the work.
    Neutral,
    /// Objects get marked (Latin -m, Japanese o).
    NomAcc,
    /// Transitive subjects get marked (Basque, Georgian).
    ErgAbs,
}

impl Alignment {
    pub fn key(self) -> &'static str {
        match self {
            Alignment::Neutral => "neutral",
            Alignment::NomAcc => "nomacc",
            Alignment::ErgAbs => "ergabs",
        }
    }
    pub fn parse(s: &str) -> Option<Alignment> {
        [Alignment::Neutral, Alignment::NomAcc, Alignment::ErgAbs]
            .into_iter()
            .find(|a| a.key() == s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseAffix {
    pub name: String,
    pub suffix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenderSystem {
    None,
    AnimateInanimate,
    MascFem,
}

impl GenderSystem {
    pub fn key(self) -> &'static str {
        match self {
            GenderSystem::None => "none",
            GenderSystem::AnimateInanimate => "animate",
            GenderSystem::MascFem => "mascfem",
        }
    }
    pub fn parse(s: &str) -> Option<GenderSystem> {
        [GenderSystem::None, GenderSystem::AnimateInanimate, GenderSystem::MascFem]
            .into_iter()
            .find(|g| g.key() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NegationStrategy {
    /// A free word before the verb (the global favourite).
    Particle,
    Prefix,
    Suffix,
    /// A negative verb that carries the tense while the main verb goes
    /// bare (Finnish).
    Auxiliary,
}

impl NegationStrategy {
    pub fn key(self) -> &'static str {
        match self {
            NegationStrategy::Particle => "particle",
            NegationStrategy::Prefix => "prefix",
            NegationStrategy::Suffix => "suffix",
            NegationStrategy::Auxiliary => "auxiliary",
        }
    }
    pub fn parse(s: &str) -> Option<NegationStrategy> {
        [
            NegationStrategy::Particle,
            NegationStrategy::Prefix,
            NegationStrategy::Suffix,
            NegationStrategy::Auxiliary,
        ]
        .into_iter()
        .find(|n| n.key() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStrategy {
    /// Sentence-final question particle (Japanese ka, Mandarin ma).
    FinalParticle,
    /// Sentence-initial particle (Polish czy).
    InitialParticle,
    /// Verb fronting (English, German).
    Inversion,
    /// Rising intonation only (Italian, colloquial everywhere).
    Intonation,
}

impl QuestionStrategy {
    pub fn key(self) -> &'static str {
        match self {
            QuestionStrategy::FinalParticle => "final_particle",
            QuestionStrategy::InitialParticle => "initial_particle",
            QuestionStrategy::Inversion => "inversion",
            QuestionStrategy::Intonation => "intonation",
        }
    }
    pub fn parse(s: &str) -> Option<QuestionStrategy> {
        [
            QuestionStrategy::FinalParticle,
            QuestionStrategy::InitialParticle,
            QuestionStrategy::Inversion,
            QuestionStrategy::Intonation,
        ]
        .into_iter()
        .find(|q| q.key() == s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TenseSystem {
    /// No tense — aspect carries time (Mandarin le).
    Tenseless { perfective: String },
    /// Past vs everything else (many languages).
    PastNonpast { past: String },
    /// Past, present, future all marked.
    ThreeWay { past: String, future: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModalStrategy {
    /// Bound to the verb stem (Silágo má-).
    Prefixes,
    Suffixes,
    /// Full verbs taking a complement (English can, want).
    Verbs,
    /// Free particles in the verb phrase (Mandarin huì).
    Particles,
}

impl ModalStrategy {
    pub fn key(self) -> &'static str {
        match self {
            ModalStrategy::Prefixes => "prefixes",
            ModalStrategy::Suffixes => "suffixes",
            ModalStrategy::Verbs => "verbs",
            ModalStrategy::Particles => "particles",
        }
    }
    pub fn parse(s: &str) -> Option<ModalStrategy> {
        [
            ModalStrategy::Prefixes,
            ModalStrategy::Suffixes,
            ModalStrategy::Verbs,
            ModalStrategy::Particles,
        ]
        .into_iter()
        .find(|m| m.key() == s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Comparative {
    /// "big than-X" with a than-word.
    Particle { than: String },
    /// "big-er than X" with a degree suffix.
    Suffix { suffix: String, than: String },
    /// "big exceeds X" — a verb does it (Mandarin, Yoruba).
    ExceedVerb { verb: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PronounRow {
    pub person: u8,
    pub plural: bool,
    /// Subject-case form (nominative or absolutive).
    pub a: String,
    /// Marked-case form (accusative or ergative); equals `a` when
    /// pronouns don't decline.
    pub b: String,
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

/// The full grammar sketch: which categories this language bothers to
/// mark, and by which strategy. Every generated form is a suggestion the
/// wizard lets the user overwrite; whole subsystems can be absent —
/// that's typology, not a bug.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarSpec {
    pub morphology: MorphType,
    // Clause structure
    pub word_order: WordOrder,
    /// true = prepositions, false = postpositions.
    pub prepositions: bool,
    pub adj_before_noun: bool,
    pub possessor_before_noun: bool,
    // Nouns
    pub number: NumberSystem,
    pub alignment: Alignment,
    /// The marked core case (accusative or ergative suffix).
    pub core_case: String,
    pub extra_cases: Vec<CaseAffix>,
    pub gender: GenderSystem,
    pub definite_article: Option<String>,
    pub indefinite_article: Option<String>,
    // Pronouns
    pub pronoun_case: bool,
    pub pronouns: Vec<PronounRow>,
    // Verbs: present/nonpast is the bare stem, always.
    pub agreement: Option<Vec<(String, String)>>,
    pub tense: TenseSystem,
    /// true = tense markers are free pre-verbal particles, not suffixes.
    pub tense_particles: bool,
    pub continuous: Option<String>,
    pub perfect_aux: Option<String>,
    /// None = zero copula (predicate stands bare next to its subject).
    pub copula: Option<String>,
    pub negation: NegationStrategy,
    pub negation_form: String,
    pub question: QuestionStrategy,
    pub question_form: String,
    /// (witnessed, hearsay) verb suffixes — Turkish/Quechua territory.
    pub evidentiality: Option<(String, String)>,
    // Word-building
    pub modality: ModalStrategy,
    pub modals: Vec<(String, String)>,
    pub comparative: Comparative,
    pub converbs: Option<Vec<(String, String)>>,
    pub derivations: Vec<(String, String)>,
}

pub const MODAL_CONCEPTS: &[&str] =
    &["ability (can)", "possibility (might)", "obligation (must)", "desire (want to)", "necessity (need to)"];

pub const AGREEMENT_LABELS: &[&str] = &["1sg", "2sg", "3sg", "1pl", "2pl", "3pl"];

pub const CONVERB_MEANINGS: &[&str] =
    &["while (simultaneous)", "because (causal)", "in order to (purposive)"];

pub const EXTRA_CASE_NAMES: &[&str] =
    &["genitive", "dative", "locative", "instrumental", "ablative"];

/// The derivation pool: each language gets a different subset.
pub const DERIVATION_POOL: &[&str] = &[
    "agent (one who does)",
    "place of",
    "diminutive (small)",
    "augmentative (great)",
    "abstract quality (-ness)",
    "potential (-able)",
    "adverb (-ly)",
    "collection of",
    "instrument for",
    "result of action",
    "opposite of (un-)",
    "again (re-)",
    "-like, resembling",
    "full of",
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

/// Deterministic first-draft grammar. `force_morph` lets the wizard
/// re-draft under a different morphological temperament; everything
/// downstream re-rolls consistently with it.
pub fn generate(spec: WordSpec, force_morph: Option<MorphType>) -> Result<GrammarSpec, GenError> {
    let mut g = Generator::new(spec)?;
    let rolled = MorphType::ALL[g.pick_index(&[30, 40, 30])];
    let morphology = force_morph.unwrap_or(rolled);
    use MorphType::*;

    // Per-morphology weight helper: (isolating, agglutinative, fusional).
    macro_rules! by {
        ($iso:expr, $agg:expr, $fus:expr) => {
            match morphology {
                Isolating => $iso,
                Agglutinative => $agg,
                Fusional => $fus,
            }
        };
    }

    let word_order = WordOrder::ALL[g.pick_index(&[40, 35, 12, 6, 4, 3])];
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

    // ---- Nouns ----
    let number = match g.pick_index(&by!(&[25, 70, 5], &[5, 70, 25], &[5, 85, 10])) {
        0 => NumberSystem::NoMarking,
        n => {
            let strategy = match g.pick_index(&by!(
                &[15, 5, 55, 25],
                &[75, 15, 5, 5],
                &[80, 15, 5, 0]
            )) {
                0 => NumStrategy::Suffix,
                1 => NumStrategy::Prefix,
                2 => NumStrategy::Particle,
                _ => NumStrategy::Reduplication,
            };
            let plural = g.short_word();
            if n == 2 {
                NumberSystem::DualPlural { strategy, dual: g.short_word(), plural }
            } else {
                NumberSystem::Plural { strategy, plural }
            }
        }
    };
    let alignment = match g.pick_index(&by!(&[70, 25, 5], &[25, 45, 30], &[15, 60, 25])) {
        0 => Alignment::Neutral,
        1 => Alignment::NomAcc,
        _ => Alignment::ErgAbs,
    };
    let core_case = g.short_word();
    let n_extra = match morphology {
        Isolating => 0,
        Agglutinative => g.pick_index(&[15, 20, 25, 25, 15]),
        Fusional => g.pick_index(&[35, 35, 30]),
    };
    let extra_cases: Vec<CaseAffix> = EXTRA_CASE_NAMES
        .iter()
        .take(n_extra)
        .map(|n| CaseAffix { name: n.to_string(), suffix: g.short_word() })
        .collect();
    let gender = match g.pick_index(&[55, 25, 20]) {
        0 => GenderSystem::None,
        1 => GenderSystem::AnimateInanimate,
        _ => GenderSystem::MascFem,
    };
    let definite_article =
        (g.pick_index(&by!(&[35, 65], &[30, 70], &[45, 55])) == 0).then(|| g.short_word());
    let indefinite_article = (g.pick_index(&[20, 80]) == 0).then(|| g.short_word());

    // ---- Pronouns ----
    let pronoun_case = g.pick_index(&by!(&[25, 75], &[50, 50], &[75, 25])) == 0;
    let case_suffix = g.short_word();
    let gen_prefix = g.short_word();
    let plural_bit = g.short_word();
    let stems = [g.short_word(), g.short_word(), g.short_word()];
    let mut pronouns = Vec::new();
    for plural in [false, true] {
        for person in 1..=3u8 {
            let base = &stems[(person - 1) as usize];
            let a = if plural { attach_suffix(base, &plural_bit) } else { base.clone() };
            let b = if pronoun_case { attach_suffix(&a, &case_suffix) } else { a.clone() };
            let gen = if pronoun_case { attach_prefix(&gen_prefix, &a) } else { a.clone() };
            pronouns.push(PronounRow { person, plural, a, b, gen });
        }
    }

    // ---- Verbs ----
    let agreement = (g.pick_index(&by!(&[5, 95], &[45, 55], &[60, 40])) == 0).then(|| {
        AGREEMENT_LABELS
            .iter()
            .map(|l| (l.to_string(), g.short_word()))
            .collect::<Vec<_>>()
    });
    let tense = match g.pick_index(&by!(&[25, 45, 30], &[10, 35, 55], &[5, 30, 65])) {
        0 => TenseSystem::Tenseless { perfective: g.short_word() },
        1 => TenseSystem::PastNonpast { past: g.short_word() },
        _ => TenseSystem::ThreeWay { past: g.short_word(), future: g.short_word() },
    };
    let tense_particles = morphology == Isolating;
    let continuous = (g.pick_index(&[50, 50]) == 0).then(|| g.short_word());
    let perfect_aux = (g.pick_index(&[45, 55]) == 0).then(|| g.short_word());
    let copula = (g.pick_index(&[55, 45]) == 0).then(|| g.short_word());
    let negation = match g.pick_index(&by!(
        &[70, 5, 5, 20],
        &[35, 25, 25, 15],
        &[45, 25, 20, 10]
    )) {
        0 => NegationStrategy::Particle,
        1 => NegationStrategy::Prefix,
        2 => NegationStrategy::Suffix,
        _ => NegationStrategy::Auxiliary,
    };
    let negation_form = g.short_word();
    let question = match g.pick_index(&[35, 20, 15, 30]) {
        0 => QuestionStrategy::FinalParticle,
        1 => QuestionStrategy::InitialParticle,
        2 => QuestionStrategy::Inversion,
        _ => QuestionStrategy::Intonation,
    };
    let question_form = g.short_word();
    let evidentiality =
        (g.pick_index(&[20, 80]) == 0).then(|| (g.short_word(), g.short_word()));

    // ---- Word-building ----
    let modality = match g.pick_index(&by!(
        &[5, 5, 45, 45],
        &[30, 30, 25, 15],
        &[25, 20, 45, 10]
    )) {
        0 => ModalStrategy::Prefixes,
        1 => ModalStrategy::Suffixes,
        2 => ModalStrategy::Verbs,
        _ => ModalStrategy::Particles,
    };
    let modals = MODAL_CONCEPTS
        .iter()
        .map(|c| (c.to_string(), g.short_word()))
        .collect();
    let comparative = match g.pick_index(&[45, 30, 25]) {
        0 => Comparative::Particle { than: g.short_word() },
        1 => Comparative::Suffix { suffix: g.short_word(), than: g.short_word() },
        _ => Comparative::ExceedVerb { verb: g.short_word() },
    };
    let converbs = (g.pick_index(&by!(&[25, 75], &[50, 50], &[30, 70])) == 0).then(|| {
        CONVERB_MEANINGS
            .iter()
            .map(|m| (m.to_string(), g.short_word()))
            .collect::<Vec<_>>()
    });
    let keep = by!(25u32, 55, 40);
    let mut derivations: Vec<(String, String)> = Vec::new();
    for m in DERIVATION_POOL {
        let form = g.short_word();
        if g.pick_index(&[keep, 100 - keep]) == 0 {
            derivations.push((m.to_string(), form));
        }
    }
    if derivations.len() < 3 {
        for m in DERIVATION_POOL.iter().take(3) {
            if !derivations.iter().any(|(x, _)| x == m) {
                derivations.push((m.to_string(), g.short_word()));
            }
        }
    }

    Ok(GrammarSpec {
        morphology,
        word_order,
        prepositions,
        adj_before_noun,
        possessor_before_noun,
        number,
        alignment,
        core_case,
        extra_cases,
        gender,
        definite_article,
        indefinite_article,
        pronoun_case,
        pronouns,
        agreement,
        tense,
        tense_particles,
        continuous,
        perfect_aux,
        copula,
        negation,
        negation_form,
        question,
        question_form,
        evidentiality,
        modality,
        modals,
        comparative,
        converbs,
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
            onset_triples: None,
            coda_triples: None,
            seed: 99,
        };
        let a = generate(spec(), None).unwrap();
        let b = generate(spec(), None).unwrap();
        assert_eq!(a.word_order, b.word_order);
        assert_eq!(a.morphology, b.morphology);
        assert_eq!(a.negation_form, b.negation_form);
        assert_eq!(a.pronouns.len(), 6);
        assert_eq!(a.modals.len(), MODAL_CONCEPTS.len());
        assert!(a.derivations.len() >= 3);
    }

    #[test]
    fn forced_morphology_is_respected() {
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
            onset_triples: None,
            coda_triples: None,
            seed: 7,
        };
        for m in MorphType::ALL {
            let g = generate(spec(), Some(m)).unwrap();
            assert_eq!(g.morphology, m);
        }
        let iso = generate(spec(), Some(MorphType::Isolating)).unwrap();
        assert!(iso.tense_particles);
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
