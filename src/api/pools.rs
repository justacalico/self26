//! /pools endpoints.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use rusqlite::params;
use serde_json::{json, Value};

use crate::api::post_json::{post_json, PostFormat};
use crate::api::posts::read_post_form;
use crate::api::{strip_json_suffix, ApiError, ApiResult};
use crate::auth::ApiAuth;
use crate::db;
use crate::models::StoredPool;
use crate::AppState;

fn pool_json(conn: &rusqlite::Connection, p: &StoredPool) -> Value {
    json!({
        "id": p.id,
        "name": p.name,
        "created_at": p.created_at,
        "updated_at": p.updated_at,
        "creator_id": p.creator_id,
        "creator_name": db::user_name(conn, p.creator_id),
        "description": p.description,
        "is_active": p.is_active,
        "is_deleted": p.is_deleted,
        "category": p.category,
        "post_ids": p.post_ids,
        "post_count": p.post_ids.len(),
    })
}

fn get_pool(conn: &rusqlite::Connection, id: i64) -> Option<StoredPool> {
    let mut pool = conn
        .query_row(
            "SELECT id, name, description, creator_id, category, is_active,
                    is_deleted, created_at, updated_at FROM pools WHERE id = ?1",
            params![id],
            |r| {
                Ok(StoredPool {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    description: r.get(2)?,
                    creator_id: r.get(3)?,
                    category: r.get(4)?,
                    is_active: r.get::<_, i64>(5)? != 0,
                    is_deleted: r.get::<_, i64>(6)? != 0,
                    created_at: db::parse_ts(&r.get::<_, String>(7)?),
                    updated_at: db::parse_ts(&r.get::<_, String>(8)?),
                    post_ids: vec![],
                })
            },
        )
        .ok()?;
    pool.post_ids = conn
        .prepare("SELECT post_id FROM pool_posts WHERE pool_id = ?1 ORDER BY ord, post_id")
        .and_then(|mut s| {
            let rows = s.query_map(params![id], |r| r.get(0))?;
            Ok(rows.flatten().collect())
        })
        .unwrap_or_default();
    Some(pool)
}

pub async fn list_pools(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let mut sql = "SELECT id FROM pools WHERE is_deleted = 0".to_string();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(name) = params
        .get("search[name_matches]")
        .or_else(|| params.get("search[name]"))
    {
        args.push(Box::new(name.replace('*', "%")));
        sql.push_str(&format!(" AND name LIKE ?{} ESCAPE '\\'", args.len()));
    }
    sql.push_str(" ORDER BY id DESC LIMIT 320");
    let mut stmt = conn.prepare(&sql).map_err(ApiError::internal)?;
    let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    let ids: Vec<i64> = stmt
        .query_map(refs.as_slice(), |r| r.get(0))
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    let items: Vec<Value> = ids
        .iter()
        .filter_map(|id| get_pool(&conn, *id))
        .map(|p| pool_json(&conn, &p))
        .collect();
    Ok(Json(json!(items)))
}

pub async fn show_pool(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let pool = get_pool(&conn, id).ok_or_else(ApiError::not_found)?;
    Ok(Json(pool_json(&conn, &pool)))
}

pub async fn pool_posts(
    State(state): State<AppState>,
    auth: ApiAuth,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let format = PostFormat::from_params(
        params.get("v2").map(String::as_str),
        params.get("mode").map(String::as_str),
    );
    let base = crate::api::posts::base_url_pub(&headers, &state.base_url);
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let pool = get_pool(&conn, id).ok_or_else(ApiError::not_found)?;
    let items: Vec<Value> = pool
        .post_ids
        .iter()
        .filter_map(|pid| db::get_post(&conn, *pid))
        .filter(|p| !p.is_deleted)
        .map(|p| post_json(&conn, &p, &base, auth.id(), format))
        .collect();
    if format.is_v2() {
        Ok(Json(Value::Array(items)))
    } else {
        Ok(Json(json!({ "posts": items })))
    }
}

pub async fn post_pools(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let items: Vec<Value> = db::post_pools(&conn, id)
        .iter()
        .filter_map(|pid| get_pool(&conn, *pid))
        .map(|p| pool_json(&conn, &p))
        .collect();
    Ok(Json(json!(items)))
}

