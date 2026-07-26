//! The shipped catalog: thirty-seven documented sound changes.
//!
//! Every entry is a change that has actually happened in an attested
//! language, expressed as feature-matrix rules, with an applicability
//! predicate so the evolve menu only offers changes that could touch the
//! language at hand. Naturalness is a rough cross-linguistic frequency
//! weight for sorting the menu — 0.95 means "happens constantly,
//! everywhere", 0.5 means "well documented but you'd mention it in the
//! grammar's introduction".
//!
//! Rule-writing discipline: a delta that needs to *remove* a feature
//! sets it to `Unspecified` (the resolver treats explicit-Unspecified
//! and absent as identical); explicit `Minus` means the segment really
//! carries the negative value.

use crate::{Applicability, Boundary, CatalogEntry, EnvSegment, FeaturePattern, Rule};
use phon::Feature::{self, *};
use phon::FeatureValue::{self, *};
use std::sync::OnceLock;

type F = (Feature, FeatureValue);

fn pat(pairs: &[F]) -> FeaturePattern {
    pairs.iter().copied().collect()
}

fn has(pairs: &[F]) -> Applicability {
    Applicability::HasSegment { pattern: pat(pairs) }
}

fn all_of(of: Vec<Applicability>) -> Applicability {
    Applicability::All { of }
}

/// Compact rule spec; unset fields take the obvious defaults.
struct R<'a> {
    name: &'a str,
    target: &'a [F],
    change: &'a [F],
    left: &'a [F],
    right: &'a [F],
    boundary: Boundary,
    delete: bool,
}

impl Default for R<'_> {
    fn default() -> Self {
        R {
            name: "",
            target: &[],
            change: &[],
            left: &[],
            right: &[],
            boundary: Boundary::Anywhere,
            delete: false,
        }
    }
}

fn rule(r: R) -> Rule {
    let env = |pairs: &[F]| -> Vec<EnvSegment> {
        if pairs.is_empty() {
            vec![]
        } else {
            vec![EnvSegment { pattern: pat(pairs) }]
        }
    };
    Rule {
        name: r.name.to_string(),
        target: pat(r.target),
        change: pat(r.change),
        delete: r.delete,
        left: env(r.left),
        right: env(r.right),
        boundary: r.boundary,
    }
}

fn entry(
    id: &str,
    display_name: &str,
    description: &str,
    naturalness: f32,
    applicable_when: Applicability,
    rules: Vec<Rule>,
) -> CatalogEntry {
    CatalogEntry {
        id: id.to_string(),
        display_name: display_name.to_string(),
        description: description.to_string(),
        rules,
        applicable_when,
        naturalness,
    }
}

// Recurring patterns.
const V: &[F] = &[(Syllabic, Plus)];
const ANY_C: &[F] = &[(Consonantal, Plus)];
const VOICED_OBSTRUENT: &[F] = &[(Consonantal, Plus), (Sonorant, Minus), (Voice, Plus)];
const VOICELESS_STOP: &[F] = &[
    (Consonantal, Plus), (Sonorant, Minus), (Continuant, Minus), (Voice, Minus),
];
const VOICED_STOP: &[F] = &[
    (Consonantal, Plus), (Sonorant, Minus), (Continuant, Minus), (Voice, Plus),
];

