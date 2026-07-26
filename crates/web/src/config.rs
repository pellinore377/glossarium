use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    /// Public base URL of this app, no trailing slash, e.g. https://conlang.example.com
    pub base_url: String,
    pub bind_addr: String,
    pub database_url: String,
    /// Pocket ID base URL (it is its own OIDC issuer), e.g. https://id.example.com
    pub oidc_issuer_url: String,
    pub oidc_client_id: String,
    pub oidc_client_secret: String,
    /// Set true behind TLS (reverse proxy). Controls the session cookie's Secure flag.
    pub cookie_secure: bool,
    /// Local development escape hatch: skip OIDC entirely, log in as a dev user.
    /// Never enable in a deployed environment.
    pub auth_disabled: bool,
}

fn var(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("missing required env var {key}"))
}

fn var_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let auth_disabled = var_or("AUTH_DISABLED", "false") == "true";
        let (issuer, client_id, client_secret) = if auth_disabled {
            (String::new(), String::new(), String::new())
        } else {
            (
                var("OIDC_ISSUER_URL")?.trim_end_matches('/').to_string(),
                var("OIDC_CLIENT_ID")?,
                var("OIDC_CLIENT_SECRET")?,
            )
        };
        Ok(Config {
            base_url: var_or("BASE_URL", "http://localhost:8080")
                .trim_end_matches('/')
                .to_string(),
            bind_addr: var_or("BIND_ADDR", "0.0.0.0:8080"),
            database_url: var_or("DATABASE_URL", "sqlite://data/conlang.db"),
            oidc_issuer_url: issuer,
            oidc_client_id: client_id,
            oidc_client_secret: client_secret,
            cookie_secure: var_or("COOKIE_SECURE", "true") == "true",
            auth_disabled,
        })
    }

    pub fn redirect_url(&self) -> String {
        format!("{}/auth/callback", self.base_url)
    }
}
