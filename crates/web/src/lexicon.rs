//! The proto-lexicon: seeding, browsing, and editing.
//!
//! Seeding is deterministic — the generator seed is derived from the
//! language id, so wiping and reseeding an untouched lexicon reproduces
//! it exactly. Forms are stored as bare segmental IPA (no stress marks,
//! no syllable dots): with fixed stress those are predictable, and the
//! sound-change engine wants clean segment strings.

use anyhow::anyhow;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use lex::gen::{Generator, WordSpec};
use lex::Pos;
use maud::{html, Markup};
use serde::Deserialize;
use std::collections::BTreeMap;
use tower_sessions::Session;

use crate::{
    error::AppError,
    phonology::{owned_language_with_phonology, require_user, Phonology},
    romanization,
    state::AppState,
    views,
};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LexemeRow {
    pub id: i64,
    pub gloss: String,
    pub form_ipa: String,
    pub pos: String,
    pub notes: String,
}

/// Generator seed for a language: fixed salt × language id. Stable across
/// restarts, distinct across languages.
fn lexicon_seed(language_id: i64) -> u64 {
    0x476C_6F73_7361_7269u64 ^ (language_id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

fn word_spec(phonology: &Phonology, language_id: i64) -> WordSpec {
    let syl = phonology.syllable.unwrap_or_default();
    WordSpec {
        consonants: phonology.consonants.clone(),
        vowels: phonology.vowels.clone(),
        diphthongs: phonology.diphthongs.clone(),
        onset_min: syl.onset_min,
        onset_max: syl.onset_max,
        coda_min: syl.coda_min,
        coda_max: syl.coda_max,
        seed: lexicon_seed(language_id),
    }
}

async fn lexeme_count(state: &AppState, language_id: i64) -> Result<i64, AppError> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM lexemes WHERE language_id = ?")
        .bind(language_id)
        .fetch_one(&state.db)
        .await?;
    Ok(n)
}

// ---------- Rows ----------

fn pos_options(selected: &str) -> Markup {
    html! {
        @for p in Pos::ALL {
            option value=(p.as_str()) selected[p.as_str() == selected] { (p.abbrev()) }
        }
    }
}

fn display_row(l: &LexemeRow, rom: &BTreeMap<String, String>) -> Markup {
    let abbrev = Pos::parse(&l.pos).map(Pos::abbrev).unwrap_or("?");
    html! {
        tr id={ "lex-" (l.id) } {
            td.gloss { (l.gloss) }
            td.ph { "/" (l.form_ipa) "/" }
            td.ph { "⟨" (romanization::romanize(&l.form_ipa, rom)) "⟩" }
            td.muted { (abbrev) }
            td.muted.notes { (l.notes) }
            td.actions {
                button.mini.quiet
                    hx-get={ "/lexemes/" (l.id) "/edit" }
                    hx-target={ "#lex-" (l.id) }
                    hx-swap="outerHTML"
                { "edit" }
                button.mini.quiet
                    hx-post={ "/lexemes/" (l.id) "/delete" }
                    hx-confirm={ "Delete \"" (l.gloss) "\"?" }
                    hx-target={ "#lex-" (l.id) }
                    hx-swap="outerHTML"
                { "delete" }
            }
        }
    }
}

fn edit_row(l: &LexemeRow) -> Markup {
    html! {
        tr id={ "lex-" (l.id) } {
            td colspan="6" {
                form.rowedit
                    hx-post={ "/lexemes/" (l.id) }
                    hx-target={ "#lex-" (l.id) }
                    hx-swap="outerHTML"
                {
                    input type="text" name="gloss" value=(l.gloss) required;
                    input.ph type="text" name="form_ipa" value=(l.form_ipa) required;
                    select name="pos" { (pos_options(&l.pos)) }
                    input type="text" name="notes" value=(l.notes) placeholder="notes";
                    button.mini type="submit" { "save" }
                    button.mini.quiet type="button"
                        hx-get={ "/lexemes/" (l.id) "/row" }
                        hx-target={ "#lex-" (l.id) }
                        hx-swap="outerHTML"
                    { "cancel" }
                }
            }
        }
    }
}

async fn fetch_rows(
    state: &AppState,
    language_id: i64,
    q: &str,
) -> Result<Vec<LexemeRow>, AppError> {
    let rows = if q.is_empty() {
        sqlx::query_as::<_, LexemeRow>(
            "SELECT id, gloss, form_ipa, pos, notes FROM lexemes
             WHERE language_id = ? ORDER BY gloss COLLATE NOCASE",
        )
        .bind(language_id)
        .fetch_all(&state.db)
        .await?
    } else {
        let like = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        sqlx::query_as::<_, LexemeRow>(
            "SELECT id, gloss, form_ipa, pos, notes FROM lexemes
             WHERE language_id = ?
               AND (gloss LIKE ? ESCAPE '\\'
                    OR form_ipa LIKE ? ESCAPE '\\'
                    OR notes LIKE ? ESCAPE '\\')
             ORDER BY gloss COLLATE NOCASE",
        )
        .bind(language_id)
        .bind(&like)
        .bind(&like)
        .bind(&like)
        .fetch_all(&state.db)
        .await?
    };
    Ok(rows)
}

fn rows_fragment(rows: &[LexemeRow], rom: &BTreeMap<String, String>) -> Markup {
    html! {
        @if rows.is_empty() {
            tr { td.muted colspan="6" style="text-align:center;padding:1rem" {
                "Nothing matches."
            } }
        }
        @for l in rows {
            (display_row(l, rom))
        }
    }
}

// ---------- Pages ----------

/// GET /languages/{id}/lexicon
pub async fn lexicon_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;

    // Daughters own no lexemes: their dictionary is the proto-lexicon
    // pushed through every sound change between there and here.
    if language.parent_id.is_some() {
        return derived_lexicon_page(&state, &user, &language, &phonology).await;
    }
    let count = lexeme_count(&state, language.id).await?;

    let body = if count == 0 {
        let total = lex::seed_concepts().count();
        let ready = !phonology.vowels.is_empty();
        html! {
            p.eyebrow { a href={ "/languages/" (language.id) } class="muted" { "← " (language.name) } }
            h1 { "Lexicon" }
            p {
                "Every family starts with a seed lexicon: the hundred "
                "concepts of the Leipzig–Jakarta list — vocabulary "
                "empirically least likely to be borrowed, which makes it "
                "the best anchor for cognates — plus a second hundred of "
                "kinship, landscape, and everyday verbs. Forms are built "
                "from " (language.name) "'s own phonology: its inventory, "
                "its syllable shape, its phoneme frequencies."
            }
            @if ready {
                form.inline method="post" action={ "/languages/" (language.id) "/lexicon/seed" } {
                    button type="submit" { "Seed " (total) " proto-roots" }
                }
                p.muted style="font-size:.9rem" {
                    "Deterministic: the same language always seeds the same "
                    "words. Edit or delete freely afterwards — the seed is "
                    "a starting point, not a cage."
                }
            } @else {
                div.empty {
                    "The generator needs a phonology first — at minimum "
                    "some vowels. "
                    a href={ "/languages/" (language.id) "/phonology" } {
                        "Open the phonology wizard →"
                    }
                }
            }
        }
    } else {
        let rows = fetch_rows(&state, language.id, "").await?;
        html! {
            p.eyebrow { a href={ "/languages/" (language.id) } class="muted" { "← " (language.name) } }
            h1 { "Lexicon" }
            p.eyebrow { (count) " entr" @if count == 1 { "y" } @else { "ies" } }
            div.lexbar {
                input type="search" name="q" placeholder="Search gloss, form, notes…"
                    hx-get={ "/languages/" (language.id) "/lexicon/search" }
                    hx-trigger="input changed delay:250ms, search"
                    hx-target="#lexbody"
                    hx-swap="innerHTML";
            }
            form.addlex method="post" action={ "/languages/" (language.id) "/lexicon" } {
                input type="text" name="gloss" placeholder="gloss (meaning)" required;
                input.ph type="text" name="form_ipa" placeholder="IPA form" required;
                select name="pos" { (pos_options("noun")) }
                input type="text" name="notes" placeholder="notes";
                button type="submit" { "Add" }
            }
            div.chart-scroll {
                table.lex {
                    thead {
                        tr {
                            th { "gloss" } th { "form" } th { "romanized" }
                            th { "pos" } th { "notes" } th {}
                        }
                    }
                    tbody #lexbody {
                        (rows_fragment(&rows, &phonology.romanization))
                    }
                }
            }
        }
    };
    Ok(views::layout("Lexicon", Some(&user), body).into_response())
}

