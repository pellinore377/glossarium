-- Core schema. The load-bearing decision: lexemes belong to the PROTO
-- language of a family; daughter languages own no lexicon rows except
-- overrides in `reflexes`. Daughters are parent + ordered sound_changes.

CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    oidc_subject  TEXT NOT NULL UNIQUE,
    display_name  TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE projects (
    id          INTEGER PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_projects_user ON projects(user_id);

-- parent_id NULL = proto-language (family root). A project with no
-- languages gets its family started by the first language created in it.
CREATE TABLE languages (
    id          INTEGER PRIMARY KEY,
    project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    parent_id   INTEGER REFERENCES languages(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    -- Deeply nested, high-churn specs live as JSON; sqlite json1 can still
    -- reach into them when needed.
    phonology   TEXT NOT NULL DEFAULT '{}',   -- inventory, phonotactics, romanization
    grammar     TEXT NOT NULL DEFAULT '{}',   -- word order, morphology spec
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_languages_project ON languages(project_id);
CREATE INDEX idx_languages_parent ON languages(parent_id);

-- The ordered chain that turns parent into child. order_index is semantic:
-- feeding/bleeding relationships depend on it.
CREATE TABLE sound_changes (
    id           INTEGER PRIMARY KEY,
    language_id  INTEGER NOT NULL REFERENCES languages(id) ON DELETE CASCADE,
    order_index  INTEGER NOT NULL,
    catalog_ref  TEXT,                        -- id into the shipped catalog, NULL = custom
    rule_json    TEXT NOT NULL,               -- sca::Rule (or bundle) as JSON
    notes        TEXT NOT NULL DEFAULT '',
    UNIQUE (language_id, order_index)
);

-- Authored lexicon: always attached to the family's proto-language.
CREATE TABLE lexemes (
    id           INTEGER PRIMARY KEY,
    language_id  INTEGER NOT NULL REFERENCES languages(id) ON DELETE CASCADE,
    form_ipa     TEXT NOT NULL,
    gloss        TEXT NOT NULL,
    concept_ids  TEXT NOT NULL DEFAULT '[]',  -- JSON array; multi-ID = colexification-ready
    pos          TEXT NOT NULL,
    morph_class  TEXT NOT NULL DEFAULT '',
    notes        TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_lexemes_language ON lexemes(language_id);

-- Materialized derivation cache + irregularity escape hatch.
-- Invalidated (deleted) for the affected subtree whenever a rule chain or
-- an ancestor lexeme changes; re-derived lazily or by background task.
CREATE TABLE reflexes (
    lexeme_id     INTEGER NOT NULL REFERENCES lexemes(id) ON DELETE CASCADE,
    language_id   INTEGER NOT NULL REFERENCES languages(id) ON DELETE CASCADE,
    derived_form  TEXT NOT NULL,
    is_irregular  INTEGER NOT NULL DEFAULT 0,
    override_form TEXT,                       -- pinned form when is_irregular=1
    PRIMARY KEY (lexeme_id, language_id)
);
CREATE INDEX idx_reflexes_language ON reflexes(language_id);
