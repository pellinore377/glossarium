//! Maud views. Server-rendered shell + HTMX for partial swaps.
//!
//! Design direction (kept deliberately quiet at scaffold stage): historical
//! philology, not SaaS. Cool paper, iron-gall ink, one verdigris accent,
//! serifs for display, mono for anything in IPA. The signature element is
//! the family tree rendered as a descent column — it arrives with the
//! evolve milestone; nothing here should fight it.

use maud::{html, Markup, DOCTYPE};

use crate::auth::User;
use crate::routes::{Language, Project};

const STYLE: &str = r#"
:root {
  --paper: #f6f5f1; --ink: #23261f; --faded: #6b6f63;
  --accent: #2e6e63; --accent-ink: #1d4a43; --line: #d8d6cc;
  --card: #fdfcf9;
}
@media (prefers-color-scheme: dark) {
  :root {
    --paper: #1c1e1a; --ink: #e4e2d8; --faded: #9a9e90;
    --accent: #5cb3a4; --accent-ink: #8ed3c6; --line: #3a3d35;
    --card: #24261f;
  }
}
* { box-sizing: border-box; }
body {
  margin: 0; background: var(--paper); color: var(--ink);
  font: 16px/1.65 "Iowan Old Style", Palatino, Georgia, serif;
}
main { max-width: 46rem; margin: 0 auto; padding: 2rem 1.25rem 4rem; }
header.site {
  border-bottom: 1px solid var(--line);
  padding: 1rem 1.25rem; display: flex; align-items: baseline; gap: 1rem;
}
header.site .mark { font-size: 1.15rem; letter-spacing: .02em; }
header.site .mark a { color: var(--ink); text-decoration: none; }
header.site .mark .ipa { color: var(--accent); font-family: ui-monospace, monospace; }
header.site nav { margin-left: auto; display: flex; gap: 1rem; align-items: baseline; }
h1 { font-size: 1.6rem; font-weight: 500; margin: 0 0 .25rem; }
h2 { font-size: 1.15rem; font-weight: 500; margin: 2rem 0 .5rem; }
.eyebrow {
  font: 500 .72rem/1 ui-monospace, monospace; letter-spacing: .14em;
  text-transform: uppercase; color: var(--faded); margin: 0 0 .4rem;
}
.muted { color: var(--faded); }
a { color: var(--accent-ink); }
ul.cards { list-style: none; margin: 1rem 0; padding: 0; display: grid; gap: .6rem; }
ul.cards li {
  background: var(--card); border: 1px solid var(--line); border-radius: 6px;
  padding: .8rem 1rem;
}
ul.cards li a { text-decoration: none; font-size: 1.05rem; }
form.inline { display: flex; gap: .5rem; margin-top: 1rem; flex-wrap: wrap; }
input[type=text] {
  font: inherit; padding: .45rem .6rem; border: 1px solid var(--line);
  border-radius: 4px; background: var(--card); color: var(--ink); min-width: 16rem;
}
button {
  font: inherit; padding: .45rem .9rem; border: 1px solid var(--accent);
  border-radius: 4px; background: var(--accent); color: var(--paper); cursor: pointer;
}
button.quiet { background: transparent; color: var(--accent-ink); border-color: var(--line); }
.empty {
  border: 1px dashed var(--line); border-radius: 6px; padding: 1.4rem;
  color: var(--faded); margin-top: 1rem;
}
main:has(.chart-scroll) { max-width: 68rem; }
.chart-scroll {
  overflow-x: auto; margin: 1.25rem 0; border: 1px solid var(--line);
  border-radius: 6px; background: var(--card);
}
table.ipa { border-collapse: collapse; min-width: 58rem; width: 100%; }
table.ipa th, table.ipa td {
  border: 1px solid var(--line); padding: .25rem .3rem;
  text-align: center; vertical-align: middle;
}
table.ipa thead th {
  font: 500 .72rem/1.3 ui-monospace, monospace; letter-spacing: .03em;
}
table.ipa th.manner {
  text-align: left; font-size: .78rem; font-weight: 500;
  white-space: nowrap; padding: .25rem .6rem;
}
td.x {
  background: repeating-linear-gradient(45deg,
    transparent, transparent 4px, var(--line) 4px, var(--line) 6px);
}
button.sym {
  font: 500 1.1rem/1 "Gentium Plus", "Charis SIL", Gentium,
    "Times New Roman", serif;
  min-width: 2rem; padding: .4rem .35rem; margin: .05rem;
  border: 1px solid transparent; border-radius: 4px;
  background: transparent; color: var(--ink); cursor: pointer;
}
button.sym:hover { border-color: var(--accent); }
button.sym.on { background: var(--accent); color: var(--paper); font-weight: 700; }
.warnbox { margin-top: 1.25rem; }
p.warn {
  border-left: 3px solid #b58a3c; border-radius: 0;
  padding: .4rem .7rem; background: var(--card);
  margin: .4rem 0; font-size: .95rem;
}
p.ok { color: var(--faded); font-size: .95rem; }
ul.presets { list-style: none; padding: 0; margin: 1.25rem 0; display: grid; gap: .7rem; }
ul.presets li {
  background: var(--card); border: 1px solid var(--line);
  border-radius: 6px; padding: 1rem 1.1rem;
}
ul.presets .ph {
  font-family: "Gentium Plus", "Times New Roman", serif;
  color: var(--accent-ink); font-size: 1.05rem; margin: 0;
}
.wizsteps {
  font: 400 .78rem/1.4 ui-monospace, monospace; letter-spacing: .04em;
  color: var(--faded); margin: 0 0 .5rem;
}
.wizsteps strong { color: var(--accent-ink); font-weight: 500; }
.vowel-wrap {
  position: relative; max-width: 40rem; aspect-ratio: 10 / 7.2;
  margin: 1.5rem auto; padding: 0;
}
.vowel-wrap svg.trap {
  position: absolute; inset: 0; width: 100%; height: 100%;
  pointer-events: none;
}
.vpoint {
  position: absolute; transform: translate(-50%, -50%);
  display: flex; gap: .1rem; background: var(--paper);
  padding: 0 .15rem; border-radius: 4px;
}
select {
  font: inherit; padding: .4rem .5rem; border: 1px solid var(--line);
  border-radius: 4px; background: var(--card); color: var(--ink);
}
form.builder {
  display: flex; gap: 1rem; align-items: flex-end; flex-wrap: wrap;
  margin: 1rem 0;
}
form.builder label {
  display: flex; flex-direction: column; gap: .3rem;
  font: 500 .72rem/1 ui-monospace, monospace; letter-spacing: .1em;
  text-transform: uppercase; color: var(--faded);
}
form.builder .nucleus {
  font: 500 1.3rem/1 ui-monospace, monospace; color: var(--accent-ink);
  padding: .3rem .2rem;
}
.syltemplate {
  font: 500 1.5rem/1 ui-monospace, monospace; letter-spacing: .08em;
  color: var(--accent-ink); margin: .3rem 0 .6rem;
}
"#;

