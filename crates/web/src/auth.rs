//! OIDC against Pocket ID: authorization-code + PKCE, nonce-checked
//! ID token, local user provisioned on first login keyed by the `sub` claim.
//!
//! Pocket ID setup (Administration → OIDC Clients → Add):
//!   - Callback URL: {BASE_URL}/auth/callback
//!   - PKCE: enabled
//!   - copy client ID + secret into the env

use anyhow::{anyhow, Context, Result};
use axum::{
    extract::{Query, State},
    response::Redirect,
};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata},
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet,
    EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenResponse,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{error::AppError, state::AppState};

type OidcClient = CoreClient<
    EndpointSet,      // auth url: present after discovery
    EndpointNotSet,   // device auth
    EndpointNotSet,   // introspection
    EndpointNotSet,   // revocation
    EndpointMaybeSet, // token url
    EndpointMaybeSet, // userinfo url
>;

const SESSION_USER_ID: &str = "user_id";
const SESSION_CSRF: &str = "oidc_csrf";
const SESSION_NONCE: &str = "oidc_nonce";
const SESSION_PKCE: &str = "oidc_pkce";

pub async fn discover(state: &AppState) -> Result<CoreProviderMetadata> {
    CoreProviderMetadata::discover_async(
        IssuerUrl::new(state.cfg.oidc_issuer_url.clone())?,
        &state.http,
    )
    .await
    .context("OIDC discovery against Pocket ID failed — check OIDC_ISSUER_URL")
}

fn client(state: &AppState) -> Result<OidcClient> {
    let metadata = state
        .oidc_metadata
        .as_ref()
        .ok_or_else(|| anyhow!("OIDC metadata missing (auth disabled?)"))?
        .clone();
    Ok(CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(state.cfg.oidc_client_id.clone()),
        Some(ClientSecret::new(state.cfg.oidc_client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(state.cfg.redirect_url())?))
}

/// GET /auth/login — kick off the code flow (or dev-mode instant login).
pub async fn login(
    State(state): State<AppState>,
    session: Session,
) -> Result<Redirect, AppError> {
    if state.cfg.auth_disabled {
        let user_id = upsert_user(&state, "dev-local", "Local developer").await?;
        session.insert(SESSION_USER_ID, user_id).await?;
        return Ok(Redirect::to("/"));
    }

    let client = client(&state)?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".into()))
        .add_scope(Scope::new("profile".into()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    session.insert(SESSION_CSRF, csrf.secret().clone()).await?;
    session.insert(SESSION_NONCE, nonce.secret().clone()).await?;
    session
        .insert(SESSION_PKCE, pkce_verifier.secret().clone())
        .await?;

    Ok(Redirect::to(auth_url.as_str()))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

/// GET /auth/callback — verify state, exchange code, verify ID token, log in.
pub async fn callback(
    State(state): State<AppState>,
    session: Session,
    Query(q): Query<CallbackQuery>,
) -> Result<Redirect, AppError> {
    let stored_csrf: String = session
        .remove(SESSION_CSRF)
        .await?
        .ok_or_else(|| anyhow!("no login in progress"))?;
    if stored_csrf != q.state {
        return Err(anyhow!("OIDC state mismatch").into());
    }
    let nonce: String = session
        .remove(SESSION_NONCE)
        .await?
        .ok_or_else(|| anyhow!("missing nonce"))?;
    let pkce: String = session
        .remove(SESSION_PKCE)
        .await?
        .ok_or_else(|| anyhow!("missing PKCE verifier"))?;

    let client = client(&state)?;
    let tokens = client
        .exchange_code(AuthorizationCode::new(q.code))
        .map_err(|e| anyhow!("token endpoint not configured: {e}"))?
        .set_pkce_verifier(PkceCodeVerifier::new(pkce))
        .request_async(&state.http)
        .await
        .context("code-for-token exchange failed")?;

    let id_token = tokens
        .id_token()
        .ok_or_else(|| anyhow!("Pocket ID returned no ID token"))?;
    let claims = id_token
        .claims(&client.id_token_verifier(), &Nonce::new(nonce))
        .context("ID token verification failed")?;

    let subject = claims.subject().as_str().to_string();
    let display_name = claims
        .preferred_username()
        .map(|u| u.as_str().to_string())
        .or_else(|| {
            claims
                .name()
                .and_then(|n| n.get(None))
                .map(|n| n.as_str().to_string())
        })
        .or_else(|| claims.email().map(|e| e.as_str().to_string()))
        .unwrap_or_else(|| "Unnamed user".to_string());

    let user_id = upsert_user(&state, &subject, &display_name).await?;

    // Rotate the session id on privilege change, then attach the user.
    session.cycle_id().await?;
    session.insert(SESSION_USER_ID, user_id).await?;

    // Access/refresh tokens are deliberately dropped: the app runs on its
    // own session from here and never calls Pocket ID's APIs on the
    // user's behalf.
    let _ = tokens.access_token();

    Ok(Redirect::to("/"))
}

/// POST /auth/logout
pub async fn logout(session: Session) -> Result<Redirect, AppError> {
    session.delete().await?;
    Ok(Redirect::to("/"))
}

async fn upsert_user(state: &AppState, subject: &str, display_name: &str) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO users (oidc_subject, display_name) VALUES (?, ?)
         ON CONFLICT(oidc_subject) DO UPDATE SET display_name = excluded.display_name
         RETURNING id",
    )
    .bind(subject)
    .bind(display_name)
    .fetch_one(&state.db)
    .await?;
    Ok(row.0)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub display_name: String,
}

/// Load the current user, or None if the session is anonymous.
pub async fn current_user(state: &AppState, session: &Session) -> Result<Option<User>> {
    let Some(user_id) = session.get::<i64>(SESSION_USER_ID).await? else {
        return Ok(None);
    };
    let user = sqlx::query_as::<_, User>("SELECT id, display_name FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?;
    Ok(user)
}
