//! /post_flags, /tickets and the small admin API used by the web UI.

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
use crate::models::{StoredFlag, StoredTicket};
use crate::AppState;

fn flag_json(conn: &rusqlite::Connection, f: &StoredFlag) -> Value {
    json!({
        "id": f.id,
        "created_at": f.created_at,
        "post_id": f.post_id,
        "reason": f.reason,
        "creator_id": f.creator_id,
        "creator_name": f.creator_id.map(|id| db::user_name(conn, id)),
        "is_resolved": f.is_resolved,
        "updated_at": f.updated_at,
        "is_deletion": f.is_deletion,
        "type": if f.is_deletion { "deletion" } else { "flag" },
        "note": Value::Null,
    })
}

fn row_to_flag(r: &rusqlite::Row) -> rusqlite::Result<StoredFlag> {
    Ok(StoredFlag {
        id: r.get("id")?,
        post_id: r.get("post_id")?,
        creator_id: r.get("creator_id")?,
        reason: r.get("reason")?,
        is_resolved: r.get::<_, i64>("is_resolved")? != 0,
        is_deletion: r.get::<_, i64>("is_deletion")? != 0,
        created_at: db::parse_ts(&r.get::<_, String>("created_at")?),
        updated_at: db::parse_ts(&r.get::<_, String>("updated_at")?),
    })
}

pub async fn list_flags(
    State(state): State<AppState>,
    auth: ApiAuth,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    auth.user().ok_or_else(ApiError::unauthorized)?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let mut sql = "SELECT * FROM post_flags WHERE 1=1".to_string();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(pid) = params
        .get("search[post_id]")
        .and_then(|s| s.parse::<i64>().ok())
    {
        args.push(Box::new(pid));
        sql.push_str(&format!(" AND post_id = ?{}", args.len()));
    }
    if let Some(resolved) = params.get("search[is_resolved]") {
        args.push(Box::new(matches!(resolved.as_str(), "true" | "1") as i64));
        sql.push_str(&format!(" AND is_resolved = ?{}", args.len()));
    }
    sql.push_str(" ORDER BY id DESC LIMIT 320");
    let mut stmt = conn.prepare(&sql).map_err(ApiError::internal)?;
    let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    let flags: Vec<StoredFlag> = stmt
        .query_map(refs.as_slice(), row_to_flag)
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    let items: Vec<Value> = flags.iter().map(|f| flag_json(&conn, f)).collect();
    Ok(Json(json!(items)))
}

pub async fn show_flag(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    auth.user().ok_or_else(ApiError::unauthorized)?;
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let f = conn
        .query_row(
            "SELECT * FROM post_flags WHERE id = ?1",
            params![id],
            row_to_flag,
        )
        .map_err(|_| ApiError::not_found())?;
    Ok(Json(flag_json(&conn, &f)))
}

pub async fn create_flag(
    State(state): State<AppState>,
    auth: ApiAuth,
    request: axum::extract::Request,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let form = read_post_form(request).await?;
    let post_id: i64 = form
        .get("post_id")
        .or_else(|| form.get("post_flag[post_id]"))
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ApiError::bad_request("post_id required"))?;
    let reason = form
        .get("reason")
        .or_else(|| form.get("post_flag[reason]"))
        .unwrap_or_default()
        .to_string();
    let is_deletion = matches!(form.get("is_deletion"), Some("true") | Some("1"));
    let conn = state.db.lock().map_err(ApiError::internal)?;
    if db::get_post(&conn, post_id).is_none() {
        return Err(ApiError::not_found());
    }
    let now = db::ts(Utc::now());
    conn.execute(
        "INSERT INTO post_flags (post_id, creator_id, reason, is_deletion,
            created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![post_id, user.id, reason, is_deletion as i64, now],
    )
    .map_err(ApiError::internal)?;
    conn.execute(
        "UPDATE posts SET is_flagged = 1 WHERE id = ?1",
        params![post_id],
    )
    .map_err(ApiError::internal)?;
    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(ApiError::internal)?;
    let f = conn
        .query_row(
            "SELECT * FROM post_flags WHERE id = ?1",
            params![id],
            row_to_flag,
        )
        .map_err(|_| ApiError::not_found())?;
    Ok((StatusCode::CREATED, Json(flag_json(&conn, &f))))
}

pub async fn delete_flag(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    if !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    let conn = state.db.lock().map_err(ApiError::internal)?;
    conn.execute("DELETE FROM post_flags WHERE id = ?1", params![id])
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "success": true })))
}

fn ticket_json(conn: &rusqlite::Connection, t: &StoredTicket) -> Value {
    json!({
        "id": t.id,
        "creator_id": t.creator_id,
        "creator_name": t.creator_id.map(|id| db::user_name(conn, id)),
        "reason": t.reason,
        "disp_id": t.disp_id,
        "qtype": t.qtype,
        "status": t.status,
        "created_at": t.created_at,
        "updated_at": t.updated_at,
        "response": "",
        "handler_id": Value::Null,
        "report_reason": Value::Null,
        "accused_id": Value::Null,
        "claimant_id": Value::Null,
    })
}

fn row_to_ticket(r: &rusqlite::Row) -> rusqlite::Result<StoredTicket> {
    Ok(StoredTicket {
        id: r.get("id")?,
        creator_id: r.get("creator_id")?,
        qtype: r.get("qtype")?,
        disp_id: r.get("disp_id")?,
        reason: r.get("reason")?,
        status: r.get("status")?,
        created_at: db::parse_ts(&r.get::<_, String>("created_at")?),
        updated_at: db::parse_ts(&r.get::<_, String>("updated_at")?),
    })
}

