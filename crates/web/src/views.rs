//! Maud views. Server-rendered shell + HTMX for partial swaps.
//!
//! Design direction (kept deliberately quiet at scaffold stage): historical
//! philology, not SaaS. Cool paper, iron-gall ink, one verdigris accent,
//! serifs for display, mono for anything in IPA. The signature element is
//! the family tree rendered as a descent column — it arrives with the
//! evolve milestone; nothing here should fight it.

use maud::{html, Markup, DOCTYPE};

use crate::auth::User;
use crate::ipa_chart::{self, Cell};
use crate::phonology::Phonology;
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
.ph {
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
.romgrid {
  display: grid; grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr));
  gap: .5rem; margin: .75rem 0 1.25rem;
}
.romcell {
  display: flex; align-items: center; gap: .5rem;
  background: var(--card); border: 1px solid var(--line);
  border-radius: 6px; padding: .4rem .6rem;
}
.romcell .psym {
  font: 500 1.05rem/1 "Gentium Plus", "Charis SIL", Gentium,
    "Times New Roman", serif;
  color: var(--faded); white-space: nowrap;
}
input.rom {
  font: 500 1.05rem/1.3 "Gentium Plus", "Charis SIL", Gentium,
    "Times New Roman", serif;
  width: 100%; min-width: 0; padding: .25rem .4rem;
  border: 1px solid transparent; border-radius: 4px;
  background: transparent; color: var(--accent-ink);
}
input.rom:hover { border-color: var(--line); }
input.rom:focus { border-color: var(--accent); outline: none; background: var(--paper); }
main:has(table.lex) { max-width: 68rem; }
.lexbar { margin: 1rem 0 .5rem; }
.lexbar input[type=search] {
  font: inherit; width: 100%; padding: .5rem .7rem;
  border: 1px solid var(--line); border-radius: 6px;
  background: var(--card); color: var(--ink);
}
form.addlex {
  display: flex; gap: .4rem; flex-wrap: wrap; margin: 0 0 1rem;
}
form.addlex input[type=text] { min-width: 0; flex: 1 1 9rem; }
form.addlex input.ph { flex: 1 1 8rem; }
table.lex { border-collapse: collapse; width: 100%; min-width: 44rem; }
table.lex th, table.lex td {
  border-bottom: 1px solid var(--line); padding: .45rem .6rem;
  text-align: left; vertical-align: baseline;
}
table.lex thead th {
  font: 500 .72rem/1.3 ui-monospace, monospace; letter-spacing: .1em;
  text-transform: uppercase; color: var(--faded);
}
table.lex td.gloss { font-weight: 500; }
table.lex td.notes { font-size: .88rem; max-width: 14rem; }
table.lex td.actions { white-space: nowrap; text-align: right; }
button.mini {
  font-size: .78rem; padding: .2rem .55rem; margin-left: .25rem;
}
form.rowedit { display: flex; gap: .4rem; flex-wrap: wrap; align-items: center; }
form.rowedit input[type=text] { min-width: 0; flex: 1 1 8rem; }
nav.langtabs {
  display: flex; gap: .25rem; flex-wrap: wrap; align-items: baseline;
  border-bottom: 1px solid var(--line); margin: 1.2rem 0 1.5rem;
}
nav.langtabs a, nav.langtabs span.soon {
  padding: .45rem .9rem; text-decoration: none; font-size: .92rem;
  border: 1px solid transparent; border-bottom: none;
  border-radius: 6px 6px 0 0;
}
nav.langtabs a { color: var(--accent-ink); }
nav.langtabs a:hover { background: var(--card); border-color: var(--line); }
nav.langtabs span.soon { color: var(--faded); font-size: .82rem; }
.symro {
  font: 600 1.05rem/1 "Gentium Plus", "Charis SIL", Gentium,
    "Times New Roman", serif;
  color: var(--accent-ink); padding: .15rem .25rem; display: inline-block;
}
.settings { margin-top: 2.5rem; border-top: 1px solid var(--line); padding-top: 1rem; }
form.danger button { background: #8a3232; border-color: #8a3232; }
ol.chain { margin: 1rem 0; padding-left: 1.6rem; }
ol.chain li {
  background: var(--card); border: 1px solid var(--line); border-radius: 6px;
  padding: .5rem .8rem; margin: .4rem 0;
  display: flex; align-items: center; justify-content: space-between; gap: .6rem;
}
ol.chain li::marker { color: var(--accent-ink); font-family: ui-monospace, monospace; }
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
                "Founding a language drops you straight into the phonology "
                "wizard: aesthetic → consonants → vowels → syllables → "
                "romanization."
            }
        },
    )
}

