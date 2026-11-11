//! e621-compatible JSON API.

pub mod comments;
pub mod misc;
pub mod notes;
pub mod pools;
pub mod post_json;
pub mod posts;
pub mod tags;
pub mod users;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::AppState;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
    pub reason: Option<String>,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            reason: None,
        }
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, msg)
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "record not found")
    }

    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN, "Access Denied")
    }

    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "authentication required")
    }

    pub fn internal(e: impl std::fmt::Display) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({
            "success": false,
            "error": self.reason.unwrap_or_else(|| self.message.clone()),
            "message": self.message,
            "reason": self.message,
        });
        (self.status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

/// Extract a bare id from a path segment that may carry a `.json` suffix.
pub fn strip_json_suffix(s: &str) -> &str {
    s.trim_end_matches(".json")
}

pub fn ok_json(v: Value) -> ApiResult<Json<Value>> {
    Ok(Json(v))
}

/// The e621-compatible API surface. All routes are also reachable without
/// the `.json` suffix because `strip_json_suffix` is applied to path params
/// and the SPA fallback never claims these paths.
pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/posts.json", get(posts::list_posts).post(posts::create_post))
        .route("/posts", get(posts::list_posts).post(posts::create_post))
        .route("/posts/random.json", get(posts::random_post))
        .route(
            "/posts/:id",
            get(posts::show_post)
                .patch(posts::update_post)
                .put(posts::update_post)
                .delete(posts::delete_post),
        )
        .route("/posts/:id/votes.json", post(posts::vote_post))
        .route("/popular.json", get(posts::popular_posts))
        .route("/explore/posts/popular.json", get(posts::popular_posts))
        .route("/explore/posts/hot.json", get(posts::hot_posts))
        .route(
            "/favorites.json",
            get(posts::list_favorites).post(posts::add_favorite),
        )
        .route(
            "/favorites/:id",
            delete(posts::remove_favorite).get(posts::show_favorite_post),
        )
        .route(
            "/comments.json",
            get(comments::list_comments).post(comments::create_comment),
        )
        .route(
            "/comments/:id",
            get(comments::show_comment)
                .patch(comments::update_comment)
                .put(comments::update_comment)
                .delete(comments::delete_comment),
        )
        .route("/comments/:id/votes.json", post(comments::vote_comment))
        .route("/notes.json", get(notes::list_notes).post(notes::create_note))
        .route(
            "/notes/:id",
            get(notes::show_note)
                .put(notes::update_note)
                .patch(notes::update_note)
                .delete(notes::delete_note),
        )
        .route("/tags.json", get(tags::list_tags))
        .route("/tags/autocomplete.json", get(tags::autocomplete))
        .route("/tags/popular.json", get(tags::popular_tags))
        .route(
            "/tags/:id",
            get(tags::show_tag).patch(tags::update_tag).put(tags::update_tag),
        )
        .route("/tag_aliases.json", get(tags::list_aliases))
        .route("/tag_implications.json", get(tags::list_implications))
        .route("/pools.json", get(pools::list_pools).post(pools::create_pool))
        .route(
            "/pools/:id",
            get(pools::show_pool)
                .put(pools::update_pool)
                .patch(pools::update_pool)
                .delete(pools::delete_pool),
        )
        .route("/pools/:id/posts.json", get(pools::pool_posts))
        .route("/posts/:id/pools.json", get(pools::post_pools))
        .route("/users.json", get(users::list_users).post(users::create_user))
        .route(
            "/users/:id",
            get(users::show_user).patch(users::update_user),
        )
        .route("/session.json", post(users::login).delete(users::logout))
        .route("/session", post(users::login).delete(users::logout))
        .route("/uploads.json", post(posts::create_post))
        .route(
            "/post_flags.json",
            get(misc::list_flags).post(misc::create_flag),
        )
        .route(
            "/post_flags/:id",
            get(misc::show_flag).delete(misc::delete_flag),
        )
        .route("/tickets.json", get(misc::list_tickets).post(misc::create_ticket))
        .route("/tickets", get(misc::list_tickets).post(misc::create_ticket))
        .route("/post_counts.json", get(posts::post_counts))
        .route("/iqdb_queries.json", post(posts::iqdb_stub))
}