pub fn layout(title: &str, user: Option<&User>, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · Glossarium" }
                style { (maud::PreEscaped(STYLE)) }
                // Vendor this file for fully-offline use: see README §Static assets.
                script src="https://unpkg.com/htmx.org@2.0.4" {}
            }
            body {
                header.site {
                    p.mark { a href="/" { "Glossarium " span.ipa { "/ɡlɒˈsɑːrium/" } } }
                    nav {
                        @if let Some(u) = user {
                            span.muted { (u.display_name) }
                            form method="post" action="/auth/logout" style="margin:0" {
                                button.quiet type="submit" { "Sign out" }
                            }
                        }
                    }
                }
                main { (body) }
            }
        }
    }
}

pub fn landing() -> Markup {
    layout(
        "Welcome",
        None,
        html! {
            p.eyebrow { "A workshop for constructed language families" }
            h1 { "Build a proto-language. Then let time loose on it." }
            p {
                "Design a phonology, seed a lexicon, and evolve daughter "
                "languages through documented sound changes — with every "
                "derived form traceable back to the rule that produced it."
            }
            form.inline method="get" action="/auth/login" {
                button type="submit" { "Sign in with Pocket ID" }
            }
        },
    )
}

pub fn home(user: &User, projects: &[Project]) -> Markup {
    layout(
        "Projects",
        Some(user),
        html! {
            p.eyebrow { "Projects" }
            h1 { "Language families" }
            @if projects.is_empty() {
                div.empty {
                    "No projects yet. A project is a folder for one language "
                    "family — create one, then found its proto-language."
                }
            } @else {
                ul.cards {
                    @for p in projects {
                        li {
                            a href={ "/projects/" (p.id) } { (p.name) }
                            @if !p.description.is_empty() {
                                p.muted style="margin:.2rem 0 0" { (p.description) }
                            }
                        }
                    }
                }
            }
            h2 { "New project" }
            form.inline method="post" action="/projects" {
                input type="text" name="name" placeholder="e.g. The Vethric languages" required;
                button type="submit" { "Create project" }
            }
        },
    )
}

pub fn project_page(user: &User, project: &Project, languages: &[Language]) -> Markup {
    layout(
        &project.name,
        Some(user),
        html! {
            p.eyebrow { a href="/" class="muted" { "← All projects" } }
            h1 { (project.name) }
            @if languages.is_empty() {
                div.empty {
                    "This family is unfounded. The first language you create "
                    "here becomes its proto-language; everything after "
                    "descends from it."
                }
            } @else {
                ul.cards {
                    @for l in languages {
                        li {
                            a href={ "/languages/" (l.id) } { (l.name) }
                            @if l.parent_id.is_none() {
                                span.muted { " · proto-language" }
                            }
                        }
                    }
                }
            }
            h2 { "New language" }
            form.inline method="post" action={ "/projects/" (project.id) "/languages" } {
                input type="text" name="name" placeholder="Language name" required;
                button type="submit" { "Found language" }
            }
            p.muted style="font-size:.9rem" {
                "Next milestone: this button opens the phonology wizard "
                "(consonants → vowels → diphthongs → phonotactics → romanization)."
            }
        },
    )
}

pub fn language_page(user: &User, project: &Project, language: &Language) -> Markup {
    layout(
        &language.name,
        Some(user),
        html! {
            p.eyebrow {
                a href={ "/projects/" (project.id) } class="muted" { "← " (project.name) }
            }
            h1 { (language.name) }
            @if language.parent_id.is_none() {
                p.muted { "Proto-language of this family." }
            }
            form.inline method="get" action={ "/languages/" (language.id) "/phonology" } {
                button type="submit" { "Design the phonology →" }
            }
            div.empty {
                "Lexicon and the evolve menu land here in later milestones. "
                "The schema underneath is already shaped for them."
            }
        },
    )
}
