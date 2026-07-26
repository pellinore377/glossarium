# Glossarium

A self-hosted workshop for constructed language families. Design a
proto-language's phonology, seed a lexicon, and evolve daughter languages
through documented sound changes — every derived form traceable to the rule
that produced it. Rust, Axum, Maud + HTMX, SQLite, Pocket ID for sign-in.
Fully deterministic: no LLMs, no external APIs at runtime.

## Architecture in one paragraph

A daughter language is stored as *its parent plus an ordered chain of sound
change rules*; its word forms are derived, never authored (with a per-word
irregularity override as the escape hatch). Phonemes are distinctive feature
bundles, not IPA strings, so every rule in the catalog carries a
machine-checkable applicability predicate — the evolve menu only offers
changes that make sense for the language at hand. Lexemes belong to the
family's proto-language; the `reflexes` table is a materialized cache,
invalidated when a rule chain changes.

## Workspace layout

    crates/
      phon   segments, features, IPA parsing; phonotactics next
      sca    sound change rules, ordered chains, applicability predicates
      lex    concept lists (Leipzig–Jakarta seed), lexeme model
      web    axum app: auth, routing, views, migrations

The linguistics crates have no HTTP anywhere near them and are plain
`cargo test` targets. The sound change catalog will ship as data files under
`data/` so new changes don't require recompiling.

## Running it

### 1. Register the app in Pocket ID

Administration → OIDC Clients → Add OIDC Client:

- **Callback URL**: `{BASE_URL}/auth/callback`
  (e.g. `https://conlang.example.com/auth/callback`)
- **PKCE**: enabled
- Copy the client ID and secret.

### 2. Configure

    cp .env.example .env
    # fill in OIDC_ISSUER_URL (Pocket ID's base URL), client id + secret

### 3. Run

    docker compose up --build

App on `:8080`, Pocket ID (if using the bundled service) on `:1411`. The
SQLite database lives in `./data/` on the host — that directory *is* your
backup surface.

If you already run Pocket ID elsewhere, delete its service from
`docker-compose.yml` and point `OIDC_ISSUER_URL` at your instance.

### Local development without an IdP

    AUTH_DISABLED=true cargo run -p web

Auto-logs you in as a local dev user. Refuse the temptation to deploy with
this set.

### Reverse proxy notes

Terminate TLS at your proxy (Caddy/Traefik/nginx), set `BASE_URL` to the
public HTTPS URL, and set `COOKIE_SECURE=true`. Passkeys are origin-bound,
so Pocket ID's `APP_URL` must exactly match the URL you visit it at.

## Static assets

The layout currently loads htmx from unpkg. For a fully offline server,
download `htmx.min.js` into `crates/web/static/` and swap the script tag —
a `ServeDir` for `/static` is one line in `main.rs` (tower-http `fs` feature
is already enabled).

## Build notes

The scaffold was written without a compiler pass, so treat the first
`cargo build` as a shakedown. The two places most likely to need a nudge:

- **openidconnect 4.x typestates** (`crates/web/src/auth.rs`): the
  `OidcClient` alias pins the endpoint typestate parameters in the order
  the discovery constructor produces them. If the compiler disagrees,
  it will tell you the exact type it wants — adjust the alias, nothing else.
- **maud / axum version pairing**: `maud 0.27` ↔ `axum 0.8`. If crates.io
  has moved on, keep the pair in lockstep.

## Roadmap (build vertically)

1. ✅ Auth + projects + languages shell (this scaffold)
2. Phonology wizard: consonants → vowels → diphthongs → **phonotactics** →
   romanization, with aesthetic presets and typological warnings
3. Lexicon: ~200 proto-roots seeded from Leipzig–Jakarta via the
   phonotactic generator; CRUD + FTS5 search
4. Sound change engine + 30–40 curated catalog entries with predicates;
   evolve flow with before/after preview  ← *the app becomes real here*
5. Morphology spec + realizer; paradigm tables; templated grammar sketch
6. Story realization from glossed skeletons; the same narrative rendered
   down every branch of the family
7. v1.5: IDS concept expansion toward 4,000 words, derivation engine,
   colexification. v2: tone. v3: triconsonantal roots.

## License

MIT
