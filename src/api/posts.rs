//! /posts endpoints: search, show, upload, update, delete, votes, favorites.

use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::{FromRequest, Multipart, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use rusqlite::params;
use serde_json::{json, Value};

use crate::api::post_json::{post_json, PostFormat};
use crate::api::{strip_json_suffix, ApiError, ApiResult};
use crate::auth::ApiAuth;
use crate::db;
use crate::media;
use crate::models::{Rating, StoredPost};
use crate::search;
use crate::AppState;

/// Form fields pulled from multipart, urlencoded or JSON bodies. e621
/// clients send nested keys like `post[tag_string]`; we accept the flat
/// names too.
#[derive(Default)]
pub struct PostForm {
    pub fields: HashMap<String, String>,
    pub file: Option<(String, Bytes)>,
}

impl PostForm {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .get(&format!("post[{key}]"))
            .or_else(|| self.fields.get(key))
            .or_else(|| self.fields.get(&format!("upload[{key}]")))
            .map(String::as_str)
    }
}

pub async fn read_post_form(req: axum::extract::Request) -> ApiResult<PostForm> {
    let ctype = req
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let mut form = PostForm::default();

    if ctype.starts_with("multipart/form-data") {
        let mut mp = <Multipart as FromRequest<()>>::from_request(req, &())
            .await
            .map_err(|e| ApiError::bad_request(e.body_text()))?;
        while let Some(mut field) = mp
            .next_field()
            .await
            .map_err(|e| ApiError::bad_request(e.body_text()))?
        {
            let name = field.name().unwrap_or_default().to_string();
            let mut data = Vec::new();
            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|e| ApiError::bad_request(e.body_text()))?
            {
                data.extend_from_slice(&chunk);
                if data.len() > 128 * 1024 * 1024 {
                    return Err(ApiError::bad_request("body too large"));
                }
            }
            if field.file_name().is_some() {
                let fname = field.file_name().unwrap_or("upload.bin").to_string();
                if !data.is_empty() {
                    form.file = Some((fname, data.into()));
                }
            } else if let Ok(text) = String::from_utf8(data) {
                if text.len() <= 1024 * 1024 {
                    push_field(&mut form.fields, name, text);
                }
            }
        }
        return Ok(form);
    }

    let body = axum::body::to_bytes(req.into_body(), 32 * 1024 * 1024)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    if ctype.contains("application/json") {
        if let Ok(v) = serde_json::from_slice::<Value>(&body) {
            flatten_json(&v, &mut form.fields, "");
        }
    } else {
        // application/x-www-form-urlencoded
        for pair in String::from_utf8_lossy(&body).split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            push_field(&mut form.fields, urldec(k), urldec(v));
        }
    }
    Ok(form)
}

/// e621 clients submit arrays as `key[]=1&key[]=2`; collapse those into a
/// single space-separated value under `key` and merge repeated keys the
/// same way.
fn push_field(fields: &mut HashMap<String, String>, name: String, value: String) {
    let key = name.strip_suffix("[]").map(str::to_string).unwrap_or(name);
    match fields.get_mut(&key) {
        Some(existing) => {
            existing.push(' ');
            existing.push_str(&value);
        }
        None => {
            fields.insert(key, value);
        }
    }
}

fn flatten_json(v: &Value, out: &mut HashMap<String, String>, prefix: &str) {
    match v {
        Value::Object(map) => {
            for (k, val) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}[{k}]")
                };
                flatten_json(val, out, &key);
            }
        }
        Value::Array(items) => {
            out.insert(
                prefix.to_string(),
                items
                    .iter()
                    .filter_map(|i| i.as_str().map(String::from))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

pub(crate) fn urldec(s: &str) -> String {
    percent_encoding::percent_decode_str(&s.replace('+', " "))
        .decode_utf8()
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| s.to_string())
}

pub(crate) fn base_url_pub(headers: &HeaderMap, configured: &str) -> String {
    if !configured.is_empty() {
        return configured.trim_end_matches('/').to_string();
    }
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    if host.is_empty()
        || host.len() > 253
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':' | b'[' | b']'))
    {
        return String::new();
    }
    let scheme = match headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
    {
        Some("https") => "https",
        _ => "http",
    };
    format!("{scheme}://{host}")
}

