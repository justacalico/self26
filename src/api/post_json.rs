//! Serializes posts to e621's legacy format and the newer `v2` format
//! (`?v2=true&mode=basic|extended|thumbnail`).

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::db;
use crate::models::StoredPost;

/// Which post representation a request wants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostFormat {
    Legacy,
    V2Basic,
    V2Extended,
    V2Thumbnail,
}

impl PostFormat {
    pub fn from_params(v2: Option<&str>, mode: Option<&str>) -> Self {
        let wants_v2 = matches!(v2, Some("true") | Some("1") | Some("t"));
        if !wants_v2 {
            return PostFormat::Legacy;
        }
        match mode.unwrap_or("basic") {
            "extended" => PostFormat::V2Extended,
            "thumbnail" => PostFormat::V2Thumbnail,
            _ => PostFormat::V2Basic,
        }
    }

    pub fn is_v2(self) -> bool {
        self != PostFormat::Legacy
    }
}

fn data_url(base: &str, p: &StoredPost, kind: &str, ext: &str) -> String {
    format!(
        "{}/data/{}/{}/{}.{}",
        base.trim_end_matches('/'),
        kind,
        &p.md5[..2],
        p.md5,
        ext
    )
}

/// Serialize one post for list/index responses.
pub fn post_json(
    conn: &Connection,
    p: &StoredPost,
    base: &str,
    viewer: Option<i64>,
    format: PostFormat,
) -> Value {
    match format {
        PostFormat::Legacy => legacy_post(conn, p, base, viewer),
        PostFormat::V2Thumbnail => thumbnail_post(conn, p, base, viewer),
        PostFormat::V2Basic => v2_post(conn, p, base, viewer, false),
        PostFormat::V2Extended => v2_post(conn, p, base, viewer, true),
    }
}

fn tags_by_category(conn: &Connection, post_id: i64) -> Value {
    let mut cats: Vec<(&str, Vec<String>)> = vec![
        ("general", vec![]),
        ("artist", vec![]),
        ("contributor", vec![]),
        ("copyright", vec![]),
        ("character", vec![]),
        ("species", vec![]),
        ("invalid", vec![]),
        ("meta", vec![]),
        ("lore", vec![]),
    ];
    for (name, cat) in db::post_tags(conn, post_id) {
        let key = crate::models::tag_category::name(cat);
        if let Some(entry) = cats.iter_mut().find(|(k, _)| *k == key) {
            entry.1.push(name);
        } else {
            cats[0].1.push(name);
        }
    }
    let mut map = serde_json::Map::new();
    for (k, v) in cats {
        map.insert(k.to_string(), json!(v));
    }
    Value::Object(map)
}

fn all_tag_names(conn: &Connection, post_id: i64) -> Vec<String> {
    db::post_tags(conn, post_id)
        .into_iter()
        .map(|(n, _)| n)
        .collect()
}

fn preview_dims(p: &StoredPost) -> (i64, i64) {
    if p.preview_width > 0 {
        (p.preview_width, p.preview_height)
    } else {
        let (w, h) = fit_dims(p.width, p.height, 150);
        (w, h)
    }
}

fn sample_dims(p: &StoredPost) -> (i64, i64) {
    if p.sample_width > 0 {
        (p.sample_width, p.sample_height)
    } else {
        fit_dims(p.width, p.height, 850)
    }
}

fn fit_dims(w: i64, h: i64, max: i64) -> (i64, i64) {
    if w <= max && h <= max || w <= 0 || h <= 0 {
        return (w.max(1), h.max(1));
    }
    let scale = max as f64 / w.max(h) as f64;
    (
        ((w as f64) * scale).round() as i64,
        ((h as f64) * scale).round() as i64,
    )
}

fn legacy_post(conn: &Connection, p: &StoredPost, base: &str, viewer: Option<i64>) -> Value {
    let (up, down) = db::score_parts(conn, p.id);
    let (pw, ph) = preview_dims(p);
    let (sw, sh) = sample_dims(p);
    let children = db::post_children(conn, p.id);
    let has_children = !children.is_empty();
    let tags = tags_by_category(conn, p.id);
    let flat: Vec<String> = db::post_tags(conn, p.id)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    let file_url = data_url(base, p, "original", &p.ext);
    let preview_url = data_url(base, p, "preview", "jpg");
    let sample_url = if p.has_sample {
        data_url(base, p, "sample", "jpg")
    } else {
        file_url.clone()
    };

    json!({
        "id": p.id,
        "created_at": p.created_at,
        "updated_at": p.updated_at,
        "file": {
            "width": p.width,
            "height": p.height,
            "ext": p.ext,
            "size": p.size,
            "md5": p.md5,
            "url": file_url,
        },
        "preview": { "width": pw, "height": ph, "url": preview_url },
        "sample": {
            "has": p.has_sample,
            "width": sw,
            "height": sh,
            "url": sample_url,
            "alternates": {},
        },
        "score": { "up": up, "down": down, "total": up - down },
        "tags": tags,
        "locked_tags": p.locked_tags,
        "change_seq": p.change_seq,
        "flags": {
            "pending": p.is_pending,
            "flagged": p.is_flagged,
            "note_locked": p.note_locked,
            "status_locked": p.status_locked,
            "rating_locked": p.rating_locked,
            "deleted": p.is_deleted,
        },
        "rating": p.rating.as_str(),
        "fav_count": db::fav_count(conn, p.id),
        "sources": p.sources,
        "pools": db::post_pools(conn, p.id),
        "relationships": {
            "parent_id": p.parent_id,
            "has_children": has_children,
            "has_active_children": has_children,
            "children": children,
        },
        "approver_id": p.approver_id,
        "uploader_id": p.uploader_id,
        "description": p.description,
        "comment_count": db::comment_count(conn, p.id),
        "is_favorited": db::is_favorited(conn, p.id, viewer),
        "has_notes": has_notes(conn, p.id),
        "duration": if p.duration > 0.0 { json!(p.duration) } else { Value::Null },
        // Extra string form used by several e621 clients.
        "tag_string": flat.join(" "),
    })
}