pub async fn create_pool(
    State(state): State<AppState>,
    auth: ApiAuth,
    request: axum::extract::Request,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let form = read_post_form(request).await?;
    let name = form
        .get("name")
        .or_else(|| form.get("pool[name]"))
        .unwrap_or_default()
        .trim()
        .to_lowercase()
        .replace(' ', "_");
    if name.is_empty() {
        return Err(ApiError::bad_request("name required"));
    }
    let description = form
        .get("description")
        .or_else(|| form.get("pool[description]"))
        .unwrap_or_default()
        .to_string();
    let category = match form
        .get("category")
        .or_else(|| form.get("pool[category]"))
        .unwrap_or("series")
    {
        "collection" => "collection",
        _ => "series",
    };
    let now = db::ts(Utc::now());
    let conn = state.db.lock().map_err(ApiError::internal)?;
    conn.execute(
        "INSERT INTO pools (name, description, creator_id, category, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![name, description, user.id, category, now],
    )
    .map_err(|e| ApiError::bad_request(e.to_string()))?;
    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(ApiError::internal)?;
    if let Some(ids) = form.get("post_ids").or_else(|| form.get("pool[post_ids]")) {
        set_pool_posts(&conn, id, ids)?;
    }
    let pool = get_pool(&conn, id).ok_or_else(ApiError::not_found)?;
    Ok((StatusCode::CREATED, Json(pool_json(&conn, &pool))))
}

fn set_pool_posts(conn: &rusqlite::Connection, pool_id: i64, ids_raw: &str) -> ApiResult<()> {
    conn.execute(
        "DELETE FROM pool_posts WHERE pool_id = ?1",
        params![pool_id],
    )
    .map_err(ApiError::internal)?;
    let cleaned = ids_raw.replace(['[', ']'], " ");
    for (ord, piece) in cleaned
        .split(|c: char| c.is_whitespace() || c == ',')
        .enumerate()
    {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        if let Ok(pid) = piece.parse::<i64>() {
            conn.execute(
                "INSERT OR IGNORE INTO pool_posts (pool_id, post_id, ord) VALUES (?1, ?2, ?3)",
                params![pool_id, pid, ord as i64],
            )
            .map_err(ApiError::internal)?;
        }
    }
    Ok(())
}

pub async fn update_pool(
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
    let pool = get_pool(&conn, id).ok_or_else(ApiError::not_found)?;
    if pool.creator_id != user.id && !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    if let Some(name) = form.get("name").or_else(|| form.get("pool[name]")) {
        conn.execute(
            "UPDATE pools SET name = ?1 WHERE id = ?2",
            params![name.trim().to_lowercase().replace(' ', "_"), id],
        )
        .map_err(ApiError::internal)?;
    }
    if let Some(desc) = form
        .get("description")
        .or_else(|| form.get("pool[description]"))
    {
        conn.execute(
            "UPDATE pools SET description = ?1 WHERE id = ?2",
            params![desc, id],
        )
        .map_err(ApiError::internal)?;
    }
    if let Some(ids) = form.get("post_ids").or_else(|| form.get("pool[post_ids]")) {
        set_pool_posts(&conn, id, ids)?;
    }
    if let Some(cat) = form.get("category").or_else(|| form.get("pool[category]")) {
        let cat = if cat == "collection" {
            "collection"
        } else {
            "series"
        };
        conn.execute(
            "UPDATE pools SET category = ?1 WHERE id = ?2",
            params![cat, id],
        )
        .map_err(ApiError::internal)?;
    }
    if let Some(active) = form.get("is_active") {
        conn.execute(
            "UPDATE pools SET is_active = ?1 WHERE id = ?2",
            params![matches!(active, "1" | "true") as i64, id],
        )
        .map_err(ApiError::internal)?;
    }
    conn.execute(
        "UPDATE pools SET updated_at = ?1 WHERE id = ?2",
        params![db::ts(Utc::now()), id],
    )
    .map_err(ApiError::internal)?;
    let pool = get_pool(&conn, id).ok_or_else(ApiError::not_found)?;
    Ok(Json(pool_json(&conn, &pool)))
}

pub async fn delete_pool(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let pool = get_pool(&conn, id).ok_or_else(ApiError::not_found)?;
    if pool.creator_id != user.id && !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    conn.execute("UPDATE pools SET is_deleted = 1 WHERE id = ?1", params![id])
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "success": true })))
}