fn viewer_id(auth: &ApiAuth) -> Option<i64> {
    auth.user().map(|u| u.id)
}

#[derive(Debug, serde::Deserialize)]
pub struct ListParams {
    pub tags: Option<String>,
    pub limit: Option<i64>,
    pub page: Option<String>,
    pub v2: Option<String>,
    pub mode: Option<String>,
    pub random: Option<String>,
}

pub async fn list_posts(
    State(state): State<AppState>,
    auth: ApiAuth,
    headers: HeaderMap,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<Value>> {
    let (page, boundary) = search::page_params(params.page.as_deref(), params.limit);
    let mut q = search::parse(params.tags.as_deref().unwrap_or_default(), page);

    if params.random.as_deref() == Some("true") {
        q.order_sql = "RANDOM()".into();
    }
    if let Some((dir, id)) = boundary {
        let p = format!("?{}", q.params.len() + 1);
        q.params.push(Box::new(id));
        let cond = if dir == 'b' {
            format!("posts.id < {p}")
        } else {
            format!("posts.id > {p}")
        };
        q.where_sql = if q.where_sql.is_empty() {
            cond
        } else {
            format!("{} AND {cond}", q.where_sql)
        };
    }

    let format = PostFormat::from_params(params.v2.as_deref(), params.mode.as_deref());
    let base = base_url_pub(&headers, &state.base_url);
    let posts = {
        let conn = state.db.lock().map_err(ApiError::internal)?;
        let mut sql = format!("SELECT {} FROM posts", db::POST_COLS);
        let wants_status = params
            .tags
            .as_deref()
            .map(|t| {
                t.split_whitespace()
                    .any(|tok| tok.trim_start_matches(['-', '~']).starts_with("status:"))
            })
            .unwrap_or(false);
        let admin = auth.user().map(|u| u.is_admin()).unwrap_or(false);
        let mut conds = Vec::new();
        if !(admin && wants_status) {
            conds.push("posts.is_deleted = 0".to_string());
        }
        if !q.where_sql.is_empty() {
            conds.push(q.where_sql.clone());
        }
        if !conds.is_empty() {
            sql.push_str(&format!(" WHERE {}", conds.join(" AND ")));
        }
        sql.push_str(&format!(
            " ORDER BY {} LIMIT {} OFFSET {}",
            q.order_sql, q.limit, page.offset
        ));

        let mut stmt = conn.prepare(&sql).map_err(ApiError::internal)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = q.params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(params_refs.as_slice(), db::row_to_post)
            .map_err(ApiError::internal)?;
        rows.flatten().collect::<Vec<StoredPost>>()
    };

    let conn = state.db.lock().map_err(ApiError::internal)?;
    let items: Vec<Value> = posts
        .iter()
        .map(|p| post_json(&conn, p, &base, viewer_id(&auth), format))
        .collect();
    if format.is_v2() {
        Ok(Json(Value::Array(items)))
    } else {
        Ok(Json(json!({ "posts": items })))
    }
}

pub async fn show_post(
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
    let base = base_url_pub(&headers, &state.base_url);
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let post = db::get_post(&conn, id).ok_or_else(ApiError::not_found)?;
    let body = post_json(&conn, &post, &base, viewer_id(&auth), format);
    if format.is_v2() {
        Ok(Json(body))
    } else {
        Ok(Json(json!({ "post": body })))
    }
}

fn apply_tag_edits(conn: &rusqlite::Connection, post_id: i64, tag_string: &str) -> ApiResult<()> {
    let names: Vec<String> = tag_string
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let resolved = resolve_aliases(conn, &names);
    let all = apply_implications(conn, resolved);
    db::set_post_tags(conn, post_id, &all).map_err(ApiError::internal)
}

fn resolve_aliases(conn: &rusqlite::Connection, names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for n in names {
        let target: Option<String> = conn
            .query_row(
                "SELECT consequent FROM tag_aliases
                 WHERE antecedent = ?1 AND status = 'active'",
                params![n.to_lowercase().replace(' ', "_")],
                |r| r.get(0),
            )
            .ok();
        out.push(target.unwrap_or_else(|| n.clone()));
    }
    out
}

fn apply_implications(conn: &rusqlite::Connection, names: Vec<String>) -> Vec<String> {
    let mut all: Vec<String> = names.clone();
    let mut i = 0;
    while i < all.len() {
        let name = all[i].to_lowercase().replace(' ', "_");
        let mut stmt = conn
            .prepare(
                "SELECT consequent FROM tag_implications
                 WHERE antecedent = ?1 AND status = 'active'",
            )
            .expect("implications");
        let implied: Vec<String> = stmt
            .query_map(params![name], |r| r.get(0))
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default();
        for t in implied {
            if !all.iter().any(|x| x.eq_ignore_ascii_case(&t)) {
                all.push(t);
            }
        }
        i += 1;
    }
    all
}

pub async fn create_post(
    State(state): State<AppState>,
    auth: ApiAuth,
    headers: HeaderMap,
    request: axum::extract::Request,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let form = read_post_form(request).await?;
    let (filename, bytes) = match &form.file {
        Some(f) => f.clone(),
        None => return Err(ApiError::bad_request("no file uploaded")),
    };
    let md5 = media::file_md5(&bytes);
    let ext = media::file_ext(&bytes, &filename);
    if ext == "bin" {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "file type not supported",
        ));
    }

    {
        let conn = state.db.lock().map_err(ApiError::internal)?;
        if let Ok(existing) =
            conn.query_row("SELECT id FROM posts WHERE md5 = ?1", params![md5], |r| {
                r.get::<_, i64>(0)
            })
        {
            let mut e = ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("duplicate: post #{existing} already has md5:{md5}"),
            );
            e.reason = Some("duplicate".into());
            return Err(e);
        }
    }

    let media_proc = state.media.clone();
    let store_bytes = bytes.clone();
    let store_md5 = md5.clone();
    let store_ext = ext.clone();
    let info =
        tokio::task::spawn_blocking(move || media_proc.store(&store_bytes, &store_md5, &store_ext))
            .await
            .map_err(ApiError::internal)?
            .map_err(ApiError::internal)?;

    let rating = Rating::parse(form.get("rating").unwrap_or("s")).unwrap_or(Rating::S);
    let sources: Vec<String> = form
        .get("source")
        .or_else(|| form.get("sources"))
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default();
    let description = form.get("description").unwrap_or_default().to_string();
    let tag_string = form
        .get("tag_string")
        .or_else(|| form.get("tags"))
        .unwrap_or_default()
        .to_string();
    let now = db::ts(Utc::now());

    let post_id = {
        let conn = state.db.lock().map_err(ApiError::internal)?;
        if let Err(e) = conn.execute(
            "INSERT INTO posts (created_at, updated_at, change_seq, uploader_id,
                approver_id, md5, ext, size, width, height, has_sample,
                preview_width, preview_height, sample_width, sample_height,
                rating, sources, description, parent_id, is_pending)
             VALUES (?1, ?1, 0, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                     ?14, ?15, ?16, ?17, 0)",
            params![
                now,
                user.id,
                user.id, // self-hosted: uploader auto-approves
                md5,
                ext,
                bytes.len() as i64,
                info.width,
                info.height,
                info.has_sample as i64,
                info.preview_width,
                info.preview_height,
                info.sample_width,
                info.sample_height,
                rating.as_str(),
                serde_json::to_string(&sources).unwrap_or_else(|_| "[]".into()),
                description,
                form.get("parent_id").and_then(|s| s.parse::<i64>().ok()),
            ],
        ) {
            state.media.remove(&md5);
            return Err(ApiError::internal(e));
        }
        let id: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .map_err(ApiError::internal)?;
        apply_tag_edits(&conn, id, &tag_string)?;
        id
    };

    let conn = state.db.lock().map_err(ApiError::internal)?;
    let post = db::get_post(&conn, post_id).ok_or_else(ApiError::not_found)?;
    let base = base_url_pub(&headers, &state.base_url);
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "post_id": post_id,
            "location": format!("{}/posts/{}", base, post_id),
            "post": post_json(&conn, &post, &base, Some(user.id), PostFormat::Legacy),
        })),
    ))
}

