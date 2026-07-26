//! Phonology core: segments as distinctive feature bundles.
//!
//! Design contract (see project notes): phonemes are NEVER bare IPA strings
//! anywhere past the input boundary. IPA is parsed into a [`Segment`] on the
//! way in, and every sound-change rule in the `sca` crate matches against
//! feature matrices, not glyphs. This is what makes applicability predicates
//! ("this language has back vowels and a following /i/ or /j/, so i-umlaut
//! is on the menu") possible at all.

pub mod data;
pub use data::{
    effective_eq, parse_universal, resolve, resolve_nearest, universal_inventory,
};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Ternary feature value. `Unspecified` matters: rules that don't mention a
/// feature must not accidentally constrain it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureValue {
    Plus,
    Minus,
    Unspecified,
}

/// The distinctive feature set for v1 (concatenative, no tone).
/// Deliberately a closed enum rather than free strings so rule files
/// fail loudly on typos at deserialization time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    // Major class
    Consonantal,
    Sonorant,
    Syllabic,
    // Laryngeal
    Voice,
    SpreadGlottis,
    ConstrictedGlottis,
    // Manner
    Continuant,
    Nasal,
    Lateral,
    DelayedRelease,
    // Place
    Labial,
    Coronal,
    Anterior,
    Distributed,
    Dorsal,
    // Vowel space
    High,
    Low,
    Back,
    Round,
    Tense,
    Long,
}

impl Feature {
    /// Every feature, for effective-equality sweeps in resolution.
    pub const ALL: [Feature; 21] = [
        Feature::Consonantal,
        Feature::Sonorant,
        Feature::Syllabic,
        Feature::Voice,
        Feature::SpreadGlottis,
        Feature::ConstrictedGlottis,
        Feature::Continuant,
        Feature::Nasal,
        Feature::Lateral,
        Feature::DelayedRelease,
        Feature::Labial,
        Feature::Coronal,
        Feature::Anterior,
        Feature::Distributed,
        Feature::Dorsal,
        Feature::High,
        Feature::Low,
        Feature::Back,
        Feature::Round,
        Feature::Tense,
        Feature::Long,
    ];
}

/// A single segment: its canonical IPA rendering plus its feature bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    /// Canonical IPA form, e.g. "kʷ". Display only — never matched against.
    pub ipa: String,
    pub features: BTreeMap<Feature, FeatureValue>,
}

impl Segment {
    pub fn get(&self, f: Feature) -> FeatureValue {
        self.features
            .get(&f)
            .copied()
            .unwrap_or(FeatureValue::Unspecified)
    }

    /// Does this segment satisfy every constraint in `pattern`?
    /// Unspecified constraints match anything.
    pub fn matches(&self, pattern: &BTreeMap<Feature, FeatureValue>) -> bool {
        pattern.iter().all(|(f, want)| match want {
            FeatureValue::Unspecified => true,
            _ => self.get(*f) == *want,
        })
    }
}

/// A phonological word: an ordered sequence of segments.
/// Syllabification and stress live here in the next milestone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Word {
    pub segments: Vec<Segment>,
}

impl Word {
    pub fn ipa(&self) -> String {
        self.segments.iter().map(|s| s.ipa.as_str()).collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PhonError {
    #[error("unknown IPA sequence: {0}")]
    UnknownIpa(String),
}

/// Parse an IPA string into segments using the given inventory
/// (longest-match, so "tʃ" wins over "t" + "ʃ" when both exist).
pub fn parse_ipa(input: &str, inventory: &[Segment]) -> Result<Word, PhonError> {
    let mut sorted: Vec<&Segment> = inventory.iter().collect();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.ipa.chars().count()));

    let mut rest = input;
    let mut segments = Vec::new();
    'outer: while !rest.is_empty() {
        for seg in &sorted {
            if let Some(r) = rest.strip_prefix(seg.ipa.as_str()) {
                segments.push((*seg).clone());
                rest = r;
                continue 'outer;
            }
        }
        return Err(PhonError::UnknownIpa(rest.to_string()));
    }
    Ok(Word { segments })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(ipa: &str, feats: &[(Feature, FeatureValue)]) -> Segment {
        Segment {
            ipa: ipa.to_string(),
            features: feats.iter().copied().collect(),
        }
    }

    #[test]
    fn longest_match_wins() {
        use Feature::*;
        use FeatureValue::*;
        let inv = vec![
            seg("t", &[(Consonantal, Plus)]),
            seg("ʃ", &[(Consonantal, Plus)]),
            seg("tʃ", &[(Consonantal, Plus), (DelayedRelease, Plus)]),
            seg("a", &[(Syllabic, Plus)]),
        ];
        let w = parse_ipa("tʃa", &inv).unwrap();
        assert_eq!(w.segments.len(), 2);
        assert_eq!(w.segments[0].ipa, "tʃ");
    }

    #[test]
    fn pattern_matching_ignores_unspecified() {
        use Feature::*;
        use FeatureValue::*;
        let t = seg("t", &[(Consonantal, Plus), (Voice, Minus)]);
        let want: BTreeMap<_, _> = [(Consonantal, Plus)].into_iter().collect();
        assert!(t.matches(&want));
        let want_voiced: BTreeMap<_, _> = [(Voice, Plus)].into_iter().collect();
        assert!(!t.matches(&want_voiced));
    }
}
