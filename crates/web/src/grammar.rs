//! The grammar wizard, sketch tab, and story realization.
//!
//! The grammar spec lives on the proto (languages.grammar JSON). The
//! wizard materializes a deterministic first draft, then walks through
//! clause structure → nouns & pronouns → verbs → word-building, every
//! generated form editable. Daughters read the proto's grammar and push
//! every realized word — stems, affixed forms, particles, pronouns —
//! through their sound-change chain: grammar erodes exactly like
//! vocabulary.

use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use lex::grammar::{
    attach_prefix, attach_suffix, GrammarSpec, Marking, NegationStrategy, StoryLine,
    WordOrder, STORY, STORY_TITLE,
};
use maud::{html, Markup};
use sca::Rule;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use tower_sessions::Session;

use crate::{
    error::AppError,
    evolve,
    phonology::{owned_language_with_phonology, require_user, Phonology},
    romanization,
    routes::Language,
    state::AppState,
    views,
};

pub(crate) async fn grammar_of(
    state: &AppState,
    language_id: i64,
) -> Result<Option<GrammarSpec>, AppError> {
    let (json,): (String,) = sqlx::query_as("SELECT grammar FROM languages WHERE id = ?")
        .bind(language_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError(anyhow!("language {language_id} not found")))?;
    // '{}' (the column default) and any pre-wizard blob fail to parse —
    // both mean "no grammar yet".
    Ok(serde_json::from_str(&json).ok())
}