pub async fn update_post(
    State(state): State<AppState>,
    auth: ApiAuth,
    headers: HeaderMap,
    Path(id): Path<String>,
    request: axum::extract::Request,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let form = read_post_form(request).await?;

    let conn = state.db.lock().map_err(ApiError::internal)?;
    let post = db::get_post(&conn, id).ok_or_else(ApiError::not_found)?;
    if post.is_deleted && !user.is_admin() {
        return Err(ApiError::not_found());
    }
    let can_manage = post.uploader_id == user.id || user.is_admin();
    let sensitive = [
        "rating",
        "description",
        "source",
        "sources",
        "parent_id",
        "is_rating_locked",
        "is_note_locked",
        "is_status_locked",
        "locked_tags",
    ]
    .iter()
    .any(|k| form.get(k).is_some());
    if sensitive && !can_manage {
        return Err(ApiError::forbidden());
    }

    if let Some(tags) = form.get("tag_string").or_else(|| form.get("tags")) {
        if user.is_admin() || post.locked_tags.is_empty() {
            apply_tag_edits(&conn, id, tags)?;
        } else {
            let before: std::collections::HashSet<String> =
                db::post_tags(&conn, id).into_iter().map(|t| t.0).collect();
            apply_tag_edits(&conn, id, tags)?;
            let mut now: std::collections::HashSet<String> =
                db::post_tags(&conn, id).into_iter().map(|t| t.0).collect();
            let mut changed = false;
            for lt in &post.locked_tags {
                match (before.contains(lt), now.contains(lt)) {
                    (true, false) => {
                        now.insert(lt.clone());
                        changed = true;
                    }
                    (false, true) => {
                        now.remove(lt);
                        changed = true;
                    }
                    _ => {}
                }
            }
            if changed {
                db::set_post_tags(&conn, id, &now.into_iter().collect::<Vec<_>>())
                    .map_err(ApiError::internal)?;
            }
        }
    }
    if let Some(r) = form.get("rating") {
        if !post.rating_locked || user.is_admin() {
            if let Some(rating) = Rating::parse(r) {
                conn.execute(
                    "UPDATE posts SET rating = ?1 WHERE id = ?2",
                    params![rating.as_str(), id],
                )
                .map_err(ApiError::internal)?;
            }
        }
    }
    if let Some(desc) = form.get("description") {
        conn.execute(
            "UPDATE posts SET description = ?1 WHERE id = ?2",
            params![desc, id],
        )
        .map_err(ApiError::internal)?;
    }
    if let Some(src) = form.get("source").or_else(|| form.get("sources")) {
        let sources: Vec<String> = src.split_whitespace().map(String::from).collect();
        conn.execute(
            "UPDATE posts SET sources = ?1 WHERE id = ?2",
            params![
                serde_json::to_string(&sources).unwrap_or_else(|_| "[]".into()),
                id
            ],
        )
        .map_err(ApiError::internal)?;
    }
    if let Some(pid) = form.get("parent_id") {
        let parent: Option<i64> = pid.parse().ok();
        conn.execute(
            "UPDATE posts SET parent_id = ?1 WHERE id = ?2",
            params![parent, id],
        )
        .map_err(ApiError::internal)?;
    }
    if user.is_admin() {
        if let Some(v) = form.get("is_rating_locked") {
            conn.execute(
                "UPDATE posts SET rating_locked = ?1 WHERE id = ?2",
                params![truthy(v) as i64, id],
            )
            .map_err(ApiError::internal)?;
        }
        if let Some(v) = form.get("is_note_locked") {
            conn.execute(
                "UPDATE posts SET note_locked = ?1 WHERE id = ?2",
                params![truthy(v) as i64, id],
            )
            .map_err(ApiError::internal)?;
        }
        if let Some(v) = form.get("is_status_locked") {
            conn.execute(
                "UPDATE posts SET status_locked = ?1 WHERE id = ?2",
                params![truthy(v) as i64, id],
            )
            .map_err(ApiError::internal)?;
        }
        if let Some(v) = form.get("locked_tags") {
            let tags: Vec<String> = v
                .split_whitespace()
                .map(|t| t.to_lowercase().replace(' ', "_"))
                .collect();
            conn.execute(
                "UPDATE posts SET locked_tags = ?1 WHERE id = ?2",
                params![
                    serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into()),
                    id
                ],
            )
            .map_err(ApiError::internal)?;
        }
    }
    conn.execute(
        "UPDATE posts SET updated_at = ?1, change_seq = change_seq + 1 WHERE id = ?2",
        params![db::ts(Utc::now()), id],
    )
    .map_err(ApiError::internal)?;

    let post = db::get_post(&conn, id).ok_or_else(ApiError::not_found)?;
    let base = base_url_pub(&headers, &state.base_url);
    Ok(Json(
        json!({ "post": post_json(&conn, &post, &base, Some(user.id), PostFormat::Legacy) }),
    ))
}

