//! The universal segment table: every symbol the wizard's charts can
//! produce, with its distinctive-feature bundle.
//!
//! Feature assignments are pragmatic, not doctrinaire: they exist to
//! (a) distinguish every segment we ship from every other, and (b) make
//! the catalog's rules land on real glyphs after mutation. Bilabials are
//! [+distributed] against labiodentals; trills are [+continuant] against
//! taps; central vowels simply omit [back] (an absent feature reads as
//! Unspecified, which no [±back] pattern matches — exactly the behavior
//! central vowels should have under fronting/backing rules).

use crate::{Feature, FeatureValue, Segment, Word};
use Feature::*;
use FeatureValue::*;
use std::collections::BTreeMap;
use std::sync::OnceLock;

type F = (Feature, FeatureValue);

// Places.
const BILABIAL: &[F] = &[(Labial, Plus), (Distributed, Plus)];
const LABIODENTAL: &[F] = &[(Labial, Plus), (Distributed, Minus)];
const DENTAL: &[F] = &[(Coronal, Plus), (Anterior, Plus), (Distributed, Plus)];
const ALVEOLAR: &[F] = &[(Coronal, Plus), (Anterior, Plus), (Distributed, Minus)];
const POSTALVEOLAR: &[F] = &[(Coronal, Plus), (Anterior, Minus), (Distributed, Plus)];
const RETROFLEX: &[F] = &[(Coronal, Plus), (Anterior, Minus), (Distributed, Minus)];
const PALATAL: &[F] = &[(Dorsal, Plus), (High, Plus), (Back, Minus)];
const VELAR: &[F] = &[(Dorsal, Plus), (High, Plus), (Back, Plus)];
const UVULAR: &[F] = &[(Dorsal, Plus), (High, Minus), (Back, Plus)];
const PHARYNGEAL: &[F] = &[(Dorsal, Plus), (Low, Plus), (Back, Plus)];

// Manners. Every consonant carries an explicit [lateral] value so rules
// can tell /r/ from /l/ in both directions.
const PLOSIVE: &[F] = &[
    (Consonantal, Plus), (Sonorant, Minus), (Continuant, Minus), (Lateral, Minus),
];
const NASAL_M: &[F] = &[
    (Consonantal, Plus), (Sonorant, Plus), (Continuant, Minus), (Nasal, Plus), (Lateral, Minus),
];
const TRILL: &[F] = &[
    (Consonantal, Plus), (Sonorant, Plus), (Continuant, Plus), (Lateral, Minus),
];
const TAP: &[F] = &[
    (Consonantal, Plus), (Sonorant, Plus), (Continuant, Minus), (Lateral, Minus),
];
const FRICATIVE: &[F] = &[
    (Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus), (Lateral, Minus),
];
const LATERAL_FRICATIVE: &[F] = &[
    (Consonantal, Plus), (Sonorant, Minus), (Continuant, Plus), (Lateral, Plus),
];
const APPROXIMANT: &[F] = &[
    (Consonantal, Minus), (Sonorant, Plus), (Continuant, Plus), (Lateral, Minus),
];
const LATERAL_APPROXIMANT: &[F] = &[
    (Consonantal, Plus), (Sonorant, Plus), (Continuant, Plus), (Lateral, Plus),
];

fn seg(ipa: &str, parts: &[&[F]], voiced: bool) -> Segment {
    let mut features: BTreeMap<Feature, FeatureValue> = BTreeMap::new();
    for part in parts {
        for (f, v) in *part {
            features.insert(*f, *v);
        }
    }
    features.insert(Voice, if voiced { Plus } else { Minus });
    Segment {
        ipa: ipa.to_string(),
        features,
    }
}

/// A vowel: height and backness spelled out, `back`/`tense` optional
/// because central vowels omit [back] and the "plain" low/mid vowels
/// (a, ə…) omit [tense].
fn vowel(
    ipa: &str,
    high: FeatureValue,
    low: FeatureValue,
    back: Option<FeatureValue>,
    round: FeatureValue,
    tense: Option<FeatureValue>,
) -> Segment {
    let mut features: BTreeMap<Feature, FeatureValue> = [
        (Syllabic, Plus),
        (Consonantal, Minus),
        (Sonorant, Plus),
        (Continuant, Plus),
        (Voice, Plus),
        (High, high),
        (Low, low),
        (Round, round),
    ]
    .into_iter()
    .collect();
    if let Some(b) = back {
        features.insert(Back, b);
    }
    if let Some(t) = tense {
        features.insert(Tense, t);
    }
    Segment {
        ipa: ipa.to_string(),
        features,
    }
}