async fn save_grammar(
    state: &AppState,
    language_id: i64,
    g: &GrammarSpec,
) -> Result<(), AppError> {
    sqlx::query("UPDATE languages SET grammar = ? WHERE id = ?")
        .bind(serde_json::to_string(g)?)
        .bind(language_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

/// Load the grammar, or materialize the deterministic first draft.
async fn ensure_grammar(
    state: &AppState,
    language: &Language,
    phonology: &Phonology,
) -> Result<GrammarSpec, AppError> {
    if let Some(g) = grammar_of(state, language.id).await? {
        return Ok(g);
    }
    let mut spec = crate::lexicon::word_spec(phonology, language.id);
    spec.seed ^= 0x6772_616D_6D61_7221;
    let g = lex::grammar::generate(spec)
        .map_err(|e| AppError(anyhow!("cannot draft a grammar yet: {e}")))?;
    save_grammar(state, language.id, &g).await?;
    Ok(g)
}

fn derive(form: &str, chain: &[Rule]) -> String {
    sca::derive_ipa(form, chain).unwrap_or_else(|| form.to_string())
}

// ---------- Wizard scaffolding ----------

fn gsteps(language_id: i64, current: &str) -> Markup {
    let steps = ["clauses", "nouns", "verbs", "word-building", "summary"];
    html! {
        p.wizsteps {
            @for (i, s) in steps.iter().enumerate() {
                @if i > 0 { " → " }
                @if *s == current {
                    strong { (s) }
                } @else {
                    a.muted href={ "/languages/" (language_id) "/grammar/" (s) } { (s) }
                }
            }
        }
    }
}

/// Proto with wizard access, or a redirect for daughters and the
/// signed-out.
macro_rules! wizard_gate {
    ($state:expr, $session:expr, $id:expr) => {{
        let user = match require_user($state, $session).await? {
            Ok(u) => u,
            Err(landing) => return Ok(landing),
        };
        let (language, phonology) = owned_language_with_phonology($state, &user, $id).await?;
        if language.parent_id.is_some() {
            return Ok(Redirect::to(&format!("/languages/{}", language.id)).into_response());
        }
        (user, language, phonology)
    }};
}

/// GET /languages/{id}/grammar → first wizard page.
pub async fn wizard_entry(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let (_user, language, _phonology) = wizard_gate!(&state, &session, id);
    Ok(Redirect::to(&format!("/languages/{}/grammar/clauses", language.id)).into_response())
}

// ---------- Step 1: clauses ----------

/// GET /languages/{id}/grammar/clauses
pub async fn clauses_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let (user, language, phonology) = wizard_gate!(&state, &session, id);
    let g = ensure_grammar(&state, &language, &phonology).await?;

    let body = html! {
        p.eyebrow { a href={ "/languages/" (language.id) } class="muted" { "← " (language.name) } }
        (gsteps(language.id, "clauses"))
        h1 { "Clause structure" }
        p {
            "The skeleton every sentence hangs on. The pre-selected "
            "answers are a plausible draft rolled from " (language.name)
            "'s seed — change anything."
        }
        form method="post" action={ "/languages/" (language.id) "/grammar/clauses" } {
            h2 { "Word order" }
            @for wo in WordOrder::ALL {
                label.radio {
                    input type="radio" name="word_order" value=(wo.key())
                        checked[g.word_order == wo];
                    " " strong { (wo.label()) }
                    span.muted { " — " (wo.blurb()) }
                }
            }
            h2 { "Adpositions" }
            label.radio {
                input type="radio" name="adpositions" value="pre" checked[g.prepositions];
                " Prepositions — " span.ph { "in house" }
            }
            label.radio {
                input type="radio" name="adpositions" value="post" checked[!g.prepositions];
                " Postpositions — " span.ph { "house in" }
                span.muted { " (the SOV classic)" }
            }
            h2 { "Adjectives" }
            label.radio {
                input type="radio" name="adjectives" value="before" checked[g.adj_before_noun];
                " Before the noun — " span.ph { "old wolf" }
            }
            label.radio {
                input type="radio" name="adjectives" value="after" checked[!g.adj_before_noun];
                " After the noun — " span.ph { "wolf old" }
            }
            h2 { "Possessors" }
            label.radio {
                input type="radio" name="possessor" value="before"
                    checked[g.possessor_before_noun];
                " Before — " span.ph { "wolf's den" }
            }
            label.radio {
                input type="radio" name="possessor" value="after"
                    checked[!g.possessor_before_noun];
                " After — " span.ph { "den of-wolf" }
            }
            button type="submit" style="margin-top:1.5rem" { "Save — on to nouns →" }
        }
    };
    Ok(views::layout("Grammar: clauses", Some(&user), body).into_response())
}

#[derive(Deserialize)]
pub struct ClausesForm {
    word_order: String,
    adpositions: String,
    adjectives: String,
    possessor: String,
}

/// POST /languages/{id}/grammar/clauses
pub async fn save_clauses(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<ClausesForm>,
) -> Result<Response, AppError> {
    let (_user, language, phonology) = wizard_gate!(&state, &session, id);
    let mut g = ensure_grammar(&state, &language, &phonology).await?;
    if let Some(wo) = WordOrder::parse(&form.word_order) {
        g.word_order = wo;
    }
    g.prepositions = form.adpositions == "pre";
    g.adj_before_noun = form.adjectives == "before";
    g.possessor_before_noun = form.possessor == "before";
    save_grammar(&state, language.id, &g).await?;
    Ok(Redirect::to(&format!("/languages/{}/grammar/nouns", language.id)).into_response())
}

// ---------- Step 2: nouns & pronouns ----------

/// GET /languages/{id}/grammar/nouns
pub async fn nouns_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let (user, language, phonology) = wizard_gate!(&state, &session, id);
    let g = ensure_grammar(&state, &language, &phonology).await?;

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/grammar/clauses" } class="muted" { "← Clauses" }
        }
        (gsteps(language.id, "nouns"))
        h1 { "Nouns & pronouns" }
        form method="post" action={ "/languages/" (language.id) "/grammar/nouns" } {
            h2 { "Plural" }
            div.gramrow {
                select name="plural_marking" {
                    option value="suffix" selected[g.plural_marking == Marking::Suffix] { "suffix" }
                    option value="prefix" selected[g.plural_marking == Marking::Prefix] { "prefix" }
                    option value="particle" selected[g.plural_marking == Marking::Particle] { "particle" }
                }
                input.ph type="text" name="plural_form" value=(g.plural_form) required;
            }
            p.muted style="font-size:.9rem" {
                "Suffixes get automatic allomorphy: after a vowel-final "
                "stem, a vowel-initial suffix drops its own vowel "
                "(Silágo's -o/-yo pattern, mirrored)."
            }
            h2 { "Definite article" }
            label.radio {
                input type="checkbox" name="article_on" checked[g.definite_article.is_some()];
                " The language has a definite article: "
                input.ph type="text" name="article_form"
                    value=(g.definite_article.as_deref().unwrap_or("")) ;
            }
            h2 { "Pronouns" }
            label.radio {
                input type="checkbox" name="pronoun_case" checked[g.pronoun_case];
                " Pronouns decline for case (nominative / accusative / genitive) — "
                "Silágo-style fusional pronouns"
            }
            label.radio {
                input type="checkbox" name="animacy" checked[g.animacy];
                " Third person distinguishes animate from inanimate "
                "(inanimates use the demonstrative instead)"
            }
            div.chart-scroll {
                table.lex {
                    thead { tr { th {} th { "nominative" } th { "accusative" } th { "genitive" } } }
                    tbody {
                        @for (i, p) in g.pronouns.iter().enumerate() {
                            tr {
                                th.manner { (p.label()) }
                                td { input.ph type="text" name={ "nom_" (i) } value=(p.nom); }
                                td { input.ph type="text" name={ "acc_" (i) } value=(p.acc); }
                                td { input.ph type="text" name={ "gen_" (i) } value=(p.gen); }
                            }
                        }
                    }
                }
            }
            button type="submit" style="margin-top:1rem" { "Save — on to verbs →" }
        }
    };
    Ok(views::layout("Grammar: nouns", Some(&user), body).into_response())
}

