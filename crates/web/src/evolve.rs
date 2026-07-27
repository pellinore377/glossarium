//! The evolve flow: daughters, rule chains, and the workbench.
//!
//! A daughter language stores no lexicon — only its parent pointer and an
//! ordered chain of sound changes (the `sound_changes` table). Everything
//! it "has" is derived on demand from the proto-lexicon through every
//! chain between the proto and itself. This module owns that walk, the
//! workbench UI for building chains, and the before/after previews.

use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use maud::html;
use phon::Segment;
use sca::{catalog, Rule};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    error::AppError,
    lexicon::LexemeRow,
    phonology::{owned_language_with_phonology, require_user, Phonology},
    routes::Language,
    state::AppState,
    views,
};

// ---------- Ancestry ----------

/// Every rule between the family's proto-language and `language`,
/// oldest generation first, plus the proto itself.
pub(crate) async fn proto_and_chain(
    state: &AppState,
    user_id: i64,
    language: &Language,
) -> Result<(Language, Vec<Rule>), AppError> {
    let mut generations: Vec<Vec<Rule>> = Vec::new();
    let mut cur = language.clone();
    while let Some(parent_id) = cur.parent_id {
        generations.push(rules_of(state, cur.id).await?);
        cur = fetch_language(state, user_id, parent_id).await?;
    }
    generations.reverse();
    Ok((cur, generations.into_iter().flatten().collect()))
}

async fn fetch_language(state: &AppState, user_id: i64, id: i64) -> Result<Language, AppError> {
    sqlx::query_as::<_, Language>(
        "SELECT l.id, l.project_id, l.parent_id, l.name
         FROM languages l JOIN projects p ON p.id = l.project_id
         WHERE l.id = ? AND p.user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError(anyhow!("language {id} not found for this user")))
}

/// One language's own chain, in order. Each row's rule_json is the
/// Vec<Rule> bundle of one catalog entry.
async fn rules_of(state: &AppState, language_id: i64) -> Result<Vec<Rule>, AppError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT rule_json FROM sound_changes WHERE language_id = ? ORDER BY order_index",
    )
    .bind(language_id)
    .fetch_all(&state.db)
    .await?;
    let mut rules = Vec::new();
    for (json,) in rows {
        let bundle: Vec<Rule> = serde_json::from_str(&json)?;
        rules.extend(bundle);
    }
    Ok(rules)
}

pub(crate) async fn proto_lexemes(
    state: &AppState,
    proto_id: i64,
) -> Result<Vec<LexemeRow>, AppError> {
    Ok(sqlx::query_as::<_, LexemeRow>(
        "SELECT id, gloss, form_ipa, pos, notes FROM lexemes
         WHERE language_id = ? ORDER BY gloss COLLATE NOCASE",
    )
    .bind(proto_id)
    .fetch_all(&state.db)
    .await?)
}

/// The daughter's current segment inventory, for applicability
/// predicates: every segment appearing in the derived lexicon — or, for
/// a family without a seeded lexicon yet, the proto phonology's symbols
/// pushed through the chain one at a time.
fn derived_inventory(
    lexemes: &[LexemeRow],
    chain: &[Rule],
    proto_phonology: &Phonology,
) -> Vec<Segment> {
    let mut seen: Vec<Segment> = Vec::new();
    let mut add_word = |w: phon::Word| {
        for seg in w.segments {
            if !seen.iter().any(|s| s.ipa == seg.ipa) {
                seen.push(seg);
            }
        }
    };
    if lexemes.is_empty() {
        for sym in proto_phonology
            .consonants
            .iter()
            .chain(&proto_phonology.vowels)
            .chain(&proto_phonology.diphthongs)
        {
            if let Some(w) = sca::derive_word(sym, chain) {
                add_word(w);
            }
        }
    } else {
        for l in lexemes {
            if let Some(w) = sca::derive_word(&l.form_ipa, chain) {
                add_word(w);
            }
        }
    }
    seen
}

/// Glosses for the workbench touchstone: common, concrete words that
/// show off what a chain does. Whichever exist in the proto-lexicon are
/// used, up to six.
const TOUCHSTONE_GLOSSES: &[&str] = &[
    "water", "fire", "stone, rock", "night", "dog", "name", "eye", "star",
];

