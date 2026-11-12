//! /notes endpoints.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rusqlite::params;
use serde_json::{json, Value};

use crate::api::posts::read_post_form;
use crate::api::{strip_json_suffix, ApiError, ApiResult};
use crate::auth::ApiAuth;
use crate::db;
use crate::models::StoredNote;
use crate::AppState;

fn note_json(conn: &rusqlite::Connection, n: &StoredNote) -> Value {
    json!({
        "id": n.id,
        "created_at": n.created_at,
        "updated_at": n.updated_at,
        "creator_id": n.creator_id,
        "creator_name": db::user_name(conn, n.creator_id),
        "x": n.x,
        "y": n.y,
        "width": n.width,
        "height": n.height,
        "version": 1,
        "is_active": n.is_active,
        "post_id": n.post_id,
        "body": n.body,
    })
}

fn row_to_note(r: &rusqlite::Row) -> rusqlite::Result<StoredNote> {
    Ok(StoredNote {
        id: r.get("id")?,
        post_id: r.get("post_id")?,
        creator_id: r.get("creator_id")?,
        x: r.get("x")?,
        y: r.get("y")?,
        width: r.get("width")?,
        height: r.get("height")?,
        body: r.get("body")?,
        is_active: r.get::<_, i64>("is_active")? != 0,
        created_at: db::parse_ts(&r.get::<_, String>("created_at")?),
        updated_at: db::parse_ts(&r.get::<_, String>("updated_at")?),
    })
}

fn get_note(conn: &rusqlite::Connection, id: i64) -> Option<StoredNote> {
    conn.query_row(
        "SELECT * FROM notes WHERE id = ?1",
        params![id],
        row_to_note,
    )
    .ok()
}

pub async fn list_notes(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let mut sql = "SELECT * FROM notes WHERE is_active = 1".to_string();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(pid) = params
        .get("search[post_id]")
        .or_else(|| params.get("post_id"))
        .and_then(|s| s.parse::<i64>().ok())
    {
        args.push(Box::new(pid));
        sql.push_str(&format!(" AND post_id = ?{}", args.len()));
    }
    if let Some(body) = params.get("search[body_matches]") {
        args.push(Box::new(body.replace('*', "%")));
        sql.push_str(&format!(" AND body LIKE ?{} ESCAPE '\\'", args.len()));
    }
    sql.push_str(" ORDER BY id DESC LIMIT 320");
    let mut stmt = conn.prepare(&sql).map_err(ApiError::internal)?;
    let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    let notes: Vec<StoredNote> = stmt
        .query_map(refs.as_slice(), row_to_note)
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    let items: Vec<Value> = notes.iter().map(|n| note_json(&conn, n)).collect();
    Ok(Json(json!(items)))
}

pub async fn show_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let n = get_note(&conn, id).ok_or_else(ApiError::not_found)?;
    Ok(Json(note_json(&conn, &n)))
}

fn note_fields(form: &crate::api::posts::PostForm, key: &str) -> Option<String> {
    form.get(key)
        .or_else(|| form.get(&format!("note[{key}]")))
        .map(String::from)
}

pub async fn create_note(
    State(state): State<AppState>,
    auth: ApiAuth,
    request: axum::extract::Request,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let form = read_post_form(request).await?;
    let post_id: i64 = note_fields(&form, "post_id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ApiError::bad_request("post_id required"))?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let post = db::get_post(&conn, post_id).ok_or_else(ApiError::not_found)?;
    if post.is_deleted || (post.note_locked && !user.is_admin()) {
        return Err(ApiError::forbidden());
    }
    let num = |k: &str, def: i64| {
        note_fields(&form, k)
            .and_then(|s| s.parse().ok())
            .unwrap_or(def)
    };
    let now = db::ts(Utc::now());
    conn.execute(
        "INSERT INTO notes (post_id, creator_id, x, y, width, height, body,
            is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?8)",
        params![
            post_id,
            user.id,
            num("x", 0),
            num("y", 0),
            num("width", 10),
            num("height", 10),
            note_fields(&form, "body").unwrap_or_default(),
            now
        ],
    )
    .map_err(ApiError::internal)?;
    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(ApiError::internal)?;
    let n = get_note(&conn, id).ok_or_else(ApiError::not_found)?;
    Ok((StatusCode::CREATED, Json(note_json(&conn, &n))))
}

pub async fn update_note(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
    request: axum::extract::Request,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let form = read_post_form(request).await?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let note = get_note(&conn, id).ok_or_else(ApiError::not_found)?;
    if note.creator_id != user.id && !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    let post = db::get_post(&conn, note.post_id).ok_or_else(ApiError::not_found)?;
    if post.note_locked && !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    for (key, col) in [
        ("x", "x"),
        ("y", "y"),
        ("width", "width"),
        ("height", "height"),
    ] {
        if let Some(v) = note_fields(&form, key).and_then(|s| s.parse::<i64>().ok()) {
            conn.execute(
                &format!("UPDATE notes SET {col} = ?1 WHERE id = ?2"),
                params![v, id],
            )
            .map_err(ApiError::internal)?;
        }
    }
    if let Some(body) = note_fields(&form, "body") {
        conn.execute(
            "UPDATE notes SET body = ?1 WHERE id = ?2",
            params![body, id],
        )
        .map_err(ApiError::internal)?;
    }
    if let Some(active) = note_fields(&form, "is_active") {
        conn.execute(
            "UPDATE notes SET is_active = ?1 WHERE id = ?2",
            params![matches!(active.as_str(), "1" | "true") as i64, id],
        )
        .map_err(ApiError::internal)?;
    }
    conn.execute(
        "UPDATE notes SET updated_at = ?1 WHERE id = ?2",
        params![db::ts(Utc::now()), id],
    )
    .map_err(ApiError::internal)?;
    let n = get_note(&conn, id).ok_or_else(ApiError::not_found)?;
    Ok(Json(note_json(&conn, &n)))
}

pub async fn delete_note(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let n = get_note(&conn, id).ok_or_else(ApiError::not_found)?;
    if n.creator_id != user.id && !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    let post = db::get_post(&conn, n.post_id).ok_or_else(ApiError::not_found)?;
    if post.note_locked && !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    conn.execute(
        "UPDATE notes SET is_active = 0, updated_at = ?1 WHERE id = ?2",
        params![db::ts(Utc::now()), id],
    )
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "success": true })))
}
