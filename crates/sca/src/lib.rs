//! Sound change engine.
//!
//! A daughter language is its parent plus an ordered [`RuleChain`]. Derived
//! forms are computed, never authored. Rule order is semantic (feeding /
//! bleeding), so a chain is a Vec, not a set.
//!
//! The catalog of well-attested changes ships as data files (TOML/JSON), not
//! Rust source — each catalog entry carries an [`Applicability`] predicate so
//! the UI can offer only changes that make sense for the language at hand.

pub mod catalog;

use phon::{Feature, FeatureValue, Segment, Word};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type FeaturePattern = BTreeMap<Feature, FeatureValue>;

/// Where in the word/syllable an environment element must sit.
/// v1 keeps this coarse; syllable-aware positions arrive with phonotactics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Boundary {
    WordInitial,
    WordFinal,
    Anywhere,
}

/// One element of a rule environment: match a segment against a pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvSegment {
    pub pattern: FeaturePattern,
}

/// A rewrite rule: target → change / left _ right, bounded by position.
///
/// `change` is a feature *delta*: only the features named are altered.
/// Deletion is `delete: true`; epenthesis and metathesis come later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub target: FeaturePattern,
    #[serde(default)]
    pub change: FeaturePattern,
    #[serde(default)]
    pub delete: bool,
    #[serde(default)]
    pub left: Vec<EnvSegment>,
    #[serde(default)]
    pub right: Vec<EnvSegment>,
    #[serde(default = "Boundary::anywhere")]
    pub boundary: Boundary,
    /// Minimal-word condition: words shorter than this many segments are
    /// exempt. Real erosion works this way — French apocope never turned
    /// /no/ into /n/; content words defend a minimal CV. Zero = no guard.
    #[serde(default)]
    pub min_segments: usize,
}

impl Boundary {
    fn anywhere() -> Self {
        Boundary::Anywhere
    }
}

/// Machine-checkable "does this change make sense for this language?"
/// predicate. Evaluated against the language's current derived inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Applicability {
    /// Inventory contains at least one segment matching the pattern.
    HasSegment { pattern: FeaturePattern },
    /// All sub-predicates must hold (e.g. umlaut: back vowels AND a trigger).
    All { of: Vec<Applicability> },
    /// Any sub-predicate suffices.
    Any { of: Vec<Applicability> },
}

impl Applicability {
    pub fn holds(&self, inventory: &[Segment]) -> bool {
        match self {
            Applicability::HasSegment { pattern } => {
                inventory.iter().any(|s| s.matches(pattern))
            }
            Applicability::All { of } => of.iter().all(|p| p.holds(inventory)),
            Applicability::Any { of } => of.iter().any(|p| p.holds(inventory)),
        }
    }
}

/// A catalog entry: a documented change (or chain-shift bundle — one entry,
/// several rules, applied atomically) plus its applicability predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub display_name: String,
    pub description: String,
    /// Chain shifts are bundles: one catalog entry, N ordered rules.
    pub rules: Vec<Rule>,
    pub applicable_when: Applicability,
    /// 0.0–1.0 cross-linguistic naturalness weight for sorting the menu.
    pub naturalness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleChain {
    pub rules: Vec<Rule>,
}

impl RuleChain {
    /// Apply the whole ordered chain to one word.
    pub fn derive(&self, word: &Word) -> Word {
        let mut w = word.clone();
        for rule in &self.rules {
            w = apply_rule(rule, &w);
        }
        w
    }
}