fn touchstone<'a>(lexemes: &'a [LexemeRow]) -> Vec<&'a LexemeRow> {
    TOUCHSTONE_GLOSSES
        .iter()
        .filter_map(|g| lexemes.iter().find(|l| l.gloss == *g))
        .take(6)
        .collect()
}

fn touchstone_line(words: &[&LexemeRow], rules: &[Rule]) -> String {
    words
        .iter()
        .map(|l| sca::derive_ipa(&l.form_ipa, rules).unwrap_or_else(|| l.form_ipa.clone()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The daughter's displayable phonology: derived inventory split back
/// into chart symbols, with romanization re-materialized on top of the
/// proto's map. Computed, never stored — like everything else about a
/// daughter.
pub(crate) async fn derived_display_phonology(
    state: &AppState,
    user: &crate::auth::User,
    language: &Language,
) -> Result<Phonology, AppError> {
    let (proto, chain) = proto_and_chain(state, user.id, language).await?;
    let (_, proto_phonology) = owned_language_with_phonology(state, user, proto.id).await?;
    let lexemes = proto_lexemes(state, proto.id).await?;
    let inventory = derived_inventory(&lexemes, &chain, &proto_phonology);

    let is_vowel = |s: &Segment| {
        s.features.get(&phon::Feature::Syllabic).copied() == Some(phon::FeatureValue::Plus)
    };
    let mut consonants: Vec<String> = inventory
        .iter()
        .filter(|s| !is_vowel(s))
        .map(|s| s.ipa.clone())
        .collect();
    consonants.sort_by_key(|s| crate::ipa_chart::consonant_order(s));
    let mut vowels: Vec<String> = inventory
        .iter()
        .filter(|s| is_vowel(s))
        .map(|s| s.ipa.clone())
        .collect();
    vowels.sort_by_key(|s| crate::ipa_chart::vowel_order(s));

    // A proto diphthong survives as a diphthong if its derived form is
    // still two vowels of the current inventory.
    let diphthongs: Vec<String> = proto_phonology
        .diphthongs
        .iter()
        .filter_map(|d| sca::derive_ipa(d, &chain))
        .filter(|d| {
            let cs: Vec<String> = d.chars().map(|c| c.to_string()).collect();
            cs.len() == 2 && cs.iter().all(|c| vowels.contains(c))
        })
        .collect();

    let mut romanization = proto_phonology.romanization.clone();
    crate::romanization::materialize(&mut romanization, &consonants, &vowels, &diphthongs);

    Ok(Phonology {
        aesthetic: None,
        consonants,
        vowels,
        diphthongs,
        syllable: proto_phonology.syllable,
        onset_clusters: None,
        coda_clusters: None,
        onset_singles: None,
        coda_singles: None,
        stress: proto_phonology.stress.clone(),
        romanization,
    })
}

// ---------- Daughter creation ----------

#[derive(Deserialize)]
pub struct EvolveForm {
    name: String,
}

/// POST /languages/{id}/evolve
pub async fn create_daughter(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<EvolveForm>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (parent, parent_phonology) = owned_language_with_phonology(&state, &user, id).await?;
    let name = form.name.trim();
    if name.is_empty() {
        return Ok(Redirect::to(&format!("/languages/{}", parent.id)).into_response());
    }
    // The daughter starts with a copy of the parent's phonology blob so
    // romanization keeps working; its real inventory is always derived.
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO languages (project_id, parent_id, name, phonology)
         VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(parent.project_id)
    .bind(parent.id)
    .bind(name)
    .bind(serde_json::to_string(&parent_phonology)?)
    .fetch_one(&state.db)
    .await?;
    Ok(Redirect::to(&format!("/languages/{}/changes", row.0)).into_response())
}

// ---------- Workbench ----------

fn naturalness_dots(n: f32) -> String {
    let filled = (n * 5.0).round() as usize;
    "●".repeat(filled.min(5)) + &"○".repeat(5 - filled.min(5))
}

/// GET /languages/{id}/changes
pub async fn changes_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, _) = owned_language_with_phonology(&state, &user, id).await?;
    if language.parent_id.is_none() {
        // Protos don't have chains; they ARE the chain's starting point.
        return Ok(Redirect::to(&format!("/languages/{}", language.id)).into_response());
    }
    let (proto, chain) = proto_and_chain(&state, user.id, &language).await?;
    let (_, proto_phonology) = owned_language_with_phonology(&state, &user, proto.id).await?;
    let lexemes = proto_lexemes(&state, proto.id).await?;
    let inventory = derived_inventory(&lexemes, &chain, &proto_phonology);

    let own_chain: Vec<(i64, i64, Option<String>, String)> = sqlx::query_as(
        "SELECT id, order_index, catalog_ref, notes FROM sound_changes
         WHERE language_id = ? ORDER BY order_index",
    )
    .bind(language.id)
    .fetch_all(&state.db)
    .await?;

    // A change already in the chain isn't offered again — adopting
    // "intervocalic voicing" five times is noise, not phonology. (True
    // re-application after an intervening change is a later refinement.)
    let adopted: Vec<&str> = own_chain
        .iter()
        .filter_map(|(_, _, cref, _)| cref.as_deref())
        .collect();
    let mut offered: Vec<&sca::CatalogEntry> = catalog::catalog()
        .iter()
        .filter(|e| !adopted.contains(&e.id.as_str()))
        .filter(|e| e.applicable_when.holds(&inventory))
        .collect();
    offered.sort_by(|a, b| b.naturalness.partial_cmp(&a.naturalness).unwrap());

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) } class="muted" { "← " (language.name) }
        }
        h1 { (language.name) ": sound changes" }
        p {
            (language.name) " is " (proto.name) " plus everything listed "
            "below, applied in order. Order matters — a rule can feed or "
            "starve the ones after it."
        }
        @let stone = touchstone(&lexemes);
        @if !stone.is_empty() {
            div.warnbox {
                p.eyebrow { "Touchstone — six words, before and after the chain" }
                p.ph { (proto.name) ":  /" (touchstone_line(&stone, &[])) "/" }
                p.ph { (language.name) ":  /" (touchstone_line(&stone, &chain)) "/" }
                p.muted style="font-size:.82rem" {
                    "(" (stone.iter().map(|l| l.gloss.as_str()).collect::<Vec<_>>().join(", ")) ")"
                }
            }
        }

        h2 { "The chain so far" }
        @if own_chain.is_empty() {
            div.empty {
                "No changes yet. " (language.name) " currently sounds "
                "identical to its parent — adopt a change from the menu "
                "below and watch the lexicon drift."
            }
        } @else {
            ol.chain {
                @for (cid, _, catalog_ref, notes) in &own_chain {
                    li {
                        @let name = catalog_ref
                            .as_deref()
                            .and_then(catalog::catalog_entry)
                            .map(|e| e.display_name.as_str())
                            .unwrap_or(notes.as_str());
                        span { (name) }
                        form.inline style="margin:0;display:inline" method="post"
                            action={ "/languages/" (language.id) "/changes/" (cid) "/delete" } {
                            button.mini.quiet type="submit" { "remove" }
                        }
                    }
                }
            }
        }

        h2 { "Available changes" }
        @if lexemes.is_empty() {
            p.warn {
                "The proto-language has no lexicon yet, so previews are "
                "empty — seed " (proto.name) "'s lexicon to see real "
                "before/after forms."
            }
        }
        p.muted style="font-size:.9rem" {
            "Only changes that could touch " (language.name) "'s current "
            "sounds are listed. Dots are cross-linguistic frequency: "
            "●●●●● happens in every second language family, ●○○○○ is a "
            "rarity worth a footnote."
        }
        div #preview {}
        ul.presets {
            @for e in &offered {
                li {
                    form method="post" style="display:inline"
                        action={ "/languages/" (language.id) "/changes/add" } {
                        input type="hidden" name="entry" value=(e.id);
                        button type="submit" { "Adopt" }
                    }
                    " "
                    button.quiet
                        hx-get={ "/languages/" (language.id) "/changes/preview/" (e.id) }
                        hx-target="#preview"
                        hx-swap="innerHTML"
                    { "Preview" }
                    span style="margin-left:.7rem;font-weight:500" { (e.display_name) }
                    span.muted style="margin-left:.6rem;font-size:.8rem" {
                        (naturalness_dots(e.naturalness))
                    }
                    p.muted style="margin:.45rem 0 0" { (e.description) }
                }
            }
        }

        h2 { "Generate" }
        p.muted style="font-size:.9rem" {
            "When the chain feels right, generate " (language.name) " — "
            "you'll land on its home page with the full derived lexicon "
            "one click away. You can always come back and add or remove "
            "changes; the lexicon re-derives every time."
        }
        form.inline method="get" action={ "/languages/" (language.id) } {
            button type="submit" { "Generate " (language.name) " →" }
        }
    };
    Ok(views::layout("Sound changes", Some(&user), body).into_response())
}