fn build() -> Vec<CatalogEntry> {
    vec![
        // ---- Laryngeal ----
        entry(
            "final-devoicing", "Final devoicing",
            "Obstruents lose voicing at the end of a word: bad > bat. \
             German, Dutch, Russian, Polish, Turkish — one of the most \
             common changes on Earth.",
            0.95,
            has(VOICED_OBSTRUENT),
            vec![rule(R {
                name: "final devoicing",
                target: VOICED_OBSTRUENT,
                change: &[(Voice, Minus)],
                boundary: Boundary::WordFinal,
                ..R::default()
            })],
        ),
        entry(
            "intervocalic-voicing", "Intervocalic voicing",
            "Voiceless stops voice between vowels: lupa > luba. The first \
             step of Western Romance lenition (Latin vita > Spanish vida).",
            0.9,
            has(VOICELESS_STOP),
            vec![rule(R {
                name: "intervocalic voicing",
                target: VOICELESS_STOP,
                change: &[(Voice, Plus)],
                left: V, right: V,
                ..R::default()
            })],
        ),
        entry(
            "regressive-devoicing", "Cluster devoicing",
            "A voiced obstruent devoices before a voiceless one: abta > \
             apta. Russian and Polish do this without exception.",
            0.8,
            has(VOICED_OBSTRUENT),
            vec![rule(R {
                name: "regressive devoicing",
                target: VOICED_OBSTRUENT,
                change: &[(Voice, Minus)],
                right: &[(Consonantal, Plus), (Sonorant, Minus), (Voice, Minus)],
                ..R::default()
            })],
        ),
        entry(
            "regressive-voicing", "Cluster voicing",
            "A voiceless obstruent voices before a voiced one: akda > \
             agda. The mirror of cluster devoicing; Slavic has both.",
            0.75,
            has(&[(Consonantal, Plus), (Sonorant, Minus), (Voice, Minus)]),
            vec![rule(R {
                name: "regressive voicing",
                target: &[(Consonantal, Plus), (Sonorant, Minus), (Voice, Minus)],
                change: &[(Voice, Plus)],
                right: VOICED_OBSTRUENT,
                ..R::default()
            })],
        ),
        // ---- Lenition ----
        entry(
            "spirantization", "Intervocalic spirantization",
            "Voiced stops soften to fricatives between vowels: aba > aβa. \
             Spanish does this live; Hebrew and Danish did it historically.",
            0.85,
            has(VOICED_STOP),
            vec![rule(R {
                name: "spirantization",
                target: VOICED_STOP,
                change: &[(Continuant, Plus)],
                left: V, right: V,
                ..R::default()
            })],
        ),
        entry(
            "voiced-stop-loss", "Intervocalic voiced-stop loss",
            "Voiced stops vanish between vowels entirely: ada > aa. The \
             late stage of lenition — Spanish -ado > -ao in speech.",
            0.6,
            has(VOICED_STOP),
            vec![rule(R {
                name: "voiced stop loss",
                target: VOICED_STOP,
                delete: true,
                left: V, right: V,
                ..R::default()
            })],
        ),
        entry(
            "flapping", "Intervocalic flapping",
            "Alveolar stops become a tap between vowels: ata > aɾa. North \
             American English \"water\"; also regular in many Australian \
             languages.",
            0.7,
            has(&[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Minus),
                  (Coronal, Plus), (Anterior, Plus)]),
            vec![rule(R {
                name: "flapping",
                target: &[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Minus),
                          (Coronal, Plus), (Anterior, Plus)],
                change: &[(Sonorant, Plus), (Voice, Plus)],
                left: V, right: V,
                ..R::default()
            })],
        ),
        entry(
            "h-loss", "H-loss",
            "/h/ disappears everywhere: hara > ara. Romance lost Latin's \
             /h/ twice; Cockney and many Caribbean Englishes are doing it \
             now.",
            0.85,
            has(&[(SpreadGlottis, Plus)]),
            vec![rule(R {
                name: "h loss",
                target: &[(SpreadGlottis, Plus)],
                delete: true,
                ..R::default()
            })],
        ),
        entry(
            "glottal-stop-loss", "Glottal-stop loss",
            "/ʔ/ disappears everywhere. Extremely common — the glottal \
             stop is the quietest consonant a language can lose.",
            0.8,
            has(&[(ConstrictedGlottis, Plus)]),
            vec![rule(R {
                name: "glottal stop loss",
                target: &[(ConstrictedGlottis, Plus)],
                delete: true,
                ..R::default()
            })],
        ),
        entry(
            "s-debuccalization", "Final s-debuccalization",
            "Word-final /s/ weakens to [h]: estas > estah. Caribbean and \
             Andalusian Spanish; also ancient Greek word-initially.",
            0.7,
            has(&[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                  (Coronal, Plus), (Anterior, Plus), (Voice, Minus), (Lateral, Minus)]),
            vec![rule(R {
                name: "s debuccalization",
                target: &[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                          (Coronal, Plus), (Anterior, Plus), (Voice, Minus), (Lateral, Minus)],
                change: &[(Consonantal, Minus), (Coronal, Unspecified),
                          (Anterior, Unspecified), (Distributed, Unspecified),
                          (SpreadGlottis, Plus)],
                boundary: Boundary::WordFinal,
                ..R::default()
            })],
        ),
        entry(
            "back-fricative-debuccalization", "Back-fricative debuccalization",
            "Velar, uvular, and pharyngeal fricatives collapse to [h]: \
             xala > hala. English did this to /x/ (\"night\"); Maltese to \
             its pharyngeals.",
            0.7,
            has(&[(Dorsal, Plus), (Continuant, Plus), (Sonorant, Minus), (Back, Plus)]),
            vec![
                rule(R {
                    name: "voiceless back fricative > h",
                    target: &[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                              (Dorsal, Plus), (Back, Plus), (Voice, Minus)],
                    change: &[(Consonantal, Minus), (Dorsal, Unspecified),
                              (High, Unspecified), (Back, Unspecified), (Low, Unspecified),
                              (SpreadGlottis, Plus)],
                    ..R::default()
                }),
                rule(R {
                    name: "voiced back fricative > ɦ",
                    target: &[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                              (Dorsal, Plus), (Back, Plus), (Voice, Plus)],
                    change: &[(Consonantal, Minus), (Dorsal, Unspecified),
                              (High, Unspecified), (Back, Unspecified), (Low, Unspecified),
                              (SpreadGlottis, Plus)],
                    ..R::default()
                }),
            ],
        ),
        entry(
            "f-to-h", "Labial-fricative debuccalization",
            "/f/ and /ɸ/ weaken to [h]: fara > hara. Old Spanish (Latin \
             farina > harina) and Japanese both walked this road.",
            0.7,
            has(&[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                  (Labial, Plus), (Voice, Minus)]),
            vec![rule(R {
                name: "f > h",
                target: &[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                          (Labial, Plus), (Voice, Minus)],
                change: &[(Consonantal, Minus), (Labial, Unspecified),
                          (Distributed, Unspecified), (SpreadGlottis, Plus)],
                ..R::default()
            })],
        ),
        // ---- Deletion ----
        entry(
            "apocope", "Final-vowel loss (apocope)",
            "Word-final vowels drop after a consonant: kata > kat. French \
             and Old English both gutted their final syllables this way. \
             CV monosyllables lose their only vowel too — check the \
             preview before adopting.",
            0.8,
            has(V),
            vec![rule(R {
                name: "apocope",
                target: V,
                delete: true,
                left: ANY_C,
                boundary: Boundary::WordFinal,
                ..R::default()
            })],
        ),
        entry(
            "final-obstruent-loss", "Final-obstruent loss",
            "Word-final obstruents delete: kat > ka. French again (petit), \
             and the road Chinese varieties took from Middle Chinese codas.",
            0.65,
            has(&[(Consonantal, Plus), (Sonorant, Minus)]),
            vec![rule(R {
                name: "final obstruent loss",
                target: &[(Consonantal, Plus), (Sonorant, Minus)],
                delete: true,
                boundary: Boundary::WordFinal,
                ..R::default()
            })],
        ),
        entry(
            "final-cluster-reduction", "Final cluster reduction",
            "The last consonant of a word-final cluster drops: kast > kas. \
             African-American English and Caribbean creoles, plus most \
             fast speech everywhere.",
            0.75,
            has(&[(Consonantal, Plus), (Sonorant, Minus)]),
            vec![rule(R {
                name: "final cluster reduction",
                target: &[(Consonantal, Plus), (Sonorant, Minus)],
                delete: true,
                left: ANY_C,
                boundary: Boundary::WordFinal,
                ..R::default()
            })],
        ),
        entry(
            "final-nasal-loss", "Final-nasal loss",
            "Word-final nasals delete: kan > ka. French did this while \
             nasalizing the vowel; Mandarin kept only -n and -ŋ from a \
             richer set.",
            0.6,
            has(&[(Consonantal, Plus), (Nasal, Plus)]),
            vec![rule(R {
                name: "final nasal loss",
                target: &[(Consonantal, Plus), (Nasal, Plus)],
                delete: true,
                boundary: Boundary::WordFinal,
                ..R::default()
            })],
        ),
        // ---- Rhotics, laterals, glides ----
        entry(
            "rhotacism", "Rhotacism",
            "/s z/ become [r] between vowels: asa > ara. Latin (flos, \
             floris) and Germanic (English was/were) both did it.",
            0.75,
            has(&[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                  (Coronal, Plus), (Anterior, Plus), (Distributed, Minus), (Lateral, Minus)]),
            vec![rule(R {
                name: "rhotacism",
                target: &[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                          (Coronal, Plus), (Anterior, Plus), (Distributed, Minus),
                          (Lateral, Minus)],
                change: &[(Sonorant, Plus), (Voice, Plus)],
                left: V, right: V,
                ..R::default()
            })],
        ),
        entry(
            "lambdacism", "Lambdacism (r > l)",
            "The trilled rhotic becomes lateral: rama > lama. Regular in \
             several Bantu and Austronesian histories; sporadic in Romance.",
            0.55,
            has(&[(Consonantal, Plus), (Sonorant, Plus), (Continuant, Plus),
                  (Coronal, Plus), (Lateral, Minus)]),
            vec![rule(R {
                name: "lambdacism",
                target: &[(Consonantal, Plus), (Sonorant, Plus), (Continuant, Plus),
                          (Coronal, Plus), (Lateral, Minus)],
                change: &[(Lateral, Plus)],
                ..R::default()
            })],
        ),
        entry(
            "l-rhotacism", "Lateral rhotacism (l > r)",
            "Laterals become rhotics: lama > rama. Korean and Japanese \
             famously refuse to keep the two apart; Romanian turned Latin \
             -l- into -r-.",
            0.55,
            has(&[(Consonantal, Plus), (Sonorant, Plus), (Continuant, Plus), (Lateral, Plus)]),
            vec![rule(R {
                name: "l rhotacism",
                target: &[(Consonantal, Plus), (Sonorant, Plus), (Continuant, Plus),
                          (Lateral, Plus)],
                change: &[(Lateral, Minus)],
                ..R::default()
            })],
        ),
        entry(
            "rhotic-uvularization", "Rhotic uvularization",
            "The front trill retreats to the uvula: r > ʀ. French, German, \
             Danish, and Sorani Kurdish all did this within recorded \
             history.",
            0.6,
            has(&[(Consonantal, Plus), (Sonorant, Plus), (Continuant, Plus),
                  (Coronal, Plus), (Anterior, Plus), (Lateral, Minus)]),
            vec![rule(R {
                name: "uvularization",
                target: &[(Consonantal, Plus), (Sonorant, Plus), (Continuant, Plus),
                          (Coronal, Plus), (Anterior, Plus), (Lateral, Minus)],
                change: &[(Coronal, Unspecified), (Anterior, Unspecified),
                          (Distributed, Unspecified), (Dorsal, Plus),
                          (High, Minus), (Back, Plus)],
                ..R::default()
            })],
        ),
        entry(
            "l-vocalization", "Final l-vocalization",
            "Word-final laterals turn into [u]: sal > sau. Brazilian \
             Portuguese, Polish ł, and London English (\"miwk\" for milk).",
            0.6,
            has(&[(Lateral, Plus), (Sonorant, Plus)]),
            vec![rule(R {
                name: "l vocalization",
                target: &[(Lateral, Plus), (Sonorant, Plus)],
                change: &[(Consonantal, Minus), (Lateral, Unspecified), (Syllabic, Plus),
                          (High, Plus), (Low, Minus), (Back, Plus), (Round, Plus),
                          (Tense, Plus), (Coronal, Unspecified), (Anterior, Unspecified),
                          (Distributed, Unspecified), (Dorsal, Unspecified)],
                boundary: Boundary::WordFinal,
                ..R::default()
            })],
        ),
        entry(
            "glide-fortition", "Initial glide fortition (j > ʝ)",
            "Word-initial /j/ hardens to a fricative: jama > ʝama. \
             Rioplatense Spanish took this further still (to [ʒ~ʃ]).",
            0.6,
            has(&[(Consonantal, Minus), (Sonorant, Plus), (Continuant, Plus),
                  (Dorsal, Plus), (High, Plus), (Back, Minus)]),
            vec![rule(R {
                name: "glide fortition",
                target: &[(Consonantal, Minus), (Sonorant, Plus), (Continuant, Plus),
                          (Dorsal, Plus), (High, Plus), (Back, Minus)],
                change: &[(Consonantal, Plus), (Sonorant, Minus)],
                boundary: Boundary::WordInitial,
                ..R::default()
            })],
        ),
        entry(
            "labial-fortition", "Initial labial-glide fortition (ʋ > v)",
            "A word-initial labial approximant hardens to a fricative. \
             Germanic w > v in German and Yiddish is the classic case.",
            0.55,
            has(&[(Consonantal, Minus), (Sonorant, Plus), (Continuant, Plus), (Labial, Plus)]),
            vec![rule(R {
                name: "labial fortition",
                target: &[(Consonantal, Minus), (Sonorant, Plus), (Continuant, Plus),
                          (Labial, Plus)],
                change: &[(Consonantal, Plus), (Sonorant, Minus)],
                boundary: Boundary::WordInitial,
                ..R::default()
            })],
        ),
        entry(
            "initial-fortition", "Initial fricative fortition",
            "Word-initial voiced fricatives harden to stops: βara > bara. \
             How Germanic *b *d *g surfaced in stressed positions; also \
             common in loan adaptation.",
            0.6,
            has(&[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                  (Voice, Plus), (Lateral, Minus)]),
            vec![rule(R {
                name: "initial fortition",
                target: &[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                          (Voice, Plus), (Lateral, Minus)],
                change: &[(Continuant, Minus)],
                boundary: Boundary::WordInitial,
                ..R::default()
            })],
        ),
        // ---- Place assimilation & palatalization ----
        entry(
            "velar-palatalization", "Velar palatalization",
            "Velars slide forward before front sounds: ki > ci. The change \
             behind Latin centum's two daughters and Slavic's alternation \
             jungles; arguably the most repeated change in recorded \
             history.",
            0.85,
            all_of(vec![
                has(&[(Dorsal, Plus), (High, Plus), (Back, Plus)]),
                has(&[(Syllabic, Plus), (Back, Minus)]),
            ]),
            vec![
                rule(R {
                    name: "velar stop palatalization",
                    target: &[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Minus),
                              (Dorsal, Plus), (High, Plus), (Back, Plus)],
                    change: &[(Back, Minus)],
                    right: &[(Back, Minus)],
                    ..R::default()
                }),
                rule(R {
                    name: "velar fricative palatalization",
                    target: &[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                              (Dorsal, Plus), (High, Plus), (Back, Plus)],
                    change: &[(Back, Minus)],
                    right: &[(Back, Minus)],
                    ..R::default()
                }),
                rule(R {
                    name: "velar nasal palatalization",
                    target: &[(Consonantal, Plus), (Nasal, Plus),
                              (Dorsal, Plus), (High, Plus), (Back, Plus)],
                    change: &[(Back, Minus)],
                    right: &[(Back, Minus)],
                    ..R::default()
                }),
            ],
        ),
        entry(
            "assibilation", "Assibilation (s > ʃ)",
            "/s z/ hush to [ʃ ʒ] before high front sounds: si > ʃi. \
             Japanese (si > shi), Korean, and Brazilian Portuguese di/ti.",
            0.75,
            all_of(vec![
                has(&[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                      (Coronal, Plus), (Anterior, Plus), (Distributed, Minus)]),
                has(&[(High, Plus), (Back, Minus)]),
            ]),
            vec![rule(R {
                name: "assibilation",
                target: &[(Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus),
                          (Coronal, Plus), (Anterior, Plus), (Distributed, Minus),
                          (Lateral, Minus)],
                change: &[(Anterior, Minus), (Distributed, Plus)],
                right: &[(High, Plus), (Back, Minus)],
                ..R::default()
            })],
        ),
        entry(
            "nasal-assimilation-labial", "Nasal place assimilation (labial)",
            "/n/ becomes [m] before labials: anpa > ampa. Nearly \
             exceptionless wherever the cluster arises — Latin in- + \
             possibilis = impossibilis.",
            0.9,
            all_of(vec![
                has(&[(Nasal, Plus), (Coronal, Plus)]),
                has(&[(Consonantal, Plus), (Labial, Plus)]),
            ]),
            vec![rule(R {
                name: "nasal labial assimilation",
                target: &[(Nasal, Plus), (Coronal, Plus)],
                change: &[(Coronal, Unspecified), (Anterior, Unspecified),
                          (Distributed, Plus), (Labial, Plus)],
                right: &[(Consonantal, Plus), (Labial, Plus)],
                ..R::default()
            })],
        ),
        entry(
            "nasal-assimilation-velar", "Nasal place assimilation (velar)",
            "/n/ becomes [ŋ] before velars: anka > aŋka. English \"ink\", \
             Spanish \"un gato\" — wherever n meets k, this follows.",
            0.9,
            all_of(vec![
                has(&[(Nasal, Plus), (Coronal, Plus)]),
                has(&[(Consonantal, Plus), (Dorsal, Plus), (High, Plus), (Back, Plus)]),
            ]),
            vec![rule(R {
                name: "nasal velar assimilation",
                target: &[(Nasal, Plus), (Coronal, Plus)],
                change: &[(Coronal, Unspecified), (Anterior, Unspecified),
                          (Distributed, Unspecified), (Dorsal, Plus),
                          (High, Plus), (Back, Plus)],
                right: &[(Consonantal, Plus), (Dorsal, Plus), (High, Plus), (Back, Plus)],
                ..R::default()
            })],
        ),
        entry(
            "final-nasal-merger", "Final-nasal merger (> n)",
            "All word-final nasals collapse to [n]: kam > kan. Spanish and \
             Greek both levelled their final nasals this way.",
            0.7,
            has(&[(Consonantal, Plus), (Nasal, Plus)]),
            vec![rule(R {
                name: "final nasal merger",
                target: &[(Consonantal, Plus), (Nasal, Plus)],
                change: &[(Labial, Unspecified), (Dorsal, Unspecified),
                          (High, Unspecified), (Back, Unspecified),
                          (Coronal, Plus), (Anterior, Plus), (Distributed, Minus)],
                boundary: Boundary::WordFinal,
                ..R::default()
            })],
        ),
        entry(
            "th-fronting", "Th-fronting (θ > f)",
            "Dental fricatives merge into labiodentals: θin > fin. London \
             English, and the fate of θ in most languages that ever had \
             it.",
            0.6,
            has(&[(Coronal, Plus), (Anterior, Plus), (Distributed, Plus),
                  (Continuant, Plus), (Sonorant, Minus)]),
            vec![
                rule(R {
                    name: "th fronting voiceless",
                    target: &[(Coronal, Plus), (Anterior, Plus), (Distributed, Plus),
                              (Continuant, Plus), (Sonorant, Minus), (Voice, Minus)],
                    change: &[(Coronal, Unspecified), (Anterior, Unspecified),
                              (Distributed, Minus), (Labial, Plus)],
                    ..R::default()
                }),
                rule(R {
                    name: "th fronting voiced",
                    target: &[(Coronal, Plus), (Anterior, Plus), (Distributed, Plus),
                              (Continuant, Plus), (Sonorant, Minus), (Voice, Plus)],
                    change: &[(Coronal, Unspecified), (Anterior, Unspecified),
                              (Distributed, Minus), (Labial, Plus)],
                    ..R::default()
                }),
            ],
        ),
        // ---- Vowel shifts ----
        entry(
            "tense-mid-raising", "Tense mid-vowel raising",
            "Tense mid vowels rise: e > i, o > u. Greek eta became ita; \
             the upper half of England's Great Vowel Shift.",
            0.7,
            has(&[(Syllabic, Plus), (High, Minus), (Low, Minus), (Tense, Plus)]),
            vec![rule(R {
                name: "mid raising",
                target: &[(Syllabic, Plus), (High, Minus), (Low, Minus), (Tense, Plus)],
                change: &[(High, Plus)],
                ..R::default()
            })],
        ),
        entry(
            "lax-vowel-tensing", "Open-mid raising",
            "Open-mid vowels close up: ɛ > e, ɔ > o. Half of every \
             \"five-vowel system\" origin story, Romance included.",
            0.7,
            has(&[(Syllabic, Plus), (High, Minus), (Low, Minus), (Tense, Minus)]),
            vec![rule(R {
                name: "open-mid raising",
                target: &[(Syllabic, Plus), (High, Minus), (Low, Minus), (Tense, Minus)],
                change: &[(Tense, Plus)],
                ..R::default()
            })],
        ),
        entry(
            "back-vowel-fronting", "Rounded back-vowel fronting",
            "Rounded back vowels move front, keeping their rounding: u > \
             y, o > ø. French and Ancient Greek u > y; the source of most \
             front rounded vowels alive today.",
            0.6,
            has(&[(Syllabic, Plus), (Back, Plus), (Round, Plus)]),
            vec![rule(R {
                name: "back fronting",
                target: &[(Syllabic, Plus), (Back, Plus), (Round, Plus)],
                change: &[(Back, Minus)],
                ..R::default()
            })],
        ),
        entry(
            "front-unrounding", "Front-vowel unrounding",
            "Front rounded vowels lose their rounding: y > i, ø > e. How \
             English and Greek eventually disposed of the y's they'd made.",
            0.7,
            has(&[(Syllabic, Plus), (Back, Minus), (Round, Plus)]),
            vec![rule(R {
                name: "front unrounding",
                target: &[(Syllabic, Plus), (Back, Minus), (Round, Plus)],
                change: &[(Round, Minus)],
                ..R::default()
            })],
        ),
        entry(
            "low-back-rounding", "Low back rounding (ɑ > ɔ)",
            "The low back vowel rounds and lifts: ɑ > ɔ. English \"caught\" \
             class; Persian and western Armenian did the same.",
            0.6,
            has(&[(Syllabic, Plus), (Low, Plus), (Back, Plus), (Round, Minus)]),
            vec![rule(R {
                name: "low back rounding",
                target: &[(Syllabic, Plus), (Low, Plus), (Back, Plus), (Round, Minus)],
                change: &[(Low, Minus), (Round, Plus), (Tense, Minus)],
                ..R::default()
            })],
        ),
        entry(
            "pre-nasal-raising", "Pre-nasal raising",
            "Open-mid vowels rise before nasals: ɛn > ɪn. Southern US \
             English pin/pen merger; regular in several Chinese varieties.",
            0.5,
            all_of(vec![
                has(&[(Syllabic, Plus), (High, Minus), (Low, Minus), (Tense, Minus)]),
                has(&[(Nasal, Plus)]),
            ]),
            vec![rule(R {
                name: "pre-nasal raising",
                target: &[(Syllabic, Plus), (High, Minus), (Low, Minus), (Tense, Minus)],
                change: &[(High, Plus)],
                right: &[(Nasal, Plus)],
                ..R::default()
            })],
        ),
        entry(
            "monophthongization", "Monophthongization",
            "Closing diphthongs smooth into single vowels: ai > e, au > o. \
             Sanskrit to Pali, Latin to Romance, Old English to Middle — \
             diphthongs rarely survive a millennium.",
            0.65,
            all_of(vec![
                has(&[(Syllabic, Plus), (Low, Plus)]),
                has(&[(Syllabic, Plus), (High, Plus)]),
            ]),
            vec![
                rule(R {
                    name: "nucleus fronting-raising before i",
                    target: &[(Syllabic, Plus), (Low, Plus)],
                    change: &[(Low, Minus), (Tense, Plus)],
                    right: &[(Syllabic, Plus), (High, Plus), (Back, Minus)],
                    ..R::default()
                }),
                rule(R {
                    name: "nucleus backing-raising before u",
                    target: &[(Syllabic, Plus), (Low, Plus)],
                    change: &[(Low, Minus), (Tense, Plus), (Back, Plus), (Round, Plus)],
                    right: &[(Syllabic, Plus), (High, Plus), (Back, Plus)],
                    ..R::default()
                }),
                rule(R {
                    name: "offglide absorption",
                    target: &[(Syllabic, Plus), (High, Plus)],
                    delete: true,
                    left: &[(Syllabic, Plus), (High, Minus), (Low, Minus)],
                    ..R::default()
                }),
            ],
        ),
    ]
}

