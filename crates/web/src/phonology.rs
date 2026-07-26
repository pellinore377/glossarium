//! Phonology wizard: aesthetic → consonants → (vowels → diphthongs →
//! phonotactics → stress → romanization in later passes).
//!
//! Selections persist per-click over HTMX, so a half-finished session
//! survives a closed tab. Any hand edit silently detaches the aesthetic
//! label (it becomes "custom") — presets are starting points, not modes.

use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use maud::{html, Markup};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::{
    auth::{current_user, User},
    error::AppError,
    ipa_chart::{self, Cell, AESTHETICS, CONSONANT_ROWS, PLACES},
    routes::Language,
    state::AppState,
    typology, views,
};

/// The JSON blob stored in `languages.phonology`. Fields default so old
/// rows and future additions deserialize without migrations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Phonology {
    #[serde(default)]
    pub aesthetic: Option<String>,
    #[serde(default)]
    pub consonants: Vec<String>,
    #[serde(default)]
    pub vowels: Vec<String>,
    #[serde(default)]
    pub diphthongs: Vec<String>,
}

async fn owned_language_with_phonology(
    state: &AppState,
    user: &User,
    id: i64,
) -> Result<(Language, Phonology), AppError> {
    let row: Option<(i64, i64, Option<i64>, String, String)> = sqlx::query_as(
        "SELECT l.id, l.project_id, l.parent_id, l.name, l.phonology
         FROM languages l JOIN projects p ON p.id = l.project_id
         WHERE l.id = ? AND p.user_id = ?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;
    let (id, project_id, parent_id, name, phon_json) =
        row.ok_or_else(|| AppError(anyhow!("language {id} not found for this user")))?;
    let phonology: Phonology = serde_json::from_str(&phon_json).unwrap_or_default();
    Ok((
        Language {
            id,
            project_id,
            parent_id,
            name,
        },
        phonology,
    ))
}

async fn save_phonology(state: &AppState, language_id: i64, p: &Phonology) -> Result<(), AppError> {
    sqlx::query("UPDATE languages SET phonology = ? WHERE id = ?")
        .bind(serde_json::to_string(p)?)
        .bind(language_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

async fn require_user(state: &AppState, session: &Session) -> Result<Result<User, Response>, AppError> {
    match current_user(state, session).await? {
        Some(u) => Ok(Ok(u)),
        None => Ok(Err(views::landing().into_response())),
    }
}

fn wizard_steps(current: &str) -> Markup {
    let steps = [
        "aesthetic",
        "consonants",
        "vowels",
        "diphthongs",
        "phonotactics",
        "stress",
        "romanization",
    ];
    html! {
        p.wizsteps {
            @for (i, s) in steps.iter().enumerate() {
                @if i > 0 { " → " }
                @if *s == current { strong { (s) } } @else { (s) }
            }
        }
    }
}

// ---------- Aesthetic page ----------

/// GET /languages/{id}/phonology
pub async fn aesthetic_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;

    let body = html! {
        p.eyebrow { a href={ "/languages/" (language.id) } class="muted" { "← " (language.name) } }
        (wizard_steps("aesthetic"))
        h1 { "How should " (language.name) " sound?" }
        p {
            "Pick an aesthetic to pre-fill the sound charts, or start from a "
            "blank chart. Either way, every phoneme stays hand-editable — "
            "these are starting points, not commitments."
        }
        ul.presets {
            @for a in AESTHETICS {
                li {
                    form method="post" action={ "/languages/" (language.id) "/phonology/aesthetic" } {
                        input type="hidden" name="preset" value=(a.id);
                        button type="submit" { (a.name) }
                        @if phonology.aesthetic.as_deref() == Some(a.id) {
                            span.muted { " · current" }
                        }
                    }
                    p.muted style="margin:.45rem 0 .3rem" { (a.blurb) }
                    p.ph { (a.consonants.join(" ")) "  ·  " (a.vowels.join(" ")) }
                }
            }
            li {
                form method="post" action={ "/languages/" (language.id) "/phonology/aesthetic" } {
                    input type="hidden" name="preset" value="custom";
                    button.quiet type="submit" { "Custom — start from a blank chart" }
                }
                p.muted style="margin:.45rem 0 0" {
                    "Keeps anything you've already selected."
                }
            }
        }
    };
    Ok(views::layout("Aesthetic", Some(&user), body).into_response())
}

#[derive(Deserialize)]
pub struct ChooseAesthetic {
    preset: String,
}

/// POST /languages/{id}/phonology/aesthetic
pub async fn choose_aesthetic(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<ChooseAesthetic>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, mut phonology) = owned_language_with_phonology(&state, &user, id).await?;

    if let Some(a) = ipa_chart::aesthetic_by_id(&form.preset) {
        phonology.aesthetic = Some(a.id.to_string());
        phonology.consonants = a.consonants.iter().map(|s| s.to_string()).collect();
        phonology.vowels = a.vowels.iter().map(|s| s.to_string()).collect();
        phonology.diphthongs.clear();
    } else {
        // "custom": label only; existing selections untouched.
        phonology.aesthetic = Some("custom".to_string());
    }
    save_phonology(&state, language.id, &phonology).await?;
    Ok(Redirect::to(&format!("/languages/{}/phonology/consonants", language.id)).into_response())
}

// ---------- Consonant chart ----------

fn warnings_fragment(selected: &[String]) -> Markup {
    let warnings = typology::consonant_warnings(selected);
    html! {
        div #warnings .warnbox {
            p.eyebrow { (selected.len()) " consonant" @if selected.len() != 1 { "s" } " selected" }
            @if warnings.is_empty() {
                @if !selected.is_empty() {
                    p.ok { "Nothing typologically alarming so far." }
                }
            } @else {
                @for w in &warnings {
                    p.warn { (w) }
                }
            }
        }
    }
}