#[derive(Deserialize)]
pub struct NounsForm {
    plural_marking: String,
    plural_form: String,
    article_on: Option<String>,
    #[serde(default)]
    article_form: String,
    pronoun_case: Option<String>,
    animacy: Option<String>,
    #[serde(default)]
    nom_0: String,
    #[serde(default)]
    acc_0: String,
    #[serde(default)]
    gen_0: String,
    #[serde(default)]
    nom_1: String,
    #[serde(default)]
    acc_1: String,
    #[serde(default)]
    gen_1: String,
    #[serde(default)]
    nom_2: String,
    #[serde(default)]
    acc_2: String,
    #[serde(default)]
    gen_2: String,
    #[serde(default)]
    nom_3: String,
    #[serde(default)]
    acc_3: String,
    #[serde(default)]
    gen_3: String,
    #[serde(default)]
    nom_4: String,
    #[serde(default)]
    acc_4: String,
    #[serde(default)]
    gen_4: String,
    #[serde(default)]
    nom_5: String,
    #[serde(default)]
    acc_5: String,
    #[serde(default)]
    gen_5: String,
}

/// POST /languages/{id}/grammar/nouns
pub async fn save_nouns(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<NounsForm>,
) -> Result<Response, AppError> {
    let (_user, language, phonology) = wizard_gate!(&state, &session, id);
    let mut g = ensure_grammar(&state, &language, &phonology).await?;
    if let Some(m) = Marking::parse(&form.plural_marking) {
        g.plural_marking = m;
    }
    if !form.plural_form.trim().is_empty() {
        g.plural_form = form.plural_form.trim().to_string();
    }
    g.definite_article = match (form.article_on.is_some(), form.article_form.trim()) {
        (true, f) if !f.is_empty() => Some(f.to_string()),
        _ => None,
    };
    g.pronoun_case = form.pronoun_case.is_some();
    g.animacy = form.animacy.is_some();
    let cells = [
        (&form.nom_0, &form.acc_0, &form.gen_0),
        (&form.nom_1, &form.acc_1, &form.gen_1),
        (&form.nom_2, &form.acc_2, &form.gen_2),
        (&form.nom_3, &form.acc_3, &form.gen_3),
        (&form.nom_4, &form.acc_4, &form.gen_4),
        (&form.nom_5, &form.acc_5, &form.gen_5),
    ];
    for (row, (nom, acc, gen)) in g.pronouns.iter_mut().zip(cells) {
        if !nom.trim().is_empty() {
            row.nom = nom.trim().to_string();
        }
        if !acc.trim().is_empty() {
            row.acc = acc.trim().to_string();
        }
        if !gen.trim().is_empty() {
            row.gen = gen.trim().to_string();
        }
    }
    save_grammar(&state, language.id, &g).await?;
    Ok(Redirect::to(&format!("/languages/{}/grammar/verbs", language.id)).into_response())
}

// ---------- Step 3: verbs ----------

fn opt_row(name: &str, on: bool, form_value: &str, label: &str, hint: &str) -> Markup {
    html! {
        label.radio {
            input type="checkbox" name={ (name) "_on" } checked[on];
            " " (label) ": "
            input.ph type="text" name={ (name) "_form" } value=(form_value);
            span.muted { " " (hint) }
        }
    }
}

