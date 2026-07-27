//! Romanization: phoneme → spelling suggestions and collision checking.
//!
//! Pure map logic, no handlers. The stored form is a `BTreeMap<String,
//! String>` inside the phonology blob; this module materializes it
//! (suggest spellings for new phonemes, prune removed ones) and audits it
//! (duplicate spellings, empty spellings). Suggestions are conventions,
//! not rules — every cell on the page is editable.

use std::collections::BTreeMap;

/// Conventional latinizations for symbols that aren't already ASCII.
/// Lax vowels get grave accents; retroflexes get underdots; the rest lean
/// on the digraphs a linguistics reader would guess first (sh, ng, kh…).
const SUGGESTIONS: &[(&str, &str)] = &[
    // Plosives
    ("ʈ", "ṭ"),
    ("ɖ", "ḍ"),
    ("c", "ty"),
    ("ɟ", "gy"),
    ("ɡ", "g"),
    ("ɢ", "ġ"),
    ("ʔ", "'"),
    // Nasals
    ("ɱ", "ṃ"),
    ("ɳ", "ṇ"),
    ("ɲ", "ny"),
    ("ŋ", "ng"),
    ("ɴ", "ṅ"),
    // Trills, taps
    ("ʙ", "bb"),
    ("ʀ", "rr"),
    ("ⱱ", "vr"),
    ("ɾ", "r"),
    ("ɽ", "ṛ"),
    // Fricatives
    ("ɸ", "ph"),
    ("β", "bh"),
    ("θ", "th"),
    ("ð", "dh"),
    ("ʃ", "sh"),
    ("ʒ", "zh"),
    ("ʂ", "ṣ"),
    ("ʐ", "ẓ"),
    ("ç", "ch"),
    ("ʝ", "jh"),
    ("x", "kh"),
    ("ɣ", "gh"),
    ("χ", "x̱"),
    ("ʁ", "ǧ"),
    ("ħ", "ḥ"),
    ("ʕ", "ʻ"),
    ("ɦ", "hh"),
    ("ɬ", "lh"),
    ("ɮ", "dl"),
    // Approximants, laterals
    ("ʋ", "v"),
    ("ɹ", "r"),
    ("ɻ", "ṟ"),
    ("j", "y"),
    ("ɰ", "ğ"),
    ("ɭ", "ḷ"),
    ("ʎ", "ly"),
    ("ʟ", "ḻ"),
    ("ʍ", "wh"),
    ("ɥ", "ẅ"),
    // Vowels
    ("y", "ü"),
    ("ɨ", "î"),
    ("ʉ", "û"),
    ("ɯ", "ı"),
    ("ɪ", "ì"),
    ("ʏ", "ỳ"),
    ("ʊ", "ù"),
    ("ø", "ö"),
    ("ɘ", "ė"),
    ("ɵ", "ô"),
    ("ɤ", "ơ"),
    ("ə", "ë"),
    ("ɛ", "è"),
    ("œ", "œ"),
    ("ɜ", "ě"),
    ("ɞ", "ő"),
    ("ʌ", "â"),
    ("ɔ", "ò"),
    ("æ", "ä"),
    ("ɐ", "ă"),
    ("ɶ", "ǽ"),
    ("ɑ", "å"),
    ("ɒ", "ǫ"),
];

pub fn suggest(sym: &str) -> String {
    SUGGESTIONS
        .iter()
        .find(|(s, _)| *s == sym)
        .map(|(_, r)| r.to_string())
        .unwrap_or_else(|| sym.to_string())
}

/// Bring the stored map in line with the inventory: drop entries whose
/// phoneme is gone, suggest spellings for phonemes that lack one.
/// Diphthongs are filled last so they can concatenate their components'
/// (possibly hand-edited) spellings. Returns true if anything changed.
pub fn materialize(
    map: &mut BTreeMap<String, String>,
    consonants: &[String],
    vowels: &[String],
    diphthongs: &[String],
) -> bool {
    let mut changed = false;

    let in_inventory = |k: &str| {
        consonants.iter().any(|s| s == k)
            || vowels.iter().any(|s| s == k)
            || diphthongs.iter().any(|s| s == k)
    };
    let stale: Vec<String> = map
        .keys()
        .filter(|k| !in_inventory(k))
        .cloned()
        .collect();
    for k in stale {
        map.remove(&k);
        changed = true;
    }

    for sym in consonants.iter().chain(vowels) {
        if !map.contains_key(sym) {
            map.insert(sym.clone(), suggest(sym));
            changed = true;
        }
    }
    for d in diphthongs {
        if !map.contains_key(d) {
            let spelling: String = d
                .chars()
                .map(|c| {
                    let s = c.to_string();
                    map.get(&s).cloned().unwrap_or(s)
                })
                .collect();
            map.insert(d.clone(), spelling);
            changed = true;
        }
    }
    changed
}