pub fn catalog() -> &'static [CatalogEntry] {
    static CAT: OnceLock<Vec<CatalogEntry>> = OnceLock::new();
    CAT.get_or_init(build)
}

pub fn catalog_entry(id: &str) -> Option<&'static CatalogEntry> {
    catalog().iter().find(|e| e.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::derive_ipa;

    #[test]
    fn ids_are_unique() {
        let cat = catalog();
        for (i, a) in cat.iter().enumerate() {
            assert!(cat[i + 1..].iter().all(|b| b.id != a.id), "dup id {}", a.id);
        }
    }

    #[test]
    fn final_devoicing_end_to_end() {
        let e = catalog_entry("final-devoicing").unwrap();
        assert_eq!(derive_ipa("bad", &e.rules).unwrap(), "bat");
        assert_eq!(derive_ipa("dab", &e.rules).unwrap(), "dap");
    }

    #[test]
    fn palatalization_end_to_end() {
        let e = catalog_entry("velar-palatalization").unwrap();
        assert_eq!(derive_ipa("kima", &e.rules).unwrap(), "cima");
        assert_eq!(derive_ipa("kuma", &e.rules).unwrap(), "kuma");
    }

    #[test]
    fn monophthongization_end_to_end() {
        let e = catalog_entry("monophthongization").unwrap();
        assert_eq!(derive_ipa("kai", &e.rules).unwrap(), "ke");
        assert_eq!(derive_ipa("tau", &e.rules).unwrap(), "to");
    }

    #[test]
    fn rhotacism_end_to_end() {
        let e = catalog_entry("rhotacism").unwrap();
        assert_eq!(derive_ipa("asa", &e.rules).unwrap(), "ara");
        // Word-initial s untouched (not intervocalic).
        assert_eq!(derive_ipa("sata", &e.rules).unwrap(), "sata");
    }

    #[test]
    fn l_vocalization_end_to_end() {
        let e = catalog_entry("l-vocalization").unwrap();
        assert_eq!(derive_ipa("sal", &e.rules).unwrap(), "sau");
    }

    #[test]
    fn every_predicate_is_satisfiable_by_the_universal_inventory() {
        let universal: Vec<phon::Segment> = phon::universal_inventory().to_vec();
        for e in catalog() {
            assert!(
                e.applicable_when.holds(&universal),
                "{} can never be offered",
                e.id
            );
        }
    }
}