/// Apply a single rule left-to-right, non-overlapping, one pass.
/// (Iterative-to-fixpoint application is a per-rule flag in a later
/// milestone; one pass is the standard default and always terminates.)
pub fn apply_rule(rule: &Rule, word: &Word) -> Word {
    let segs = &word.segments;
    let n = segs.len();
    if n < rule.min_segments {
        return word.clone();
    }
    let mut out: Vec<Segment> = Vec::with_capacity(n);
    let mut i = 0;

    while i < n {
        let l = rule.left.len();
        let r = rule.right.len();

        let fits = i >= l && i + 1 + r <= n;
        let boundary_ok = match rule.boundary {
            Boundary::Anywhere => true,
            Boundary::WordInitial => i == l && l == 0 || (l > 0 && i == l),
            Boundary::WordFinal => i + 1 + r == n,
        };
        let target_ok = fits && boundary_ok && segs[i].matches(&rule.target);
        let left_ok = target_ok
            && rule
                .left
                .iter()
                .enumerate()
                .all(|(k, e)| segs[i - l + k].matches(&e.pattern));
        let right_ok = left_ok
            && rule
                .right
                .iter()
                .enumerate()
                .all(|(k, e)| segs[i + 1 + k].matches(&e.pattern));

        if right_ok {
            if !rule.delete {
                let mut seg = segs[i].clone();
                for (f, v) in &rule.change {
                    seg.features.insert(*f, *v);
                }
                // IPA rendering of a mutated segment is resolved against the
                // language's inventory at a higher layer; mark it dirty here.
                seg.ipa = format!("~{}", seg.ipa);
                out.push(seg);
            }
            i += 1;
        } else {
            out.push(segs[i].clone());
            i += 1;
        }
    }

    Word { segments: out }
}

/// Resolve every rule-mutated segment (marked `~` by [`apply_rule`]) back
/// to a real glyph: exact feature match against the universal table
/// first, nearest-neighbour for chart gaps, and if even that fails the
/// marker is stripped and the old glyph kept (the change was vacuous at
/// the surface).
pub fn rerender(word: &mut Word) {
    for seg in &mut word.segments {
        if seg.ipa.starts_with('~') {
            match phon::resolve(&seg.features).or_else(|| phon::resolve_nearest(&seg.features)) {
                Some(u) => *seg = u.clone(),
                None => seg.ipa = seg.ipa.trim_start_matches('~').to_string(),
            }
        }
    }
}

/// Parse → apply chain → re-render. `None` when the form contains
/// symbols outside the universal table (hand-typed exotica); callers
/// display the original untouched in that case.
pub fn derive_word(form_ipa: &str, rules: &[Rule]) -> Option<Word> {
    let mut w = phon::parse_universal(form_ipa).ok()?;
    for rule in rules {
        w = apply_rule(rule, &w);
    }
    rerender(&mut w);
    Some(w)
}

pub fn derive_ipa(form_ipa: &str, rules: &[Rule]) -> Option<String> {
    derive_word(form_ipa, rules).map(|w| w.ipa())
}

#[cfg(test)]
mod tests {
    use super::*;
    use phon::{Feature::*, FeatureValue::*};

    fn seg(ipa: &str, feats: &[(Feature, FeatureValue)]) -> Segment {
        Segment {
            ipa: ipa.into(),
            features: feats.iter().copied().collect(),
        }
    }

    #[test]
    fn final_devoicing() {
        let rule = Rule {
            name: "final devoicing".into(),
            target: [(Consonantal, Plus), (Voice, Plus)].into_iter().collect(),
            change: [(Voice, Minus)].into_iter().collect(),
            delete: false,
            left: vec![],
            right: vec![],
            boundary: Boundary::WordFinal,
            min_segments: 0,
        };
        let word = Word {
            segments: vec![
                seg("b", &[(Consonantal, Plus), (Voice, Plus)]),
                seg("a", &[(Syllabic, Plus)]),
                seg("d", &[(Consonantal, Plus), (Voice, Plus)]),
            ],
        };
        let out = apply_rule(&rule, &word);
        assert_eq!(out.segments[0].get(Voice), Plus, "initial b untouched");
        assert_eq!(out.segments[2].get(Voice), Minus, "final d devoiced");
    }

    #[test]
    fn applicability_predicate() {
        let inv = vec![
            seg("k", &[(Consonantal, Plus), (Dorsal, Plus)]),
            seg("i", &[(Syllabic, Plus), (High, Plus), (Back, Minus)]),
        ];
        // Velar palatalization needs a velar AND a front vowel.
        let pred = Applicability::All {
            of: vec![
                Applicability::HasSegment {
                    pattern: [(Dorsal, Plus)].into_iter().collect(),
                },
                Applicability::HasSegment {
                    pattern: [(Syllabic, Plus), (Back, Minus)].into_iter().collect(),
                },
            ],
        };
        assert!(pred.holds(&inv));
    }
}
