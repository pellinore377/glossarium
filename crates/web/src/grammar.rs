//! Grammar tab and story realization.
//!
//! The grammar spec lives on the proto (languages.grammar JSON), like the
//! lexicon. Daughters read the proto's grammar and push every realized
//! word — stems, affixed forms, particles — through their sound-change
//! chain, so grammatical material erodes exactly like vocabulary does.

use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use lex::grammar::{GrammarSpec, Marking, StoryLine, WordOrder, STORY, STORY_TITLE};
use maud::{html, Markup};
use sca::Rule;
use std::collections::BTreeMap;
use std::collections::HashMap;
use tower_sessions::Session;

use crate::{
    error::AppError,
    evolve,
    phonology::{owned_language_with_phonology, require_user, Phonology},
    romanization,
    routes::Language,
    state::AppState,
};

async fn grammar_of(state: &AppState, language_id: i64) -> Result<Option<GrammarSpec>, AppError> {
    let (json,): (String,) = sqlx::query_as("SELECT grammar FROM languages WHERE id = ?")
        .bind(language_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError(anyhow!("language {language_id} not found")))?;
    // The column defaults to '{}', which deliberately fails to parse as a
    // GrammarSpec — "no grammar yet".
    Ok(serde_json::from_str(&json).ok())
}

fn derive(form: &str, chain: &[Rule]) -> String {
    sca::derive_ipa(form, chain).unwrap_or_else(|| form.to_string())
}

// ---------- Realization ----------

fn apply_marking(stem: &str, marking: Marking, form: &str) -> Vec<String> {
    match marking {
        Marking::Suffix => vec![format!("{stem}{form}")],
        Marking::Prefix => vec![format!("{form}{stem}")],
        Marking::Particle => vec![stem.to_string(), form.to_string()],
    }
}

/// A noun phrase: adjectives ordered per the grammar, plural marked on
/// the head noun. `None` if any gloss is missing from the lexicon.
fn realize_np(
    phrase: &[&str],
    plural: bool,
    grammar: &GrammarSpec,
    lookup: &HashMap<&str, &str>,
) -> Option<Vec<String>> {
    let (adjs, noun) = phrase.split_at(phrase.len() - 1);
    let noun_form = *lookup.get(noun[0])?;
    let noun_words = if plural {
        apply_marking(noun_form, grammar.plural_marking, &grammar.plural_form)
    } else {
        vec![noun_form.to_string()]
    };
    let adj_words: Option<Vec<String>> = adjs
        .iter()
        .map(|a| lookup.get(a).map(|s| s.to_string()))
        .collect();
    let adj_words = adj_words?;
    Some(if grammar.adj_before_noun {
        adj_words.into_iter().chain(noun_words).collect()
    } else {
        noun_words.into_iter().chain(adj_words).collect()
    })
}

