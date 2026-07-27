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
    phonotactics::{self, SyllableStructure, STRESS_PATTERNS, SYLLABLE_PRESETS},
    romanization,
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
    #[serde(default)]
    pub syllable: Option<SyllableStructure>,
    /// Allowed two-consonant sequences in onsets/codas ("pr", "st").
    /// None = sonority defaults not yet materialized by the wizard.
    #[serde(default)]
    pub onset_clusters: Option<Vec<String>>,
    #[serde(default)]
    pub coda_clusters: Option<Vec<String>>,
    /// Which single consonants may appear in each position at all.
    /// None = everything selected on the consonant chart.
    #[serde(default)]
    pub onset_singles: Option<Vec<String>>,
    #[serde(default)]
    pub coda_singles: Option<Vec<String>>,
    /// Allowed coda+onset junctions across syllable boundaries.
    /// None = default heuristic (no geminates, no voicing clashes).
    #[serde(default)]
    pub medial_clusters: Option<Vec<String>>,
    /// Allowed three-consonant windows for clusters of length 3+.
    /// None = any chain of allowed pairs.
    #[serde(default)]
    pub onset_triples: Option<Vec<String>>,
    #[serde(default)]
    pub coda_triples: Option<Vec<String>>,
    #[serde(default)]
    pub stress: Option<String>,
    #[serde(default)]
    pub romanization: std::collections::BTreeMap<String, String>,
}