/// GET /languages/{id}/changes/preview/{entry} (HTMX)
pub async fn preview_change(
    State(state): State<AppState>,
    session: Session,
    Path((id, entry_id)): Path<(i64, String)>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, _) = owned_language_with_phonology(&state, &user, id).await?;
    let Some(entry) = catalog::catalog_entry(&entry_id) else {
        return Ok(html! {}.into_response());
    };
    let (proto, chain) = proto_and_chain(&state, user.id, &language).await?;
    let lexemes = proto_lexemes(&state, proto.id).await?;

    let mut extended = chain.clone();
    extended.extend(entry.rules.iter().cloned());

    let mut samples: Vec<(String, String, String)> = Vec::new();
    let mut touched = 0usize;
    for l in &lexemes {
        let before = sca::derive_ipa(&l.form_ipa, &chain).unwrap_or_else(|| l.form_ipa.clone());
        let after = sca::derive_ipa(&l.form_ipa, &extended).unwrap_or_else(|| l.form_ipa.clone());
        if before != after {
            touched += 1;
            if samples.len() < 10 {
                samples.push((l.gloss.clone(), before, after));
            }
        }
    }

    let stone = touchstone(&lexemes);
    let markup = html! {
        div.warnbox {
            p.eyebrow { (entry.display_name) " — preview" }
            @if !stone.is_empty() {
                p.ph {
                    "/" (touchstone_line(&stone, &chain)) "/ → /"
                    (touchstone_line(&stone, &extended)) "/"
                }
            }
            @if samples.is_empty() {
                p.ok {
                    "This change wouldn't touch a single current form. It "
                    "can still be adopted — later changes might feed it — "
                    "but right now it's a no-op."
                }
            } @else {
                p.muted style="font-size:.9rem" {
                    (touched) " of " (lexemes.len()) " forms would change:"
                }
                @for (gloss, before, after) in &samples {
                    p.ph style="margin:.15rem 0" {
                        span.muted style="font-family:inherit" { "“" (gloss) "”  " }
                        "/" (before) "/ → /" (after) "/"
                    }
                }
            }
        }
    };
    Ok(markup.into_response())
}