/// GET /languages/{id}/grammar/verbs
pub async fn verbs_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let (user, language, phonology) = wizard_gate!(&state, &session, id);
    let g = ensure_grammar(&state, &language, &phonology).await?;

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/grammar/nouns" } class="muted" { "← Nouns" }
        }
        (gsteps(language.id, "verbs"))
        h1 { "The verb system" }
        p {
            "The present tense is always the bare stem. Everything else "
            "is up to you — Silágo built twelve tenses from four "
            "principal parts and one auxiliary; the pieces below are "
            "exactly that kit."
        }
        form method="post" action={ "/languages/" (language.id) "/grammar/verbs" } {
            h2 { "Tense suffixes" }
            div.gramrow {
                span { "Past: " }
                input.ph type="text" name="past_form" value=(g.past_form) required;
            }
            (opt_row("future", g.future_form.is_some(),
                g.future_form.as_deref().unwrap_or(""),
                "Future suffix", "(unchecked: future is expressed by context)"))
            h2 { "Aspect" }
            (opt_row("continuous", g.continuous_form.is_some(),
                g.continuous_form.as_deref().unwrap_or(""),
                "Continuous participle suffix", "(stacks after tense: walked-ing = was walking)"))
            (opt_row("aux", g.perfect_aux.is_some(),
                g.perfect_aux.as_deref().unwrap_or(""),
                "Perfect auxiliary verb", "(a \"have\": aux + past form = perfect)"))
            h2 { "Copula" }
            (opt_row("copula", g.copula.is_some(),
                g.copula.as_deref().unwrap_or(""),
                "Overt copula (\"to be\")",
                "(unchecked: zero copula — \"night cold\" is a sentence)"))
            h2 { "Negation" }
            div.gramrow {
                select name="negation" {
                    option value="particle" selected[g.negation == NegationStrategy::Particle] {
                        "particle before the verb"
                    }
                    option value="prefix" selected[g.negation == NegationStrategy::Prefix] {
                        "prefix bound to the verb"
                    }
                }
                input.ph type="text" name="negation_form" value=(g.negation_form) required;
            }
            p.muted style="font-size:.9rem" {
                "Commands use the bare stem with no subject — the "
                "imperative comes free."
            }
            button type="submit" style="margin-top:1rem" { "Save — on to word-building →" }
        }
    };
    Ok(views::layout("Grammar: verbs", Some(&user), body).into_response())
}

#[derive(Deserialize)]
pub struct VerbsForm {
    past_form: String,
    future_on: Option<String>,
    #[serde(default)]
    future_form: String,
    continuous_on: Option<String>,
    #[serde(default)]
    continuous_form: String,
    aux_on: Option<String>,
    #[serde(default)]
    aux_form: String,
    copula_on: Option<String>,
    #[serde(default)]
    copula_form: String,
    negation: String,
    negation_form: String,
}

/// POST /languages/{id}/grammar/verbs
pub async fn save_verbs(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<VerbsForm>,
) -> Result<Response, AppError> {
    let (_user, language, phonology) = wizard_gate!(&state, &session, id);
    let mut g = ensure_grammar(&state, &language, &phonology).await?;
    let opt = |on: bool, v: &str| -> Option<String> {
        (on && !v.trim().is_empty()).then(|| v.trim().to_string())
    };
    if !form.past_form.trim().is_empty() {
        g.past_form = form.past_form.trim().to_string();
    }
    g.future_form = opt(form.future_on.is_some(), &form.future_form);
    g.continuous_form = opt(form.continuous_on.is_some(), &form.continuous_form);
    g.perfect_aux = opt(form.aux_on.is_some(), &form.aux_form);
    g.copula = opt(form.copula_on.is_some(), &form.copula_form);
    g.negation = if form.negation == "prefix" {
        NegationStrategy::Prefix
    } else {
        NegationStrategy::Particle
    };
    if !form.negation_form.trim().is_empty() {
        g.negation_form = form.negation_form.trim().to_string();
    }
    save_grammar(&state, language.id, &g).await?;
    Ok(
        Redirect::to(&format!("/languages/{}/grammar/word-building", language.id))
            .into_response(),
    )
}

// ---------- Step 4: word-building ----------

