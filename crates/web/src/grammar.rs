//! The grammar wizard, sketch tab, and story realization.
//!
//! The wizard drafts a typologically coherent grammar — a morphological
//! temperament first, then per-category strategies rolled under its
//! influence — and walks the user through profile → nouns → verbs →
//! word-building with every choice and form editable. Whole subsystems
//! may simply not exist in a given language; that is the point.
//!
//! Daughters read the proto's grammar and push every realized word —
//! stems, affixed forms, particles, pronouns — through their
//! sound-change chain: grammar erodes exactly like vocabulary.

use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use lex::grammar::{
    attach_prefix, attach_suffix, Alignment, CaseAffix, Comparative, GenderSystem,
    GrammarSpec, ModalStrategy, MorphType, NegationStrategy, NumStrategy, NumberSystem,
    QuestionStrategy, StoryLine, TenseSystem, WordOrder, STORY, STORY_TITLE,
};
use maud::{html, Markup};
use sca::Rule;
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

type FormMap = HashMap<String, String>;

pub(crate) async fn grammar_of(
    state: &AppState,
    language_id: i64,
) -> Result<Option<GrammarSpec>, AppError> {
    let (json,): (String,) = sqlx::query_as("SELECT grammar FROM languages WHERE id = ?")
        .bind(language_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError(anyhow!("language {language_id} not found")))?;
    // '{}' (the column default) and any older-shaped blob fail to parse —
    // both mean "run the wizard".
    Ok(serde_json::from_str(&json).ok())
}

async fn save_grammar(state: &AppState, language_id: i64, g: &GrammarSpec) -> Result<(), AppError> {
    sqlx::query("UPDATE languages SET grammar = ? WHERE id = ?")
        .bind(serde_json::to_string(g)?)
        .bind(language_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

fn draft(phonology: &Phonology, language_id: i64, force: Option<MorphType>) -> Result<GrammarSpec, AppError> {
    let mut spec = crate::lexicon::word_spec(phonology, language_id);
    spec.seed ^= 0x6772_616D_6D61_7221;
    lex::grammar::generate(spec, force)
        .map_err(|e| AppError(anyhow!("cannot draft a grammar yet: {e}")))
}

async fn ensure_grammar(
    state: &AppState,
    language: &Language,
    phonology: &Phonology,
) -> Result<GrammarSpec, AppError> {
    if let Some(g) = grammar_of(state, language.id).await? {
        return Ok(g);
    }
    let g = draft(phonology, language.id, None)?;
    save_grammar(state, language.id, &g).await?;
    Ok(g)
}

fn derive(form: &str, chain: &[Rule]) -> String {
    sca::derive_ipa(form, chain).unwrap_or_else(|| form.to_string())
}

// ---------- Wizard scaffolding ----------

fn gsteps(language_id: i64, current: &str) -> Markup {
    let steps = ["profile", "nouns", "verbs", "word-building", "summary"];
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

/// GET /languages/{id}/grammar
pub async fn wizard_entry(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let (_user, language, _phonology) = wizard_gate!(&state, &session, id);
    Ok(Redirect::to(&format!("/languages/{}/grammar/profile", language.id)).into_response())
}

// Small form helpers.
fn val<'a>(f: &'a FormMap, k: &str) -> &'a str {
    f.get(k).map(String::as_str).unwrap_or("").trim()
}
fn set_if(f: &FormMap, k: &str, target: &mut String) {
    let v = val(f, k);
    if !v.is_empty() {
        *target = v.to_string();
    }
}
fn opt_of(f: &FormMap, on_key: &str, form_key: &str) -> Option<String> {
    (f.contains_key(on_key) && !val(f, form_key).is_empty())
        .then(|| val(f, form_key).to_string())
}

fn text_in(name: &str, value: &str) -> Markup {
    html! { input.ph type="text" name=(name) value=(value); }
}

// ---------- Step 1: profile & clauses ----------

/// GET /languages/{id}/grammar/profile
pub async fn profile_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let (user, language, phonology) = wizard_gate!(&state, &session, id);
    let g = ensure_grammar(&state, &language, &phonology).await?;

    let body = html! {
        p.eyebrow { a href={ "/languages/" (language.id) } class="muted" { "← " (language.name) } }
        (gsteps(language.id, "profile"))
        h1 { "Grammatical profile" }
        p {
            "First the temperament, then the skeleton. Everything below "
            "was drafted from " (language.name) "'s seed — change "
            "anything; changing the temperament re-drafts the later pages "
            "to match it."
        }
        form method="post" action={ "/languages/" (language.id) "/grammar/profile" } {
            h2 { "Morphological type" }
            @for m in MorphType::ALL {
                label.radio {
                    input type="radio" name="morphology" value=(m.key())
                        checked[g.morphology == m];
                    " " strong { (m.label()) }
                    span.muted { " — " (m.blurb()) }
                }
            }
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
            }
            h2 { "Adjectives" }
            label.radio {
                input type="radio" name="adjectives" value="before" checked[g.adj_before_noun];
                " Before the noun"
            }
            label.radio {
                input type="radio" name="adjectives" value="after" checked[!g.adj_before_noun];
                " After the noun"
            }
            h2 { "Possessors" }
            label.radio {
                input type="radio" name="possessor" value="before" checked[g.possessor_before_noun];
                " Before — " span.ph { "wolf's den" }
            }
            label.radio {
                input type="radio" name="possessor" value="after" checked[!g.possessor_before_noun];
                " After — " span.ph { "den of-wolf" }
            }
            button type="submit" style="margin-top:1.5rem" { "Save — on to nouns →" }
        }
    };
    Ok(views::layout("Grammar: profile", Some(&user), body).into_response())
}