fn truthy(v: &str) -> bool {
    matches!(v, "1" | "true" | "yes")
}

pub async fn delete_post(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let post = db::get_post(&conn, id).ok_or_else(ApiError::not_found)?;
    if post.uploader_id != user.id && !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    if post.status_locked && !user.is_admin() {
        return Err(ApiError::forbidden());
    }
    conn.execute(
        "UPDATE posts SET is_deleted = 1, updated_at = ?1 WHERE id = ?2",
        params![db::ts(Utc::now()), id],
    )
    .map_err(ApiError::internal)?;
    db::refresh_tag_counts(&conn).map_err(ApiError::internal)?;
    Ok(Json(json!({ "success": true })))
}

pub async fn random_post(
    State(state): State<AppState>,
    auth: ApiAuth,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let format = PostFormat::from_params(
        params.get("v2").map(String::as_str),
        params.get("mode").map(String::as_str),
    );
    let base = base_url_pub(&headers, &state.base_url);
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let (page, _) = search::page_params(Some("1"), Some(1));
    let q = search::parse(
        params.get("tags").map(String::as_str).unwrap_or_default(),
        page,
    );
    let admin = auth.user().map(|u| u.is_admin()).unwrap_or(false);
    let wants_status = params
        .get("tags")
        .map(|t| {
            t.split_whitespace()
                .any(|tok| tok.trim_start_matches(['-', '~']).starts_with("status:"))
        })
        .unwrap_or(false);
    let mut sql = format!("SELECT {} FROM posts", db::POST_COLS);
    let mut conds = Vec::new();
    if !(admin && wants_status) {
        conds.push("posts.is_deleted = 0".to_string());
    }
    if !q.where_sql.is_empty() {
        conds.push(q.where_sql.clone());
    }
    if !conds.is_empty() {
        sql.push_str(&format!(" WHERE {}", conds.join(" AND ")));
    }
    sql.push_str(" ORDER BY RANDOM() LIMIT 1");
    let params_refs: Vec<&dyn rusqlite::ToSql> = q.params.iter().map(|p| p.as_ref()).collect();
    let post = conn
        .query_row(&sql, params_refs.as_slice(), db::row_to_post)
        .map_err(|_| ApiError::not_found())?;
    let body = post_json(&conn, &post, &base, viewer_id(&auth), format);
    if format.is_v2() {
        Ok(Json(body))
    } else {
        Ok(Json(json!({ "post": body })))
    }
}

