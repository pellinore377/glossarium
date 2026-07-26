//! Soft typological warnings over a consonant selection.
//!
//! These never block — Arabic lacks /p/ and is doing fine. They exist so
//! that someone clicking around at random learns *why* the result would be
//! unusual, in terms a grammar sketch would actually use.

use std::collections::HashSet;

const PLOSIVES: &[&str] = &["p", "b", "t", "d", "ʈ", "ɖ", "c", "ɟ", "k", "ɡ", "q", "ɢ", "ʔ"];
const SIBILANTS: &[&str] = &["s", "z", "ʃ", "ʒ", "ʂ", "ʐ"];
const FRICATIVES: &[&str] = &[
    "ɸ", "β", "f", "v", "θ", "ð", "s", "z", "ʃ", "ʒ", "ʂ", "ʐ", "ç", "ʝ", "x", "ɣ", "χ", "ʁ",
    "ħ", "ʕ", "h", "ɦ", "ɬ", "ɮ",
];
const NASALS: &[&str] = &["m", "ɱ", "n", "ɳ", "ɲ", "ŋ", "ɴ"];
const UVULARS: &[&str] = &["q", "ɢ", "ɴ", "ʀ", "χ", "ʁ"];
const VELARS: &[&str] = &["k", "ɡ", "ŋ", "x", "ɣ", "ɰ", "ʟ"];

/// (voiceless, voiced) obstruent pairs for the voicing-implication check.
const VOICE_PAIRS: &[(&str, &str)] = &[
    ("p", "b"),
    ("t", "d"),
    ("ʈ", "ɖ"),
    ("c", "ɟ"),
    ("k", "ɡ"),
    ("q", "ɢ"),
    ("ɸ", "β"),
    ("f", "v"),
    ("θ", "ð"),
    ("s", "z"),
    ("ʃ", "ʒ"),
    ("ʂ", "ʐ"),
    ("ç", "ʝ"),
    ("x", "ɣ"),
    ("χ", "ʁ"),
    ("ɬ", "ɮ"),
];

pub fn consonant_warnings(selected: &[String]) -> Vec<String> {
    let sel: HashSet<&str> = selected.iter().map(|s| s.as_str()).collect();
    let mut warnings = Vec::new();

    if sel.is_empty() {
        return warnings;
    }

    let has_any = |set: &[&str]| set.iter().any(|s| sel.contains(s));

    if !has_any(PLOSIVES) {
        warnings.push(
            "No plosives selected. Every documented language has at least one \
             plosive — usually /p t k/ or a subset."
                .to_string(),
        );
    }

    let orphaned_voiced: Vec<&str> = VOICE_PAIRS
        .iter()
        .filter(|(vl, vd)| sel.contains(vd) && !sel.contains(vl))
        .map(|(vl, _)| *vl)
        .collect();
    if !orphaned_voiced.is_empty() {
        let missing = orphaned_voiced
            .iter()
            .map(|s| format!("/{s}/"))
            .collect::<Vec<_>>()
            .join(", ");
        warnings.push(format!(
            "Voiced obstruents without their voiceless counterparts \
             (missing {missing}). Voicing contrasts almost always imply the \
             voiceless member — the reverse of Arabic-style gaps."
        ));
    }

    if has_any(FRICATIVES) && !has_any(SIBILANTS) {
        warnings.push(
            "Fricatives without any sibilant. Languages that have fricatives \
             nearly always include /s/ or something like it."
                .to_string(),
        );
    }

    if !has_any(NASALS) && sel.len() >= 5 {
        warnings.push(
            "No nasal consonants. Genuinely rare — a handful of Pacific \
             Northwest and Papuan languages manage it, but it's a bold choice."
                .to_string(),
        );
    }

    if sel.contains("ŋ") && !sel.contains("n") {
        warnings.push(
            "/ŋ/ without /n/. A velar nasal virtually always implies a \
             coronal one."
                .to_string(),
        );
    }

    if has_any(UVULARS) && !has_any(VELARS) {
        warnings.push(
            "Uvulars without velars. Cross-linguistically, /q/-type sounds \
             imply /k/-type sounds, not the other way around."
                .to_string(),
        );
    }

    if sel.len() < 6 {
        warnings.push(format!(
            "Only {} consonant(s) — the smallest natural inventories \
             (Rotokas, Pirahã) sit around 6–8. Not wrong, just extreme.",
            sel.len()
        ));
    } else if sel.len() > 40 {
        warnings.push(format!(
            "{} consonants puts this in !Xóõ territory — the far tail of the \
             distribution. Expect the romanization step to get creative.",
            sel.len()
        ));
    }

    warnings
}

const LOW_VOWELS: &[&str] = &["a", "ɶ", "ɑ", "ɒ", "æ", "ɐ"];
const HIGH_VOWELS: &[&str] = &["i", "y", "ɨ", "ʉ", "ɯ", "u", "ɪ", "ʏ", "ʊ"];
const FRONT_ROUNDED: &[&str] = &["y", "ʏ", "ø", "œ", "ɶ"];
const FRONT_UNROUNDED: &[&str] = &["i", "ɪ", "e", "ɛ", "æ", "a"];
const BACK_UNROUNDED_NONLOW: &[&str] = &["ɯ", "ɤ", "ʌ"];
const BACK_ROUNDED: &[&str] = &["u", "ʊ", "o", "ɔ"];

