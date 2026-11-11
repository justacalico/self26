//! /users and /session endpoints.

use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rusqlite::params;
use serde_json::{json, Value};

use crate::api::posts::read_post_form;
use crate::api::{strip_json_suffix, ApiError, ApiResult};
use crate::auth::{self, ApiAuth};
use crate::models::StoredUser;
use crate::AppState;

fn user_json(conn: &rusqlite::Connection, u: &StoredUser, full: bool) -> Value {
    let post_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM posts WHERE uploader_id = ?1 AND is_deleted = 0",
            params![u.id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let fav_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM favorites WHERE user_id = ?1",
            params![u.id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let comment_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM comments WHERE creator_id = ?1",
            params![u.id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let mut v = json!({
        "id": u.id,
        "created_at": u.created_at,
        "name": u.name,
        "level": u.level,
        "level_string": if u.is_admin() { "Admin" } else { "Member" },
        "base_upload_limit": 100,
        "post_upload_count": post_count,
        "post_update_count": 0,
        "note_update_count": 0,
        "is_banned": false,
        "can_approve_posts": u.is_admin(),
        "can_upload_free": u.is_admin(),
        "avatar_id": u.avatar_id,
        "fav_count": fav_count,
        "comment_count": comment_count,
        "blacklisted_tags": "",
    });
    if full {
        v["api_key"] = json!(u.api_key);
    }
    v
}

pub async fn list_users(
    State(state): State<AppState>,
    auth: ApiAuth,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    let is_admin = auth.user().map(|u| u.is_admin()).unwrap_or(false);
    let mut sql = "SELECT id FROM users".to_string();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(name) = params.get("search[name_matches]").or_else(|| params.get("name")) {
        args.push(Box::new(name.replace('*', "%")));
        sql.push_str(&format!(" WHERE name LIKE ?{} ESCAPE '\\'", args.len()));
    }
    sql.push_str(" ORDER BY id LIMIT 320");
    let mut stmt = conn.prepare(&sql).map_err(ApiError::internal)?;
    let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    let ids: Vec<i64> = stmt
        .query_map(refs.as_slice(), |r| r.get(0))
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    let items: Vec<Value> = ids
        .iter()
        .filter_map(|id| auth::get_user(&conn, *id))
        .map(|u| {
            let full = is_admin || auth.id() == Some(u.id);
            user_json(&conn, &u, full)
        })
        .collect();
    Ok(Json(json!(items)))
}

pub async fn show_user(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let raw = strip_json_suffix(&id);
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    let user = if raw == "current" || raw == "me" {
        auth.user().cloned().ok_or_else(ApiError::unauthorized)?
    } else if let Ok(id) = raw.parse::<i64>() {
        auth::get_user(&conn, id).ok_or_else(ApiError::not_found)?
    } else {
        auth::user_by_name(&conn, raw).ok_or_else(ApiError::not_found)?
    };
    let full = auth.user().map(|a| a.is_admin() || a.id == user.id).unwrap_or(false);
    Ok(Json(user_json(&conn, &user, full)))
}

pub async fn update_user(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
    request: axum::extract::Request,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id).parse().map_err(|_| ApiError::not_found())?;
    let actor = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    if actor.id != id && !actor.is_admin() {
        return Err(ApiError::forbidden());
    }
    let form = read_post_form(request).await?;
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    let user = auth::get_user(&conn, id).ok_or_else(ApiError::not_found)?;
    if let Some(pw) = form.get("password").or_else(|| form.get("user[password]")) {
        if !pw.is_empty() {
            conn.execute(
                "UPDATE users SET password_hash = ?1 WHERE id = ?2",
                params![auth::hash_password(pw), id],
            )
            .map_err(ApiError::internal)?;
        }
    }
    if actor.is_admin() {
        if let Some(level) = form.get("level").and_then(|s| s.parse::<i64>().ok()) {
            conn.execute(
                "UPDATE users SET level = ?1 WHERE id = ?2",
                params![level, id],
            )
            .map_err(ApiError::internal)?;
        }
    }
    Ok(Json(user_json(&conn, &user, true)))
}

/// Account creation — open registration for the first run, admin-only after
/// an admin exists (self-hosted instances should not be open signups).
pub async fn create_user(
    State(state): State<AppState>,
    auth: ApiAuth,
    request: axum::extract::Request,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let form = read_post_form(request).await?;
    let name = form
        .get("name")
        .or_else(|| form.get("user[name]"))
        .unwrap_or_default()
        .trim()
        .to_string();
    let password = form
        .get("password")
        .or_else(|| form.get("user[password]"))
        .unwrap_or_default()
        .to_string();
    if name.is_empty() || password.len() < 4 {
        return Err(ApiError::bad_request(
            "name required and password must be at least 4 characters",
        ));
    }
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    let user_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
        .unwrap_or(0);
    let first = user_count == 0;
    if !first && !auth.user().map(|u| u.is_admin()).unwrap_or(false) {
        return Err(ApiError::forbidden());
    }
    let level = if first {
        StoredUser::LEVEL_ADMIN
    } else {
        form.get("level")
            .and_then(|s| s.parse().ok())
            .unwrap_or(StoredUser::LEVEL_MEMBER)
    };
    let id = auth::create_user(&conn, &name, &password, level)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    let user = auth::get_user(&conn, id).ok_or_else(ApiError::not_found)?;
    Ok((StatusCode::CREATED, Json(user_json(&conn, &user, true))))
}

/// POST /session — log in and set the session cookie.
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Response> {
    let raw = String::from_utf8_lossy(&body);
    let (mut name, mut password) = (String::new(), String::new());
    if raw.trim_start().starts_with('{') {
        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
            name = v["name"].as_str().or_else(|| v["login"].as_str()).unwrap_or_default().to_string();
            password = v["password"].as_str().unwrap_or_default().to_string();
        }
    } else {
        for pair in raw.split('&') {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let key = crate::api::posts::urldec(k);
            let val = crate::api::posts::urldec(v);
            if matches!(key.as_str(), "name" | "login" | "session[name]") {
                name = val;
            } else if matches!(key.as_str(), "password" | "session[password]") {
                password = val;
            }
        }
    }
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    let user = auth::user_by_name(&conn, &name)
        .filter(|u| auth::verify_password(&password, &u.password_hash))
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "invalid login or password"))?;
    let token = auth::create_session(&conn, user.id).map_err(ApiError::internal)?;
    let secure = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        == Some("https");
    let cookie = format!(
        "self26_session={token}; Path=/; HttpOnly; SameSite=Lax{}",
        if secure { "; Secure" } else { "" }
    );
    let body = Json(json!({ "success": true, "user": user_json(&conn, &user, true) }));
    Ok(([(header::SET_COOKIE, cookie)], body).into_response())
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    if let Some(raw) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        for c in raw.split(';').map(str::trim) {
            if let Some(token) = c.strip_prefix("self26_session=") {
                let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
                let _ = conn.execute(
                    "DELETE FROM sessions WHERE token = ?1",
                    params![token],
                );
            }
        }
    }
    let cookie = "self26_session=; Path=/; HttpOnly; Max-Age=0".to_string();
    Ok(([(header::SET_COOKIE, cookie)], Json(json!({ "success": true }))).into_response())
}

/// Admin: regenerate a user's API key.
pub async fn reset_api_key(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id).parse().map_err(|_| ApiError::not_found())?;
    let actor = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    if actor.id != id && !actor.is_admin() {
        return Err(ApiError::forbidden());
    }
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    let key = auth::new_api_key();
    conn.execute(
        "UPDATE users SET api_key = ?1 WHERE id = ?2",
        params![key, id],
    )
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "success": true, "api_key": key })))
}