/// POST /languages/{id}/grammar/profile
pub async fn save_profile(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<FormMap>,
) -> Result<Response, AppError> {
    let (_user, language, phonology) = wizard_gate!(&state, &session, id);
    let mut g = ensure_grammar(&state, &language, &phonology).await?;
    // A temperament change re-drafts everything under the new bias; the
    // clause choices from this very form are applied on top.
    if let Some(m) = MorphType::parse(val(&form, "morphology")) {
        if m != g.morphology {
            g = draft(&phonology, language.id, Some(m))?;
        }
    }
    if let Some(wo) = WordOrder::parse(val(&form, "word_order")) {
        g.word_order = wo;
    }
    g.prepositions = val(&form, "adpositions") == "pre";
    g.adj_before_noun = val(&form, "adjectives") == "before";
    g.possessor_before_noun = val(&form, "possessor") == "before";
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

    let (num_kind, num_strategy, dual_v, plural_v) = match &g.number {
        NumberSystem::NoMarking => ("none", NumStrategy::Suffix, String::new(), String::new()),
        NumberSystem::Plural { strategy, plural } => {
            ("plural", *strategy, String::new(), plural.clone())
        }
        NumberSystem::DualPlural { strategy, dual, plural } => {
            ("dual", *strategy, dual.clone(), plural.clone())
        }
    };
    let extra_of = |name: &str| -> Option<&CaseAffix> {
        g.extra_cases.iter().find(|c| c.name == name)
    };

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/grammar/profile" } class="muted" { "← Profile" }
        }
        (gsteps(language.id, "nouns"))
        h1 { "Nouns & pronouns" }
        form method="post" action={ "/languages/" (language.id) "/grammar/nouns" } {
            h2 { "Number" }
            label.radio {
                input type="radio" name="number" value="none" checked[num_kind == "none"];
                " No marking — numerals and context carry it (Mandarin)"
            }
            label.radio {
                input type="radio" name="number" value="plural" checked[num_kind == "plural"];
                " Singular / plural"
            }
            label.radio {
                input type="radio" name="number" value="dual" checked[num_kind == "dual"];
                " Singular / dual / plural (Arabic, Slovene)"
            }
            div.gramrow {
                span { "Strategy:" }
                select name="num_strategy" {
                    option value="suffix" selected[num_strategy == NumStrategy::Suffix] { "suffix" }
                    option value="prefix" selected[num_strategy == NumStrategy::Prefix] { "prefix" }
                    option value="particle" selected[num_strategy == NumStrategy::Particle] { "particle" }
                    option value="reduplication" selected[num_strategy == NumStrategy::Reduplication] {
                        "reduplication (kela → kelakela)"
                    }
                }
                span { "plural:" } (text_in("plural_form", &plural_v))
                span { "dual:" } (text_in("dual_form", &dual_v))
            }
            h2 { "Case" }
            label.radio {
                input type="radio" name="alignment" value="neutral"
                    checked[g.alignment == Alignment::Neutral];
                " No case marking — word order does the work"
            }
            label.radio {
                input type="radio" name="alignment" value="nomacc"
                    checked[g.alignment == Alignment::NomAcc];
                " Nominative–accusative — objects get a suffix"
            }
            label.radio {
                input type="radio" name="alignment" value="ergabs"
                    checked[g.alignment == Alignment::ErgAbs];
                " Ergative–absolutive — transitive subjects get the suffix "
                span.muted { "(Basque, Georgian — a bold, wonderful choice)" }
            }
            div.gramrow {
                span { "Core case suffix:" } (text_in("core_case", &g.core_case))
            }
            p.muted style="font-size:.9rem" { "Further cases, each a suffix:" }
            @for name in lex::grammar::EXTRA_CASE_NAMES {
                @let existing = extra_of(name);
                label.radio {
                    input type="checkbox" name={ "case_" (name) } checked[existing.is_some()];
                    " " (name) " "
                    input.ph type="text" name={ "caseform_" (name) }
                        value=(existing.map(|c| c.suffix.as_str()).unwrap_or(""));
                }
            }
            h2 { "Gender / noun class" }
            label.radio {
                input type="radio" name="gender" value="none" checked[g.gender == GenderSystem::None];
                " None"
            }
            label.radio {
                input type="radio" name="gender" value="animate"
                    checked[g.gender == GenderSystem::AnimateInanimate];
                " Animate / inanimate"
            }
            label.radio {
                input type="radio" name="gender" value="mascfem"
                    checked[g.gender == GenderSystem::MascFem];
                " Masculine / feminine"
            }
            h2 { "Articles" }
            label.radio {
                input type="checkbox" name="def_on" checked[g.definite_article.is_some()];
                " Definite (\"the\"): "
                input.ph type="text" name="def_form"
                    value=(g.definite_article.as_deref().unwrap_or(""));
            }
            label.radio {
                input type="checkbox" name="indef_on" checked[g.indefinite_article.is_some()];
                " Indefinite (\"a\"): "
                input.ph type="text" name="indef_form"
                    value=(g.indefinite_article.as_deref().unwrap_or(""));
            }
            h2 { "Pronouns" }
            label.radio {
                input type="checkbox" name="pronoun_case" checked[g.pronoun_case];
                " Pronouns decline for case even if nouns don't (like English him/his)"
            }
            div.chart-scroll {
                table.lex {
                    thead {
                        tr {
                            th {}
                            th { @if g.alignment == Alignment::ErgAbs { "absolutive" } @else { "subject" } }
                            th { @if g.alignment == Alignment::ErgAbs { "ergative" } @else { "object" } }
                            th { "possessive" }
                        }
                    }
                    tbody {
                        @for (i, p) in g.pronouns.iter().enumerate() {
                            tr {
                                th.manner { (p.label()) }
                                td { (text_in(&format!("pr_{i}_a"), &p.a)) }
                                td { (text_in(&format!("pr_{i}_b"), &p.b)) }
                                td { (text_in(&format!("pr_{i}_g"), &p.gen)) }
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

/// POST /languages/{id}/grammar/nouns
pub async fn save_nouns(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<FormMap>,
) -> Result<Response, AppError> {
    let (_user, language, phonology) = wizard_gate!(&state, &session, id);
    let mut g = ensure_grammar(&state, &language, &phonology).await?;

    let strategy = NumStrategy::parse(val(&form, "num_strategy")).unwrap_or(NumStrategy::Suffix);
    let plural = val(&form, "plural_form").to_string();
    let dual = val(&form, "dual_form").to_string();
    g.number = match val(&form, "number") {
        "none" => NumberSystem::NoMarking,
        "dual" if !plural.is_empty() && !dual.is_empty() => {
            NumberSystem::DualPlural { strategy, dual, plural }
        }
        _ if !plural.is_empty() => NumberSystem::Plural { strategy, plural },
        _ => g.number,
    };
    if let Some(a) = Alignment::parse(val(&form, "alignment")) {
        g.alignment = a;
    }
    set_if(&form, "core_case", &mut g.core_case);
    g.extra_cases = lex::grammar::EXTRA_CASE_NAMES
        .iter()
        .filter(|n| form.contains_key(&format!("case_{n}")))
        .filter_map(|n| {
            let suffix = val(&form, &format!("caseform_{n}"));
            (!suffix.is_empty()).then(|| CaseAffix {
                name: n.to_string(),
                suffix: suffix.to_string(),
            })
        })
        .collect();
    if let Some(gs) = GenderSystem::parse(val(&form, "gender")) {
        g.gender = gs;
    }
    g.definite_article = opt_of(&form, "def_on", "def_form");
    g.indefinite_article = opt_of(&form, "indef_on", "indef_form");
    g.pronoun_case = form.contains_key("pronoun_case");
    for (i, row) in g.pronouns.iter_mut().enumerate() {
        set_if(&form, &format!("pr_{i}_a"), &mut row.a);
        set_if(&form, &format!("pr_{i}_b"), &mut row.b);
        set_if(&form, &format!("pr_{i}_g"), &mut row.gen);
    }
    save_grammar(&state, language.id, &g).await?;
    Ok(Redirect::to(&format!("/languages/{}/grammar/verbs", language.id)).into_response())
}

// ---------- Step 3: verbs ----------

/// GET /languages/{id}/grammar/verbs
pub async fn verbs_page(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let (user, language, phonology) = wizard_gate!(&state, &session, id);
    let g = ensure_grammar(&state, &language, &phonology).await?;

    let (tense_kind, past_v, future_v, perfective_v) = match &g.tense {
        TenseSystem::Tenseless { perfective } => {
            ("tenseless", String::new(), String::new(), perfective.clone())
        }
        TenseSystem::PastNonpast { past } => ("two", past.clone(), String::new(), String::new()),
        TenseSystem::ThreeWay { past, future } => {
            ("three", past.clone(), future.clone(), String::new())
        }
    };

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/grammar/nouns" } class="muted" { "← Nouns" }
        }
        (gsteps(language.id, "verbs"))
        h1 { "The verb system" }
        form method="post" action={ "/languages/" (language.id) "/grammar/verbs" } {
            h2 { "Tense" }
            label.radio {
                input type="radio" name="tense" value="tenseless" checked[tense_kind == "tenseless"];
                " Tenseless — aspect carries time (Mandarin). Perfective marker: "
                (text_in("perfective_form", &perfective_v))
            }
            label.radio {
                input type="radio" name="tense" value="two" checked[tense_kind == "two"];
                " Past vs non-past. Past: " (text_in("past_form", &past_v))
            }
            label.radio {
                input type="radio" name="tense" value="three" checked[tense_kind == "three"];
                " Past / present / future. Future: " (text_in("future_form", &future_v))
            }
            label.radio {
                input type="checkbox" name="tense_particles" checked[g.tense_particles];
                " Tense markers are free particles before the verb, not suffixes "
                span.muted { "(the isolating way)" }
            }
            h2 { "Subject agreement" }
            label.radio {
                input type="checkbox" name="agreement_on" checked[g.agreement.is_some()];
                " The verb agrees with its subject (six person/number suffixes):"
            }
            div.gramrow {
                @let rows = g.agreement.clone().unwrap_or_else(|| {
                    lex::grammar::AGREEMENT_LABELS.iter().map(|l| (l.to_string(), String::new())).collect()
                });
                @for (i, (label, form_)) in rows.iter().enumerate() {
                    span.muted { (label) }
                    (text_in(&format!("agr_{i}"), form_))
                }
            }
            h2 { "Aspect & auxiliaries" }
            label.radio {
                input type="checkbox" name="continuous_on" checked[g.continuous.is_some()];
                " Continuous suffix (stacks after tense): "
                input.ph type="text" name="continuous_form"
                    value=(g.continuous.as_deref().unwrap_or(""));
            }
            label.radio {
                input type="checkbox" name="aux_on" checked[g.perfect_aux.is_some()];
                " Perfect auxiliary (\"have\"): "
                input.ph type="text" name="aux_form"
                    value=(g.perfect_aux.as_deref().unwrap_or(""));
            }
            label.radio {
                input type="checkbox" name="copula_on" checked[g.copula.is_some()];
                " Overt copula (\"to be\"): "
                input.ph type="text" name="copula_form"
                    value=(g.copula.as_deref().unwrap_or(""));
                span.muted { " (unchecked: “night cold” is a sentence)" }
            }
            h2 { "Negation" }
            div.gramrow {
                select name="negation" {
                    option value="particle" selected[g.negation == NegationStrategy::Particle] {
                        "particle before the verb"
                    }
                    option value="prefix" selected[g.negation == NegationStrategy::Prefix] {
                        "prefix on the verb"
                    }
                    option value="suffix" selected[g.negation == NegationStrategy::Suffix] {
                        "suffix on the verb"
                    }
                    option value="auxiliary" selected[g.negation == NegationStrategy::Auxiliary] {
                        "negative auxiliary verb (Finnish-style)"
                    }
                }
                (text_in("negation_form", &g.negation_form))
            }
            h2 { "Yes/no questions" }
            @let q = |k: QuestionStrategy, label: &str| html! {
                label.radio {
                    input type="radio" name="question" value=(k.key()) checked[g.question == k];
                    " " (label)
                }
            };
            (q(QuestionStrategy::FinalParticle, "Sentence-final particle (Japanese ka)"))
            (q(QuestionStrategy::InitialParticle, "Sentence-initial particle (Polish czy)"))
            (q(QuestionStrategy::Inversion, "Verb fronting (English)"))
            (q(QuestionStrategy::Intonation, "Intonation only"))
            div.gramrow {
                span { "Particle form (if used):" } (text_in("question_form", &g.question_form))
            }
            h2 { "Evidentiality" }
            label.radio {
                input type="checkbox" name="evid_on" checked[g.evidentiality.is_some()];
                " Verbs mark how the speaker knows (Turkish, Quechua): witnessed "
                input.ph type="text" name="evid_seen"
                    value=(g.evidentiality.as_ref().map(|e| e.0.as_str()).unwrap_or(""));
                " hearsay "
                input.ph type="text" name="evid_heard"
                    value=(g.evidentiality.as_ref().map(|e| e.1.as_str()).unwrap_or(""));
            }
            button type="submit" style="margin-top:1rem" { "Save — on to word-building →" }
        }
    };
    Ok(views::layout("Grammar: verbs", Some(&user), body).into_response())
}

/// POST /languages/{id}/grammar/verbs
pub async fn save_verbs(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<FormMap>,
) -> Result<Response, AppError> {
    let (_user, language, phonology) = wizard_gate!(&state, &session, id);
    let mut g = ensure_grammar(&state, &language, &phonology).await?;

    let past = val(&form, "past_form").to_string();
    let future = val(&form, "future_form").to_string();
    let perfective = val(&form, "perfective_form").to_string();
    g.tense = match val(&form, "tense") {
        "tenseless" if !perfective.is_empty() => TenseSystem::Tenseless { perfective },
        "three" if !past.is_empty() && !future.is_empty() => {
            TenseSystem::ThreeWay { past, future }
        }
        _ if !past.is_empty() => TenseSystem::PastNonpast { past },
        _ => g.tense,
    };
    g.tense_particles = form.contains_key("tense_particles");
    g.agreement = form.contains_key("agreement_on").then(|| {
        lex::grammar::AGREEMENT_LABELS
            .iter()
            .enumerate()
            .map(|(i, l)| (l.to_string(), val(&form, &format!("agr_{i}")).to_string()))
            .filter(|(_, f)| !f.is_empty())
            .collect::<Vec<_>>()
    });
    if let Some(a) = &g.agreement {
        if a.len() != lex::grammar::AGREEMENT_LABELS.len() {
            g.agreement = None; // incomplete paradigm = no agreement
        }
    }
    g.continuous = opt_of(&form, "continuous_on", "continuous_form");
    g.perfect_aux = opt_of(&form, "aux_on", "aux_form");
    g.copula = opt_of(&form, "copula_on", "copula_form");
    if let Some(n) = NegationStrategy::parse(val(&form, "negation")) {
        g.negation = n;
    }
    set_if(&form, "negation_form", &mut g.negation_form);
    if let Some(q) = QuestionStrategy::parse(val(&form, "question")) {
        g.question = q;
    }
    set_if(&form, "question_form", &mut g.question_form);
    g.evidentiality = (form.contains_key("evid_on")
        && !val(&form, "evid_seen").is_empty()
        && !val(&form, "evid_heard").is_empty())
    .then(|| {
        (
            val(&form, "evid_seen").to_string(),
            val(&form, "evid_heard").to_string(),
        )
    });
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

    let (comp_kind, comp_a, comp_b) = match &g.comparative {
        Comparative::Particle { than } => ("particle", than.clone(), String::new()),
        Comparative::Suffix { suffix, than } => ("suffix", suffix.clone(), than.clone()),
        Comparative::ExceedVerb { verb } => ("exceed", verb.clone(), String::new()),
    };

    let body = html! {
        p.eyebrow {
            a href={ "/languages/" (language.id) "/grammar/verbs" } class="muted" { "← Verbs" }
        }
        (gsteps(language.id, "word-building"))
        h1 { "Word-building" }
        form method="post" action={ "/languages/" (language.id) "/grammar/word-building" } {
            h2 { "Modality" }
            p.muted style="font-size:.9rem" {
                "How does the language say can / might / must / want / "
                "need? Not every language bolts these onto the verb — "
                "many use whole verbs or free particles."
            }
            @let ms = |k: ModalStrategy, label: &str| html! {
                label.radio {
                    input type="radio" name="modality" value=(k.key()) checked[g.modality == k];
                    " " (label)
                }
            };
            (ms(ModalStrategy::Verbs, "Modal verbs taking a complement (English can go)"))
            (ms(ModalStrategy::Particles, "Free particles in the verb phrase (Mandarin huì)"))
            (ms(ModalStrategy::Prefixes, "Prefixes bound to the verb stem"))
            (ms(ModalStrategy::Suffixes, "Suffixes bound to the verb stem"))
            @for (i, (concept, form_)) in g.modals.iter().enumerate() {
                div.gramrow {
                    span style="min-width:11rem" { (concept) }
                    (text_in(&format!("modal_{i}"), form_))
                }
            }
            h2 { "Comparison" }
            label.radio {
                input type="radio" name="comparative" value="particle" checked[comp_kind == "particle"];
                " Bare adjective + a \"than\" word — " span.ph { "big than hill" }
            }
            label.radio {
                input type="radio" name="comparative" value="suffix" checked[comp_kind == "suffix"];
                " Degree suffix + \"than\" — " span.ph { "big-er than hill" }
            }
            label.radio {
                input type="radio" name="comparative" value="exceed" checked[comp_kind == "exceed"];
                " An \"exceed\" verb — " span.ph { "mountain exceeds hill big-ness" }
                span.muted { " (Mandarin, Yoruba)" }
            }
            div.gramrow {
                span { "form:" } (text_in("comp_a", &comp_a))
                span { "than-word (suffix strategy):" } (text_in("comp_b", &comp_b))
            }
            h2 { "Converbs" }
            label.radio {
                input type="checkbox" name="converbs_on" checked[g.converbs.is_some()];
                " Subordinate clauses use converb suffixes (while / because / in-order-to):"
            }
            @let cvs = g.converbs.clone().unwrap_or_else(|| {
                lex::grammar::CONVERB_MEANINGS.iter().map(|m| (m.to_string(), String::new())).collect()
            });
            @for (i, (meaning, form_)) in cvs.iter().enumerate() {
                div.gramrow {
                    span style="min-width:11rem" { (meaning) }
                    (text_in(&format!("cv_{i}"), form_))
                }
            }
            h2 { "Derivational affixes" }
            p.muted style="font-size:.9rem" {
                "Check what exists; every language keeps a different "
                "subset. All suffixes."
            }
            @for (i, meaning) in lex::grammar::DERIVATION_POOL.iter().enumerate() {
                @let existing = g.derivations.iter().find(|(m, _)| m == meaning);
                label.radio {
                    input type="checkbox" name={ "der_" (i) } checked[existing.is_some()];
                    " " (meaning) " "
                    input.ph type="text" name={ "derform_" (i) }
                        value=(existing.map(|(_, f)| f.as_str()).unwrap_or(""));
                }
            }
            button type="submit" style="margin-top:1rem" { "Save — see the summary →" }
        }
    };
    Ok(views::layout("Grammar: word-building", Some(&user), body).into_response())
}

/// POST /languages/{id}/grammar/word-building
pub async fn save_wordbuilding(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Form(form): Form<FormMap>,
) -> Result<Response, AppError> {
    let (_user, language, phonology) = wizard_gate!(&state, &session, id);
    let mut g = ensure_grammar(&state, &language, &phonology).await?;

    if let Some(m) = ModalStrategy::parse(val(&form, "modality")) {
        g.modality = m;
    }
    for (i, slot) in g.modals.iter_mut().enumerate() {
        set_if(&form, &format!("modal_{i}"), &mut slot.1);
    }
    let a = val(&form, "comp_a").to_string();
    let b = val(&form, "comp_b").to_string();
    if !a.is_empty() {
        g.comparative = match val(&form, "comparative") {
            "suffix" if !b.is_empty() => Comparative::Suffix { suffix: a, than: b },
            "exceed" => Comparative::ExceedVerb { verb: a },
            _ => Comparative::Particle { than: a },
        };
    }
    g.converbs = form.contains_key("converbs_on").then(|| {
        lex::grammar::CONVERB_MEANINGS
            .iter()
            .enumerate()
            .map(|(i, m)| (m.to_string(), val(&form, &format!("cv_{i}")).to_string()))
            .filter(|(_, f)| !f.is_empty())
            .collect::<Vec<_>>()
    });
    if g.converbs.as_ref().is_some_and(Vec::is_empty) {
        g.converbs = None;
    }
    g.derivations = lex::grammar::DERIVATION_POOL
        .iter()
        .enumerate()
        .filter(|(i, _)| form.contains_key(&format!("der_{i}")))
        .filter_map(|(i, m)| {
            let f = val(&form, &format!("derform_{i}"));
            (!f.is_empty()).then(|| (m.to_string(), f.to_string()))
        })
        .collect();
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
                    "The grammar wizard drafts a typologically coherent "
                    "grammar — morphological temperament, number and case "
                    "systems, agreement, tense, modality, evidentiality — "
                    "then walks you through every choice with editable "
                    "forms. No two languages get the same feature set."
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
            dt { "Profile" }
            dd { (g.morphology.label()) " — " (g.morphology.blurb()) "." }
            dt { "Clause" }
            dd {
                (g.word_order.label()) "; "
                @if g.prepositions { "prepositions" } @else { "postpositions" } "; adjectives "
                @if g.adj_before_noun { "before" } @else { "after" } " the noun; possessors "
                @if g.possessor_before_noun { "before" } @else { "after" } "."
            }
            dt { "Number" }
            dd {
                @match &g.number {
                    NumberSystem::NoMarking => {
                        "Unmarked — numerals and context carry it."
                    }
                    NumberSystem::Plural { strategy, plural } => {
                        "Singular/plural, "
                        @match strategy {
                            NumStrategy::Reduplication => { "by reduplication (stem doubles)." }
                            s => { (s.label()) " " span.ph { "/" (d(plural)) "/" } "." }
                        }
                    }
                    NumberSystem::DualPlural { strategy, dual, plural } => {
                        "Singular/dual/plural (" (strategy.label()) "): dual "
                        span.ph { "/" (d(dual)) "/" } ", plural "
                        span.ph { "/" (d(plural)) "/" } "."
                    }
                }
            }
            dt { "Case" }
            dd {
                @match g.alignment {
                    Alignment::Neutral => { "No case on nouns — word order carries the roles." }
                    Alignment::NomAcc => {
                        "Nominative–accusative: objects take "
                        span.ph { "-" (d(&g.core_case)) } "."
                    }
                    Alignment::ErgAbs => {
                        "Ergative–absolutive: transitive subjects take "
                        span.ph { "-" (d(&g.core_case)) } "."
                    }
                }
                @if !g.extra_cases.is_empty() {
                    " Further cases: "
                    @for (i, c) in g.extra_cases.iter().enumerate() {
                        @if i > 0 { ", " }
                        (c.name) " " span.ph { "-" (d(&c.suffix)) }
                    }
                    "."
                }
            }
            dt { "Gender" }
            dd {
                @match g.gender {
                    GenderSystem::None => { "None." }
                    GenderSystem::AnimateInanimate => {
                        "Animate vs inanimate; third-person pronouns are "
                        "for animates, demonstratives cover the rest."
                    }
                    GenderSystem::MascFem => {
                        "Masculine vs feminine, agreement on articles and pronouns."
                    }
                }
            }
            dt { "Articles" }
            dd {
                @match (&g.definite_article, &g.indefinite_article) {
                    (None, None) => { "None — definiteness from context." }
                    (def, indef) => {
                        @if let Some(a) = def { "Definite " span.ph { "/" (d(a)) "/" } ". " }
                        @if let Some(a) = indef { "Indefinite " span.ph { "/" (d(a)) "/" } "." }
                    }
                }
            }
            dt { "Pronouns" }
            dd {
                @if g.pronoun_case { "Decline for case:" } @else { "One form each:" }
                div.chart-scroll {
                    table.lex {
                        thead {
                            tr {
                                th {}
                                th { @if g.alignment == Alignment::ErgAbs { "abs" } @else { "subj" } }
                                @if g.pronoun_case {
                                    th { @if g.alignment == Alignment::ErgAbs { "erg" } @else { "obj" } }
                                    th { "poss" }
                                }
                            }
                        }
                        tbody {
                            @for p in &g.pronouns {
                                tr {
                                    th.manner { (p.label()) }
                                    td.ph { "/" (d(&p.a)) "/" }
                                    @if g.pronoun_case {
                                        td.ph { "/" (d(&p.b)) "/" }
                                        td.ph { "/" (d(&p.gen)) "/" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            dt { "Verb" }
            dd {
                @match &g.tense {
                    TenseSystem::Tenseless { perfective } => {
                        "Tenseless: bare stem plus a perfective marker "
                        span.ph { @if g.tense_particles { "/" (d(perfective)) "/ (particle)" }
                                  @else { "-" (d(perfective)) } }
                        " — time from context and adverbs."
                    }
                    TenseSystem::PastNonpast { past } => {
                        "Past vs non-past: past "
                        span.ph { @if g.tense_particles { "/" (d(past)) "/ (particle)" }
                                  @else { "-" (d(past)) } } "."
                    }
                    TenseSystem::ThreeWay { past, future } => {
                        "Three tenses: past "
                        span.ph { @if g.tense_particles { "/" (d(past)) "/" } @else { "-" (d(past)) } }
                        ", future "
                        span.ph { @if g.tense_particles { "/" (d(future)) "/" } @else { "-" (d(future)) } }
                        ", present bare."
                    }
                }
                @if let Some(c) = &g.continuous { " Continuous " span.ph { "-" (d(c)) } " stacks after tense." }
                @if let Some(aux) = &g.perfect_aux {
                    " Perfects with auxiliary " span.ph { "/" (d(aux)) "/" } " + past form."
                }
                @if let Some(agr) = &g.agreement {
                    p style="margin:.4rem 0 0" {
                        "Subject agreement: "
                        @for (i, (l, f)) in agr.iter().enumerate() {
                            @if i > 0 { " · " }
                            (l) " " span.ph { "-" (d(f)) }
                        }
                    }
                }
                @if let Some((seen, heard)) = &g.evidentiality {
                    p style="margin:.4rem 0 0" {
                        "Evidentiality: witnessed " span.ph { "-" (d(seen)) }
                        ", hearsay " span.ph { "-" (d(heard)) }
                        " — a claim always says how you know."
                    }
                }
                @if let Some(w) = &walk {
                    @let past_ex = match &g.tense {
                        TenseSystem::Tenseless { perfective } => perfective.clone(),
                        TenseSystem::PastNonpast { past } => past.clone(),
                        TenseSystem::ThreeWay { past, .. } => past.clone(),
                    };
                    p.ph style="margin:.4rem 0 0" {
                        "“walk”: /" (d(w)) "/ · walked: /"
                        @if g.tense_particles { (d(&past_ex)) " " (d(w)) }
                        @else { (d(&attach_suffix(w, &past_ex))) }
                        "/"
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
                        "Prefix " span.ph { (d(&g.negation_form)) "-" } " on the verb."
                    }
                    NegationStrategy::Suffix => {
                        "Suffix " span.ph { "-" (d(&g.negation_form)) } " on the verb."
                    }
                    NegationStrategy::Auxiliary => {
                        "A negative auxiliary " span.ph { "/" (d(&g.negation_form)) "/" }
                        " carries the tense while the main verb goes bare (Finnish-style)."
                    }
                }
            }
            dt { "Questions" }
            dd {
                @match g.question {
                    QuestionStrategy::FinalParticle => {
                        "Sentence-final particle " span.ph { "/" (d(&g.question_form)) "/" } "."
                    }
                    QuestionStrategy::InitialParticle => {
                        "Sentence-initial particle " span.ph { "/" (d(&g.question_form)) "/" } "."
                    }
                    QuestionStrategy::Inversion => { "Verb fronting." }
                    QuestionStrategy::Intonation => { "Rising intonation only." }
                }
            }
            dt { "Modality" }
            dd {
                @match g.modality {
                    ModalStrategy::Prefixes => { "Prefixes on the verb stem: " }
                    ModalStrategy::Suffixes => { "Suffixes on the verb stem: " }
                    ModalStrategy::Verbs => { "Modal verbs taking a bare-stem complement: " }
                    ModalStrategy::Particles => { "Free particles in the verb phrase: " }
                }
                @for (i, (concept, form)) in g.modals.iter().enumerate() {
                    @if i > 0 { " · " }
                    (concept) " "
                    span.ph {
                        @match g.modality {
                            ModalStrategy::Prefixes => { (d(form)) "-" }
                            ModalStrategy::Suffixes => { "-" (d(form)) }
                            _ => { "/" (d(form)) "/" }
                        }
                    }
                }
            }
            dt { "Comparison" }
            dd {
                @match &g.comparative {
                    Comparative::Particle { than } => {
                        "Bare adjective with than-word " span.ph { "/" (d(than)) "/" } "."
                    }
                    Comparative::Suffix { suffix, than } => {
                        "Degree suffix " span.ph { "-" (d(suffix)) }
                        " with than-word " span.ph { "/" (d(than)) "/" } "."
                    }
                    Comparative::ExceedVerb { verb } => {
                        "An exceed-verb " span.ph { "/" (d(verb)) "/" }
                        " — “the mountain exceeds the hill (in) bigness.”"
                    }
                }
            }
            @if let Some(cv) = &g.converbs {
                dt { "Converbs" }
                dd {
                    "Clause-chaining suffixes: "
                    @for (i, (m, f)) in cv.iter().enumerate() {
                        @if i > 0 { " · " }
                        (m) " " span.ph { "-" (d(f)) }
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

fn pronoun_form<'a>(g: &'a GrammarSpec, gloss: &str, marked: bool) -> Option<&'a str> {
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
        .map(|p| if marked { p.b.as_str() } else { p.a.as_str() })
}

fn apply_number(stem: &str, g: &GrammarSpec) -> Vec<String> {
    match &g.number {
        NumberSystem::NoMarking => vec![stem.to_string()],
        NumberSystem::Plural { strategy, plural }
        | NumberSystem::DualPlural { strategy, plural, .. } => match strategy {
            NumStrategy::Suffix => vec![attach_suffix(stem, plural)],
            NumStrategy::Prefix => vec![attach_prefix(plural, stem)],
            NumStrategy::Particle => vec![stem.to_string(), plural.clone()],
            NumStrategy::Reduplication => vec![format!("{stem}{stem}")],
        },
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    TransitiveSubject,
    IntransitiveSubject,
    Object,
    Oblique,
}

fn realize_np(
    phrase: &[&str],
    plural: bool,
    role: Role,
    g: &GrammarSpec,
    lookup: &HashMap<&str, &str>,
) -> Option<Vec<String>> {
    let marked = match (g.alignment, role) {
        (Alignment::NomAcc, Role::Object) => true,
        (Alignment::ErgAbs, Role::TransitiveSubject) => true,
        _ => false,
    };
    if phrase.len() == 1 {
        if let Some(p) = pronoun_form(g, phrase[0], marked && g.pronoun_case) {
            return Some(vec![p.to_string()]);
        }
    }
    let (adjs, noun) = phrase.split_at(phrase.len() - 1);
    let noun_form = *lookup.get(noun[0])?;
    let mut noun_words = if plural {
        apply_number(noun_form, g)
    } else {
        vec![noun_form.to_string()]
    };
    if marked {
        if let Some(head) = noun_words.first_mut() {
            *head = attach_suffix(head, &g.core_case);
        }
    }
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

/// The past-flavoured tense marker (stories are told in the past).
fn past_marker(g: &GrammarSpec) -> &str {
    match &g.tense {
        TenseSystem::Tenseless { perfective } => perfective,
        TenseSystem::PastNonpast { past } => past,
        TenseSystem::ThreeWay { past, .. } => past,
    }
}

fn agreement_suffix<'a>(g: &'a GrammarSpec, person: u8, plural: bool) -> Option<&'a str> {
    let idx = (person - 1) as usize + if plural { 3 } else { 0 };
    g.agreement
        .as_ref()
        .and_then(|a| a.get(idx))
        .map(|(_, f)| f.as_str())
}

fn realize_verb(
    stem: &str,
    negated: bool,
    subj_person: u8,
    subj_plural: bool,
    g: &GrammarSpec,
) -> Vec<String> {
    let tense = past_marker(g);
    let inflect = |s: &str| -> String {
        let mut w = if g.tense_particles { s.to_string() } else { attach_suffix(s, tense) };
        if let Some(agr) = agreement_suffix(g, subj_person, subj_plural) {
            w = attach_suffix(&w, agr);
        }
        w
    };
    let mut words: Vec<String> = Vec::new();
    match (negated, g.negation) {
        (true, NegationStrategy::Auxiliary) => {
            // The negative auxiliary carries tense; the verb goes bare.
            words.push(inflect(&g.negation_form));
            if g.tense_particles {
                words.insert(0, tense.to_string());
            }
            words.push(stem.to_string());
            return words;
        }
        (true, NegationStrategy::Particle) => words.push(g.negation_form.clone()),
        _ => {}
    }
    if g.tense_particles {
        words.push(tense.to_string());
    }
    let base = match (negated, g.negation) {
        (true, NegationStrategy::Prefix) => attach_prefix(&g.negation_form, stem),
        _ => stem.to_string(),
    };
    let mut inflected = inflect(&base);
    if negated && g.negation == NegationStrategy::Suffix {
        inflected = attach_suffix(&inflected, &g.negation_form);
    }
    words.push(inflected);
    words
}

fn subject_features(line: &StoryLine) -> (u8, bool) {
    if line.subject.len() == 1 && line.subject[0] == "he/she/it" {
        (3, false)
    } else {
        (3, line.subject_plural)
    }
}

fn realize_line(
    line: &StoryLine,
    g: &GrammarSpec,
    lookup: &HashMap<&str, &str>,
) -> Option<Vec<String>> {
    let transitive = line.verb.is_some() && !line.object.is_empty();
    let subj_role = if transitive {
        Role::TransitiveSubject
    } else {
        Role::IntransitiveSubject
    };
    let subject = realize_np(line.subject, line.subject_plural, subj_role, g, lookup)?;
    let object = if line.object.is_empty() {
        vec![]
    } else {
        realize_np(line.object, false, Role::Object, g, lookup)?
    };
    let (sp, spl) = subject_features(line);
    let verb: Vec<String> = match line.verb {
        Some(v) => realize_verb(lookup.get(v)?, line.negated, sp, spl, g),
        None => match &g.copula {
            Some(c) => realize_verb(c, line.negated, sp, spl, g),
            None => vec![],
        },
    };
    let oblique: Vec<String> = match line.oblique {
        None => vec![],
        Some((adp, phrase)) => {
            let adp_form = lookup.get(adp)?.to_string();
            let np = realize_np(phrase, false, Role::Oblique, g, lookup)?;
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