fn ranked_posts(
    state: &AppState,
    auth: &ApiAuth,
    headers: &HeaderMap,
    params: &HashMap<String, String>,
    score_sql: &str,
) -> ApiResult<Json<Value>> {
    let format = PostFormat::from_params(
        params.get("v2").map(String::as_str),
        params.get("mode").map(String::as_str),
    );
    let scale = params.get("scale").cloned().unwrap_or_default();
    let base = base_url_pub(headers, &state.base_url);
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let mut where_extra = String::new();
    if scale.starts_with("week") {
        where_extra = "AND posts.created_at >= datetime('now', '-7 days')".into();
    } else if scale.starts_with("month") {
        where_extra = "AND posts.created_at >= datetime('now', '-1 month')".into();
    } else if scale.starts_with("day") {
        where_extra = "AND posts.created_at >= datetime('now', '-1 day')".into();
    }
    let sql = format!(
        "SELECT {} FROM posts WHERE posts.is_deleted = 0 {where_extra}
         ORDER BY {score_sql} LIMIT 100",
        db::POST_COLS
    );
    let mut stmt = conn.prepare(&sql).map_err(ApiError::internal)?;
    let posts: Vec<StoredPost> = stmt
        .query_map([], db::row_to_post)
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    let items: Vec<Value> = posts
        .iter()
        .map(|p| post_json(&conn, p, &base, viewer_id(auth), format))
        .collect();
    if format.is_v2() {
        Ok(Json(Value::Array(items)))
    } else {
        Ok(Json(json!({ "posts": items })))
    }
}