pub fn vowel_warnings(selected: &[String]) -> Vec<String> {
    let sel: HashSet<&str> = selected.iter().map(|s| s.as_str()).collect();
    let mut warnings = Vec::new();
    if sel.is_empty() {
        return warnings;
    }
    let has_any = |set: &[&str]| set.iter().any(|s| sel.contains(s));

    if !has_any(LOW_VOWELS) {
        warnings.push(
            "No open (low) vowel. Virtually every language has an /a/-like \
             vowel anchoring the bottom of the space."
                .to_string(),
        );
    }
    if !has_any(HIGH_VOWELS) && sel.len() >= 2 {
        warnings.push(
            "No close (high) vowels. Vowel systems almost always stretch to \
             the top corners — /i/ and /u/ are the two most common vowels \
             on Earth."
                .to_string(),
        );
    }
    if has_any(FRONT_ROUNDED) && !has_any(FRONT_UNROUNDED) {
        warnings.push(
            "Front rounded vowels without front unrounded ones. /y ø œ/ \
             virtually always imply /i e ɛ/ — rounding is the marked option \
             up front."
                .to_string(),
        );
    }
    if has_any(BACK_UNROUNDED_NONLOW) && !has_any(BACK_ROUNDED) {
        warnings.push(
            "Back unrounded vowels without back rounded ones. /ɯ ɤ ʌ/ \
             normally coexist with (or derive from) /u o ɔ/."
                .to_string(),
        );
    }
    if sel.len() < 3 {
        warnings.push(format!(
            "Only {} vowel(s). The smallest defensible systems have 3 \
             (/i a u/); anything less is contested even for natural \
             languages.",
            sel.len()
        ));
    } else if sel.len() > 14 {
        warnings.push(format!(
            "{} vowel qualities is Germanic-and-beyond territory — \
             expect the romanization step to lean hard on digraphs.",
            sel.len()
        ));
    }
    warnings
}

/// Diphthongs are stored as two-character strings, nucleus + offglide.
pub fn diphthong_warnings(diphthongs: &[String], vowels: &[String]) -> Vec<String> {
    let vset: HashSet<&str> = vowels.iter().map(|s| s.as_str()).collect();
    let mut warnings = Vec::new();
    if diphthongs.is_empty() {
        return warnings;
    }

    let mut orphans: Vec<String> = Vec::new();
    for d in diphthongs {
        let bad = d
            .chars()
            .any(|c| !vset.contains(c.to_string().as_str()));
        if bad {
            orphans.push(format!("/{d}/"));
        }
    }
    if !orphans.is_empty() {
        warnings.push(format!(
            "{} use(s) a vowel no longer in the inventory. They'll still \
             work, but consider re-adding the vowel or dropping the \
             diphthong.",
            orphans.join(", ")
        ));
    }

    let closing = |d: &str| {
        d.chars()
            .last()
            .map(|c| matches!(c, 'i' | 'u' | 'ɪ' | 'ʊ' | 'y' | 'ɯ'))
            .unwrap_or(false)
    };
    let non_closing = diphthongs.iter().filter(|d| !closing(d)).count();
    if non_closing * 2 > diphthongs.len() {
        warnings.push(
            "Most of these don't close toward a high vowel. Closing \
             diphthongs (/ai au ei ou/-types) dominate cross-linguistically; \
             opening ones like /ia ua/ are real but rarer."
                .to_string(),
        );
    }

    warnings
}

#[cfg(test)]
mod vowel_tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn triangle_is_clean() {
        assert!(vowel_warnings(&s(&["i", "u", "a"])).is_empty());
    }

    #[test]
    fn missing_low_flagged() {
        let w = vowel_warnings(&s(&["i", "u", "e", "o"]));
        assert!(w.iter().any(|w| w.contains("open (low)")));
    }

    #[test]
    fn orphan_diphthong_flagged() {
        let w = diphthong_warnings(&s(&["ai", "au"]), &s(&["a", "i"]));
        assert!(w.iter().any(|w| w.contains("no longer in the inventory")));
    }

    #[test]
    fn empty_selection_is_quiet() {
        assert!(consonant_warnings(&[]).is_empty());
    }

    #[test]
    fn orphaned_voiced_flagged() {
        let w = consonant_warnings(&s(&["b", "d", "m", "n", "l", "a"]));
        assert!(w.iter().any(|w| w.contains("voiceless counterparts")));
    }

    #[test]
    fn balanced_inventory_mostly_clean() {
        let w = consonant_warnings(&s(&[
            "p", "t", "k", "b", "d", "ɡ", "s", "m", "n", "r", "l", "j", "w", "h",
        ]));
        assert!(w.is_empty(), "unexpected warnings: {w:?}");
    }
}