fn build_inventory() -> Vec<Segment> {
    let mut inv = Vec::with_capacity(90);

    // Plosives
    inv.push(seg("p", &[BILABIAL, PLOSIVE], false));
    inv.push(seg("b", &[BILABIAL, PLOSIVE], true));
    inv.push(seg("t", &[ALVEOLAR, PLOSIVE], false));
    inv.push(seg("d", &[ALVEOLAR, PLOSIVE], true));
    inv.push(seg("ʈ", &[RETROFLEX, PLOSIVE], false));
    inv.push(seg("ɖ", &[RETROFLEX, PLOSIVE], true));
    inv.push(seg("c", &[PALATAL, PLOSIVE], false));
    inv.push(seg("ɟ", &[PALATAL, PLOSIVE], true));
    inv.push(seg("k", &[VELAR, PLOSIVE], false));
    inv.push(seg("ɡ", &[VELAR, PLOSIVE], true));
    inv.push(seg("q", &[UVULAR, PLOSIVE], false));
    inv.push(seg("ɢ", &[UVULAR, PLOSIVE], true));
    // Glottals: placeless, [-consonantal].
    inv.push(Segment {
        ipa: "ʔ".into(),
        features: [
            (Consonantal, Minus), (Sonorant, Minus), (Continuant, Minus),
            (Lateral, Minus), (ConstrictedGlottis, Plus), (Voice, Minus),
        ].into_iter().collect(),
    });
    // Nasals
    inv.push(seg("m", &[BILABIAL, NASAL_M], true));
    inv.push(seg("ɱ", &[LABIODENTAL, NASAL_M], true));
    inv.push(seg("n", &[ALVEOLAR, NASAL_M], true));
    inv.push(seg("ɳ", &[RETROFLEX, NASAL_M], true));
    inv.push(seg("ɲ", &[PALATAL, NASAL_M], true));
    inv.push(seg("ŋ", &[VELAR, NASAL_M], true));
    inv.push(seg("ɴ", &[UVULAR, NASAL_M], true));
    // Trills
    inv.push(seg("ʙ", &[BILABIAL, TRILL], true));
    inv.push(seg("r", &[ALVEOLAR, TRILL], true));
    inv.push(seg("ʀ", &[UVULAR, TRILL], true));
    // Taps
    inv.push(seg("ⱱ", &[LABIODENTAL, TAP], true));
    inv.push(seg("ɾ", &[ALVEOLAR, TAP], true));
    inv.push(seg("ɽ", &[RETROFLEX, TAP], true));
    // Fricatives
    inv.push(seg("ɸ", &[BILABIAL, FRICATIVE], false));
    inv.push(seg("β", &[BILABIAL, FRICATIVE], true));
    inv.push(seg("f", &[LABIODENTAL, FRICATIVE], false));
    inv.push(seg("v", &[LABIODENTAL, FRICATIVE], true));
    inv.push(seg("θ", &[DENTAL, FRICATIVE], false));
    inv.push(seg("ð", &[DENTAL, FRICATIVE], true));
    inv.push(seg("s", &[ALVEOLAR, FRICATIVE], false));
    inv.push(seg("z", &[ALVEOLAR, FRICATIVE], true));
    inv.push(seg("ʃ", &[POSTALVEOLAR, FRICATIVE], false));
    inv.push(seg("ʒ", &[POSTALVEOLAR, FRICATIVE], true));
    inv.push(seg("ʂ", &[RETROFLEX, FRICATIVE], false));
    inv.push(seg("ʐ", &[RETROFLEX, FRICATIVE], true));
    inv.push(seg("ç", &[PALATAL, FRICATIVE], false));
    inv.push(seg("ʝ", &[PALATAL, FRICATIVE], true));
    inv.push(seg("x", &[VELAR, FRICATIVE], false));
    inv.push(seg("ɣ", &[VELAR, FRICATIVE], true));
    inv.push(seg("χ", &[UVULAR, FRICATIVE], false));
    inv.push(seg("ʁ", &[UVULAR, FRICATIVE], true));
    inv.push(seg("ħ", &[PHARYNGEAL, FRICATIVE], false));
    inv.push(seg("ʕ", &[PHARYNGEAL, FRICATIVE], true));
    inv.push(Segment {
        ipa: "h".into(),
        features: [
            (Consonantal, Minus), (Sonorant, Minus), (Continuant, Plus),
            (Lateral, Minus), (SpreadGlottis, Plus), (Voice, Minus),
        ].into_iter().collect(),
    });
    inv.push(Segment {
        ipa: "ɦ".into(),
        features: [
            (Consonantal, Minus), (Sonorant, Minus), (Continuant, Plus),
            (Lateral, Minus), (SpreadGlottis, Plus), (Voice, Plus),
        ].into_iter().collect(),
    });
    // Lateral fricatives
    inv.push(seg("ɬ", &[ALVEOLAR, LATERAL_FRICATIVE], false));
    inv.push(seg("ɮ", &[ALVEOLAR, LATERAL_FRICATIVE], true));
    // Approximants
    inv.push(seg("ʋ", &[LABIODENTAL, APPROXIMANT], true));
    inv.push(seg("ɹ", &[ALVEOLAR, APPROXIMANT], true));
    inv.push(seg("ɻ", &[RETROFLEX, APPROXIMANT], true));
    inv.push(seg("j", &[PALATAL, APPROXIMANT], true));
    inv.push(seg("ɰ", &[VELAR, APPROXIMANT], true));
    // Lateral approximants
    inv.push(seg("l", &[ALVEOLAR, LATERAL_APPROXIMANT], true));
    inv.push(seg("ɭ", &[RETROFLEX, LATERAL_APPROXIMANT], true));
    inv.push(seg("ʎ", &[PALATAL, LATERAL_APPROXIMANT], true));
    inv.push(seg("ʟ", &[VELAR, LATERAL_APPROXIMANT], true));

    // Vowels: high, low, back (None = central), round, tense.
    inv.push(vowel("i", Plus, Minus, Some(Minus), Minus, Some(Plus)));
    inv.push(vowel("y", Plus, Minus, Some(Minus), Plus, Some(Plus)));
    inv.push(vowel("ɨ", Plus, Minus, None, Minus, Some(Plus)));
    inv.push(vowel("ʉ", Plus, Minus, None, Plus, Some(Plus)));
    inv.push(vowel("ɯ", Plus, Minus, Some(Plus), Minus, Some(Plus)));
    inv.push(vowel("u", Plus, Minus, Some(Plus), Plus, Some(Plus)));
    inv.push(vowel("ɪ", Plus, Minus, Some(Minus), Minus, Some(Minus)));
    inv.push(vowel("ʏ", Plus, Minus, Some(Minus), Plus, Some(Minus)));
    inv.push(vowel("ʊ", Plus, Minus, Some(Plus), Plus, Some(Minus)));
    inv.push(vowel("e", Minus, Minus, Some(Minus), Minus, Some(Plus)));
    inv.push(vowel("ø", Minus, Minus, Some(Minus), Plus, Some(Plus)));
    inv.push(vowel("ɘ", Minus, Minus, None, Minus, Some(Plus)));
    inv.push(vowel("ɵ", Minus, Minus, None, Plus, Some(Plus)));
    inv.push(vowel("ɤ", Minus, Minus, Some(Plus), Minus, Some(Plus)));
    inv.push(vowel("o", Minus, Minus, Some(Plus), Plus, Some(Plus)));
    inv.push(vowel("ə", Minus, Minus, None, Minus, None));
    inv.push(vowel("ɛ", Minus, Minus, Some(Minus), Minus, Some(Minus)));
    inv.push(vowel("œ", Minus, Minus, Some(Minus), Plus, Some(Minus)));
    inv.push(vowel("ɜ", Minus, Minus, None, Minus, Some(Minus)));
    inv.push(vowel("ɞ", Minus, Minus, None, Plus, Some(Minus)));
    inv.push(vowel("ʌ", Minus, Minus, Some(Plus), Minus, Some(Minus)));
    inv.push(vowel("ɔ", Minus, Minus, Some(Plus), Plus, Some(Minus)));
    inv.push(vowel("æ", Minus, Plus, Some(Minus), Minus, Some(Minus)));
    inv.push(vowel("ɐ", Minus, Plus, None, Minus, Some(Minus)));
    inv.push(vowel("a", Minus, Plus, Some(Minus), Minus, None));
    inv.push(vowel("ɶ", Minus, Plus, Some(Minus), Plus, None));
    inv.push(vowel("ɑ", Minus, Plus, Some(Plus), Minus, None));
    inv.push(vowel("ɒ", Minus, Plus, Some(Plus), Plus, None));

    inv
}

