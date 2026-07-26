use openidconnect::core::CoreProviderMetadata;
use sqlx::SqlitePool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub db: SqlitePool,
    pub http: reqwest::Client,
    /// Discovered once at startup; None only when AUTH_DISABLED=true.
    pub oidc_metadata: Option<CoreProviderMetadata>,
}