pub async fn popular_posts(
    State(state): State<AppState>,
    auth: ApiAuth,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    ranked_posts(
        &state,
        &auth,
        &headers,
        &params,
        "(SELECT COALESCE(SUM(score),0) FROM votes v WHERE v.post_id = posts.id)
         + (SELECT COUNT(*) FROM favorites f WHERE f.post_id = posts.id) * 3 DESC",
    )
}

pub async fn hot_posts(
    State(state): State<AppState>,
    auth: ApiAuth,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    ranked_posts(
        &state,
        &auth,
        &headers,
        &params,
        "(SELECT COALESCE(SUM(score),0) FROM votes v WHERE v.post_id = posts.id)
         / (1.0 + (julianday('now') - julianday(posts.created_at))) DESC",
    )
}

#[derive(serde::Deserialize)]
pub struct VoteBody {
    pub score: Option<Value>,
    pub no_unvote: Option<bool>,
}

pub async fn vote_post(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;

    let raw = String::from_utf8_lossy(&body);
    let score_str = if raw.trim_start().starts_with('{') {
        serde_json::from_slice::<VoteBody>(&body)
            .ok()
            .and_then(|b| b.score)
            .map(|v| match v {
                Value::String(s) => s,
                other => other.to_string(),
            })
            .unwrap_or_default()
    } else {
        raw.split('&')
            .find_map(|p| p.strip_prefix("score=").map(urldec))
            .unwrap_or_default()
    };
    let score: i64 = match score_str.as_str() {
        "up" | "1" => 1,
        "down" | "-1" => -1,
        "0" | "none" => 0,
        _ => return Err(ApiError::bad_request("score must be up, down, 1, -1 or 0")),
    };

    let conn = state.db.lock().map_err(ApiError::internal)?;
    let post = db::get_post(&conn, id).ok_or_else(ApiError::not_found)?;
    if post.is_deleted {
        return Err(ApiError::not_found());
    }
    if score == 0 {
        conn.execute(
            "DELETE FROM votes WHERE post_id = ?1 AND user_id = ?2",
            params![id, user.id],
        )
        .map_err(ApiError::internal)?;
    } else {
        conn.execute(
            "INSERT INTO votes (post_id, user_id, score, created_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(post_id, user_id) DO UPDATE SET score = excluded.score",
            params![id, user.id, score, db::ts(Utc::now())],
        )
        .map_err(ApiError::internal)?;
    }
    let (up, down) = db::score_parts(&conn, id);
    Ok(Json(json!({
        "success": true,
        "score": up - down,
        "our_score": score,
    })))
}

