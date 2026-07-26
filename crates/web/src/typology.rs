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

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
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