pub async fn list_tickets(
    State(state): State<AppState>,
    auth: ApiAuth,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    if !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let mut sql = "SELECT * FROM tickets WHERE 1=1".to_string();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(qtype) = params.get("search[qtype]") {
        args.push(Box::new(qtype.clone()));
        sql.push_str(&format!(" AND qtype = ?{}", args.len()));
    }
    sql.push_str(" ORDER BY id DESC LIMIT 320");
    let mut stmt = conn.prepare(&sql).map_err(ApiError::internal)?;
    let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    let tickets: Vec<StoredTicket> = stmt
        .query_map(refs.as_slice(), row_to_ticket)
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    let items: Vec<Value> = tickets.iter().map(|t| ticket_json(&conn, t)).collect();
    Ok(Json(json!(items)))
}

pub async fn create_ticket(
    State(state): State<AppState>,
    auth: ApiAuth,
    request: axum::extract::Request,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let form = read_post_form(request).await?;
    let qtype = form
        .get("qtype")
        .or_else(|| form.get("ticket[qtype]"))
        .unwrap_or("post")
        .to_string();
    let disp_id: i64 = form
        .get("disp_id")
        .or_else(|| form.get("ticket[disp_id]"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let reason = form
        .get("reason")
        .or_else(|| form.get("ticket[reason]"))
        .unwrap_or_default()
        .to_string();
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let now = db::ts(Utc::now());
    conn.execute(
        "INSERT INTO tickets (creator_id, qtype, disp_id, reason, status,
            created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?5)",
        params![auth.id(), qtype, disp_id, reason, now],
    )
    .map_err(ApiError::internal)?;
    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(ApiError::internal)?;
    let t = conn
        .query_row(
            "SELECT * FROM tickets WHERE id = ?1",
            params![id],
            row_to_ticket,
        )
        .map_err(|_| ApiError::not_found())?;
    Ok((StatusCode::CREATED, Json(ticket_json(&conn, &t))))
}

// ----- admin helpers used by the web UI -----

fn require_admin(auth: &ApiAuth) -> ApiResult<i64> {
    let user = auth.user().ok_or_else(ApiError::unauthorized)?;
    if !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    Ok(user.id)
}

pub async fn admin_stats(State(state): State<AppState>, auth: ApiAuth) -> ApiResult<Json<Value>> {
    require_admin(&auth)?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap_or(0) };
    Ok(Json(json!({
        "posts": count("SELECT COUNT(*) FROM posts WHERE is_deleted = 0"),
        "deleted_posts": count("SELECT COUNT(*) FROM posts WHERE is_deleted = 1"),
        "users": count("SELECT COUNT(*) FROM users"),
        "tags": count("SELECT COUNT(*) FROM tags"),
        "comments": count("SELECT COUNT(*) FROM comments"),
        "pools": count("SELECT COUNT(*) FROM pools WHERE is_deleted = 0"),
        "pending_flags": count("SELECT COUNT(*) FROM post_flags WHERE is_resolved = 0"),
        "pending_tickets": count("SELECT COUNT(*) FROM tickets WHERE status = 'pending'"),
        "disk_bytes": count("SELECT COALESCE(SUM(size),0) FROM posts WHERE is_deleted = 0"),
    })))
}

pub async fn admin_create_relation(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(kind): Path<String>,
    request: axum::extract::Request,
) -> ApiResult<Json<Value>> {
    require_admin(&auth)?;
    let table = match strip_json_suffix(&kind) {
        "aliases" | "tag_aliases" => "tag_aliases",
        "implications" | "tag_implications" => "tag_implications",
        _ => return Err(ApiError::not_found()),
    };
    let form = read_post_form(request).await?;
    let antecedent = form.get("antecedent").unwrap_or_default().to_string();
    let consequent = form.get("consequent").unwrap_or_default().to_string();
    if antecedent.is_empty() || consequent.is_empty() {
        return Err(ApiError::bad_request("antecedent and consequent required"));
    }
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let id = crate::api::tags::create_relation(&conn, table, &antecedent, &consequent)
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "success": true, "id": id })))
}

pub async fn admin_resolve_flag(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_admin(&auth)?;
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    conn.execute(
        "UPDATE post_flags SET is_resolved = 1, updated_at = ?1 WHERE id = ?2",
        params![db::ts(Utc::now()), id],
    )
    .map_err(ApiError::internal)?;
    let post_id: Option<i64> = conn
        .query_row(
            "SELECT post_id FROM post_flags WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .ok();
    if let Some(pid) = post_id {
        let remaining: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM post_flags WHERE post_id = ?1 AND is_resolved = 0",
                params![pid],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if remaining == 0 {
            conn.execute(
                "UPDATE posts SET is_flagged = 0 WHERE id = ?1",
                params![pid],
            )
            .map_err(ApiError::internal)?;
        }
    }
    Ok(Json(json!({ "success": true })))
}

pub async fn admin_resolve_ticket(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    require_admin(&auth)?;
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    conn.execute(
        "UPDATE tickets SET status = 'approved', updated_at = ?1 WHERE id = ?2",
        params![db::ts(Utc::now()), id],
    )
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "success": true })))
}
