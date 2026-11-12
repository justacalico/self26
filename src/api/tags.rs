//! /tags, /tag_aliases, /tag_implications endpoints.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::Json;
use rusqlite::params;
use serde_json::{json, Value};

use crate::api::posts::read_post_form;
use crate::api::{strip_json_suffix, ApiError, ApiResult};
use crate::auth::ApiAuth;
use crate::db;
use crate::models::StoredTag;
use crate::AppState;

fn tag_json(t: &StoredTag) -> Value {
    json!({
        "id": t.id,
        "name": t.name,
        "post_count": t.post_count,
        "related_tags": "",
        "related_tags_updated_at": Value::Null,
        "category": t.category,
        "is_locked": false,
        "created_at": Value::Null,
        "updated_at": Value::Null,
    })
}

fn row_to_tag(r: &rusqlite::Row) -> rusqlite::Result<StoredTag> {
    Ok(StoredTag {
        id: r.get("id")?,
        name: r.get("name")?,
        category: r.get("category")?,
        post_count: r.get("post_count")?,
    })
}

pub async fn list_tags(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let mut conds = Vec::new();
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(m) = params
        .get("search[name_matches]")
        .or_else(|| params.get("name_matches"))
    {
        args.push(Box::new(m.replace('*', "%")));
        conds.push(format!("name LIKE ?{} ESCAPE '\\'", args.len()));
    }
    if let Some(cat) = params
        .get("search[category]")
        .or_else(|| params.get("category"))
        .and_then(|s| s.parse::<i64>().ok())
    {
        args.push(Box::new(cat));
        conds.push(format!("category = ?{}", args.len()));
    }
    if params.get("search[hide_empty]").map(String::as_str) == Some("yes")
        || params.get("search[hide_empty]").map(String::as_str) == Some("true")
    {
        conds.push("post_count > 0".into());
    }
    let order = match params.get("search[order]").map(String::as_str) {
        Some("name") => "name ASC",
        Some("count") | Some("post_count") => "post_count DESC, name ASC",
        _ => "post_count DESC, name ASC",
    };
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(75)
        .clamp(1, 1000);
    let page = params
        .get("page")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);

    let mut sql = "SELECT id, name, category, post_count FROM tags".to_string();
    if !conds.is_empty() {
        sql.push_str(&format!(" WHERE {}", conds.join(" AND ")));
    }
    sql.push_str(&format!(
        " ORDER BY {order} LIMIT {limit} OFFSET {}",
        (page - 1) * limit
    ));

    let mut stmt = conn.prepare(&sql).map_err(ApiError::internal)?;
    let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|a| a.as_ref()).collect();
    let tags: Vec<StoredTag> = stmt
        .query_map(refs.as_slice(), row_to_tag)
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    Ok(Json(json!(tags.iter().map(tag_json).collect::<Vec<_>>())))
}

pub async fn autocomplete(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let query = params
        .get("search[query]")
        .or_else(|| params.get("query"))
        .cloned()
        .unwrap_or_default();
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, category, post_count FROM tags
             WHERE name LIKE ?1 ESCAPE '\\'
             ORDER BY post_count DESC LIMIT 20",
        )
        .map_err(ApiError::internal)?;
    let pattern = format!("{}%", query.replace('*', "%"));
    let tags: Vec<StoredTag> = stmt
        .query_map(params![pattern], row_to_tag)
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    let items: Vec<Value> = tags
        .iter()
        .map(|t| {
            json!({
                "type": "tag",
                "label": t.name.replace('_', " "),
                "value": t.name,
                "category": t.category,
                "post_count": t.post_count,
                "name": t.name,
            })
        })
        .collect();
    Ok(Json(json!(items)))
}

pub async fn popular_tags(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, category, post_count FROM tags
             WHERE post_count > 0 ORDER BY post_count DESC LIMIT 25",
        )
        .map_err(ApiError::internal)?;
    let tags: Vec<StoredTag> = stmt
        .query_map([], row_to_tag)
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    Ok(Json(json!({
        "tags": tags.iter().map(|t| json!({ "name": t.name, "post_count": t.post_count, "category": t.category })).collect::<Vec<_>>()
    })))
}

pub async fn show_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let raw = strip_json_suffix(&id);
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let tag = if let Ok(id) = raw.parse::<i64>() {
        conn.query_row(
            "SELECT id, name, category, post_count FROM tags WHERE id = ?1",
            params![id],
            row_to_tag,
        )
        .ok()
    } else {
        conn.query_row(
            "SELECT id, name, category, post_count FROM tags WHERE name = ?1",
            params![raw],
            row_to_tag,
        )
        .ok()
    };
    let tag = tag.ok_or_else(ApiError::not_found)?;
    Ok(Json(tag_json(&tag)))
}

pub async fn update_tag(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
    request: axum::extract::Request,
) -> ApiResult<Json<Value>> {
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    if !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    let raw = strip_json_suffix(&id).to_string();
    let form = read_post_form(request).await?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let tag_id: i64 = raw.parse().map_err(|_| ApiError::not_found())?;
    if let Some(cat) = form.get("category").and_then(|s| s.parse::<i64>().ok()) {
        conn.execute(
            "UPDATE tags SET category = ?1 WHERE id = ?2",
            params![cat, tag_id],
        )
        .map_err(ApiError::internal)?;
    }
    let tag = conn
        .query_row(
            "SELECT id, name, category, post_count FROM tags WHERE id = ?1",
            params![tag_id],
            row_to_tag,
        )
        .map_err(|_| ApiError::not_found())?;
    Ok(Json(tag_json(&tag)))
}

fn alias_rows(conn: &rusqlite::Connection, table: &str) -> Vec<Value> {
    let sql = format!(
        "SELECT id, antecedent, consequent, status, created_at FROM {table} ORDER BY id DESC LIMIT 1000"
    );
    conn.prepare(&sql)
        .and_then(|mut s| {
            let rows = s.query_map([], |r| {
                Ok(json!({
                    "id": r.get::<_, i64>(0)?,
                    "antecedent_name": r.get::<_, String>(1)?,
                    "consequent_name": r.get::<_, String>(2)?,
                    "status": r.get::<_, String>(3)?,
                    "created_at": db::parse_ts(&r.get::<_, String>(4)?),
                    "reason": "",
                    "post_count": 0,
                    "approver_id": Value::Null,
                    "forum_post_id": Value::Null,
                    "forum_topic_id": Value::Null,
                }))
            })?;
            Ok(rows.flatten().collect())
        })
        .unwrap_or_default()
}

pub async fn list_aliases(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let conn = state.db.lock().map_err(ApiError::internal)?;
    Ok(Json(json!(alias_rows(&conn, "tag_aliases"))))
}

pub async fn list_implications(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let conn = state.db.lock().map_err(ApiError::internal)?;
    Ok(Json(json!(alias_rows(&conn, "tag_implications"))))
}

/// Admin helper used by the web UI to create an alias or implication.
pub fn create_relation(
    conn: &rusqlite::Connection,
    table: &str,
    antecedent: &str,
    consequent: &str,
) -> rusqlite::Result<i64> {
    let sql = format!(
        "INSERT INTO {table} (antecedent, consequent, status, created_at)
         VALUES (?1, ?2, 'active', ?3)"
    );
    conn.execute(
        &sql,
        params![
            antecedent.to_lowercase().replace(' ', "_"),
            consequent.to_lowercase().replace(' ', "_"),
            db::ts(chrono::Utc::now())
        ],
    )?;
    conn.query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
}