/// GET /languages/{id}/grammar/word-building
pub async fn wordbuilding_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let (user, language, phonology) = wizard_gate!(&state, &session, id);
    let g = ensure_grammar(&state, &language, &phonology).await?;

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/grammar/verbs" } class="muted" { "← Verbs" }
        }
        (gsteps(language.id, "word-building"))
        h1 { "Word-building" }
        p {
            "Modal prefixes stack onto verb stems (want-walk, must-walk); "
            "derivational suffixes grow the lexicon sideways (walk → "
            "walker, walk-place). Silágo ran on exactly this machinery."
        }
        form method="post" action={ "/languages/" (language.id) "/grammar/word-building" } {
            h2 { "Modal prefixes" }
            @for (i, (concept, form_)) in g.modals.iter().enumerate() {
                div.gramrow {
                    span style="min-width:11rem" { (concept) }
                    input.ph type="text" name={ "modal_" (i) } value=(form_);
                }
            }
            h2 { "Derivational suffixes" }
            @for (i, (meaning, form_)) in g.derivations.iter().enumerate() {
                div.gramrow {
                    span style="min-width:11rem" { (meaning) }
                    input.ph type="text" name={ "deriv_" (i) } value=(form_);
                }
            }
            button type="submit" style="margin-top:1rem" { "Save — see the summary →" }
        }
    };
    Ok(views::layout("Grammar: word-building", Some(&user), body).into_response())
}

#[derive(Deserialize)]
pub struct WordbuildingForm {
    #[serde(default)]
    modal_0: String,
    #[serde(default)]
    modal_1: String,
    #[serde(default)]
    modal_2: String,
    #[serde(default)]
    modal_3: String,
    #[serde(default)]
    modal_4: String,
    #[serde(default)]
    deriv_0: String,
    #[serde(default)]
    deriv_1: String,
    #[serde(default)]
    deriv_2: String,
    #[serde(default)]
    deriv_3: String,
    #[serde(default)]
    deriv_4: String,
    #[serde(default)]
    deriv_5: String,
    #[serde(default)]
    deriv_6: String,
    #[serde(default)]
    deriv_7: String,
}

/// POST /languages/{id}/grammar/word-building
pub async fn save_wordbuilding(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<WordbuildingForm>,
) -> Result<Response, AppError> {
    let (_user, language, phonology) = wizard_gate!(&state, &session, id);
    let mut g = ensure_grammar(&state, &language, &phonology).await?;
    let modal_vals = [
        &form.modal_0, &form.modal_1, &form.modal_2, &form.modal_3, &form.modal_4,
    ];
    for (slot, v) in g.modals.iter_mut().zip(modal_vals) {
        if !v.trim().is_empty() {
            slot.1 = v.trim().to_string();
        }
    }
    let deriv_vals = [
        &form.deriv_0, &form.deriv_1, &form.deriv_2, &form.deriv_3,
        &form.deriv_4, &form.deriv_5, &form.deriv_6, &form.deriv_7,
    ];
    for (slot, v) in g.derivations.iter_mut().zip(deriv_vals) {
        if !v.trim().is_empty() {
            slot.1 = v.trim().to_string();
        }
    }
    save_grammar(&state, language.id, &g).await?;
    Ok(Redirect::to(&format!("/languages/{}/grammar/summary", language.id)).into_response())
}

/// GET /languages/{id}/grammar/summary
pub async fn summary_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let (user, language, _phonology) = wizard_gate!(&state, &session, id);
    let content = grammar_body(&state, &user, &language).await?;
    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/grammar/word-building" } class="muted" { "← Word-building" }
        }
        (gsteps(language.id, "summary"))
        h1 { (language.name) ": the grammar" }
        (content)
        form.inline method="get" action={ "/languages/" (language.id) } {
            button type="submit" { "Finish — back to " (language.name) " →" }
        }
    };
    Ok(views::layout("Grammar summary", Some(&user), body).into_response())
}

// ---------- The sketch (Grammar tab) ----------