fn language_tabs(language: &Language, lexeme_count: i64, change_count: i64) -> Markup {
    html! {
        nav.langtabs {
            @if language.parent_id.is_some() {
                a href={ "/languages/" (language.id) "/changes" } {
                    "Sound changes (" (change_count) ")"
                }
                a href={ "/languages/" (language.id) "/lexicon" } { "Lexicon" }
            } @else {
                a href={ "/languages/" (language.id) "/phonology" } { "Phonology" }
                a href={ "/languages/" (language.id) "/lexicon" } {
                    "Lexicon"
                    @if lexeme_count > 0 { " (" (lexeme_count) ")" }
                }
            }
            span.soon { "Grammar · soon" }
            span.soon { "Stories · soon" }
            a href={ "/languages/" (language.id) "/settings" } { "Settings" }
        }
    }
}

fn row_has_selection(row: &ipa_chart::MannerRow, sel: &dyn Fn(&str) -> bool) -> bool {
    row.cells.iter().any(|c| {
        matches!(c, Cell::Sounds { vl, vd, .. }
            if vl.map(|s| sel(s)).unwrap_or(false) || vd.map(|s| sel(s)).unwrap_or(false))
    })
}

/// Read-only phoneme charts for a language's home page: the wizard's
/// layouts, stripped of interaction, showing only what was selected.
fn phoneme_charts(phonology: &Phonology) -> Markup {
    let cs = |s: &str| phonology.consonants.iter().any(|x| x == s);
    let vs = |s: &str| phonology.vowels.iter().any(|x| x == s);
    html! {
        @if !phonology.consonants.is_empty() {
            p.eyebrow { "Consonants (" (phonology.consonants.len()) ")" }
            div.chart-scroll {
                table.ipa {
                    thead {
                        tr {
                            th {}
                            @for p in ipa_chart::PLACES { th { (p) } }
                        }
                    }
                    tbody {
                        @for row in ipa_chart::CONSONANT_ROWS {
                            @if row_has_selection(row, &cs) {
                                tr {
                                    th.manner { (row.name) }
                                    @for cell in row.cells {
                                        @match cell {
                                            Cell::Sounds { span, vl, vd } => {
                                                td colspan=(span) {
                                                    @if let Some(s) = vl {
                                                        @if cs(s) { span.symro { (s) } }
                                                    }
                                                    @if let Some(s) = vd {
                                                        @if cs(s) { span.symro { (s) } }
                                                    }
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
            }
        }
        @if !phonology.vowels.is_empty() {
            p.eyebrow { "Vowels (" (phonology.vowels.len()) ")" }
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
                    @let show = p.unrounded.map(|s| vs(s)).unwrap_or(false)
                        || p.rounded.map(|s| vs(s)).unwrap_or(false);
                    @if show {
                        div.vpoint style={ "left:" (p.x) "%;top:" (p.y) "%" } {
                            @if let Some(s) = p.unrounded {
                                @if vs(s) { span.symro { (s) } }
                            }
                            @if let Some(s) = p.rounded {
                                @if vs(s) { span.symro { (s) } }
                            }
                        }
                    }
                }
            }
        }
        @if !phonology.diphthongs.is_empty() {
            p.eyebrow { "Diphthongs (" (phonology.diphthongs.len()) ")" }
            (diphthong_grid(&phonology.diphthongs))
        }
    }
}

/// Read-only nucleus × offglide grid, rows and columns limited to vowels
/// that actually participate in some diphthong.
fn diphthong_grid(diphthongs: &[String]) -> Markup {
    let mut nuclei: Vec<String> = Vec::new();
    let mut glides: Vec<String> = Vec::new();
    for d in diphthongs {
        let mut ch = d.chars();
        if let Some(n) = ch.next() {
            let n = n.to_string();
            if !nuclei.contains(&n) {
                nuclei.push(n);
            }
        }
        if let Some(g) = ch.next() {
            let g = g.to_string();
            if !glides.contains(&g) {
                glides.push(g);
            }
        }
    }
    nuclei.sort_by_key(|v| ipa_chart::vowel_order(v));
    glides.sort_by_key(|v| ipa_chart::vowel_order(v));
    let has = |n: &str, g: &str| diphthongs.iter().any(|d| d == &format!("{n}{g}"));

    html! {
        div.chart-scroll {
            table.ipa {
                thead {
                    tr {
                        th { span.muted { "nucleus ↓ glide →" } }
                        @for g in &glides { th { (g) } }
                    }
                }
                tbody {
                    @for n in &nuclei {
                        tr {
                            th.manner { (n) }
                            @for g in &glides {
                                @if n == g {
                                    td.x {}
                                } @else if has(n, g) {
                                    td { span.symro { (n) (g) } }
                                } @else {
                                    td {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn language_page(
    user: &User,
    project: &Project,
    language: &Language,
    phonology: &Phonology,
    lexeme_count: i64,
    change_count: i64,
) -> Markup {
    let wizard_done = !phonology.consonants.is_empty() && !phonology.vowels.is_empty();
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
            } @else {
                p.muted {
                    "Daughter language — its lexicon is derived from the "
                    "parent through its sound-change chain."
                }
            }
            (language_tabs(language, lexeme_count, change_count))

            @if language.parent_id.is_none() {
                @if wizard_done {
                    h2 { "Sound system" }
                    (phoneme_charts(phonology))
                    // Once the lexicon exists, its forms are built on this
                    // sound system — reopening the wizard would desync
                    // them, so the door quietly closes.
                    @if lexeme_count == 0 {
                        p.muted style="font-size:.9rem" {
                            a href={ "/languages/" (language.id) "/phonology" } {
                                "Edit the phonology →"
                            }
                        }
                    }
                } @else {
                    div.empty {
                        "No phonology yet. The wizard walks you from an "
                        "aesthetic through consonants, vowels, syllables, "
                        "and romanization."
                    }
                    form.inline method="get" action={ "/languages/" (language.id) "/phonology" } {
                        button type="submit" { "Design the phonology →" }
                    }
                }
            }

            h2 { "Evolve" }
            p.muted style="font-size:.9rem" {
                "A daughter starts as a perfect copy and drifts one sound "
                "change at a time. Daughters can have daughters — that's "
                "how a family tree grows."
            }
            form.inline method="post" action={ "/languages/" (language.id) "/evolve" } {
                input type="text" name="name" placeholder="Daughter language name" required;
                button type="submit" { "Evolve a daughter →" }
            }
        },
    )
}

pub fn language_settings_page(user: &User, language: &Language) -> Markup {
    layout(
        "Settings",
        Some(user),
        html! {
            p.eyebrow {
                a href={ "/languages/" (language.id) } class="muted" { "← " (language.name) }
            }
            h1 { (language.name) ": settings" }
            h2 { "Rename" }
            form.inline method="post" action={ "/languages/" (language.id) "/rename" } {
                input type="text" name="name" value=(language.name) required;
                button.quiet type="submit" { "Rename" }
            }
            div.settings {
                p.eyebrow { "Danger" }
                p.muted style="font-size:.9rem" {
                    "Deleting a language deletes every daughter descended "
                    "from it, along with their sound changes"
                    @if language.parent_id.is_none() { " — and, for a proto-language, the family's entire lexicon" }
                    ". There is no undo."
                }
                form.inline.danger method="post"
                    action={ "/languages/" (language.id) "/delete" }
                    onsubmit="return confirm('Delete this language and every daughter descended from it? This cannot be undone.')" {
                    button type="submit" { "Delete language" }
                }
            }
        },
    )
}
