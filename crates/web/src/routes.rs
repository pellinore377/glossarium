use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use maud::Markup;
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    auth::{current_user, User},
    error::AppError,
    state::AppState,
    views,
};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Language {
    pub id: i64,
    pub project_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
}

/// Either the signed-in user, or a redirect to the landing flow.
async fn require_user(state: &AppState, session: &Session) -> Result<Result<User, Response>, AppError> {
    match current_user(state, session).await? {
        Some(u) => Ok(Ok(u)),
        None => Ok(Err(views::landing().into_response())),
    }
}

/// GET /
pub async fn home(
    State(state): State<AppState>,
    session: Session,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let projects = sqlx::query_as::<_, Project>(
        "SELECT id, name, description FROM projects WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(views::home(&user, &projects).into_response())
}

#[derive(Deserialize)]
pub struct CreateProject {
    name: String,
}

/// POST /projects
pub async fn create_project(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<CreateProject>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let name = form.name.trim();
    if name.is_empty() {
        return Ok(Redirect::to("/").into_response());
    }
    let row: (i64,) =
        sqlx::query_as("INSERT INTO projects (user_id, name) VALUES (?, ?) RETURNING id")
            .bind(user.id)
            .bind(name)
            .fetch_one(&state.db)
            .await?;
    Ok(Redirect::to(&format!("/projects/{}", row.0)).into_response())
}

/// Fetch a project only if it belongs to this user — ownership check on
/// every project-scoped route, single-user deployment or not.
async fn owned_project(state: &AppState, user: &User, id: i64) -> Result<Project, AppError> {
    sqlx::query_as::<_, Project>(
        "SELECT id, name, description FROM projects WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError(anyhow!("project {id} not found for this user")))
}

/// GET /projects/{id}
pub async fn show_project(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let project = owned_project(&state, &user, id).await?;
    let languages = sqlx::query_as::<_, Language>(
        "SELECT id, project_id, parent_id, name FROM languages
         WHERE project_id = ? ORDER BY parent_id IS NOT NULL, created_at",
    )
    .bind(project.id)
    .fetch_all(&state.db)
    .await?;
    Ok(views::project_page(&user, &project, &languages).into_response())
}

#[derive(Deserialize)]
pub struct CreateLanguage {
    name: String,
}

/// POST /projects/{id}/languages
///
/// Requirement 4 lives here: an empty project's first language is founded
/// with parent_id = NULL and becomes the family's proto-language. Later
/// languages are created through the evolve flow and carry a parent.
pub async fn create_language(
    State(state): State<AppState>,
    session: Session,
    Path(project_id): Path<i64>,
    Form(form): Form<CreateLanguage>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let project = owned_project(&state, &user, project_id).await?;
    let name = form.name.trim();
    if name.is_empty() {
        return Ok(Redirect::to(&format!("/projects/{}", project.id)).into_response());
    }
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO languages (project_id, parent_id, name) VALUES (?, NULL, ?) RETURNING id",
    )
    .bind(project.id)
    .bind(name)
    .fetch_one(&state.db)
    .await?;
    Ok(Redirect::to(&format!("/languages/{}", row.0)).into_response())
}

/// GET /languages/{id}
pub async fn show_language(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = match require_user(&state, &session).await? {
        Ok(u) => u,
        Err(landing) => return Ok(landing),
    };
    let language = sqlx::query_as::<_, Language>(
        "SELECT l.id, l.project_id, l.parent_id, l.name
         FROM languages l JOIN projects p ON p.id = l.project_id
         WHERE l.id = ? AND p.user_id = ?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError(anyhow!("language {id} not found for this user")))?;
    let project = owned_project(&state, &user, language.project_id).await?;
    let (lexeme_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM lexemes WHERE language_id = ?")
            .bind(language.id)
            .fetch_one(&state.db)
            .await?;
    let (change_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sound_changes WHERE language_id = ?")
            .bind(language.id)
            .fetch_one(&state.db)
            .await?;
    Ok(
        views::language_page(&user, &project, &language, lexeme_count, change_count)
            .into_response(),
    )
}

/// Convenience so views can be returned directly from small handlers later.
#[allow(dead_code)]
pub fn markup(m: Markup) -> Response {
    m.into_response()
}
