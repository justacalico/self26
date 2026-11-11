//! Embedded Flutter web frontend. `scripts/build-flutter.sh` writes the
//! bundle into frontend/dist/ before the Rust binary is compiled so the
//! binary is fully self-contained.

use axum::body::Body;
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use include_dir::{include_dir, Dir};

use crate::AppState;

static FRONTEND_DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/frontend/dist");

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", any(|| async { serve_file("index.html") }))
        .fallback(spa_fallback)
}

async fn spa_fallback(uri: Uri) -> Response {
    let path = uri.path();
    if is_api_path(path) {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    let clean_path = path.trim_start_matches('/');
    if !clean_path.is_empty() {
        if let Some(resp) = try_serve_file(clean_path) {
            return resp;
        }
    }
    serve_file("index.html")
}

fn is_api_path(path: &str) -> bool {
    path.starts_with("/data/")
        || path.starts_with("/admin/api/")
        || path.starts_with("/.well-known/")
        || path == "/metrics"
        || path == "/ready"
        || path == "/live"
        || path.ends_with(".json")
}

fn try_serve_file(path: &str) -> Option<Response> {
    let file = FRONTEND_DIST.get_file(path)?;
    let mut resp = Response::new(Body::from(file.contents()));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&content_type_for(path)).unwrap(),
    );
    if let Ok(val) = HeaderValue::from_str(cache_control_for(path)) {
        resp.headers_mut().insert(header::CACHE_CONTROL, val);
    }
    Some(resp)
}

fn content_type_for(path: &str) -> String {
    if path == "manifest.json" {
        "application/manifest+json".to_string()
    } else {
        mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string()
    }
}

fn cache_control_for(path: &str) -> &'static str {
    if path == "index.html" {
        return "no-cache";
    }
    if path == "manifest.json"
        || path == "version.json"
        || path == "favicon.png"
        || path == "flutter_service_worker.js"
        || path.starts_with("icons/")
    {
        return "public, max-age=3600";
    }
    "public, max-age=31536000, immutable"
}

fn serve_file(path: &str) -> Response {
    try_serve_file(path).unwrap_or_else(|| {
        let mut resp = Response::new(Body::from("not found"));
        *resp.status_mut() = StatusCode::NOT_FOUND;
        resp
    })
}