/// Spell an IPA form using the language's romanization map: greedy
/// longest-match, so two-character diphthong keys win over their
/// component vowels. Unmapped characters pass through unchanged.
pub fn romanize(form: &str, map: &BTreeMap<String, String>) -> String {
    let chars: Vec<char> = form.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() {
            let two: String = chars[i..i + 2].iter().collect();
            if let Some(sp) = map.get(&two) {
                out.push_str(sp);
                i += 2;
                continue;
            }
        }
        let one = chars[i].to_string();
        match map.get(&one) {
            Some(sp) => out.push_str(sp),
            None => out.push_str(&one),
        }
        i += 1;
    }
    out
}

/// Audit the map: spellings doing double duty, and phonemes spelled as
/// nothing. `ordered` fixes presentation order (chart order upstream).
pub fn warnings(map: &BTreeMap<String, String>, ordered: &[String]) -> Vec<String> {
    let mut out = Vec::new();

    let unspelled: Vec<String> = ordered
        .iter()
        .filter(|s| map.get(*s).is_some_and(|v| v.is_empty()))
        .map(|s| format!("/{s}/"))
        .collect();
    if !unspelled.is_empty() {
        out.push(format!(
            "{} spelled as nothing — those phonemes will simply vanish \
             from romanized text.",
            unspelled.join(", ")
        ));
    }

    // Group phonemes by spelling, preserving chart order within groups.
    let mut seen: Vec<&str> = Vec::new();
    for sym in ordered {
        let Some(spelling) = map.get(sym) else { continue };
        if spelling.is_empty() || seen.contains(&spelling.as_str()) {
            continue;
        }
        let sharers: Vec<String> = ordered
            .iter()
            .filter(|s| map.get(*s) == Some(spelling))
            .map(|s| format!("/{s}/"))
            .collect();
        if sharers.len() > 1 {
            out.push(format!(
                "⟨{spelling}⟩ is doing double duty: {}. Readers cope with \
                 ambiguity (English does fine), but the dictionary gets \
                 harder to skim.",
                sharers.join(" and ")
            ));
            seen.push(spelling.as_str());
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn ascii_symbols_spell_themselves() {
        assert_eq!(suggest("p"), "p");
        assert_eq!(suggest("ʃ"), "sh");
    }

    #[test]
    fn materialize_fills_prunes_and_concatenates() {
        let mut map = BTreeMap::new();
        map.insert("q".to_string(), "q".to_string()); // stale
        let changed = materialize(
            &mut map,
            &s(&["ʃ"]),
            &s(&["a", "i"]),
            &s(&["ai"]),
        );
        assert!(changed);
        assert!(!map.contains_key("q"));
        assert_eq!(map.get("ʃ").unwrap(), "sh");
        assert_eq!(map.get("ai").unwrap(), "ai");
    }

    #[test]
    fn diphthong_spelling_follows_hand_edits() {
        let mut map = BTreeMap::new();
        map.insert("a".to_string(), "a".to_string());
        map.insert("i".to_string(), "j".to_string());
        materialize(&mut map, &[], &s(&["a", "i"]), &s(&["ai"]));
        assert_eq!(map.get("ai").unwrap(), "aj");
    }

    #[test]
    fn romanize_prefers_diphthong_keys() {
        let mut map = BTreeMap::new();
        map.insert("ʃ".to_string(), "sh".to_string());
        map.insert("a".to_string(), "a".to_string());
        map.insert("i".to_string(), "i".to_string());
        map.insert("ai".to_string(), "ay".to_string());
        assert_eq!(romanize("ʃai", &map), "shay");
        assert_eq!(romanize("ʃia", &map), "shia");
        assert_eq!(romanize("q", &map), "q"); // unmapped passes through
    }

    #[test]
    fn collisions_and_empties_flagged() {
        let mut map = BTreeMap::new();
        map.insert("ʃ".to_string(), "sh".to_string());
        map.insert("ʂ".to_string(), "sh".to_string());
        map.insert("x".to_string(), String::new());
        let ordered = s(&["ʃ", "ʂ", "x"]);
        let w = warnings(&map, &ordered);
        assert!(w.iter().any(|w| w.contains("double duty")));
        assert!(w.iter().any(|w| w.contains("vanish")));
    }
}