/// The universal segment table, built once per process.
pub fn universal_inventory() -> &'static [Segment] {
    static INV: OnceLock<Vec<Segment>> = OnceLock::new();
    INV.get_or_init(build_inventory)
}

/// Do two bundles agree on the *effective* value of every feature?
/// (Absent and explicit `Unspecified` are the same thing.)
pub fn effective_eq(
    a: &BTreeMap<Feature, FeatureValue>,
    b: &BTreeMap<Feature, FeatureValue>,
) -> bool {
    Feature::ALL.iter().all(|f| {
        let va = a.get(f).copied().unwrap_or(Unspecified);
        let vb = b.get(f).copied().unwrap_or(Unspecified);
        va == vb
    })
}

/// The universal segment whose bundle exactly (effectively) matches.
pub fn resolve(features: &BTreeMap<Feature, FeatureValue>) -> Option<&'static Segment> {
    universal_inventory()
        .iter()
        .find(|s| effective_eq(&s.features, features))
}

/// Nearest neighbour when a rule pushes a bundle into a gap in the chart
/// (e.g. voiced dental stop): the closest segment of the same major
/// class, at most two features away. Ties break by table order, which is
/// deterministic — critical for reproducible derivations.
pub fn resolve_nearest(features: &BTreeMap<Feature, FeatureValue>) -> Option<&'static Segment> {
    let effective = |m: &BTreeMap<Feature, FeatureValue>, f: &Feature| {
        m.get(f).copied().unwrap_or(Unspecified)
    };
    let mut best: Option<(&'static Segment, usize)> = None;
    for cand in universal_inventory() {
        if effective(&cand.features, &Syllabic) != effective(features, &Syllabic)
            || effective(&cand.features, &Consonantal) != effective(features, &Consonantal)
        {
            continue;
        }
        let dist = Feature::ALL
            .iter()
            .filter(|f| effective(&cand.features, f) != effective(features, f))
            .count();
        if dist <= 2 && best.map(|(_, d)| dist < d).unwrap_or(true) {
            best = Some((cand, dist));
        }
    }
    best.map(|(s, _)| s)
}