/// The read-only derived dictionary for a daughter language.
async fn derived_lexicon_page(
    state: &AppState,
    user: &crate::auth::User,
    language: &crate::routes::Language,
    phonology: &Phonology,
) -> Result<Response, AppError> {
    let (proto, chain) = crate::evolve::proto_and_chain(state, user.id, language).await?;
    let lexemes = crate::evolve::proto_lexemes(state, proto.id).await?;
    let derived: Vec<(&LexemeRow, String)> = lexemes
        .iter()
        .map(|l| {
            let d = sca::derive_ipa(&l.form_ipa, &chain).unwrap_or_else(|| l.form_ipa.clone());
            (l, d)
        })
        .collect();
    let changed = derived.iter().filter(|(l, d)| l.form_ipa != *d).count();

    let body = html! {
        p.eyebrow { a href={ "/languages/" (language.id) } class="muted" { "← " (language.name) } }
        h1 { (language.name) ": derived lexicon" }
        @if lexemes.is_empty() {
            div.empty {
                "The proto-language " (proto.name) " has no lexicon yet — "
                "seed it there, and every daughter inherits instantly."
            }
        } @else {
            p.eyebrow {
                (changed) " of " (lexemes.len()) " forms differ from " (proto.name)
            }
            p.muted style="font-size:.9rem" {
                "Nothing here is stored. Each form is " (proto.name) "'s "
                "root run through the chain — change the chain (or the "
                "proto's lexicon) and this page follows."
            }
            div.chart-scroll {
                table.lex {
                    thead {
                        tr {
                            th { "gloss" } th { (proto.name) } th { (language.name) }
                            th { "romanized" } th { "pos" }
                        }
                    }
                    tbody {
                        @for (l, d) in &derived {
                            tr {
                                td.gloss { (l.gloss) }
                                td.ph.muted { "/" (l.form_ipa) "/" }
                                td.ph {
                                    @if l.form_ipa == *d {
                                        span.muted { "/" (d) "/" }
                                    } @else {
                                        strong { "/" (d) "/" }
                                    }
                                }
                                td.ph { "⟨" (romanization::romanize(d, &phonology.romanization)) "⟩" }
                                td.muted { (Pos::parse(&l.pos).map(Pos::abbrev).unwrap_or("?")) }
                            }
                        }
                    }
                }
            }
        }
        form.inline method="get" action={ "/languages/" (language.id) "/changes" } {
            button type="submit" { "Edit the sound changes →" }
        }
    };
    Ok(views::layout("Derived lexicon", Some(user), body).into_response())
}