pub(crate) async fn grammar_body(
    state: &AppState,
    user: &crate::auth::User,
    language: &Language,
) -> Result<Markup, AppError> {
    let (proto, chain) = evolve::proto_and_chain(state, user.id, language).await?;
    let grammar = grammar_of(state, proto.id).await?;

    let Some(g) = grammar else {
        return Ok(if language.parent_id.is_none() {
            html! {
                p {
                    "The grammar wizard pins down clause structure, noun "
                    "and pronoun systems, the verb kit, and word-building "
                    "affixes — every generated form editable, every "
                    "choice explained."
                }
                form.inline method="get" action={ "/languages/" (language.id) "/grammar" } {
                    button type="submit" { "Open the grammar wizard →" }
                }
            }
        } else {
            html! { div.empty {
                "Grammar lives on the proto-language (" (proto.name) ") "
                "and daughters inherit it, eroded by their sound changes. "
                "Run the wizard there first."
            } }
        });
    };

    let d = |f: &str| derive(f, &chain);
    // A live example verb, if the lexicon has one.
    let lexemes = evolve::proto_lexemes(state, proto.id).await?;
    let walk = lexemes
        .iter()
        .find(|l| l.gloss == "to walk")
        .map(|l| l.form_ipa.clone());

    Ok(html! {
        @if language.parent_id.is_some() {
            p.muted style="font-size:.9rem" {
                "Inherited from " (proto.name) "; every form below has "
                "been run through " (language.name) "'s sound changes."
            }
        }
        dl.gram {
            dt { "Word order" }
            dd { (g.word_order.label()) " — " (g.word_order.blurb()) "." }
            dt { "Adpositions" }
            dd { @if g.prepositions { "Prepositions." } @else { "Postpositions." } }
            dt { "Adjectives" }
            dd { @if g.adj_before_noun { "Before the noun." } @else { "After the noun." } }
            dt { "Possessors" }
            dd { @if g.possessor_before_noun { "Before the possessed noun." } @else { "After the possessed noun." } }
            dt { "Plural" }
            dd {
                (g.plural_marking.label()) " "
                span.ph { "/" (d(&g.plural_form)) "/" }
                @if g.plural_marking == Marking::Suffix {
                    span.muted { " (vowel-initial suffixes elide after vowel-final stems)" }
                }
            }
            dt { "Definite article" }
            dd {
                @match &g.definite_article {
                    Some(a) => { span.ph { "/" (d(a)) "/" } " before the noun phrase." }
                    None => { "None — definiteness from context." }
                }
            }
            dt { "Pronouns" }
            dd {
                @if g.pronoun_case { "Decline for nominative, accusative, and genitive:" }
                @else { "One form per person and number:" }
                div.chart-scroll {
                    table.lex {
                        thead {
                            tr {
                                th {}
                                th { "nom" }
                                @if g.pronoun_case { th { "acc" } th { "gen" } }
                            }
                        }
                        tbody {
                            @for p in &g.pronouns {
                                tr {
                                    th.manner { (p.label()) }
                                    td.ph { "/" (d(&p.nom)) "/" }
                                    @if g.pronoun_case {
                                        td.ph { "/" (d(&p.acc)) "/" }
                                        td.ph { "/" (d(&p.gen)) "/" }
                                    }
                                }
                            }
                        }
                    }
                }
                @if g.animacy {
                    p.muted style="font-size:.9rem" {
                        "Third-person pronouns are for animates only; "
                        "inanimates take the demonstrative."
                    }
                }
            }
            dt { "Verb system" }
            dd {
                "Present = bare stem. Past "
                span.ph { "-" (d(&g.past_form)) }
                @if let Some(f) = &g.future_form { ", future " span.ph { "-" (d(f)) } }
                @if let Some(c) = &g.continuous_form {
                    ", continuous " span.ph { "-" (d(c)) }
                    span.muted { " (stacks after tense)" }
                }
                "."
                @if let Some(aux) = &g.perfect_aux {
                    " Perfects: auxiliary "
                    span.ph { "/" (d(aux)) "/" }
                    " + past form — a four-principal-parts system, "
                    "twelve tenses for free."
                }
                @if let Some(w) = &walk {
                    p.ph style="margin:.4rem 0 0" {
                        "“walk”: /" (d(w)) "/ · walked: /"
                        (d(&attach_suffix(w, &g.past_form))) "/"
                        @if let Some(c) = &g.continuous_form {
                            " · was walking: /"
                            (d(&attach_suffix(&attach_suffix(w, &g.past_form), c))) "/"
                        }
                        @if let Some(aux) = &g.perfect_aux {
                            " · has walked: /" (d(aux)) " "
                            (d(&attach_suffix(w, &g.past_form))) "/"
                        }
                    }
                }
            }
            dt { "Copula" }
            dd {
                @match &g.copula {
                    Some(c) => { span.ph { "/" (d(c)) "/" } " — conjugates like any verb." }
                    None => { "None — “the night cold” is a full sentence." }
                }
            }
            dt { "Negation" }
            dd {
                @match g.negation {
                    NegationStrategy::Particle => {
                        "Particle " span.ph { "/" (d(&g.negation_form)) "/" } " before the verb."
                    }
                    NegationStrategy::Prefix => {
                        "Prefix " span.ph { (d(&g.negation_form)) "-" }
                        " bound to the verb, before any modal."
                    }
                }
            }
            dt { "Modal prefixes" }
            dd {
                @for (i, (concept, form)) in g.modals.iter().enumerate() {
                    @if i > 0 { " · " }
                    (concept) " " span.ph { (d(form)) "-" }
                }
                @if let Some(w) = &walk {
                    p.ph style="margin:.4rem 0 0" {
                        "“want to walk”: /"
                        (d(&attach_prefix(&g.modals.get(3).map(|m| m.1.clone()).unwrap_or_default(), w)))
                        "/"
                    }
                }
            }
            dt { "Derivation" }
            dd {
                @for (i, (meaning, form)) in g.derivations.iter().enumerate() {
                    @if i > 0 { " · " }
                    (meaning) " " span.ph { "-" (d(form)) }
                }
            }
            dt { "Imperative" }
            dd { "Bare stem, no subject: the shortest possible sentence." }
        }
        @if language.parent_id.is_none() {
            p.muted style="font-size:.9rem" {
                a href={ "/languages/" (language.id) "/grammar" } { "Edit in the grammar wizard →" }
            }
        }
        p.muted style="font-size:.9rem" { "See it all working in the Stories tab." }
    })
}

