mod auth;
mod config;
mod error;
mod ipa_chart;
mod lexicon;
mod phonology;
mod phonotactics;
mod romanization;
mod routes;
mod state;
mod typology;
mod views;

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use time::Duration;
use tower_http::trace::TraceLayer;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

use crate::{config::Config, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let cfg = Config::from_env()?;

    let opts = SqliteConnectOptions::from_str(&cfg.database_url)
        .context("bad DATABASE_URL")?
        .create_if_missing(true)
        .foreign_keys(true);
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
        .context("opening sqlite database")?;
    sqlx::migrate!("./migrations").run(&db).await?;

    // Redirects must not be followed silently during OIDC calls.
    let http = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let mut state = AppState {
        cfg: cfg.clone(),
        db,
        http,
        oidc_metadata: None,
    };
    if !cfg.auth_disabled {
        state.oidc_metadata = Some(auth::discover(&state).await?);
        tracing::info!(issuer = %cfg.oidc_issuer_url, "OIDC discovery OK");
    } else {
        tracing::warn!("AUTH_DISABLED=true — dev mode, do not deploy like this");
    }

    // In-memory sessions: fine for a home server; logins survive until the
    // process restarts, and signing back in is one passkey tap. Swap in
    // tower-sessions-sqlx-store if that ever annoys you.
    let session_layer = SessionManagerLayer::new(MemoryStore::default())
        .with_secure(cfg.cookie_secure)
        .with_expiry(Expiry::OnInactivity(Duration::days(14)));

    let app = Router::new()
        .route("/", get(routes::home))
        .route("/auth/login", get(auth::login))
        .route("/auth/callback", get(auth::callback))
        .route("/auth/logout", post(auth::logout))
        .route("/projects", post(routes::create_project))
        .route("/projects/{id}", get(routes::show_project))
        .route("/projects/{id}/languages", post(routes::create_language))
        .route("/languages/{id}", get(routes::show_language))
        .route("/languages/{id}/phonology", get(phonology::aesthetic_page))
        .route(
            "/languages/{id}/phonology/aesthetic",
            post(phonology::choose_aesthetic),
        )
        .route(
            "/languages/{id}/phonology/consonants",
            get(phonology::consonants_page),
        )
        .route(
            "/languages/{id}/phonology/consonants/toggle",
            post(phonology::toggle_consonant),
        )
        .route(
            "/languages/{id}/phonology/vowels",
            get(phonology::vowels_page),
        )
        .route(
            "/languages/{id}/phonology/vowels/toggle",
            post(phonology::toggle_vowel),
        )
        .route(
            "/languages/{id}/phonology/diphthongs",
            get(phonology::diphthongs_page),
        )
        .route(
            "/languages/{id}/phonology/diphthongs/toggle",
            post(phonology::toggle_diphthong),
        )
        .route(
            "/languages/{id}/phonology/phonotactics",
            get(phonology::phonotactics_page),
        )
        .route(
            "/languages/{id}/phonology/phonotactics/preset",
            post(phonology::choose_syllable_preset),
        )
        .route(
            "/languages/{id}/phonology/phonotactics/set",
            post(phonology::set_phonotactics),
        )
        .route(
            "/languages/{id}/phonology/stress",
            get(phonology::stress_page).post(phonology::choose_stress),
        )
        .route(
            "/languages/{id}/phonology/romanization",
            get(phonology::romanization_page),
        )
        .route(
            "/languages/{id}/phonology/romanization/set",
            post(phonology::set_romanization),
        )
        .route(
            "/languages/{id}/phonology/summary",
            get(phonology::summary_page),
        )
        .route(
            "/languages/{id}/lexicon",
            get(lexicon::lexicon_page).post(lexicon::create_lexeme),
        )
        .route("/languages/{id}/lexicon/seed", post(lexicon::seed_lexicon))
        .route(
            "/languages/{id}/lexicon/search",
            get(lexicon::search_lexicon),
        )
        .route("/lexemes/{id}", post(lexicon::update_lexeme))
        .route("/lexemes/{id}/edit", get(lexicon::lexeme_edit_row))
        .route("/lexemes/{id}/row", get(lexicon::lexeme_display_row))
        .route("/lexemes/{id}/delete", post(lexicon::delete_lexeme))
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    tracing::info!("listening on {} ({})", cfg.bind_addr, cfg.base_url);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
