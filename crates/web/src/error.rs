use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Anyhow-backed error that renders as a 500 and logs the chain.
/// Handlers return `Result<impl IntoResponse, AppError>` and use `?` freely.
pub struct AppError(pub anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error = ?self.0, "request failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong. The details are in the server log.",
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        AppError(err.into())
    }
}