// ---------- Story realization ----------

fn pronoun_form<'a>(g: &'a GrammarSpec, gloss: &str, accusative: bool) -> Option<&'a str> {
    let (person, plural) = match gloss {
        "I" => (1, false),
        "you (sg)" => (2, false),
        "he/she/it" => (3, false),
        "we" => (1, true),
        "you (pl)" => (2, true),
        "they" => (3, true),
        _ => return None,
    };
    g.pronouns
        .iter()
        .find(|p| p.person == person && p.plural == plural)
        .map(|p| if accusative { p.acc.as_str() } else { p.nom.as_str() })
}

fn apply_plural(stem: &str, g: &GrammarSpec) -> Vec<String> {
    match g.plural_marking {
        Marking::Suffix => vec![attach_suffix(stem, &g.plural_form)],
        Marking::Prefix => vec![attach_prefix(&g.plural_form, stem)],
        Marking::Particle => vec![stem.to_string(), g.plural_form.clone()],
    }
}

fn realize_np(
    phrase: &[&str],
    plural: bool,
    accusative: bool,
    g: &GrammarSpec,
    lookup: &HashMap<&str, &str>,
) -> Option<Vec<String>> {
    // Pronoun phrases are single-word and skip articles.
    if phrase.len() == 1 {
        if let Some(p) = pronoun_form(g, phrase[0], accusative) {
            return Some(vec![p.to_string()]);
        }
    }
    let (adjs, noun) = phrase.split_at(phrase.len() - 1);
    let noun_form = *lookup.get(noun[0])?;
    let noun_words = if plural {
        apply_plural(noun_form, g)
    } else {
        vec![noun_form.to_string()]
    };
    let adj_words: Option<Vec<String>> = adjs
        .iter()
        .map(|a| lookup.get(a).map(|s| s.to_string()))
        .collect();
    let adj_words = adj_words?;
    let mut out: Vec<String> = Vec::new();
    if let Some(article) = &g.definite_article {
        out.push(article.clone());
    }
    if g.adj_before_noun {
        out.extend(adj_words);
        out.extend(noun_words);
    } else {
        out.extend(noun_words);
        out.extend(adj_words);
    }
    Some(out)
}

fn realize_verb(stem: &str, negated: bool, g: &GrammarSpec) -> Vec<String> {
    let base = if negated && g.negation == NegationStrategy::Prefix {
        attach_prefix(&g.negation_form, stem)
    } else {
        stem.to_string()
    };
    let inflected = attach_suffix(&base, &g.past_form);
    if negated && g.negation == NegationStrategy::Particle {
        vec![g.negation_form.clone(), inflected]
    } else {
        vec![inflected]
    }
}