/// POST /languages/{id}/lexicon/seed
pub async fn seed_lexicon(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;

    // Seeding is for protos with an empty lexicon only; daughters derive.
    if language.parent_id.is_none() && lexeme_count(&state, language.id).await? == 0 {
        if let Ok(mut generator) = Generator::new(word_spec(&phonology, language.id)) {
            let mut tx = state.db.begin().await?;
            for concept in lex::seed_concepts() {
                let form = generator.word();
                let concept_ids = serde_json::to_string(&[concept.concept_id])?;
                sqlx::query(
                    "INSERT INTO lexemes (language_id, form_ipa, gloss, concept_ids, pos)
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(language.id)
                .bind(&form)
                .bind(concept.gloss)
                .bind(&concept_ids)
                .bind(concept.pos.as_str())
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
        }
    }
    Ok(Redirect::to(&format!("/languages/{}/lexicon", language.id)).into_response())
}

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    q: String,
}

/// GET /languages/{id}/lexicon/search (HTMX)
pub async fn search_lexicon(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Query(query): Query<SearchQuery>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;
    let rows = fetch_rows(&state, language.id, query.q.trim()).await?;
    Ok(rows_fragment(&rows, &phonology.romanization).into_response())
}

#[derive(Deserialize)]
pub struct LexemeForm {
    gloss: String,
    form_ipa: String,
    pos: String,
    #[serde(default)]
    notes: String,
}

/// POST /languages/{id}/lexicon — add an entry.
pub async fn create_lexeme(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<LexemeForm>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, _) = owned_language_with_phonology(&state, &user, id).await?;

    let gloss = form.gloss.trim();
    let form_ipa = form.form_ipa.trim();
    let pos = Pos::parse(&form.pos).unwrap_or(Pos::Noun);
    if !gloss.is_empty() && !form_ipa.is_empty() && language.parent_id.is_none() {
        sqlx::query(
            "INSERT INTO lexemes (language_id, form_ipa, gloss, pos, notes)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(language.id)
        .bind(form_ipa)
        .bind(gloss)
        .bind(pos.as_str())
        .bind(form.notes.trim())
        .execute(&state.db)
        .await?;
    }
    Ok(Redirect::to(&format!("/languages/{}/lexicon", language.id)).into_response())
}

// ---------- Row operations (HTMX) ----------

/// A lexeme with its owning language's romanization map, ownership-checked.
async fn owned_lexeme(
    state: &AppState,
    user_id: i64,
    lexeme_id: i64,
) -> Result<(LexemeRow, BTreeMap<String, String>), AppError> {
    let row: Option<(i64, String, String, String, String, String)> = sqlx::query_as(
        "SELECT x.id, x.gloss, x.form_ipa, x.pos, x.notes, l.phonology
         FROM lexemes x
         JOIN languages l ON l.id = x.language_id
         JOIN projects p ON p.id = l.project_id
         WHERE x.id = ? AND p.user_id = ?",
    )
    .bind(lexeme_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;
    let (id, gloss, form_ipa, pos, notes, phon_json) =
        row.ok_or_else(|| AppError(anyhow!("lexeme {lexeme_id} not found for this user")))?;
    let phonology: Phonology = serde_json::from_str(&phon_json).unwrap_or_default();
    Ok((
        LexemeRow {
            id,
            gloss,
            form_ipa,
            pos,
            notes,
        },
        phonology.romanization,
    ))
}

/// GET /lexemes/{id}/edit
pub async fn lexeme_edit_row(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (lexeme, _) = owned_lexeme(&state, user.id, id).await?;
    Ok(edit_row(&lexeme).into_response())
}

/// GET /lexemes/{id}/row — cancel an edit.
pub async fn lexeme_display_row(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (lexeme, rom) = owned_lexeme(&state, user.id, id).await?;
    Ok(display_row(&lexeme, &rom).into_response())
}

/// POST /lexemes/{id} — save an edit, return the refreshed display row.
pub async fn update_lexeme(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<LexemeForm>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (mut lexeme, rom) = owned_lexeme(&state, user.id, id).await?;

    let gloss = form.gloss.trim();
    let form_ipa = form.form_ipa.trim();
    if !gloss.is_empty() && !form_ipa.is_empty() {
        let pos = Pos::parse(&form.pos).unwrap_or(Pos::Noun);
        sqlx::query("UPDATE lexemes SET gloss = ?, form_ipa = ?, pos = ?, notes = ? WHERE id = ?")
            .bind(gloss)
            .bind(form_ipa)
            .bind(pos.as_str())
            .bind(form.notes.trim())
            .bind(lexeme.id)
            .execute(&state.db)
            .await?;
        lexeme.gloss = gloss.to_string();
        lexeme.form_ipa = form_ipa.to_string();
        lexeme.pos = pos.as_str().to_string();
        lexeme.notes = form.notes.trim().to_string();
    }
    Ok(display_row(&lexeme, &rom).into_response())
}

/// POST /lexemes/{id}/delete — remove the row from the table.
pub async fn delete_lexeme(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (lexeme, _) = owned_lexeme(&state, user.id, id).await?;
    sqlx::query("DELETE FROM lexemes WHERE id = ?")
        .bind(lexeme.id)
        .execute(&state.db)
        .await?;
    // Empty response: HTMX swaps the row away.
    Ok(html! {}.into_response())
}