/// One clause as a flat word list, proto-level IPA.
fn realize_line(
    line: &StoryLine,
    grammar: &GrammarSpec,
    lookup: &HashMap<&str, &str>,
) -> Option<Vec<String>> {
    let subject = realize_np(line.subject, line.subject_plural, grammar, lookup)?;
    let object = if line.object.is_empty() {
        vec![]
    } else {
        realize_np(line.object, false, grammar, lookup)?
    };
    let verb: Vec<String> = match line.verb {
        None => vec![],
        Some(v) => {
            let stem = *lookup.get(v)?;
            let mut words =
                apply_marking(stem, grammar.past_marking, &grammar.past_form);
            if line.negated {
                words.insert(0, grammar.negation_form.clone());
            }
            words
        }
    };
    let oblique: Vec<String> = match line.oblique {
        None => vec![],
        Some((adp, phrase)) => {
            let adp_form = lookup.get(adp)?.to_string();
            let np = realize_np(phrase, false, grammar, lookup)?;
            // SOV languages overwhelmingly use postpositions.
            if grammar.word_order == WordOrder::Sov {
                np.into_iter().chain([adp_form]).collect()
            } else {
                [adp_form].into_iter().chain(np).collect()
            }
        }
    };

    let ordered: Vec<Vec<String>> = match grammar.word_order {
        WordOrder::Sov => vec![subject, oblique, object, verb],
        WordOrder::Svo => vec![subject, verb, object, oblique],
        WordOrder::Vso => vec![verb, subject, object, oblique],
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

// ---------- Grammar tab ----------

fn marked_example(marking: Marking, form: &str, chain: &[Rule]) -> Markup {
    let derived = derive(form, chain);
    html! {
        (marking.label()) " "
        span.ph { "/" (derived) "/" }
        @match marking {
            Marking::Suffix => { span.muted { " (word-" (derived) ")" } }
            Marking::Prefix => { span.muted { " (" (derived) "-word)" } }
            Marking::Particle => { span.muted { " (a separate word)" } }
        }
    }
}

pub(crate) async fn grammar_body(
    state: &AppState,
    user: &crate::auth::User,
    language: &Language,
) -> Result<Markup, AppError> {
    let (proto, chain) = evolve::proto_and_chain(state, user.id, language).await?;
    let grammar = grammar_of(state, proto.id).await?;

    let Some(g) = grammar else {
        return Ok(if language.parent_id.is_none() {
            let (_, phonology) = owned_language_with_phonology(state, user, proto.id).await?;
            let ready = !phonology.vowels.is_empty();
            html! {
                p {
                    "A grammar sketch pins down how " (language.name) " "
                    "builds sentences: word order, where adjectives sit, "
                    "and how plural, past, and negation are marked — with "
                    "the actual affixes generated from your phonology."
                }
                @if ready {
                    button
                        hx-post={ "/languages/" (language.id) "/grammar/generate" }
                        hx-target="#tabpanel"
                        hx-swap="innerHTML"
                    { "Generate the grammar →" }
                    p.muted style="font-size:.9rem" {
                        "Deterministic, like everything else: the same "
                        "language always gets the same grammar."
                    }
                } @else {
                    div.empty { "Design the phonology first — affixes need sounds." }
                }
            }
        } else {
            html! {
                div.empty {
                    "Grammar lives on the proto-language (" (proto.name) ") "
                    "and daughters inherit it, eroded by their sound "
                    "changes. Generate it there first."
                }
            }
        });
    };

    Ok(html! {
        @if language.parent_id.is_some() {
            p.muted style="font-size:.9rem" {
                "Inherited from " (proto.name) "; every affix and particle "
                "below has been run through " (language.name) "'s sound "
                "changes."
            }
        }
        dl.gram {
            dt { "Word order" }
            dd { (g.word_order.label()) " — " (g.word_order.blurb()) "." }
            dt { "Adjectives" }
            dd {
                @if g.adj_before_noun { "Before the noun, English-style." }
                @else { "After the noun, Romance-style." }
            }
            dt { "Plural" }
            dd { (marked_example(g.plural_marking, &g.plural_form, &chain)) }
            dt { "Past tense" }
            dd { (marked_example(g.past_marking, &g.past_form, &chain)) }
            dt { "Negation" }
            dd {
                "Particle "
                span.ph { "/" (derive(&g.negation_form, &chain)) "/" }
                " before the verb."
            }
            dt { "Copula" }
            dd { "None — predicate adjectives stand next to their subject bare." }
        }
        p.muted style="font-size:.9rem" {
            "See it all working in the Stories tab."
        }
    })
}

// ---------- Stories tab ----------

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
            "Stories need a grammar. Generate one on the Grammar tab"
            @if language.parent_id.is_some() { " of " (proto.name) } ""
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

    // The romanization used for display: the daughter's derived map when
    // this is a daughter, the proto's own otherwise.
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

// ---------- Handlers ----------

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

/// POST /languages/{id}/grammar/generate (HTMX)
pub async fn generate_grammar(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let (language, phonology) = owned_language_with_phonology(&state, &user, id).await?;

    if language.parent_id.is_none() && grammar_of(&state, language.id).await?.is_none() {
        let mut spec = crate::lexicon::word_spec(&phonology, language.id);
        // A different stream than the lexicon, same determinism.
        spec.seed ^= 0x6772_616D_6D61_7221;
        if let Ok(g) = lex::grammar::generate(spec) {
            sqlx::query("UPDATE languages SET grammar = ? WHERE id = ?")
                .bind(serde_json::to_string(&g)?)
                .bind(language.id)
                .execute(&state.db)
                .await?;
        }
    }
    Ok(grammar_body(&state, &user, &language).await?.into_response())
}