fn realize_line(
    line: &StoryLine,
    g: &GrammarSpec,
    lookup: &HashMap<&str, &str>,
) -> Option<Vec<String>> {
    let subject = realize_np(line.subject, line.subject_plural, false, g, lookup)?;
    let object = if line.object.is_empty() {
        vec![]
    } else {
        realize_np(line.object, false, true, g, lookup)?
    };
    let verb: Vec<String> = match line.verb {
        Some(v) => realize_verb(lookup.get(v)?, line.negated, g),
        // Zero-copula predicate — unless the language has an overt one.
        None => match &g.copula {
            Some(c) => realize_verb(c, line.negated, g),
            None => vec![],
        },
    };
    let oblique: Vec<String> = match line.oblique {
        None => vec![],
        Some((adp, phrase)) => {
            let adp_form = lookup.get(adp)?.to_string();
            let np = realize_np(phrase, false, false, g, lookup)?;
            if g.prepositions {
                [adp_form].into_iter().chain(np).collect()
            } else {
                np.into_iter().chain([adp_form]).collect()
            }
        }
    };

    let ordered: Vec<Vec<String>> = match g.word_order {
        WordOrder::Sov => vec![subject, oblique, object, verb],
        WordOrder::Svo => vec![subject, verb, object, oblique],
        WordOrder::Vso => vec![verb, subject, object, oblique],
        WordOrder::Vos => vec![verb, object, oblique, subject],
        WordOrder::Ovs => vec![object, verb, subject, oblique],
        WordOrder::Osv => vec![object, subject, verb, oblique],
    };
    Some(ordered.into_iter().flatten().collect())
}

fn romanize_words(words: &[String], rom: &BTreeMap<String, String>) -> String {
    words
        .iter()
        .map(|w| romanization::romanize(w, rom))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) async fn stories_body(
    state: &AppState,
    user: &crate::auth::User,
    language: &Language,
) -> Result<Markup, AppError> {
    let (proto, chain) = evolve::proto_and_chain(state, user.id, language).await?;
    let grammar = grammar_of(state, proto.id).await?;
    let lexemes = evolve::proto_lexemes(state, proto.id).await?;

    let Some(g) = grammar else {
        return Ok(html! { div.empty {
            "Stories need a grammar — run the wizard on the Grammar tab"
            @if language.parent_id.is_some() { " of " (proto.name) }
            " first."
        } });
    };
    if lexemes.is_empty() {
        return Ok(html! { div.empty {
            "Stories need words. Seed " (proto.name) "'s lexicon first."
        } });
    }

    let lookup: HashMap<&str, &str> = lexemes
        .iter()
        .map(|l| (l.gloss.as_str(), l.form_ipa.as_str()))
        .collect();

    let rom: Phonology = if language.parent_id.is_some() {
        evolve::derived_display_phonology(state, user, language).await?
    } else {
        owned_language_with_phonology(state, user, proto.id).await?.1
    };

    Ok(html! {
        h2 { "“" (STORY_TITLE) "”" }
        p.muted style="font-size:.9rem" {
            "Realized live from the lexicon and grammar"
            @if language.parent_id.is_some() {
                ", then run through " (language.name) "'s sound changes"
            }
            ". Edit either and the story follows."
        }
        div.story {
            @for line in STORY {
                @match realize_line(line, &g, &lookup) {
                    Some(words) => {
                        @let surface: Vec<String> =
                            words.iter().map(|w| derive(w, &chain)).collect();
                        div.storyline {
                            p.ph.st1 { "/" (surface.join(" ")) "/" }
                            p.st2 { "⟨" (romanize_words(&surface, &rom.romanization)) "⟩" }
                            p.muted.st3 { (line.english) }
                        }
                    }
                    None => {
                        div.storyline {
                            p.muted {
                                "(" (line.english) " — a word this line "
                                "needs is missing from the lexicon)"
                            }
                        }
                    }
                }
            }
        }
    })
}

// ---------- Tab handlers ----------

/// GET /languages/{id}/tab/grammar (HTMX)
pub async fn tab_grammar(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, _) = owned_language_with_phonology(&state, &user, id).await?;
    Ok(grammar_body(&state, &user, &language).await?.into_response())
}

/// GET /languages/{id}/tab/stories (HTMX)
pub async fn tab_stories(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, _) = owned_language_with_phonology(&state, &user, id).await?;
    Ok(stories_body(&state, &user, &language).await?.into_response())
}