pub async fn list_favorites(
    State(state): State<AppState>,
    auth: ApiAuth,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let format = PostFormat::from_params(
        params.get("v2").map(String::as_str),
        params.get("mode").map(String::as_str),
    );
    let base = base_url_pub(&headers, &state.base_url);
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let sql = "SELECT posts.* FROM posts JOIN favorites f ON f.post_id = posts.id
         WHERE f.user_id = ?1 AND posts.is_deleted = 0
         ORDER BY f.created_at DESC LIMIT 320";
    let mut stmt = conn.prepare(sql).map_err(ApiError::internal)?;
    let posts: Vec<StoredPost> = stmt
        .query_map(params![user.id], db::row_to_post)
        .map_err(ApiError::internal)?
        .flatten()
        .collect();
    let items: Vec<Value> = posts
        .iter()
        .map(|p| post_json(&conn, p, &base, Some(user.id), format))
        .collect();
    if format.is_v2() {
        Ok(Json(Value::Array(items)))
    } else {
        Ok(Json(json!({ "posts": items })))
    }
}

pub async fn add_favorite(
    State(state): State<AppState>,
    auth: ApiAuth,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let raw = String::from_utf8_lossy(&body);
    let post_id: i64 = if raw.trim_start().starts_with('{') {
        serde_json::from_slice::<Value>(&body)
            .ok()
            .and_then(|v| v["post_id"].as_i64())
            .ok_or_else(|| ApiError::bad_request("post_id required"))?
    } else {
        raw.split('&')
            .find_map(|p| p.strip_prefix("post_id=").map(urldec))
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ApiError::bad_request("post_id required"))?
    };
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let post = db::get_post(&conn, post_id).ok_or_else(ApiError::not_found)?;
    if post.is_deleted {
        return Err(ApiError::not_found());
    }
    conn.execute(
        "INSERT OR IGNORE INTO favorites (post_id, user_id, created_at) VALUES (?1, ?2, ?3)",
        params![post_id, user.id, db::ts(Utc::now())],
    )
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "success": true, "post_id": post_id })))
}

pub async fn remove_favorite(
    State(state): State<AppState>,
    auth: ApiAuth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id: i64 = strip_json_suffix(&id)
        .parse()
        .map_err(|_| ApiError::not_found())?;
    let user = auth.user().cloned().ok_or_else(ApiError::unauthorized)?;
    let conn = state.db.lock().map_err(ApiError::internal)?;
    conn.execute(
        "DELETE FROM favorites WHERE post_id = ?1 AND user_id = ?2",
        params![id, user.id],
    )
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "success": true })))
}

/// e621 clients GET /favorites/:id to check state; mirror a post-shaped reply.
pub async fn show_favorite_post(
    State(state): State<AppState>,
    auth: ApiAuth,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    show_post(State(state), auth, headers, Path(id), Query(HashMap::new())).await
}

#[derive(serde::Deserialize)]
pub struct CountsParams {
    pub tags: Option<String>,
}

pub async fn post_counts(
    State(state): State<AppState>,
    Query(params): Query<CountsParams>,
) -> ApiResult<Json<Value>> {
    let q = search::parse(
        params.tags.as_deref().unwrap_or_default(),
        search::Page {
            limit: 1,
            offset: 0,
        },
    );
    let conn = state.db.lock().map_err(ApiError::internal)?;
    let mut sql = "SELECT COUNT(*) FROM posts".to_string();
    let mut conds = vec!["posts.is_deleted = 0".to_string()];
    if !q.where_sql.is_empty() {
        conds.push(q.where_sql);
    }
    sql.push_str(&format!(" WHERE {}", conds.join(" AND ")));
    let params_refs: Vec<&dyn rusqlite::ToSql> = q.params.iter().map(|p| p.as_ref()).collect();
    let count: i64 = conn
        .query_row(&sql, params_refs.as_slice(), |r| r.get(0))
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "counts": { "posts": count } })))
}

/// Reverse-image-search is not implemented; return an empty result set so
/// clients that probe the endpoint degrade gracefully.
pub async fn iqdb_stub() -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "posts": [] })))
}