fn sym_button(language_id: i64, sym: &str, on: bool) -> Markup {
    let vals = format!(r#"{{"symbol":"{sym}"}}"#);
    html! {
        button.sym.on[on]
            type="button"
            onclick="this.classList.toggle('on')"
            hx-post={ "/languages/" (language_id) "/phonology/consonants/toggle" }
            hx-vals=(vals)
            hx-target="#warnings"
            hx-swap="outerHTML"
        { (sym) }
    }
}

/// GET /languages/{id}/phonology/consonants
pub async fn consonants_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;
    let is_on = |s: &str| phonology.consonants.iter().any(|x| x == s);

    let body = html! {
        p.eyebrow { a href={ "/languages/" (language.id) "/phonology" } class="muted" { "← Aesthetic" } }
        (wizard_steps("consonants"))
        h1 { "Consonants" }
        p {
            "Click a symbol to add it to " (language.name) "'s inventory. "
            "Where symbols share a cell, the left one is voiceless, the "
            "right voiced. Hatched cells are articulations judged impossible."
        }
        div.chart-scroll {
            table.ipa {
                thead {
                    tr {
                        th {}
                        @for p in PLACES { th { (p) } }
                    }
                }
                tbody {
                    @for row in CONSONANT_ROWS {
                        tr {
                            th.manner { (row.name) }
                            @for cell in row.cells {
                                @match cell {
                                    Cell::Sounds { span, vl, vd } => {
                                        td colspan=(span) {
                                            @if let Some(s) = vl { (sym_button(language.id, s, is_on(s))) }
                                            @if let Some(s) = vd { (sym_button(language.id, s, is_on(s))) }
                                        }
                                    }
                                    Cell::Shaded { span } => { td.x colspan=(span) {} }
                                    Cell::Empty { span } => { td colspan=(span) {} }
                                }
                            }
                        }
                    }
                }
            }
        }
        (warnings_fragment(&phonology.consonants))
        form.inline method="get" action={ "/languages/" (language.id) "/phonology/vowels" } {
            button type="submit" { "Continue to vowels →" }
        }
    };
    Ok(views::layout("Consonants", Some(&user), body).into_response())
}

#[derive(Deserialize)]
pub struct ToggleSymbol {
    symbol: String,
}

/// POST /languages/{id}/phonology/consonants/toggle (HTMX)
///
/// Persists the toggle, silently detaches the aesthetic, and returns the
/// refreshed warnings fragment.
pub async fn toggle_consonant(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<ToggleSymbol>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, mut phonology) = owned_language_with_phonology(&state, &user, id).await?;

    if ipa_chart::all_consonant_symbols().contains(&form.symbol.as_str()) {
        match phonology.consonants.iter().position(|s| *s == form.symbol) {
            Some(i) => {
                phonology.consonants.remove(i);
            }
            None => phonology.consonants.push(form.symbol.clone()),
        }
        // Silent detachment: hand edits make it "custom".
        phonology.aesthetic = Some("custom".to_string());
        save_phonology(&state, language.id, &phonology).await?;
    }

    Ok(warnings_fragment(&phonology.consonants).into_response())
}

// ---------- Vowels (stub until next pass) ----------

/// GET /languages/{id}/phonology/vowels
pub async fn vowels_stub(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;
    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/phonology/consonants" } class="muted" { "← Consonants" }
        }
        (wizard_steps("vowels"))
        h1 { "Vowels" }
        div.empty {
            "The vowel trapezoid lands in the next pass. Your consonant "
            "selections (" (phonology.consonants.len()) " so far) are saved."
        }
    };
    Ok(views::layout("Vowels", Some(&user), body).into_response())
}
