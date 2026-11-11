//! self26 — a self-hosted, e621-compatible media server.

pub mod api;
pub mod assets;
pub mod auth;
pub mod db;
pub mod media;
pub mod models;
pub mod search;

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::db::DbRef;
use crate::media::MediaProcessor;

#[derive(Clone)]
pub struct AppState {
    pub db: DbRef,
    pub media: Arc<MediaProcessor>,
    pub base_url: String,
    pub local_mode: bool,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/.well-known/self26", get(identity))
        .route("/health.json", get(health))
        .route("/live", get(|| async { "ok" }))
        .route("/ready", get(|| async { "ok" }))
        .route("/metrics", get(metrics))
        .route("/data/:kind/:prefix/:file", get(serve_media))
        .route("/admin/api/stats.json", get(api::misc::admin_stats))
        .route(
            "/admin/api/relations/:kind.json",
            axum::routing::post(api::misc::admin_create_relation),
        )
        .route(
            "/admin/api/flags/:id/resolve.json",
            axum::routing::post(api::misc::admin_resolve_flag),
        )
        .route(
            "/admin/api/tickets/:id/resolve.json",
            axum::routing::post(api::misc::admin_resolve_ticket),
        )
        .route(
            "/admin/api/users/:id/reset_key.json",
            axum::routing::post(api::users::reset_api_key),
        )
        .merge(api::api_router())
        .merge(assets::router())
        .with_state(state)
}

async fn identity() -> Json<Value> {
    Json(json!({
        "name": "self26",
        "version": env!("CARGO_PKG_VERSION"),
        "api": "e621-compatible",
        "v2": true,
    }))
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    let posts: i64 = state
        .db
        .lock()
        .ok()
        .and_then(|c| {
            c.query_row("SELECT COUNT(*) FROM posts WHERE is_deleted = 0", [], |r| {
                r.get(0)
            })
            .ok()
        })
        .unwrap_or(0);
    Json(json!({
        "posts": posts,
        "version": env!("CARGO_PKG_VERSION"),
        "ok": true,
    }))
}

async fn metrics(State(state): State<AppState>) -> String {
    let guard = state.db.lock().ok();
    let count = |sql: &str| -> i64 {
        guard
            .as_ref()
            .and_then(|c| c.query_row(sql, [], |r| r.get(0)).ok())
            .unwrap_or(0)
    };
    format!(
        "self26_posts {}\nself26_users {}\nself26_tags {}\nself26_comments {}\n",
        count("SELECT COUNT(*) FROM posts WHERE is_deleted = 0"),
        count("SELECT COUNT(*) FROM users"),
        count("SELECT COUNT(*) FROM tags"),
        count("SELECT COUNT(*) FROM comments"),
    )
}

async fn serve_media(
    State(state): State<AppState>,
    Path((kind, prefix, file)): Path<(String, String, String)>,
) -> Response {
    if !matches!(kind.as_str(), "original" | "preview" | "sample")
        || file.contains("..")
        || prefix.len() != 2
        || !prefix.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    let path = state.media.data_dir.join(&kind).join(&prefix).join(&file);
    if !path.starts_with(state.media.data_dir.join(&kind)) {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            let mut resp = Response::new(axum::body::Body::from(bytes));
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref()).unwrap(),
            );
            resp.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            resp
        }
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

pub fn default_data_dir() -> PathBuf {
    std::env::var("SELF26_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs_home().join(".local/share/self26")
        })
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
