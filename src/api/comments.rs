//! /comments endpoints.

use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rusqlite::params;
use serde_json::{json, Value};

use crate::api::posts::urldec;
use crate::api::{strip_json_suffix, ApiError, ApiResult};
use crate::auth::ApiAuth;
use crate::db;
use crate::models::StoredComment;
use crate::AppState;

fn comment_json(conn: &rusqlite::Connection, c: &StoredComment, viewer: Option<i64>) -> Value {
    let my_vote: i64 = viewer
        .and_then(|uid| {
            conn.query_row(
                "SELECT score FROM comment_votes WHERE comment_id = ?1 AND user_id = ?2",
                params![c.id, uid],
                |r| r.get(0),
            )
            .ok()
        })
        .unwrap_or(0);
    json!({
        "id": c.id,
        "created_at": c.created_at,
        "post_id": c.post_id,
        "creator_id": c.creator_id,
        "body": c.body,
        "score": c.score,
        "updated_at": c.updated_at,
        "updater_id": c.creator_id,
        "do_not_bump_post": false,
        "is_hidden": c.is_hidden,
        "is_sticky": c.is_sticky,
        "warning_type": Value::Null,
        "warning_user_id": Value::Null,
        "creator_name": db::user_name(conn, c.creator_id),
        "updater_name": db::user_name(conn, c.creator_id),
        "vote": my_vote,
    })
}

fn row_to_comment(r: &rusqlite::Row) -> rusqlite::Result<StoredComment> {
    Ok(StoredComment {
        id: r.get("id")?,
        post_id: r.get("post_id")?,
        creator_id: r.get("creator_id")?,
        body: r.get("body")?,
        score: r.get("score")?,
        is_hidden: r.get::<_, i64>("is_hidden")? != 0,
        is_sticky: r.get::<_, i64>("is_sticky")? != 0,
        created_at: db::parse_ts(&r.get::<_, String>("created_at")?),
        updated_at: db::parse_ts(&r.get::<_, String>("updated_at")?),
    })
}

fn get_comment(conn: &rusqlite::Connection, id: i64) -> Option<StoredComment> {
    conn.query_row(
        "SELECT * FROM comments WHERE id = ?1",
        params![id],
        row_to_comment,
    )
    .ok()
}

pub async fn list_comments(
    State(state): State<AppState>,
    auth: ApiAuth,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    let mut conds = vec!["is_hidden = 0".to_string()];
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(pid) = params
        .get("search[post_id]")
        .or_else(|| params.get("post_id"))
        .and_then(|s| s.parse::<i64>().ok())
    {
        args.push(Box::new(pid));
        conds.push(format!("post_id = ?{}", args.len()));
    }
    if let Some(cid) = params
        .get("search[creator_id]")
        .and_then(|s| s.parse::<i64>().ok())
    {
        args.push(Box::new(cid));
        conds.push(format!("creator_id = ?{}", args.len()));
    }
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(75)
        .clamp(1, 320);
    let group_by_post = params.get("group_by").map(String::as_str) == Some("post");

    let sql = if group_by_post {
        format!(
            "SELECT * FROM comments c WHERE {} GROUP BY c.post_id
             ORDER BY MAX(c.created_at) DESC LIMIT {limit}",
            conds.join(" AND ").replace("post_id", "c.post_id").replace("creator_id", "c.creator_id")
        )
    } else {
        format!(
            "SELECT * FROM comments WHERE {} ORDER BY created_at DESC LIMIT {limit}",
            conds.join(" AND ")
        )
    };
    let mut stmt = conn.prepare(&sql).map_err(ApiError::internal)?;
    let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    let comments: Vec<StoredComment> = stmt
        .query_map(refs.as_slice(), row_to_comment)
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    let viewer = auth.id();
    let items: Vec<Value> = comments
        .iter()
        .map(|c| comment_json(&conn, c, viewer))
        .collect();
    Ok(Json(json!(items)))
}

fn comment_form(body: &Bytes) -> HashMap<String, String> {
    let raw = String::from_utf8_lossy(body);
    let mut out = HashMap::new();
    if raw.trim_start().starts_with('{') {
        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
            let comment = v.get("comment").cloned().unwrap_or(v);
            if let Some(map) = comment.as_object() {
                for (k, val) in map {
                    out.insert(k.clone(), val.as_str().unwrap_or(&val.to_string()).to_string());
                }
            }
        }
        return out;
    }
    for pair in raw.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        out.insert(urldec(k), urldec(v));
    }
    out
}