#[derive(Deserialize)]
pub struct AddChange {
    entry: String,
}

/// POST /languages/{id}/changes/add
pub async fn add_change(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<AddChange>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, _) = owned_language_with_phonology(&state, &user, id).await?;
    if language.parent_id.is_some() {
        if let Some(entry) = catalog::catalog_entry(&form.entry) {
            sqlx::query(
                "INSERT INTO sound_changes (language_id, order_index, catalog_ref, rule_json, notes)
                 VALUES (?,
                         (SELECT COALESCE(MAX(order_index), 0) + 1
                          FROM sound_changes WHERE language_id = ?),
                         ?, ?, ?)",
            )
            .bind(language.id)
            .bind(language.id)
            .bind(&entry.id)
            .bind(serde_json::to_string(&entry.rules)?)
            .bind(&entry.display_name)
            .execute(&state.db)
            .await?;
        }
    }
    Ok(Redirect::to(&format!("/languages/{}/changes", language.id)).into_response())
}

/// POST /languages/{id}/changes/{cid}/delete
pub async fn delete_change(
    State(state): State<AppState>,
    session: Session,
    Path((id, cid)): Path<(i64, i64)>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, _) = owned_language_with_phonology(&state, &user, id).await?;
    sqlx::query("DELETE FROM sound_changes WHERE id = ? AND language_id = ?")
        .bind(cid)
        .bind(language.id)
        .execute(&state.db)
        .await?;
    Ok(Redirect::to(&format!("/languages/{}/changes", language.id)).into_response())
}