fn has_notes(conn: &Connection, post_id: i64) -> bool {
    conn.query_row(
        "SELECT 1 FROM notes WHERE post_id = ?1 AND is_active = 1 LIMIT 1",
        rusqlite::params![post_id],
        |_| Ok(()),
    )
    .is_ok()
}

/// The shared v2 object used by `mode=basic` and `mode=extended`.
fn v2_post(
    conn: &Connection,
    p: &StoredPost,
    base: &str,
    viewer: Option<i64>,
    extended_tags: bool,
) -> Value {
    let (up, down) = db::score_parts(conn, p.id);
    let (pw, ph) = preview_dims(p);
    let (sw, sh) = sample_dims(p);
    let children = db::post_children(conn, p.id);
    let file_url = data_url(base, p, "original", &p.ext);
    let is_video = matches!(p.ext.as_str(), "webm" | "mp4" | "mov");

    let mut files = json!({
        "meta": {
            "md5": p.md5,
            "ext": p.ext,
            "size": p.size,
            "duration": p.duration,
            "has_sample": p.has_sample,
        },
        "original": { "width": p.width, "height": p.height, "url": file_url },
        "preview": {
            "width": pw,
            "height": ph,
            "jpg": data_url(base, p, "preview", "jpg"),
            "webp": data_url(base, p, "preview", "webp"),
        },
        "sample": {
            "width": sw,
            "height": sh,
            "jpg": data_url(base, p, "sample", "jpg"),
            "webp": data_url(base, p, "sample", "webp"),
        },
    });
    if is_video {
        files["video"] = json!({
            "has": true,
            "original": { "fps": Value::Null, "codec": Value::Null, "size": p.size,
                          "width": p.width, "height": p.height,
                          "url": data_url(base, p, "original", &p.ext) },
            "variants": {},
            "samples": {},
        });
    }

    let tags = if extended_tags {
        tags_by_category(conn, p.id)
    } else {
        json!(all_tag_names(conn, p.id))
    };

    json!({
        "id": p.id,
        "created_at": p.created_at,
        "updated_at": p.updated_at,
        "change_seq": p.change_seq,
        "files": files,
        "uploader_id": p.uploader_id,
        "uploader_name": db::user_name(conn, p.uploader_id),
        "approver_id": p.approver_id,
        "stats": {
            "score": { "up": up, "down": down, "total": up - down },
            "fav_count": db::fav_count(conn, p.id),
            "is_favorited": db::is_favorited(conn, p.id, viewer),
            "vote": db::user_vote(conn, p.id, viewer),
            "comment_count": db::comment_count(conn, p.id),
            "hotness": up - down,
        },
        "flags": {
            "pending": p.is_pending,
            "flagged": p.is_flagged,
            "note_locked": p.note_locked,
            "status_locked": p.status_locked,
            "rating_locked": p.rating_locked,
            "deleted": p.is_deleted,
        },
        "has": {
            "parent": p.parent_id.is_some(),
            "children": !children.is_empty(),
            "active_children": !children.is_empty(),
            "notes": has_notes(conn, p.id),
            "sample": p.has_sample,
        },
        "relationships": { "parent_id": p.parent_id, "children": children },
        "pools": db::post_pools(conn, p.id),
        "rating": p.rating.as_str(),
        "locked_tags": p.locked_tags,
        "sources": p.sources,
        "description": p.description,
        "tags": tags,
    })
}

/// `mode=thumbnail`: the flat, lightweight listing format.
fn thumbnail_post(conn: &Connection, p: &StoredPost, base: &str, viewer: Option<i64>) -> Value {
    let (up, down) = db::score_parts(conn, p.id);
    let (pw, ph) = preview_dims(p);
    let mut flags = Vec::new();
    if p.is_pending {
        flags.push("pending");
    }
    if p.is_flagged {
        flags.push("flagged");
    }
    if p.is_deleted {
        flags.push("deleted");
    }

    json!({
        "id": p.id,
        "created_at": p.created_at,
        "md5": p.md5,
        "file_ext": p.ext,
        "width": p.width,
        "height": p.height,
        "size": p.size,
        "duration": p.duration,
        "preview_url": data_url(base, p, "preview", "jpg"),
        "preview_webp": data_url(base, p, "preview", "webp"),
        "sample_url": data_url(base, p, "sample", "jpg"),
        "file_url": data_url(base, p, "original", &p.ext),
        "preview_width": pw,
        "preview_height": ph,
        "uploader_id": p.uploader_id,
        "uploader": db::user_name(conn, p.uploader_id),
        "score": up - down,
        "fav_count": db::fav_count(conn, p.id),
        "is_favorited": db::is_favorited(conn, p.id, viewer),
        "vote": db::user_vote(conn, p.id, viewer),
        "comment_count": db::comment_count(conn, p.id),
        "flags": flags.join(" "),
        "pools": db::post_pools(conn, p.id),
        "rating": p.rating.as_str(),
        "tags": all_tag_names(conn, p.id),
    })
}