pub(crate) async fn owned_language_with_phonology(
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

pub(crate) async fn require_user(state: &AppState, session: &Session) -> Result<Result<User, Response>, AppError> {
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
        "clusters",
        "stress",
        "romanization",
        "summary",
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
        h2 { "Other symbols" }
        p.muted style="font-size:.9rem" {
            "Co-articulated consonants — two places at once, so the grid "
            "has no column for them. /w/ lives here."
        }
        div.chart-scroll {
            table.ipa style="min-width:auto;width:auto" {
                tbody {
                    @for (vl, vd, label) in ipa_chart::OTHER_CONSONANTS {
                        tr {
                            th.manner { (label) }
                            td {
                                @if let Some(s) = vl { (sym_button(language.id, s, is_on(s))) }
                                @if let Some(s) = vd { (sym_button(language.id, s, is_on(s))) }
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

// ---------- Vowels ----------

fn vowel_warnings_fragment(selected: &[String]) -> Markup {
    let warnings = typology::vowel_warnings(selected);
    html! {
        div #warnings .warnbox {
            p.eyebrow { (selected.len()) " vowel" @if selected.len() != 1 { "s" } " selected" }
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

fn vowel_button(language_id: i64, sym: &str, on: bool) -> Markup {
    let vals = format!(r#"{{"symbol":"{sym}"}}"#);
    html! {
        button.sym.on[on]
            type="button"
            onclick="this.classList.toggle('on')"
            hx-post={ "/languages/" (language_id) "/phonology/vowels/toggle" }
            hx-vals=(vals)
            hx-target="#warnings"
            hx-swap="outerHTML"
        { (sym) }
    }
}

/// GET /languages/{id}/phonology/vowels
pub async fn vowels_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;
    let is_on = |s: &str| phonology.vowels.iter().any(|x| x == s);

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/phonology/consonants" } class="muted" { "← Consonants" }
        }
        (wizard_steps("vowels"))
        h1 { "Vowels" }
        p {
            "Front vowels sit on the left, back on the right; the space "
            "narrows toward the bottom because the jaw does. Where symbols "
            "share a point, the left is unrounded, the right rounded."
        }
        div.vowel-wrap {
            svg.trap viewBox="0 0 100 100" preserveAspectRatio="none" {
                polygon points="8,7 92,7 92,88 38,88" fill="none"
                    stroke="var(--line)" stroke-width="1"
                    vector-effect="non-scaling-stroke" {}
                line x1="19" y1="34" x2="92" y2="34" stroke="var(--line)"
                    stroke-width="1" vector-effect="non-scaling-stroke" {}
                line x1="29" y1="61" x2="92" y2="61" stroke="var(--line)"
                    stroke-width="1" vector-effect="non-scaling-stroke" {}
                line x1="50" y1="7" x2="63" y2="88" stroke="var(--line)"
                    stroke-width="1" vector-effect="non-scaling-stroke" {}
            }
            @for p in ipa_chart::VOWEL_POINTS {
                div.vpoint style={ "left:" (p.x) "%;top:" (p.y) "%" } {
                    @if let Some(s) = p.unrounded { (vowel_button(language.id, s, is_on(s))) }
                    @if let Some(s) = p.rounded { (vowel_button(language.id, s, is_on(s))) }
                }
            }
        }
        (vowel_warnings_fragment(&phonology.vowels))
        form.inline method="get" action={ "/languages/" (language.id) "/phonology/diphthongs" } {
            button type="submit" { "Continue to diphthongs →" }
        }
    };
    Ok(views::layout("Vowels", Some(&user), body).into_response())
}

/// POST /languages/{id}/phonology/vowels/toggle (HTMX)
pub async fn toggle_vowel(
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

    if ipa_chart::all_vowel_symbols().contains(&form.symbol.as_str()) {
        match phonology.vowels.iter().position(|s| *s == form.symbol) {
            Some(i) => {
                phonology.vowels.remove(i);
            }
            None => phonology.vowels.push(form.symbol.clone()),
        }
        phonology.aesthetic = Some("custom".to_string());
        save_phonology(&state, language.id, &phonology).await?;
    }

    Ok(vowel_warnings_fragment(&phonology.vowels).into_response())
}

// ---------- Diphthongs ----------

fn diphthong_warnings_fragment(diphthongs: &[String], vowels: &[String]) -> Markup {
    let warnings = typology::diphthong_warnings(diphthongs, vowels);
    html! {
        div #warnings .warnbox {
            p.eyebrow {
                (diphthongs.len()) " diphthong" @if diphthongs.len() != 1 { "s" } " selected"
            }
            @if warnings.is_empty() {
                @if !diphthongs.is_empty() {
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

/// GET /languages/{id}/phonology/diphthongs
///
/// Diphthongs aren't chart primitives — the grid is generated from the
/// vowels this language actually selected: rows are the nucleus, columns
/// the offglide.
pub async fn diphthongs_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;

    let mut vowels = phonology.vowels.clone();
    vowels.sort_by_key(|v| ipa_chart::vowel_order(v));
    let is_on = |d: &str| phonology.diphthongs.iter().any(|x| x == d);

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/phonology/vowels" } class="muted" { "← Vowels" }
        }
        (wizard_steps("diphthongs"))
        h1 { "Diphthongs" }
        @if vowels.len() < 2 {
            div.empty {
                "Diphthongs are built from your vowel inventory, and "
                (language.name) " has " (vowels.len()) " vowel(s) so far — "
                "select at least two on the previous page and come back."
            }
        } @else {
            p {
                "Rows are the starting vowel (nucleus), columns the vowel "
                "it glides toward. /ai/-style closing diphthongs are the "
                "cross-linguistic bread and butter; anything else is spice."
            }
            div.chart-scroll {
                table.ipa {
                    thead {
                        tr {
                            th { span.muted { "nucleus ↓ glide →" } }
                            @for g in &vowels { th { (g) } }
                        }
                    }
                    tbody {
                        @for n in &vowels {
                            tr {
                                th.manner { (n) }
                                @for g in &vowels {
                                    @if n == g {
                                        td.x {}
                                    } @else {
                                        @let d = format!("{n}{g}");
                                        td {
                                            button.sym.on[is_on(&d)]
                                                type="button"
                                                onclick="this.classList.toggle('on')"
                                                hx-post={ "/languages/" (language.id) "/phonology/diphthongs/toggle" }
                                                hx-vals=(format!(r#"{{"symbol":"{d}"}}"#))
                                                hx-target="#warnings"
                                                hx-swap="outerHTML"
                                            { (d) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            (diphthong_warnings_fragment(&phonology.diphthongs, &phonology.vowels))
        }
        form.inline method="get" action={ "/languages/" (language.id) "/phonology/phonotactics" } {
            button type="submit" { "Continue to phonotactics →" }
        }
    };
    Ok(views::layout("Diphthongs", Some(&user), body).into_response())
}

/// POST /languages/{id}/phonology/diphthongs/toggle (HTMX)
pub async fn toggle_diphthong(
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

    let chars: Vec<String> = form.symbol.chars().map(|c| c.to_string()).collect();
    let valid = chars.len() == 2
        && chars[0] != chars[1]
        && chars
            .iter()
            .all(|c| ipa_chart::all_vowel_symbols().contains(&c.as_str()));

    if valid {
        match phonology.diphthongs.iter().position(|s| *s == form.symbol) {
            Some(i) => {
                phonology.diphthongs.remove(i);
            }
            None => phonology.diphthongs.push(form.symbol.clone()),
        }
        phonology.aesthetic = Some("custom".to_string());
        save_phonology(&state, language.id, &phonology).await?;
    }

    Ok(diphthong_warnings_fragment(&phonology.diphthongs, &phonology.vowels).into_response())
}

// ---------- Phonotactics ----------

/// The template + warnings block the HTMX builder swaps in place.
fn tactics_fragment(syl: &SyllableStructure, consonant_count: usize) -> Markup {
    let warnings = typology::phonotactics_warnings(syl, consonant_count);
    html! {
        div #tactics-out {
            p.eyebrow { "Syllable template" }
            p.syltemplate { (syl.template()) }
            div.warnbox {
                @if warnings.is_empty() {
                    p.ok { "Nothing typologically alarming so far." }
                } @else {
                    @for w in &warnings {
                        p.warn { (w) }
                    }
                }
            }
        }
    }
}

fn margin_select(name: &str, label: &str, value: u8) -> Markup {
    html! {
        label {
            (label)
            select name=(name) {
                @for n in 0..=phonotactics::MAX_MARGIN {
                    option value=(n) selected[n == value] { (n) }
                }
            }
        }
    }
}

/// GET /languages/{id}/phonology/phonotactics
pub async fn phonotactics_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;
    let syl = phonology.syllable.unwrap_or_default();

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/phonology/diphthongs" } class="muted" { "← Diphthongs" }
        }
        (wizard_steps("phonotactics"))
        h1 { "Syllable structure" }
        p {
            "Every syllable is an onset, a nucleus, and a coda. The nucleus "
            "is always exactly one vowel or diphthong; what you decide here "
            "is how many consonants may crowd in on either side. This single "
            "template shapes every word the lexicon generator will ever "
            "build for " (language.name) "."
        }
        ul.presets {
            @for p in SYLLABLE_PRESETS {
                li {
                    form method="post" action={ "/languages/" (language.id) "/phonology/phonotactics/preset" } {
                        input type="hidden" name="preset" value=(p.id);
                        button type="submit" { (p.name) }
                        span.ph style="margin-left:.6rem" { (p.structure.template()) }
                        @if syl == p.structure {
                            span.muted { " · current" }
                        }
                    }
                    p.muted style="margin:.45rem 0 0" { (p.blurb) }
                }
            }
        }
        h2 { "Or tune it by hand" }
        form.builder
            hx-post={ "/languages/" (language.id) "/phonology/phonotactics/set" }
            hx-trigger="change"
            hx-target="#tactics-out"
            hx-swap="outerHTML"
        {
            (margin_select("onset_min", "Onset min", syl.onset_min))
            (margin_select("onset_max", "Onset max", syl.onset_max))
            span.nucleus { "V" }
            (margin_select("coda_min", "Coda min", syl.coda_min))
            (margin_select("coda_max", "Coda max", syl.coda_max))
        }
        (tactics_fragment(&syl, phonology.consonants.len()))
        form.inline method="get" action={ "/languages/" (language.id) "/phonology/clusters" } {
            button type="submit" { "Continue to clusters →" }
        }
    };
    Ok(views::layout("Phonotactics", Some(&user), body).into_response())
}

#[derive(Deserialize)]
pub struct ChooseSyllablePreset {
    preset: String,
}

/// POST /languages/{id}/phonology/phonotactics/preset
pub async fn choose_syllable_preset(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<ChooseSyllablePreset>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, mut phonology) = owned_language_with_phonology(&state, &user, id).await?;

    if let Some(p) = phonotactics::syllable_preset_by_id(&form.preset) {
        phonology.syllable = Some(p.structure);
        save_phonology(&state, language.id, &phonology).await?;
    }
    // PRG back to the same page: the builder re-renders with the preset's
    // numbers, ready for hand-tuning.
    Ok(Redirect::to(&format!(
        "/languages/{}/phonology/phonotactics",
        language.id
    ))
    .into_response())
}

/// POST /languages/{id}/phonology/phonotactics/set (HTMX)
///
/// Fires on any change to the builder selects; saves and returns the
/// refreshed template + warnings fragment.
pub async fn set_phonotactics(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<SyllableStructure>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, mut phonology) = owned_language_with_phonology(&state, &user, id).await?;

    let syl = form.normalized();
    phonology.syllable = Some(syl);
    save_phonology(&state, language.id, &phonology).await?;

    Ok(tactics_fragment(&syl, phonology.consonants.len()).into_response())
}

// ---------- Consonant clusters ----------

fn cluster_info_fragment(kind: &str, allowed: usize, possible: usize) -> Markup {
    html! {
        p.eyebrow id={ "cinfo-" (kind) } {
            (allowed) " of " (possible) " possible " (kind) " pairs allowed"
        }
    }
}

fn cluster_grid(
    language_id: i64,
    kind: &str,
    consonants: &[String],
    allowed: &[String],
) -> Markup {
    pair_grid(language_id, kind, consonants, consonants, allowed)
}

/// Toggle grid over `rows` × `cols` pairs (rows first). Geminate cells
/// (same symbol) are hatched and unclickable.
fn pair_grid(
    language_id: i64,
    kind: &str,
    rows: &[String],
    cols: &[String],
    allowed: &[String],
) -> Markup {
    let is_on = |pair: &str| allowed.iter().any(|x| x == pair);
    let possible = rows
        .iter()
        .flat_map(|a| cols.iter().map(move |b| (a, b)))
        .filter(|(a, b)| a != b)
        .count();
    html! {
        div.chart-scroll {
            table.ipa {
                thead {
                    tr {
                        th { span.muted { "first ↓ second →" } }
                        @for c in cols { th { (c) } }
                    }
                }
                tbody {
                    @for a in rows {
                        tr {
                            th.manner { (a) }
                            @for b in cols {
                                @if a == b {
                                    td.x {}
                                } @else {
                                    @let pair = format!("{a}{b}");
                                    td {
                                        button.sym.on[is_on(&pair)]
                                            type="button"
                                            onclick="this.classList.toggle('on')"
                                            hx-post={ "/languages/" (language_id) "/phonology/clusters/toggle" }
                                            hx-vals=(format!(r#"{{"kind":"{kind}","pair":"{pair}"}}"#))
                                            hx-target={ "#cinfo-" (kind) }
                                            hx-swap="outerHTML"
                                        { (pair) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        (cluster_info_fragment(kind, allowed.len(), possible))
    }
}

/// Chip list for three-consonant windows: every triple chainable from
/// the allowed pairs, toggleable.
fn triples_row(
    language_id: i64,
    kind: &str,
    candidates: &[String],
    allowed: &[String],
) -> Markup {
    let is_on = |t: &str| allowed.iter().any(|x| x == t);
    html! {
        @if candidates.is_empty() {
            p.muted style="font-size:.9rem" {
                "No three-consonant chains are possible with the current "
                "pair grid — allow more pairs above and revisit."
            }
        } @else {
            div.singlesrow {
                @for t in candidates {
                    button.sym.on[is_on(t)]
                        type="button"
                        onclick="this.classList.toggle('on')"
                        hx-post={ "/languages/" (language_id) "/phonology/clusters/toggle" }
                        hx-vals=(format!(r#"{{"kind":"{kind}","pair":"{t}"}}"#))
                        hx-target={ "#cinfo-" (kind) }
                        hx-swap="outerHTML"
                    { (t) }
                }
            }
            (cluster_info_fragment(kind, allowed.len(), candidates.len()))
        }
    }
}

/// A row of toggle buttons: which consonants may occupy this position.
fn singles_row(
    language_id: i64,
    kind: &str,
    consonants: &[String],
    allowed: &[String],
) -> Markup {
    let is_on = |s: &str| allowed.iter().any(|x| x == s);
    html! {
        div.singlesrow {
            @for c in consonants {
                button.sym.on[is_on(c)]
                    type="button"
                    onclick="this.classList.toggle('on')"
                    hx-post={ "/languages/" (language_id) "/phonology/clusters/single" }
                    hx-vals=(format!(r#"{{"kind":"{kind}","symbol":"{c}"}}"#))
                    hx-target={ "#sinfo-" (kind) }
                    hx-swap="outerHTML"
                { (c) }
            }
        }
        (singles_info_fragment(kind, allowed.len(), consonants.len()))
    }
}

fn singles_info_fragment(kind: &str, allowed: usize, total: usize) -> Markup {
    html! {
        p.eyebrow id={ "sinfo-" (kind) } {
            (allowed) " of " (total) " consonants allowed in " (kind) " position"
        }
    }
}

/// GET /languages/{id}/phonology/clusters
///
/// Positional phonotactics: which consonants may occupy each margin at
/// all, then which pairs may cluster. Defaults are materialized on first
/// visit — every consonant allowed everywhere, clusters from the
/// sonority heuristic — so the page starts sensible rather than empty.
pub async fn clusters_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, mut phonology) = owned_language_with_phonology(&state, &user, id).await?;
    let syl = phonology.syllable.unwrap_or_default();

    let mut consonants = phonology.consonants.clone();
    consonants.sort_by_key(|s| ipa_chart::consonant_order(s));

    let onsets_used = syl.onset_max >= 1 && !consonants.is_empty();
    let codas_used = syl.coda_max >= 1 && !consonants.is_empty();
    let onset_pairs_used = syl.onset_max >= 2 && consonants.len() >= 2;
    let coda_pairs_used = syl.coda_max >= 2 && consonants.len() >= 2;

    let mut changed = false;
    if onsets_used && phonology.onset_singles.is_none() {
        phonology.onset_singles = Some(consonants.clone());
        changed = true;
    }
    if codas_used && phonology.coda_singles.is_none() {
        phonology.coda_singles = Some(consonants.clone());
        changed = true;
    }
    if onset_pairs_used && phonology.onset_clusters.is_none() {
        phonology.onset_clusters = Some(lex::gen::default_pairs(&consonants, true));
        changed = true;
    }
    if coda_pairs_used && phonology.coda_clusters.is_none() {
        phonology.coda_clusters = Some(lex::gen::default_pairs(&consonants, false));
        changed = true;
    }
    let onset_triples_used = syl.onset_max >= 3 && consonants.len() >= 3;
    let coda_triples_used = syl.coda_max >= 3 && consonants.len() >= 3;
    if onset_triples_used && phonology.onset_triples.is_none() {
        let pairs = phonology.onset_clusters.as_deref().unwrap_or(&[]);
        phonology.onset_triples = Some(lex::gen::chain_triples(pairs, &consonants));
        changed = true;
    }
    if coda_triples_used && phonology.coda_triples.is_none() {
        let pairs = phonology.coda_clusters.as_deref().unwrap_or(&[]);
        phonology.coda_triples = Some(lex::gen::chain_triples(pairs, &consonants));
        changed = true;
    }
    // Medial junctions arise whenever codas and onsets both exist.
    let medial_used = onsets_used && codas_used;
    if medial_used && phonology.medial_clusters.is_none() {
        let codas = phonology.coda_singles.as_deref().unwrap_or(&consonants);
        let onsets = phonology.onset_singles.as_deref().unwrap_or(&consonants);
        phonology.medial_clusters = Some(lex::gen::default_medial_pairs(codas, onsets));
        changed = true;
    }
    if changed {
        save_phonology(&state, language.id, &phonology).await?;
    }

    let mut coda_rows = phonology
        .coda_singles
        .clone()
        .unwrap_or_else(|| consonants.clone());
    coda_rows.sort_by_key(|s| ipa_chart::consonant_order(s));
    let mut onset_cols = phonology
        .onset_singles
        .clone()
        .unwrap_or_else(|| consonants.clone());
    onset_cols.sort_by_key(|s| ipa_chart::consonant_order(s));

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/phonology/phonotactics" } class="muted" { "← Syllable structure" }
        }
        (wizard_steps("clusters"))
        h1 { "Consonant positions" }
        @if !onsets_used && !codas_used {
            div.empty {
                "Your syllable template (" (syl.template()) ") has no "
                "consonant slots, so there is nothing to curate here."
            }
        } @else {
            p {
                "Not every consonant goes everywhere. English never begins "
                "a word with /ŋ/; Japanese ends syllables only in nasals; "
                "Spanish tolerates no word-final /p t k/. First decide "
                "which consonants may occupy each position at all — then, "
                "if your template allows clusters, which pairs may touch."
            }
            @if onsets_used {
                h2 { "Syllable-initial consonants" }
                (singles_row(language.id, "onset", &consonants,
                    phonology.onset_singles.as_deref().unwrap_or(&[])))
            }
            @if codas_used {
                h2 { "Syllable-final consonants" }
                (singles_row(language.id, "coda", &consonants,
                    phonology.coda_singles.as_deref().unwrap_or(&[])))
            }
            @if onset_pairs_used {
                h2 { "Onset clusters" }
                p.muted style="font-size:.9rem" {
                    "Pre-filled from sonority: clusters that rise toward "
                    "the vowel (/pr/, /st/). Click to toggle."
                }
                (cluster_grid(language.id, "onset", &consonants,
                    phonology.onset_clusters.as_deref().unwrap_or(&[])))
            }
            @if coda_pairs_used {
                h2 { "Coda clusters" }
                p.muted style="font-size:.9rem" {
                    "Pre-filled falling away from the vowel (/rp/, /nt/)."
                }
                (cluster_grid(language.id, "coda", &consonants,
                    phonology.coda_clusters.as_deref().unwrap_or(&[])))
            }
            @if onset_triples_used {
                h2 { "Onset clusters of three" }
                p.muted style="font-size:.9rem" {
                    "Every chain your pair grid allows, individually "
                    "toggleable — keep /str/, kill /mgl/. Four- and "
                    "five-consonant clusters must pass through these "
                    "windows too."
                }
                (triples_row(language.id, "onset3",
                    &lex::gen::chain_triples(
                        phonology.onset_clusters.as_deref().unwrap_or(&[]), &consonants),
                    phonology.onset_triples.as_deref().unwrap_or(&[])))
            }
            @if coda_triples_used {
                h2 { "Coda clusters of three" }
                (triples_row(language.id, "coda3",
                    &lex::gen::chain_triples(
                        phonology.coda_clusters.as_deref().unwrap_or(&[]), &consonants),
                    phonology.coda_triples.as_deref().unwrap_or(&[])))
            }
            @if medial_used {
                h2 { "Across syllables" }
                p.muted style="font-size:.9rem" {
                    "When one syllable's coda meets the next one's onset "
                    "(the d·t in a word like \"lidtep\"), which pairs are "
                    "tolerable? Pre-filled to ban geminates and "
                    "voicing-mismatched obstruents — the two clashes ears "
                    "notice first. Rows are the coda, columns the "
                    "following onset."
                }
                (pair_grid(language.id, "medial", &coda_rows, &onset_cols,
                    phonology.medial_clusters.as_deref().unwrap_or(&[])))
            }
        }
        form.inline method="get" action={ "/languages/" (language.id) "/phonology/stress" } {
            button type="submit" { "Continue to stress →" }
        }
    };
    Ok(views::layout("Positions", Some(&user), body).into_response())
}

#[derive(Deserialize)]
pub struct ToggleSingle {
    kind: String,
    symbol: String,
}

/// POST /languages/{id}/phonology/clusters/single (HTMX)
pub async fn toggle_single(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<ToggleSingle>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, mut phonology) = owned_language_with_phonology(&state, &user, id).await?;
    let total = phonology.consonants.len();
    let valid = phonology.consonants.iter().any(|x| *x == form.symbol)
        && matches!(form.kind.as_str(), "onset" | "coda");

    let mut allowed = 0usize;
    if valid {
        let list = if form.kind == "onset" {
            phonology.onset_singles.get_or_insert_with(Vec::new)
        } else {
            phonology.coda_singles.get_or_insert_with(Vec::new)
        };
        match list.iter().position(|p| *p == form.symbol) {
            Some(i) => {
                list.remove(i);
            }
            None => list.push(form.symbol.clone()),
        }
        allowed = list.len();
        save_phonology(&state, language.id, &phonology).await?;
    }
    Ok(singles_info_fragment(&form.kind, allowed, total).into_response())
}

#[derive(Deserialize)]
pub struct ToggleCluster {
    kind: String,
    pair: String,
}

/// POST /languages/{id}/phonology/clusters/toggle (HTMX)
pub async fn toggle_cluster(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<ToggleCluster>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, mut phonology) = owned_language_with_phonology(&state, &user, id).await?;

    // Symbols can be multi-codepoint (tʃ), so validate by splitting
    // against the inventory rather than by counting chars.
    let expected_len = if form.kind.ends_with('3') { 3 } else { 2 };
    let valid = lex::gen::split_cluster(&form.pair, &phonology.consonants)
        .map_or(false, |v| v.len() == expected_len);
    let n = phonology.consonants.len();

    if valid
        && matches!(
            form.kind.as_str(),
            "onset" | "coda" | "medial" | "onset3" | "coda3"
        )
    {
        let list = match form.kind.as_str() {
            "onset" => phonology.onset_clusters.get_or_insert_with(Vec::new),
            "coda" => phonology.coda_clusters.get_or_insert_with(Vec::new),
            "onset3" => phonology.onset_triples.get_or_insert_with(Vec::new),
            "coda3" => phonology.coda_triples.get_or_insert_with(Vec::new),
            _ => phonology.medial_clusters.get_or_insert_with(Vec::new),
        };
        match list.iter().position(|p| *p == form.pair) {
            Some(i) => {
                list.remove(i);
            }
            None => list.push(form.pair.clone()),
        }
        let allowed = list.len();
        save_phonology(&state, language.id, &phonology).await?;
        return Ok(
            cluster_info_fragment(&form.kind, allowed, n * n.saturating_sub(1)).into_response(),
        );
    }

    let allowed = match form.kind.as_str() {
        "onset" => phonology.onset_clusters.as_ref().map_or(0, |v| v.len()),
        "coda" => phonology.coda_clusters.as_ref().map_or(0, |v| v.len()),
        _ => phonology.medial_clusters.as_ref().map_or(0, |v| v.len()),
    };
    Ok(cluster_info_fragment(&form.kind, allowed, n * n.saturating_sub(1)).into_response())
}

// ---------- Stress ----------

/// GET /languages/{id}/phonology/stress
pub async fn stress_page(
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
            a href={ "/languages/" (language.id) "/phonology/clusters" } class="muted" { "← Clusters" }
        }
        (wizard_steps("stress"))
        h1 { "Stress" }
        p {
            "Where does the emphasis fall? A fixed stress rule does a lot of "
            "quiet work: it gives every generated word the same rhythmic "
            "signature, and later it gives sound changes a landmark — vowel "
            "reduction and syncope hunt unstressed syllables."
        }
        ul.presets {
            @for p in STRESS_PATTERNS {
                li {
                    form method="post" action={ "/languages/" (language.id) "/phonology/stress" } {
                        input type="hidden" name="pattern" value=(p.id);
                        button type="submit" { (p.name) }
                        span.ph style="margin-left:.6rem" { "[" (p.example) "]" }
                        @if phonology.stress.as_deref() == Some(p.id) {
                            span.muted { " · current" }
                        }
                    }
                    p.muted style="margin:.45rem 0 0" { (p.blurb) }
                }
            }
        }
    };
    Ok(views::layout("Stress", Some(&user), body).into_response())
}

#[derive(Deserialize)]
pub struct ChooseStress {
    pattern: String,
}

/// POST /languages/{id}/phonology/stress
pub async fn choose_stress(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<ChooseStress>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, mut phonology) = owned_language_with_phonology(&state, &user, id).await?;

    if phonotactics::stress_by_id(&form.pattern).is_some() {
        phonology.stress = Some(form.pattern);
        save_phonology(&state, language.id, &phonology).await?;
    }
    Ok(Redirect::to(&format!(
        "/languages/{}/phonology/romanization",
        language.id
    ))
    .into_response())
}

// ---------- Romanization ----------

/// The full inventory in presentation order: consonants (chart order),
/// vowels (chart order), then diphthongs (nucleus-major chart order).
fn ordered_inventory(phonology: &Phonology) -> Vec<String> {
    let mut consonants = phonology.consonants.clone();
    consonants.sort_by_key(|s| ipa_chart::consonant_order(s));
    let mut vowels = phonology.vowels.clone();
    vowels.sort_by_key(|s| ipa_chart::vowel_order(s));
    let mut diphthongs = phonology.diphthongs.clone();
    diphthongs.sort_by_key(|d| {
        let mut ch = d.chars();
        let n = ch.next().map(|c| ipa_chart::vowel_order(&c.to_string()));
        let g = ch.next().map(|c| ipa_chart::vowel_order(&c.to_string()));
        (n, g)
    });
    consonants
        .into_iter()
        .chain(vowels)
        .chain(diphthongs)
        .collect()
}

fn rom_warnings_fragment(phonology: &Phonology) -> Markup {
    let ordered = ordered_inventory(phonology);
    let warnings = romanization::warnings(&phonology.romanization, &ordered);
    html! {
        div #warnings .warnbox {
            @if warnings.is_empty() {
                p.ok { "No collisions — every phoneme reads back unambiguously." }
            } @else {
                @for w in &warnings {
                    p.warn { (w) }
                }
            }
        }
    }
}

fn rom_section(language_id: i64, title: &str, syms: &[String], map: &std::collections::BTreeMap<String, String>) -> Markup {
    html! {
        @if !syms.is_empty() {
            h2 { (title) }
            div.romgrid {
                @for sym in syms {
                    label.romcell {
                        span.psym { "/" (sym) "/" }
                        input.rom type="text" name="spelling"
                            value=(map.get(sym).map(String::as_str).unwrap_or(""))
                            hx-post={ "/languages/" (language_id) "/phonology/romanization/set" }
                            hx-vals=(format!(r#"{{"symbol":"{sym}"}}"#))
                            hx-trigger="change"
                            hx-target="#warnings"
                            hx-swap="outerHTML";
                    }
                }
            }
        }
    }
}

/// GET /languages/{id}/phonology/romanization
///
/// Materializes the map on render: suggests spellings for phonemes that
/// lack one, prunes phonemes that left the inventory, and persists the
/// result so downstream milestones always find a complete map in the DB.
pub async fn romanization_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, mut phonology) = owned_language_with_phonology(&state, &user, id).await?;

    let changed = romanization::materialize(
        &mut phonology.romanization,
        &phonology.consonants,
        &phonology.vowels,
        &phonology.diphthongs,
    );
    if changed {
        save_phonology(&state, language.id, &phonology).await?;
    }

    let mut consonants = phonology.consonants.clone();
    consonants.sort_by_key(|s| ipa_chart::consonant_order(s));
    let mut vowels = phonology.vowels.clone();
    vowels.sort_by_key(|s| ipa_chart::vowel_order(s));
    let mut diphthongs = phonology.diphthongs.clone();
    diphthongs.sort_by_key(|d| {
        let mut ch = d.chars();
        let n = ch.next().map(|c| ipa_chart::vowel_order(&c.to_string()));
        let g = ch.next().map(|c| ipa_chart::vowel_order(&c.to_string()));
        (n, g)
    });

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/phonology/stress" } class="muted" { "← Stress" }
        }
        (wizard_steps("romanization"))
        h1 { "Romanization" }
        p {
            "How " (language.name) " gets written down for human eyes — the "
            "dictionary, the stories, the family tree all use these "
            "spellings. Suggestions follow convention (⟨sh⟩ for /ʃ/, "
            "underdots for retroflexes, grave accents for lax vowels); "
            "edit any cell, and diphthongs inherit their parts."
        }
        (rom_section(language.id, "Consonants", &consonants, &phonology.romanization))
        (rom_section(language.id, "Vowels", &vowels, &phonology.romanization))
        (rom_section(language.id, "Diphthongs", &diphthongs, &phonology.romanization))
        (rom_warnings_fragment(&phonology))
        form.inline method="get" action={ "/languages/" (language.id) "/phonology/summary" } {
            button type="submit" { "Continue to summary →" }
        }
    };
    Ok(views::layout("Romanization", Some(&user), body).into_response())
}

#[derive(Deserialize)]
pub struct SetSpelling {
    symbol: String,
    spelling: String,
}

/// POST /languages/{id}/phonology/romanization/set (HTMX)
pub async fn set_romanization(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<SetSpelling>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, mut phonology) = owned_language_with_phonology(&state, &user, id).await?;

    let in_inventory = phonology.consonants.iter().any(|s| *s == form.symbol)
        || phonology.vowels.iter().any(|s| *s == form.symbol)
        || phonology.diphthongs.iter().any(|s| *s == form.symbol);
    if in_inventory {
        phonology
            .romanization
            .insert(form.symbol, form.spelling.trim().to_string());
        save_phonology(&state, language.id, &phonology).await?;
    }

    Ok(rom_warnings_fragment(&phonology).into_response())
}

// ---------- Summary ----------

/// GET /languages/{id}/phonology/summary
///
/// The wizard's closing page: everything decided, in one place, with a
/// final pass of every warnings engine over the finished system.
pub async fn summary_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;
    let lexemes = crate::lexicon::lexeme_count(&state, language.id).await?;
    let syl = phonology.syllable.unwrap_or_default();
    let stress = phonology
        .stress
        .as_deref()
        .and_then(phonotactics::stress_by_id);

    let mut consonants = phonology.consonants.clone();
    consonants.sort_by_key(|s| ipa_chart::consonant_order(s));
    let mut vowels = phonology.vowels.clone();
    vowels.sort_by_key(|s| ipa_chart::vowel_order(s));

    let spelled = |sym: &str| -> String {
        phonology
            .romanization
            .get(sym)
            .cloned()
            .unwrap_or_else(|| sym.to_string())
    };

    let mut review: Vec<String> = Vec::new();
    review.extend(typology::consonant_warnings(&phonology.consonants));
    review.extend(typology::vowel_warnings(&phonology.vowels));
    review.extend(typology::diphthong_warnings(
        &phonology.diphthongs,
        &phonology.vowels,
    ));
    review.extend(typology::phonotactics_warnings(
        &syl,
        phonology.consonants.len(),
    ));
    review.extend(romanization::warnings(
        &phonology.romanization,
        &ordered_inventory(&phonology),
    ));

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/phonology/romanization" } class="muted" { "← Romanization" }
        }
        (wizard_steps("summary"))
        h1 { (language.name) ": the sound system" }

        h2 { "Consonants (" (consonants.len()) ")" }
        p.ph {
            @for (i, s) in consonants.iter().enumerate() {
                @if i > 0 { "  " }
                "/" (s) "/ ⟨" (spelled(s)) "⟩"
            }
        }
        h2 { "Vowels (" (vowels.len()) ")" }
        p.ph {
            @for (i, s) in vowels.iter().enumerate() {
                @if i > 0 { "  " }
                "/" (s) "/ ⟨" (spelled(s)) "⟩"
            }
        }
        @if !phonology.diphthongs.is_empty() {
            h2 { "Diphthongs (" (phonology.diphthongs.len()) ")" }
            p.ph {
                @for (i, d) in phonology.diphthongs.iter().enumerate() {
                    @if i > 0 { "  " }
                    "/" (d) "/ ⟨" (spelled(d)) "⟩"
                }
            }
        }
        h2 { "Syllables" }
        p.syltemplate { (syl.template()) }
        @if let Some(s) = stress {
            h2 { "Stress" }
            p { (s.name) " — " span.ph { "[" (s.example) "]" } }
        }

        h2 { "Typological review" }
        @if review.is_empty() {
            p.ok {
                "A clean bill of health — nothing here would raise a "
                "field linguist's eyebrow."
            }
        } @else {
            @for w in &review {
                p.warn { (w) }
            }
        }

        @if language.parent_id.is_none() && lexemes == 0 {
            h2 { "Seed the lexicon" }
            p {
                "The last step: " (language.name) " gets its first words. "
                "Two hundred proto-roots — the Leipzig–Jakarta core "
                "vocabulary plus a hundred everyday concepts — generated "
                "from exactly the sound system above."
            }
            form.inline method="post" action={ "/languages/" (language.id) "/lexicon/seed" } {
                button type="submit" { "Seed the lexicon →" }
            }
            p.muted style="font-size:.9rem" {
                a href={ "/languages/" (language.id) } { "Skip for now" }
                " — you can seed later from the Lexicon tab. Any wizard "
                "step can be revisited until then."
            }
        } @else {
            form.inline method="get" action={ "/languages/" (language.id) } {
                button type="submit" { "Finish — back to " (language.name) " →" }
            }
        }
    };
    Ok(views::layout("Summary", Some(&user), body).into_response())
}