fn field<'a>(form: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    form.get(&format!("comment[{key}]"))
        .or_else(|| form.get(key))
        .map(String::as_str)
}

pub async fn create_comment(
    State(state): State<AppState>,
    auth: ApiAuth,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let form = comment_form(&body);
    let post_id: i64 = field(&form, "post_id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ApiError::bad_request("post_id required"))?;
    let text = field(&form, "body").unwrap_or_default().to_string();
    if text.is_empty() {
        return Err(ApiError::bad_request("body required"));
    }
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    if db::get_post(&conn, post_id).is_none() {
        return Err(ApiError::not_found());
    }
    let now = db::ts(Utc::now());
    conn.execute(
        "INSERT INTO comments (post_id, creator_id, body, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![post_id, user.id, text, now],
    )
    .map_err(ApiError::internal)?;
    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(ApiError::internal)?;
    let c = get_comment(&conn, id).ok_or_else(ApiError::not_found)?;
    Ok((
        StatusCode::CREATED,
        Json(comment_json(&conn, &c, Some(user.id))),
    ))
}

pub async fn show_comment(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id).parse().map_err(|_| ApiError::not_found())?;
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    let c = get_comment(&conn, id).ok_or_else(ApiError::not_found)?;
    Ok(Json(comment_json(&conn, &c, auth.id())))
}

pub async fn update_comment(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id).parse().map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let form = comment_form(&body);
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    let c = get_comment(&conn, id).ok_or_else(ApiError::not_found)?;
    if c.creator_id != user.id && !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    if let Some(text) = field(&form, "body") {
        conn.execute(
            "UPDATE comments SET body = ?1, updated_at = ?2 WHERE id = ?3",
            params![text, db::ts(Utc::now()), id],
        )
        .map_err(ApiError::internal)?;
    }
    if user.is_admin() {
        if let Some(v) = field(&form, "is_hidden") {
            conn.execute(
                "UPDATE comments SET is_hidden = ?1 WHERE id = ?2",
                params![matches!(v, "1" | "true") as i64, id],
            )
            .map_err(ApiError::internal)?;
        }
        if let Some(v) = field(&form, "is_sticky") {
            conn.execute(
                "UPDATE comments SET is_sticky = ?1 WHERE id = ?2",
                params![matches!(v, "1" | "true") as i64, id],
            )
            .map_err(ApiError::internal)?;
        }
    }
    let c = get_comment(&conn, id).ok_or_else(ApiError::not_found)?;
    Ok(Json(comment_json(&conn, &c, Some(user.id))))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id).parse().map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    let c = get_comment(&conn, id).ok_or_else(ApiError::not_found)?;
    if c.creator_id != user.id && !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    conn.execute("DELETE FROM comments WHERE id = ?1", params![id])
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "success": true })))
}

pub async fn vote_comment(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id).parse().map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let raw = String::from_utf8_lossy(&body);
    let score_str = raw
        .split('&')
        .find_map(|p| p.strip_prefix("score=").map(urldec))
        .unwrap_or_else(|| {
            serde_json::from_str::<Value>(&raw)
                .ok()
                .and_then(|v| v["score"].as_i64().map(|n| n.to_string()))
                .unwrap_or_default()
        });
    let score: i64 = match score_str.as_str() {
        "up" | "1" => 1,
        "down" | "-1" => -1,
        _ => return Err(ApiError::bad_request("score must be up or down")),
    };
    let conn = state.db.lock().map_err(|e| ApiError::internal(e))?;
    if get_comment(&conn, id).is_none() {
        return Err(ApiError::not_found());
    }
    let old: i64 = conn
        .query_row(
            "SELECT score FROM comment_votes WHERE comment_id = ?1 AND user_id = ?2",
            params![id, user.id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO comment_votes (comment_id, user_id, score, created_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(comment_id, user_id) DO UPDATE SET score = excluded.score",
        params![id, user.id, score, db::ts(Utc::now())],
    )
    .map_err(ApiError::internal)?;
    conn.execute(
        "UPDATE comments SET score = score + ?1 WHERE id = ?2",
        params![score - old, id],
    )
    .map_err(ApiError::internal)?;
    let total: i64 = conn
        .query_row("SELECT score FROM comments WHERE id = ?1", params![id], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    Ok(Json(json!({ "success": true, "score": total, "our_score": score })))
}