/// Parse an IPA string against the universal table.
pub fn parse_universal(input: &str) -> Result<Word, crate::PhonError> {
    crate::parse_ipa(input, universal_inventory())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_segment_is_distinct() {
        let inv = universal_inventory();
        for (i, a) in inv.iter().enumerate() {
            for b in &inv[i + 1..] {
                assert!(
                    !effective_eq(&a.features, &b.features),
                    "{} and {} share a feature bundle",
                    a.ipa,
                    b.ipa
                );
            }
        }
    }

    #[test]
    fn resolve_roundtrips_every_segment() {
        for s in universal_inventory() {
            assert_eq!(resolve(&s.features).unwrap().ipa, s.ipa);
        }
    }

    #[test]
    fn devoiced_d_resolves_to_t() {
        let d = universal_inventory().iter().find(|s| s.ipa == "d").unwrap();
        let mut mutated = d.features.clone();
        mutated.insert(Voice, Minus);
        assert_eq!(resolve(&mutated).unwrap().ipa, "t");
    }

    #[test]
    fn gap_falls_to_nearest() {
        // Fortited /v/ = voiced labiodental stop: not in the table;
        // nearest is /b/ (distributed mismatch only).
        let v = universal_inventory().iter().find(|s| s.ipa == "v").unwrap();
        let mut mutated = v.features.clone();
        mutated.insert(Continuant, Minus);
        assert!(resolve(&mutated).is_none());
        assert_eq!(resolve_nearest(&mutated).unwrap().ipa, "b");
    }

    #[test]
    fn parses_lexicon_style_forms() {
        let w = parse_universal("tʃai").unwrap_or_else(|_| panic!());
        // No affricate in the table: t + ʃ + a + i.
        assert_eq!(w.segments.len(), 4);
    }
}
