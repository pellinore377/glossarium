//! Phonotactics and stress: pure data, no handlers.
//!
//! A syllable here is onset + nucleus + coda, with the nucleus fixed at
//! exactly one vowel or diphthong (v1 has no syllabic consonants — the
//! feature system in `phon` can express them, but the wizard doesn't offer
//! them yet). What varies is how many consonants may flank it, expressed
//! as min/max counts and rendered in the familiar `(C)(C)CV(C)` notation:
//! bare C = required, (C) = optional.

use serde::{Deserialize, Serialize};

pub const MAX_MARGIN: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyllableStructure {
    pub onset_min: u8,
    pub onset_max: u8,
    pub coda_min: u8,
    pub coda_max: u8,
}

impl Default for SyllableStructure {
    /// (C)V(C) — the modest middle of the typological road.
    fn default() -> Self {
        Self {
            onset_min: 0,
            onset_max: 1,
            coda_min: 0,
            coda_max: 1,
        }
    }
}

impl SyllableStructure {
    /// Clamp to sane bounds and resolve min > max in favour of min (the
    /// user just raised a minimum past the old maximum; follow them).
    pub fn normalized(self) -> Self {
        let onset_min = self.onset_min.min(MAX_MARGIN);
        let coda_min = self.coda_min.min(MAX_MARGIN);
        Self {
            onset_min,
            onset_max: self.onset_max.clamp(onset_min, MAX_MARGIN),
            coda_min,
            coda_max: self.coda_max.clamp(coda_min, MAX_MARGIN),
        }
    }

    /// `(C)(C)CV(C)(C)`-style template. Optional slots go outside the
    /// required core on both edges.
    pub fn template(&self) -> String {
        let mut s = String::new();
        for _ in 0..(self.onset_max - self.onset_min) {
            s.push_str("(C)");
        }
        for _ in 0..self.onset_min {
            s.push('C');
        }
        s.push('V');
        for _ in 0..self.coda_min {
            s.push('C');
        }
        for _ in 0..(self.coda_max - self.coda_min) {
            s.push_str("(C)");
        }
        s
    }
}

pub struct SyllablePreset {
    pub id: &'static str,
    pub name: &'static str,
    pub blurb: &'static str,
    pub structure: SyllableStructure,
}

pub const SYLLABLE_PRESETS: &[SyllablePreset] = &[
    SyllablePreset {
        id: "open",
        name: "Open & flowing",
        blurb: "Every syllable ends in a vowel. Hawaiian and Māori live \
                here; words come out long, liquid, and easy to sing.",
        structure: SyllableStructure { onset_min: 0, onset_max: 1, coda_min: 0, coda_max: 0 },
    },
    SyllablePreset {
        id: "gentle",
        name: "Gently closed",
        blurb: "A single consonant may close a syllable. Japanese-adjacent \
                up through Swahili — codas exist but never pile up.",
        structure: SyllableStructure { onset_min: 0, onset_max: 1, coda_min: 0, coda_max: 1 },
    },
    SyllablePreset {
        id: "balanced",
        name: "Balanced",
        blurb: "Two-consonant onsets, simple codas — Spanish or Modern \
                Greek. Clusters like /pl tr/ open syllables without \
                clogging their ends.",
        structure: SyllableStructure { onset_min: 0, onset_max: 2, coda_min: 0, coda_max: 1 },
    },
    SyllablePreset {
        id: "cluster",
        name: "Cluster-happy",
        blurb: "Three consonants may stack on either edge — English \
                \"strengths\" territory, with Russian and Georgian waving \
                from further out.",
        structure: SyllableStructure { onset_min: 0, onset_max: 3, coda_min: 0, coda_max: 3 },
    },
];

pub fn syllable_preset_by_id(id: &str) -> Option<&'static SyllablePreset> {
    SYLLABLE_PRESETS.iter().find(|p| p.id == id)
}

// ---------- Stress ----------

pub struct StressPattern {
    pub id: &'static str,
    pub name: &'static str,
    pub blurb: &'static str,
    /// The same nonsense word four times over, stressed per pattern, so
    /// the choices are audible at a glance.
    pub example: &'static str,
}

pub const STRESS_PATTERNS: &[StressPattern] = &[
    StressPattern {
        id: "initial",
        name: "Initial",
        blurb: "Stress the first syllable, always. Finnish, Hungarian, \
                Czech, Icelandic — gives a steady front-loaded drumbeat.",
        example: "ˈta.ki.so.na",
    },
    StressPattern {
        id: "penultimate",
        name: "Penultimate",
        blurb: "Stress the second-to-last syllable. Polish, Swahili, and \
                the broad default of Spanish — probably the single most \
                common fixed pattern.",
        example: "ta.ki.ˈso.na",
    },
    StressPattern {
        id: "final",
        name: "Final",
        blurb: "Stress the last syllable. Turkish, Persian, and French \
                at phrase level — words lean forward into their endings.",
        example: "ta.ki.so.ˈna",
    },
    StressPattern {
        id: "antepenultimate",
        name: "Antepenultimate",
        blurb: "Stress the third-from-last syllable. Macedonian's fixed \
                rule, and the classical flavor of Latin and Greek loans.",
        example: "ta.ˈki.so.na",
    },
];

pub fn stress_by_id(id: &str) -> Option<&'static StressPattern> {
    STRESS_PATTERNS.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_renders_optional_outside_required() {
        let s = SyllableStructure { onset_min: 1, onset_max: 3, coda_min: 0, coda_max: 2 };
        assert_eq!(s.template(), "(C)(C)CV(C)(C)");
    }

    #[test]
    fn bare_v_template() {
        let s = SyllableStructure { onset_min: 0, onset_max: 0, coda_min: 0, coda_max: 0 };
        assert_eq!(s.template(), "V");
    }

    #[test]
    fn normalize_follows_a_raised_min() {
        let s = SyllableStructure { onset_min: 2, onset_max: 1, coda_min: 0, coda_max: 9 }
            .normalized();
        assert_eq!((s.onset_min, s.onset_max), (2, 2));
        assert_eq!(s.coda_max, MAX_MARGIN);
    }
}
